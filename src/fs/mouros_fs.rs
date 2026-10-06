use alloc::string::String;
use alloc::vec::Vec;
use super::block::{BLOCK_SIZE, CachedBlockDevice, FsError};

pub const FS_MAGIC: u32 = 0x4D4F5552; // "MOUR"
pub const FS_VERSION: u32 = 1;

pub const INODE_SIZE: usize = 128;
pub const INODES_PER_BLOCK: usize = BLOCK_SIZE / INODE_SIZE; // 8
pub const DIRENTRY_SIZE: usize = 64;
pub const DIRENTRIES_PER_BLOCK: usize = BLOCK_SIZE / DIRENTRY_SIZE; // 16

pub const ROOT_INODE: u32 = 1;

pub const FILE_TYPE_UNUSED: u8 = 0;
pub const FILE_TYPE_REGULAR: u8 = 1;
pub const FILE_TYPE_DIR: u8 = 2;
pub const FILE_TYPE_CHARDEV: u8 = 3;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Superblock {
    pub magic: u32,
    pub version: u32,
    pub block_size: u32,
    pub total_blocks: u32,
    pub total_inodes: u32,
    pub free_blocks: u32,
    pub free_inodes: u32,
    pub inode_bitmap_block: u32,
    pub inode_bitmap_blocks: u32,
    pub block_bitmap_block: u32,
    pub block_bitmap_blocks: u32,
    pub inode_table_block: u32,
    pub inode_table_blocks: u32,
    pub data_blocks_start: u32,
    pub root_inode: u32,
    pub padding: [u8; 964],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RawInode {
    pub file_type: u8,
    pub permissions: u16,
    pub uid: u16,
    pub gid: u16,
    pub size: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub atime: u32,
    pub direct_blocks: [u32; 12],
    pub indirect_block: u32,
    pub double_indirect_block: u32,
    pub padding: [u8; 51],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct RawDirEntry {
    pub inode: u32,
    pub file_type: u8,
    pub name_len: u8,
    pub name: [u8; 58],
}

#[derive(Debug, Clone)]
pub struct InodeStat {
    pub inode: u32,
    pub file_type: u8,
    pub permissions: u16,
    pub uid: u16,
    pub gid: u16,
    pub size: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub atime: u32,
    pub block_count: u32,
}

#[derive(Debug, Clone)]
pub struct DirEntryInfo {
    pub inode: u32,
    pub file_type: u8,
    pub name: String,
}

pub struct MourosFileSystem {
    pub device: CachedBlockDevice,
    pub sb: Superblock,
}

impl MourosFileSystem {
    /// Try mounting an existing filesystem, or format if magic is missing
    pub fn open_or_format(mut device: CachedBlockDevice) -> Result<Self, FsError> {
        let mut block = [0u8; BLOCK_SIZE];
        device.read_block(0, &mut block)?;

        let sb = unsafe { core::ptr::read_unaligned(block.as_ptr() as *const Superblock) };
        if sb.magic == FS_MAGIC && sb.version == FS_VERSION {
            let tot_blocks = sb.total_blocks;
            let tot_inodes = sb.total_inodes;
            let free_blocks = sb.free_blocks;
            let free_inodes = sb.free_inodes;
            crate::serial_println!(
                "[FS] Mounted MourosFS: {} total blocks, {} inodes (free: {} blocks, {} inodes)",
                tot_blocks,
                tot_inodes,
                free_blocks,
                free_inodes
            );
            Ok(Self { device, sb })
        } else {
            crate::serial_println!("[FS] No valid MourosFS detected. Formatting storage device...");
            Self::format_device(device)
        }
    }

    /// Format the block device with a fresh MourosFS filesystem
    pub fn format_device(mut device: CachedBlockDevice) -> Result<Self, FsError> {
        let total_blocks = device.block_count().min(u32::MAX as u64) as u32;
        if total_blocks < 256 {
            return Err(FsError::NoSpace);
        }

        let total_inodes = 1024u32;
        let inode_bitmap_blocks = 1u32;
        let block_bitmap_blocks = ((total_blocks + 8191) / 8192).max(1);
        let inode_table_blocks = (total_inodes * INODE_SIZE as u32 + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32;

        let inode_bitmap_block = 1u32;
        let block_bitmap_block = inode_bitmap_block + inode_bitmap_blocks;
        let inode_table_block = block_bitmap_block + block_bitmap_blocks;
        let data_blocks_start = inode_table_block + inode_table_blocks;

        let free_blocks = total_blocks.saturating_sub(data_blocks_start + 1); // 1 data block used for root dir
        let free_inodes = total_inodes.saturating_sub(2); // inode 0 reserved, inode 1 is root

        let sb = Superblock {
            magic: FS_MAGIC,
            version: FS_VERSION,
            block_size: BLOCK_SIZE as u32,
            total_blocks,
            total_inodes,
            free_blocks,
            free_inodes,
            inode_bitmap_block,
            inode_bitmap_blocks,
            block_bitmap_block,
            block_bitmap_blocks,
            inode_table_block,
            inode_table_blocks,
            data_blocks_start,
            root_inode: ROOT_INODE,
            padding: [0u8; 964],
        };

        // Write Superblock
        let sb_bytes = unsafe {
            core::slice::from_raw_parts(&sb as *const Superblock as *const u8, BLOCK_SIZE)
        };
        device.write_block(0, sb_bytes)?;

        // Initialize Inode Bitmap (inode 0 and 1 allocated -> bits 0 and 1 set: 0b00000011 = 0x03)
        let mut in_bm = [0u8; BLOCK_SIZE];
        in_bm[0] = 0x03;
        device.write_block(inode_bitmap_block as u64, &in_bm)?;

        // Initialize Block Bitmap (mark metadata blocks + root data block as allocated)
        let root_data_block = data_blocks_start;
        let used_blocks = root_data_block + 1;
        let mut blk_bm = [0u8; BLOCK_SIZE];

        let full_bytes = (used_blocks / 8) as usize;
        let rem_bits = (used_blocks % 8) as u8;
        for i in 0..full_bytes {
            if i < BLOCK_SIZE {
                blk_bm[i] = 0xFF;
            }
        }
        if rem_bits > 0 && full_bytes < BLOCK_SIZE {
            blk_bm[full_bytes] = (1u8 << rem_bits) - 1;
        }
        device.write_block(block_bitmap_block as u64, &blk_bm)?;

        // Clear remaining block bitmap blocks if any
        let empty = [0u8; BLOCK_SIZE];
        for b in (block_bitmap_block + 1)..(block_bitmap_block + block_bitmap_blocks) {
            device.write_block(b as u64, &empty)?;
        }

        // Initialize Inode Table with zeroed inodes
        for b in inode_table_block..(inode_table_block + inode_table_blocks) {
            device.write_block(b as u64, &empty)?;
        }

        let now = crate::drivers::rtc::read_seconds() as u32;

        // Initialize Root Inode (Inode 1)
        let mut root_inode = RawInode {
            file_type: FILE_TYPE_DIR,
            permissions: 0o755,
            uid: 0,
            gid: 0,
            size: (2 * DIRENTRY_SIZE) as u32, // '.' and '..'
            ctime: now,
            mtime: now,
            atime: now,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect_block: 0,
            padding: [0; 51],
        };
        root_inode.direct_blocks[0] = root_data_block;

        let root_bytes = unsafe {
            core::slice::from_raw_parts(&root_inode as *const RawInode as *const u8, INODE_SIZE)
        };
        let mut first_inode_block = [0u8; BLOCK_SIZE];
        // Inode 1 is at index 1 * 128
        first_inode_block[INODE_SIZE..INODE_SIZE * 2].copy_from_slice(root_bytes);
        device.write_block(inode_table_block as u64, &first_inode_block)?;

        // Initialize Root Directory Data Block with '.' and '..'
        let mut root_dir_block = [0u8; BLOCK_SIZE];
        let dot = RawDirEntry::new(ROOT_INODE, FILE_TYPE_DIR, ".");
        let dotdot = RawDirEntry::new(ROOT_INODE, FILE_TYPE_DIR, "..");

        dot.write_to_bytes(&mut root_dir_block[0..DIRENTRY_SIZE]);
        dotdot.write_to_bytes(&mut root_dir_block[DIRENTRY_SIZE..DIRENTRY_SIZE * 2]);
        device.write_block(root_data_block as u64, &root_dir_block)?;

        device.flush()?;

        crate::serial_println!("[FS] Formatted MourosFS successfully!");
        Ok(Self { device, sb })
    }

    // --- Inode Management ---

    pub fn read_inode(&mut self, inode_idx: u32) -> Result<RawInode, FsError> {
        if inode_idx == 0 || inode_idx >= self.sb.total_inodes {
            return Err(FsError::NotFound);
        }
        let block_offset = (inode_idx as usize * INODE_SIZE) / BLOCK_SIZE;
        let inode_offset = (inode_idx as usize * INODE_SIZE) % BLOCK_SIZE;
        let block_idx = self.sb.inode_table_block as u64 + block_offset as u64;

        let mut block = [0u8; BLOCK_SIZE];
        self.device.read_block(block_idx, &mut block)?;

        let raw = unsafe {
            core::ptr::read_unaligned(block[inode_offset..].as_ptr() as *const RawInode)
        };
        Ok(raw)
    }

    pub fn write_inode(&mut self, inode_idx: u32, inode: &RawInode) -> Result<(), FsError> {
        if inode_idx == 0 || inode_idx >= self.sb.total_inodes {
            return Err(FsError::InvalidArgument);
        }
        let block_offset = (inode_idx as usize * INODE_SIZE) / BLOCK_SIZE;
        let inode_offset = (inode_idx as usize * INODE_SIZE) % BLOCK_SIZE;
        let block_idx = self.sb.inode_table_block as u64 + block_offset as u64;

        let mut block = [0u8; BLOCK_SIZE];
        self.device.read_block(block_idx, &mut block)?;

        let raw_bytes = unsafe {
            core::slice::from_raw_parts(inode as *const RawInode as *const u8, INODE_SIZE)
        };
        block[inode_offset..inode_offset + INODE_SIZE].copy_from_slice(raw_bytes);
        self.device.write_block(block_idx, &block)?;
        Ok(())
    }

    pub fn allocate_inode(&mut self, file_type: u8, permissions: u16) -> Result<u32, FsError> {
        let mut bm_block = [0u8; BLOCK_SIZE];
        self.device.read_block(self.sb.inode_bitmap_block as u64, &mut bm_block)?;

        let total_inodes = self.sb.total_inodes as usize;
        let mut free_idx = None;

        for byte_idx in 0..(total_inodes / 8) {
            if bm_block[byte_idx] != 0xFF {
                for bit in 0..8 {
                    if (bm_block[byte_idx] & (1 << bit)) == 0 {
                        let idx = byte_idx * 8 + bit;
                        if idx > 0 && idx < total_inodes {
                            free_idx = Some(idx as u32);
                            bm_block[byte_idx] |= 1 << bit;
                            break;
                        }
                    }
                }
                if free_idx.is_some() {
                    break;
                }
            }
        }

        let inode_idx = free_idx.ok_or(FsError::NoInode)?;
        self.device.write_block(self.sb.inode_bitmap_block as u64, &bm_block)?;

        self.sb.free_inodes = self.sb.free_inodes.saturating_sub(1);
        self.save_superblock()?;

        let now = crate::drivers::rtc::read_seconds() as u32;
        let new_inode = RawInode {
            file_type,
            permissions,
            uid: 0,
            gid: 0,
            size: 0,
            ctime: now,
            mtime: now,
            atime: now,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect_block: 0,
            padding: [0; 51],
        };
        self.write_inode(inode_idx, &new_inode)?;
        Ok(inode_idx)
    }

    pub fn free_inode(&mut self, inode_idx: u32) -> Result<(), FsError> {
        if inode_idx <= ROOT_INODE || inode_idx >= self.sb.total_inodes {
            return Err(FsError::InvalidArgument);
        }
        let byte_idx = (inode_idx / 8) as usize;
        let bit_idx = (inode_idx % 8) as u8;

        let mut bm_block = [0u8; BLOCK_SIZE];
        self.device.read_block(self.sb.inode_bitmap_block as u64, &mut bm_block)?;
        bm_block[byte_idx] &= !(1 << bit_idx);
        self.device.write_block(self.sb.inode_bitmap_block as u64, &bm_block)?;

        self.sb.free_inodes += 1;
        self.save_superblock()?;

        let empty = RawInode {
            file_type: FILE_TYPE_UNUSED,
            permissions: 0,
            uid: 0,
            gid: 0,
            size: 0,
            ctime: 0,
            mtime: 0,
            atime: 0,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect_block: 0,
            padding: [0; 51],
        };
        self.write_inode(inode_idx, &empty)?;
        Ok(())
    }

    // --- Block Allocation Management ---

    pub fn allocate_block(&mut self) -> Result<u32, FsError> {
        let mut bm_block = [0u8; BLOCK_SIZE];
        let total = self.sb.total_blocks as usize;
        let start = self.sb.data_blocks_start as usize;

        for b_idx in 0..self.sb.block_bitmap_blocks {
            let cur_bm_block = self.sb.block_bitmap_block as u64 + b_idx as u64;
            self.device.read_block(cur_bm_block, &mut bm_block)?;

            for byte_idx in 0..BLOCK_SIZE {
                if bm_block[byte_idx] != 0xFF {
                    for bit in 0..8 {
                        if (bm_block[byte_idx] & (1 << bit)) == 0 {
                            let block_num = (b_idx as usize * 8192) + (byte_idx * 8) + bit;
                            if block_num >= start && block_num < total {
                                bm_block[byte_idx] |= 1 << bit;
                                self.device.write_block(cur_bm_block, &bm_block)?;

                                self.sb.free_blocks = self.sb.free_blocks.saturating_sub(1);
                                self.save_superblock()?;

                                // Zero out allocated data block
                                let zero = [0u8; BLOCK_SIZE];
                                self.device.write_block(block_num as u64, &zero)?;
                                return Ok(block_num as u32);
                            }
                        }
                    }
                }
            }
        }
        Err(FsError::NoSpace)
    }

    pub fn free_block(&mut self, block_num: u32) -> Result<(), FsError> {
        if block_num < self.sb.data_blocks_start || block_num >= self.sb.total_blocks {
            return Err(FsError::InvalidArgument);
        }
        let b_idx = block_num / 8192;
        let in_b = (block_num % 8192) as usize;
        let byte_idx = in_b / 8;
        let bit_idx = (in_b % 8) as u8;

        let cur_bm_block = self.sb.block_bitmap_block as u64 + b_idx as u64;
        let mut bm_block = [0u8; BLOCK_SIZE];
        self.device.read_block(cur_bm_block, &mut bm_block)?;
        bm_block[byte_idx] &= !(1 << bit_idx);
        self.device.write_block(cur_bm_block, &bm_block)?;

        self.sb.free_blocks += 1;
        self.save_superblock()?;
        Ok(())
    }

    fn save_superblock(&mut self) -> Result<(), FsError> {
        let sb_bytes = unsafe {
            core::slice::from_raw_parts(&self.sb as *const Superblock as *const u8, BLOCK_SIZE)
        };
        self.device.write_block(0, sb_bytes)
    }

    // --- File Read / Write / Truncate ---

    pub fn read_file(&mut self, inode_idx: u32, offset: usize, buf: &mut [u8]) -> Result<usize, FsError> {
        let inode = self.read_inode(inode_idx)?;
        if inode.file_type != FILE_TYPE_REGULAR && inode.file_type != FILE_TYPE_CHARDEV {
            return Err(FsError::IsADirectory);
        }

        let file_size = inode.size as usize;
        if offset >= file_size {
            return Ok(0);
        }

        let bytes_to_read = buf.len().min(file_size - offset);
        let mut bytes_read = 0;

        let mut temp_block = [0u8; BLOCK_SIZE];

        while bytes_read < bytes_to_read {
            let cur_pos = offset + bytes_read;
            let block_logical_idx = cur_pos / BLOCK_SIZE;
            let in_block_offset = cur_pos % BLOCK_SIZE;

            let physical_block = self.get_inode_block(&inode, block_logical_idx)?;
            if physical_block == 0 {
                // Sparse block or hole: zeroes
                temp_block.fill(0);
            } else {
                self.device.read_block(physical_block as u64, &mut temp_block)?;
            }

            let chunk_size = (BLOCK_SIZE - in_block_offset).min(bytes_to_read - bytes_read);
            buf[bytes_read..bytes_read + chunk_size]
                .copy_from_slice(&temp_block[in_block_offset..in_block_offset + chunk_size]);
            bytes_read += chunk_size;
        }

        Ok(bytes_read)
    }

    pub fn write_file(&mut self, inode_idx: u32, offset: usize, buf: &[u8]) -> Result<usize, FsError> {
        let mut inode = self.read_inode(inode_idx)?;
        if inode.file_type != FILE_TYPE_REGULAR && inode.file_type != FILE_TYPE_CHARDEV {
            return Err(FsError::IsADirectory);
        }

        let mut bytes_written = 0;
        let mut temp_block = [0u8; BLOCK_SIZE];

        while bytes_written < buf.len() {
            let cur_pos = offset + bytes_written;
            let block_logical_idx = cur_pos / BLOCK_SIZE;
            let in_block_offset = cur_pos % BLOCK_SIZE;

            let mut physical_block = self.get_inode_block(&inode, block_logical_idx)?;
            if physical_block == 0 {
                physical_block = self.allocate_block()?;
                self.set_inode_block(&mut inode, block_logical_idx, physical_block)?;
            }

            self.device.read_block(physical_block as u64, &mut temp_block)?;

            let chunk_size = (BLOCK_SIZE - in_block_offset).min(buf.len() - bytes_written);
            temp_block[in_block_offset..in_block_offset + chunk_size]
                .copy_from_slice(&buf[bytes_written..bytes_written + chunk_size]);

            self.device.write_block(physical_block as u64, &temp_block)?;
            bytes_written += chunk_size;
        }

        let new_size = (offset + bytes_written) as u32;
        if new_size > inode.size {
            inode.size = new_size;
        }
        inode.mtime = crate::drivers::rtc::read_seconds() as u32;
        self.write_inode(inode_idx, &inode)?;

        Ok(bytes_written)
    }

    pub fn truncate_file(&mut self, inode_idx: u32, new_size: usize) -> Result<(), FsError> {
        let mut inode = self.read_inode(inode_idx)?;
        if inode.file_type != FILE_TYPE_REGULAR {
            return Err(FsError::IsADirectory);
        }

        let old_block_count = (inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let new_block_count = (new_size + BLOCK_SIZE - 1) / BLOCK_SIZE;

        if new_block_count < old_block_count {
            for b in new_block_count..old_block_count {
                let phys = self.get_inode_block(&inode, b)?;
                if phys != 0 {
                    self.free_block(phys)?;
                    self.set_inode_block(&mut inode, b, 0)?;
                }
            }
        }

        inode.size = new_size as u32;
        inode.mtime = crate::drivers::rtc::read_seconds() as u32;
        self.write_inode(inode_idx, &inode)?;
        Ok(())
    }

    fn get_inode_block(&mut self, inode: &RawInode, logical_idx: usize) -> Result<u32, FsError> {
        if logical_idx < 12 {
            Ok(inode.direct_blocks[logical_idx])
        } else if logical_idx < 12 + 256 {
            if inode.indirect_block == 0 {
                return Ok(0);
            }
            let mut ind_block = [0u8; BLOCK_SIZE];
            self.device.read_block(inode.indirect_block as u64, &mut ind_block)?;
            let entry_idx = logical_idx - 12;
            let offset = entry_idx * 4;
            let phys_num = u32::from_le_bytes([
                ind_block[offset],
                ind_block[offset + 1],
                ind_block[offset + 2],
                ind_block[offset + 3],
            ]);
            Ok(phys_num)
        } else {
            Err(FsError::NoSpace)
        }
    }

    fn set_inode_block(&mut self, inode: &mut RawInode, logical_idx: usize, phys_block: u32) -> Result<(), FsError> {
        if logical_idx < 12 {
            inode.direct_blocks[logical_idx] = phys_block;
            Ok(())
        } else if logical_idx < 12 + 256 {
            if inode.indirect_block == 0 {
                inode.indirect_block = self.allocate_block()?;
            }
            let mut ind_block = [0u8; BLOCK_SIZE];
            self.device.read_block(inode.indirect_block as u64, &mut ind_block)?;
            let entry_idx = logical_idx - 12;
            let offset = entry_idx * 4;
            ind_block[offset..offset + 4].copy_from_slice(&phys_block.to_le_bytes());
            self.device.write_block(inode.indirect_block as u64, &ind_block)?;
            Ok(())
        } else {
            Err(FsError::NoSpace)
        }
    }

    // --- Directory Operations ---

    pub fn lookup(&mut self, dir_inode_idx: u32, name: &str) -> Result<u32, FsError> {
        let dir_inode = self.read_inode(dir_inode_idx)?;
        if dir_inode.file_type != FILE_TYPE_DIR {
            return Err(FsError::NotADirectory);
        }

        let mut block = [0u8; BLOCK_SIZE];
        let total_blocks = (dir_inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;

        for b in 0..total_blocks {
            let phys = self.get_inode_block(&dir_inode, b)?;
            if phys == 0 {
                continue;
            }
            self.device.read_block(phys as u64, &mut block)?;

            for i in 0..DIRENTRIES_PER_BLOCK {
                let off = i * DIRENTRY_SIZE;
                let entry = RawDirEntry::read_from_bytes(&block[off..off + DIRENTRY_SIZE]);
                if entry.inode != 0 {
                    let len = entry.name_len as usize;
                    if len == name.len() && &entry.name[..len] == name.as_bytes() {
                        return Ok(entry.inode);
                    }
                }
            }
        }
        Err(FsError::NotFound)
    }

    pub fn list_dir(&mut self, dir_inode_idx: u32) -> Result<Vec<DirEntryInfo>, FsError> {
        let dir_inode = self.read_inode(dir_inode_idx)?;
        if dir_inode.file_type != FILE_TYPE_DIR {
            return Err(FsError::NotADirectory);
        }

        let mut result = Vec::new();
        let mut block = [0u8; BLOCK_SIZE];
        let total_blocks = (dir_inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;

        for b in 0..total_blocks {
            let phys = self.get_inode_block(&dir_inode, b)?;
            if phys == 0 {
                continue;
            }
            self.device.read_block(phys as u64, &mut block)?;

            for i in 0..DIRENTRIES_PER_BLOCK {
                let off = i * DIRENTRY_SIZE;
                let entry = RawDirEntry::read_from_bytes(&block[off..off + DIRENTRY_SIZE]);
                if entry.inode != 0 {
                    let len = entry.name_len as usize;
                    if let Ok(s) = core::str::from_utf8(&entry.name[..len]) {
                        result.push(DirEntryInfo {
                            inode: entry.inode,
                            file_type: entry.file_type,
                            name: String::from(s),
                        });
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn create_file_entry(&mut self, parent_inode: u32, name: &str, permissions: u16) -> Result<u32, FsError> {
        if self.lookup(parent_inode, name).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        let new_inode = self.allocate_inode(FILE_TYPE_REGULAR, permissions)?;
        self.add_dir_entry(parent_inode, new_inode, FILE_TYPE_REGULAR, name)?;
        Ok(new_inode)
    }

    pub fn create_dir_entry(&mut self, parent_inode: u32, name: &str, permissions: u16) -> Result<u32, FsError> {
        if self.lookup(parent_inode, name).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        let new_dir_inode = self.allocate_inode(FILE_TYPE_DIR, permissions)?;
        let data_block = self.allocate_block()?;

        let mut dir_inode = self.read_inode(new_dir_inode)?;
        dir_inode.direct_blocks[0] = data_block;
        dir_inode.size = (2 * DIRENTRY_SIZE) as u32;
        self.write_inode(new_dir_inode, &dir_inode)?;

        // Populate '.' and '..'
        let mut block = [0u8; BLOCK_SIZE];
        let dot = RawDirEntry::new(new_dir_inode, FILE_TYPE_DIR, ".");
        let dotdot = RawDirEntry::new(parent_inode, FILE_TYPE_DIR, "..");
        dot.write_to_bytes(&mut block[0..DIRENTRY_SIZE]);
        dotdot.write_to_bytes(&mut block[DIRENTRY_SIZE..DIRENTRY_SIZE * 2]);
        self.device.write_block(data_block as u64, &block)?;

        self.add_dir_entry(parent_inode, new_dir_inode, FILE_TYPE_DIR, name)?;
        Ok(new_dir_inode)
    }

    pub fn unlink_entry(&mut self, parent_inode: u32, name: &str) -> Result<(), FsError> {
        if name == "." || name == ".." {
            return Err(FsError::InvalidArgument);
        }
        let child_inode_idx = self.lookup(parent_inode, name)?;
        let child_inode = self.read_inode(child_inode_idx)?;

        if child_inode.file_type == FILE_TYPE_DIR {
            let entries = self.list_dir(child_inode_idx)?;
            // Only allowed to unlink if empty (only '.' and '..')
            if entries.len() > 2 {
                return Err(FsError::DirectoryNotEmpty);
            }
        }

        // Remove from parent dir
        self.remove_dir_entry(parent_inode, name)?;

        // Free data blocks
        let block_count = (child_inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;
        for b in 0..block_count {
            let phys = self.get_inode_block(&child_inode, b)?;
            if phys != 0 {
                self.free_block(phys)?;
            }
        }
        if child_inode.indirect_block != 0 {
            self.free_block(child_inode.indirect_block)?;
        }

        // Free inode
        self.free_inode(child_inode_idx)?;
        Ok(())
    }

    fn add_dir_entry(&mut self, parent_inode: u32, child_inode: u32, file_type: u8, name: &str) -> Result<(), FsError> {
        if name.len() > 58 {
            return Err(FsError::InvalidArgument);
        }
        let mut p_inode = self.read_inode(parent_inode)?;
        let total_blocks = (p_inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;

        let mut block = [0u8; BLOCK_SIZE];

        // 1. Look for an empty slot in existing directory blocks
        for b in 0..total_blocks {
            let phys = self.get_inode_block(&p_inode, b)?;
            if phys == 0 {
                continue;
            }
            self.device.read_block(phys as u64, &mut block)?;
            for i in 0..DIRENTRIES_PER_BLOCK {
                let off = i * DIRENTRY_SIZE;
                let entry = RawDirEntry::read_from_bytes(&block[off..off + DIRENTRY_SIZE]);
                if entry.inode == 0 {
                    let new_entry = RawDirEntry::new(child_inode, file_type, name);
                    new_entry.write_to_bytes(&mut block[off..off + DIRENTRY_SIZE]);
                    self.device.write_block(phys as u64, &block)?;
                    p_inode.mtime = crate::drivers::rtc::read_seconds() as u32;
                    self.write_inode(parent_inode, &p_inode)?;
                    return Ok(());
                }
            }
        }

        // 2. Need to allocate a new block for parent directory
        let new_block = self.allocate_block()?;
        self.set_inode_block(&mut p_inode, total_blocks, new_block)?;

        block.fill(0);
        let new_entry = RawDirEntry::new(child_inode, file_type, name);
        new_entry.write_to_bytes(&mut block[0..DIRENTRY_SIZE]);
        self.device.write_block(new_block as u64, &block)?;

        p_inode.size += DIRENTRY_SIZE as u32;
        p_inode.mtime = crate::drivers::rtc::read_seconds() as u32;
        self.write_inode(parent_inode, &p_inode)?;
        Ok(())
    }

    fn remove_dir_entry(&mut self, parent_inode: u32, name: &str) -> Result<(), FsError> {
        let mut p_inode = self.read_inode(parent_inode)?;
        let total_blocks = (p_inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let mut block = [0u8; BLOCK_SIZE];

        for b in 0..total_blocks {
            let phys = self.get_inode_block(&p_inode, b)?;
            if phys == 0 {
                continue;
            }
            self.device.read_block(phys as u64, &mut block)?;
            for i in 0..DIRENTRIES_PER_BLOCK {
                let off = i * DIRENTRY_SIZE;
                let entry = RawDirEntry::read_from_bytes(&block[off..off + DIRENTRY_SIZE]);
                if entry.inode != 0 && entry.name_len as usize == name.len() && &entry.name[..name.len()] == name.as_bytes() {
                    let mut cleared = entry;
                    cleared.inode = 0;
                    cleared.write_to_bytes(&mut block[off..off + DIRENTRY_SIZE]);
                    self.device.write_block(phys as u64, &block)?;
                    p_inode.mtime = crate::drivers::rtc::read_seconds() as u32;
                    self.write_inode(parent_inode, &p_inode)?;
                    return Ok(());
                }
            }
        }
        Err(FsError::NotFound)
    }

    pub fn stat(&mut self, inode_idx: u32) -> Result<InodeStat, FsError> {
        let inode = self.read_inode(inode_idx)?;
        let block_count = (inode.size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE;
        Ok(InodeStat {
            inode: inode_idx,
            file_type: inode.file_type,
            permissions: inode.permissions,
            uid: inode.uid,
            gid: inode.gid,
            size: inode.size,
            ctime: inode.ctime,
            mtime: inode.mtime,
            atime: inode.atime,
            block_count: block_count as u32,
        })
    }

    pub fn chmod(&mut self, inode_idx: u32, mode: u16) -> Result<(), FsError> {
        let mut inode = self.read_inode(inode_idx)?;
        inode.permissions = mode;
        inode.mtime = crate::drivers::rtc::read_seconds() as u32;
        self.write_inode(inode_idx, &inode)?;
        Ok(())
    }

    pub fn sync(&mut self) -> Result<(), FsError> {
        self.device.flush()
    }
}

impl RawDirEntry {
    pub fn new(inode: u32, file_type: u8, name: &str) -> Self {
        let mut entry = RawDirEntry {
            inode,
            file_type,
            name_len: name.len().min(58) as u8,
            name: [0u8; 58],
        };
        let len = entry.name_len as usize;
        entry.name[..len].copy_from_slice(&name.as_bytes()[..len]);
        entry
    }

    pub fn read_from_bytes(bytes: &[u8]) -> Self {
        let inode = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let file_type = bytes[4];
        let name_len = bytes[5];
        let mut name = [0u8; 58];
        name.copy_from_slice(&bytes[6..64]);
        RawDirEntry {
            inode,
            file_type,
            name_len,
            name,
        }
    }

    pub fn write_to_bytes(&self, bytes: &mut [u8]) {
        bytes[0..4].copy_from_slice(&self.inode.to_le_bytes());
        bytes[4] = self.file_type;
        bytes[5] = self.name_len;
        bytes[6..64].copy_from_slice(&self.name);
    }
}

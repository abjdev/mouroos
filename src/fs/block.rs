use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

pub const BLOCK_SIZE: usize = 1024;
pub const SECTOR_SIZE: usize = 512;
pub const SECTORS_PER_BLOCK: u16 = (BLOCK_SIZE / SECTOR_SIZE) as u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    IoError,
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    NoSpace,
    NoInode,
    InvalidPath,
    InvalidArgument,
    PermissionDenied,
    Corrupted,
    DeviceBusy,
}

pub trait BlockDevice: Send + Sync {
    fn read_block(&mut self, block_idx: u64, buf: &mut [u8]) -> Result<(), FsError>;
    fn write_block(&mut self, block_idx: u64, buf: &[u8]) -> Result<(), FsError>;
    fn block_size(&self) -> usize;
    fn block_count(&self) -> u64;
    fn flush(&mut self) -> Result<(), FsError>;
}

/// ATA Hard Drive Block Device (1024-byte blocks mapped to two 512-byte LBA sectors)
pub struct AtaBlockDevice {
    total_blocks: u64,
}

impl AtaBlockDevice {
    pub fn new() -> Option<Self> {
        let ata = crate::drivers::ata::ATA_PRIMARY_MASTER.lock();
        if !ata.is_present {
            return None;
        }
        let sector_count = ata.info.as_ref().map(|i| i.sector_count).unwrap_or(0);
        if sector_count < SECTORS_PER_BLOCK as u64 {
            return None;
        }
        let total_blocks = sector_count / SECTORS_PER_BLOCK as u64;
        Some(Self { total_blocks })
    }
}

impl BlockDevice for AtaBlockDevice {
    fn read_block(&mut self, block_idx: u64, buf: &mut [u8]) -> Result<(), FsError> {
        if block_idx >= self.total_blocks || buf.len() < BLOCK_SIZE {
            return Err(FsError::InvalidArgument);
        }
        let lba = block_idx * SECTORS_PER_BLOCK as u64;
        let mut ata = crate::drivers::ata::ATA_PRIMARY_MASTER.lock();
        ata.read_sectors(lba, SECTORS_PER_BLOCK, &mut buf[..BLOCK_SIZE])
            .map_err(|_| FsError::IoError)
    }

    fn write_block(&mut self, block_idx: u64, buf: &[u8]) -> Result<(), FsError> {
        if block_idx >= self.total_blocks || buf.len() < BLOCK_SIZE {
            return Err(FsError::InvalidArgument);
        }
        let lba = block_idx * SECTORS_PER_BLOCK as u64;
        let mut ata = crate::drivers::ata::ATA_PRIMARY_MASTER.lock();
        ata.write_sectors(lba, SECTORS_PER_BLOCK, &buf[..BLOCK_SIZE])
            .map_err(|_| FsError::IoError)
    }

    fn block_size(&self) -> usize {
        BLOCK_SIZE
    }

    fn block_count(&self) -> u64 {
        self.total_blocks
    }

    fn flush(&mut self) -> Result<(), FsError> {
        let mut ata = crate::drivers::ata::ATA_PRIMARY_MASTER.lock();
        ata.flush_cache().map_err(|_| FsError::IoError)
    }
}

/// In-Memory RAM Disk Block Device (Fallback when no ATA drive is attached)
pub struct RamBlockDevice {
    data: Vec<u8>,
    total_blocks: u64,
}

impl RamBlockDevice {
    pub fn new(block_count: usize) -> Self {
        let size = block_count * BLOCK_SIZE;
        Self {
            data: vec![0u8; size],
            total_blocks: block_count as u64,
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn read_block(&mut self, block_idx: u64, buf: &mut [u8]) -> Result<(), FsError> {
        if block_idx >= self.total_blocks || buf.len() < BLOCK_SIZE {
            return Err(FsError::InvalidArgument);
        }
        let start = (block_idx as usize) * BLOCK_SIZE;
        let end = start + BLOCK_SIZE;
        buf[..BLOCK_SIZE].copy_from_slice(&self.data[start..end]);
        Ok(())
    }

    fn write_block(&mut self, block_idx: u64, buf: &[u8]) -> Result<(), FsError> {
        if block_idx >= self.total_blocks || buf.len() < BLOCK_SIZE {
            return Err(FsError::InvalidArgument);
        }
        let start = (block_idx as usize) * BLOCK_SIZE;
        let end = start + BLOCK_SIZE;
        self.data[start..end].copy_from_slice(&buf[..BLOCK_SIZE]);
        Ok(())
    }

    fn block_size(&self) -> usize {
        BLOCK_SIZE
    }

    fn block_count(&self) -> u64 {
        self.total_blocks
    }

    fn flush(&mut self) -> Result<(), FsError> {
        Ok(())
    }
}

/// Block Cache to buffer I/O and reduce sector writes
const CACHE_SIZE: usize = 64;

struct CacheEntry {
    block_idx: u64,
    dirty: bool,
    valid: bool,
    data: [u8; BLOCK_SIZE],
}

pub struct CachedBlockDevice {
    device: Box<dyn BlockDevice>,
    cache: Vec<CacheEntry>,
    next_evict: usize,
}

impl CachedBlockDevice {
    pub fn new(device: Box<dyn BlockDevice>) -> Self {
        let mut cache = Vec::with_capacity(CACHE_SIZE);
        for _ in 0..CACHE_SIZE {
            cache.push(CacheEntry {
                block_idx: 0,
                dirty: false,
                valid: false,
                data: [0u8; BLOCK_SIZE],
            });
        }
        Self {
            device,
            cache,
            next_evict: 0,
        }
    }

    pub fn block_size(&self) -> usize {
        self.device.block_size()
    }

    pub fn block_count(&self) -> u64 {
        self.device.block_count()
    }

    pub fn read_block(&mut self, block_idx: u64, buf: &mut [u8]) -> Result<(), FsError> {
        // Check cache hit
        for entry in &self.cache {
            if entry.valid && entry.block_idx == block_idx {
                buf[..BLOCK_SIZE].copy_from_slice(&entry.data);
                return Ok(());
            }
        }

        // Cache miss: find slot to load
        let slot = self.find_or_evict_slot(block_idx)?;
        self.device.read_block(block_idx, &mut self.cache[slot].data)?;
        self.cache[slot].block_idx = block_idx;
        self.cache[slot].valid = true;
        self.cache[slot].dirty = false;
        buf[..BLOCK_SIZE].copy_from_slice(&self.cache[slot].data);
        Ok(())
    }

    pub fn write_block(&mut self, block_idx: u64, buf: &[u8]) -> Result<(), FsError> {
        // If already in cache, update and mark dirty
        for entry in &mut self.cache {
            if entry.valid && entry.block_idx == block_idx {
                entry.data.copy_from_slice(&buf[..BLOCK_SIZE]);
                entry.dirty = true;
                return Ok(());
            }
        }

        // Not in cache: get slot
        let slot = self.find_or_evict_slot(block_idx)?;
        self.cache[slot].block_idx = block_idx;
        self.cache[slot].data.copy_from_slice(&buf[..BLOCK_SIZE]);
        self.cache[slot].valid = true;
        self.cache[slot].dirty = true;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), FsError> {
        for entry in &mut self.cache {
            if entry.valid && entry.dirty {
                self.device.write_block(entry.block_idx, &entry.data)?;
                entry.dirty = false;
            }
        }
        self.device.flush()
    }

    fn find_or_evict_slot(&mut self, _new_block_idx: u64) -> Result<usize, FsError> {
        // 1. First look for an invalid slot
        for (i, entry) in self.cache.iter().enumerate() {
            if !entry.valid {
                return Ok(i);
            }
        }

        // 2. Otherwise evict in round-robin fashion
        let slot = self.next_evict;
        self.next_evict = (self.next_evict + 1) % CACHE_SIZE;

        if self.cache[slot].valid && self.cache[slot].dirty {
            let b_idx = self.cache[slot].block_idx;
            self.device.write_block(b_idx, &self.cache[slot].data)?;
            self.cache[slot].dirty = false;
        }

        self.cache[slot].valid = false;
        Ok(slot)
    }
}

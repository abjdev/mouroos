use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

use super::block::{AtaBlockDevice, CachedBlockDevice, FsError, RamBlockDevice};
use super::mouros_fs::{
    DirEntryInfo, FILE_TYPE_DIR, FILE_TYPE_REGULAR, InodeStat, MourosFileSystem, ROOT_INODE,
};

static HELLO_ELF: &[u8] = include_bytes!("../../samples/hello.elf");
static FIBONACCI_ELF: &[u8] = include_bytes!("../../samples/fibonacci.elf");
static MANDELBROT_ELF: &[u8] = include_bytes!("../../samples/mandelbrot.elf");
static SYSBENCH_ELF: &[u8] = include_bytes!("../../samples/sysbench.elf");

pub struct VirtualFileSystem {
    fs: MourosFileSystem,
}

pub static VFS: Mutex<Option<VirtualFileSystem>> = Mutex::new(None);

impl VirtualFileSystem {
    pub fn new(fs: MourosFileSystem) -> Self {
        Self { fs }
    }

    /// Resolve absolute or relative path to an inode
    pub fn resolve_path(&mut self, path: &str) -> Result<u32, FsError> {
        let clean = canonicalize_path(path);
        if clean == "/" {
            return Ok(ROOT_INODE);
        }

        let mut current_inode = ROOT_INODE;
        let parts: Vec<&str> = clean.split('/').filter(|p| !p.is_empty()).collect();

        for part in parts {
            current_inode = self.fs.lookup(current_inode, part)?;
        }
        Ok(current_inode)
    }

    /// Resolve parent directory inode and child name
    pub fn resolve_parent_and_name(&mut self, path: &str) -> Result<(u32, String), FsError> {
        let clean = canonicalize_path(path);
        if clean == "/" {
            return Err(FsError::InvalidArgument);
        }

        let trimmed = clean.trim_end_matches('/');
        let (parent_path, name) = match trimmed.rfind('/') {
            Some(idx) => {
                let p = if idx == 0 { "/" } else { &trimmed[..idx] };
                let n = &trimmed[idx + 1..];
                (p, String::from(n))
            }
            None => ("/", String::from(trimmed)),
        };

        let parent_inode = self.resolve_path(parent_path)?;
        Ok((parent_inode, name))
    }

    pub fn read_file_all(&mut self, path: &str) -> Result<Vec<u8>, FsError> {
        let inode_idx = self.resolve_path(path)?;
        let stat = self.fs.stat(inode_idx)?;
        if stat.file_type != FILE_TYPE_REGULAR && stat.file_type != super::mouros_fs::FILE_TYPE_CHARDEV {
            return Err(FsError::IsADirectory);
        }

        let mut data = vec![0u8; stat.size as usize];
        if stat.size > 0 {
            self.fs.read_file(inode_idx, 0, &mut data)?;
        }
        Ok(data)
    }

    pub fn write_file_all(&mut self, path: &str, data: &[u8]) -> Result<(), FsError> {
        let inode_idx = match self.resolve_path(path) {
            Ok(idx) => {
                self.fs.truncate_file(idx, 0)?;
                idx
            }
            Err(FsError::NotFound) => {
                let (parent_inode, name) = self.resolve_parent_and_name(path)?;
                self.fs.create_file_entry(parent_inode, &name, 0o644)?
            }
            Err(e) => return Err(e),
        };

        if !data.is_empty() {
            self.fs.write_file(inode_idx, 0, data)?;
        }
        self.fs.sync()
    }

    pub fn append_file(&mut self, path: &str, data: &[u8]) -> Result<(), FsError> {
        let inode_idx = match self.resolve_path(path) {
            Ok(idx) => idx,
            Err(FsError::NotFound) => {
                let (parent_inode, name) = self.resolve_parent_and_name(path)?;
                self.fs.create_file_entry(parent_inode, &name, 0o644)?
            }
            Err(e) => return Err(e),
        };

        let stat = self.fs.stat(inode_idx)?;
        let cur_size = stat.size as usize;
        self.fs.write_file(inode_idx, cur_size, data)?;
        self.fs.sync()
    }

    pub fn create_file(&mut self, path: &str, mode: u16) -> Result<(), FsError> {
        if self.resolve_path(path).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        let (parent_inode, name) = self.resolve_parent_and_name(path)?;
        self.fs.create_file_entry(parent_inode, &name, mode)?;
        self.fs.sync()
    }

    pub fn mkdir(&mut self, path: &str, mode: u16) -> Result<(), FsError> {
        if self.resolve_path(path).is_ok() {
            return Err(FsError::AlreadyExists);
        }
        let (parent_inode, name) = self.resolve_parent_and_name(path)?;
        self.fs.create_dir_entry(parent_inode, &name, mode)?;
        self.fs.sync()
    }

    pub fn remove(&mut self, path: &str) -> Result<(), FsError> {
        let (parent_inode, name) = self.resolve_parent_and_name(path)?;
        self.fs.unlink_entry(parent_inode, &name)?;
        self.fs.sync()
    }

    pub fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), FsError> {
        let old_inode = self.resolve_path(old_path)?;
        let (old_parent, old_name) = self.resolve_parent_and_name(old_path)?;
        let (new_parent, new_name) = self.resolve_parent_and_name(new_path)?;

        let stat = self.fs.stat(old_inode)?;

        // If target already exists, remove it first
        if let Ok(_) = self.resolve_path(new_path) {
            self.remove(new_path)?;
        }

        if stat.file_type == FILE_TYPE_DIR {
            self.fs.create_dir_entry(new_parent, &new_name, stat.permissions)?;
            // Note: moving nested dirs can be recursively copied
        } else {
            let data = self.read_file_all(old_path)?;
            self.write_file_all(new_path, &data)?;
        }

        self.fs.unlink_entry(old_parent, &old_name)?;
        self.fs.sync()
    }

    pub fn copy(&mut self, src_path: &str, dst_path: &str) -> Result<(), FsError> {
        let data = self.read_file_all(src_path)?;
        self.write_file_all(dst_path, &data)
    }

    pub fn read_dir(&mut self, path: &str) -> Result<Vec<DirEntryInfo>, FsError> {
        let inode_idx = self.resolve_path(path)?;
        self.fs.list_dir(inode_idx)
    }

    pub fn stat(&mut self, path: &str) -> Result<InodeStat, FsError> {
        let inode_idx = self.resolve_path(path)?;
        self.fs.stat(inode_idx)
    }

    pub fn chmod(&mut self, path: &str, mode: u16) -> Result<(), FsError> {
        let inode_idx = self.resolve_path(path)?;
        self.fs.chmod(inode_idx, mode)?;
        self.fs.sync()
    }

    pub fn sync(&mut self) -> Result<(), FsError> {
        self.fs.sync()
    }

    pub fn disk_usage(&self) -> (u32, u32, u32, u32) {
        (
            self.fs.sb.total_blocks,
            self.fs.sb.free_blocks,
            self.fs.sb.total_inodes,
            self.fs.sb.free_inodes,
        )
    }
}

pub fn canonicalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        } else if part == ".." {
            parts.pop();
        } else {
            parts.push(part);
        }
    }
    if parts.is_empty() {
        String::from("/")
    } else {
        let mut res = String::new();
        for p in parts {
            res.push('/');
            res.push_str(p);
        }
        res
    }
}

pub fn resolve_relative_path(cwd: &str, path: &str) -> String {
    if path.starts_with('/') {
        canonicalize_path(path)
    } else {
        let combined = if cwd.ends_with('/') {
            format!("{}{}", cwd, path)
        } else {
            format!("{}/{}", cwd, path)
        };
        canonicalize_path(&combined)
    }
}

/// Initialize VFS and ensure proper Unix directory structure
pub fn init() {
    let cached_device = if let Some(ata) = AtaBlockDevice::new() {
        crate::serial_println!("[VFS] Initializing VFS on ATA Primary Master...");
        CachedBlockDevice::new(Box::new(ata))
    } else {
        crate::serial_println!("[VFS] No ATA disk found. Initializing 16MB RamBlockDevice fallback...");
        let ramdisk = RamBlockDevice::new(16 * 1024); // 16 MB (16384 blocks)
        CachedBlockDevice::new(Box::new(ramdisk))
    };

    let fs = match MourosFileSystem::open_or_format(cached_device) {
        Ok(f) => f,
        Err(e) => {
            crate::serial_println!("[VFS] Failed to mount or format filesystem: {:?}", e);
            return;
        }
    };

    let mut vfs = VirtualFileSystem::new(fs);
    populate_initial_directories(&mut vfs);

    *VFS.lock() = Some(vfs);
    crate::serial_println!("[VFS] Virtual File System initialized successfully!");
}

fn populate_initial_directories(vfs: &mut VirtualFileSystem) {
    let dirs = [
        "/bin",
        "/etc",
        "/home",
        "/home/user",
        "/tmp",
        "/dev",
        "/usr",
        "/usr/bin",
        "/usr/share",
        "/usr/games",
    ];

    for d in &dirs {
        if vfs.resolve_path(d).is_err() {
            let _ = vfs.mkdir(d, 0o755);
        }
    }

    // Default configuration in /etc
    if vfs.resolve_path("/etc/hostname").is_err() {
        let _ = vfs.write_file_all("/etc/hostname", b"mouros\n");
    }
    if vfs.resolve_path("/etc/version").is_err() {
        let _ = vfs.write_file_all("/etc/version", b"0.2.0\n");
    }
    if vfs.resolve_path("/etc/os-release").is_err() {
        let os_rel = b"NAME=\"Mouros Desktop OS\"\nVERSION=\"0.2.0\"\nID=mouros\nPRETTY_NAME=\"Mouros Desktop OS (x86_64)\"\n";
        let _ = vfs.write_file_all("/etc/os-release", os_rel);
    }
    if vfs.resolve_path("/etc/motd").is_err() {
        let motd = b"Welcome to Mouros Desktop OS (Bare-Metal x86_64)!\nType 'help' in terminal for available commands.\n";
        let _ = vfs.write_file_all("/etc/motd", motd);
    }
    if vfs.resolve_path("/etc/passwd").is_err() {
        let pwd = b"root:x:0:0:root:/home/root:/bin/sh\nuser:x:1000:1000:User:/home/user:/bin/sh\n";
        let _ = vfs.write_file_all("/etc/passwd", pwd);
    }

    // Default user files in /home/user
    if vfs.resolve_path("/home/user/readme.txt").is_err() {
        let readme = b"Welcome to your home directory in Mouros OS!\nFiles created here are persistently stored.\n";
        let _ = vfs.write_file_all("/home/user/readme.txt", readme);
    }
    if vfs.resolve_path("/home/user/notes.txt").is_err() {
        let notes = b"Mouros OS Notes\n---------------\n1. Persistent filesystem with ATA hard disk support\n2. Shell commands: ls, cd, cat, touch, rm, cp, mv, echo, stat, df\n3. GUI text editor and terminal integrated with VFS\n";
        let _ = vfs.write_file_all("/home/user/notes.txt", notes);
    }

    // Populate /bin with built-in ELF binaries
    if vfs.resolve_path("/bin/hello.elf").is_err() {
        let _ = vfs.write_file_all("/bin/hello.elf", HELLO_ELF);
        let _ = vfs.chmod("/bin/hello.elf", 0o755);
    }
    if vfs.resolve_path("/bin/fibonacci.elf").is_err() {
        let _ = vfs.write_file_all("/bin/fibonacci.elf", FIBONACCI_ELF);
        let _ = vfs.chmod("/bin/fibonacci.elf", 0o755);
    }
    if vfs.resolve_path("/bin/mandelbrot.elf").is_err() {
        let _ = vfs.write_file_all("/bin/mandelbrot.elf", MANDELBROT_ELF);
        let _ = vfs.chmod("/bin/mandelbrot.elf", 0o755);
    }
    if vfs.resolve_path("/bin/sysbench.elf").is_err() {
        let _ = vfs.write_file_all("/bin/sysbench.elf", SYSBENCH_ELF);
        let _ = vfs.chmod("/bin/sysbench.elf", 0o755);
    }

    let _ = vfs.sync();
}

// Global VFS Helper APIs

pub fn read(path: &str) -> Result<Vec<u8>, FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.read_file_all(path)
}

pub fn write(path: &str, data: &[u8]) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.write_file_all(path, data)
}

pub fn append(path: &str, data: &[u8]) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.append_file(path, data)
}

pub fn create_file(path: &str, mode: u16) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.create_file(path, mode)
}

pub fn mkdir(path: &str) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.mkdir(path, 0o755)
}

pub fn remove(path: &str) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.remove(path)
}

pub fn rename(src: &str, dst: &str) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.rename(src, dst)
}

pub fn copy(src: &str, dst: &str) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.copy(src, dst)
}

pub fn read_dir(path: &str) -> Result<Vec<DirEntryInfo>, FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.read_dir(path)
}

pub fn stat(path: &str) -> Result<InodeStat, FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.stat(path)
}

pub fn chmod(path: &str, mode: u16) -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.chmod(path, mode)
}

pub fn exists(path: &str) -> bool {
    let mut lock = VFS.lock();
    if let Some(vfs) = lock.as_mut() {
        vfs.resolve_path(path).is_ok()
    } else {
        false
    }
}

pub fn is_dir(path: &str) -> bool {
    let mut lock = VFS.lock();
    if let Some(vfs) = lock.as_mut() {
        if let Ok(inode) = vfs.resolve_path(path) {
            if let Ok(stat) = vfs.fs.stat(inode) {
                return stat.file_type == FILE_TYPE_DIR;
            }
        }
    }
    false
}

pub fn sync() -> Result<(), FsError> {
    let mut lock = VFS.lock();
    let vfs = lock.as_mut().ok_or(FsError::IoError)?;
    vfs.sync()
}

pub fn disk_usage() -> Result<(u32, u32, u32, u32), FsError> {
    let lock = VFS.lock();
    let vfs = lock.as_ref().ok_or(FsError::IoError)?;
    Ok(vfs.disk_usage())
}

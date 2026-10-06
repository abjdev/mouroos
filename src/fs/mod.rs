pub mod block;
pub mod mouros_fs;
pub mod vfs;

pub use block::FsError;
pub use mouros_fs::{DirEntryInfo, InodeStat};
pub use vfs::{
    append, chmod, copy, create_file, disk_usage, exists, is_dir, mkdir, read, read_dir, remove,
    rename, resolve_relative_path, stat, sync, write,
};

pub fn init() {
    vfs::init();
}

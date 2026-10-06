pub mod numbers;

pub use numbers::*;

use crate::process::{self, Pid};
use alloc::string::String;
use x86_64::instructions::port::Port;

pub fn init() {
    crate::serial_println!("[SYSCALL] Subsystem initialized (POSIX-like ABI dispatch ready).");
}

pub fn dispatch(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYS_EXIT => sys_exit(args[0] as i32),
        SYS_GETPID => sys_getpid(),
        SYS_GETPPID => sys_getppid(),
        SYS_YIELD => sys_yield(),
        SYS_SLEEP => sys_sleep(args[0] as u64),
        SYS_UPTIME => sys_uptime(),
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        SYS_OPEN => sys_open(args[0] as *const u8, args[1], args[2]),
        SYS_CLOSE => sys_close(args[0]),
        SYS_KILL => sys_kill(args[0] as u32, args[1] as i32),
        SYS_WAITPID => sys_waitpid(args[0] as u32),
        SYS_SYNC => sys_sync(),
        SYS_REBOOT => sys_reboot(),
        SYS_SHUTDOWN => sys_shutdown(),
        SYS_SPAWN => sys_spawn(args[0] as *const u8, args[1], args[2] as u8, args[3], args[4] != 0),
        _ => -38, // -ENOSYS
    }
}

fn sys_exit(code: i32) -> isize {
    let pid = process::current_pid();
    process::terminate(pid, code);
    process::yield_now();
    0
}

fn sys_getpid() -> isize {
    process::current_pid().0 as isize
}

fn sys_getppid() -> isize {
    let pid = process::current_pid();
    let table = process::PROCESS_TABLE.lock();
    if let Some(p) = table.get(pid) {
        p.ppid.map(|parent| parent.0 as isize).unwrap_or(0)
    } else {
        0
    }
}

fn sys_yield() -> isize {
    process::yield_now();
    0
}

fn sys_sleep(ticks: u64) -> isize {
    process::sleep_ticks(ticks);
    0
}

fn sys_uptime() -> isize {
    crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed) as isize
}

fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    if buf.is_null() || len == 0 || len > 1024 * 1024 {
        return -22; // -EINVAL
    }

    let slice = unsafe { core::slice::from_raw_parts(buf, len) };

    if fd == 1 || fd == 2 {
        // stdout or stderr
        let cur_pid = process::current_pid();
        let mut table = process::PROCESS_TABLE.lock();
        if let Some(p) = table.get_mut(cur_pid) {
            if let Some(buf_str) = &mut p.stdout_capture {
                if let Ok(s) = core::str::from_utf8(slice) {
                    buf_str.push_str(s);
                }
            } else {
                // Direct print to serial log if not capturing
                if let Ok(s) = core::str::from_utf8(slice) {
                    crate::serial_print!("{}", s);
                }
            }
        }
        len as isize
    } else {
        // Open file descriptor
        let cur_pid = process::current_pid();
        let mut table = process::PROCESS_TABLE.lock();
        if let Some(p) = table.get_mut(cur_pid) {
            if fd < p.fds.len() {
                if let Some(fdesc) = &mut p.fds[fd] {
                    if !fdesc.writable {
                        return -9; // -EBADF
                    }
                    let path = fdesc.path.clone();
                    drop(table);
                    match crate::fs::write(&path, slice) {
                        Ok(_) => len as isize,
                        Err(_) => -5, // -EIO
                    }
                } else {
                    -9
                }
            } else {
                -9
            }
        } else {
            -3
        }
    }
}

fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    if buf.is_null() || len == 0 || len > 1024 * 1024 {
        return -22;
    }

    let cur_pid = process::current_pid();
    let mut table = process::PROCESS_TABLE.lock();
    if let Some(p) = table.get_mut(cur_pid) {
        if fd < p.fds.len() {
            if let Some(fdesc) = &mut p.fds[fd] {
                if !fdesc.readable {
                    return -9;
                }
                let path = fdesc.path.clone();
                let offset = fdesc.offset;
                drop(table);
                match crate::fs::read(&path) {
                    Ok(data) => {
                        if offset >= data.len() {
                            0 // EOF
                        } else {
                            let available = &data[offset..];
                            let to_read = len.min(available.len());
                            unsafe {
                                core::ptr::copy_nonoverlapping(available.as_ptr(), buf, to_read);
                            }
                            let mut table = process::PROCESS_TABLE.lock();
                            if let Some(p) = table.get_mut(cur_pid) {
                                if let Some(Some(desc)) = p.fds.get_mut(fd) {
                                    desc.offset += to_read;
                                }
                            }
                            to_read as isize
                        }
                    }
                    Err(_) => -5,
                }
            } else {
                -9
            }
        } else {
            -9
        }
    } else {
        -3
    }
}

fn sys_open(path_ptr: *const u8, path_len: usize, flags: usize) -> isize {
    if path_ptr.is_null() || path_len == 0 || path_len > 256 {
        return -22;
    }
    let slice = unsafe { core::slice::from_raw_parts(path_ptr, path_len) };
    let path = match core::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return -22,
    };

    let writable = (flags & 1) != 0 || (flags & 2) != 0;
    let readable = (flags & 1) == 0;

    let cur_pid = process::current_pid();
    let mut table = process::PROCESS_TABLE.lock();
    if let Some(p) = table.get_mut(cur_pid) {
        let fd_idx = p.fds.len();
        p.fds.push(Some(process::FileDescriptor {
            path: String::from(path),
            inode: 0,
            offset: 0,
            readable,
            writable,
            is_pipe: false,
        }));
        fd_idx as isize
    } else {
        -3
    }
}

fn sys_close(fd: usize) -> isize {
    let cur_pid = process::current_pid();
    let mut table = process::PROCESS_TABLE.lock();
    if let Some(p) = table.get_mut(cur_pid) {
        if fd < p.fds.len() && p.fds[fd].is_some() {
            p.fds[fd] = None;
            0
        } else {
            -9
        }
    } else {
        -3
    }
}

fn sys_kill(pid: u32, signal: i32) -> isize {
    match process::kill(Pid(pid), signal) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

fn sys_waitpid(pid: u32) -> isize {
    match process::waitpid(Pid(pid)) {
        Some(code) => code as isize,
        None => -1,
    }
}

fn sys_sync() -> isize {
    match crate::fs::sync() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

fn sys_reboot() -> isize {
    unsafe {
        let mut port = Port::new(0x64);
        for _ in 0..10_000 {
            let status: u8 = port.read();
            if status & 0x02 == 0 {
                break;
            }
        }
        port.write(0xFEu8);
    }
    0
}

fn sys_shutdown() -> isize {
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(0x10u32);
        let mut acpi_port = Port::new(0x604);
        acpi_port.write(0x2000u16);
    }
    0
}

fn sys_spawn(name_ptr: *const u8, name_len: usize, priority: u8, memory_bytes: usize, bg: bool) -> isize {
    if name_ptr.is_null() || name_len == 0 || name_len > 64 {
        return -22;
    }
    let slice = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
    let name = match core::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return -22,
    };

    let ppid = Some(process::current_pid());
    let pid = process::spawn(name, ppid, priority, memory_bytes, bg);
    pid.0 as isize
}

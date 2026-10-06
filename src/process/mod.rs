pub mod scheduler;
pub mod table;

pub use scheduler::SCHEDULER;
pub use table::{FileDescriptor, Pid, ProcessDescriptor, ProcessInfo, ProcessState, PROCESS_TABLE};

use alloc::vec::Vec;

pub fn init() {
    PROCESS_TABLE.lock().init_defaults();
    crate::serial_println!("[PROCESS] Process subsystem initialized with core system tasks (PIDs 0-4).");
}

pub fn spawn(
    name: &str,
    ppid: Option<Pid>,
    priority: u8,
    memory_bytes: usize,
    background: bool,
) -> Pid {
    PROCESS_TABLE.lock().spawn(name, ppid, priority, memory_bytes, background)
}

pub fn terminate(pid: Pid, exit_code: i32) {
    PROCESS_TABLE.lock().terminate(pid, exit_code);
}

pub fn kill(pid: Pid, signal: i32) -> Result<(), &'static str> {
    PROCESS_TABLE.lock().kill(pid, signal)
}

pub fn waitpid(pid: Pid) -> Option<i32> {
    PROCESS_TABLE.lock().waitpid(pid)
}

pub fn is_terminated(pid: Pid) -> bool {
    PROCESS_TABLE.lock().is_terminated(pid)
}

pub fn list_processes() -> Vec<ProcessInfo> {
    PROCESS_TABLE.lock().list()
}

pub fn current_pid() -> Pid {
    PROCESS_TABLE.lock().get_current_pid()
}

pub fn schedule_tick(current_tick: u64) {
    SCHEDULER.lock().on_tick(current_tick);
}

pub fn yield_now() {
    SCHEDULER.lock().yield_now();
}

pub fn sleep_ticks(ticks: u64) {
    SCHEDULER.lock().sleep_ticks(ticks);
}

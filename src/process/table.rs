use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(pub u32);

impl Pid {
    pub fn new() -> Self {
        static NEXT_PID: AtomicU32 = AtomicU32::new(5); // 0-4 reserved for kernel & system
        Pid(NEXT_PID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked { until_tick: u64 },
    Terminated { exit_code: i32 },
    Zombie,
}

impl ProcessState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProcessState::Ready => "READY",
            ProcessState::Running => "RUNNING",
            ProcessState::Blocked { .. } => "BLOCKED",
            ProcessState::Terminated { .. } => "TERMINATED",
            ProcessState::Zombie => "ZOMBIE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub path: String,
    pub inode: u32,
    pub offset: usize,
    pub readable: bool,
    pub writable: bool,
    pub is_pipe: bool,
}

#[derive(Debug, Clone)]
pub struct ProcessDescriptor {
    pub pid: Pid,
    pub ppid: Option<Pid>,
    pub name: String,
    pub state: ProcessState,
    pub priority: u8,
    pub cpu_ticks: u64,
    pub start_tick: u64,
    pub memory_bytes: usize,
    pub exit_code: Option<i32>,
    pub fds: Vec<Option<FileDescriptor>>,
    pub stdout_capture: Option<String>,
    pub background: bool,
}

impl ProcessDescriptor {
    pub fn new(
        pid: Pid,
        ppid: Option<Pid>,
        name: &str,
        priority: u8,
        memory_bytes: usize,
        background: bool,
    ) -> Self {
        let current_tick = crate::interrupts::TICKS.load(Ordering::Relaxed);
        let mut fds = Vec::new();
        // 0: stdin, 1: stdout, 2: stderr
        fds.push(Some(FileDescriptor {
            path: String::from("/dev/stdin"),
            inode: 0,
            offset: 0,
            readable: true,
            writable: false,
            is_pipe: false,
        }));
        fds.push(Some(FileDescriptor {
            path: String::from("/dev/stdout"),
            inode: 0,
            offset: 0,
            readable: false,
            writable: true,
            is_pipe: false,
        }));
        fds.push(Some(FileDescriptor {
            path: String::from("/dev/stderr"),
            inode: 0,
            offset: 0,
            readable: false,
            writable: true,
            is_pipe: false,
        }));

        Self {
            pid,
            ppid,
            name: String::from(name),
            state: ProcessState::Ready,
            priority,
            cpu_ticks: 0,
            start_tick: current_tick,
            memory_bytes,
            exit_code: None,
            fds,
            stdout_capture: if background { Some(String::new()) } else { None },
            background,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub state_str: &'static str,
    pub priority: u8,
    pub cpu_ticks: u64,
    pub memory_bytes: usize,
    pub exit_code: Option<i32>,
    pub background: bool,
}

pub struct ProcessTable {
    processes: Vec<ProcessDescriptor>,
    current_running_pid: Pid,
}

pub static PROCESS_TABLE: Mutex<ProcessTable> = Mutex::new(ProcessTable::new());

impl ProcessTable {
    pub const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current_running_pid: Pid(0),
        }
    }

    pub fn init_defaults(&mut self) {
        self.processes.clear();

        // PID 0: Kernel Idle
        let mut p0 = ProcessDescriptor::new(Pid(0), None, "kernel/idle", 20, 64 * 1024, false);
        p0.state = ProcessState::Running;
        self.processes.push(p0);

        // PID 1: System Init
        let p1 = ProcessDescriptor::new(Pid(1), Some(Pid(0)), "system/init", 10, 256 * 1024, false);
        self.processes.push(p1);

        // PID 2: Compositor / GUI
        let p2 = ProcessDescriptor::new(Pid(2), Some(Pid(1)), "compositor/gui", 5, 1024 * 1024, false);
        self.processes.push(p2);

        // PID 3: Audio Subsystem
        let p3 = ProcessDescriptor::new(Pid(3), Some(Pid(1)), "drivers/audio", 8, 512 * 1024, false);
        self.processes.push(p3);

        // PID 4: Command Shell
        let p4 = ProcessDescriptor::new(Pid(4), Some(Pid(1)), "shell/term", 10, 512 * 1024, false);
        self.processes.push(p4);

        self.current_running_pid = Pid(4);
    }

    pub fn spawn(
        &mut self,
        name: &str,
        ppid: Option<Pid>,
        priority: u8,
        memory_bytes: usize,
        background: bool,
    ) -> Pid {
        let pid = Pid::new();
        let desc = ProcessDescriptor::new(pid, ppid, name, priority, memory_bytes, background);
        self.processes.push(desc);
        pid
    }

    pub fn get(&self, pid: Pid) -> Option<&ProcessDescriptor> {
        self.processes.iter().find(|p| p.pid == pid)
    }

    pub fn get_mut(&mut self, pid: Pid) -> Option<&mut ProcessDescriptor> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }

    pub fn set_state(&mut self, pid: Pid, state: ProcessState) {
        if let Some(p) = self.get_mut(pid) {
            p.state = state;
        }
    }

    pub fn terminate(&mut self, pid: Pid, exit_code: i32) {
        if let Some(p) = self.get_mut(pid) {
            p.state = ProcessState::Terminated { exit_code };
            p.exit_code = Some(exit_code);
        }
    }

    pub fn kill(&mut self, pid: Pid, signal: i32) -> Result<(), &'static str> {
        if pid.0 <= 4 {
            return Err("Cannot terminate core system process");
        }
        if let Some(p) = self.get_mut(pid) {
            match signal {
                9 | 15 => {
                    // SIGKILL / SIGTERM
                    p.state = ProcessState::Terminated { exit_code: 128 + signal };
                    p.exit_code = Some(128 + signal);
                    Ok(())
                }
                19 => {
                    // SIGSTOP
                    p.state = ProcessState::Blocked { until_tick: u64::MAX };
                    Ok(())
                }
                18 => {
                    // SIGCONT
                    p.state = ProcessState::Ready;
                    Ok(())
                }
                _ => Err("Unsupported signal"),
            }
        } else {
            Err("No such process")
        }
    }

    pub fn waitpid(&mut self, pid: Pid) -> Option<i32> {
        if let Some(pos) = self.processes.iter().position(|p| p.pid == pid) {
            if let ProcessState::Terminated { exit_code } = self.processes[pos].state {
                // Reap terminated process
                let code = exit_code;
                self.processes.remove(pos);
                return Some(code);
            }
        }
        None
    }

    pub fn is_terminated(&self, pid: Pid) -> bool {
        if let Some(p) = self.get(pid) {
            matches!(p.state, ProcessState::Terminated { .. })
        } else {
            true
        }
    }

    pub fn get_current_pid(&self) -> Pid {
        self.current_running_pid
    }

    pub fn set_current_pid(&mut self, pid: Pid) {
        self.current_running_pid = pid;
    }

    pub fn list(&self) -> Vec<ProcessInfo> {
        self.processes
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid.0,
                ppid: p.ppid.map(|parent| parent.0).unwrap_or(0),
                name: p.name.clone(),
                state_str: p.state.as_str(),
                priority: p.priority,
                cpu_ticks: p.cpu_ticks,
                memory_bytes: p.memory_bytes,
                exit_code: p.exit_code,
                background: p.background,
            })
            .collect()
    }

    pub fn reap_zombies(&mut self) {
        self.processes.retain(|p| {
            if let ProcessState::Terminated { .. } = p.state {
                // Keep if someone might wait on it, unless it's background and done
                !p.background
            } else {
                true
            }
        });
    }
}

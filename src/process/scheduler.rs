use super::table::{Pid, ProcessState, PROCESS_TABLE};
use spin::Mutex;

pub struct Scheduler {
    quantum_remaining: u32,
    quantum_default: u32,
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            quantum_remaining: 10,
            quantum_default: 10,
        }
    }

    pub fn on_tick(&mut self, current_tick: u64) {
        let mut table = PROCESS_TABLE.lock();

        // 1. Unblock sleeping processes whose tick has arrived
        let list_len = table.list().len();
        for i in 0..list_len {
            let pid = {
                let procs = table.list();
                if i < procs.len() {
                    Pid(procs[i].pid)
                } else {
                    break;
                }
            };

            if let Some(p) = table.get_mut(pid) {
                if let ProcessState::Blocked { until_tick } = p.state {
                    if current_tick >= until_tick {
                        p.state = ProcessState::Ready;
                    }
                }
            }
        }

        // 2. Charge CPU time to currently running process
        let cur_pid = table.get_current_pid();
        if let Some(p) = table.get_mut(cur_pid) {
            p.cpu_ticks += 1;
        }

        // 3. Time quantum check for round-robin preemption
        if self.quantum_remaining > 0 {
            self.quantum_remaining -= 1;
        } else {
            self.quantum_remaining = self.quantum_default;
            // Cycle current running process if still Running
            if let Some(p) = table.get_mut(cur_pid) {
                if p.state == ProcessState::Running {
                    p.state = ProcessState::Ready;
                }
            }

            // Pick next ready process
            let procs = table.list();
            let mut next_pid = None;
            for p in &procs {
                if p.state_str == "READY" && p.pid != cur_pid.0 {
                    next_pid = Some(Pid(p.pid));
                    break;
                }
            }

            if let Some(next) = next_pid {
                if let Some(p) = table.get_mut(next) {
                    p.state = ProcessState::Running;
                    table.set_current_pid(next);
                }
            }
        }
    }

    pub fn yield_now(&mut self) {
        let mut table = PROCESS_TABLE.lock();
        let cur_pid = table.get_current_pid();

        if let Some(p) = table.get_mut(cur_pid) {
            if p.state == ProcessState::Running {
                p.state = ProcessState::Ready;
            }
        }

        let procs = table.list();
        let mut next_pid = None;
        for p in &procs {
            if p.state_str == "READY" {
                next_pid = Some(Pid(p.pid));
                break;
            }
        }

        if let Some(next) = next_pid {
            if let Some(p) = table.get_mut(next) {
                p.state = ProcessState::Running;
                table.set_current_pid(next);
            }
        }

        self.quantum_remaining = self.quantum_default;
    }

    pub fn sleep_ticks(&mut self, ticks: u64) {
        let current_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
        let until = current_tick + ticks;
        let mut table = PROCESS_TABLE.lock();
        let cur_pid = table.get_current_pid();

        if let Some(p) = table.get_mut(cur_pid) {
            p.state = ProcessState::Blocked { until_tick: until };
        }

        drop(table);
        self.yield_now();
    }
}

use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static IS_MUTED: AtomicBool = AtomicBool::new(false);
static CURRENT_FREQ: AtomicU32 = AtomicU32::new(0);

/// Play tone at given frequency in Hz (e.g. 440 for A4)
pub fn play_tone(freq: u32) {
    if freq == 0 || IS_MUTED.load(Ordering::Relaxed) {
        mute();
        return;
    }

    CURRENT_FREQ.store(freq, Ordering::Relaxed);

    unsafe {
        let divisor = (1193180 / freq) as u16;

        let mut cmd_port = Port::<u8>::new(0x43);
        let mut data_port = Port::<u8>::new(0x42);
        let mut ctrl_port = Port::<u8>::new(0x61);

        // PIT Channel 2: Mode 3 (Square Wave), LSB then MSB
        cmd_port.write(0xB6);
        data_port.write((divisor & 0xFF) as u8);
        data_port.write(((divisor >> 8) & 0xFF) as u8);

        // Turn on speaker output (bits 0 and 1 of port 0x61)
        let tmp = ctrl_port.read();
        if tmp & 3 != 3 {
            ctrl_port.write(tmp | 3);
        }
    }
}

/// Mute the PC speaker
pub fn mute() {
    CURRENT_FREQ.store(0, Ordering::Relaxed);
    unsafe {
        let mut ctrl_port = Port::<u8>::new(0x61);
        let tmp = ctrl_port.read();
        ctrl_port.write(tmp & 0xFC);
    }
}

pub fn get_current_frequency() -> u32 {
    CURRENT_FREQ.load(Ordering::Relaxed)
}

pub fn set_muted(muted: bool) {
    IS_MUTED.store(muted, Ordering::Relaxed);
    if muted {
        mute();
    }
}

pub fn is_muted() -> bool {
    IS_MUTED.load(Ordering::Relaxed)
}

pub fn stop_tone() {
    mute();
}

pub fn beep(freq: u32, _duration_ticks: u32) {
    play_tone(freq);
}

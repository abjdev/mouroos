use conquer_once::spin::OnceCell;
use core::sync::atomic::{AtomicIsize, AtomicU8, Ordering};
use crossbeam_queue::ArrayQueue;
use x86_64::instructions::port::Port;

const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const PS2_COMMAND_PORT: u16 = 0x64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseState {
    pub x: isize,
    pub y: isize,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub middle_pressed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub x: isize,
    pub y: isize,
    pub dx: i32,
    pub dy: i32,
    pub left_pressed: bool,
    pub right_pressed: bool,
}

static MOUSE_X: AtomicIsize = AtomicIsize::new(400);
static MOUSE_Y: AtomicIsize = AtomicIsize::new(300);
static MOUSE_BUTTONS: AtomicU8 = AtomicU8::new(0);
static SCREEN_W: AtomicIsize = AtomicIsize::new(800);
static SCREEN_H: AtomicIsize = AtomicIsize::new(600);

static MOUSE_EVENTS: OnceCell<ArrayQueue<MouseEvent>> = OnceCell::uninit();

// Internal state for 3-byte packet assembly in interrupt handler
struct PacketParser {
    cycle: u8,
    packet: [u8; 3],
}

static PARSER: spin::Mutex<PacketParser> = spin::Mutex::new(PacketParser {
    cycle: 0,
    packet: [0; 3],
});

pub fn set_screen_bounds(width: isize, height: isize) {
    SCREEN_W.store(width, Ordering::Relaxed);
    SCREEN_H.store(height, Ordering::Relaxed);
    MOUSE_X.store(width / 2, Ordering::Relaxed);
    MOUSE_Y.store(height / 2, Ordering::Relaxed);
}

pub fn get_mouse_state() -> MouseState {
    let x = MOUSE_X.load(Ordering::Relaxed);
    let y = MOUSE_Y.load(Ordering::Relaxed);
    let btn = MOUSE_BUTTONS.load(Ordering::Relaxed);
    MouseState {
        x,
        y,
        left_pressed: btn & 0x01 != 0,
        right_pressed: btn & 0x02 != 0,
        middle_pressed: btn & 0x04 != 0,
    }
}

pub fn pop_mouse_event() -> Option<MouseEvent> {
    if let Ok(queue) = MOUSE_EVENTS.try_get() {
        queue.pop()
    } else {
        None
    }
}

pub fn has_events() -> bool {
    if let Ok(queue) = MOUSE_EVENTS.try_get() {
        !queue.is_empty()
    } else {
        false
    }
}

unsafe fn wait_write() {
    let mut port = Port::<u8>::new(PS2_STATUS_PORT);
    for _ in 0..100_000 {
        let status: u8 = unsafe { port.read() };
        if status & 0x02 == 0 {
            return;
        }
    }
}

unsafe fn wait_read() {
    let mut port = Port::<u8>::new(PS2_STATUS_PORT);
    for _ in 0..100_000 {
        let status: u8 = unsafe { port.read() };
        if status & 0x01 != 0 {
            return;
        }
    }
}

unsafe fn mouse_write(byte: u8) {
    unsafe {
        wait_write();
        Port::<u8>::new(PS2_COMMAND_PORT).write(0xD4u8);
        wait_write();
        Port::<u8>::new(PS2_DATA_PORT).write(byte);
    }
}

unsafe fn mouse_read() -> u8 {
    unsafe {
        wait_read();
        Port::<u8>::new(PS2_DATA_PORT).read()
    }
}

pub fn init(screen_width: isize, screen_height: isize) {
    set_screen_bounds(screen_width, screen_height);
    MOUSE_EVENTS.try_init_once(|| ArrayQueue::new(1024)).ok();

    unsafe {
        // Enable auxiliary device (mouse)
        wait_write();
        Port::<u8>::new(PS2_COMMAND_PORT).write(0xA8u8);

        // Read Compaq Status Byte
        wait_write();
        Port::<u8>::new(PS2_COMMAND_PORT).write(0x20u8);
        wait_read();
        let mut status: u8 = Port::<u8>::new(PS2_DATA_PORT).read();

        // Enable IRQ12 (bit 1) and disable mouse clock disable (bit 5)
        status |= 0x02;
        status &= !0x20;

        // Write Compaq Status Byte
        wait_write();
        Port::<u8>::new(PS2_COMMAND_PORT).write(0x60u8);
        wait_write();
        Port::<u8>::new(PS2_DATA_PORT).write(status);

        // Set mouse default settings
        mouse_write(0xF6u8);
        let _ = mouse_read(); // ACK (0xFA)

        // Enable data streaming
        mouse_write(0xF4u8);
        let _ = mouse_read(); // ACK (0xFA)
    }
}

/// Called from the PS/2 mouse interrupt handler (IRQ 12).
pub fn process_packet_byte(byte: u8) {
    let mut parser = PARSER.lock();

    match parser.cycle {
        0 => {
            // Bit 3 of byte 0 is always 1 in a valid PS/2 packet
            if byte & 0x08 != 0 {
                parser.packet[0] = byte;
                parser.cycle = 1;
            }
        }
        1 => {
            parser.packet[1] = byte;
            parser.cycle = 2;
        }
        2 => {
            parser.packet[2] = byte;
            parser.cycle = 0;

            let flags = parser.packet[0];
            let raw_x = parser.packet[1];
            let raw_y = parser.packet[2];

            let dx = if flags & 0x40 != 0 {
                0
            } else {
                raw_x as i8 as i32
            };

            let dy = if flags & 0x80 != 0 {
                0
            } else {
                raw_y as i8 as i32
            };

            // Invert dy for screen coordinates (PS/2 y is positive upwards)
            let dy_screen = -dy;

            let screen_w = SCREEN_W.load(Ordering::Relaxed);
            let screen_h = SCREEN_H.load(Ordering::Relaxed);

            let cur_x = MOUSE_X.load(Ordering::Relaxed);
            let cur_y = MOUSE_Y.load(Ordering::Relaxed);

            let new_x = (cur_x + dx as isize).clamp(0, screen_w - 1);
            let new_y = (cur_y + dy_screen as isize).clamp(0, screen_h - 1);

            let left_pressed = flags & 0x01 != 0;
            let right_pressed = flags & 0x02 != 0;
            let middle_pressed = flags & 0x04 != 0;

            let mut btn = 0u8;
            if left_pressed {
                btn |= 0x01;
            }
            if right_pressed {
                btn |= 0x02;
            }
            if middle_pressed {
                btn |= 0x04;
            }

            MOUSE_X.store(new_x, Ordering::Relaxed);
            MOUSE_Y.store(new_y, Ordering::Relaxed);
            MOUSE_BUTTONS.store(btn, Ordering::Relaxed);

            let event = MouseEvent {
                x: new_x,
                y: new_y,
                dx,
                dy: dy_screen,
                left_pressed,
                right_pressed,
            };

            if let Ok(queue) = MOUSE_EVENTS.try_get() {
                let _ = queue.push(event);
            }
        }
        _ => {
            parser.cycle = 0;
        }
    }
}

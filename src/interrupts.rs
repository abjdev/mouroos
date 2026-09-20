use crate::{gdt, hlt_loop, println};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use core::sync::atomic::{AtomicU64, Ordering};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static TICKS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Mouse = PIC_2_OFFSET + 4,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.divide_error.set_handler_fn(divide_error_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.as_u8()].set_handler_fn(mouse_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    crate::serial_println!("EXCEPTION: PAGE FAULT");
    crate::serial_println!("Accessed Address: {:?}", Cr2::read());
    crate::serial_println!("Error Code: {:?}", error_code);
    crate::serial_println!("{:#?}", stack_frame);
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    crate::serial_println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    crate::serial_println!("EXCEPTION: GENERAL PROTECTION FAULT");
    crate::serial_println!("Error Code: {:?}", error_code);
    crate::serial_println!("{:#?}", stack_frame);
    panic!("EXCEPTION: GENERAL PROTECTION FAULT");
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
    panic!("EXCEPTION: INVALID OPCODE");
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("EXCEPTION: DIVIDE ERROR\n{:#?}", stack_frame);
    panic!("EXCEPTION: DIVIDE ERROR");
}

pub fn init_pit(hz: u32) {
    let divisor = (1193182 / hz).clamp(1, 65535) as u16;
    unsafe {
        let mut cmd_port = x86_64::instructions::port::Port::<u8>::new(0x43);
        let mut data_port = x86_64::instructions::port::Port::<u8>::new(0x40);
        cmd_port.write(0x36);
        data_port.write((divisor & 0xFF) as u8);
        data_port.write(((divisor >> 8) & 0xFF) as u8);
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut status_port = Port::new(0x64);
    let mut data_port = Port::new(0x60);
    for _ in 0..16 {
        let status: u8 = unsafe { status_port.read() };
        if status & 0x01 == 0 {
            break;
        }
        let byte: u8 = unsafe { data_port.read() };
        if status & 0x20 != 0 {
            crate::drivers::mouse::process_packet_byte(byte);
        } else {
            crate::task::keyboard::add_scancode(byte);
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut status_port = Port::new(0x64);
    let mut data_port = Port::new(0x60);
    for _ in 0..16 {
        let status: u8 = unsafe { status_port.read() };
        if status & 0x01 == 0 {
            break;
        }
        let byte: u8 = unsafe { data_port.read() };
        if status & 0x20 != 0 {
            crate::drivers::mouse::process_packet_byte(byte);
        } else {
            crate::task::keyboard::add_scancode(byte);
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

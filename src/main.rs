#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(mouros::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

#[cfg(not(test))]
use mouros::println;
use mouros::task::{Task, executor::Executor};
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use mouros::allocator;
    use mouros::drivers::{ac97, bga, mouse};
    use mouros::gui::{self, Desktop, Framebuffer, run_desktop};
    use mouros::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    mouros::init();

    let mem_info = memory::init_memory_stats(&boot_info.memory_map);

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // Dynamically scale kernel heap based on detected usable physical RAM:
    // If usable RAM >= 512 MB: allocate 48 MiB heap (ample for Doom + MP3 + Desktop)
    // If usable RAM >= 256 MB: allocate 32 MiB heap
    // If usable RAM >= 128 MB: allocate 16 MiB heap
    // Otherwise: allocate 8 MiB heap
    let heap_size = if mem_info.usable_ram_bytes >= 512 * 1024 * 1024 {
        48 * 1024 * 1024
    } else if mem_info.usable_ram_bytes >= 256 * 1024 * 1024 {
        32 * 1024 * 1024
    } else if mem_info.usable_ram_bytes >= 128 * 1024 * 1024 {
        16 * 1024 * 1024
    } else {
        8 * 1024 * 1024
    };

    allocator::init_heap_with_size(&mut mapper, &mut frame_allocator, heap_size)
        .expect("heap initialization failed");

    mouros::task::keyboard::init();

    #[cfg(test)]
    test_main();

    // Initialize Bochs Graphics Adapter (800x600 32bpp True Color)
    let bga_device = bga::BgaDevice::init(
        &mut mapper,
        &mut frame_allocator,
        bga::DEFAULT_WIDTH,
        bga::DEFAULT_HEIGHT,
    )
    .expect("Failed to initialize BGA display device");

    // Initialize AC97 Audio Device
    ac97::init(phys_mem_offset, &mut frame_allocator);

    // Initialize PS/2 Mouse
    mouse::init(bga::DEFAULT_WIDTH as isize, bga::DEFAULT_HEIGHT as isize);

    let mut fb = Framebuffer::new(bga_device);

    // Show system welcome screen before launching desktop
    gui::show_welcome_screen(&mut fb);

    let desktop = Desktop::new(fb);

    let mut executor = Executor::new();
    executor.spawn(Task::new(run_desktop(desktop)));
    executor.run();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    mouros::serial_println!("{}", info);
    println!("{}", info);
    mouros::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    mouros::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::paging::{FrameAllocator, Size4KiB};
use x86_64::VirtAddr;
use super::pci::{self, PciDevice};

pub const BDL_ENTRIES: usize = 32;
// 576 stereo frames = 1152 16-bit values = 2304 bytes (fits comfortably within 4096-byte frame)
pub const SAMPLES_PER_BD: usize = 1152;

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct BufferDescriptor {
    pub addr: u32,
    pub ctl_len: u32, // Bit 31: IOC, Bit 30: BUP, Bits 0..15: length in 16-bit samples
}

pub struct Ac97Device {
    pub pci_dev: PciDevice,
    pub nambar: u16,  // Mixer base port (e.g. 0xC000)
    pub nabmbar: u16, // Bus Master base port (e.g. 0xC400)
    pub bdl_phys: u32,
    pub bdl_virt: *mut BufferDescriptor,
    pub page_phys: [u32; BDL_ENTRIES],
    pub page_virt: [*mut i16; BDL_ENTRIES],
    pub next_desc: usize,
    pub is_running: bool,
    pub current_sample_rate: u32,
}

unsafe impl Send for Ac97Device {}
unsafe impl Sync for Ac97Device {}

static AC97_INSTANCE: Mutex<Option<Ac97Device>> = Mutex::new(None);

pub fn find_ac97_device() -> Option<PciDevice> {
    for bus in 0..=1 {
        for slot in 0..32 {
            let vendor_and_device = unsafe { pci::pci_read_32(bus, slot, 0, 0) };
            let vendor_id = (vendor_and_device & 0xFFFF) as u16;
            let device_id = ((vendor_and_device >> 16) & 0xFFFF) as u16;

            if vendor_id == 0xFFFF {
                continue;
            }

            let class_rev = unsafe { pci::pci_read_32(bus, slot, 0, 0x08) };
            let class_code = ((class_rev >> 24) & 0xFF) as u8;
            let subclass = ((class_rev >> 16) & 0xFF) as u8;

            // Class 0x04 (Multimedia), Subclass 0x01 (Audio) or Intel AC97 (0x8086:0x2415)
            if (class_code == 0x04 && subclass == 0x01) || (vendor_id == 0x8086 && device_id == 0x2415) {
                return Some(PciDevice {
                    bus,
                    slot,
                    func: 0,
                    vendor_id,
                    device_id,
                    class_code,
                    subclass,
                });
            }
        }
    }
    None
}

pub fn init<A: FrameAllocator<Size4KiB>>(phys_mem_offset: VirtAddr, frame_allocator: &mut A) {
    let pci_dev = match find_ac97_device() {
        Some(dev) => dev,
        None => {
            crate::serial_println!("[AC97] No AC97 audio PCI controller detected; using PC speaker fallback.");
            return;
        }
    };

    pci_dev.enable_bus_master_and_memory();

    let nambar = (pci_dev.read_bar(0) & 0xFFFF_FFFC) as u16;
    let nabmbar = (pci_dev.read_bar(1) & 0xFFFF_FFFC) as u16;

    if nambar == 0 || nabmbar == 0 {
        crate::serial_println!("[AC97] Invalid BAR addresses (NAMBAR=0x{:X}, NABMBAR=0x{:X})", nambar, nabmbar);
        return;
    }

    crate::serial_println!(
        "[AC97] Found Intel AC97 Controller (0x{:04X}:0x{:04X}) at bus {} slot {}",
        pci_dev.vendor_id, pci_dev.device_id, pci_dev.bus, pci_dev.slot
    );
    crate::serial_println!("[AC97] Mixer NAMBAR: 0x{:04X}, Bus Master NABMBAR: 0x{:04X}", nambar, nabmbar);

    // 1. Reset mixer and initialize master + PCM volumes to unmuted 0dB attenuation
    unsafe {
        Port::<u16>::new(nambar + 0x00).write(0x0001);
        for _ in 0..1000 { core::hint::spin_loop(); }
        Port::<u16>::new(nambar + 0x02).write(0x0000); // Master Volume (0dB, unmuted)
        Port::<u16>::new(nambar + 0x18).write(0x0000); // PCM Out Volume (0dB, unmuted)

        // Check if Variable Rate Audio (VRA) is supported
        let ext_id = Port::<u16>::new(nambar + 0x28).read();
        if ext_id & 1 != 0 {
            let ext_ctrl = Port::<u16>::new(nambar + 0x2A).read();
            Port::<u16>::new(nambar + 0x2A).write(ext_ctrl | 1); // Enable VRA
            Port::<u16>::new(nambar + 0x2C).write(44100);       // Default to 44.1 kHz
            let actual_rate = Port::<u16>::new(nambar + 0x2C).read();
            crate::serial_println!("[AC97] VRA enabled; Front DAC rate set to {} Hz", actual_rate);
        } else {
            crate::serial_println!("[AC97] Fixed 48000 Hz codec");
        }
    }

    // 2. Reset PCM Out channel in Bus Master
    unsafe {
        // Write RR (Reset Registers) to PO_CR
        Port::<u8>::new(nabmbar + 0x1B).write(0x02);
        for _ in 0..10_000 {
            if Port::<u8>::new(nabmbar + 0x1B).read() & 0x02 == 0 {
                break;
            }
        }
        // Clear status bits (PO_SR)
        Port::<u16>::new(nabmbar + 0x16).write(0x1C);
    }

    // 3. Allocate physical DMA memory for BDL (Buffer Descriptor List) and PCM pages
    let bdl_frame = match frame_allocator.allocate_frame() {
        Some(f) => f,
        None => {
            crate::serial_println!("[AC97] Failed to allocate frame for BDL");
            return;
        }
    };
    let bdl_phys = bdl_frame.start_address().as_u64() as u32;
    let bdl_virt = (phys_mem_offset + bdl_frame.start_address().as_u64()).as_mut_ptr::<BufferDescriptor>();

    let mut page_phys = [0u32; BDL_ENTRIES];
    let mut page_virt = [core::ptr::null_mut::<i16>(); BDL_ENTRIES];

    for i in 0..BDL_ENTRIES {
        let frame = match frame_allocator.allocate_frame() {
            Some(f) => f,
            None => {
                crate::serial_println!("[AC97] Failed to allocate frame for PCM page {}", i);
                return;
            }
        };
        page_phys[i] = frame.start_address().as_u64() as u32;
        page_virt[i] = (phys_mem_offset + frame.start_address().as_u64()).as_mut_ptr::<i16>();

        unsafe {
            // Zero out physical memory buffer
            core::ptr::write_bytes(page_virt[i] as *mut u8, 0, 4096);
            // Set up BDL entry
            let desc = bdl_virt.add(i);
            (*desc).addr = page_phys[i];
            (*desc).ctl_len = 0;
        }
    }

    // 4. Program BDBAR (Buffer Descriptor Base Address) in hardware
    unsafe {
        Port::<u32>::new(nabmbar + 0x10).write(bdl_phys);
        Port::<u8>::new(nabmbar + 0x15).write(0); // LVI = 0 initially
    }

    let dev = Ac97Device {
        pci_dev,
        nambar,
        nabmbar,
        bdl_phys,
        bdl_virt,
        page_phys,
        page_virt,
        next_desc: 0,
        is_running: false,
        current_sample_rate: 44100,
    };

    *AC97_INSTANCE.lock() = Some(dev);
    crate::serial_println!("[AC97] Initialization successful! 32-entry DMA ring buffer ready.");
}

pub fn is_available() -> bool {
    AC97_INSTANCE.lock().is_some()
}

pub fn set_sample_rate(rate: u32) {
    if let Some(ref mut dev) = *AC97_INSTANCE.lock() {
        if dev.current_sample_rate != rate {
            unsafe {
                Port::<u16>::new(dev.nambar + 0x2C).write(rate as u16);
                // In QEMU, writing front DAC rate re-opens the voice with audio_be_open_out,
                // which deactivates the voice. If DMA is running, re-activate it by writing PO_CR!
                if dev.is_running {
                    let cr = Port::<u8>::new(dev.nabmbar + 0x1B).read();
                    if cr & 0x01 != 0 {
                        Port::<u8>::new(dev.nabmbar + 0x1B).write(cr & !0x01);
                        Port::<u8>::new(dev.nabmbar + 0x1B).write(cr | 0x01);
                    }
                }
            }
            dev.current_sample_rate = rate;
            crate::serial_println!("[AC97] Front DAC sample rate set to {} Hz", rate);
        }
    }
}

pub fn can_write() -> bool {
    if let Some(ref dev) = *AC97_INSTANCE.lock() {
        let civ = unsafe { Port::<u8>::new(dev.nabmbar + 0x14).read() as usize & 0x1F };
        let in_flight = (dev.next_desc + BDL_ENTRIES - civ) % BDL_ENTRIES;
        in_flight < 24
    } else {
        false
    }
}

pub fn write_pcm_samples(mut samples: &[i16]) -> usize {
    let mut guard = AC97_INSTANCE.lock();
    let dev = match *guard {
        Some(ref mut d) => d,
        None => return 0,
    };

    let mut total_written = 0;
    let mut last_written = None;

    while !samples.is_empty() {
        let civ = unsafe { Port::<u8>::new(dev.nabmbar + 0x14).read() as usize & 0x1F };
        let in_flight = (dev.next_desc + BDL_ENTRIES - civ) % BDL_ENTRIES;
        if in_flight >= 26 {
            break;
        }

        let count = samples.len().min(SAMPLES_PER_BD);
        let desc_idx = dev.next_desc;
        unsafe {
            core::ptr::copy_nonoverlapping(samples.as_ptr(), dev.page_virt[desc_idx], count);
            let desc = dev.bdl_virt.add(desc_idx);
            (*desc).addr = dev.page_phys[desc_idx];
            (*desc).ctl_len = count as u32; // Lower 16 bits = number of 16-bit samples
        }

        last_written = Some(desc_idx);
        dev.next_desc = (dev.next_desc + 1) % BDL_ENTRIES;
        samples = &samples[count..];
        total_written += count;
    }

    if let Some(lvi) = last_written {
        unsafe {
            // Update Last Valid Index in hardware
            Port::<u8>::new(dev.nabmbar + 0x15).write(lvi as u8);

            let cr = Port::<u8>::new(dev.nabmbar + 0x1B).read();
            let sr = Port::<u16>::new(dev.nabmbar + 0x16).read();

            if cr & 0x01 == 0 {
                // If DMA is not active yet, only start it once we have queued enough descriptors
                // (at least 4 descriptors = ~9KB) to satisfy PulseAudio prebuf immediately.
                let civ = Port::<u8>::new(dev.nabmbar + 0x14).read() as usize & 0x1F;
                let in_flight = (dev.next_desc + BDL_ENTRIES - civ) % BDL_ENTRIES;
                if in_flight >= 4 || samples.is_empty() {
                    Port::<u16>::new(dev.nabmbar + 0x16).write(sr & 0x1E);
                    Port::<u8>::new(dev.nabmbar + 0x1B).write(0x01); // RPBM = 1 (Run)
                    dev.is_running = true;
                }
            } else if sr & 0x01 != 0 {
                // DMA was running but halted due to underrun (SR_DCH).
                // Writing PO_LVI above already woke QEMU because CR_RPBM is 1.
                // Clear the status bits to reset error/completion flags.
                Port::<u16>::new(dev.nabmbar + 0x16).write(sr & 0x1E);
            }
        }
    }

    total_written
}

pub fn write_pcm_chunk(samples: &[i16]) -> bool {
    write_pcm_samples(samples) > 0
}

pub fn start_playback() {
    if let Some(ref mut dev) = *AC97_INSTANCE.lock() {
        dev.is_running = true;
    }
}

pub fn pause_playback() {
    if let Some(ref mut dev) = *AC97_INSTANCE.lock() {
        unsafe {
            Port::<u8>::new(dev.nabmbar + 0x1B).write(0x00); // Stop running
        }
        dev.is_running = false;
    }
}

pub fn stop_playback() {
    if let Some(ref mut dev) = *AC97_INSTANCE.lock() {
        unsafe {
            Port::<u8>::new(dev.nabmbar + 0x1B).write(0x00); // Stop running
            // Reset channel registers
            Port::<u8>::new(dev.nabmbar + 0x1B).write(0x02);
            for _ in 0..10_000 {
                if Port::<u8>::new(dev.nabmbar + 0x1B).read() & 0x02 == 0 {
                    break;
                }
            }
            Port::<u16>::new(dev.nabmbar + 0x16).write(0x1C);
            // CRITICAL: Re-program BDBAR because QEMU resets BDBAR to 0 on channel reset (RR)!
            Port::<u32>::new(dev.nabmbar + 0x10).write(dev.bdl_phys);
            Port::<u8>::new(dev.nabmbar + 0x15).write(0);
        }
        dev.next_desc = 0;
        dev.is_running = false;
    }
}

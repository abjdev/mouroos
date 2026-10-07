#![allow(dead_code)]

use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::paging::{FrameAllocator, Size4KiB};
use x86_64::VirtAddr;
use super::pci::{self, PciDevice};

// RTL8139 Register Offsets
const REG_MAC0: u16 = 0x00;
const REG_MAR0: u16 = 0x08;
const REG_TSD0: u16 = 0x10;
const REG_TSAD0: u16 = 0x20;
const REG_RBSTART: u16 = 0x30;
const REG_CMD: u16 = 0x37;
const REG_CAPR: u16 = 0x38;
const REG_CBR: u16 = 0x3A;
const REG_IMR: u16 = 0x3C;
const REG_ISR: u16 = 0x3E;
const REG_TCR: u16 = 0x40;
const REG_RCR: u16 = 0x44;
const REG_CONFIG1: u16 = 0x52;

// Command register bits
const CMD_BUFE: u8 = 0x01; // Buffer Empty
const CMD_TE: u8 = 0x04;   // Transmitter Enable
const CMD_RE: u8 = 0x08;   // Receiver Enable
const CMD_RST: u8 = 0x10;  // Software Reset

// Interrupt Status / Mask bits
const INT_ROK: u16 = 0x0001; // Receive OK
const INT_TOK: u16 = 0x0004; // Transmit OK
const INT_RER: u16 = 0x0002; // Receive Error
const INT_TER: u16 = 0x0008; // Transmit Error

// Buffer configuration
const RX_BUF_SIZE: usize = 8192;
const RX_BUF_ALLOC_SIZE: usize = 12288; // 8K + 16 wrap + 1500 MTU margin = 3x 4KB frames
const TX_BUF_COUNT: usize = 4;
const TX_BUF_SIZE: usize = 2048; // Fits 1514 MTU frame, 4 * 2K = 2x 4KB frames

pub struct Rtl8139Device {
    pub pci_dev: PciDevice,
    pub io_base: u16,
    pub mac: [u8; 6],
    pub rx_virt: *mut u8,
    pub rx_phys: u32,
    pub rx_offset: usize,
    pub tx_virt: [*mut u8; TX_BUF_COUNT],
    pub tx_phys: [u32; TX_BUF_COUNT],
    pub tx_cur: usize,
    pub irq: u8,
}

unsafe impl Send for Rtl8139Device {}
unsafe impl Sync for Rtl8139Device {}

pub static RTL8139_INSTANCE: Mutex<Option<Rtl8139Device>> = Mutex::new(None);

impl Rtl8139Device {
    pub fn send_packet(&mut self, packet: &[u8]) -> bool {
        if packet.is_empty() || packet.len() > 1514 {
            return false;
        }

        let desc = self.tx_cur;
        let dest_buf = self.tx_virt[desc];

        // Ethernet frames must be at least 60 bytes (excluding 4-byte CRC)
        let tx_len = packet.len().max(60);

        unsafe {
            // Copy packet content
            core::ptr::copy_nonoverlapping(packet.as_ptr(), dest_buf, packet.len());
            // Zero pad if shorter than 60 bytes
            if packet.len() < 60 {
                core::ptr::write_bytes(dest_buf.add(packet.len()), 0, 60 - packet.len());
            }

            // Write physical address to TSAD
            let mut tsad_port = Port::<u32>::new(self.io_base + REG_TSAD0 + (desc as u16 * 4));
            tsad_port.write(self.tx_phys[desc]);

            // Write length to TSD (starts transmit immediately)
            // Bits 0..12: size, Bit 13: OWN (0 = starts DMA)
            let mut tsd_port = Port::<u32>::new(self.io_base + REG_TSD0 + (desc as u16 * 4));
            tsd_port.write(tx_len as u32);
        }

        self.tx_cur = (self.tx_cur + 1) % TX_BUF_COUNT;
        true
    }

    pub fn poll_packet(&mut self, out_buf: &mut [u8]) -> Option<usize> {
        unsafe {
            // Check if buffer is empty
            let cmd: u8 = Port::<u8>::new(self.io_base + REG_CMD).read();
            if (cmd & CMD_BUFE) != 0 {
                return None;
            }

            // Acknowledge interrupts
            let mut isr_port = Port::<u16>::new(self.io_base + REG_ISR);
            let isr: u16 = isr_port.read();
            if isr != 0 {
                isr_port.write(isr);
            }

            let pkt_ptr = self.rx_virt.add(self.rx_offset);
            let status = core::ptr::read_unaligned(pkt_ptr as *const u16);
            let length = core::ptr::read_unaligned(pkt_ptr.add(2) as *const u16) as usize;

            // Bit 0 = ROK (Receive OK)
            if (status & 0x01) == 0 || length < 4 || length > 1536 {
                // False packet or error; reset offset
                return None;
            }

            let payload_len = length.saturating_sub(4); // Exclude 4-byte CRC
            let copy_len = payload_len.min(out_buf.len());

            core::ptr::copy_nonoverlapping(pkt_ptr.add(4), out_buf.as_mut_ptr(), copy_len);

            // Advance rx_offset: DWORD aligned (4-byte aligned)
            self.rx_offset = (self.rx_offset + length + 4 + 3) & !3;
            self.rx_offset %= RX_BUF_SIZE;

            // Update CAPR (offset by 16 bytes per RTL8139 spec)
            let capr_val = ((self.rx_offset as isize - 16).rem_euclid(RX_BUF_SIZE as isize)) as u16;
            Port::<u16>::new(self.io_base + REG_CAPR).write(capr_val);

            Some(copy_len)
        }
    }
}

pub fn init<A: FrameAllocator<Size4KiB>>(phys_mem_offset: VirtAddr, frame_allocator: &mut A) {
    let pci_dev = match pci::find_rtl8139_device() {
        Some(dev) => dev,
        None => {
            crate::serial_println!("[RTL8139] No RTL8139 PCI network controller detected; loopback available.");
            return;
        }
    };

    pci_dev.enable_bus_master_and_memory();

    let bar0 = pci_dev.read_bar(0);
    let io_base = (bar0 & 0xFFFC) as u16;

    if io_base == 0 {
        crate::serial_println!("[RTL8139] Invalid I/O port base address (BAR0=0x{:X})", bar0);
        return;
    }

    let irq = (pci_dev.read_u32(0x3C) & 0xFF) as u8;

    crate::serial_println!(
        "[RTL8139] Found Realtek RTL8139 NIC at bus {} slot {} (io_base=0x{:04X}, irq={})",
        pci_dev.bus, pci_dev.slot, io_base, irq
    );

    // 1. Power on chip (wake from sleep)
    unsafe {
        Port::<u8>::new(io_base + REG_CONFIG1).write(0x00);
    }

    // 2. Software Reset
    unsafe {
        let mut cmd_port = Port::<u8>::new(io_base + REG_CMD);
        cmd_port.write(CMD_RST);
        let mut timeout = 100_000;
        while (cmd_port.read() & CMD_RST) != 0 && timeout > 0 {
            timeout -= 1;
            core::hint::spin_loop();
        }
    }

    // 3. Read Hardware MAC Address
    let mut mac = [0u8; 6];
    unsafe {
        for i in 0..6 {
            mac[i] = Port::<u8>::new(io_base + REG_MAC0 + i as u16).read();
        }
    }
    crate::serial_println!(
        "[RTL8139] MAC Address: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );

    // 4. Allocate 3 contiguous frames for 12KB RX buffer
    let rx_f0 = frame_allocator.allocate_frame().expect("RTL8139 RX frame 0 alloc failed");
    let _rx_f1 = frame_allocator.allocate_frame().expect("RTL8139 RX frame 1 alloc failed");
    let _rx_f2 = frame_allocator.allocate_frame().expect("RTL8139 RX frame 2 alloc failed");

    let rx_phys = rx_f0.start_address().as_u64() as u32;
    let rx_virt = (phys_mem_offset + rx_f0.start_address().as_u64()).as_mut_ptr::<u8>();

    unsafe {
        core::ptr::write_bytes(rx_virt, 0, RX_BUF_ALLOC_SIZE);
        // Write physical address to RBSTART
        Port::<u32>::new(io_base + REG_RBSTART).write(rx_phys);
    }

    // 5. Allocate 2 contiguous frames for 4x 2KB TX buffers
    let tx_f0 = frame_allocator.allocate_frame().expect("RTL8139 TX frame 0 alloc failed");
    let _tx_f1 = frame_allocator.allocate_frame().expect("RTL8139 TX frame 1 alloc failed");

    let mut tx_phys = [0u32; TX_BUF_COUNT];
    let mut tx_virt = [core::ptr::null_mut::<u8>(); TX_BUF_COUNT];

    let base_phys = tx_f0.start_address().as_u64();
    for i in 0..TX_BUF_COUNT {
        let offset = (i * TX_BUF_SIZE) as u64;
        tx_phys[i] = (base_phys + offset) as u32;
        tx_virt[i] = (phys_mem_offset + base_phys + offset).as_mut_ptr::<u8>();
        unsafe {
            core::ptr::write_bytes(tx_virt[i], 0, TX_BUF_SIZE);
            Port::<u32>::new(io_base + REG_TSAD0 + (i as u16 * 4)).write(tx_phys[i]);
        }
    }

    // 6. Set Interrupt Mask (IMR): Enable ROK | TOK
    unsafe {
        Port::<u16>::new(io_base + REG_IMR).write(INT_ROK | INT_TOK | INT_RER | INT_TER);
    }

    // 7. Receive Configuration (RCR): Accept Broadcast, Multicast, Physical match, wrap = 0
    unsafe {
        // AAP (0x01) | APM (0x02) | AM (0x04) | AB (0x08) = 0x0F
        Port::<u32>::new(io_base + REG_RCR).write(0x0000_000F);
    }

    // 8. Enable Transmitter and Receiver
    unsafe {
        Port::<u8>::new(io_base + REG_CMD).write(CMD_TE | CMD_RE);
    }

    let dev = Rtl8139Device {
        pci_dev,
        io_base,
        mac,
        rx_virt,
        rx_phys,
        rx_offset: 0,
        tx_virt,
        tx_phys,
        tx_cur: 0,
        irq,
    };

    *RTL8139_INSTANCE.lock() = Some(dev);
    crate::serial_println!("[RTL8139] Driver initialized successfully!");
}

pub fn send(packet: &[u8]) -> bool {
    if let Some(dev) = RTL8139_INSTANCE.lock().as_mut() {
        dev.send_packet(packet)
    } else {
        false
    }
}

pub fn poll(buf: &mut [u8]) -> Option<usize> {
    if let Some(dev) = RTL8139_INSTANCE.lock().as_mut() {
        dev.poll_packet(buf)
    } else {
        None
    }
}

pub fn get_mac() -> Option<[u8; 6]> {
    RTL8139_INSTANCE.lock().as_ref().map(|d| d.mac)
}

pub fn is_present() -> bool {
    RTL8139_INSTANCE.lock().is_some()
}

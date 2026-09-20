use super::pci;
use crate::memory;
use x86_64::{
    PhysAddr, VirtAddr,
    instructions::port::Port,
    structures::paging::{FrameAllocator, Mapper, PageTableFlags, Size4KiB},
};

const VBE_DISPI_IOPORT_INDEX: u16 = 0x01CE;
const VBE_DISPI_IOPORT_DATA: u16 = 0x01CF;

const VBE_DISPI_INDEX_ID: u16 = 0;
const VBE_DISPI_INDEX_XRES: u16 = 1;
const VBE_DISPI_INDEX_YRES: u16 = 2;
const VBE_DISPI_INDEX_BPP: u16 = 3;
const VBE_DISPI_INDEX_ENABLE: u16 = 4;
const VBE_DISPI_INDEX_BANK: u16 = 5;
#[allow(dead_code)]
const VBE_DISPI_INDEX_VIRT_WIDTH: u16 = 6;
#[allow(dead_code)]
const VBE_DISPI_INDEX_VIRT_HEIGHT: u16 = 7;
#[allow(dead_code)]
const VBE_DISPI_INDEX_X_OFFSET: u16 = 8;
#[allow(dead_code)]
const VBE_DISPI_INDEX_Y_OFFSET: u16 = 9;

const VBE_DISPI_DISABLED: u16 = 0x00;
const VBE_DISPI_ENABLED: u16 = 0x01;
const VBE_DISPI_LFB_ENABLED: u16 = 0x40;

pub const DEFAULT_WIDTH: usize = 800;
pub const DEFAULT_HEIGHT: usize = 600;
pub const DEFAULT_BPP: usize = 32;

pub const FRAMEBUFFER_VIRT_START: u64 = 0x_5000_0000_0000;

pub struct BgaDevice {
    pub width: usize,
    pub height: usize,
    pub bpp: usize,
    pub framebuffer_ptr: *mut u32,
    pub phys_addr: u64,
}

unsafe impl Send for BgaDevice {}
unsafe impl Sync for BgaDevice {}

impl BgaDevice {
    pub fn is_available() -> bool {
        let id = unsafe { bga_read(VBE_DISPI_INDEX_ID) };
        // BGA versions range from 0xB0C0 to 0xB0C5
        id >= 0xB0C0 && id <= 0xB0C5
    }

    pub fn init(
        mapper: &mut impl Mapper<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
        width: usize,
        height: usize,
    ) -> Result<Self, &'static str> {
        if !Self::is_available() {
            return Err("BGA device not found on I/O ports 0x1CE/0x1CF");
        }

        let pci_dev = pci::find_vga_device().ok_or("PCI VGA device not found")?;
        pci_dev.enable_bus_master_and_memory();

        let bar0 = pci_dev.read_bar(0);
        let fb_phys = (bar0 & 0xFFFF_FFF0) as u64;
        if fb_phys == 0 {
            return Err("Invalid BAR0 framebuffer physical address");
        }

        // Set video mode
        unsafe {
            bga_write(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_DISABLED);
            bga_write(VBE_DISPI_INDEX_XRES, width as u16);
            bga_write(VBE_DISPI_INDEX_YRES, height as u16);
            bga_write(VBE_DISPI_INDEX_BPP, DEFAULT_BPP as u16);
            bga_write(
                VBE_DISPI_INDEX_ENABLE,
                VBE_DISPI_ENABLED | VBE_DISPI_LFB_ENABLED,
            );
            bga_write(VBE_DISPI_INDEX_BANK, 0);
        }

        // Map framebuffer to virtual memory
        let fb_bytes = width * height * 4;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::WRITE_THROUGH;

        unsafe {
            memory::map_physical_range(
                mapper,
                frame_allocator,
                PhysAddr::new(fb_phys),
                VirtAddr::new(FRAMEBUFFER_VIRT_START),
                fb_bytes,
                flags,
            )
            .map_err(|_| "Failed to map physical framebuffer to virtual memory")?;
        }

        Ok(BgaDevice {
            width,
            height,
            bpp: DEFAULT_BPP,
            framebuffer_ptr: FRAMEBUFFER_VIRT_START as *mut u32,
            phys_addr: fb_phys,
        })
    }

    #[inline(always)]
    pub fn copy_framebuffer(&mut self, buffer: &[u32]) {
        let count = self.width * self.height;
        let len = buffer.len().min(count);
        unsafe {
            core::ptr::copy_nonoverlapping(buffer.as_ptr(), self.framebuffer_ptr, len);
        }
    }

    #[inline(always)]
    pub fn copy_rect(&mut self, buffer: &[u32], rx: usize, ry: usize, rw: usize, rh: usize) {
        let rx2 = (rx + rw).min(self.width);
        let ry2 = (ry + rh).min(self.height);
        if rx >= rx2 || ry >= ry2 {
            return;
        }
        let copy_width = rx2 - rx;
        unsafe {
            for y in ry..ry2 {
                let offset = y * self.width + rx;
                core::ptr::copy_nonoverlapping(
                    buffer.as_ptr().add(offset),
                    self.framebuffer_ptr.add(offset),
                    copy_width,
                );
            }
        }
    }
}

unsafe fn bga_write(index: u16, data: u16) {
    let mut index_port = Port::<u16>::new(VBE_DISPI_IOPORT_INDEX);
    let mut data_port = Port::<u16>::new(VBE_DISPI_IOPORT_DATA);
    unsafe {
        index_port.write(index);
        data_port.write(data);
    }
}

unsafe fn bga_read(index: u16) -> u16 {
    let mut index_port = Port::<u16>::new(VBE_DISPI_IOPORT_INDEX);
    let mut data_port = Port::<u16>::new(VBE_DISPI_IOPORT_DATA);
    unsafe {
        index_port.write(index);
        data_port.read()
    }
}

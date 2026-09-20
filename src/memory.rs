use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

/// Initialize a new OffsetPageTable.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// A FrameAllocator that always returns `None`.
pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    current_region_idx: usize,
    current_addr: u64,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        let mut allocator = BootInfoFrameAllocator {
            memory_map,
            current_region_idx: 0,
            current_addr: 0,
        };
        allocator.advance_to_next_usable();
        allocator
    }

    fn advance_to_next_usable(&mut self) {
        while self.current_region_idx < self.memory_map.len() {
            let region = &self.memory_map[self.current_region_idx];
            if region.region_type == MemoryRegionType::Usable && region.range.start_addr() < region.range.end_addr() {
                self.current_addr = region.range.start_addr();
                return;
            }
            self.current_region_idx += 1;
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        while self.current_region_idx < self.memory_map.len() {
            let region = &self.memory_map[self.current_region_idx];
            if region.region_type == MemoryRegionType::Usable && self.current_addr + 4096 <= region.range.end_addr() {
                let frame = PhysFrame::containing_address(PhysAddr::new(self.current_addr));
                self.current_addr += 4096;
                return Some(frame);
            }
            self.current_region_idx += 1;
            self.advance_to_next_usable();
        }
        None
    }
}

/// Maps a contiguous range of physical memory to a contiguous virtual memory range.
pub unsafe fn map_physical_range(
    mapper: &mut impl x86_64::structures::paging::Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    phys_start: PhysAddr,
    virt_start: VirtAddr,
    size: usize,
    flags: x86_64::structures::paging::PageTableFlags,
) -> Result<(), x86_64::structures::paging::mapper::MapToError<Size4KiB>> {
    use x86_64::structures::paging::Page;

    if size == 0 {
        return Ok(());
    }

    let page_start = Page::<Size4KiB>::containing_address(virt_start);
    let page_end = Page::<Size4KiB>::containing_address(virt_start + (size as u64) - 1u64);
    let mut phys = phys_start;

    for page in Page::range_inclusive(page_start, page_end) {
        let frame = PhysFrame::containing_address(phys);
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
        phys += 4096u64;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct SystemMemoryInfo {
    pub total_ram_bytes: u64,
    pub usable_ram_bytes: u64,
}

pub static SYSTEM_MEMORY_INFO: spin::Mutex<SystemMemoryInfo> = spin::Mutex::new(SystemMemoryInfo {
    total_ram_bytes: 8 * 1024 * 1024,
    usable_ram_bytes: 8 * 1024 * 1024,
});

pub fn get_system_memory_info() -> SystemMemoryInfo {
    *SYSTEM_MEMORY_INFO.lock()
}

pub fn init_memory_stats(memory_map: &'static MemoryMap) -> SystemMemoryInfo {
    let mut max_ram_below_4g: u64 = 0;
    let mut above_4g_bytes: u64 = 0;
    let mut usable_bytes: u64 = 0;

    for r in memory_map.iter() {
        let size = r.range.end_addr().saturating_sub(r.range.start_addr());
        if r.region_type == MemoryRegionType::Usable {
            usable_bytes += size;
        }

        if r.range.end_addr() <= 0xF000_0000 {
            if r.range.end_addr() > max_ram_below_4g {
                max_ram_below_4g = r.range.end_addr();
            }
        } else if r.range.start_addr() >= 0x1_0000_0000 && r.region_type != MemoryRegionType::Reserved {
            above_4g_bytes += size;
        }
    }

    let total_ram_bytes = max_ram_below_4g + above_4g_bytes;
    let info = SystemMemoryInfo {
        total_ram_bytes,
        usable_ram_bytes: usable_bytes,
    };
    *SYSTEM_MEMORY_INFO.lock() = info;
    info
}

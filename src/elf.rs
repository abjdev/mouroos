use alloc::string::String;
use alloc::vec;
use core::sync::atomic::Ordering;
use spin::Mutex;

#[repr(C)]
pub struct MourosApi {
    pub print_fn: extern "C" fn(ptr: *const u8, len: usize),
    pub get_ticks_fn: extern "C" fn() -> u64,
    pub get_memory_fn: extern "C" fn(total_ram: *mut u64, heap_used: *mut u64),
}

static OUTPUT_BUFFER: Mutex<Option<String>> = Mutex::new(None);

extern "C" fn api_print(ptr: *const u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
    if let Ok(s) = core::str::from_utf8(slice) {
        if let Some(buf) = &mut *OUTPUT_BUFFER.lock() {
            buf.push_str(s);
        }
    }
}

extern "C" fn api_get_ticks() -> u64 {
    crate::interrupts::TICKS.load(Ordering::Relaxed)
}

extern "C" fn api_get_memory(total_ram: *mut u64, heap_used: *mut u64) {
    let mem_info = crate::memory::get_system_memory_info();
    let (used, _) = crate::allocator::heap_stats();
    if !total_ram.is_null() {
        unsafe { *total_ram = mem_info.total_ram_bytes as u64 };
    }
    if !heap_used.is_null() {
        unsafe { *heap_used = used as u64 };
    }
}

#[derive(Debug)]
pub struct ProcessResult {
    pub exit_code: i32,
    pub output: String,
}

pub fn execute_elf(elf_bytes: &[u8]) -> Result<ProcessResult, &'static str> {
    if elf_bytes.len() < 64 {
        return Err("ELF file too small");
    }

    // 1. Verify Magic
    if &elf_bytes[0..4] != b"\x7fELF" {
        return Err("Invalid ELF magic header");
    }

    // Check 64-bit (class = 2) and little-endian (data = 1)
    if elf_bytes[4] != 2 || elf_bytes[5] != 1 {
        return Err("Unsupported architecture (requires 64-bit little endian)");
    }

    // Read header fields
    let _e_type = u16::from_le_bytes(elf_bytes[16..18].try_into().unwrap());
    let e_machine = u16::from_le_bytes(elf_bytes[18..20].try_into().unwrap());
    let e_entry = u64::from_le_bytes(elf_bytes[24..32].try_into().unwrap());
    let e_phoff = u64::from_le_bytes(elf_bytes[32..40].try_into().unwrap()) as usize;
    let e_phentsize = u16::from_le_bytes(elf_bytes[54..56].try_into().unwrap()) as usize;
    let e_phnum = u16::from_le_bytes(elf_bytes[56..58].try_into().unwrap()) as usize;

    if e_machine != 0x3E {
        return Err("Unsupported machine type (requires x86_64)");
    }

    // 2. Scan PT_LOAD segments to determine memory span
    let mut min_vaddr = u64::MAX;
    let mut max_vaddr = 0u64;

    for i in 0..e_phnum {
        let ph_offset = e_phoff + i * e_phentsize;
        if ph_offset + 56 > elf_bytes.len() {
            return Err("Program header out of bounds");
        }

        let p_type = u32::from_le_bytes(elf_bytes[ph_offset..ph_offset + 4].try_into().unwrap());
        if p_type == 1 {
            // PT_LOAD
            let p_vaddr = u64::from_le_bytes(elf_bytes[ph_offset + 16..ph_offset + 24].try_into().unwrap());
            let p_memsz = u64::from_le_bytes(elf_bytes[ph_offset + 40..ph_offset + 48].try_into().unwrap());

            min_vaddr = min_vaddr.min(p_vaddr);
            max_vaddr = max_vaddr.max(p_vaddr + p_memsz);
        }
    }

    if min_vaddr == u64::MAX || max_vaddr <= min_vaddr {
        return Err("No valid PT_LOAD segments found in ELF");
    }

    let total_size = (max_vaddr - min_vaddr) as usize;
    if total_size > 10 * 1024 * 1024 {
        return Err("ELF executable memory requirement too large (>10MB)");
    }

    // 3. Allocate memory buffer for process execution
    // Align up to 4096 bytes
    let aligned_size = (total_size + 4095) & !4095;
    let mut memory_buffer = vec![0u8; aligned_size + 4096];
    let base_ptr = memory_buffer.as_mut_ptr();

    // 4. Load segments into memory
    for i in 0..e_phnum {
        let ph_offset = e_phoff + i * e_phentsize;
        let p_type = u32::from_le_bytes(elf_bytes[ph_offset..ph_offset + 4].try_into().unwrap());
        if p_type == 1 {
            // PT_LOAD
            let p_offset = u64::from_le_bytes(elf_bytes[ph_offset + 8..ph_offset + 16].try_into().unwrap()) as usize;
            let p_vaddr = u64::from_le_bytes(elf_bytes[ph_offset + 16..ph_offset + 24].try_into().unwrap());
            let p_filesz = u64::from_le_bytes(elf_bytes[ph_offset + 32..ph_offset + 40].try_into().unwrap()) as usize;

            let dest_offset = (p_vaddr - min_vaddr) as usize;
            if p_offset + p_filesz > elf_bytes.len() || dest_offset + p_filesz > memory_buffer.len() {
                return Err("Segment data out of bounds");
            }

            unsafe {
                core::ptr::copy_nonoverlapping(
                    elf_bytes[p_offset..].as_ptr(),
                    base_ptr.add(dest_offset),
                    p_filesz,
                );
            }
        }
    }

    // 5. Calculate entry point function
    let entry_offset = (e_entry - min_vaddr) as usize;
    if entry_offset >= memory_buffer.len() {
        return Err("Entry point address out of bounds");
    }

    let entry_fn_ptr = unsafe { base_ptr.add(entry_offset) };
    let entry_fn: extern "C" fn(*const MourosApi) -> i32 = unsafe {
        core::mem::transmute(entry_fn_ptr)
    };

    let api = MourosApi {
        print_fn: api_print,
        get_ticks_fn: api_get_ticks,
        get_memory_fn: api_get_memory,
    };

    // 6. Execute process
    *OUTPUT_BUFFER.lock() = Some(String::new());

    let exit_code = entry_fn(&api);

    let output = OUTPUT_BUFFER.lock().take().unwrap_or_default();

    Ok(ProcessResult { exit_code, output })
}

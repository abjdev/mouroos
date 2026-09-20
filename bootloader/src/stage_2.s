.section .boot, "awx"
.code16

# This stage sets the target operating mode, loads the kernel from disk,
# creates an e820 memory map, enters protected mode, and jumps to the
# third stage.

second_stage_start_str: .asciz "Mouros Protected Mode Loader (Stage 2)..."
kernel_load_failed_str: .asciz "Mouros Boot Error: Failed to load kernel from disk"

kernel_load_failed:
    mov si, offset kernel_load_failed_str
    call real_mode_println
kernel_load_failed_spin:
    jmp kernel_load_failed_spin

stage_2:
    xor ax, ax
    mov ds, ax
    mov es, ax

    mov si, offset second_stage_start_str
    call real_mode_println

set_target_operating_mode:
    pushf
    mov ax, 0xec00
    mov bl, 0x2
    int 0x15
    popf

load_kernel_from_disk:
    # start of memory buffer
    mov eax, offset _kernel_buffer
    mov [dap_buffer_addr], ax
    mov dword ptr [dap_start_lba + 4], 0

    # check if booted from CD-ROM via ISO boot info table
    mov edx, [bi_boot_file_lba]
    test edx, edx
    jnz load_kernel_from_cdrom

    # --- STANDARD HDD LOADING (512-byte blocks) ---
    mov word ptr [dap_blocks], 1

    mov eax, offset _kernel_start_addr
    mov ebx, offset _start
    sub eax, ebx
    shr eax, 9 # divide by 512
    mov [dap_start_lba], eax

    mov edi, 0x400000

    mov ecx, offset _kernel_size
    add ecx, 511
    shr ecx, 9

load_next_kernel_block_from_disk:
    mov dl, byte ptr [0x7000]
    mov si, offset dap
    mov ah, 0x42
    int 0x13
    jc kernel_load_failed

    push ecx
    push esi
    mov ecx, 512 / 4
    movzx esi, word ptr [dap_buffer_addr]
    rep movsd [edi], [esi]
    pop esi
    pop ecx

    mov eax, [dap_start_lba]
    add eax, 1
    mov [dap_start_lba], eax

    sub ecx, 1
    jnz load_next_kernel_block_from_disk
    jmp create_memory_map

load_kernel_from_cdrom:
    # --- CD-ROM LOADING (2048-byte sectors) ---
    mov word ptr [dap_blocks], 1

    mov eax, offset _kernel_start_addr
    mov ebx, offset _start
    sub eax, ebx
    shr eax, 11 # divide by 2048
    add eax, [bi_boot_file_lba]
    mov [dap_start_lba], eax

    mov edi, 0x400000

    mov ecx, offset _kernel_size
    add ecx, 2047
    shr ecx, 11

load_next_cd_kernel_sector:
    mov dl, byte ptr [0x7000]
    mov si, offset dap
    mov ah, 0x42
    int 0x13
    jc kernel_load_failed

    push ecx
    push esi
    mov ecx, 2048 / 4 # 512 dwords = 2048 bytes
    movzx esi, word ptr [dap_buffer_addr]
    rep movsd [edi], [esi]
    pop esi
    pop ecx

    mov eax, [dap_start_lba]
    add eax, 1
    mov [dap_start_lba], eax

    sub ecx, 1
    jnz load_next_cd_kernel_sector

create_memory_map:
    lea di, es:[_memory_map]
    call do_e820

video_mode_config:
    call config_video_mode

enter_protected_mode_again:
    cli
    lgdt [gdt32info]
    mov eax, cr0
    or al, 1    # set protected mode bit
    mov cr0, eax

    push 0x8
    mov eax, offset stage_3
    push eax
    retf

spin32:
    jmp spin32

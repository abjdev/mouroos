use alloc::format;
use alloc::string::String;
use crate::drivers::mouse;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use crate::gui::framebuffer::Framebuffer;

pub fn show_welcome_screen(fb: &mut Framebuffer) {
    let mem_info = crate::memory::get_system_memory_info();
    let (_, heap_total) = crate::allocator::heap_stats();
    let total_ram_mb = mem_info.total_ram_bytes / (1024 * 1024);
    let heap_mb = heap_total / (1024 * 1024);

    let ram_str = if total_ram_mb >= 1024 {
        format!("{} MiB ({}.{} GiB)", total_ram_mb, total_ram_mb / 1024, ((total_ram_mb % 1024) * 10) / 1024)
    } else {
        format!("{} MiB", total_ram_mb)
    };

    let mut progress: usize = 0;
    let max_progress: usize = 100;

    // Drain any hardware self-test or initialization bytes from controller
    while crate::task::keyboard::pop_scancode().is_some() {}
    while mouse::pop_mouse_event().is_some() {}

    let mut last_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);

    // Initial render pass at 0%
    render_welcome_frame(fb, 0, &ram_str, heap_mb);
    fb.flush();

    // Run welcome splash animation loop
    loop {
        // Check for user skip inputs (Enter, Space, or any key press)
        while let Some(scancode) = crate::task::keyboard::pop_scancode() {
            // Only trigger on key down (< 0x80), ignore ACK/BAT (0xFA/0xAA)
            if scancode < 0x80 && scancode != 0x00 {
                progress = max_progress;
                break;
            }
        }

        while let Some(mouse_ev) = mouse::pop_mouse_event() {
            if mouse_ev.left_pressed || mouse_ev.right_pressed {
                progress = max_progress;
                break;
            }
        }

        if progress >= max_progress {
            // Render final 100% frame and briefly hold before launching desktop
            render_welcome_frame(fb, 100, &ram_str, heap_mb);
            fb.flush();

            let end_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
            while crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed) < end_tick + 6 {
                x86_64::instructions::hlt();
            }

            // Drain any pending input so skip key doesn't leak into terminal
            while crate::task::keyboard::pop_scancode().is_some() {}
            while mouse::pop_mouse_event().is_some() {}
            break;
        }

        // Render current progress frame
        render_welcome_frame(fb, progress, &ram_str, heap_mb);
        fb.flush();

        // Advance progress based on hardware timer ticks
        let current_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
        if current_tick != last_tick {
            let elapsed = current_tick.saturating_sub(last_tick) as usize;
            last_tick = current_tick;
            progress = (progress + elapsed * 3).min(max_progress);
        }

        // Idle CPU until next hardware interrupt (Timer, Keyboard, Mouse)
        x86_64::instructions::hlt();
    }
}

fn render_welcome_frame(fb: &mut Framebuffer, progress: usize, ram_str: &str, heap_mb: usize) {
    let width = fb.width;
    let _height = fb.height;

    // 1. Classic Windows 98 Teal canvas (#008080)
    fb.clear(Color::RETRO_TEAL);

    // 2. Retro OS Title & Branding
    let title = "MOUROS 98";
    let title_scale = 4;
    let title_w = title.len() * (FONT_WIDTH * title_scale);
    let title_x = (width as isize - title_w as isize) / 2;
    let title_y = 36;

    // Classic 3D Shadow & Highlight
    fb.draw_string_scaled(title_x + 2, title_y + 2, title, title_scale, Color::from_rgb(0, 96, 96));
    fb.draw_string_scaled(title_x, title_y, title, title_scale, Color::WHITE);

    let subtitle = "Bare-Metal 64-Bit Operating System (Second Edition)";
    let sub_w = subtitle.len() * FONT_WIDTH;
    let sub_x = (width as isize - sub_w as isize) / 2;
    fb.draw_string(sub_x + 1, 85, subtitle, Color::BLACK);
    fb.draw_string(sub_x, 84, subtitle, Color::WHITE);

    // 3. Centered 3D Raised System Diagnostics Dialog
    let card_w = 580;
    let card_h = 200;
    let card_x = (width as isize - card_w as isize) / 2;
    let card_y = 114;

    fb.fill_rect(card_x, card_y, card_w, card_h, Color::RETRO_FACE);
    fb.draw_bevel_raised(card_x, card_y, card_w, card_h);

    // Windows 98 Titlebar for Dialog
    fb.draw_gradient_h(
        card_x + 3,
        card_y + 3,
        card_w - 6,
        20,
        Color::RETRO_ACTIVE_TITLE_LEFT,
        Color::RETRO_ACTIVE_TITLE_RIGHT,
    );
    fb.draw_string(card_x + 10, card_y + 2, "System & Hardware Initialization", Color::WHITE);

    let diag_items: [(&str, String); 6] = [
        ("Bootloader", String::from("Mouros In-Tree MBR / Stage 2 / Stage 3 / Stage 4")),
        ("Processor", String::from("x86_64 Long Mode (Paging Active, SSE2 Enabled)")),
        ("Physical RAM", format!("{} [Kernel Heap: {} MB]", ram_str, heap_mb)),
        ("Video Device", String::from("Bochs BGA 800x600 32bpp True Color Linear FB")),
        ("Input Driver", String::from("Lock-Free PS/2 Mouse & Keyboard (IRQ 1 & 12)")),
        ("Real-Time", String::from("CMOS Hardware RTC & 8254 PIT Timer Active")),
    ];

    let mut item_y = card_y + 34;
    for (label, val) in &diag_items {
        fb.draw_string(card_x + 16, item_y, "[ OK ]", Color::from_rgb(0, 128, 0));
        fb.draw_string(card_x + 72, item_y, label, Color::BLACK);
        fb.draw_string(card_x + 172, item_y, ":", Color::BLACK);
        fb.draw_string(card_x + 184, item_y, val, Color::from_rgb(0, 0, 128));
        item_y += 24;
    }

    // 4. Loading Status Message
    let status_msg = if progress < 25 {
        "Initializing kernel paging and memory structures..."
    } else if progress < 50 {
        "Configuring Bochs BGA linear framebuffer & backbuffer..."
    } else if progress < 75 {
        "Starting lock-free mouse and keyboard event queues..."
    } else if progress < 95 {
        "Compositing desktop wallpaper, windows, and applications..."
    } else {
        "Mouros Desktop Environment Ready! Starting OS..."
    };

    let msg_w = status_msg.len() * FONT_WIDTH;
    let msg_x = (width as isize - msg_w as isize) / 2;
    fb.draw_string(msg_x + 1, 345, status_msg, Color::BLACK);
    fb.draw_string(msg_x, 344, status_msg, Color::WHITE);

    // 5. Classic Windows 98 Progress Bar (Sunken white track with discrete navy blocks)
    let bar_w = 460;
    let bar_h = 18;
    let bar_x = (width as isize - bar_w as isize) / 2;
    let bar_y = 368;

    fb.fill_rect(bar_x, bar_y, bar_w, bar_h, Color::WHITE);
    fb.draw_bevel_sunken(bar_x, bar_y, bar_w, bar_h);

    let fill_w = (bar_w.saturating_sub(6) * progress) / 100;
    let mut cur_bx = 0;
    while cur_bx + 10 <= fill_w {
        fb.fill_rect(
            bar_x + 3 + cur_bx as isize,
            bar_y + 3,
            8,
            bar_h - 6,
            Color::RETRO_SELECTION, // Windows 98 Navy Blocks
        );
        cur_bx += 10;
    }

    // Progress percentage label
    let pct_str = format!("{:3}%", progress);
    fb.draw_string(bar_x + bar_w as isize + 12, bar_y + 4, &pct_str, Color::WHITE);

    // 6. User Instruction Hints
    let hint1 = "Press [ENTER], [SPACE], or CLICK anywhere to launch Desktop";
    let h1_w = hint1.len() * FONT_WIDTH;
    let h1_x = (width as isize - h1_w as isize) / 2;
    fb.draw_string(h1_x + 1, 421, hint1, Color::BLACK);
    fb.draw_string(h1_x, 420, hint1, Color::WHITE);

    let hint2 = "Or wait for automatic system initialization...";
    let h2_w = hint2.len() * FONT_WIDTH;
    let h2_x = (width as isize - h2_w as isize) / 2;
    fb.draw_string(h2_x + 1, 441, hint2, Color::BLACK);
    fb.draw_string(h2_x, 440, hint2, Color::from_rgb(180, 220, 220));

    // 7. Render mouse cursor on top
    let mouse_state = mouse::get_mouse_state();
    fb.draw_cursor(mouse_state.x, mouse_state.y);
}

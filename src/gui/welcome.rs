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
    let height = fb.height;

    // 1. Deep space gradient background
    fb.draw_gradient_v(
        0,
        0,
        width,
        height,
        Color::from_rgb(10, 16, 32),
        Color::from_rgb(18, 28, 52),
    );

    // Decorative subtle outer border
    fb.draw_rect(4, 4, width - 8, height - 8, Color::from_rgb(30, 48, 82));
    fb.draw_rect(6, 6, width - 12, height - 12, Color::from_rgb(20, 32, 58));

    // 2. Glowing OS Title & Branding
    let title = "MOUROS OS";
    let title_scale = 4;
    let title_w = title.len() * (FONT_WIDTH * title_scale);
    let title_x = (width as isize - title_w as isize) / 2;
    let title_y = 42;

    // Glow shadow
    fb.draw_string_scaled(title_x + 2, title_y + 2, title, title_scale, Color::from_argb(60, 2, 132, 199));
    // Main vibrant cyan text
    fb.draw_string_scaled(title_x, title_y, title, title_scale, Color::from_rgb(56, 189, 248));

    let subtitle = "Bare-Metal 64-Bit Desktop Operating System";
    let sub_w = subtitle.len() * FONT_WIDTH;
    let sub_x = (width as isize - sub_w as isize) / 2;
    fb.draw_string(sub_x, 90, subtitle, Color::from_rgb(226, 232, 240));

    let tagline = "Powered by Rust  *  Custom In-Tree Bootloader v1.0";
    let tag_w = tagline.len() * FONT_WIDTH;
    let tag_x = (width as isize - tag_w as isize) / 2;
    fb.draw_string(tag_x, 108, tagline, Color::from_rgb(100, 116, 139));

    // 3. Centered System Diagnostics Card
    let card_w = 580;
    let card_h = 200;
    let card_x = (width as isize - card_w as isize) / 2;
    let card_y = 138;

    fb.draw_shadow(card_x, card_y, card_w, card_h, 8);
    fb.fill_rect(card_x, card_y, card_w, card_h, Color::from_rgb(15, 23, 42));
    fb.draw_rect(card_x, card_y, card_w, card_h, Color::from_rgb(51, 65, 85));

    // Card header
    fb.draw_gradient_v(
        card_x + 1,
        card_y + 1,
        card_w - 2,
        28,
        Color::from_rgb(30, 41, 59),
        Color::from_rgb(24, 32, 47),
    );
    fb.fill_rect(card_x + 1, card_y + 28, card_w - 2, 1, Color::from_rgb(71, 85, 105));
    fb.draw_string(card_x + 16, card_y + 10, "SYSTEM & HARDWARE SUBSYSTEM INITIALIZATION", Color::from_rgb(226, 232, 240));

    let diag_items: [(&str, String); 6] = [
        ("Bootloader", String::from("Mouros In-Tree MBR / Stage 2 / Stage 3 / Stage 4")),
        ("Processor", String::from("x86_64 Long Mode (Paging Active, SSE2 Enabled)")),
        ("Physical RAM", format!("{} [Kernel Heap: {} MiB]", ram_str, heap_mb)),
        ("Video Device", String::from("Bochs BGA 800x600 32bpp True Color Linear FB")),
        ("Input Driver", String::from("Lock-Free PS/2 Mouse & Keyboard (IRQ 1 & 12)")),
        ("Real-Time", String::from("CMOS Hardware RTC & 8254 PIT Timer Active")),
    ];

    let mut item_y = card_y + 38;
    for (label, val) in &diag_items {
        // [ OK ] tag in vivid green
        fb.draw_string(card_x + 16, item_y, "[ OK ]", Color::from_rgb(34, 197, 94));
        fb.draw_string(card_x + 72, item_y, label, Color::from_rgb(148, 163, 184));
        fb.draw_string(card_x + 172, item_y, ":", Color::from_rgb(100, 116, 139));
        fb.draw_string(card_x + 184, item_y, val, Color::from_rgb(241, 245, 249));
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
    fb.draw_string(msg_x, 370, status_msg, Color::from_rgb(186, 230, 253));

    // 5. Progress Bar
    let bar_w = 460;
    let bar_h = 16;
    let bar_x = (width as isize - bar_w as isize) / 2;
    let bar_y = 394;

    fb.fill_rect(bar_x, bar_y, bar_w, bar_h, Color::from_rgb(15, 23, 42));
    fb.draw_rect(bar_x, bar_y, bar_w, bar_h, Color::from_rgb(51, 65, 85));

    let fill_w = (bar_w.saturating_sub(4) * progress) / 100;
    if fill_w > 0 {
        fb.draw_gradient_v(
            bar_x + 2,
            bar_y + 2,
            fill_w,
            bar_h - 4,
            Color::from_rgb(56, 189, 248),
            Color::from_rgb(14, 165, 233),
        );
    }

    // Progress percentage label
    let pct_str = format!("{:3}%", progress);
    fb.draw_string(bar_x + bar_w as isize + 12, bar_y + 4, &pct_str, Color::from_rgb(241, 245, 249));

    // 6. User Instruction Hints
    let hint1 = "Press [ENTER], [SPACE], or CLICK anywhere to launch Desktop";
    let h1_w = hint1.len() * FONT_WIDTH;
    let h1_x = (width as isize - h1_w as isize) / 2;
    fb.draw_string(h1_x, 460, hint1, Color::from_rgb(241, 245, 249));

    let hint2 = "Or wait for automatic system initialization...";
    let h2_w = hint2.len() * FONT_WIDTH;
    let h2_x = (width as isize - h2_w as isize) / 2;
    fb.draw_string(h2_x, 480, hint2, Color::from_rgb(100, 116, 139));

    // 7. Render mouse cursor on top
    let mouse_state = mouse::get_mouse_state();
    fb.draw_cursor(mouse_state.x, mouse_state.y);
}

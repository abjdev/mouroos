use super::super::color::Color;
use super::super::font::FONT_HEIGHT;
use super::super::framebuffer::Framebuffer;
use super::super::window::Application;
use crate::drivers::rtc;
use crate::interrupts::TICKS;
use alloc::format;
use core::sync::atomic::Ordering;
use pc_keyboard::DecodedKey;

pub struct SysInfoApp;

impl SysInfoApp {
    pub fn new() -> Self {
        SysInfoApp
    }
}

impl Application for SysInfoApp {
    fn title(&self) -> &str {
        "System Information"
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        // Windows 98 Classic Gray casing
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::RETRO_FACE);

        let padding = 12;
        let mut y = client_y + 10;

        // Header section styled like System Properties
        fb.draw_string(client_x + padding, y, "System:", Color::BLACK);
        fb.draw_string(client_x + padding + 1, y, "System:", Color::BLACK); // Bold
        fb.draw_string(client_x + padding + 70, y, "Mouros 98", Color::BLACK);
        fb.draw_string(client_x + padding + 71, y, "Mouros 98", Color::BLACK); // Bold
        y += 16;

        fb.draw_string(client_x + padding + 70, y, "Second Edition (Bare-Metal)", Color::BLACK);
        y += 14;

        // Etched groove divider
        let div_w = client_w.saturating_sub(padding as usize * 2);
        fb.draw_groove(client_x + padding, y, div_w, 2);
        y += 8;

        // Computer section
        fb.draw_string(client_x + padding, y, "Computer:", Color::BLACK);
        fb.draw_string(client_x + padding + 1, y, "Computer:", Color::BLACK); // Bold
        y += 16;

        let mem_info = crate::memory::get_system_memory_info();
        let (heap_used, heap_total) = crate::allocator::heap_stats();

        let ram_total_mb = (mem_info.total_ram_bytes / (1024 * 1024)).max(1);
        let ram_str = if ram_total_mb >= 1024 {
            let gib_int = ram_total_mb / 1024;
            let gib_dec = ((ram_total_mb % 1024) * 10) / 1024;
            format!("{} MB ({}.{} GB) of RAM", ram_total_mb, gib_int, gib_dec)
        } else {
            format!("{} MB Physical RAM", ram_total_mb)
        };

        let heap_used_mb = heap_used / (1024 * 1024);
        let heap_used_dec = ((heap_used % (1024 * 1024)) * 10) / (1024 * 1024);
        let heap_total_mb = heap_total / (1024 * 1024);
        let heap_str = format!("{}.{} / {} MB", heap_used_mb, heap_used_dec, heap_total_mb);

        let specs = [
            ("Processor:", "Genuine x86_64 CPU (Long Mode)"),
            ("Display:", "Bochs VBE / BGA 800x600 32bpp"),
            ("Memory:", ram_str.as_str()),
            ("Kernel Heap:", heap_str.as_str()),
            ("System Clock:", "100 Hz PIT (10ms tick)"),
        ];

        let val_offset = 96;
        for (label, value) in specs.iter() {
            fb.draw_string(client_x + padding + 10, y, label, Color::BLACK);
            fb.draw_string(
                client_x + padding + 10 + val_offset,
                y,
                value,
                Color::from_rgb(0, 0, 128), // Classic deep navy specs text
            );
            y += FONT_HEIGHT as isize + 3;
        }

        y += 4;

        // Etched groove divider
        fb.draw_groove(client_x + padding, y, div_w, 2);
        y += 8;

        // Memory Usage Gauge Bar (Sunken 3D trough)
        let heap_pct = if heap_total_mb > 0 {
            ((heap_used_mb * 100) / heap_total_mb).clamp(5, 100)
        } else {
            10
        };
        let mem_label = format!("Heap Allocation: {}%", heap_pct);
        fb.draw_string(client_x + padding, y, &mem_label, Color::BLACK);
        y += FONT_HEIGHT as isize + 3;

        let bar_width = client_w.saturating_sub(padding as usize * 2);
        let bar_height = 14;
        // Sunken track
        fb.fill_rect(client_x + padding, y, bar_width, bar_height, Color::WHITE);
        fb.draw_bevel_sunken(client_x + padding, y, bar_width, bar_height);

        // Filled blocks (classic Windows 98 discrete blocks)
        let fill_width = (bar_width.saturating_sub(4) * heap_pct as usize) / 100;
        let mut cur_bx = 0;
        while cur_bx + 8 <= fill_width {
            fb.fill_rect(
                client_x + padding + 2 + cur_bx as isize,
                y + 2,
                6,
                bar_height.saturating_sub(4),
                Color::RETRO_SELECTION, // Navy blue blocks
            );
            cur_bx += 8;
        }

        y += bar_height as isize + 10;

        // Live Uptime & RTC Clock
        let ticks = TICKS.load(Ordering::Relaxed);
        let seconds = ticks / 100;
        let uptime_str = format!("Uptime: {}s", seconds);
        fb.draw_string(client_x + padding, y, &uptime_str, Color::BLACK);

        let time = rtc::read_time();
        let rtc_str = format!(
            "{:02}:{:02}:{:02} {:04}-{:02}-{:02}",
            time.hours, time.minutes, time.seconds, time.year, time.month, time.day
        );
        let rtc_w = (rtc_str.len() * 8) as isize;
        let rtc_x = client_x + client_w as isize - padding - rtc_w;
        if rtc_x > client_x + padding + (uptime_str.len() * 8) as isize + 10 {
            fb.draw_string(rtc_x, y, &rtc_str, Color::BLACK);
        }
    }

    fn on_key(&mut self, _key: DecodedKey) {}
    fn on_mouse_click(&mut self, _local_x: isize, _local_y: isize, _left: bool) {}
}

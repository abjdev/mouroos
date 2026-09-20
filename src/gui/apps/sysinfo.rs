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
        // Soft dark background
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::from_rgb(20, 26, 38));

        let padding = 12;
        let mut y = client_y + 10;

        // Big Title
        fb.draw_string_scaled(
            client_x + padding,
            y,
            "MOUROS OS",
            2,
            Color::from_rgb(96, 165, 250),
        );
        y += 20;

        fb.draw_string(
            client_x + padding,
            y,
            "Bare-Metal 64-bit Rust Desktop OS",
            Color::from_rgb(148, 163, 184),
        );
        y += 14;

        // Divider line
        fb.fill_rect(
            client_x + padding,
            y,
            client_w.saturating_sub(padding as usize * 2),
            1,
            Color::from_rgb(51, 65, 85),
        );
        y += 10;

        // Hardware & OS specifications
        let mem_info = crate::memory::get_system_memory_info();
        let (heap_used, heap_total) = crate::allocator::heap_stats();

        let ram_total_mb = (mem_info.total_ram_bytes / (1024 * 1024)).max(1);
        let ram_str = if ram_total_mb >= 1024 {
            let gib_int = ram_total_mb / 1024;
            let gib_dec = ((ram_total_mb % 1024) * 10) / 1024;
            format!("{} MiB ({}.{} GiB)", ram_total_mb, gib_int, gib_dec)
        } else {
            format!("{} MiB Physical RAM", ram_total_mb)
        };

        let heap_used_mb = heap_used / (1024 * 1024);
        let heap_used_dec = ((heap_used % (1024 * 1024)) * 10) / (1024 * 1024);
        let heap_total_mb = heap_total / (1024 * 1024);
        let heap_str = format!("{}.{} / {} MiB", heap_used_mb, heap_used_dec, heap_total_mb);

        let specs = [
            ("OS:", "Mouros Desktop v0.2.0"),
            ("Kernel:", "x86_64 Long Mode (64-bit)"),
            ("Display:", "Bochs BGA (800x600 32bpp)"),
            ("RAM (QEMU):", ram_str.as_str()),
            ("Heap:", heap_str.as_str()),
            ("Timer:", "100 Hz (PIT 10ms)"),
        ];

        let val_offset = 96;
        for (label, value) in specs.iter() {
            fb.draw_string(client_x + padding, y, label, Color::from_rgb(148, 163, 184));
            fb.draw_string(
                client_x + padding + val_offset,
                y,
                value,
                Color::from_rgb(241, 245, 249),
            );
            y += FONT_HEIGHT as isize + 4;
        }

        y += 6;

        // Memory Usage Gauge Bar
        let heap_pct = if heap_total_mb > 0 {
            ((heap_used_mb * 100) / heap_total_mb).clamp(5, 100)
        } else {
            10
        };
        let mem_label = format!("Heap Allocation: {}.{} / {} MiB ({}%)", heap_used_mb, heap_used_dec, heap_total_mb, heap_pct);
        fb.draw_string(
            client_x + padding,
            y,
            &mem_label,
            Color::from_rgb(148, 163, 184),
        );
        y += FONT_HEIGHT as isize + 4;

        let bar_width = client_w.saturating_sub(padding as usize * 2);
        let bar_height = 12;
        // Background track
        fb.fill_rect(
            client_x + padding,
            y,
            bar_width,
            bar_height,
            Color::from_rgb(30, 41, 59),
        );
        fb.draw_rect(
            client_x + padding,
            y,
            bar_width,
            bar_height,
            Color::from_rgb(71, 85, 105),
        );

        // Filled portion
        let fill_width = (bar_width * heap_pct as usize) / 100;
        fb.fill_rect(
            client_x + padding + 1,
            y + 1,
            fill_width,
            bar_height.saturating_sub(2),
            Color::from_rgb(34, 197, 94),
        );

        y += bar_height as isize + 12;

        // Live Uptime & RTC Clock
        let ticks = TICKS.load(Ordering::Relaxed);
        let seconds = ticks / 100; // 100 Hz PIT (10ms precision)
        let uptime_str = format!("Uptime: {}s ({}t)", seconds, ticks);
        fb.draw_string(client_x + padding, y, &uptime_str, Color::from_rgb(56, 189, 248));

        let time = rtc::read_time();
        let rtc_str = format!(
            "{:02}:{:02}:{:02} {:04}-{:02}-{:02}",
            time.hours, time.minutes, time.seconds, time.year, time.month, time.day
        );
        let rtc_w = (rtc_str.len() * 8) as isize;
        let rtc_x = client_x + client_w as isize - padding - rtc_w;
        if rtc_x > client_x + padding + (uptime_str.len() * 8) as isize + 10 {
            fb.draw_string(
                rtc_x,
                y,
                &rtc_str,
                Color::from_rgb(251, 191, 36),
            );
        }
    }

    fn on_key(&mut self, _key: DecodedKey) {}
    fn on_mouse_click(&mut self, _local_x: isize, _local_y: isize, _left: bool) {}
}

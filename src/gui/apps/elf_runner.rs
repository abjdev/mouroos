use alloc::string::String;
use alloc::vec::Vec;
use crate::elf::execute_elf;
use crate::gui::color::Color;
use crate::gui::framebuffer::Framebuffer;
use crate::gui::window::Application;
use pc_keyboard::DecodedKey;

// Embed compiled ELF samples into the kernel binary
static HELLO_ELF: &[u8] = include_bytes!("../../../samples/hello.elf");
static FIBONACCI_ELF: &[u8] = include_bytes!("../../../samples/fibonacci.elf");
static MANDELBROT_ELF: &[u8] = include_bytes!("../../../samples/mandelbrot.elf");
static SYSBENCH_ELF: &[u8] = include_bytes!("../../../samples/sysbench.elf");

struct ElfEntry {
    name: &'static str,
    desc: &'static str,
    data: &'static [u8],
}

pub struct ElfRunnerApp {
    entries: [ElfEntry; 4],
    selected_idx: usize,
    output_lines: Vec<String>,
    status_text: String,
    scroll_offset: usize,
}

impl ElfRunnerApp {
    pub fn new() -> Self {
        let mut app = Self {
            entries: [
                ElfEntry {
                    name: "hello.elf",
                    desc: "Mouros Greeting & Host API test",
                    data: HELLO_ELF,
                },
                ElfEntry {
                    name: "fibonacci.elf",
                    desc: "Fibonacci Sequence Generator",
                    data: FIBONACCI_ELF,
                },
                ElfEntry {
                    name: "mandelbrot.elf",
                    desc: "ASCII Mandelbrot Fractal Generator",
                    data: MANDELBROT_ELF,
                },
                ElfEntry {
                    name: "sysbench.elf",
                    desc: "CPU & Memory Performance Benchmark",
                    data: SYSBENCH_ELF,
                },
            ],
            selected_idx: 0,
            output_lines: Vec::new(),
            status_text: String::from("Ready - Select a binary and click 'Run ELF'"),
            scroll_offset: 0,
        };
        app.output_lines.push(String::from("=== Mouros ELF 64-bit Userspace Subsystem ==="));
        app.output_lines.push(String::from("Select an executable from the left panel and click 'Run ELF'."));
        app.output_lines.push(String::from("Binaries execute with native x86_64 machine code via C MourosApi."));
        app
    }

    pub fn run_selected(&mut self) {
        let entry = &self.entries[self.selected_idx];
        self.output_lines.clear();
        self.output_lines.push(alloc::format!(">>> Executing: {} ({} bytes)...", entry.name, entry.data.len()));
        self.scroll_offset = 0;

        let start_ticks = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
        match execute_elf(entry.data) {
            Ok(res) => {
                let end_ticks = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                let duration_ms = (end_ticks - start_ticks) * 10; // ~100Hz PIT ticks

                for line in res.output.lines() {
                    self.output_lines.push(String::from(line));
                }
                self.output_lines.push(alloc::format!(">>> Process exited with code {} in ~{} ms", res.exit_code, duration_ms));
                self.status_text = alloc::format!("Finished: code {} ({} ms)", res.exit_code, duration_ms);
            }
            Err(e) => {
                self.output_lines.push(alloc::format!(">>> EXECUTION ERROR: {}", e));
                self.status_text = alloc::format!("Error: {}", e);
            }
        }

        // Auto scroll to end if output is long
        let visible_lines = 16;
        if self.output_lines.len() > visible_lines {
            self.scroll_offset = self.output_lines.len() - visible_lines;
        }
    }
}

impl Application for ElfRunnerApp {
    fn title(&self) -> &str {
        "ELF Executable Runner"
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        x: isize,
        y: isize,
        w: usize,
        h: usize,
    ) {
        // Background
        fb.fill_rect(x, y, w, h, Color::new(24, 26, 32));

        // Left panel: Executable list (width: 175)
        let list_w = 175;
        fb.fill_rect(x, y, list_w, h, Color::new(32, 35, 44));
        fb.fill_rect(x + list_w as isize, y, 1, h, Color::new(50, 55, 68));

        fb.draw_string(x + 10, y + 8, "EXECUTABLES", Color::new(140, 150, 175));

        for (i, entry) in self.entries.iter().enumerate() {
            let item_y = y + 28 + (i as isize * 46);
            let is_sel = i == self.selected_idx;

            if is_sel {
                fb.fill_rect(x + 6, item_y, list_w - 12, 40, Color::new(55, 80, 130));
                fb.draw_rect(x + 6, item_y, list_w - 12, 40, Color::new(100, 140, 220));
            } else {
                fb.fill_rect(x + 6, item_y, list_w - 12, 40, Color::new(40, 44, 56));
            }

            // Name
            let name_color = if is_sel { Color::WHITE } else { Color::new(210, 215, 225) };
            fb.draw_string(x + 12, item_y + 6, entry.name, name_color);

            // Size badge
            let sz_str = alloc::format!("{} B", entry.data.len());
            fb.draw_string(x + 12, item_y + 22, &sz_str, Color::new(130, 140, 160));
        }

        // Run Button in left panel bottom
        let btn_y = y + h.saturating_sub(45) as isize;
        fb.fill_rect(x + 10, btn_y, list_w - 20, 32, Color::new(40, 130, 70));
        fb.draw_rect(x + 10, btn_y, list_w - 20, 32, Color::new(70, 190, 110));
        fb.draw_string(x + 38, btn_y + 8, "[ RUN ELF ]", Color::WHITE);

        // Right panel: Header & Console output
        let console_x = x + list_w as isize + 1;
        let console_w = w.saturating_sub(list_w + 1);

        // Status bar at top of right panel
        fb.fill_rect(console_x, y, console_w, 28, Color::new(30, 33, 42));
        fb.fill_rect(console_x, y + 28, console_w, 1, Color::new(50, 55, 68));
        let status_desc = self.entries[self.selected_idx].desc;
        let disp = alloc::format!("{}: {}", status_desc, self.status_text);
        fb.draw_string(console_x + 10, y + 8, &disp, Color::new(200, 215, 240));

        // Console background
        let term_y = y + 29;
        let term_h = h.saturating_sub(29);
        fb.fill_rect(console_x, term_y, console_w, term_h, Color::new(16, 18, 22));

        // Console text
        let max_visible = term_h / 16;
        let start = self.scroll_offset.min(self.output_lines.len().saturating_sub(1));
        let end = (start + max_visible).min(self.output_lines.len());

        for (row, line_idx) in (start..end).enumerate() {
            let line = &self.output_lines[line_idx];
            let py = term_y + 6 + (row as isize * 16);
            let color = if line.starts_with(">>>") {
                Color::new(100, 220, 130)
            } else if line.starts_with("===") {
                Color::new(100, 180, 255)
            } else if line.contains("ERROR") {
                Color::new(255, 90, 90)
            } else {
                Color::new(220, 225, 230)
            };

            // Clip text to window width
            let max_chars = console_w.saturating_sub(20) / 8;
            if line.len() > max_chars && max_chars > 3 {
                let mut clipped = String::from(&line[..max_chars - 3]);
                clipped.push_str("...");
                fb.draw_string(console_x + 10, py, &clipped, color);
            } else {
                fb.draw_string(console_x + 10, py, line, color);
            }
        }
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('w') | DecodedKey::Unicode('W') => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                }
            }
            DecodedKey::Unicode('s') | DecodedKey::Unicode('S') => {
                if self.selected_idx + 1 < self.entries.len() {
                    self.selected_idx += 1;
                }
            }
            DecodedKey::Unicode('\n') | DecodedKey::Unicode('\r') => {
                self.run_selected();
            }
            DecodedKey::Unicode('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            DecodedKey::Unicode('j') => {
                if self.scroll_offset + 1 < self.output_lines.len() {
                    self.scroll_offset += 1;
                }
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowUp) => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                }
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowDown) => {
                if self.selected_idx + 1 < self.entries.len() {
                    self.selected_idx += 1;
                }
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, local_x: isize, local_y: isize, left: bool) {
        if !left {
            return;
        }
        let list_w = 175;
        if local_x >= 0 && local_x < list_w {
            for i in 0..self.entries.len() {
                let item_y = 28 + (i as isize * 46);
                if local_y >= item_y && local_y <= item_y + 40 {
                    self.selected_idx = i;
                    return;
                }
            }

            // Check Run Button
            if local_y >= 210 && local_y <= 270 && local_x >= 10 && local_x <= list_w - 10 {
                self.run_selected();
            }
        }
    }
}

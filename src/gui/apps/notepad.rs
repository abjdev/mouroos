use super::super::color::Color;
use super::super::font::{FONT_HEIGHT, FONT_WIDTH};
use super::super::framebuffer::Framebuffer;
use super::super::window::Application;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use pc_keyboard::{DecodedKey, KeyCode};

pub struct NotepadApp {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    cursor_visible: bool,
    blink_counter: usize,
}

impl NotepadApp {
    pub fn new() -> Self {
        let mut lines = Vec::new();
        lines.push(String::from("Welcome to Mouros OS!"));
        lines.push(String::from("--------------------"));
        lines.push(String::from("Graphical text editor"));
        lines.push(String::from("built on bare-metal Rust."));
        lines.push(String::new());
        lines.push(String::from("Features:"));
        lines.push(String::from(" * Interactive typing"));
        lines.push(String::from(" * Arrow navigation"));
        lines.push(String::from(" * Multiline buffer"));
        lines.push(String::new());
        lines.push(String::from("Try editing this text!"));

        NotepadApp {
            lines,
            cursor_row: 10,
            cursor_col: 22,
            cursor_visible: true,
            blink_counter: 0,
        }
    }
}

impl Application for NotepadApp {
    fn title(&self) -> &str {
        "Notepad"
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        let status_h = 22;
        let text_h = client_h.saturating_sub(status_h);

        // Windows 98 Notepad pure white canvas
        fb.fill_rect(client_x, client_y, client_w, text_h, Color::WHITE);

        let line_height = FONT_HEIGHT as isize + 2;
        let padding_y = 4;
        let mut y = client_y + padding_y;
        let max_chars = client_w.saturating_sub(12) / FONT_WIDTH;

        for (idx, line) in self.lines.iter().enumerate() {
            if y + line_height > client_y + text_h as isize {
                break;
            }

            // Line content in crisp black font
            let text_x = client_x + 6;
            let display_text = if line.len() > max_chars {
                let end = line.char_indices().nth(max_chars).map(|(i, _)| i).unwrap_or(line.len());
                &line[..end]
            } else {
                line.as_str()
            };
            fb.draw_string(text_x, y, display_text, Color::BLACK);

            // Render classic Windows 98 black vertical bar cursor
            if idx == self.cursor_row && self.cursor_visible && self.cursor_col <= max_chars {
                let cur_x = text_x + (self.cursor_col * FONT_WIDTH) as isize;
                fb.fill_rect(cur_x, y, 1, FONT_HEIGHT, Color::BLACK);
            }

            y += line_height;
        }

        // Status bar at bottom (#C0C0C0 with sunken panel)
        let status_y = client_y + text_h as isize;
        fb.fill_rect(client_x, status_y, client_w, status_h, Color::RETRO_FACE);
        fb.fill_rect(client_x, status_y, client_w, 1, Color::RETRO_LIGHT);
        fb.draw_sunken_panel(client_x + client_w as isize - 110, status_y + 2, 106, 18);
        let status_str = format!("Ln {}, Col {}", self.cursor_row + 1, self.cursor_col + 1);
        fb.draw_string(client_x + client_w as isize - 104, status_y + 3, &status_str, Color::BLACK);
    }

    fn on_key(&mut self, key: DecodedKey) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = self.cursor_row.min(self.lines.len() - 1);
        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());

        match key {
            DecodedKey::Unicode(c) => match c {
                '\n' | '\r' => {
                    if self.lines.len() < 500 {
                        let current_line = &self.lines[self.cursor_row];
                        let rest = if self.cursor_col < current_line.len() {
                            current_line[self.cursor_col..].into()
                        } else {
                            String::new()
                        };
                        self.lines[self.cursor_row].truncate(self.cursor_col);
                        self.cursor_row += 1;
                        self.cursor_col = 0;
                        self.lines.insert(self.cursor_row, rest);
                    }
                }
                '\u{0008}' => {
                    // Backspace
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                        if self.cursor_col < self.lines[self.cursor_row].len() {
                            self.lines[self.cursor_row].remove(self.cursor_col);
                        }
                    } else if self.cursor_row > 0 {
                        let current = self.lines.remove(self.cursor_row);
                        self.cursor_row -= 1;
                        self.cursor_col = self.lines[self.cursor_row].len();
                        self.lines[self.cursor_row].push_str(&current);
                    }
                }
                c if c >= ' ' && c <= '~' => {
                    if self.lines[self.cursor_row].len() < 256 {
                        if self.cursor_col <= self.lines[self.cursor_row].len() {
                            self.lines[self.cursor_row].insert(self.cursor_col, c);
                            self.cursor_col += 1;
                        }
                    }
                }
                _ => {}
            },
            DecodedKey::RawKey(code) => match code {
                KeyCode::ArrowLeft => {
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                    }
                }
                KeyCode::ArrowRight => {
                    if self.cursor_col < self.lines[self.cursor_row].len() {
                        self.cursor_col += 1;
                    }
                }
                KeyCode::ArrowUp => {
                    if self.cursor_row > 0 {
                        self.cursor_row -= 1;
                        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                    }
                }
                KeyCode::ArrowDown => {
                    if self.cursor_row + 1 < self.lines.len() {
                        self.cursor_row += 1;
                        self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
                    }
                }
                _ => {}
            },
        }
    }

    fn on_mouse_click(&mut self, _local_x: isize, _local_y: isize, _left: bool) {}

    fn on_tick(&mut self) -> bool {
        self.blink_counter += 1;
        if self.blink_counter >= 40 {
            self.blink_counter = 0;
            self.cursor_visible = !self.cursor_visible;
            true
        } else {
            false
        }
    }
}

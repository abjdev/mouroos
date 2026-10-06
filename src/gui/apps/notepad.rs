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
    pub current_file: String,
    pub modified: bool,
    pub status_msg: Option<(String, usize)>,
}

impl NotepadApp {
    pub fn new() -> Self {
        Self::open_file("/home/user/notes.txt")
    }

    pub fn open_file(path: &str) -> Self {
        let mut lines = Vec::new();
        let current_file = String::from(path);

        if let Ok(bytes) = crate::fs::read(path) {
            if let Ok(text) = core::str::from_utf8(&bytes) {
                for l in text.lines() {
                    lines.push(String::from(l));
                }
            }
        }

        if lines.is_empty() {
            lines.push(String::from("Welcome to Mouros OS!"));
            lines.push(String::from("--------------------"));
            lines.push(String::from("Graphical text editor"));
            lines.push(String::from("built on bare-metal Rust."));
            lines.push(String::new());
            lines.push(String::from("Press F2 or Ctrl+S to save!"));
        }

        let cursor_row = lines.len().saturating_sub(1);
        let cursor_col = lines.last().map(|l| l.len()).unwrap_or(0);

        NotepadApp {
            lines,
            cursor_row,
            cursor_col,
            cursor_visible: true,
            blink_counter: 0,
            current_file,
            modified: false,
            status_msg: None,
        }
    }

    pub fn save_file(&mut self) -> bool {
        let mut text = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            text.push_str(line);
            if i + 1 < self.lines.len() || !line.is_empty() {
                text.push('\n');
            }
        }
        match crate::fs::write(&self.current_file, text.as_bytes()) {
            Ok(_) => {
                let _ = crate::fs::sync();
                self.modified = false;
                self.status_msg = Some((String::from("Saved"), 60));
                true
            }
            Err(_) => {
                self.status_msg = Some((String::from("Save Error"), 90));
                false
            }
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

        // Left status panel: file name & save indicator
        let right_panel_w = 110;
        let left_panel_w = client_w.saturating_sub(right_panel_w + 6);
        fb.draw_sunken_panel(client_x + 2, status_y + 2, left_panel_w, 18);

        let file_status = if let Some((msg, _)) = &self.status_msg {
            format!("{} [{}]", self.current_file, msg)
        } else if self.modified {
            format!("{}*", self.current_file)
        } else {
            self.current_file.clone()
        };
        let max_left_chars = left_panel_w.saturating_sub(8) / FONT_WIDTH;
        let disp_status = if file_status.len() > max_left_chars {
            &file_status[..max_left_chars]
        } else {
            &file_status
        };
        fb.draw_string(client_x + 6, status_y + 3, disp_status, Color::BLACK);

        // Right status panel: Ln / Col
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
                '\x13' => {
                    // Ctrl+S: Save file
                    self.save_file();
                }
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
                        self.modified = true;
                    }
                }
                '\u{0008}' => {
                    // Backspace
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                        if self.cursor_col < self.lines[self.cursor_row].len() {
                            self.lines[self.cursor_row].remove(self.cursor_col);
                            self.modified = true;
                        }
                    } else if self.cursor_row > 0 {
                        let current = self.lines.remove(self.cursor_row);
                        self.cursor_row -= 1;
                        self.cursor_col = self.lines[self.cursor_row].len();
                        self.lines[self.cursor_row].push_str(&current);
                        self.modified = true;
                    }
                }
                c if c >= ' ' && c <= '~' => {
                    if self.lines[self.cursor_row].len() < 256 {
                        if self.cursor_col <= self.lines[self.cursor_row].len() {
                            self.lines[self.cursor_row].insert(self.cursor_col, c);
                            self.cursor_col += 1;
                            self.modified = true;
                        }
                    }
                }
                _ => {}
            },
            DecodedKey::RawKey(code) => match code {
                KeyCode::F2 => {
                    self.save_file();
                }
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
        if let Some((_, ref mut count)) = self.status_msg {
            if *count > 0 {
                *count -= 1;
            } else {
                self.status_msg = None;
            }
        }

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

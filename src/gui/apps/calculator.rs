use super::super::color::Color;
use super::super::font::{FONT_HEIGHT, FONT_WIDTH};
use super::super::framebuffer::Framebuffer;
use super::super::window::Application;
use alloc::format;
use alloc::string::String;
use pc_keyboard::{DecodedKey, KeyCode};

const BUTTONS: [[&str; 4]; 4] = [
    ["7", "8", "9", "/"],
    ["4", "5", "6", "*"],
    ["1", "2", "3", "-"],
    ["C", "0", "=", "+"],
];

pub struct CalculatorApp {
    display: String,
    accumulator: i64,
    pending_op: Option<char>,
    clear_on_next_digit: bool,
}

impl CalculatorApp {
    pub fn new() -> Self {
        CalculatorApp {
            display: String::from("0"),
            accumulator: 0,
            pending_op: None,
            clear_on_next_digit: false,
        }
    }

    fn push_digit(&mut self, digit: char) {
        if self.clear_on_next_digit || self.display == "0" {
            self.display.clear();
            self.clear_on_next_digit = false;
        }

        if self.display.len() < 12 {
            self.display.push(digit);
        }
    }

    fn apply_op(&mut self, op: char) {
        let current_val = self.display.parse::<i64>().unwrap_or(0);

        if let Some(pending) = self.pending_op {
            self.accumulator = match pending {
                '+' => self.accumulator.saturating_add(current_val),
                '-' => self.accumulator.saturating_sub(current_val),
                '*' => self.accumulator.saturating_mul(current_val),
                '/' => {
                    if current_val != 0 {
                        self.accumulator / current_val
                    } else {
                        0
                    }
                }
                _ => current_val,
            };
        } else {
            self.accumulator = current_val;
        }

        self.pending_op = Some(op);
        self.clear_on_next_digit = true;
        self.display = format!("{}", self.accumulator);
    }

    fn calculate_equals(&mut self) {
        let current_val = self.display.parse::<i64>().unwrap_or(0);
        if let Some(op) = self.pending_op {
            let res = match op {
                '+' => self.accumulator.saturating_add(current_val),
                '-' => self.accumulator.saturating_sub(current_val),
                '*' => self.accumulator.saturating_mul(current_val),
                '/' => {
                    if current_val != 0 {
                        self.accumulator / current_val
                    } else {
                        0
                    }
                }
                _ => current_val,
            };
            self.display = format!("{}", res);
            self.accumulator = res;
            self.pending_op = None;
            self.clear_on_next_digit = true;
        }
    }

    fn clear(&mut self) {
        self.display = String::from("0");
        self.accumulator = 0;
        self.pending_op = None;
        self.clear_on_next_digit = false;
    }

    fn handle_action(&mut self, action: &str) {
        match action {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                self.push_digit(action.chars().next().unwrap());
            }
            "+" | "-" | "*" | "/" => {
                self.apply_op(action.chars().next().unwrap());
            }
            "=" => {
                self.calculate_equals();
            }
            "C" => {
                self.clear();
            }
            _ => {}
        }
    }
}

impl Application for CalculatorApp {
    fn title(&self) -> &str {
        "Calculator"
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        // Calculator casing background
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::from_rgb(30, 36, 48));

        let padding = 12;
        let display_height = 40;
        let display_width = client_w.saturating_sub(padding * 2);

        // Display screen LCD box
        let disp_x = client_x + padding as isize;
        let disp_y = client_y + padding as isize;
        fb.fill_rect(disp_x, disp_y, display_width, display_height, Color::from_rgb(15, 23, 42));
        fb.draw_rect(disp_x, disp_y, display_width, display_height, Color::from_rgb(71, 85, 105));

        // Display numbers right-aligned
        let num_str = &self.display;
        let text_width = num_str.len() * (FONT_WIDTH * 2);
        let text_x = (disp_x + display_width as isize - 12 - text_width as isize).max(disp_x + 8);
        let text_y = disp_y + 12;
        fb.draw_string_scaled(text_x, text_y, num_str, 2, Color::from_rgb(52, 211, 153)); // LCD green

        // 4x4 Button Grid
        let grid_top = disp_y + display_height as isize + 12;
        let available_w = client_w.saturating_sub(padding * 2);
        let available_h = client_h.saturating_sub((grid_top - client_y) as usize + padding);

        let btn_spacing = 8;
        let btn_w = (available_w.saturating_sub(btn_spacing * 3)) / 4;
        let btn_h = (available_h.saturating_sub(btn_spacing * 3)) / 4;

        for (row_idx, row) in BUTTONS.iter().enumerate() {
            for (col_idx, &label) in row.iter().enumerate() {
                let bx = disp_x + (col_idx * (btn_w + btn_spacing)) as isize;
                let by = grid_top + (row_idx * (btn_h + btn_spacing)) as isize;

                let btn_bg = match label {
                    "C" => Color::from_rgb(220, 38, 38), // red
                    "+" | "-" | "*" | "/" | "=" => Color::from_rgb(234, 88, 12), // vibrant orange
                    _ => Color::from_rgb(51, 65, 85), // dark slate
                };

                fb.fill_rect(bx, by, btn_w, btn_h, btn_bg);
                fb.draw_rect(bx, by, btn_w, btn_h, Color::from_rgb(100, 116, 139));

                // Button label centered
                let lbl_x = bx + (btn_w as isize - FONT_WIDTH as isize) / 2;
                let lbl_y = by + (btn_h as isize - FONT_HEIGHT as isize) / 2;
                fb.draw_string(lbl_x, lbl_y, label, Color::WHITE);
            }
        }
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode(c) => match c {
                '0'..='9' => self.push_digit(c),
                '+' | '-' | '*' | '/' => self.apply_op(c),
                '=' | '\n' | '\r' => self.calculate_equals(),
                'c' | 'C' => self.clear(),
                _ => {}
            },
            DecodedKey::RawKey(KeyCode::Backspace) => {
                if self.display.len() > 1 {
                    self.display.pop();
                } else {
                    self.display = String::from("0");
                }
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, local_x: isize, local_y: isize, left: bool) {
        if !left {
            return;
        }

        let padding = 12 as isize;
        let display_height = 40 as isize;
        let grid_top = padding + display_height + 12;

        if local_y < grid_top {
            return;
        }

        // Window width is roughly 240, height 300
        let btn_spacing = 8 as isize;
        let btn_w = 44 as isize;
        let btn_h = 36 as isize;

        for (row_idx, row) in BUTTONS.iter().enumerate() {
            for (col_idx, &label) in row.iter().enumerate() {
                let bx = padding + (col_idx as isize * (btn_w + btn_spacing));
                let by = grid_top + (row_idx as isize * (btn_h + btn_spacing));

                if local_x >= bx && local_x < bx + btn_w && local_y >= by && local_y < by + btn_h {
                    self.handle_action(label);
                    return;
                }
            }
        }
    }
}

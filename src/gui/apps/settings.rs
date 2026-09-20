use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use crate::gui::framebuffer::Framebuffer;
use crate::gui::theme::ThemeKind;
use crate::gui::window::Application;
use pc_keyboard::DecodedKey;

pub struct SettingsApp {
    pub selected_theme: ThemeKind,
    pub applied_theme: ThemeKind,
    pub pending_apply: bool,
}

impl SettingsApp {
    pub fn new() -> Self {
        Self {
            selected_theme: ThemeKind::DeepSpace,
            applied_theme: ThemeKind::DeepSpace,
            pending_apply: false,
        }
    }
}

impl Application for SettingsApp {
    fn title(&self) -> &str {
        "Desktop Settings & Appearance"
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('1') => self.selected_theme = ThemeKind::DeepSpace,
            DecodedKey::Unicode('2') => self.selected_theme = ThemeKind::CyberpunkNeon,
            DecodedKey::Unicode('3') => self.selected_theme = ThemeKind::MatrixEmerald,
            DecodedKey::Unicode('4') => self.selected_theme = ThemeKind::RetroSunset,
            DecodedKey::Unicode('\n') => {
                self.applied_theme = self.selected_theme;
                self.pending_apply = true;
                crate::gui::theme::set_theme(self.selected_theme);
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, x: isize, y: isize, left: bool) {
        if !left {
            return;
        }

        // Check Theme Cards (x: 14..320, 4 cards from y: 35..175)
        let card_h = 32;
        let themes = [
            (ThemeKind::DeepSpace, 35),
            (ThemeKind::CyberpunkNeon, 72),
            (ThemeKind::MatrixEmerald, 109),
            (ThemeKind::RetroSunset, 146),
        ];

        for (kind, cy) in themes {
            if x >= 14 && x <= 320 && y >= cy && y <= cy + card_h {
                self.selected_theme = kind;
                return;
            }
        }

        // Check Apply Button (x: 14..120, y: 195..222)
        if x >= 14 && x <= 120 && y >= 195 && y <= 222 {
            self.applied_theme = self.selected_theme;
            self.pending_apply = true;
            crate::gui::theme::set_theme(self.selected_theme);
        }
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        bx: isize,
        by: isize,
        bw: usize,
        bh: usize,
    ) {

        // Card background
        fb.fill_rect(bx, by, bw, bh, Color::from_rgb(15, 23, 42));

        // Header
        fb.draw_string(bx + 14, by + 12, "SELECT DESKTOP THEME & COLOR SCHEME:", Color::from_rgb(148, 163, 184));

        // 4 Theme Cards
        let themes = [
            (ThemeKind::DeepSpace, "1. Deep Space", Color::from_rgb(56, 189, 248), Color::from_rgb(2, 132, 199), 35),
            (ThemeKind::CyberpunkNeon, "2. Cyberpunk Neon", Color::from_rgb(244, 63, 94), Color::from_rgb(217, 70, 239), 72),
            (ThemeKind::MatrixEmerald, "3. Matrix Emerald", Color::from_rgb(34, 197, 94), Color::from_rgb(22, 101, 52), 109),
            (ThemeKind::RetroSunset, "4. Retro Sunset", Color::from_rgb(245, 158, 11), Color::from_rgb(234, 88, 12), 146),
        ];

        for (kind, label, accent, sec, cy) in themes {
            let card_y = by + cy;
            let is_sel = self.selected_theme == kind;
            let is_app = self.applied_theme == kind;

            let bg = if is_sel {
                Color::from_rgb(30, 41, 59)
            } else {
                Color::from_rgb(20, 28, 43)
            };
            let border = if is_sel { accent } else { Color::from_rgb(51, 65, 85) };

            fb.fill_rect(bx + 14, card_y, bw - 28, 30, bg);
            fb.draw_rect(bx + 14, card_y, bw - 28, 30, border);

            // Color Swatches
            fb.fill_rect(bx + 22, card_y + 8, 14, 14, accent);
            fb.fill_rect(bx + 40, card_y + 8, 14, 14, sec);
            fb.draw_rect(bx + 22, card_y + 8, 14, 14, Color::WHITE);
            fb.draw_rect(bx + 40, card_y + 8, 14, 14, Color::WHITE);

            // Label
            let txt_c = if is_sel { Color::WHITE } else { Color::from_rgb(203, 213, 225) };
            fb.draw_string(bx + 64, card_y + 11, label, txt_c);

            // Status tag
            if is_app {
                fb.draw_string(bx + bw as isize - 90, card_y + 11, "[ACTIVE]", Color::from_rgb(34, 197, 94));
            } else if is_sel {
                fb.draw_string(bx + bw as isize - 100, card_y + 11, "[SELECTED]", Color::from_rgb(56, 189, 248));
            }
        }

        // Apply Button
        let btn_y = by + 192;
        let is_changed = self.selected_theme != self.applied_theme;
        let btn_bg = if is_changed {
            Color::from_rgb(37, 99, 235)
        } else {
            Color::from_rgb(51, 65, 85)
        };
        fb.fill_rect(bx + 14, btn_y, 110, 26, btn_bg);
        fb.draw_rect(bx + 14, btn_y, 110, 26, Color::from_rgb(96, 165, 250));
        let btn_text = if is_changed { "APPLY THEME" } else { "APPLIED (OK)" };
        let tw = btn_text.len() * FONT_WIDTH;
        let tx = bx + 14 + ((110 - tw as isize) / 2);
        fb.draw_string(tx, btn_y + 9, btn_text, Color::WHITE);

        // Information Footer
        fb.draw_string(
            bx + 14,
            by + bh as isize - 24,
            "Themes dynamically update titlebars, borders, and wallpaper.",
            Color::from_rgb(100, 116, 139),
        );
    }
}

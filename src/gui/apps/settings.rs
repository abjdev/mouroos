use crate::gui::color::Color;
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
            selected_theme: ThemeKind::Windows98,
            applied_theme: ThemeKind::Windows98,
            pending_apply: false,
        }
    }
}

impl Application for SettingsApp {
    fn title(&self) -> &str {
        "Display Properties"
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('1') => self.selected_theme = ThemeKind::Windows98,
            DecodedKey::Unicode('2') => self.selected_theme = ThemeKind::MacOS9,
            DecodedKey::Unicode('3') => self.selected_theme = ThemeKind::DeepSpace,
            DecodedKey::Unicode('4') => self.selected_theme = ThemeKind::CyberpunkNeon,
            DecodedKey::Unicode('5') => self.selected_theme = ThemeKind::MatrixEmerald,
            DecodedKey::Unicode('6') => self.selected_theme = ThemeKind::RetroSunset,
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

        // Theme list items inside sunken listbox (x: 18..350, y from 42 onwards, 24px each)
        let themes = [
            ThemeKind::Windows98,
            ThemeKind::MacOS9,
            ThemeKind::DeepSpace,
            ThemeKind::CyberpunkNeon,
            ThemeKind::MatrixEmerald,
            ThemeKind::RetroSunset,
        ];

        for (i, kind) in themes.iter().enumerate() {
            let item_y = 44 + (i as isize * 22);
            if x >= 18 && x <= 350 && y >= item_y && y < item_y + 22 {
                self.selected_theme = *kind;
                return;
            }
        }

        // Check Apply Button (x: 290..358, y: 195..220)
        if x >= 290 && x <= 358 && y >= 195 && y <= 220 {
            self.applied_theme = self.selected_theme;
            self.pending_apply = true;
            crate::gui::theme::set_theme(self.selected_theme);
        }

        // Check OK Button (x: 215..283, y: 195..220)
        if x >= 215 && x <= 283 && y >= 195 && y <= 220 {
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
        // Windows 98 Dialog Body (#C0C0C0)
        fb.fill_rect(bx, by, bw, bh, Color::RETRO_FACE);

        // 3D Notebook Tab at top: "Themes & Appearance"
        let tab_w = 140;
        let tab_h = 20;
        let tab_x = bx + 10;
        let tab_y = by + 6;
        // Tab body
        fb.fill_rect(tab_x, tab_y, tab_w, tab_h, Color::RETRO_FACE);
        fb.draw_bevel_raised(tab_x, tab_y, tab_w, tab_h + 1);
        fb.draw_string(tab_x + 10, tab_y + 5, "Themes & Scheme", Color::BLACK);

        // Main Tab Sheet Border (Grooved or raised border)
        let sheet_x = bx + 8;
        let sheet_y = tab_y + tab_h as isize - 1;
        let sheet_w = bw.saturating_sub(16);
        let sheet_h = bh.saturating_sub(tab_h + 38);
        fb.draw_bevel_raised(sheet_x, sheet_y, sheet_w, sheet_h);
        // Overwrite the line between tab and sheet so tab seamlessly connects
        fb.fill_rect(tab_x + 2, sheet_y, tab_w.saturating_sub(4), 1, Color::RETRO_FACE);

        // Label inside sheet
        fb.draw_string(sheet_x + 10, sheet_y + 10, "Select Desktop Theme Scheme:", Color::BLACK);

        // Sunken Listbox for Themes
        let list_x = sheet_x + 10;
        let list_y = sheet_y + 24;
        let list_w = sheet_w.saturating_sub(20);
        let list_h = 136;
        fb.fill_rect(list_x, list_y, list_w, list_h, Color::WHITE);
        fb.draw_bevel_sunken(list_x, list_y, list_w, list_h);

        let themes = [
            (ThemeKind::Windows98, "Windows 98 (Memphis Classic)", Color::RETRO_TEAL),
            (ThemeKind::MacOS9, "Mac OS 9 (Apple Platinum)", Color::RETRO_MACOS_PLATINUM),
            (ThemeKind::DeepSpace, "Deep Space (Modern Dark)", Color::from_rgb(56, 189, 248)),
            (ThemeKind::CyberpunkNeon, "Cyberpunk Neon (Synthwave)", Color::from_rgb(244, 63, 94)),
            (ThemeKind::MatrixEmerald, "Matrix Emerald (Green phosphor)", Color::from_rgb(34, 197, 94)),
            (ThemeKind::RetroSunset, "Retro Sunset (Amber glow)", Color::from_rgb(245, 158, 11)),
        ];

        for (i, (kind, label, accent)) in themes.iter().enumerate() {
            let item_y = list_y + 3 + (i as isize * 22);
            let is_sel = self.selected_theme == *kind;
            let is_app = self.applied_theme == *kind;

            if is_sel {
                // Windows 98 Selection Blue
                fb.fill_rect(list_x + 2, item_y, list_w - 4, 20, Color::RETRO_SELECTION);
            }

            // Accent color chip
            fb.fill_rect(list_x + 6, item_y + 4, 12, 12, *accent);
            fb.draw_bevel_sunken(list_x + 6, item_y + 4, 12, 12);

            let txt_color = if is_sel { Color::WHITE } else { Color::BLACK };
            fb.draw_string(list_x + 24, item_y + 5, label, txt_color);

            if is_app {
                let tag = if is_sel { "(Current)" } else { "[Current]" };
                let tag_color = if is_sel { Color::WHITE } else { Color::from_rgb(0, 128, 0) };
                fb.draw_string(list_x + list_w as isize - 76, item_y + 5, tag, tag_color);
            }
        }

        // Dialog Bottom Buttons (Windows 98 3D Beveled Buttons)
        let btn_y = by + bh as isize - 28;

        // OK Button
        let ok_x = bx + bw as isize - 156;
        fb.draw_button(ok_x, btn_y, 68, 22, false);
        fb.draw_string(ok_x + 24, btn_y + 6, "OK", Color::BLACK);

        // Apply Button
        let apply_x = bx + bw as isize - 80;
        let is_changed = self.selected_theme != self.applied_theme;
        fb.draw_button(apply_x, btn_y, 68, 22, false);
        let apply_color = if is_changed { Color::BLACK } else { Color::RETRO_SHADOW };
        fb.draw_string(apply_x + 16, btn_y + 6, "Apply", apply_color);
    }
}

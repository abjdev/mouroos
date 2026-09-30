use alloc::format;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use crate::gui::framebuffer::Framebuffer;
use crate::gui::theme::{self, Theme, ThemeKind, WallpaperKind};
use crate::gui::window::Application;
use pc_keyboard::DecodedKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Themes,
    Display,
    Sound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundDeviceKind {
    Ac97Pci,
    PcSpeaker,
}

pub struct SettingsApp {
    pub active_tab: SettingsTab,

    // Themes & Background State
    pub selected_theme: ThemeKind,
    pub applied_theme: ThemeKind,
    pub selected_wallpaper: WallpaperKind,
    pub applied_wallpaper: WallpaperKind,

    // Display State
    pub selected_res_idx: usize,
    pub applied_res_idx: usize,
    pub selected_color_depth: usize,
    pub selected_refresh_rate: usize,

    // Sound State
    pub selected_sound_device: SoundDeviceKind,
    pub master_volume: u8,
    pub is_muted: bool,
    pub test_sound_timer: u32,

    // Layout cache & Window control
    pub client_w: usize,
    pub client_h: usize,
    pub should_close: bool,
}

pub const RESOLUTIONS: [(&str, usize, usize, &str); 4] = [
    ("640 x 480", 640, 480, "VGA Standard (4:3)"),
    ("800 x 600", 800, 600, "SVGA Native (4:3)"),
    ("1024 x 768", 1024, 768, "XGA Desktop (4:3)"),
    ("1280 x 1024", 1280, 1024, "SXGA Workstation (5:4)"),
];

impl SettingsApp {
    pub fn new() -> Self {
        let cur_theme = theme::current_theme();
        let cur_wp = theme::current_wallpaper();
        let (cur_w, cur_h) = theme::current_resolution();
        let cur_res_idx = RESOLUTIONS
            .iter()
            .position(|(_, w, h, _)| *w == cur_w && *h == cur_h)
            .unwrap_or(1);
        let cur_vol = crate::drivers::ac97::get_master_volume();
        let cur_muted = crate::drivers::ac97::is_muted() || crate::drivers::speaker::is_muted();

        Self {
            active_tab: SettingsTab::Themes,
            selected_theme: cur_theme,
            applied_theme: cur_theme,
            selected_wallpaper: cur_wp,
            applied_wallpaper: cur_wp,
            selected_res_idx: cur_res_idx,
            applied_res_idx: cur_res_idx,
            selected_color_depth: 0,
            selected_refresh_rate: 0,
            selected_sound_device: if crate::drivers::ac97::is_available() {
                SoundDeviceKind::Ac97Pci
            } else {
                SoundDeviceKind::PcSpeaker
            },
            master_volume: cur_vol,
            is_muted: cur_muted,
            test_sound_timer: 0,
            client_w: 402,
            client_h: 304,
            should_close: false,
        }
    }

    pub fn apply_changes(&mut self) {
        // Apply theme and wallpaper
        self.applied_theme = self.selected_theme;
        self.applied_wallpaper = self.selected_wallpaper;
        self.applied_res_idx = self.selected_res_idx;
        theme::set_theme(self.selected_theme);
        theme::set_wallpaper(self.selected_wallpaper);

        // Apply display resolution
        let (_, w, h, _) = RESOLUTIONS[self.selected_res_idx];
        theme::set_pending_resolution(w, h);

        // Apply audio settings
        crate::drivers::ac97::set_master_volume(self.master_volume);
        crate::drivers::ac97::set_muted(self.is_muted);
        crate::drivers::speaker::set_muted(self.is_muted);
    }

    pub fn revert_changes(&mut self) {
        self.selected_theme = self.applied_theme;
        self.selected_wallpaper = self.applied_wallpaper;
        self.selected_res_idx = self.applied_res_idx;
        self.master_volume = crate::drivers::ac97::get_master_volume();
        self.is_muted = crate::drivers::ac97::is_muted() || crate::drivers::speaker::is_muted();
    }

    pub fn has_pending_changes(&self) -> bool {
        self.selected_theme != self.applied_theme
            || self.selected_wallpaper != self.applied_wallpaper
            || self.selected_res_idx != self.applied_res_idx
            || self.master_volume != crate::drivers::ac97::get_master_volume()
            || self.is_muted != (crate::drivers::ac97::is_muted() || crate::drivers::speaker::is_muted())
    }

    pub fn trigger_test_sound(&mut self) {
        match self.selected_sound_device {
            SoundDeviceKind::Ac97Pci => {
                crate::drivers::ac97::set_muted(false);
                crate::drivers::ac97::play_test_chime();
                self.test_sound_timer = 15;
            }
            SoundDeviceKind::PcSpeaker => {
                crate::drivers::speaker::set_muted(false);
                crate::drivers::speaker::play_tone(523); // C5
                self.test_sound_timer = 24;
            }
        }
    }
}

impl Application for SettingsApp {
    fn title(&self) -> &str {
        "System & Display Settings"
    }

    fn on_tick(&mut self) -> bool {
        if self.test_sound_timer > 0 {
            self.test_sound_timer -= 1;
            if self.selected_sound_device == SoundDeviceKind::PcSpeaker {
                match self.test_sound_timer {
                    16..=23 => crate::drivers::speaker::play_tone(523), // C5
                    8..=15 => crate::drivers::speaker::play_tone(659),  // E5
                    1..=7 => crate::drivers::speaker::play_tone(784),   // G5
                    0 => crate::drivers::speaker::mute(),
                    _ => {}
                }
            }
            return true;
        }
        false
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('1') => self.active_tab = SettingsTab::Themes,
            DecodedKey::Unicode('2') => self.active_tab = SettingsTab::Display,
            DecodedKey::Unicode('3') => self.active_tab = SettingsTab::Sound,
            DecodedKey::Unicode('\t') => {
                self.active_tab = match self.active_tab {
                    SettingsTab::Themes => SettingsTab::Display,
                    SettingsTab::Display => SettingsTab::Sound,
                    SettingsTab::Sound => SettingsTab::Themes,
                };
            }
            DecodedKey::Unicode('\n') | DecodedKey::Unicode('o') | DecodedKey::Unicode('O') => {
                self.apply_changes();
                self.should_close = true;
            }
            DecodedKey::Unicode('a') | DecodedKey::Unicode('A') => {
                self.apply_changes();
            }
            DecodedKey::Unicode('\x1b') => {
                self.revert_changes();
                self.should_close = true;
            }
            DecodedKey::Unicode(' ') => {
                if self.active_tab == SettingsTab::Sound {
                    self.trigger_test_sound();
                }
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowLeft) => {
                if self.active_tab == SettingsTab::Display && self.selected_res_idx > 0 {
                    self.selected_res_idx -= 1;
                } else if self.active_tab == SettingsTab::Sound {
                    self.master_volume = self.master_volume.saturating_sub(5);
                    crate::drivers::ac97::set_master_volume(self.master_volume);
                } else if self.active_tab == SettingsTab::Themes {
                    let all_wp = WallpaperKind::ALL;
                    if let Some(pos) = all_wp.iter().position(|&w| w == self.selected_wallpaper) {
                        let prev = if pos == 0 { all_wp.len() - 1 } else { pos - 1 };
                        self.selected_wallpaper = all_wp[prev];
                    }
                }
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowRight) => {
                if self.active_tab == SettingsTab::Display && self.selected_res_idx + 1 < RESOLUTIONS.len() {
                    self.selected_res_idx += 1;
                } else if self.active_tab == SettingsTab::Sound {
                    self.master_volume = (self.master_volume + 5).min(100);
                    crate::drivers::ac97::set_master_volume(self.master_volume);
                } else if self.active_tab == SettingsTab::Themes {
                    let all_wp = WallpaperKind::ALL;
                    if let Some(pos) = all_wp.iter().position(|&w| w == self.selected_wallpaper) {
                        let next = (pos + 1) % all_wp.len();
                        self.selected_wallpaper = all_wp[next];
                    }
                }
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowUp) => {
                if self.active_tab == SettingsTab::Themes {
                    let themes = [
                        ThemeKind::Windows98,
                        ThemeKind::MacOS9,
                        ThemeKind::DeepSpace,
                        ThemeKind::CyberpunkNeon,
                        ThemeKind::MatrixEmerald,
                        ThemeKind::RetroSunset,
                    ];
                    if let Some(pos) = themes.iter().position(|&t| t == self.selected_theme) {
                        let prev = if pos == 0 { themes.len() - 1 } else { pos - 1 };
                        self.selected_theme = themes[prev];
                        self.selected_wallpaper = self.selected_theme.default_wallpaper();
                    }
                }
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowDown) => {
                if self.active_tab == SettingsTab::Themes {
                    let themes = [
                        ThemeKind::Windows98,
                        ThemeKind::MacOS9,
                        ThemeKind::DeepSpace,
                        ThemeKind::CyberpunkNeon,
                        ThemeKind::MatrixEmerald,
                        ThemeKind::RetroSunset,
                    ];
                    if let Some(pos) = themes.iter().position(|&t| t == self.selected_theme) {
                        let next = (pos + 1) % themes.len();
                        self.selected_theme = themes[next];
                        self.selected_wallpaper = self.selected_theme.default_wallpaper();
                    }
                }
            }
            _ => {}
        }
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn on_mouse_click(&mut self, x: isize, y: isize, left: bool) {
        if !left {
            return;
        }

        // 1. Tab Bar Navigation (x: 8..270, y: 4..26)
        if y >= 4 && y <= 26 {
            if x >= 8 && x < 92 {
                self.active_tab = SettingsTab::Themes;
                return;
            } else if x >= 94 && x < 178 {
                self.active_tab = SettingsTab::Display;
                return;
            } else if x >= 180 && x < 264 {
                self.active_tab = SettingsTab::Sound;
                return;
            }
        }

        // 2. Dialog Bottom Action Buttons (aligned to self.client_w and self.client_h)
        let btn_y = self.client_h as isize - 28;
        if y >= btn_y - 2 && y <= btn_y + 26 {
            let cancel_x = self.client_w as isize - 232;
            let ok_x = self.client_w as isize - 156;
            let apply_x = self.client_w as isize - 80;

            // Cancel Button: (x: cancel_x .. cancel_x + 68)
            if x >= cancel_x && x <= cancel_x + 68 {
                self.revert_changes();
                self.should_close = true;
                return;
            }
            // OK Button: (x: ok_x .. ok_x + 68)
            if x >= ok_x && x <= ok_x + 68 {
                self.apply_changes();
                self.should_close = true;
                return;
            }
            // Apply Button: (x: apply_x .. apply_x + 68)
            if x >= apply_x && x <= apply_x + 68 {
                self.apply_changes();
                return;
            }
        }

        // 3. Tab-Specific Content Clicks
        let sheet_x = 8isize;
        let sheet_y = 26isize;

        match self.active_tab {
            SettingsTab::Themes => {
                // Theme list items: (x: 18..205, y: 46..140, 6 items, 15px each)
                let themes = [
                    ThemeKind::Windows98,
                    ThemeKind::MacOS9,
                    ThemeKind::DeepSpace,
                    ThemeKind::CyberpunkNeon,
                    ThemeKind::MatrixEmerald,
                    ThemeKind::RetroSunset,
                ];
                let theme_list_y = sheet_y + 20;
                for (i, &kind) in themes.iter().enumerate() {
                    let item_y = theme_list_y + (i as isize * 15);
                    if x >= sheet_x + 10 && x <= sheet_x + 195 && y >= item_y && y < item_y + 15 {
                        self.selected_theme = kind;
                        self.selected_wallpaper = kind.default_wallpaper();
                        return;
                    }
                }

                // Wallpaper list items: (x: 18..205, y: 156..252, 7 items, 13px each)
                let wp_list_y = sheet_y + 130;
                for (i, &kind) in WallpaperKind::ALL.iter().enumerate() {
                    let item_y = wp_list_y + (i as isize * 13);
                    if x >= sheet_x + 10 && x <= sheet_x + 195 && y >= item_y && y < item_y + 13 {
                        self.selected_wallpaper = kind;
                        return;
                    }
                }
            }

            SettingsTab::Display => {
                // Resolution Step Buttons:
                // [ ◄ Less ]: (x: 24..88, y: 168..194)
                if x >= 24 && x <= 88 && y >= 168 && y <= 194 {
                    if self.selected_res_idx > 0 {
                        self.selected_res_idx -= 1;
                    }
                    return;
                }
                // [ More ► ]: (x: 304..368, y: 168..194)
                if x >= 304 && x <= 368 && y >= 168 && y <= 194 {
                    if self.selected_res_idx + 1 < RESOLUTIONS.len() {
                        self.selected_res_idx += 1;
                    }
                    return;
                }
                // Slider Track Clicks: (x: 96..300, y: 168..194)
                if x >= 96 && x <= 300 && y >= 168 && y <= 194 {
                    let rel_x = x - 96;
                    let idx = ((rel_x * 4) / 200).clamp(0, 3) as usize;
                    self.selected_res_idx = idx;
                    return;
                }
            }

            SettingsTab::Sound => {
                // Device 1 (AC97): (x: 20..380, y: 50..74)
                if x >= 20 && x <= 380 && y >= 50 && y <= 74 {
                    self.selected_sound_device = SoundDeviceKind::Ac97Pci;
                    return;
                }
                // Device 2 (PC Speaker): (x: 20..380, y: 78..102)
                if x >= 20 && x <= 380 && y >= 78 && y <= 102 {
                    self.selected_sound_device = SoundDeviceKind::PcSpeaker;
                    return;
                }

                // Volume Controls:
                // [ - ] Button: (x: 24..54, y: 142..168)
                if x >= 24 && x <= 54 && y >= 142 && y <= 168 {
                    self.master_volume = self.master_volume.saturating_sub(5);
                    crate::drivers::ac97::set_master_volume(self.master_volume);
                    return;
                }
                // Volume Bar Direct Click: (x: 62..174, y: 142..168)
                if x >= 62 && x <= 174 && y >= 142 && y <= 168 {
                    let vol = (((x - 62) * 100) / 110).clamp(0, 100) as u8;
                    self.master_volume = vol;
                    crate::drivers::ac97::set_master_volume(self.master_volume);
                    return;
                }
                // [ + ] Button: (x: 178..208, y: 142..168)
                if x >= 178 && x <= 208 && y >= 142 && y <= 168 {
                    self.master_volume = (self.master_volume + 5).min(100);
                    crate::drivers::ac97::set_master_volume(self.master_volume);
                    return;
                }
                // Mute Checkbox: (x: 220..370, y: 142..168)
                if x >= 220 && x <= 370 && y >= 142 && y <= 168 {
                    self.is_muted = !self.is_muted;
                    crate::drivers::ac97::set_muted(self.is_muted);
                    crate::drivers::speaker::set_muted(self.is_muted);
                    return;
                }

                // [ Test Sound ] Button: (x: 24..128, y: 206..236)
                if x >= 24 && x <= 128 && y >= 206 && y <= 236 {
                    self.trigger_test_sound();
                    return;
                }
            }
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
        self.client_w = bw;
        self.client_h = bh;

        // Windows 98 Dialog Body
        fb.fill_rect(bx, by, bw, bh, Color::RETRO_FACE);

        // 1. Notebook Tab Strip at Top
        let tabs = [
            (SettingsTab::Themes, "Themes"),
            (SettingsTab::Display, "Display"),
            (SettingsTab::Sound, "Sound"),
        ];

        let tab_w = 82isize;
        let tab_h = 20usize;
        let mut tab_x = bx + 8;
        let tab_y = by + 6;

        for &(tab_kind, label) in &tabs {
            let is_active = self.active_tab == tab_kind;
            let current_h = if is_active { tab_h + 2 } else { tab_h };
            let current_y = if is_active { tab_y - 2 } else { tab_y };

            fb.fill_rect(tab_x, current_y, tab_w as usize, current_h, Color::RETRO_FACE);
            fb.draw_bevel_raised(tab_x, current_y, tab_w as usize, current_h);

            let lbl_w = label.len() * FONT_WIDTH;
            let lx = tab_x + ((tab_w - lbl_w as isize) / 2);
            let ly = current_y + 3;
            let text_color = if is_active { Color::BLACK } else { Color::from_rgb(80, 80, 80) };
            fb.draw_string(lx, ly, label, text_color);

            tab_x += tab_w + 4;
        }

        // 2. Main Property Sheet (Raised 3D Panel)
        let sheet_x = bx + 8;
        let sheet_y = by + 26;
        let sheet_w = bw.saturating_sub(16);
        let sheet_h = bh.saturating_sub(62);

        fb.draw_bevel_raised(sheet_x, sheet_y, sheet_w, sheet_h);

        // Erase line under the active tab to seamlessly merge tab into sheet
        let active_tab_offset = match self.active_tab {
            SettingsTab::Themes => 8,
            SettingsTab::Display => 8 + tab_w + 4,
            SettingsTab::Sound => 8 + (tab_w + 4) * 2,
        };
        fb.fill_rect(bx + active_tab_offset + 2, sheet_y, tab_w as usize - 4, 1, Color::RETRO_FACE);

        // 3. Tab-Specific Content
        match self.active_tab {
            SettingsTab::Themes => self.render_themes_tab(fb, sheet_x, sheet_y, sheet_w, sheet_h),
            SettingsTab::Display => self.render_display_tab(fb, sheet_x, sheet_y, sheet_w, sheet_h),
            SettingsTab::Sound => self.render_sound_tab(fb, sheet_x, sheet_y, sheet_w, sheet_h),
        }

        // 4. Dialog Bottom Action Buttons
        let btn_y = by + bh as isize - 28;
        let ok_x = bx + bw as isize - 156;
        let apply_x = bx + bw as isize - 80;
        let cancel_x = bx + bw as isize - 232;

        // Cancel Button
        fb.draw_button(cancel_x, btn_y, 68, 22, false);
        fb.draw_string(cancel_x + 12, btn_y + 3, "Cancel", Color::BLACK);

        // OK Button
        fb.draw_button(ok_x, btn_y, 68, 22, false);
        fb.draw_string(ok_x + 24, btn_y + 3, "OK", Color::BLACK);

        // Apply Button
        let is_changed = self.has_pending_changes();
        fb.draw_button(apply_x, btn_y, 68, 22, false);
        let apply_col = if is_changed { Color::BLACK } else { Color::RETRO_SHADOW };
        fb.draw_string(apply_x + 16, btn_y + 3, "Apply", apply_col);
    }
}

impl SettingsApp {
    fn render_themes_tab(&self, fb: &mut Framebuffer, sx: isize, sy: isize, _sw: usize, _sh: usize) {
        // --- Left Section: Themes Listbox ---
        fb.draw_string(sx + 10, sy + 6, "Theme Scheme:", Color::BLACK);
        let list_x = sx + 10;
        let list_y = sy + 20;
        let list_w = 190usize;
        let list_h = 94usize;
        fb.fill_rect(list_x, list_y, list_w, list_h, Color::WHITE);
        fb.draw_bevel_sunken(list_x, list_y, list_w, list_h);

        let themes = [
            (ThemeKind::Windows98, "Classic 98", Color::RETRO_TEAL),
            (ThemeKind::MacOS9, "Classic Platinum", Color::RETRO_MACOS_PLATINUM),
            (ThemeKind::DeepSpace, "Deep Space", Color::from_rgb(56, 189, 248)),
            (ThemeKind::CyberpunkNeon, "Cyberpunk Neon", Color::from_rgb(244, 63, 94)),
            (ThemeKind::MatrixEmerald, "Matrix Emerald", Color::from_rgb(34, 197, 94)),
            (ThemeKind::RetroSunset, "Retro Sunset", Color::from_rgb(245, 158, 11)),
        ];

        for (i, (kind, label, accent)) in themes.iter().enumerate() {
            let item_y = list_y + 2 + (i as isize * 15);
            let is_sel = self.selected_theme == *kind;
            let is_app = self.applied_theme == *kind;

            if is_sel {
                fb.fill_rect(list_x + 2, item_y, list_w - 4, 14, Color::RETRO_SELECTION);
            }

            // Color Chip
            fb.fill_rect(list_x + 4, item_y + 2, 10, 10, *accent);
            fb.draw_bevel_sunken(list_x + 4, item_y + 2, 10, 10);

            let txt_col = if is_sel { Color::WHITE } else { Color::BLACK };
            fb.draw_string(list_x + 18, item_y - 1, label, txt_col);

            if is_app {
                let badge = "[*]";
                let badge_col = if is_sel { Color::from_rgb(250, 204, 21) } else { Color::from_rgb(22, 101, 52) };
                fb.draw_string(list_x + list_w as isize - 28, item_y - 1, badge, badge_col);
            }
        }

        // --- Left Section: Wallpaper Listbox ---
        fb.draw_string(sx + 10, sy + 118, "Desktop Wallpaper:", Color::BLACK);
        let wp_x = sx + 10;
        let wp_y = sy + 132;
        let wp_w = 190usize;
        let wp_h = 96usize;
        fb.fill_rect(wp_x, wp_y, wp_w, wp_h, Color::WHITE);
        fb.draw_bevel_sunken(wp_x, wp_y, wp_w, wp_h);

        for (i, &kind) in WallpaperKind::ALL.iter().enumerate() {
            let item_y = wp_y + 2 + (i as isize * 13);
            let is_sel = self.selected_wallpaper == kind;
            let is_app = self.applied_wallpaper == kind;

            if is_sel {
                fb.fill_rect(wp_x + 2, item_y, wp_w - 4, 12, Color::RETRO_SELECTION);
            }

            // Swatch
            fb.fill_rect(wp_x + 4, item_y + 2, 8, 8, kind.preview_color());
            fb.draw_bevel_sunken(wp_x + 4, item_y + 2, 8, 8);

            let txt_col = if is_sel { Color::WHITE } else { Color::BLACK };
            let name_short = match kind {
                WallpaperKind::ClassicTeal => "Classic Teal",
                WallpaperKind::Clouds => "Clouds (Sky Blue)",
                WallpaperKind::Bliss => "Bliss (Hills)",
                WallpaperKind::MacOSPlatinum => "Platinum Pinstripe",
                WallpaperKind::MatrixGrid => "Matrix Rain",
                WallpaperKind::DeepSpace => "Deep Space Stars",
                WallpaperKind::RetroSunset => "Retro Sunset Grid",
            };
            fb.draw_string(wp_x + 16, item_y - 2, name_short, txt_col);

            if is_app {
                let badge_col = if is_sel { Color::from_rgb(250, 204, 21) } else { Color::from_rgb(22, 101, 52) };
                fb.draw_string(wp_x + wp_w as isize - 24, item_y - 2, "[*]", badge_col);
            }
        }

        // --- Right Section: Miniature CRT Monitor Preview ---
        let mon_x = sx + 212;
        let mon_y = sy + 10;
        let mon_w = 168usize;
        let mon_h = 138usize;

        // Beige CRT Monitor Casing
        fb.fill_rect(mon_x, mon_y, mon_w, mon_h, Color::from_rgb(215, 210, 195));
        fb.draw_bevel_raised(mon_x, mon_y, mon_w, mon_h);

        // Curved CRT Screen Frame
        let scr_x = mon_x + 8;
        let scr_y = mon_y + 8;
        let scr_w = mon_w - 16;
        let scr_h = mon_h - 32;
        fb.fill_rect(scr_x, scr_y, scr_w, scr_h, Color::BLACK);
        fb.draw_bevel_sunken(scr_x, scr_y, scr_w, scr_h);

        // Render Miniature Desktop inside Screen
        self.render_mini_desktop(fb, scr_x + 2, scr_y + 2, scr_w - 4, scr_h - 4);

        // Monitor Bevel Buttons & Power LED
        let led_x = mon_x + mon_w as isize - 20;
        let led_y = mon_y + mon_h as isize - 16;
        fb.fill_rect(led_x, led_y, 6, 6, Color::from_rgb(34, 197, 94)); // Green LED
        fb.fill_rect(led_x - 14, led_y + 1, 8, 4, Color::from_rgb(180, 175, 160)); // Power button

        // Monitor Stand Pedestal
        let ped_w = 64usize;
        let ped_x = mon_x + ((mon_w - ped_w) / 2) as isize;
        let ped_y = mon_y + mon_h as isize;
        fb.fill_rect(ped_x + 16, ped_y, 32, 6, Color::from_rgb(190, 185, 170));
        fb.fill_rect(ped_x, ped_y + 6, ped_w, 4, Color::from_rgb(175, 170, 155));
        fb.draw_bevel_raised(ped_x, ped_y + 6, ped_w, 4);

        // Theme Palette Summary Box below monitor
        let pal_x = sx + 212;
        let pal_y = sy + 160;
        let pal_w = 168usize;
        let pal_h = 68usize;
        fb.fill_rect(pal_x, pal_y, pal_w, pal_h, Color::RETRO_FACE);
        fb.draw_sunken_panel(pal_x, pal_y, pal_w, pal_h);

        let cur_th = Theme::get(self.selected_theme);
        fb.draw_string(pal_x + 6, pal_y + 4, cur_th.name, Color::BLACK);

        let desc_line = match self.selected_theme {
            ThemeKind::Windows98 => "Classic 98 style",
            ThemeKind::MacOS9 => "Platinum pinstripe",
            ThemeKind::DeepSpace => "Dark cosmic theme",
            ThemeKind::CyberpunkNeon => "Synthwave & neon",
            ThemeKind::MatrixEmerald => "Green phosphor CRT",
            ThemeKind::RetroSunset => "Sunset horizon amber",
        };
        fb.draw_string(pal_x + 6, pal_y + 20, desc_line, Color::from_rgb(70, 70, 70));

        // Color Swatches Bar
        let swatches = [
            cur_th.accent_color,
            cur_th.win_title_active_top,
            cur_th.win_title_active_bot,
            cur_th.taskbar_top,
            cur_th.wallpaper_top,
        ];
        for (si, &col) in swatches.iter().enumerate() {
            let cx = pal_x + 6 + (si as isize * 22);
            fb.fill_rect(cx, pal_y + 40, 18, 14, col);
            fb.draw_bevel_sunken(cx, pal_y + 40, 18, 14);
        }
    }

    fn render_display_tab(&self, fb: &mut Framebuffer, sx: isize, sy: isize, sw: usize, _sh: usize) {
        // --- 1. Centered CRT Monitor Illustration with Dynamic Desktop Resolution Scaling ---
        let mon_w = 180usize;
        let mon_h = 114usize;
        let mon_x = sx + ((sw - mon_w) / 2) as isize;
        let mon_y = sy + 6;

        fb.fill_rect(mon_x, mon_y, mon_w, mon_h, Color::from_rgb(215, 210, 195));
        fb.draw_bevel_raised(mon_x, mon_y, mon_w, mon_h);

        let scr_x = mon_x + 8;
        let scr_y = mon_y + 8;
        let scr_w = mon_w - 16;
        let scr_h = mon_h - 30;
        fb.fill_rect(scr_x, scr_y, scr_w, scr_h, Color::BLACK);
        fb.draw_bevel_sunken(scr_x, scr_y, scr_w, scr_h);

        // Scaled resolution preview inside CRT glass screen
        self.render_scaled_resolution_preview(fb, scr_x + 2, scr_y + 2, scr_w - 4, scr_h - 4);

        // Monitor buttons & green LED
        let led_x = mon_x + mon_w as isize - 20;
        let led_y = mon_y + mon_h as isize - 15;
        fb.fill_rect(led_x, led_y, 6, 6, Color::from_rgb(34, 197, 94));
        fb.fill_rect(led_x - 14, led_y + 1, 8, 4, Color::from_rgb(180, 175, 160));

        // Pedestal
        let ped_w = 64usize;
        let ped_x = mon_x + ((mon_w - ped_w) / 2) as isize;
        let ped_y = mon_y + mon_h as isize;
        fb.fill_rect(ped_x + 16, ped_y, 32, 4, Color::from_rgb(190, 185, 170));
        fb.fill_rect(ped_x, ped_y + 4, ped_w, 4, Color::from_rgb(175, 170, 155));
        fb.draw_bevel_raised(ped_x, ped_y + 4, ped_w, 4);

        // --- 2. Screen Area (Resolution) Slider ---
        let ctrl_y = sy + 128;
        fb.draw_string(sx + 16, ctrl_y, "Screen Area (Display Resolution):", Color::BLACK);

        let btn_less_x = sx + 16;
        let btn_more_x = sx + sw as isize - 90;
        let slider_y = ctrl_y + 16;

        fb.draw_button(btn_less_x, slider_y, 64, 22, false);
        fb.draw_string(btn_less_x + 8, slider_y + 3, "< Less", Color::BLACK);

        fb.draw_button(btn_more_x, slider_y, 64, 22, false);
        fb.draw_string(btn_more_x + 8, slider_y + 3, "More >", Color::BLACK);

        // Slider track groove
        let track_x = btn_less_x + 72;
        let track_w = (btn_more_x - track_x - 8).max(60) as usize;
        let track_y = slider_y + 8;
        fb.draw_groove(track_x, track_y, track_w, 2);

        // 4 Tick marks
        for i in 0..4 {
            let tx = track_x + ((i as isize * track_w as isize) / 3);
            fb.fill_rect(tx, track_y - 4, 1, 8, Color::from_rgb(120, 120, 120));
        }

        // Slider Thumb Pill
        let thumb_x = track_x + ((self.selected_res_idx as isize * track_w as isize) / 3) - 6;
        fb.fill_rect(thumb_x, slider_y + 2, 12, 16, Color::RETRO_FACE);
        fb.draw_bevel_raised(thumb_x, slider_y + 2, 12, 16);

        // Active Resolution Label
        let (res_str, _, _, desc) = RESOLUTIONS[self.selected_res_idx];
        let full_lbl = format!("{} pixels - {}", res_str, desc);
        let fl_w = full_lbl.len() * FONT_WIDTH;
        let fl_x = sx + ((sw as isize - fl_w as isize) / 2);
        fb.draw_string(fl_x, ctrl_y + 44, &full_lbl, Color::from_rgb(16, 50, 120));

        // --- 3. Hardware Details Panel ---
        let box_y = sy + 184;
        let box_h = 50usize;
        fb.fill_rect(sx + 10, box_y, sw - 20, box_h, Color::RETRO_FACE);
        fb.draw_sunken_panel(sx + 10, box_y, sw - 20, box_h);

        fb.draw_string(sx + 16, box_y + 4, "Adapter: Bochs VBE 2.0 / BGA Graphics (PCI)", Color::BLACK);
        fb.draw_string(sx + 16, box_y + 18, "Colors: 32-Bit True Color (16.7M) | 60 Hz", Color::from_rgb(50, 50, 50));
        fb.draw_string(sx + 16, box_y + 32, "Memory: 16 MB VRAM | Linear Framebuffer", Color::from_rgb(50, 50, 50));
    }

    fn render_sound_tab(&self, fb: &mut Framebuffer, sx: isize, sy: isize, sw: usize, _sh: usize) {
        // --- 1. Audio Output Device Group Box ---
        let dev_box_y = sy + 6;
        let dev_box_h = 82usize;
        fb.draw_sunken_panel(sx + 10, dev_box_y, sw - 20, dev_box_h);
        fb.draw_string(sx + 16, dev_box_y + 4, "Sound Playback Device:", Color::BLACK);

        // Option 1: AC97
        let is_ac97 = self.selected_sound_device == SoundDeviceKind::Ac97Pci;
        draw_radio_button(fb, sx + 20, dev_box_y + 22, is_ac97);
        fb.draw_string(sx + 36, dev_box_y + 20, "Intel 82801AA AC'97 Audio Controller", Color::BLACK);
        fb.draw_string(sx + 36, dev_box_y + 34, "Status: Active PCI DMA (16-Bit Stereo, 48 kHz)", Color::from_rgb(34, 130, 70));

        // Option 2: PC Speaker
        let is_spk = self.selected_sound_device == SoundDeviceKind::PcSpeaker;
        draw_radio_button(fb, sx + 20, dev_box_y + 52, is_spk);
        fb.draw_string(sx + 36, dev_box_y + 50, "Intel 8254 PIT PC Speaker Synthesizer", Color::BLACK);
        fb.draw_string(sx + 36, dev_box_y + 64, "Status: Ready (Port 0x61 Chiptune Synth)", Color::from_rgb(100, 100, 100));

        // --- 2. Master Volume Group Box ---
        let vol_box_y = sy + 94;
        let vol_box_h = 62usize;
        fb.draw_sunken_panel(sx + 10, vol_box_y, sw - 20, vol_box_h);

        let vol_lbl = format!("Master Volume: {}%", self.master_volume);
        fb.draw_string(sx + 16, vol_box_y + 4, &vol_lbl, Color::BLACK);

        // Step Buttons [ - ] and [ + ]
        let btn_down_x = sx + 18;
        let btn_y = vol_box_y + 24;
        fb.draw_button(btn_down_x, btn_y, 28, 22, false);
        fb.draw_string(btn_down_x + 10, btn_y + 3, "-", Color::BLACK);

        // 10-Segment LED Volume Bar
        let bar_x = btn_down_x + 36;
        let bar_w = 110usize;
        let bar_h = 16usize;
        fb.fill_rect(bar_x, btn_y + 3, bar_w, bar_h, Color::BLACK);
        fb.draw_bevel_sunken(bar_x, btn_y + 3, bar_w, bar_h);

        let active_segments = (self.master_volume as usize * 10) / 100;
        for s in 0..10 {
            let seg_x = bar_x + 3 + (s as isize * 10);
            let seg_color = if s < active_segments {
                if s < 6 {
                    Color::from_rgb(34, 197, 94) // Green
                } else if s < 8 {
                    Color::from_rgb(234, 179, 8) // Yellow
                } else {
                    Color::from_rgb(239, 68, 68) // Red
                }
            } else {
                Color::from_rgb(40, 40, 40) // Dim unlit
            };
            fb.fill_rect(seg_x, btn_y + 5, 8, 12, seg_color);
        }

        let btn_up_x = bar_x + bar_w as isize + 8;
        fb.draw_button(btn_up_x, btn_y, 28, 22, false);
        fb.draw_string(btn_up_x + 10, btn_y + 3, "+", Color::BLACK);

        // Mute Checkbox
        let mute_x = btn_up_x + 44;
        draw_checkbox(fb, mute_x, btn_y + 4, self.is_muted);
        let mute_lbl_col = if self.is_muted { Color::from_rgb(180, 20, 20) } else { Color::BLACK };
        fb.draw_string(mute_x + 18, btn_y + 3, "Mute All Sound", mute_lbl_col);

        // --- 3. Interactive Sound Test & Driver Specifications ---
        let test_box_y = sy + 162;
        let test_box_h = 72usize;
        fb.draw_sunken_panel(sx + 10, test_box_y, sw - 20, test_box_h);

        let is_testing = self.test_sound_timer > 0;
        let test_btn_txt = if is_testing { "Playing..." } else { "Test Sound" };
        fb.draw_button(sx + 18, test_box_y + 20, 100, 26, is_testing);
        fb.draw_string(sx + 24, test_box_y + 25, test_btn_txt, Color::BLACK);

        // Specs pane next to button
        let specs_x = sx + 124;
        fb.draw_string(specs_x, test_box_y + 8, "Driver: Dual AC97 DMA + PIT", Color::BLACK);
        fb.draw_string(specs_x, test_box_y + 24, "Format: Stereo 16-Bit Linear PCM", Color::from_rgb(50, 50, 50));
        fb.draw_string(specs_x, test_box_y + 40, "Rates: 44.1 kHz / 48.0 kHz VRA", Color::from_rgb(50, 50, 50));
        fb.draw_string(specs_x, test_box_y + 54, "Buffer: 32 Descriptors (DMA)", Color::from_rgb(50, 50, 50));
    }

    fn render_mini_desktop(&self, fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize) {
        // Draw mini wallpaper background
        let wp_color = self.selected_wallpaper.preview_color();
        fb.fill_rect(x, y, w, h, wp_color);

        // Decorative touches per wallpaper
        match self.selected_wallpaper {
            WallpaperKind::ClassicTeal => {
                fb.draw_string(x + 20, y + 26, "MOUROS 98", Color::from_rgb(0, 160, 160));
            }
            WallpaperKind::Clouds => {
                fb.fill_rect(x + 12, y + 8, 30, 8, Color::WHITE);
                fb.fill_rect(x + 18, y + 4, 18, 6, Color::WHITE);
                fb.fill_rect(x + 70, y + 16, 40, 10, Color::WHITE);
                fb.fill_rect(x + 78, y + 10, 24, 8, Color::WHITE);
            }
            WallpaperKind::Bliss => {
                let hill_y = y + (h as isize * 55 / 100);
                fb.fill_rect(x, hill_y, w, (h as isize - (hill_y - y)) as usize, Color::from_rgb(34, 197, 94));
            }
            WallpaperKind::MacOSPlatinum => {
                for py in (y..y + h as isize).step_by(2) {
                    fb.fill_rect(x, py, w, 1, Color::from_rgb(100, 120, 140));
                }
            }
            WallpaperKind::MatrixGrid => {
                for px in (x..x + w as isize).step_by(12) {
                    fb.fill_rect(px, y, 1, h, Color::from_rgb(14, 42, 20));
                }
                fb.draw_pixel(x + 20, y + 15, Color::from_rgb(74, 222, 128));
                fb.draw_pixel(x + 44, y + 25, Color::from_rgb(74, 222, 128));
                fb.draw_pixel(x + 80, y + 18, Color::from_rgb(74, 222, 128));
            }
            WallpaperKind::DeepSpace => {
                fb.draw_pixel(x + 15, y + 10, Color::WHITE);
                fb.draw_pixel(x + 45, y + 32, Color::WHITE);
                fb.draw_pixel(x + 85, y + 14, Color::WHITE);
                fb.draw_pixel(x + 115, y + 42, Color::WHITE);
                fb.draw_pixel(x + 70, y + 55, Color::WHITE);
            }
            WallpaperKind::RetroSunset => {
                let hz = y + (h as isize * 65 / 100);
                fb.fill_rect(x, hz, w, (h as isize - (hz - y)) as usize, Color::from_rgb(15, 5, 29));
                fb.fill_rect(x + (w as isize / 2) - 16, hz - 12, 32, 12, Color::from_rgb(253, 224, 71));
            }
        }

        // Draw Mini Window
        let cur_th = Theme::get(self.selected_theme);
        let win_x = x + 16;
        let win_y = y + 10;
        let win_w = (w - 32).max(40);
        let win_h = (h - 26).max(30);

        fb.fill_rect(win_x, win_y, win_w, win_h, Color::RETRO_FACE);
        fb.draw_bevel_raised(win_x, win_y, win_w, win_h);
        // Titlebar
        fb.fill_rect(win_x + 2, win_y + 2, win_w - 4, 8, cur_th.win_title_active_top);
        // Title text dot
        fb.fill_rect(win_x + 4, win_y + 4, 16, 3, Color::WHITE);
        // Close box
        fb.fill_rect(win_x + win_w as isize - 8, win_y + 3, 5, 5, Color::RETRO_FACE);

        // Draw Mini Taskbar
        let tb_y = y + h as isize - 10;
        fb.fill_rect(x, tb_y, w, 10, cur_th.taskbar_top);
        fb.draw_bevel_raised(x, tb_y, w, 10);
        // Start button
        fb.fill_rect(x + 2, tb_y + 2, 14, 6, cur_th.start_btn);
        fb.draw_bevel_raised(x + 2, tb_y + 2, 14, 6);
    }

    fn render_scaled_resolution_preview(&self, fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize) {
        // Authentic Desktop Area scaling depending on selected resolution
        let (win_scale_w, win_scale_h, taskbar_h, icon_count) = match self.selected_res_idx {
            0 => (w * 75 / 100, h * 70 / 100, 14, 2),  // 640x480: Larger windows, less desktop room
            1 => (w * 58 / 100, h * 55 / 100, 11, 3),  // 800x600: Balanced native view
            2 => (w * 44 / 100, h * 42 / 100, 9, 4),   // 1024x768: Dense workspace
            _ => (w * 34 / 100, h * 34 / 100, 8, 5),   // 1280x1024: Ultra high density
        };

        // Desktop background
        fb.fill_rect(x, y, w, h, Color::RETRO_TEAL);

        // Desktop icons
        for i in 0..icon_count {
            let iy = y + 4 + (i as isize * 14);
            fb.fill_rect(x + 4, iy, 8, 8, Color::from_rgb(250, 204, 21)); // Icon yellow
            fb.fill_rect(x + 3, iy + 9, 10, 2, Color::WHITE); // Icon label
        }

        // Window
        let wx = x + (w as isize - win_scale_w as isize) / 2 + 8;
        let wy = y + (h as isize - win_scale_h as isize) / 2 - 2;
        fb.fill_rect(wx, wy, win_scale_w, win_scale_h, Color::RETRO_FACE);
        fb.draw_bevel_raised(wx, wy, win_scale_w, win_scale_h);
        // Titlebar
        fb.fill_rect(wx + 2, wy + 2, win_scale_w - 4, 7, Color::RETRO_ACTIVE_TITLE_LEFT);
        fb.fill_rect(wx + 4, wy + 4, 18, 2, Color::WHITE);
        fb.fill_rect(wx + win_scale_w as isize - 7, wy + 3, 4, 4, Color::RETRO_FACE);

        // Taskbar at bottom
        let ty = y + h as isize - taskbar_h as isize;
        fb.fill_rect(x, ty, w, taskbar_h, Color::RETRO_FACE);
        fb.draw_bevel_raised(x, ty, w, taskbar_h);
        fb.fill_rect(x + 2, ty + 2, 14, taskbar_h - 4, Color::RETRO_FACE);
        fb.draw_bevel_raised(x + 2, ty + 2, 14, taskbar_h - 4);
    }
}

fn draw_radio_button(fb: &mut Framebuffer, x: isize, y: isize, selected: bool) {
    fb.fill_rect(x, y, 12, 12, Color::WHITE);
    fb.draw_bevel_sunken(x, y, 12, 12);
    if selected {
        fb.fill_rect(x + 3, y + 3, 6, 6, Color::BLACK);
    }
}

fn draw_checkbox(fb: &mut Framebuffer, x: isize, y: isize, checked: bool) {
    fb.fill_rect(x, y, 12, 12, Color::WHITE);
    fb.draw_bevel_sunken(x, y, 12, 12);
    if checked {
        fb.draw_string(x + 2, y - 2, "v", Color::BLACK);
    }
}


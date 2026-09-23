use super::apps::{
    CalculatorApp, DoomApp, ElfRunnerApp, ImageViewerApp, MusicApp, NotepadApp, SettingsApp,
    SnakeApp, SysInfoApp, TerminalApp,
};
use super::color::Color;
use super::font::FONT_WIDTH;
use super::framebuffer::Framebuffer;
use super::icons;
use super::theme::{Theme, ThemeKind};
use super::window::Window;
use crate::drivers::mouse::{self, MouseEvent};
use crate::drivers::rtc;
use alloc::boxed::Box;
use alloc::format;
use alloc::vec::Vec;
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};
use x86_64::instructions::port::Port;

const TASKBAR_HEIGHT: usize = 28;

struct DesktopIcon {
    name: &'static str,
    x: isize,
    y: isize,
    app_id: usize,
}

pub struct Desktop {
    pub fb: Framebuffer,
    wallpaper: Vec<u32>,
    pub windows: Vec<Window>,
    next_window_id: usize,
    dragging_window_idx: Option<usize>,
    drag_offset_x: isize,
    drag_offset_y: isize,
    start_menu_open: bool,
    icons: [DesktopIcon; 10],
    prev_left_pressed: bool,
    pub theme: Theme,
    cursor_saved: [u32; 24 * 24],
    cursor_saved_x: isize,
    cursor_saved_y: isize,
    cursor_active: bool,
    clock_cache: rtc::RtcTime,
    last_clock_sec: u8,
    tick_count: usize,
}

impl Desktop {
    pub fn new(fb: Framebuffer) -> Self {
        let theme = Theme::get(ThemeKind::Windows98);
        let wallpaper = theme.render_wallpaper(fb.width, fb.height);

        let icons = [
            DesktopIcon { name: "Terminal", x: 16, y: 16, app_id: 0 },
            DesktopIcon { name: "SysInfo",  x: 16, y: 84, app_id: 1 },
            DesktopIcon { name: "Calc",     x: 16, y: 152, app_id: 2 },
            DesktopIcon { name: "Notepad",  x: 16, y: 220, app_id: 3 },
            DesktopIcon { name: "Snake",    x: 16, y: 288, app_id: 4 },
            DesktopIcon { name: "Music",    x: 80, y: 16,  app_id: 5 },
            DesktopIcon { name: "Images",   x: 80, y: 84,  app_id: 6 },
            DesktopIcon { name: "Settings", x: 80, y: 152, app_id: 7 },
            DesktopIcon { name: "ELF Run",  x: 80, y: 220, app_id: 8 },
            DesktopIcon { name: "DOOM",     x: 80, y: 288, app_id: 9 },
        ];

        let clock = rtc::read_time();

        let mut desktop = Desktop {
            fb,
            wallpaper,
            windows: Vec::new(),
            next_window_id: 1,
            dragging_window_idx: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
            start_menu_open: false,
            icons,
            prev_left_pressed: false,
            theme,
            cursor_saved: [0; 24 * 24],
            cursor_saved_x: -100,
            cursor_saved_y: -100,
            cursor_active: false,
            clock_cache: clock,
            last_clock_sec: clock.seconds,
            tick_count: 0,
        };

        // Open initial windows in an organized layout
        desktop.spawn_sysinfo(390, 15, 395, 260);
        desktop.spawn_terminal(280, 290, 505, 265);
        desktop.spawn_music(15, 20, 365, 260);
        desktop.spawn_settings(155, 20, 380, 260);
        if let Some(settings) = desktop.windows.last_mut() {
            settings.is_minimized = true;
            settings.is_focused = false;
        }
        if let Some(music_idx) = desktop.windows.iter().position(|w| w.app.title().contains("MP3") || w.app.title().contains("Music")) {
            desktop.focus_window_at_index(music_idx);
        }

        desktop
    }

    pub fn spawn_terminal(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(TerminalApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_sysinfo(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(SysInfoApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_calculator(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(CalculatorApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_notepad(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(NotepadApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_snake(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(SnakeApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_music(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(MusicApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_image_viewer(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(ImageViewerApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_settings(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(SettingsApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_elf_runner(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(ElfRunnerApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_doom(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(DoomApp::new()));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    pub fn spawn_doom_with_map(&mut self, x: isize, y: isize, w: usize, h: usize, map_name: &str) {
        let id = self.next_window_id;
        self.next_window_id += 1;
        let mut win = Window::new(id, x, y, w, h, Box::new(DoomApp::with_map(map_name)));
        win.is_focused = true;
        self.unfocus_all();
        self.windows.push(win);
    }

    fn unfocus_all(&mut self) {
        for win in &mut self.windows {
            win.is_focused = false;
        }
    }

    fn focus_window_at_index(&mut self, idx: usize) {
        if idx < self.windows.len() {
            self.unfocus_all();
            let mut win = self.windows.remove(idx);
            win.is_focused = true;
            win.is_minimized = false;
            self.windows.push(win);
        }
    }

    fn restore_cursor_bg(&mut self) {
        if !self.cursor_active {
            return;
        }
        let w = self.fb.width;
        let h = self.fb.height;
        let sx = self.cursor_saved_x;
        let sy = self.cursor_saved_y;
        for cy in 0..24 {
            let py = sy + cy as isize;
            if py < 0 || py >= h as isize {
                continue;
            }
            for cx in 0..24 {
                let px = sx + cx as isize;
                if px < 0 || px >= w as isize {
                    continue;
                }
                let buf_idx = py as usize * w + px as usize;
                let saved_idx = cy * 24 + cx;
                self.fb.backbuffer[buf_idx] = self.cursor_saved[saved_idx];
            }
        }
        self.cursor_active = false;
    }

    fn save_cursor_bg(&mut self, x: isize, y: isize) {
        let w = self.fb.width;
        let h = self.fb.height;
        for cy in 0..24 {
            let py = y + cy as isize;
            for cx in 0..24 {
                let px = x + cx as isize;
                let saved_idx = cy * 24 + cx;
                if px >= 0 && px < w as isize && py >= 0 && py < h as isize {
                    self.cursor_saved[saved_idx] = self.fb.backbuffer[py as usize * w + px as usize];
                } else {
                    self.cursor_saved[saved_idx] = 0;
                }
            }
        }
        self.cursor_saved_x = x;
        self.cursor_saved_y = y;
        self.cursor_active = true;
    }

    pub fn update_cursor_fast(&mut self, new_x: isize, new_y: isize) {
        let old_x = self.cursor_saved_x;
        let old_y = self.cursor_saved_y;

        if old_x == new_x && old_y == new_y && self.cursor_active {
            return;
        }

        self.restore_cursor_bg();
        self.save_cursor_bg(new_x, new_y);
        self.fb.draw_cursor(new_x, new_y);

        let w = self.fb.width;
        let h = self.fb.height;

        let flush_box = |fb: &mut Framebuffer, rx: isize, ry: isize| {
            let x0 = rx.clamp(0, w as isize) as usize;
            let y0 = ry.clamp(0, h as isize) as usize;
            let x1 = (rx + 24).clamp(0, w as isize) as usize;
            let y1 = (ry + 24).clamp(0, h as isize) as usize;
            if x1 > x0 && y1 > y0 {
                fb.flush_rect(x0, y0, x1 - x0, y1 - y0);
            }
        };

        flush_box(&mut self.fb, old_x, old_y);
        flush_box(&mut self.fb, new_x, new_y);
    }

    pub fn handle_mouse_event(&mut self, ev: MouseEvent) -> bool {
        let left_just_pressed = ev.left_pressed && !self.prev_left_pressed;
        let left_just_released = !ev.left_pressed && self.prev_left_pressed;
        self.prev_left_pressed = ev.left_pressed;

        // Window Dragging in progress
        if let Some(win_idx) = self.dragging_window_idx {
            if ev.left_pressed {
                if let Some(win) = self.windows.get_mut(win_idx) {
                    win.x = ev.x - self.drag_offset_x;
                    win.y = (ev.y - self.drag_offset_y).clamp(0, (self.fb.height - TASKBAR_HEIGHT - 20) as isize);
                }
            } else {
                self.dragging_window_idx = None;
            }
            return true;
        }

        if left_just_released {
            let was_dragging = self.dragging_window_idx.is_some();
            self.dragging_window_idx = None;
            return was_dragging;
        }

        if !left_just_pressed {
            return false;
        }

        let taskbar_y = (self.fb.height - TASKBAR_HEIGHT) as isize;

        // 1. Check Taskbar Clicks
        if ev.y >= taskbar_y {
            // Start button click: (x: 3..73)
            if ev.x >= 3 && ev.x <= 73 {
                self.start_menu_open = !self.start_menu_open;
                return true;
            }

            // Taskbar window tabs (dynamically sized)
            let mut tab_x = 77;
            let tray_w = 120;
            let tray_x = (self.fb.width as isize - tray_w - 4).max(100);
            let available_tab_space = (tray_x - tab_x).max(60) as usize;
            let num_windows = self.windows.len().max(1);
            let tab_w = ((available_tab_space / num_windows).saturating_sub(4)).clamp(60, 140) as isize;

            for i in 0..self.windows.len() {
                if ev.x >= tab_x && ev.x < tab_x + tab_w {
                    if self.windows[i].is_focused && !self.windows[i].is_minimized {
                        self.windows[i].is_minimized = true;
                        self.windows[i].is_focused = false;
                    } else {
                        self.focus_window_at_index(i);
                    }
                    self.start_menu_open = false;
                    return true;
                }
                tab_x += tab_w + 4;
            }
            return true;
        }

        // 2. Check Start Menu Clicks if open
        if self.start_menu_open {
            let menu_w = 210;
            let menu_h = 276;
            let menu_x = 2;
            let menu_y = taskbar_y - menu_h as isize;

            if ev.x >= menu_x && ev.x < menu_x + menu_w && ev.y >= menu_y && ev.y < menu_y + menu_h {
                let rel_y = ev.y - (menu_y + 4);
                if rel_y >= 0 {
                    let item_idx = (rel_y / 24) as usize;
                    match item_idx {
                        0 => self.spawn_terminal(140, 80, 480, 270),
                        1 => self.spawn_sysinfo(180, 100, 420, 260),
                        2 => self.spawn_calculator(300, 140, 220, 280),
                        3 => self.spawn_notepad(220, 90, 360, 250),
                        4 => self.spawn_snake(200, 80, 320, 300),
                        5 => self.spawn_music(180, 100, 380, 300),
                        6 => self.spawn_image_viewer(160, 60, 480, 360),
                        7 => self.spawn_settings(200, 120, 380, 280),
                        8 => self.spawn_elf_runner(160, 80, 520, 360),
                        9 => self.spawn_doom(78, 45, 644, 454),
                        10 => {
                            // Reboot
                            unsafe {
                                let mut port = Port::new(0x64);
                                port.write(0xFEu8);
                            }
                        }
                        _ => {}
                    }
                }
                self.start_menu_open = false;
                return true;
            } else {
                self.start_menu_open = false;
                return true;
            }
        }

        // 3. Check Windows in reverse z-order (front to back)
        let num_windows = self.windows.len();
        for i in (0..num_windows).rev() {
            if !self.windows[i].contains(ev.x, ev.y) {
                continue;
            }

            // Close button check
            if self.windows[i].is_over_close_button(ev.x, ev.y) {
                self.windows.remove(i);
                return true;
            }

            // Minimize button check
            if self.windows[i].is_over_minimize_button(ev.x, ev.y) {
                self.windows[i].is_minimized = true;
                self.windows[i].is_focused = false;
                return true;
            }

            // Titlebar click: focus and initiate dragging
            if self.windows[i].is_over_titlebar(ev.x, ev.y) {
                self.focus_window_at_index(i);
                let top_idx = self.windows.len() - 1;
                self.dragging_window_idx = Some(top_idx);
                self.drag_offset_x = ev.x - self.windows[top_idx].x;
                self.drag_offset_y = ev.y - self.windows[top_idx].y;
                return true;
            }

            // Client area click
            self.focus_window_at_index(i);
            let top_idx = self.windows.len() - 1;
            let (cx, cy, _, _) = self.windows[top_idx].client_bounds();
            let local_x = ev.x - cx;
            let local_y = ev.y - cy;
            self.windows[top_idx].app.on_mouse_click(local_x, local_y, true);
            return true;
        }

        // 4. Check Desktop Icons
        for icon in &self.icons {
            if ev.x >= icon.x && ev.x < icon.x + 56 && ev.y >= icon.y && ev.y < icon.y + 60 {
                match icon.app_id {
                    0 => self.spawn_terminal(140, 80, 480, 270),
                    1 => self.spawn_sysinfo(180, 100, 420, 260),
                    2 => self.spawn_calculator(300, 140, 220, 280),
                    3 => self.spawn_notepad(220, 90, 360, 250),
                    4 => self.spawn_snake(200, 80, 320, 300),
                    5 => self.spawn_music(180, 100, 380, 300),
                    6 => self.spawn_image_viewer(160, 60, 480, 360),
                    7 => self.spawn_settings(200, 120, 380, 280),
                    8 => self.spawn_elf_runner(160, 80, 520, 360),
                    9 => self.spawn_doom(78, 45, 644, 454),
                    _ => {}
                }
                return true;
            }
        }

        // 5. Click on empty desktop background
        if self.start_menu_open {
            self.start_menu_open = false;
            return true;
        }

        false
    }

    pub fn handle_key_event(&mut self, key: DecodedKey) {
        for win in self.windows.iter_mut().rev() {
            if win.is_focused && !win.is_minimized {
                win.app.on_key(key);
                break;
            }
        }
    }

    pub fn on_tick(&mut self) -> bool {
        let mut changed = false;

        // Check if theme was updated
        if let Some(new_kind) = crate::gui::theme::take_pending_theme() {
            self.theme = Theme::get(new_kind);
            self.wallpaper = self.theme.render_wallpaper(self.fb.width, self.fb.height);
            changed = true;
        }

        for win in &mut self.windows {
            if !win.is_minimized {
                if win.on_tick() {
                    changed = true;
                }
            } else {
                let title = win.app.title();
                if title.contains("Music") || title.contains("MP3") || title.contains("Chiptune") {
                    win.on_tick();
                }
            }
        }

        // Check RTC second change every 10 ticks (~100ms)
        self.tick_count = self.tick_count.wrapping_add(1);
        if self.tick_count % 10 == 0 {
            let cur_sec = rtc::read_seconds();
            if cur_sec != self.last_clock_sec {
                self.last_clock_sec = cur_sec;
                self.clock_cache = rtc::read_time();
                changed = true;
            }
        }

        changed
    }

    pub fn render(&mut self) {
        // 1. Fast blit pre-rendered wallpaper (memcpy ~0.2ms)
        self.fb.backbuffer.copy_from_slice(&self.wallpaper);

        // 2. Render Desktop Icons (Clean Windows 98 style directly on teal desktop)
        let mouse_state = mouse::get_mouse_state();
        for icon in &self.icons {
            let is_hovered = mouse_state.x >= icon.x && mouse_state.x < icon.x + 54
                && mouse_state.y >= icon.y && mouse_state.y < icon.y + 54;

            if is_hovered {
                // Retro selection box
                self.fb.draw_rect(icon.x + 2, icon.y + 2, 50, 48, Color::RETRO_HIGHLIGHT);
            }

            // Custom 24x24 Pixel Art Icon
            icons::draw_icon_24(&mut self.fb, icon.x + 15, icon.y + 6, icon.app_id.into());

            // Icon label with retro drop shadow on teal background
            let label_len = icon.name.len() * FONT_WIDTH;
            let lx = icon.x + (54 - label_len as isize) / 2;
            self.fb.draw_string(lx + 1, icon.y + 36, icon.name, Color::BLACK);
            self.fb.draw_string(lx, icon.y + 35, icon.name, Color::WHITE);
        }

        // 3. Render Windows in z-order
        for win in &mut self.windows {
            win.render(&mut self.fb);
        }

        // 4. Render Taskbar (Classic 3D Raised Windows 98 Taskbar)
        let taskbar_y = (self.fb.height - TASKBAR_HEIGHT) as isize;
        self.fb.fill_rect(0, taskbar_y, self.fb.width, TASKBAR_HEIGHT, Color::RETRO_FACE);
        self.fb.fill_rect(0, taskbar_y, self.fb.width, 1, Color::RETRO_LIGHT);
        self.fb.fill_rect(0, taskbar_y + 1, self.fb.width, 1, Color::RETRO_HIGHLIGHT);

        // Start Menu Button
        let start_pressed = self.start_menu_open;
        let start_off = if start_pressed { 1 } else { 0 };
        self.fb.draw_button(3, taskbar_y + 3, 70, 22, start_pressed);

        // 4-color retro logo (red, green, blue, yellow squares)
        let flag_x = 7 + start_off;
        let flag_y = taskbar_y + 6 + start_off;
        self.fb.fill_rect(flag_x, flag_y, 4, 4, Color::from_rgb(239, 68, 68)); // Red
        self.fb.fill_rect(flag_x + 5, flag_y, 4, 4, Color::from_rgb(34, 197, 94)); // Green
        self.fb.fill_rect(flag_x, flag_y + 5, 4, 4, Color::from_rgb(59, 130, 246)); // Blue
        self.fb.fill_rect(flag_x + 5, flag_y + 5, 4, 4, Color::from_rgb(234, 179, 8)); // Yellow

        // Start text
        self.fb.draw_string(flag_x + 14, taskbar_y + 7 + start_off, "Start", Color::BLACK);
        self.fb.draw_string(flag_x + 15, taskbar_y + 7 + start_off, "Start", Color::BLACK); // Bold

        // Window Tabs on Taskbar
        let mut tab_x = 78;
        let tray_w = 120;
        let tray_x = (self.fb.width as isize - tray_w - 4).max(100);
        let available_tab_space = (tray_x - tab_x).max(60) as usize;
        let num_windows = self.windows.len().max(1);
        let tab_w = ((available_tab_space / num_windows).saturating_sub(4)).clamp(60, 140) as isize;

        for win in &self.windows {
            let is_active = win.is_focused && !win.is_minimized;
            let toff = if is_active { 1 } else { 0 };

            self.fb.draw_button(tab_x, taskbar_y + 3, tab_w as usize, 22, is_active);

            // Active tab subtle dither pattern
            if is_active {
                for dy in 2..20 {
                    for dx in (2 + (dy % 2)..tab_w as usize - 2).step_by(2) {
                        self.fb.draw_pixel(tab_x + dx as isize, taskbar_y + 3 + dy as isize, Color::RETRO_LIGHT);
                    }
                }
            }

            // Tab 16x16 icon
            let icon = Self::icon_from_title(win.app.title());
            icons::draw_icon_16(&mut self.fb, tab_x + 4 + toff, taskbar_y + 4 + toff, icon);

            // Tab title
            let title = win.app.title();
            let max_chars = (tab_w.saturating_sub(26) as usize / FONT_WIDTH).max(1);
            let display_title = if title.len() > max_chars {
                &title[..max_chars]
            } else {
                title
            };
            self.fb.draw_string(tab_x + 22 + toff, taskbar_y + 7 + toff, display_title, Color::BLACK);

            tab_x += tab_w + 4;
        }

        // System Tray (Right side of taskbar)
        self.fb.draw_sunken_panel(tray_x, taskbar_y + 3, tray_w as usize, 22);

        // Retro speaker icon in system tray (8x8)
        let spk_x = tray_x + 6;
        let spk_y = taskbar_y + 7;
        self.fb.fill_rect(spk_x, spk_y + 2, 2, 4, Color::BLACK);
        self.fb.draw_pixel(spk_x + 2, spk_y + 1, Color::BLACK);
        self.fb.draw_pixel(spk_x + 3, spk_y, Color::BLACK);
        self.fb.draw_pixel(spk_x + 2, spk_y + 6, Color::BLACK);
        self.fb.draw_pixel(spk_x + 3, spk_y + 7, Color::BLACK);
        self.fb.fill_rect(spk_x + 3, spk_y + 1, 1, 6, Color::BLACK);
        // Sound waves
        self.fb.draw_pixel(spk_x + 5, spk_y + 2, Color::BLACK);
        self.fb.draw_pixel(spk_x + 5, spk_y + 5, Color::BLACK);
        self.fb.draw_pixel(spk_x + 7, spk_y + 1, Color::BLACK);
        self.fb.draw_pixel(spk_x + 7, spk_y + 6, Color::BLACK);

        // System Tray Clock
        let clock = self.clock_cache;
        let clock_str = format!("{:02}:{:02}:{:02}", clock.hours, clock.minutes, clock.seconds);
        self.fb.draw_string(tray_x + 22, taskbar_y + 7, &clock_str, Color::BLACK);

        // 5. Render Start Menu Popup if open (Windows 98 Style)
        if self.start_menu_open {
            let menu_w = 210;
            let menu_h = 276;
            let menu_x = 2;
            let menu_y = taskbar_y - menu_h as isize;

            // 3D raised border
            self.fb.fill_rect(menu_x, menu_y, menu_w, menu_h, Color::RETRO_FACE);
            self.fb.draw_bevel_raised(menu_x, menu_y, menu_w, menu_h);

            // Iconic Windows 98 Vertical Gradient Banner
            let banner_w = 24;
            let banner_h = menu_h - 4;
            self.fb.draw_gradient_v(
                menu_x + 2,
                menu_y + 2,
                banner_w,
                banner_h,
                Color::from_rgb(0, 0, 128),
                Color::from_rgb(16, 132, 208),
            );

            // Vertical lettering: "MOUROS 98"
            let banner_chars = ['M', 'O', 'U', 'R', 'O', 'S', ' ', '9', '8'];
            for (idx, &ch) in banner_chars.iter().enumerate() {
                if ch != ' ' {
                    let by = menu_y + 24 + (idx as isize * 20);
                    // Shadow
                    self.fb.draw_char(menu_x + 9, by + 1, ch, Color::BLACK);
                    // Text
                    self.fb.draw_char(menu_x + 8, by, ch, Color::WHITE);
                }
            }

            let items = [
                (0, "Terminal"),
                (1, "System Monitor"),
                (2, "Calculator"),
                (3, "Notepad Editor"),
                (4, "Snake Game"),
                (5, "Music Player"),
                (6, "Image Viewer"),
                (7, "Desktop Settings"),
                (8, "ELF Runner"),
                (9, "DOOM (1993)"),
                (10, "Shut Down..."),
            ];

            for (i, (icon_id, name)) in items.iter().enumerate() {
                let iy = menu_y + 4 + (i as isize * 24);
                let ix = menu_x + 28;
                let iw = menu_w - 32;

                // Check hover
                let is_hovered = mouse_state.x >= ix && mouse_state.x < ix + iw as isize
                    && mouse_state.y >= iy && mouse_state.y < iy + 24;

                if is_hovered {
                    self.fb.fill_rect(ix, iy, iw, 24, Color::RETRO_SELECTION);
                }

                // Draw 16x16 icon
                icons::draw_icon_16(&mut self.fb, ix + 4, iy + 4, (*icon_id).into());

                let text_color = if is_hovered {
                    Color::WHITE
                } else if *icon_id == 10 {
                    Color::from_rgb(180, 20, 20)
                } else if *icon_id == 9 {
                    Color::from_rgb(160, 100, 0)
                } else {
                    Color::BLACK
                };

                self.fb.draw_string(ix + 26, iy + 7, name, text_color);

                // Groove separator before Shut Down
                if i == 9 {
                    self.fb.draw_groove(ix, iy + 24, iw, 2);
                }
            }
        }

        // 6. Draw Mouse Cursor on top of everything
        self.save_cursor_bg(mouse_state.x, mouse_state.y);
        self.fb.draw_cursor(mouse_state.x, mouse_state.y);

        // 7. Blit backbuffer to screen
        self.fb.flush();
    }

    fn icon_from_title(title: &str) -> icons::AppIcon {
        if title.contains("Terminal") {
            icons::AppIcon::Terminal
        } else if title.contains("System") || title.contains("SysInfo") {
            icons::AppIcon::SysInfo
        } else if title.contains("Calc") {
            icons::AppIcon::Calculator
        } else if title.contains("Notepad") {
            icons::AppIcon::Notepad
        } else if title.contains("Snake") {
            icons::AppIcon::Snake
        } else if title.contains("Music") || title.contains("MP3") || title.contains("Chiptune") {
            icons::AppIcon::Music
        } else if title.contains("Image") {
            icons::AppIcon::ImageViewer
        } else if title.contains("Setting") {
            icons::AppIcon::Settings
        } else if title.contains("ELF") {
            icons::AppIcon::ElfRunner
        } else if title.contains("DOOM") {
            icons::AppIcon::Doom
        } else {
            icons::AppIcon::Terminal
        }
    }
}

pub async fn run_desktop(mut desktop: Desktop) {
    crate::task::keyboard::init();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    // Initial render pass so desktop appears immediately on screen
    desktop.render();

    let mut last_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);

    loop {
        let mut full_render = false;
        let mut mouse_moved = false;

        // 1. Process pending mouse events
        while let Some(mouse_ev) = mouse::pop_mouse_event() {
            if desktop.handle_mouse_event(mouse_ev) {
                full_render = true;
            } else {
                mouse_moved = true;
            }
        }

        // 2. Process pending keyboard events non-blockingly
        while let Some(scancode) = crate::task::keyboard::pop_scancode() {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    desktop.handle_key_event(key);
                    full_render = true;
                }
            }
        }

        // 3. Tick animations & game state periodically based on hardware PIT timer
        let current_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
        if current_tick != last_tick {
            last_tick = current_tick;
            if desktop.on_tick() {
                full_render = true;
            }
        }

        // 4. Update display
        if full_render {
            desktop.render();
        } else if mouse_moved {
            let state = mouse::get_mouse_state();
            desktop.update_cursor_fast(state.x, state.y);
        }

        // 5. Sleep CPU until next hardware interrupt ONLY if no pending events arrived during rendering
        if !mouse::has_events() && !crate::task::keyboard::has_events() {
            x86_64::instructions::hlt();
        }
    }
}

use super::color::Color;
use super::framebuffer::Framebuffer;
use super::icons::{self, AppIcon};
use super::theme::{self, Theme, ThemeKind};
use alloc::boxed::Box;
use pc_keyboard::DecodedKey;

pub const TITLEBAR_HEIGHT: usize = 22;
pub const BORDER_WIDTH: usize = 4;

pub trait Application: Send {
    fn title(&self) -> &str;
    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    );
    fn on_key(&mut self, key: DecodedKey);
    fn on_mouse_click(&mut self, local_x: isize, local_y: isize, left: bool);
    fn on_tick(&mut self) -> bool {
        false
    }
}

pub struct Window {
    pub id: usize,
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,
    pub is_minimized: bool,
    pub is_focused: bool,
    pub app: Box<dyn Application>,
}

impl Window {
    pub fn new(id: usize, x: isize, y: isize, width: usize, height: usize, app: Box<dyn Application>) -> Self {
        Window {
            id,
            x,
            y,
            width,
            height,
            is_minimized: false,
            is_focused: false,
            app,
        }
    }

    pub fn client_bounds(&self) -> (isize, isize, usize, usize) {
        let cx = self.x + BORDER_WIDTH as isize;
        let cy = self.y + TITLEBAR_HEIGHT as isize;
        let cw = self.width.saturating_sub(BORDER_WIDTH * 2);
        let ch = self.height.saturating_sub(TITLEBAR_HEIGHT + BORDER_WIDTH);
        (cx, cy, cw, ch)
    }

    pub fn contains(&self, px: isize, py: isize) -> bool {
        if self.is_minimized {
            return false;
        }
        px >= self.x
            && px < self.x + self.width as isize
            && py >= self.y
            && py < self.y + self.height as isize
    }

    pub fn is_over_titlebar(&self, px: isize, py: isize) -> bool {
        if self.is_minimized {
            return false;
        }
        px >= self.x
            && px < self.x + self.width as isize
            && py >= self.y
            && py < self.y + TITLEBAR_HEIGHT as isize
    }

    pub fn is_over_close_button(&self, px: isize, py: isize) -> bool {
        let btn_x = self.x + self.width as isize - 20;
        let btn_y = self.y + 4;
        px >= btn_x && px < btn_x + 16 && py >= btn_y && py < btn_y + 14
    }

    pub fn is_over_minimize_button(&self, px: isize, py: isize) -> bool {
        let btn_x = self.x + self.width as isize - 38;
        let btn_y = self.y + 4;
        px >= btn_x && px < btn_x + 16 && py >= btn_y && py < btn_y + 14
    }

    pub fn on_tick(&mut self) -> bool {
        self.app.on_tick()
    }

    pub fn render(&mut self, fb: &mut Framebuffer) {
        if self.is_minimized {
            return;
        }

        let current_kind = theme::current_theme();
        let current_theme = Theme::get(current_kind);

        // 1. Classic 3D Double-Beveled Window Frame (No flat borders, no dark blur shadows)
        fb.draw_bevel_raised(self.x, self.y, self.width, self.height);

        // Fill window chassis with retro gray
        let (cx, cy, cw, ch) = self.client_bounds();

        // 2. Client area 3D sunken inset border
        fb.draw_bevel_sunken(cx - 2, cy - 2, cw + 4, ch + 4);

        // 3. Titlebar
        let title_x = self.x + 2;
        let title_y = self.y + 2;
        let title_w = self.width.saturating_sub(4);
        let title_h = 18;

        if current_kind == ThemeKind::MacOS9 {
            // Mac OS 9 Platinum pinstripe titlebar
            fb.fill_rect(title_x, title_y, title_w, title_h, Color::RETRO_MACOS_PLATINUM);
            for py in (title_y + 3..title_y + title_h as isize - 3).step_by(2) {
                fb.fill_rect(title_x + 2, py, title_w.saturating_sub(4), 1, Color::from_rgb(170, 170, 170));
            }
            // Title in center with white background pill
            let title = self.app.title();
            let tw = title.len() * 8 + 12;
            let tx = title_x + (title_w as isize - tw as isize) / 2;
            fb.fill_rect(tx, title_y + 2, tw, 14, Color::RETRO_MACOS_PLATINUM);
            fb.draw_bevel_sunken(tx, title_y + 2, tw, 14);
            fb.draw_string(tx + 6, title_y + 5, title, Color::BLACK);
        } else {
            // Authentic Windows 98 Horizontal Gradient Titlebar
            let (t_left, t_right) = if self.is_focused {
                (current_theme.win_title_active_top, current_theme.win_title_active_bot)
            } else {
                (current_theme.win_title_inactive, Color::RETRO_INACTIVE_TITLE_RIGHT)
            };

            fb.draw_gradient_h(title_x, title_y, title_w, title_h, t_left, t_right);

            // App 16x16 icon on titlebar left
            let icon = Self::icon_from_title(self.app.title());
            icons::draw_icon_16(fb, title_x + 2, title_y + 1, icon);

            // Title text (white bold look on active, light silver on inactive)
            let title_color = if self.is_focused {
                Color::WHITE
            } else {
                Color::RETRO_HIGHLIGHT
            };
            fb.draw_string(title_x + 22, title_y + 5, self.app.title(), title_color);
        }

        // 4. Square 3D Titlebar Control Buttons
        // Minimize Button [-]
        let min_x = self.x + self.width as isize - 38;
        let min_y = self.y + 4;
        fb.draw_button(min_x, min_y, 16, 14, false);
        // Minimize horizontal line glyph at bottom
        fb.fill_rect(min_x + 4, min_y + 9, 6, 2, Color::RETRO_TEXT);

        // Close Button [X]
        let close_x = self.x + self.width as isize - 20;
        let close_y = self.y + 4;
        fb.draw_button(close_x, close_y, 16, 14, false);
        // Crisp 7x7 Windows 98 Close 'X' glyph
        let cross = [
            (4, 3), (5, 3), (9, 3), (10, 3),
            (5, 4), (6, 4), (8, 4), (9, 4),
            (6, 5), (7, 5), (8, 5),
            (6, 6), (7, 6), (8, 6),
            (5, 7), (6, 7), (8, 7), (9, 7),
            (4, 8), (5, 8), (9, 8), (10, 8),
        ];
        for (dx, dy) in cross {
            fb.draw_pixel(close_x + dx, close_y + dy, Color::RETRO_TEXT);
        }

        // 5. Client Area
        fb.set_clip(cx, cy, cw, ch);
        self.app.render(fb, cx, cy, cw, ch);
        fb.clear_clip();
    }

    fn icon_from_title(title: &str) -> AppIcon {
        if title.contains("Terminal") {
            AppIcon::Terminal
        } else if title.contains("System") || title.contains("SysInfo") {
            AppIcon::SysInfo
        } else if title.contains("Calc") {
            AppIcon::Calculator
        } else if title.contains("Notepad") {
            AppIcon::Notepad
        } else if title.contains("Snake") {
            AppIcon::Snake
        } else if title.contains("Music") || title.contains("MP3") || title.contains("Chiptune") {
            AppIcon::Music
        } else if title.contains("Image") {
            AppIcon::ImageViewer
        } else if title.contains("Setting") {
            AppIcon::Settings
        } else if title.contains("ELF") {
            AppIcon::ElfRunner
        } else if title.contains("DOOM") {
            AppIcon::Doom
        } else {
            AppIcon::Terminal
        }
    }
}

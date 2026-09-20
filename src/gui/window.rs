use super::color::Color;
use super::framebuffer::Framebuffer;
use alloc::boxed::Box;
use pc_keyboard::DecodedKey;

pub const TITLEBAR_HEIGHT: usize = 24;
pub const BORDER_WIDTH: usize = 2;

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
        px >= btn_x && px < btn_x + 16 && py >= btn_y && py < btn_y + 16
    }

    pub fn is_over_minimize_button(&self, px: isize, py: isize) -> bool {
        let btn_x = self.x + self.width as isize - 40;
        let btn_y = self.y + 4;
        px >= btn_x && px < btn_x + 16 && py >= btn_y && py < btn_y + 16
    }

    pub fn on_tick(&mut self) -> bool {
        self.app.on_tick()
    }

    pub fn render(&mut self, fb: &mut Framebuffer) {
        if self.is_minimized {
            return;
        }

        // Window drop shadow
        fb.draw_shadow(self.x, self.y, self.width, self.height, 4);

        // Window border
        let current_kind = crate::gui::theme::current_theme();
        let theme = crate::gui::theme::Theme::get(current_kind);

        let border_color = if self.is_focused {
            theme.win_border_active
        } else {
            theme.win_border_inactive
        };
        fb.draw_rect(self.x, self.y, self.width, self.height, border_color);

        // Titlebar gradient
        let (title_top, title_bottom) = if self.is_focused {
            (theme.win_title_active_top, theme.win_title_active_bot)
        } else {
            (theme.win_title_inactive, theme.win_title_inactive)
        };

        fb.draw_gradient_v(
            self.x + 1,
            self.y + 1,
            self.width - 2,
            TITLEBAR_HEIGHT - 1,
            title_top,
            title_bottom,
        );

        // Title text
        fb.draw_string(
            self.x + 8,
            self.y + 8,
            self.app.title(),
            Color::WHITE,
        );

        // Window control buttons
        // Minimize button [-]
        let min_x = self.x + self.width as isize - 40;
        let min_y = self.y + 4;
        fb.fill_rect(min_x, min_y, 16, 16, Color::BTN_MIN_YELLOW);
        fb.draw_rect(min_x, min_y, 16, 16, Color::from_rgb(180, 83, 9));
        fb.fill_rect(min_x + 4, min_y + 8, 8, 2, Color::BLACK);

        // Close button [X]
        let close_x = self.x + self.width as isize - 20;
        let close_y = self.y + 4;
        fb.fill_rect(close_x, close_y, 16, 16, Color::BTN_CLOSE_RED);
        fb.draw_rect(close_x, close_y, 16, 16, Color::from_rgb(185, 28, 28));
        fb.draw_string(close_x + 4, close_y + 4, "x", Color::WHITE);

        // Client Area
        let (cx, cy, cw, ch) = self.client_bounds();
        fb.set_clip(cx, cy, cw, ch);
        self.app.render(fb, cx, cy, cw, ch);
        fb.clear_clip();
    }
}

use super::color::Color;
use super::font::{self, FONT_WIDTH};
use crate::drivers::bga::BgaDevice;
use alloc::vec::Vec;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub backbuffer: Vec<u32>,
    pub bga: BgaDevice,
    pub clip: Option<(isize, isize, isize, isize)>,
}

impl Framebuffer {
    pub fn new(bga: BgaDevice) -> Self {
        let size = bga.width * bga.height;
        let mut backbuffer = Vec::with_capacity(size);
        backbuffer.resize(size, Color::DESKTOP_BOTTOM.raw);

        Framebuffer {
            width: bga.width,
            height: bga.height,
            backbuffer,
            bga,
            clip: None,
        }
    }

    pub fn set_clip(&mut self, x: isize, y: isize, w: usize, h: usize) {
        let min_x = x.max(0);
        let min_y = y.max(0);
        let max_x = (x + w as isize).min(self.width as isize);
        let max_y = (y + h as isize).min(self.height as isize);
        self.clip = Some((min_x, min_y, max_x, max_y));
    }

    pub fn clear_clip(&mut self) {
        self.clip = None;
    }

    #[inline(always)]
    pub fn clear(&mut self, color: Color) {
        self.backbuffer.fill(color.raw);
    }

    #[inline(always)]
    pub fn draw_pixel(&mut self, x: isize, y: isize, color: Color) {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return;
        }

        if let Some((min_x, min_y, max_x, max_y)) = self.clip {
            if x < min_x || y < min_y || x >= max_x || y >= max_y {
                return;
            }
        }

        let idx = y as usize * self.width + x as usize;
        let alpha = color.a();
        if alpha == 255 {
            self.backbuffer[idx] = color.raw;
        } else if alpha > 0 {
            let bg = Color::from_raw(self.backbuffer[idx]);
            self.backbuffer[idx] = bg.blend_over(color).raw;
        }
    }

    pub fn fill_rect(&mut self, x: isize, y: isize, w: usize, h: usize, color: Color) {
        if w == 0 || h == 0 {
            return;
        }

        let min_bound_x = if let Some((cx1, _, _, _)) = self.clip { cx1 } else { 0 };
        let min_bound_y = if let Some((_, cy1, _, _)) = self.clip { cy1 } else { 0 };
        let max_bound_x = if let Some((_, _, cx2, _)) = self.clip { cx2 } else { self.width as isize };
        let max_bound_y = if let Some((_, _, _, cy2)) = self.clip { cy2 } else { self.height as isize };

        let x1 = x.max(min_bound_x) as usize;
        let y1 = y.max(min_bound_y) as usize;
        let x2 = ((x + w as isize).min(max_bound_x)).max(min_bound_x) as usize;
        let y2 = ((y + h as isize).min(max_bound_y)).max(min_bound_y) as usize;

        if x1 >= x2 || y1 >= y2 {
            return;
        }

        let alpha = color.a();
        if alpha == 255 {
            for row in y1..y2 {
                let start = row * self.width + x1;
                let end = row * self.width + x2;
                self.backbuffer[start..end].fill(color.raw);
            }
        } else if alpha > 0 {
            for row in y1..y2 {
                for col in x1..x2 {
                    let idx = row * self.width + col;
                    let bg = Color::from_raw(self.backbuffer[idx]);
                    self.backbuffer[idx] = bg.blend_over(color).raw;
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: isize, y: isize, w: usize, h: usize, color: Color) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h as isize - 1, w, 1, color);
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w as isize - 1, y, 1, h, color);
    }

    pub fn draw_gradient_v(
        &mut self,
        x: isize,
        y: isize,
        w: usize,
        h: usize,
        top_color: Color,
        bottom_color: Color,
    ) {
        if w == 0 || h == 0 {
            return;
        }

        let y1 = y.max(0);
        let y2 = (y + h as isize).min(self.height as isize);

        for curr_y in y1..y2 {
            let row_offset = (curr_y - y) as usize;
            let t = ((row_offset * 256) / h) as u16;
            let row_color = top_color.lerp(bottom_color, t);
            self.fill_rect(x, curr_y, w, 1, row_color);
        }
    }

    pub fn draw_shadow(&mut self, x: isize, y: isize, w: usize, h: usize, blur: usize) {
        let shadow_color = Color::from_rgb(10, 14, 22);
        self.fill_rect(x + 4, y + h as isize, w, blur, shadow_color);
        self.fill_rect(x + w as isize, y + 4, blur, h + blur, shadow_color);
    }

    /// Fast horizontal gradient (used for authentic Windows 98 titlebars)
    pub fn draw_gradient_h(
        &mut self,
        x: isize,
        y: isize,
        w: usize,
        h: usize,
        left_color: Color,
        right_color: Color,
    ) {
        if w == 0 || h == 0 {
            return;
        }

        // Precompute the gradient scanline row (up to 1024 width)
        let mut row_buf = [0u32; 1024];
        let clamped_w = w.min(1024);
        for col in 0..clamped_w {
            let t = ((col * 256) / clamped_w.max(1)) as u16;
            row_buf[col] = left_color.lerp(right_color, t).raw;
        }

        let min_bound_x = if let Some((cx1, _, _, _)) = self.clip { cx1 } else { 0 };
        let min_bound_y = if let Some((_, cy1, _, _)) = self.clip { cy1 } else { 0 };
        let max_bound_x = if let Some((_, _, cx2, _)) = self.clip { cx2 } else { self.width as isize };
        let max_bound_y = if let Some((_, _, _, cy2)) = self.clip { cy2 } else { self.height as isize };

        let y1 = y.max(min_bound_y) as usize;
        let y2 = ((y + h as isize).min(max_bound_y)).max(min_bound_y) as usize;

        for curr_y in y1..y2 {
            for col in 0..clamped_w {
                let px = x + col as isize;
                if px >= min_bound_x && px < max_bound_x && curr_y < self.height {
                    let idx = curr_y * self.width + px as usize;
                    self.backbuffer[idx] = row_buf[col];
                }
            }
        }
    }

    /// Standard 2px 3D Raised Bevel (Windows 98 / Mac OS 9 Classic)
    pub fn draw_bevel_raised(&mut self, x: isize, y: isize, w: usize, h: usize) {
        if w < 2 || h < 2 {
            return;
        }
        let iw = w as isize;
        let ih = h as isize;

        // Outer Top & Left: White (#FFFFFF)
        self.fill_rect(x, y, w, 1, Color::RETRO_LIGHT);
        self.fill_rect(x, y, 1, h, Color::RETRO_LIGHT);

        // Inner Top & Left: Light Gray (#DFDFDF)
        self.fill_rect(x + 1, y + 1, w.saturating_sub(2), 1, Color::RETRO_HIGHLIGHT);
        self.fill_rect(x + 1, y + 1, 1, h.saturating_sub(2), Color::RETRO_HIGHLIGHT);

        // Inner Bottom & Right: Dark Gray (#808080)
        self.fill_rect(x + 1, y + ih - 2, w.saturating_sub(2), 1, Color::RETRO_SHADOW);
        self.fill_rect(x + iw - 2, y + 1, 1, h.saturating_sub(2), Color::RETRO_SHADOW);

        // Outer Bottom & Right: Black (#000000)
        self.fill_rect(x, y + ih - 1, w, 1, Color::RETRO_DARK_SHADOW);
        self.fill_rect(x + iw - 1, y, 1, h, Color::RETRO_DARK_SHADOW);
    }

    /// Standard 2px 3D Sunken Bevel (recessed input fields, client area, sunken displays)
    pub fn draw_bevel_sunken(&mut self, x: isize, y: isize, w: usize, h: usize) {
        if w < 2 || h < 2 {
            return;
        }
        let iw = w as isize;
        let ih = h as isize;

        // Outer Top & Left: Dark Gray (#808080)
        self.fill_rect(x, y, w, 1, Color::RETRO_SHADOW);
        self.fill_rect(x, y, 1, h, Color::RETRO_SHADOW);

        // Inner Top & Left: Black (#000000)
        self.fill_rect(x + 1, y + 1, w.saturating_sub(2), 1, Color::RETRO_DARK_SHADOW);
        self.fill_rect(x + 1, y + 1, 1, h.saturating_sub(2), Color::RETRO_DARK_SHADOW);

        // Inner Bottom & Right: Light Gray (#DFDFDF)
        self.fill_rect(x + 1, y + ih - 2, w.saturating_sub(2), 1, Color::RETRO_HIGHLIGHT);
        self.fill_rect(x + iw - 2, y + 1, 1, h.saturating_sub(2), Color::RETRO_HIGHLIGHT);

        // Outer Bottom & Right: White (#FFFFFF)
        self.fill_rect(x, y + ih - 1, w, 1, Color::RETRO_LIGHT);
        self.fill_rect(x + iw - 1, y, 1, h, Color::RETRO_LIGHT);
    }

    /// 1px Sunken Panel / Inset Box (status bar panels, system tray, progress bar trough)
    pub fn draw_sunken_panel(&mut self, x: isize, y: isize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        let iw = w as isize;
        let ih = h as isize;
        self.fill_rect(x, y, w, 1, Color::RETRO_SHADOW);
        self.fill_rect(x, y, 1, h, Color::RETRO_SHADOW);
        self.fill_rect(x, y + ih - 1, w, 1, Color::RETRO_LIGHT);
        self.fill_rect(x + iw - 1, y, 1, h, Color::RETRO_LIGHT);
    }

    /// 1px Raised Panel
    pub fn draw_raised_panel(&mut self, x: isize, y: isize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        let iw = w as isize;
        let ih = h as isize;
        self.fill_rect(x, y, w, 1, Color::RETRO_LIGHT);
        self.fill_rect(x, y, 1, h, Color::RETRO_LIGHT);
        self.fill_rect(x, y + ih - 1, w, 1, Color::RETRO_SHADOW);
        self.fill_rect(x + iw - 1, y, 1, h, Color::RETRO_SHADOW);
    }

    /// Classic 3D Button (Raised or Pressed/Sunken) with gray face
    pub fn draw_button(&mut self, x: isize, y: isize, w: usize, h: usize, pressed: bool) {
        self.fill_rect(x, y, w, h, Color::RETRO_FACE);
        if pressed {
            let iw = w as isize;
            let ih = h as isize;
            // Pressed button has outer dark border and inner shadow
            self.fill_rect(x, y, w, 1, Color::RETRO_DARK_SHADOW);
            self.fill_rect(x, y, 1, h, Color::RETRO_DARK_SHADOW);
            self.fill_rect(x + 1, y + 1, w.saturating_sub(2), 1, Color::RETRO_SHADOW);
            self.fill_rect(x + 1, y + 1, 1, h.saturating_sub(2), Color::RETRO_SHADOW);
            self.fill_rect(x + 1, y + ih - 1, w.saturating_sub(1), 1, Color::RETRO_HIGHLIGHT);
            self.fill_rect(x + iw - 1, y + 1, 1, h.saturating_sub(1), Color::RETRO_HIGHLIGHT);
        } else {
            self.draw_bevel_raised(x, y, w, h);
        }
    }

    /// Classic Etched Divider / Groove (for group boxes, menus, dialog dividers)
    pub fn draw_groove(&mut self, x: isize, y: isize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        if h <= 2 {
            // Horizontal line groove
            self.fill_rect(x, y, w, 1, Color::RETRO_SHADOW);
            self.fill_rect(x, y + 1, w, 1, Color::RETRO_LIGHT);
        } else if w <= 2 {
            // Vertical line groove
            self.fill_rect(x, y, 1, h, Color::RETRO_SHADOW);
            self.fill_rect(x + 1, y, 1, h, Color::RETRO_LIGHT);
        } else {
            // Box groove
            self.draw_rect(x, y, w, h, Color::RETRO_SHADOW);
            self.draw_rect(x + 1, y + 1, w.saturating_sub(2), h.saturating_sub(2), Color::RETRO_LIGHT);
        }
    }

    pub fn draw_char(&mut self, x: isize, y: isize, c: char, color: Color) {
        // Fast path: fully within screen bounds and no clipping active
        if self.clip.is_none()
            && x >= 0
            && (x + 8) <= self.width as isize
            && y >= 0
            && (y + 8) <= self.height as isize
        {
            let glyph = font::get_glyph(c);
            let raw_color = color.raw;
            let ux = x as usize;
            let uy = y as usize;
            for (row, &byte) in glyph.iter().enumerate() {
                if byte == 0 {
                    continue;
                }
                let row_offset = (uy + row) * self.width + ux;
                let slice = &mut self.backbuffer[row_offset..row_offset + 8];
                if byte & 0x80 != 0 { slice[0] = raw_color; }
                if byte & 0x40 != 0 { slice[1] = raw_color; }
                if byte & 0x20 != 0 { slice[2] = raw_color; }
                if byte & 0x10 != 0 { slice[3] = raw_color; }
                if byte & 0x08 != 0 { slice[4] = raw_color; }
                if byte & 0x04 != 0 { slice[5] = raw_color; }
                if byte & 0x02 != 0 { slice[6] = raw_color; }
                if byte & 0x01 != 0 { slice[7] = raw_color; }
            }
            return;
        }

        // General path: with clipping or partial off-screen
        let glyph = font::get_glyph(c);
        for (row, &byte) in glyph.iter().enumerate() {
            let py = y + row as isize;
            for col in 0..8 {
                if (byte & (0x80 >> col)) != 0 {
                    let px = x + col as isize;
                    self.draw_pixel(px, py, color);
                }
            }
        }
    }

    pub fn draw_char_scaled(
        &mut self,
        x: isize,
        y: isize,
        c: char,
        scale: usize,
        color: Color,
    ) {
        let glyph = font::get_glyph(c);
        for (row, &byte) in glyph.iter().enumerate() {
            let py = y + (row * scale) as isize;
            for col in 0..8 {
                if (byte & (0x80 >> col)) != 0 {
                    let px = x + (col * scale) as isize;
                    self.fill_rect(px, py, scale, scale, color);
                }
            }
        }
    }

    pub fn draw_string(&mut self, mut x: isize, y: isize, text: &str, color: Color) {
        for c in text.chars() {
            if c == '\n' {
                break;
            }
            self.draw_char(x, y, c, color);
            x += FONT_WIDTH as isize;
        }
    }

    pub fn draw_string_scaled(
        &mut self,
        mut x: isize,
        y: isize,
        text: &str,
        scale: usize,
        color: Color,
    ) {
        for c in text.chars() {
            if c == '\n' {
                break;
            }
            self.draw_char_scaled(x, y, c, scale, color);
            x += (FONT_WIDTH * scale) as isize;
        }
    }

    pub fn draw_cursor(&mut self, x: isize, y: isize) {
        const CURSOR: [&[u8]; 17] = [
            b"X               ",
            b"XX              ",
            b"X.X             ",
            b"X..X            ",
            b"X...X           ",
            b"X....X          ",
            b"X.....X         ",
            b"X......X        ",
            b"X.......X       ",
            b"X........X      ",
            b"X.....XXXXX     ",
            b"X..X..X         ",
            b"X.X X..X        ",
            b"XX  X..X        ",
            b"X    X..X       ",
            b"     X..X       ",
            b"      XX        ",
        ];

        for (row, line) in CURSOR.iter().enumerate() {
            let py = y + row as isize;
            for (col, &ch) in line.iter().enumerate() {
                let px = x + col as isize;
                match ch {
                    b'X' => self.draw_pixel(px, py, Color::BLACK),
                    b'.' => self.draw_pixel(px, py, Color::WHITE),
                    _ => {}
                }
            }
        }
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        self.bga.copy_framebuffer(&self.backbuffer);
    }

    #[inline(always)]
    pub fn flush_rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        self.bga.copy_rect(&self.backbuffer, x, y, w, h);
    }
}

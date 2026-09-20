use alloc::vec::Vec;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use spin::Mutex;

static CURRENT_THEME: Mutex<ThemeKind> = Mutex::new(ThemeKind::DeepSpace);
static PENDING_THEME: Mutex<Option<ThemeKind>> = Mutex::new(None);

pub fn set_theme(kind: ThemeKind) {
    *PENDING_THEME.lock() = Some(kind);
    *CURRENT_THEME.lock() = kind;
}

pub fn current_theme() -> ThemeKind {
    *CURRENT_THEME.lock()
}

pub fn take_pending_theme() -> Option<ThemeKind> {
    PENDING_THEME.lock().take()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    DeepSpace,
    CyberpunkNeon,
    MatrixEmerald,
    RetroSunset,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub kind: ThemeKind,
    pub name: &'static str,
    pub accent_color: Color,
    pub win_title_active_top: Color,
    pub win_title_active_bot: Color,
    pub win_title_inactive: Color,
    pub win_border_active: Color,
    pub win_border_inactive: Color,
    pub taskbar_top: Color,
    pub taskbar_bot: Color,
    pub start_btn: Color,
    pub wallpaper_top: Color,
    pub wallpaper_bot: Color,
}

impl Theme {
    pub fn get(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::DeepSpace => Self {
                kind,
                name: "Deep Space (Default)",
                accent_color: Color::from_rgb(56, 189, 248),
                win_title_active_top: Color::from_rgb(2, 132, 199),
                win_title_active_bot: Color::from_rgb(3, 105, 161),
                win_title_inactive: Color::from_rgb(51, 65, 85),
                win_border_active: Color::from_rgb(56, 189, 248),
                win_border_inactive: Color::from_rgb(71, 85, 105),
                taskbar_top: Color::from_rgb(30, 41, 59),
                taskbar_bot: Color::from_rgb(15, 23, 42),
                start_btn: Color::from_rgb(37, 99, 235),
                wallpaper_top: Color::from_rgb(15, 23, 42),
                wallpaper_bot: Color::from_rgb(30, 58, 95),
            },
            ThemeKind::CyberpunkNeon => Self {
                kind,
                name: "Cyberpunk Neon",
                accent_color: Color::from_rgb(244, 63, 94),
                win_title_active_top: Color::from_rgb(217, 70, 239),
                win_title_active_bot: Color::from_rgb(168, 85, 247),
                win_title_inactive: Color::from_rgb(63, 63, 70),
                win_border_active: Color::from_rgb(244, 63, 94),
                win_border_inactive: Color::from_rgb(82, 82, 91),
                taskbar_top: Color::from_rgb(39, 39, 42),
                taskbar_bot: Color::from_rgb(24, 24, 27),
                start_btn: Color::from_rgb(244, 63, 94),
                wallpaper_top: Color::from_rgb(24, 16, 38),
                wallpaper_bot: Color::from_rgb(76, 29, 149),
            },
            ThemeKind::MatrixEmerald => Self {
                kind,
                name: "Matrix Emerald",
                accent_color: Color::from_rgb(34, 197, 94),
                win_title_active_top: Color::from_rgb(22, 101, 52),
                win_title_active_bot: Color::from_rgb(20, 83, 45),
                win_title_inactive: Color::from_rgb(38, 38, 38),
                win_border_active: Color::from_rgb(34, 197, 94),
                win_border_inactive: Color::from_rgb(64, 64, 64),
                taskbar_top: Color::from_rgb(23, 23, 23),
                taskbar_bot: Color::from_rgb(10, 10, 10),
                start_btn: Color::from_rgb(22, 163, 74),
                wallpaper_top: Color::from_rgb(5, 20, 10),
                wallpaper_bot: Color::from_rgb(10, 40, 20),
            },
            ThemeKind::RetroSunset => Self {
                kind,
                name: "Retro Sunset",
                accent_color: Color::from_rgb(245, 158, 11),
                win_title_active_top: Color::from_rgb(234, 88, 12),
                win_title_active_bot: Color::from_rgb(194, 65, 12),
                win_title_inactive: Color::from_rgb(68, 64, 60),
                win_border_active: Color::from_rgb(245, 158, 11),
                win_border_inactive: Color::from_rgb(87, 83, 78),
                taskbar_top: Color::from_rgb(41, 37, 36),
                taskbar_bot: Color::from_rgb(28, 25, 23),
                start_btn: Color::from_rgb(234, 88, 12),
                wallpaper_top: Color::from_rgb(30, 20, 40),
                wallpaper_bot: Color::from_rgb(120, 40, 30),
            },
        }
    }

    /// Render wallpaper into a pixel buffer
    pub fn render_wallpaper(&self, width: usize, height: usize) -> Vec<u32> {
        let mut buf = alloc::vec![0u32; width * height];

        for y in 0..height {
            let t = ((y * 256) / height.max(1)) as u16;
            let col = self.wallpaper_top.lerp(self.wallpaper_bot, t);
            let row_start = y * width;
            for x in 0..width {
                buf[row_start + x] = col.to_u32();
            }
        }

        // Add subtle decorative grid dots or stars
        for y in (20..height.saturating_sub(40)).step_by(40) {
            for x in (20..width.saturating_sub(40)).step_by(40) {
                let dot_c = Color::from_argb(40, 255, 255, 255).to_u32();
                buf[y * width + x] = dot_c;
            }
        }

        // Draw centered branding watermark
        let watermark = "MOUROS OS";
        let scale = 4;
        let wm_w = watermark.len() * (FONT_WIDTH * scale);
        let wm_x = (width as isize - wm_w as isize) / 2;
        let wm_y = (height as isize - 100) / 2;

        let wm_color = Color::from_argb(35, 255, 255, 255);
        for (ci, ch) in watermark.chars().enumerate() {
            let cx = wm_x + (ci * FONT_WIDTH * scale) as isize;
            draw_char_to_buffer(&mut buf, width, height, cx, wm_y, ch, scale, wm_color);
        }

        buf
    }
}

fn draw_char_to_buffer(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x: isize,
    y: isize,
    c: char,
    scale: usize,
    color: Color,
) {
    let glyph = crate::gui::font::get_glyph(c);
    for (row, byte) in glyph.iter().enumerate() {
        for col in 0..8 {
            if (byte >> (7 - col)) & 1 == 1 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + (col * scale + sx) as isize;
                        let py = y + (row * scale + sy) as isize;
                        if px >= 0 && px < w as isize && py >= 0 && py < h as isize {
                            let idx = (py as usize) * w + (px as usize);
                            let bg = Color::from_u32(buf[idx]);
                            buf[idx] = Color::blend(color, bg).to_u32();
                        }
                    }
                }
            }
        }
    }
}

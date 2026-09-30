use alloc::vec::Vec;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use spin::Mutex;

static CURRENT_THEME: Mutex<ThemeKind> = Mutex::new(ThemeKind::Windows98);
static PENDING_THEME: Mutex<Option<ThemeKind>> = Mutex::new(None);
static CURRENT_WALLPAPER: Mutex<WallpaperKind> = Mutex::new(WallpaperKind::ClassicTeal);
static PENDING_WALLPAPER: Mutex<Option<WallpaperKind>> = Mutex::new(None);

pub fn set_theme(kind: ThemeKind) {
    *PENDING_THEME.lock() = Some(kind);
    *CURRENT_THEME.lock() = kind;
    let def_wp = kind.default_wallpaper();
    *PENDING_WALLPAPER.lock() = Some(def_wp);
    *CURRENT_WALLPAPER.lock() = def_wp;
}

pub fn current_theme() -> ThemeKind {
    *CURRENT_THEME.lock()
}

pub fn take_pending_theme() -> Option<ThemeKind> {
    PENDING_THEME.lock().take()
}

pub fn set_wallpaper(kind: WallpaperKind) {
    *PENDING_WALLPAPER.lock() = Some(kind);
    *CURRENT_WALLPAPER.lock() = kind;
}

pub fn current_wallpaper() -> WallpaperKind {
    *CURRENT_WALLPAPER.lock()
}

pub fn take_pending_wallpaper() -> Option<WallpaperKind> {
    PENDING_WALLPAPER.lock().take()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperKind {
    ClassicTeal,
    Clouds,
    Bliss,
    MacOSPlatinum,
    MatrixGrid,
    DeepSpace,
    RetroSunset,
}

impl WallpaperKind {
    pub const ALL: [WallpaperKind; 7] = [
        WallpaperKind::ClassicTeal,
        WallpaperKind::Clouds,
        WallpaperKind::Bliss,
        WallpaperKind::MacOSPlatinum,
        WallpaperKind::MatrixGrid,
        WallpaperKind::DeepSpace,
        WallpaperKind::RetroSunset,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            WallpaperKind::ClassicTeal => "Classic Teal (#008080)",
            WallpaperKind::Clouds => "Clouds (Sky Blue)",
            WallpaperKind::Bliss => "Bliss (Rolling Hills)",
            WallpaperKind::MacOSPlatinum => "Platinum Pinstripes",
            WallpaperKind::MatrixGrid => "Matrix Digital Rain",
            WallpaperKind::DeepSpace => "Deep Space Stars",
            WallpaperKind::RetroSunset => "Retro Sunset Grid",
        }
    }

    pub fn preview_color(&self) -> Color {
        match self {
            WallpaperKind::ClassicTeal => Color::RETRO_TEAL,
            WallpaperKind::Clouds => Color::from_rgb(56, 189, 248),
            WallpaperKind::Bliss => Color::from_rgb(34, 197, 94),
            WallpaperKind::MacOSPlatinum => Color::from_rgb(110, 130, 150),
            WallpaperKind::MatrixGrid => Color::from_rgb(20, 83, 45),
            WallpaperKind::DeepSpace => Color::from_rgb(15, 23, 42),
            WallpaperKind::RetroSunset => Color::from_rgb(234, 88, 12),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Windows98,
    MacOS9,
    DeepSpace,
    CyberpunkNeon,
    MatrixEmerald,
    RetroSunset,
}

impl ThemeKind {
    pub fn default_wallpaper(&self) -> WallpaperKind {
        match self {
            ThemeKind::Windows98 => WallpaperKind::ClassicTeal,
            ThemeKind::MacOS9 => WallpaperKind::MacOSPlatinum,
            ThemeKind::DeepSpace => WallpaperKind::DeepSpace,
            ThemeKind::CyberpunkNeon => WallpaperKind::RetroSunset,
            ThemeKind::MatrixEmerald => WallpaperKind::MatrixGrid,
            ThemeKind::RetroSunset => WallpaperKind::RetroSunset,
        }
    }
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
            ThemeKind::Windows98 => Self {
                kind,
                name: "Classic 98",
                accent_color: Color::RETRO_SELECTION,
                win_title_active_top: Color::RETRO_ACTIVE_TITLE_LEFT,
                win_title_active_bot: Color::RETRO_ACTIVE_TITLE_RIGHT,
                win_title_inactive: Color::RETRO_INACTIVE_TITLE_LEFT,
                win_border_active: Color::RETRO_FACE,
                win_border_inactive: Color::RETRO_FACE,
                taskbar_top: Color::RETRO_FACE,
                taskbar_bot: Color::RETRO_FACE,
                start_btn: Color::RETRO_FACE,
                wallpaper_top: Color::RETRO_TEAL,
                wallpaper_bot: Color::RETRO_TEAL,
            },
            ThemeKind::MacOS9 => Self {
                kind,
                name: "Classic Platinum",
                accent_color: Color::from_rgb(0, 0, 128),
                win_title_active_top: Color::RETRO_MACOS_PLATINUM,
                win_title_active_bot: Color::from_rgb(180, 180, 180),
                win_title_inactive: Color::from_rgb(200, 200, 200),
                win_border_active: Color::RETRO_MACOS_PLATINUM,
                win_border_inactive: Color::RETRO_MACOS_PLATINUM,
                taskbar_top: Color::RETRO_MACOS_PLATINUM,
                taskbar_bot: Color::from_rgb(190, 190, 190),
                start_btn: Color::RETRO_MACOS_PLATINUM,
                wallpaper_top: Color::from_rgb(100, 120, 140),
                wallpaper_bot: Color::from_rgb(70, 90, 110),
            },
            ThemeKind::DeepSpace => Self {
                kind,
                name: "Deep Space",
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

    /// Render wallpaper into a pixel buffer based on active wallpaper
    pub fn render_wallpaper(&self, width: usize, height: usize) -> Vec<u32> {
        Self::render_wallpaper_by_kind(current_wallpaper(), width, height)
    }

    /// Render any wallpaper by kind
    pub fn render_wallpaper_by_kind(kind: WallpaperKind, width: usize, height: usize) -> Vec<u32> {
        let mut buf = alloc::vec![0u32; width * height];

        match kind {
            WallpaperKind::ClassicTeal => {
                // Authentic solid Windows 98 Teal canvas (#008080)
                buf.fill(Color::RETRO_TEAL.raw);

                // Centered nostalgic retro watermark
                let watermark = "MOUROS 98";
                let scale = 4;
                let wm_w = watermark.len() * (FONT_WIDTH * scale);
                let wm_x = (width as isize - wm_w as isize) / 2;
                let wm_y = (height as isize - 100) / 2;
                let wm_shadow = Color::from_rgb(0, 96, 96);
                let wm_color = Color::from_rgb(0, 160, 160);

                // Shadow
                for (ci, ch) in watermark.chars().enumerate() {
                    let cx = wm_x + (ci * FONT_WIDTH * scale) as isize + 2;
                    draw_char_to_buffer(&mut buf, width, height, cx, wm_y + 2, ch, scale, wm_shadow);
                }
                // Highlight
                for (ci, ch) in watermark.chars().enumerate() {
                    let cx = wm_x + (ci * FONT_WIDTH * scale) as isize;
                    draw_char_to_buffer(&mut buf, width, height, cx, wm_y, ch, scale, wm_color);
                }
            }
            WallpaperKind::Clouds => {
                // Windows 95/98 Sky Blue Gradient with Procedural Fluffy Clouds
                let sky_top = Color::from_rgb(26, 99, 188);
                let sky_bot = Color::from_rgb(138, 212, 247);
                for y in 0..height {
                    let t = ((y * 256) / height.max(1)) as u16;
                    let col = sky_top.lerp(sky_bot, t).to_u32();
                    let row = y * width;
                    buf[row..row + width].fill(col);
                }

                // Draw classic 90s fluffy clouds
                let cloud_centers = [
                    (width as isize / 4, height as isize / 4, 38isize),
                    (width as isize * 3 / 4, height as isize / 5, 48isize),
                    (width as isize / 2, height as isize / 2, 54isize),
                    (width as isize / 6, height as isize * 3 / 5, 42isize),
                    (width as isize * 5 / 6, height as isize * 5 / 8, 36isize),
                ];

                for &(cx, cy, r) in &cloud_centers {
                    draw_fluffy_cloud(&mut buf, width, height, cx, cy, r);
                }
            }
            WallpaperKind::Bliss => {
                // Legendary Bliss: Sky + Lush Green Rolling Hills
                let sky_top = Color::from_rgb(37, 99, 235);
                let sky_bot = Color::from_rgb(167, 218, 253);
                let horizon_y = (height * 55) / 100;

                for y in 0..height {
                    let row = y * width;
                    if y < horizon_y {
                        let t = ((y * 256) / horizon_y.max(1)) as u16;
                        let col = sky_top.lerp(sky_bot, t).to_u32();
                        buf[row..row + width].fill(col);
                    }
                }

                // Distant gentle clouds
                draw_fluffy_cloud(&mut buf, width, height, width as isize / 3, (height / 6) as isize, 28);
                draw_fluffy_cloud(&mut buf, width, height, width as isize * 3 / 4, (height / 4) as isize, 34);

                // Back rolling hill (darker emerald green)
                let c_back_hill = Color::from_rgb(21, 128, 61).to_u32();
                for x in 0..width {
                    let fx = x as f32;
                    let wave = (fx * 0.007).sin_approx() * 32.0;
                    let hill_y = (height as f32 * 0.52 + wave) as usize;
                    for y in hill_y..height {
                        buf[y * width + x] = c_back_hill;
                    }
                }

                // Foreground rolling hill (vibrant sunny grass green)
                let c_front_hill = Color::from_rgb(34, 197, 94).to_u32();
                let c_ridge = Color::from_rgb(74, 222, 128).to_u32();
                for x in 0..width {
                    let fx = x as f32;
                    let wave = (fx * 0.009 + 1.8).sin_approx() * 45.0;
                    let hill_y = (height as f32 * 0.65 + wave) as usize;
                    for y in hill_y..height {
                        if y == hill_y {
                            buf[y * width + x] = c_ridge;
                        } else {
                            buf[y * width + x] = c_front_hill;
                        }
                    }
                }
            }
            WallpaperKind::MacOSPlatinum => {
                // Classic Mac OS 9 Platinum pinstripes
                let c1 = Color::from_rgb(110, 130, 150).raw;
                let c2 = Color::from_rgb(100, 120, 140).raw;
                for y in 0..height {
                    let col = if (y / 2) % 2 == 0 { c1 } else { c2 };
                    let row = y * width;
                    buf[row..row + width].fill(col);
                }
            }
            WallpaperKind::MatrixGrid => {
                // Cyberpunk / Matrix digital rain grid
                buf.fill(Color::from_rgb(4, 10, 5).raw);
                let grid_c = Color::from_rgb(14, 42, 20).raw;
                for y in (0..height).step_by(28) {
                    let row = y * width;
                    buf[row..row + width].fill(grid_c);
                }
                for x in (0..width).step_by(28) {
                    for y in 0..height {
                        buf[y * width + x] = grid_c;
                    }
                }

                // Falling green code markers
                let green_bright = Color::from_rgb(74, 222, 128).raw;
                let green_dim = Color::from_rgb(34, 197, 94).raw;
                for col in (14..width).step_by(28) {
                    let seed = ((col * 7919) ^ 0xACE1) % height;
                    let length = 12usize;
                    for dy in 0..length {
                        let py = (seed + dy * 8) % height;
                        let col_val = if dy == length - 1 { green_bright } else { green_dim };
                        buf[py * width + col] = col_val;
                    }
                }
            }
            WallpaperKind::DeepSpace => {
                // Cosmic starfield gradient
                let top_c = Color::from_rgb(8, 12, 26);
                let bot_c = Color::from_rgb(22, 34, 56);
                for y in 0..height {
                    let t = ((y * 256) / height.max(1)) as u16;
                    let col = top_c.lerp(bot_c, t).to_u32();
                    let row = y * width;
                    buf[row..row + width].fill(col);
                }

                // Sparkling stars
                let mut seed = 0x12345678u32;
                let star_count = (width * height) / 450;
                for _ in 0..star_count {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let sx = (seed as usize) % width;
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let sy = (seed as usize) % height;
                    let brightness = 140 + ((seed >> 16) & 0x6F) as u8;
                    buf[sy * width + sx] = Color::from_rgb(brightness, brightness, (brightness as u16 + 20).min(255) as u8).raw;
                }

                // Watermark
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
            }
            WallpaperKind::RetroSunset => {
                // Synthwave 80s Sunset Grid
                let horizon_y = (height * 68) / 100;
                let sky_top = Color::from_rgb(32, 8, 56);
                let sky_mid = Color::from_rgb(192, 38, 211);
                let sky_horizon = Color::from_rgb(249, 115, 22);

                for y in 0..horizon_y {
                    let col = if y < horizon_y / 2 {
                        let t = ((y * 256) / (horizon_y / 2).max(1)) as u16;
                        sky_top.lerp(sky_mid, t)
                    } else {
                        let t = (((y - horizon_y / 2) * 256) / (horizon_y - horizon_y / 2).max(1)) as u16;
                        sky_mid.lerp(sky_horizon, t)
                    };
                    let row = y * width;
                    buf[row..row + width].fill(col.to_u32());
                }

                // Glowing setting sun disk on the horizon
                let sun_cx = width as isize / 2;
                let sun_cy = horizon_y as isize;
                let sun_r = (height as isize * 18) / 100;
                let sun_col = Color::from_rgb(253, 224, 71).to_u32();

                for dy in -sun_r..0 {
                    let py = sun_cy + dy;
                    if py >= 0 && (py as usize) < height {
                        // Scanline blinds cutouts through sun
                        if (dy.abs() % 8) < 3 && dy > -sun_r + 15 {
                            continue;
                        }
                        let dx_max = ((sun_r * sun_r - dy * dy) as f32).sqrt_approx() as isize;
                        let x1 = (sun_cx - dx_max).max(0) as usize;
                        let x2 = (sun_cx + dx_max).min(width as isize - 1) as usize;
                        if x2 > x1 {
                            let row = py as usize * width;
                            buf[row + x1..row + x2].fill(sun_col);
                        }
                    }
                }

                // Dark neon grid floor below horizon
                let floor_bg = Color::from_rgb(15, 5, 29).raw;
                for y in horizon_y..height {
                    let row = y * width;
                    buf[row..row + width].fill(floor_bg);
                }

                // Horizontal perspective grid lines
                let grid_line_c = Color::from_rgb(236, 72, 153).raw;
                let mut cur_gap = 4isize;
                let mut cur_y = horizon_y as isize + 2;
                while cur_y < height as isize {
                    let row = cur_y as usize * width;
                    buf[row..row + width].fill(grid_line_c);
                    cur_y += cur_gap;
                    cur_gap = (cur_gap * 13) / 10;
                }

                // Vertical perspective lines converging to sun center
                for fx in (0..=width as isize).step_by(60) {
                    let dx = fx - sun_cx;
                    for y in horizon_y..height {
                        let t = (y - horizon_y) as f32 / (height - horizon_y) as f32;
                        let px = sun_cx + (dx as f32 * t * 1.8) as isize;
                        if px >= 0 && (px as usize) < width {
                            buf[y * width + px as usize] = grid_line_c;
                        }
                    }
                }
            }
        }

        buf
    }
}

trait MathExt {
    fn sin_approx(self) -> f32;
    fn sqrt_approx(self) -> f32;
}

impl MathExt for f32 {
    fn sin_approx(mut self) -> f32 {
        let pi = 3.14159265f32;
        let two_pi = 2.0 * pi;
        self = self % two_pi;
        if self < 0.0 { self += two_pi; }
        if self > pi {
            return -(self - pi).sin_approx();
        }
        let num = 16.0 * self * (pi - self);
        let den = 5.0 * pi * pi - 4.0 * self * (pi - self);
        if den == 0.0 { 0.0 } else { num / den }
    }

    fn sqrt_approx(self) -> f32 {
        if self <= 0.0 { return 0.0; }
        let mut guess = self * 0.5 + 0.5;
        for _ in 0..4 {
            guess = 0.5 * (guess + self / guess);
        }
        guess
    }
}

fn draw_fluffy_cloud(buf: &mut [u32], w: usize, h: usize, cx: isize, cy: isize, r: isize) {
    let cloud_white = Color::WHITE.to_u32();
    let cloud_shade = Color::from_rgb(205, 230, 248).to_u32();

    // 4 overlapping circles forming a cloud
    let puffs = [
        (cx, cy, r),
        (cx - (r * 7 / 10), cy + (r / 6), (r * 3 / 4)),
        (cx + (r * 7 / 10), cy + (r / 5), (r * 4 / 5)),
        (cx + (r * 13 / 10), cy + (r / 3), (r * 3 / 5)),
    ];

    for &(px, py, pr) in &puffs {
        let r2 = pr * pr;
        for dy in -pr..=pr {
            let y = py + dy;
            if y < 0 || y >= h as isize { continue; }
            let dx_max = ((r2 - dy * dy) as f32).sqrt_approx() as isize;
            let x1 = (px - dx_max).max(0) as usize;
            let x2 = (px + dx_max).min(w as isize - 1) as usize;
            if x2 > x1 {
                let row = y as usize * w;
                let c = if dy > pr / 3 { cloud_shade } else { cloud_white };
                buf[row + x1..row + x2].fill(c);
            }
        }
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

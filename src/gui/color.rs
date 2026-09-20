#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub raw: u32,
}

impl Color {
    pub const BLACK: Color = Color::from_rgb(0, 0, 0);
    pub const WHITE: Color = Color::from_rgb(255, 255, 255);
    pub const RED: Color = Color::from_rgb(239, 68, 68);
    pub const GREEN: Color = Color::from_rgb(34, 197, 94);
    pub const BLUE: Color = Color::from_rgb(59, 130, 246);
    pub const YELLOW: Color = Color::from_rgb(234, 179, 8);
    pub const CYAN: Color = Color::from_rgb(6, 182, 212);
    pub const MAGENTA: Color = Color::from_rgb(217, 70, 239);
    pub const ORANGE: Color = Color::from_rgb(249, 115, 22);
    pub const TRANSPARENT: Color = Color::from_raw(0x0000_0000);

    // Theme Colors
    pub const DESKTOP_TOP: Color = Color::from_rgb(20, 34, 58);
    pub const DESKTOP_BOTTOM: Color = Color::from_rgb(11, 19, 36);

    pub const TITLEBAR_ACTIVE_TOP: Color = Color::from_rgb(37, 99, 235);
    pub const TITLEBAR_ACTIVE_BOTTOM: Color = Color::from_rgb(29, 78, 216);

    pub const TITLEBAR_INACTIVE_TOP: Color = Color::from_rgb(71, 85, 105);
    pub const TITLEBAR_INACTIVE_BOTTOM: Color = Color::from_rgb(51, 65, 85);

    pub const WINDOW_BG: Color = Color::from_rgb(24, 28, 38);
    pub const WINDOW_BORDER: Color = Color::from_rgb(55, 65, 81);
    pub const WINDOW_BORDER_ACTIVE: Color = Color::from_rgb(96, 165, 250);

    pub const BTN_CLOSE_RED: Color = Color::from_rgb(239, 68, 68);
    pub const BTN_MIN_YELLOW: Color = Color::from_rgb(245, 158, 11);

    pub const TASKBAR_TOP: Color = Color::from_rgb(30, 38, 52);
    pub const TASKBAR_BOTTOM: Color = Color::from_rgb(18, 24, 34);
    pub const TASKBAR_BORDER: Color = Color::from_rgb(51, 65, 85);

    pub const START_BTN: Color = Color::from_rgb(37, 99, 235);
    pub const START_BTN_HOVER: Color = Color::from_rgb(59, 130, 246);

    pub const TEXT_LIGHT: Color = Color::from_rgb(241, 245, 249);
    pub const TEXT_MUTED: Color = Color::from_rgb(148, 163, 184);
    pub const TEXT_DARK: Color = Color::from_rgb(15, 23, 42);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgb(r, g, b)
    }

    pub const fn from_raw(raw: u32) -> Self {
        Color { raw }
    }

    pub const fn from_u32(raw: u32) -> Self {
        Color { raw }
    }

    #[inline(always)]
    pub const fn to_u32(self) -> u32 {
        self.raw
    }

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color {
            raw: 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
        }
    }

    pub const fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Color {
            raw: ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
        }
    }

    pub fn blend(fg: Color, bg: Color) -> Color {
        bg.blend_over(fg)
    }

    #[inline(always)]
    pub fn r(self) -> u8 {
        ((self.raw >> 16) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn g(self) -> u8 {
        ((self.raw >> 8) & 0xFF) as u8
    }

    #[inline(always)]
    pub fn b(self) -> u8 {
        (self.raw & 0xFF) as u8
    }

    #[inline(always)]
    pub fn a(self) -> u8 {
        ((self.raw >> 24) & 0xFF) as u8
    }

    /// Interpolates linearly between this color and `other` at progress `t` (0.0 to 1.0 represented as 0..=256)
    pub fn lerp(self, other: Color, t: u16) -> Color {
        let t = t.min(256) as u32;
        let inv_t = 256 - t;

        let r = ((self.r() as u32 * inv_t + other.r() as u32 * t) >> 8) as u8;
        let g = ((self.g() as u32 * inv_t + other.g() as u32 * t) >> 8) as u8;
        let b = ((self.b() as u32 * inv_t + other.b() as u32 * t) >> 8) as u8;
        let a = ((self.a() as u32 * inv_t + other.a() as u32 * t) >> 8) as u8;

        Color::from_argb(a, r, g, b)
    }

    /// Alpha-blends `fg` over this color (background)
    pub fn blend_over(self, fg: Color) -> Color {
        let alpha = fg.a() as u32;
        if alpha == 255 {
            return fg;
        }
        if alpha == 0 {
            return self;
        }

        let inv_alpha = 255 - alpha;
        let r = ((fg.r() as u32 * alpha + self.r() as u32 * inv_alpha) / 255) as u8;
        let g = ((fg.g() as u32 * alpha + self.g() as u32 * inv_alpha) / 255) as u8;
        let b = ((fg.b() as u32 * alpha + self.b() as u32 * inv_alpha) / 255) as u8;

        Color::from_rgb(r, g, b)
    }
}

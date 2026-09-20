use alloc::vec::Vec;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use crate::gui::framebuffer::Framebuffer;
use crate::gui::window::Application;
use pc_keyboard::DecodedKey;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Original,
    Grayscale,
    Invert,
    Sepia,
}

pub struct ImageItem {
    pub title: &'static str,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

pub struct ImageViewerApp {
    pub gallery: Vec<ImageItem>,
    pub current_idx: usize,
    pub zoom: usize, // 1 or 2
    pub filter: FilterMode,
}

impl ImageViewerApp {
    pub fn new() -> Self {
        let w = 160;
        let h = 100;

        let img1 = generate_mouros_logo_art(w, h);
        let img2 = generate_synthwave_art(w, h);
        let img3 = generate_nebula_art(w, h);

        let gallery = alloc::vec![
            ImageItem { title: "Mouros OS 64-Bit Crest", width: w, height: h, pixels: img1 },
            ImageItem { title: "Retro Synthwave Sunset", width: w, height: h, pixels: img2 },
            ImageItem { title: "Deep Space Nebula", width: w, height: h, pixels: img3 },
        ];

        Self {
            gallery,
            current_idx: 0,
            zoom: 1,
            filter: FilterMode::Original,
        }
    }

    pub fn next(&mut self) {
        self.current_idx = (self.current_idx + 1) % self.gallery.len();
    }

    pub fn prev(&mut self) {
        if self.current_idx == 0 {
            self.current_idx = self.gallery.len() - 1;
        } else {
            self.current_idx -= 1;
        }
    }
}

impl Application for ImageViewerApp {
    fn title(&self) -> &str {
        "Mouros Image Viewer"
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode('n') | DecodedKey::Unicode('N') => self.next(),
            DecodedKey::Unicode('p') | DecodedKey::Unicode('P') => self.prev(),
            DecodedKey::Unicode('z') | DecodedKey::Unicode('Z') => {
                self.zoom = if self.zoom == 1 { 2 } else { 1 };
            }
            DecodedKey::Unicode('f') | DecodedKey::Unicode('F') => {
                self.filter = match self.filter {
                    FilterMode::Original => FilterMode::Grayscale,
                    FilterMode::Grayscale => FilterMode::Invert,
                    FilterMode::Invert => FilterMode::Sepia,
                    FilterMode::Sepia => FilterMode::Original,
                };
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, x: isize, y: isize, left: bool) {
        if !left {
            return;
        }

        // Controls bar is at top: (y: 6..28)
        if y >= 6 && y <= 28 {
            if x >= 10 && x <= 55 {
                self.prev();
            } else if x >= 60 && x <= 105 {
                self.next();
            } else if x >= 115 && x <= 170 {
                self.zoom = if self.zoom == 1 { 2 } else { 1 };
            } else if x >= 180 && x <= 250 {
                self.filter = match self.filter {
                    FilterMode::Original => FilterMode::Grayscale,
                    FilterMode::Grayscale => FilterMode::Invert,
                    FilterMode::Invert => FilterMode::Sepia,
                    FilterMode::Sepia => FilterMode::Original,
                };
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

        // Dark slate gallery background
        fb.fill_rect(bx, by, bw, bh, Color::from_rgb(15, 23, 42));

        // 1. Controls Toolbar
        let tb_y = by + 6;
        draw_tool_btn(fb, bx + 10, tb_y, 45, 22, "< PREV");
        draw_tool_btn(fb, bx + 60, tb_y, 45, 22, "NEXT >");
        let zoom_str = if self.zoom == 1 { "ZOOM 1x" } else { "ZOOM 2x" };
        draw_tool_btn(fb, bx + 115, tb_y, 55, 22, zoom_str);

        let filter_str = match self.filter {
            FilterMode::Original => "FILTER: OFF",
            FilterMode::Grayscale => "FILTER: GRAY",
            FilterMode::Invert => "FILTER: INV",
            FilterMode::Sepia => "FILTER: SEPIA",
        };
        draw_tool_btn(fb, bx + 180, tb_y, 80, 22, filter_str);

        // 2. Picture viewport
        let vp_x = bx + 10;
        let vp_y = by + 34;
        let vp_w = bw - 20;
        let vp_h = bh.saturating_sub(60);

        fb.fill_rect(vp_x, vp_y, vp_w, vp_h, Color::from_rgb(10, 15, 29));
        fb.draw_rect(vp_x, vp_y, vp_w, vp_h, Color::from_rgb(51, 65, 85));

        let cur_img = &self.gallery[self.current_idx];
        let disp_w = cur_img.width * self.zoom;
        let disp_h = cur_img.height * self.zoom;

        let img_x = vp_x + ((vp_w as isize - disp_w as isize) / 2).max(4);
        let img_y = vp_y + ((vp_h as isize - disp_h as isize) / 2).max(4);

        // Blit image pixels with active filter
        for iy in 0..cur_img.height {
            for ix in 0..cur_img.width {
                let p = cur_img.pixels[iy * cur_img.width + ix];
                let mut c = Color::from_u32(p);

                c = match self.filter {
                    FilterMode::Original => c,
                    FilterMode::Grayscale => {
                        let gray = ((c.r() as u16 * 77 + c.g() as u16 * 150 + c.b() as u16 * 29) >> 8) as u8;
                        Color::from_rgb(gray, gray, gray)
                    }
                    FilterMode::Invert => Color::from_rgb(255 - c.r(), 255 - c.g(), 255 - c.b()),
                    FilterMode::Sepia => {
                        let r = ((c.r() as u32 * 393 + c.g() as u32 * 769 + c.b() as u32 * 189) / 1000).min(255) as u8;
                        let g = ((c.r() as u32 * 349 + c.g() as u32 * 686 + c.b() as u32 * 168) / 1000).min(255) as u8;
                        let b = ((c.r() as u32 * 272 + c.g() as u32 * 534 + c.b() as u32 * 131) / 1000).min(255) as u8;
                        Color::from_rgb(r, g, b)
                    }
                };

                let px = img_x + (ix * self.zoom) as isize;
                let py = img_y + (iy * self.zoom) as isize;

                if self.zoom == 1 {
                    if px >= vp_x + 1 && px < vp_x + vp_w as isize - 1 && py >= vp_y + 1 && py < vp_y + vp_h as isize - 1 {
                        fb.draw_pixel(px, py, c);
                    }
                } else {
                    fb.fill_rect(px, py, self.zoom, self.zoom, c);
                }
            }
        }

        // 3. Information Footer
        let footer_y = by + bh as isize - 20;
        let info = cur_img.title;
        fb.draw_string(bx + 14, footer_y, info, Color::from_rgb(56, 189, 248));

        let meta = "160x100 32bpp TrueColor";
        let mw = meta.len() * FONT_WIDTH;
        fb.draw_string(bx + bw as isize - mw as isize - 14, footer_y, meta, Color::from_rgb(148, 163, 184));
    }
}

fn draw_tool_btn(fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize, text: &str) {
    fb.fill_rect(x, y, w, h, Color::from_rgb(30, 41, 59));
    fb.draw_rect(x, y, w, h, Color::from_rgb(71, 85, 105));
    let tw = text.len() * FONT_WIDTH;
    let tx = x + ((w as isize - tw as isize) / 2);
    let ty = y + ((h as isize - 8) / 2);
    fb.draw_string(tx, ty, text, Color::from_rgb(226, 232, 240));
}

// Procedural Artwork 1: Mouros 64-Bit Crest
fn generate_mouros_logo_art(w: usize, h: usize) -> Vec<u32> {
    let mut buf = alloc::vec![0u32; w * h];
    for y in 0..h {
        let t = ((y * 256) / h.max(1)) as u16;
        let bg = Color::from_rgb(15, 23, 42).lerp(Color::from_rgb(30, 58, 95), t);
        for x in 0..w {
            buf[y * w + x] = bg.to_u32();
        }
    }

    let cx = w / 2;
    let cy = h / 2;

    // Outer gold ring
    for y in 0..h as isize {
        for x in 0..w as isize {
            let dx = x - cx as isize;
            let dy = y - cy as isize;
            let dist_sq = dx * dx + dy * dy;

            // Outer Shield circle (r = 38)
            if dist_sq <= 38 * 38 && dist_sq >= 34 * 34 {
                buf[y as usize * w + x as usize] = Color::from_rgb(251, 191, 36).to_u32();
            } else if dist_sq < 34 * 34 && dist_sq >= 32 * 32 {
                buf[y as usize * w + x as usize] = Color::from_rgb(180, 83, 9).to_u32();
            }
        }
    }

    // Inner Emblem: stylized 'M'
    let m_color = Color::from_rgb(56, 189, 248).to_u32();
    for row in 0..24 {
        let py = cy - 12 + row;
        // Left pillar
        for col in 0..4 {
            buf[py * w + (cx - 16 + col)] = m_color;
        }
        // Right pillar
        for col in 0..4 {
            buf[py * w + (cx + 12 + col)] = m_color;
        }
        // Chevrons
        if row <= 12 {
            for col in 0..4 {
                buf[(py) * w + (cx - 16 + row + col)] = m_color;
                buf[(py) * w + (cx + 12 - row + col)] = m_color;
            }
        }
    }

    buf
}

// Procedural Artwork 2: Retro Synthwave Sunset
fn generate_synthwave_art(w: usize, h: usize) -> Vec<u32> {
    let mut buf = alloc::vec![0u32; w * h];
    let horizon = 60;

    for y in 0..h {
        for x in 0..w {
            let col = if y < horizon {
                // Sky gradient: Indigo to Magenta
                let t = ((y * 256) / horizon.max(1)) as u16;
                Color::from_rgb(24, 16, 38).lerp(Color::from_rgb(192, 38, 211), t)
            } else {
                // Floor wireframe grid
                let mut ground = Color::from_rgb(15, 10, 25);
                // Horizontal grid lines
                let gy = y - horizon;
                if gy == 4 || gy == 10 || gy == 18 || gy == 28 || gy == 38 {
                    ground = Color::from_rgb(56, 189, 248);
                }
                // Perspective vertical grid lines
                let dx = (x as isize - (w as isize / 2)).abs();
                if dx == (gy as isize / 2) || dx == (gy as isize) || dx == (gy as isize * 2) {
                    ground = Color::from_rgb(244, 63, 94);
                }
                ground
            };
            buf[y * w + x] = col.to_u32();
        }
    }

    // Sun disc
    let scx = (w / 2) as isize;
    let scy = (horizon - 10) as isize;
    let r = 24;
    for y in (scy - r)..(scy + r) {
        if y < 0 || y >= h as isize { continue; }
        for x in (scx - r)..(scx + r) {
            if x < 0 || x >= w as isize { continue; }
            let dist_sq = (x - scx) * (x - scx) + (y - scy) * (y - scy);
            if dist_sq <= r * r {
                // Horizontal sunset stripes cut through bottom of sun
                if y > scy && (y % 4 == 0 || y % 4 == 1) {
                    continue;
                }
                let st = (((y - scy + r) * 256) / (2 * r.max(1))) as u16;
                let sun_c = Color::from_rgb(253, 224, 71).lerp(Color::from_rgb(239, 68, 68), st);
                buf[y as usize * w + x as usize] = sun_c.to_u32();
            }
        }
    }

    buf
}

// Procedural Artwork 3: Deep Space Nebula
fn generate_nebula_art(w: usize, h: usize) -> Vec<u32> {
    let mut buf = alloc::vec![0u32; w * h];
    for y in 0..h {
        for x in 0..w {
            let dx = (x as isize - 80) as f32 / 40.0;
            let dy = (y as isize - 50) as f32 / 25.0;
            let d_sq = dx * dx + dy * dy;

            let c = if d_sq < 1.0 {
                Color::from_rgb(168, 85, 247) // Purple core
            } else if d_sq < 4.0 {
                Color::from_rgb(37, 99, 235) // Blue cloud
            } else {
                Color::from_rgb(10, 15, 30) // Deep void
            };
            buf[y * w + x] = c.to_u32();
        }
    }

    // Twinkling stars
    for i in 0..40 {
        let sx = (i * 37 + 13) % w;
        let sy = (i * 53 + 7) % h;
        buf[sy * w + sx] = Color::WHITE.to_u32();
    }

    buf
}

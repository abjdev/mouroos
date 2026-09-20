use crate::gui::color::Color;
use crate::gui::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIcon {
    Terminal,
    SysInfo,
    Calculator,
    Notepad,
    Snake,
    Music,
    ImageViewer,
    Settings,
    ElfRunner,
    Doom,
    Power,
}

impl AppIcon {
    pub fn from_id(id: usize) -> Self {
        match id {
            0 => AppIcon::Terminal,
            1 => AppIcon::SysInfo,
            2 => AppIcon::Calculator,
            3 => AppIcon::Notepad,
            4 => AppIcon::Snake,
            5 => AppIcon::Music,
            6 => AppIcon::ImageViewer,
            7 => AppIcon::Settings,
            8 => AppIcon::ElfRunner,
            9 => AppIcon::Doom,
            _ => AppIcon::Power,
        }
    }
}

impl From<usize> for AppIcon {
    fn from(id: usize) -> Self {
        Self::from_id(id)
    }
}

/// Draw a 24x24 pixel-art icon for the desktop
pub fn draw_icon_24(fb: &mut Framebuffer, x: isize, y: isize, icon: AppIcon) {
    match icon {
        AppIcon::Terminal => {
            // Dark console window frame
            fb.fill_rect(x + 1, y + 1, 22, 22, Color::from_rgb(15, 23, 42));
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(71, 85, 105));
            // Titlebar
            fb.fill_rect(x + 2, y + 2, 20, 5, Color::from_rgb(30, 41, 59));
            fb.draw_pixel(x + 4, y + 4, Color::from_rgb(239, 68, 68)); // red dot
            fb.draw_pixel(x + 7, y + 4, Color::from_rgb(234, 179, 8)); // yellow dot
            fb.draw_pixel(x + 10, y + 4, Color::from_rgb(34, 197, 94)); // green dot
            // Prompt '>_'
            // '>' symbol
            let prompt_color = Color::from_rgb(34, 197, 94);
            fb.draw_pixel(x + 4, y + 11, prompt_color);
            fb.draw_pixel(x + 5, y + 12, prompt_color);
            fb.draw_pixel(x + 6, y + 13, prompt_color);
            fb.draw_pixel(x + 5, y + 14, prompt_color);
            fb.draw_pixel(x + 4, y + 15, prompt_color);
            // '_' cursor
            fb.fill_rect(x + 9, y + 15, 5, 2, Color::from_rgb(56, 189, 248));
        }

        AppIcon::SysInfo => {
            // CPU Chip outline
            let chip_color = Color::from_rgb(30, 41, 59);
            let gold_pin = Color::from_rgb(245, 158, 11);
            let core_color = Color::from_rgb(37, 99, 235);
            // Gold pins (top & bottom)
            for i in 0..5 {
                let px = x + 4 + i * 4;
                fb.fill_rect(px, y, 2, 3, gold_pin);
                fb.fill_rect(px, y + 21, 2, 3, gold_pin);
            }
            // Gold pins (left & right)
            for i in 0..5 {
                let py = y + 4 + i * 4;
                fb.fill_rect(x, py, 3, 2, gold_pin);
                fb.fill_rect(x + 21, py, 3, 2, gold_pin);
            }
            // Ceramic package body
            fb.fill_rect(x + 3, y + 3, 18, 18, chip_color);
            fb.draw_rect(x + 3, y + 3, 18, 18, Color::from_rgb(71, 85, 105));
            // Silicon Core die
            fb.fill_rect(x + 7, y + 7, 10, 10, core_color);
            fb.draw_rect(x + 7, y + 7, 10, 10, Color::from_rgb(96, 165, 250));
            // Center indicator
            fb.fill_rect(x + 10, y + 10, 4, 4, Color::WHITE);
        }

        AppIcon::Calculator => {
            // Calculator body
            fb.fill_rect(x + 2, y + 1, 20, 22, Color::from_rgb(30, 41, 59));
            fb.draw_rect(x + 2, y + 1, 20, 22, Color::from_rgb(100, 116, 139));
            // LCD screen
            fb.fill_rect(x + 5, y + 3, 14, 5, Color::from_rgb(20, 83, 45));
            fb.draw_rect(x + 5, y + 3, 14, 5, Color::from_rgb(34, 197, 94));
            // Digits on LCD
            fb.fill_rect(x + 13, y + 5, 4, 1, Color::from_rgb(134, 239, 172));
            // Button grid (3x3)
            let btn_color = Color::from_rgb(71, 85, 105);
            for row in 0..3 {
                for col in 0..3 {
                    let bx = x + 5 + col * 4;
                    let by = y + 10 + row * 4;
                    let c = if col == 2 {
                        Color::from_rgb(245, 158, 11) // Orange operator
                    } else if row == 2 && col == 1 {
                        Color::from_rgb(14, 165, 233) // Blue equals
                    } else {
                        btn_color
                    };
                    fb.fill_rect(bx, by, 3, 3, c);
                }
            }
        }

        AppIcon::Notepad => {
            // Document paper
            fb.fill_rect(x + 3, y + 1, 15, 22, Color::from_rgb(248, 250, 252));
            fb.draw_rect(x + 3, y + 1, 15, 22, Color::from_rgb(148, 163, 184));
            // Folded top right corner
            fb.fill_rect(x + 14, y + 1, 4, 4, Color::from_rgb(203, 213, 225));
            // Text lines
            let line_color = Color::from_rgb(100, 116, 139);
            fb.fill_rect(x + 6, y + 6, 8, 1, Color::from_rgb(59, 130, 246)); // Title blue
            fb.fill_rect(x + 6, y + 9, 9, 1, line_color);
            fb.fill_rect(x + 6, y + 12, 10, 1, line_color);
            fb.fill_rect(x + 6, y + 15, 7, 1, line_color);
            fb.fill_rect(x + 6, y + 18, 9, 1, line_color);
            // Yellow pencil angled across corner
            let pencil_color = Color::from_rgb(234, 179, 8);
            fb.fill_rect(x + 14, y + 14, 3, 3, pencil_color);
            fb.fill_rect(x + 16, y + 12, 3, 3, pencil_color);
            fb.fill_rect(x + 18, y + 10, 3, 3, Color::from_rgb(244, 63, 94)); // Eraser
            fb.draw_pixel(x + 13, y + 17, Color::from_rgb(15, 23, 42)); // Tip
        }

        AppIcon::Snake => {
            // Dark field
            fb.fill_rect(x + 1, y + 1, 22, 22, Color::from_rgb(15, 23, 42));
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(51, 65, 85));
            // Green snake segments
            let snake_c = Color::from_rgb(34, 197, 94);
            let snake_head = Color::from_rgb(74, 222, 128);
            // Snake body path
            fb.fill_rect(x + 4, y + 16, 3, 3, snake_c);
            fb.fill_rect(x + 7, y + 16, 3, 3, snake_c);
            fb.fill_rect(x + 10, y + 16, 3, 3, snake_c);
            fb.fill_rect(x + 10, y + 13, 3, 3, snake_c);
            fb.fill_rect(x + 10, y + 10, 3, 3, snake_c);
            fb.fill_rect(x + 13, y + 10, 3, 3, snake_c);
            fb.fill_rect(x + 16, y + 10, 3, 3, snake_c);
            fb.fill_rect(x + 16, y + 7, 3, 3, snake_head);
            // Snake eyes
            fb.draw_pixel(x + 17, y + 7, Color::from_rgb(15, 23, 42));
            // Red apple food
            fb.fill_rect(x + 5, y + 5, 4, 4, Color::from_rgb(239, 68, 68));
            fb.draw_pixel(x + 6, y + 4, Color::from_rgb(34, 197, 94)); // leaf
        }

        AppIcon::Music => {
            // Music card
            fb.fill_rect(x + 1, y + 1, 22, 22, Color::from_rgb(24, 24, 37));
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(168, 85, 247));
            // Musical note (eighth note)
            let note_c = Color::from_rgb(244, 63, 94);
            // Note notehead
            fb.fill_rect(x + 5, y + 14, 5, 4, note_c);
            // Stem
            fb.fill_rect(x + 8, y + 5, 2, 10, note_c);
            // Flag
            fb.fill_rect(x + 10, y + 5, 4, 3, note_c);
            fb.fill_rect(x + 12, y + 7, 3, 3, note_c);
            // Audio equalizer bars on right
            let eq_c = Color::from_rgb(56, 189, 248);
            fb.fill_rect(x + 14, y + 13, 2, 6, eq_c);
            fb.fill_rect(x + 17, y + 9, 2, 10, Color::from_rgb(34, 197, 94));
            fb.fill_rect(x + 20, y + 11, 2, 8, Color::from_rgb(245, 158, 11));
        }

        AppIcon::ImageViewer => {
            // Photo frame
            fb.fill_rect(x + 1, y + 2, 22, 20, Color::from_rgb(30, 41, 59));
            fb.draw_rect(x + 1, y + 2, 22, 20, Color::from_rgb(148, 163, 184));
            // Sky gradient
            fb.fill_rect(x + 3, y + 4, 18, 10, Color::from_rgb(30, 64, 175));
            // Golden Sun
            fb.fill_rect(x + 14, y + 5, 4, 4, Color::from_rgb(251, 191, 36));
            // Mountain peak left (emerald)
            let mtn1 = Color::from_rgb(16, 185, 129);
            fb.fill_rect(x + 3, y + 14, 8, 6, mtn1);
            fb.fill_rect(x + 5, y + 12, 4, 2, mtn1);
            fb.fill_rect(x + 6, y + 10, 2, 2, mtn1);
            // Mountain peak right (slate blue)
            let mtn2 = Color::from_rgb(79, 70, 229);
            fb.fill_rect(x + 10, y + 12, 9, 8, mtn2);
            fb.fill_rect(x + 12, y + 9, 5, 3, mtn2);
            fb.fill_rect(x + 14, y + 8, 2, 1, Color::WHITE); // Snow cap
        }

        AppIcon::Settings => {
            // Mechanical Gear / Cogwheel
            let gear_dark = Color::from_rgb(30, 41, 59);
            let gear_metal = Color::from_rgb(148, 163, 184);
            let gear_accent = Color::from_rgb(56, 189, 248);

            fb.fill_rect(x + 1, y + 1, 22, 22, gear_dark);
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(71, 85, 105));

            // Gear Teeth (North, South, East, West)
            fb.fill_rect(x + 10, y + 3, 4, 3, gear_metal); // N
            fb.fill_rect(x + 10, y + 18, 4, 3, gear_metal); // S
            fb.fill_rect(x + 3, y + 10, 3, 4, gear_metal); // W
            fb.fill_rect(x + 18, y + 10, 3, 4, gear_metal); // E
            // Diagonal teeth
            fb.fill_rect(x + 5, y + 5, 3, 3, gear_metal); // NW
            fb.fill_rect(x + 16, y + 5, 3, 3, gear_metal); // NE
            fb.fill_rect(x + 5, y + 16, 3, 3, gear_metal); // SW
            fb.fill_rect(x + 16, y + 16, 3, 3, gear_metal); // SE

            // Main gear circle
            fb.fill_rect(x + 6, y + 6, 12, 12, gear_metal);
            // Gear hub / center hole
            fb.fill_rect(x + 9, y + 9, 6, 6, gear_dark);
            fb.fill_rect(x + 11, y + 11, 2, 2, gear_accent);
        }

        AppIcon::ElfRunner => {
            // Executable / Rocket Icon
            fb.fill_rect(x + 1, y + 1, 22, 22, Color::from_rgb(15, 23, 42));
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(244, 63, 94));

            let rocket_c = Color::from_rgb(244, 63, 94);
            let nose_c = Color::from_rgb(251, 113, 133);
            let fin_c = Color::from_rgb(148, 163, 184);
            let flame_c = Color::from_rgb(245, 158, 11);

            // Nose cone
            fb.fill_rect(x + 14, y + 4, 3, 3, nose_c);
            fb.draw_pixel(x + 16, y + 3, nose_c);
            // Body
            fb.fill_rect(x + 10, y + 7, 5, 5, rocket_c);
            fb.fill_rect(x + 7, y + 10, 5, 5, rocket_c);
            // Window porthole
            fb.fill_rect(x + 11, y + 8, 2, 2, Color::from_rgb(56, 189, 248));
            // Fins
            fb.fill_rect(x + 5, y + 7, 2, 4, fin_c);
            fb.fill_rect(x + 13, y + 15, 4, 2, fin_c);
            // Booster Flame
            fb.fill_rect(x + 4, y + 14, 3, 3, flame_c);
            fb.fill_rect(x + 2, y + 16, 3, 3, Color::from_rgb(239, 68, 68));
        }

        AppIcon::Doom => {
            // Demonic plate background
            fb.fill_rect(x + 1, y + 1, 22, 22, Color::from_rgb(30, 10, 10));
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(185, 28, 28));

            // Horns (gold/bone)
            let horn_c = Color::from_rgb(234, 179, 8);
            fb.fill_rect(x + 4, y + 4, 3, 3, horn_c);
            fb.fill_rect(x + 5, y + 7, 2, 3, horn_c);
            fb.fill_rect(x + 17, y + 4, 3, 3, horn_c);
            fb.fill_rect(x + 17, y + 7, 2, 3, horn_c);

            // Demon head (crimson)
            let head_c = Color::from_rgb(220, 38, 38);
            fb.fill_rect(x + 7, y + 7, 10, 11, head_c);
            fb.fill_rect(x + 6, y + 9, 12, 7, head_c);

            // Glowing yellow eyes
            let eye_c = Color::from_rgb(250, 204, 21);
            fb.fill_rect(x + 8, y + 11, 2, 2, eye_c);
            fb.fill_rect(x + 14, y + 11, 2, 2, eye_c);

            // Fangs / Teeth
            fb.fill_rect(x + 9, y + 15, 2, 2, Color::WHITE);
            fb.fill_rect(x + 13, y + 15, 2, 2, Color::WHITE);

            // Lava base
            fb.fill_rect(x + 3, y + 20, 18, 2, Color::from_rgb(249, 115, 22));
        }

        AppIcon::Power => {
            // Power off / reboot circle and line
            fb.fill_rect(x + 1, y + 1, 22, 22, Color::from_rgb(69, 10, 10));
            fb.draw_rect(x + 1, y + 1, 22, 22, Color::from_rgb(239, 68, 68));
            // Power arc
            let red = Color::from_rgb(248, 113, 113);
            fb.draw_rect(x + 5, y + 7, 14, 12, red);
            fb.fill_rect(x + 9, y + 6, 6, 3, Color::from_rgb(69, 10, 10)); // Top opening
            // Center vertical power bar
            fb.fill_rect(x + 11, y + 4, 2, 8, Color::WHITE);
        }
    }
}

/// Draw a 16x16 pixel-art icon for the Start Menu
pub fn draw_icon_16(fb: &mut Framebuffer, x: isize, y: isize, icon: AppIcon) {
    match icon {
        AppIcon::Terminal => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(15, 23, 42));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(71, 85, 105));
            // Green prompt '>'
            let c = Color::from_rgb(34, 197, 94);
            fb.draw_pixel(x + 3, y + 5, c);
            fb.draw_pixel(x + 4, y + 6, c);
            fb.draw_pixel(x + 5, y + 7, c);
            fb.draw_pixel(x + 4, y + 8, c);
            fb.draw_pixel(x + 3, y + 9, c);
            // Cursor '_'
            fb.fill_rect(x + 7, y + 9, 4, 2, Color::from_rgb(56, 189, 248));
        }

        AppIcon::SysInfo => {
            fb.fill_rect(x + 2, y + 2, 12, 12, Color::from_rgb(30, 41, 59));
            fb.draw_rect(x + 2, y + 2, 12, 12, Color::from_rgb(245, 158, 11));
            // Core
            fb.fill_rect(x + 5, y + 5, 6, 6, Color::from_rgb(37, 99, 235));
            // Pins
            fb.fill_rect(x + 4, y, 2, 2, Color::from_rgb(245, 158, 11));
            fb.fill_rect(x + 10, y, 2, 2, Color::from_rgb(245, 158, 11));
            fb.fill_rect(x + 4, y + 14, 2, 2, Color::from_rgb(245, 158, 11));
            fb.fill_rect(x + 10, y + 14, 2, 2, Color::from_rgb(245, 158, 11));
        }

        AppIcon::Calculator => {
            fb.fill_rect(x + 1, y, 14, 16, Color::from_rgb(30, 41, 59));
            fb.draw_rect(x + 1, y, 14, 16, Color::from_rgb(100, 116, 139));
            // Screen
            fb.fill_rect(x + 3, y + 2, 10, 3, Color::from_rgb(20, 83, 45));
            // Buttons
            fb.fill_rect(x + 3, y + 7, 2, 2, Color::from_rgb(148, 163, 184));
            fb.fill_rect(x + 7, y + 7, 2, 2, Color::from_rgb(148, 163, 184));
            fb.fill_rect(x + 11, y + 7, 2, 2, Color::from_rgb(245, 158, 11));
            fb.fill_rect(x + 3, y + 11, 2, 2, Color::from_rgb(148, 163, 184));
            fb.fill_rect(x + 7, y + 11, 2, 2, Color::from_rgb(14, 165, 233));
            fb.fill_rect(x + 11, y + 11, 2, 2, Color::from_rgb(245, 158, 11));
        }

        AppIcon::Notepad => {
            fb.fill_rect(x + 2, y, 12, 16, Color::from_rgb(248, 250, 252));
            fb.draw_rect(x + 2, y, 12, 16, Color::from_rgb(148, 163, 184));
            fb.fill_rect(x + 4, y + 4, 6, 1, Color::from_rgb(59, 130, 246));
            fb.fill_rect(x + 4, y + 7, 8, 1, Color::from_rgb(100, 116, 139));
            fb.fill_rect(x + 4, y + 10, 7, 1, Color::from_rgb(100, 116, 139));
            // Pencil
            fb.fill_rect(x + 9, y + 11, 4, 3, Color::from_rgb(234, 179, 8));
        }

        AppIcon::Snake => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(15, 23, 42));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(51, 65, 85));
            // Snake
            let sc = Color::from_rgb(34, 197, 94);
            fb.fill_rect(x + 3, y + 11, 2, 2, sc);
            fb.fill_rect(x + 5, y + 11, 2, 2, sc);
            fb.fill_rect(x + 7, y + 11, 2, 2, sc);
            fb.fill_rect(x + 7, y + 8, 2, 2, sc);
            fb.fill_rect(x + 10, y + 8, 2, 2, sc);
            fb.fill_rect(x + 10, y + 5, 2, 2, Color::from_rgb(74, 222, 128));
            // Apple
            fb.fill_rect(x + 3, y + 4, 3, 3, Color::from_rgb(239, 68, 68));
        }

        AppIcon::Music => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(24, 24, 37));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(168, 85, 247));
            let nc = Color::from_rgb(244, 63, 94);
            fb.fill_rect(x + 3, y + 10, 4, 3, nc);
            fb.fill_rect(x + 5, y + 4, 2, 8, nc);
            fb.fill_rect(x + 7, y + 4, 3, 2, nc);
            // Equalizer
            fb.fill_rect(x + 11, y + 8, 1, 5, Color::from_rgb(56, 189, 248));
            fb.fill_rect(x + 13, y + 5, 1, 8, Color::from_rgb(34, 197, 94));
        }

        AppIcon::ImageViewer => {
            fb.fill_rect(x, y + 1, 16, 14, Color::from_rgb(30, 41, 59));
            fb.draw_rect(x, y + 1, 16, 14, Color::from_rgb(148, 163, 184));
            // Sun
            fb.fill_rect(x + 10, y + 3, 3, 3, Color::from_rgb(251, 191, 36));
            // Mountains
            fb.fill_rect(x + 2, y + 9, 6, 5, Color::from_rgb(16, 185, 129));
            fb.fill_rect(x + 7, y + 7, 7, 7, Color::from_rgb(79, 70, 229));
        }

        AppIcon::Settings => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(30, 41, 59));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(71, 85, 105));
            let gm = Color::from_rgb(148, 163, 184);
            // Teeth
            fb.fill_rect(x + 7, y + 1, 2, 2, gm);
            fb.fill_rect(x + 7, y + 13, 2, 2, gm);
            fb.fill_rect(x + 1, y + 7, 2, 2, gm);
            fb.fill_rect(x + 13, y + 7, 2, 2, gm);
            // Gear body
            fb.fill_rect(x + 4, y + 4, 8, 8, gm);
            fb.fill_rect(x + 6, y + 6, 4, 4, Color::from_rgb(15, 23, 42));
            fb.fill_rect(x + 7, y + 7, 2, 2, Color::from_rgb(56, 189, 248));
        }

        AppIcon::ElfRunner => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(15, 23, 42));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(244, 63, 94));
            let rc = Color::from_rgb(244, 63, 94);
            fb.fill_rect(x + 9, y + 3, 3, 3, Color::from_rgb(251, 113, 133));
            fb.fill_rect(x + 6, y + 6, 4, 4, rc);
            fb.fill_rect(x + 3, y + 9, 4, 4, rc);
            fb.fill_rect(x + 2, y + 12, 2, 2, Color::from_rgb(245, 158, 11)); // flame
        }

        AppIcon::Doom => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(30, 10, 10));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(220, 38, 38));
            // Horns
            let horn_c = Color::from_rgb(234, 179, 8);
            fb.draw_pixel(x + 3, y + 3, horn_c);
            fb.draw_pixel(x + 4, y + 4, horn_c);
            fb.draw_pixel(x + 12, y + 3, horn_c);
            fb.draw_pixel(x + 11, y + 4, horn_c);
            // Face
            fb.fill_rect(x + 4, y + 5, 8, 7, Color::from_rgb(220, 38, 38));
            // Eyes
            fb.draw_pixel(x + 5, y + 7, Color::from_rgb(250, 204, 21));
            fb.draw_pixel(x + 10, y + 7, Color::from_rgb(250, 204, 21));
            // Fangs
            fb.draw_pixel(x + 6, y + 10, Color::WHITE);
            fb.draw_pixel(x + 9, y + 10, Color::WHITE);
        }

        AppIcon::Power => {
            fb.fill_rect(x, y, 16, 16, Color::from_rgb(69, 10, 10));
            fb.draw_rect(x, y, 16, 16, Color::from_rgb(239, 68, 68));
            fb.draw_rect(x + 3, y + 4, 10, 9, Color::from_rgb(248, 113, 113));
            fb.fill_rect(x + 6, y + 3, 4, 3, Color::from_rgb(69, 10, 10));
            fb.fill_rect(x + 7, y + 2, 2, 6, Color::WHITE);
        }
    }
}

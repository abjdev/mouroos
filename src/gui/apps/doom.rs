use super::super::color::Color;
use super::super::framebuffer::Framebuffer;
use super::super::window::Application;
use alloc::format;
use alloc::string::String;
use neurodoom::{Button, Buttons, ClassicEngine, PeerId, PlayerAction, SCREENHEIGHT, SCREENWIDTH};
use pc_keyboard::{DecodedKey, KeyCode, KeyEvent, KeyState};

pub static DOOM_WAD: &[u8] = include_bytes!("../../../samples/doom/doom1.wad");

const EPISODE_MAPS: [&str; 9] = [
    "E1M1", "E1M2", "E1M3", "E1M4", "E1M5", "E1M6", "E1M7", "E1M8", "E1M9",
];

pub struct DoomApp {
    engine: Option<ClassicEngine>,
    current_map_idx: usize,
    scale: usize,
    is_paused: bool,
    show_help: bool,
    tick_acc: usize,

    // Active held key states for multi-key simultaneous gameplay
    pub key_forward: bool,
    pub key_backward: bool,
    pub key_turn_left: bool,
    pub key_turn_right: bool,
    pub key_strafe_left: bool,
    pub key_strafe_right: bool,
    pub key_attack: bool,
    pub key_use: bool,
    pub key_shift: bool,

    // Debounce flags for single-press toggles
    pub key_m: bool,
    pub key_p: bool,
    pub key_r: bool,
    pub key_h: bool,
    pub key_z: bool,

    // Mouse impulse & weapon selection
    attack_impulse: u8,
    use_impulse: u8,
    weapon_select: u8,

    // Performance metrics & status
    fps: usize,
    frame_count: usize,
    fps_timer: usize,
    status_msg: Option<(String, usize)>,
}

impl DoomApp {
    pub fn new() -> Self {
        Self::with_map("E1M1")
    }

    pub fn with_map(map_name: &str) -> Self {
        let map_idx = EPISODE_MAPS
            .iter()
            .position(|&m| m.eq_ignore_ascii_case(map_name))
            .unwrap_or(0);

        let target_map = EPISODE_MAPS[map_idx];
        let engine = ClassicEngine::new(DOOM_WAD, target_map).ok();

        DoomApp {
            engine,
            current_map_idx: map_idx,
            scale: 2,
            is_paused: false,
            show_help: false,
            tick_acc: 0,
            key_forward: false,
            key_backward: false,
            key_turn_left: false,
            key_turn_right: false,
            key_strafe_left: false,
            key_strafe_right: false,
            key_attack: false,
            key_use: false,
            key_shift: false,
            key_m: false,
            key_p: false,
            key_r: false,
            key_h: false,
            key_z: false,
            attack_impulse: 0,
            use_impulse: 0,
            weapon_select: 0,
            fps: 35,
            frame_count: 0,
            fps_timer: 0,
            status_msg: None,
        }
    }

    pub fn clear_inputs(&mut self) {
        self.key_forward = false;
        self.key_backward = false;
        self.key_turn_left = false;
        self.key_turn_right = false;
        self.key_strafe_left = false;
        self.key_strafe_right = false;
        self.key_attack = false;
        self.key_use = false;
        self.key_shift = false;
        self.key_m = false;
        self.key_p = false;
        self.key_r = false;
        self.key_h = false;
        self.key_z = false;
        self.attack_impulse = 0;
        self.use_impulse = 0;
    }

    pub fn load_map(&mut self, map_name: &str) {
        let map_idx = EPISODE_MAPS
            .iter()
            .position(|&m| m.eq_ignore_ascii_case(map_name))
            .unwrap_or(0);
        self.current_map_idx = map_idx;
        let target_map = EPISODE_MAPS[map_idx];
        match ClassicEngine::new(DOOM_WAD, target_map) {
            Ok(eng) => {
                self.engine = Some(eng);
                self.status_msg = Some((format!("Loaded {}", target_map), 70));
            }
            Err(_) => {
                self.status_msg = Some((format!("Failed to load {}", target_map), 100));
            }
        }
    }

    pub fn next_map(&mut self) {
        let next_idx = (self.current_map_idx + 1) % EPISODE_MAPS.len();
        self.load_map(EPISODE_MAPS[next_idx]);
    }

    pub fn restart_map(&mut self) {
        self.load_map(EPISODE_MAPS[self.current_map_idx]);
    }

    fn step(&mut self) {
        if self.is_paused {
            return;
        }

        let speed: i8 = if self.key_shift { 50 } else { 35 };
        let strafe_speed: i8 = if self.key_shift { 40 } else { 30 };
        let turn_speed: i16 = if self.key_shift { 1200 } else { 900 };

        let mut forward_move: i8 = 0;
        if self.key_forward {
            forward_move = forward_move.saturating_add(speed);
        }
        if self.key_backward {
            forward_move = forward_move.saturating_sub(speed);
        }

        let mut side_move: i8 = 0;
        if self.key_strafe_right {
            side_move = side_move.saturating_add(strafe_speed);
        }
        if self.key_strafe_left {
            side_move = side_move.saturating_sub(strafe_speed);
        }

        let mut angle_turn: i16 = 0;
        if self.key_turn_left {
            angle_turn = angle_turn.saturating_add(turn_speed);
        }
        if self.key_turn_right {
            angle_turn = angle_turn.saturating_sub(turn_speed);
        }

        let mut buttons = Buttons::empty();
        if self.key_attack || self.attack_impulse > 0 {
            buttons |= Button::Attack;
            if self.attack_impulse > 0 {
                self.attack_impulse -= 1;
            }
        }
        if self.key_use || self.use_impulse > 0 {
            buttons |= Button::Use;
            if self.use_impulse > 0 {
                self.use_impulse -= 1;
            }
        }

        let mut action = PlayerAction::default();
        action.forward_move = forward_move;
        action.side_move = side_move;
        action.angle_turn = angle_turn;
        action.buttons = buttons;

        if self.weapon_select > 0 {
            action.weapon_select = self.weapon_select;
            self.weapon_select = 0;
        }

        if let Some(engine) = &mut self.engine {
            engine.tick_single(PeerId(0), action);
        }

        self.frame_count += 1;
        self.fps_timer += 1;
        if self.fps_timer >= 35 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.fps_timer = 0;
        }

        if let Some((_, expire)) = &mut self.status_msg {
            if *expire > 0 {
                *expire -= 1;
            } else {
                self.status_msg = None;
            }
        }
    }
}

impl Application for DoomApp {
    fn title(&self) -> &str {
        "DOOM (1993) - E1 Shareware"
    }

    fn on_blur(&mut self) {
        self.clear_inputs();
    }

    fn on_raw_key(&mut self, event: KeyEvent) {
        let is_down = matches!(event.state, KeyState::Down | KeyState::SingleShot);

        match event.code {
            KeyCode::W | KeyCode::ArrowUp => self.key_forward = is_down,
            KeyCode::S | KeyCode::ArrowDown => self.key_backward = is_down,
            KeyCode::A | KeyCode::ArrowLeft => self.key_turn_left = is_down,
            KeyCode::D | KeyCode::ArrowRight => self.key_turn_right = is_down,
            KeyCode::Q | KeyCode::OemComma => self.key_strafe_left = is_down,
            KeyCode::E | KeyCode::OemPeriod => self.key_strafe_right = is_down,
            KeyCode::F | KeyCode::LControl | KeyCode::RControl => self.key_attack = is_down,
            KeyCode::Spacebar | KeyCode::Return => self.key_use = is_down,
            KeyCode::LShift | KeyCode::RShift => self.key_shift = is_down,

            KeyCode::M => {
                if is_down && !self.key_m {
                    self.next_map();
                }
                self.key_m = is_down;
            }
            KeyCode::P => {
                if is_down && !self.key_p {
                    self.is_paused = !self.is_paused;
                }
                self.key_p = is_down;
            }
            KeyCode::R => {
                if is_down && !self.key_r {
                    self.restart_map();
                }
                self.key_r = is_down;
            }
            KeyCode::H => {
                if is_down && !self.key_h {
                    self.show_help = !self.show_help;
                }
                self.key_h = is_down;
            }
            KeyCode::Z => {
                if is_down && !self.key_z {
                    self.scale = if self.scale == 2 { 1 } else { 2 };
                }
                self.key_z = is_down;
            }

            KeyCode::Key1 => { if is_down { self.weapon_select = 1; } }
            KeyCode::Key2 => { if is_down { self.weapon_select = 2; } }
            KeyCode::Key3 => { if is_down { self.weapon_select = 3; } }
            KeyCode::Key4 => { if is_down { self.weapon_select = 4; } }
            KeyCode::Key5 => { if is_down { self.weapon_select = 5; } }
            KeyCode::Key6 => { if is_down { self.weapon_select = 6; } }
            KeyCode::Key7 => { if is_down { self.weapon_select = 7; } }

            _ => {}
        }
    }

    fn on_key(&mut self, _key: DecodedKey) {
        // Raw key handling in on_raw_key provides multi-key state tracking
    }

    fn on_mouse_click(&mut self, local_x: isize, local_y: isize, left: bool) {
        if !left {
            return;
        }

        // Top toolbar clicks (y: 2..24)
        if local_y >= 2 && local_y <= 24 {
            if local_x >= 8 && local_x <= 82 {
                self.next_map();
                return;
            } else if local_x >= 88 && local_x <= 153 {
                self.scale = if self.scale == 2 { 1 } else { 2 };
                return;
            } else if local_x >= 159 && local_x <= 219 {
                self.restart_map();
                return;
            } else if local_x >= 225 && local_x <= 280 {
                self.is_paused = !self.is_paused;
                return;
            } else if local_x >= 286 && local_x <= 352 {
                self.show_help = !self.show_help;
                return;
            }
        }

        // Clicks inside the Doom game viewport fire the active weapon
        if local_y > 24 {
            self.attack_impulse = 3;
        }
    }

    fn on_tick(&mut self) -> bool {
        if self.is_paused {
            return false;
        }

        // Rate limit simulation to 35 Hz using PIT 100 Hz timer
        self.tick_acc += 35;
        if self.tick_acc >= 100 {
            self.tick_acc -= 100;
            self.step();
            true
        } else {
            false
        }
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        // 1. Windows 98 Classic Gray casing
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::RETRO_FACE);

        // 2. Toolbar Header (height 26px)
        let tb_y = client_y + 2;
        let cur_map = EPISODE_MAPS[self.current_map_idx];
        draw_button(fb, client_x + 8, tb_y, 74, 22, &format!("MAP: {}", cur_map), Color::BLACK);
        let scale_lbl = if self.scale == 2 { "SCALE 2x" } else { "SCALE 1x" };
        draw_button(fb, client_x + 88, tb_y, 65, 22, scale_lbl, Color::BLACK);
        draw_button(fb, client_x + 159, tb_y, 60, 22, "RESTART", Color::BLACK);
        let pause_lbl = if self.is_paused { "RESUME" } else { "PAUSE" };
        draw_button(fb, client_x + 225, tb_y, 55, 22, pause_lbl, Color::BLACK);
        draw_button(fb, client_x + 286, tb_y, 66, 22, "HELP (H)", Color::BLACK);

        // FPS & Map Status on right side of toolbar
        let stat_str = format!("35Hz | {} FPS", self.fps);
        let stat_x = client_x + client_w as isize - (stat_str.len() * 8) as isize - 10;
        fb.draw_string(stat_x, tb_y + 6, &stat_str, Color::BLACK);

        if let Some((msg, _)) = &self.status_msg {
            fb.draw_string(client_x + 360, tb_y + 6, msg, Color::from_rgb(0, 128, 0));
        }

        // Toolbar etched groove
        fb.draw_groove(client_x, client_y + 26, client_w, 2);

        // 3. Render Doom Game Viewport
        let vp_top = client_y + 27;
        let vp_avail_h = client_h.saturating_sub(27);

        let disp_w = SCREENWIDTH * self.scale;
        let disp_h = SCREENHEIGHT * self.scale;

        let vp_x = client_x + ((client_w as isize - disp_w as isize) / 2).max(0);
        let vp_y = vp_top + ((vp_avail_h as isize - disp_h as isize) / 2).max(0);

        fb.draw_bevel_sunken(vp_x - 2, vp_y - 2, disp_w + 4, disp_h + 4);

        if let Some(engine) = &self.engine {
            let rgba = engine.framebuffer();

            if self.scale == 2 {
                // Fast 2x Nearest-Neighbor Blitter
                for dy in 0..SCREENHEIGHT {
                    let py1 = vp_y + (dy * 2) as isize;
                    let py2 = py1 + 1;

                    // Bounds check rows
                    if py1 < 0 || py2 >= fb.height as isize {
                        continue;
                    }

                    let doom_row_offset = dy * SCREENWIDTH * 4;
                    let r1_start = py1 as usize * fb.width;
                    let r2_start = py2 as usize * fb.width;

                    for dx in 0..SCREENWIDTH {
                        let px1 = vp_x + (dx * 2) as isize;
                        if px1 < 0 || px1 + 1 >= fb.width as isize {
                            continue;
                        }

                        let off = doom_row_offset + dx * 4;
                        let r = rgba[off] as u32;
                        let g = rgba[off + 1] as u32;
                        let b = rgba[off + 2] as u32;
                        let raw = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                        let idx1 = r1_start + px1 as usize;
                        let idx2 = r2_start + px1 as usize;

                        fb.backbuffer[idx1] = raw;
                        fb.backbuffer[idx1 + 1] = raw;
                        fb.backbuffer[idx2] = raw;
                        fb.backbuffer[idx2 + 1] = raw;
                    }
                }
            } else {
                // Fast 1x Direct Blitter
                for dy in 0..SCREENHEIGHT {
                    let py = vp_y + dy as isize;
                    if py < 0 || py >= fb.height as isize {
                        continue;
                    }

                    let doom_row_offset = dy * SCREENWIDTH * 4;
                    let r_start = py as usize * fb.width;

                    for dx in 0..SCREENWIDTH {
                        let px = vp_x + dx as isize;
                        if px < 0 || px >= fb.width as isize {
                            continue;
                        }

                        let off = doom_row_offset + dx * 4;
                        let r = rgba[off] as u32;
                        let g = rgba[off + 1] as u32;
                        let b = rgba[off + 2] as u32;
                        let raw = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                        fb.backbuffer[r_start + px as usize] = raw;
                    }
                }
            }
        } else {
            // Engine failed to load or missing WAD
            let err_x = client_x + (client_w as isize - 240) / 2;
            let err_y = client_y + (client_h as isize) / 2;
            fb.draw_string(err_x, err_y, "ERROR: Failed to load DOOM WAD", Color::from_rgb(239, 68, 68));
            fb.draw_string(err_x + 10, err_y + 18, "Check samples/doom/doom1.wad", Color::from_rgb(148, 163, 184));
        }

        // Viewport border
        fb.draw_rect(vp_x - 1, vp_y - 1, disp_w + 2, disp_h + 2, Color::from_rgb(60, 60, 75));

        // 4. Overlays: Pause & Help
        if self.is_paused {
            let pw = 240;
            let ph = 50;
            let px = client_x + (client_w as isize - pw) / 2;
            let py = vp_y + (disp_h as isize - ph) / 2;

            fb.fill_rect(px, py, pw as usize, ph as usize, Color::from_argb(220, 15, 23, 42));
            fb.draw_rect(px, py, pw as usize, ph as usize, Color::from_rgb(234, 179, 8));
            fb.draw_string(px + 30, py + 12, "GAME PAUSED", Color::from_rgb(234, 179, 8));
            fb.draw_string(px + 20, py + 28, "Press P to Resume Game", Color::WHITE);
        }

        if self.show_help {
            let hw = 380;
            let hh = 265;
            let hx = client_x + (client_w as isize - hw) / 2;
            let hy = vp_y + (disp_h as isize - hh) / 2;

            fb.fill_rect(hx, hy, hw as usize, hh as usize, Color::from_argb(235, 15, 23, 42));
            fb.draw_rect(hx, hy, hw as usize, hh as usize, Color::from_rgb(220, 38, 38));

            // Header
            fb.draw_gradient_v(hx + 1, hy + 1, hw as usize - 2, 22, Color::from_rgb(185, 28, 28), Color::from_rgb(127, 29, 29));
            fb.draw_string(hx + 10, hy + 5, "DOOM CONTROLS & INSTRUCTIONS", Color::WHITE);

            let lines = [
                ("Move Forward", "W  or  Up Arrow"),
                ("Move Backward", "S  or  Down Arrow"),
                ("Turn Left / Right", "A / D  or  Left / Right"),
                ("Strafe Left / Right", "Q / E  or  , / ."),
                ("Run / Sprint", "Shift (hold)"),
                ("Attack / Fire", "F  or  Ctrl  or  Left Click"),
                ("Use / Open / Switch", "Space  or  Enter"),
                ("Select Weapon (1-7)", "Number keys 1 to 7"),
                ("Next Map (E1M1-E1M9)", "M key"),
                ("Zoom (1x / 2x)", "Z key"),
                ("Pause / Resume", "P key"),
                ("Restart Level", "R key"),
                ("Toggle Help", "H key"),
            ];

            for (i, (action, bind)) in lines.iter().enumerate() {
                let ly = hy + 30 + (i as isize * 17);
                fb.draw_string(hx + 14, ly, action, Color::from_rgb(203, 213, 225));
                fb.draw_string(hx + 190, ly, bind, Color::from_rgb(250, 204, 21));
            }
        }
    }
}

fn draw_button(fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize, label: &str, _border: Color) {
    fb.draw_button(x, y, w, h, false);
    let lx = x + (w as isize - (label.len() * 8) as isize) / 2;
    let ly = y + (h as isize - 8) / 2;
    fb.draw_string(lx, ly, label, Color::BLACK);
}

#[test_case]
fn test_doom_multi_input() {
    let mut app = DoomApp::new();

    // 1. Press W (Forward)
    app.on_raw_key(KeyEvent {
        code: KeyCode::W,
        state: KeyState::Down,
    });
    assert!(app.key_forward);
    assert!(!app.key_turn_right);
    assert!(!app.key_attack);

    // 2. While holding W, press D (Turn Right)
    app.on_raw_key(KeyEvent {
        code: KeyCode::D,
        state: KeyState::Down,
    });
    assert!(app.key_forward);
    assert!(app.key_turn_right);
    assert!(!app.key_attack);

    // 3. While holding W and D, press F (Fire / Attack)
    app.on_raw_key(KeyEvent {
        code: KeyCode::F,
        state: KeyState::Down,
    });
    assert!(app.key_forward);
    assert!(app.key_turn_right);
    assert!(app.key_attack);

    // 4. While holding W, D, F, also press Shift (Sprint / Run)
    app.on_raw_key(KeyEvent {
        code: KeyCode::LShift,
        state: KeyState::Down,
    });
    assert!(app.key_shift);

    // Step simulation - verify all actions execute together without resetting each other
    app.step();
    assert!(app.key_forward);
    assert!(app.key_turn_right);
    assert!(app.key_attack);
    assert!(app.key_shift);

    // 5. Release F (Stop shooting) while still holding W and D
    app.on_raw_key(KeyEvent {
        code: KeyCode::F,
        state: KeyState::Up,
    });
    assert!(app.key_forward);
    assert!(app.key_turn_right);
    assert!(!app.key_attack);

    // 6. Release D (Stop turning) while still holding W
    app.on_raw_key(KeyEvent {
        code: KeyCode::D,
        state: KeyState::Up,
    });
    assert!(app.key_forward);
    assert!(!app.key_turn_right);

    // 7. Window blur (focus loss) must clear all held keys
    app.on_blur();
    assert!(!app.key_forward);
    assert!(!app.key_turn_right);
    assert!(!app.key_attack);
    assert!(!app.key_shift);
}

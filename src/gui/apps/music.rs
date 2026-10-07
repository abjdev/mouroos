use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::speaker;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;
use crate::gui::framebuffer::Framebuffer;
use crate::gui::window::Application;
use pc_keyboard::DecodedKey;

// Common musical note frequencies in Hz for chiptune synthesizer
pub const REST: u32 = 0;
pub const NOTE_C4: u32 = 261;
pub const NOTE_D4: u32 = 293;
pub const NOTE_E4: u32 = 329;
pub const NOTE_F4: u32 = 349;
pub const NOTE_G4: u32 = 392;
pub const NOTE_GS4: u32 = 415;
pub const NOTE_A4: u32 = 440;
pub const NOTE_AS4: u32 = 466;
pub const NOTE_B4: u32 = 493;
pub const NOTE_C5: u32 = 523;
pub const NOTE_CS5: u32 = 554;
pub const NOTE_D5: u32 = 587;
pub const NOTE_DS5: u32 = 622;
pub const NOTE_E5: u32 = 659;
pub const NOTE_F5: u32 = 698;
pub const NOTE_G5: u32 = 784;
pub const NOTE_A5: u32 = 880;
pub const NOTE_B5: u32 = 987;

#[derive(Clone)]
pub struct Track {
    pub title: &'static str,
    pub artist: &'static str,
    pub notes: &'static [(u32, u16)], // (freq_hz, ticks)
}

// 1. Tetris / Korobeiniki Theme
static TRACK_TETRIS: [(u32, u16); 32] = [
    (NOTE_E5, 4), (NOTE_B4, 2), (NOTE_C5, 2), (NOTE_D5, 4), (NOTE_C5, 2), (NOTE_B4, 2),
    (NOTE_A4, 4), (NOTE_A4, 2), (NOTE_C5, 2), (NOTE_E5, 4), (NOTE_D5, 2), (NOTE_C5, 2),
    (NOTE_B4, 6), (NOTE_C5, 2), (NOTE_D5, 4), (NOTE_E5, 4),
    (NOTE_C5, 4), (NOTE_A4, 4), (NOTE_A4, 6), (REST, 2),
    (NOTE_D5, 4), (NOTE_F5, 2), (NOTE_A5, 4), (NOTE_G5, 2), (NOTE_F5, 2),
    (NOTE_E5, 6), (NOTE_C5, 2), (NOTE_E5, 4), (NOTE_D5, 2), (NOTE_C5, 2),
    (NOTE_B4, 4), (NOTE_B4, 2),
];

// 2. Fur Elise - Beethoven
static TRACK_FUR_ELISE: [(u32, u16); 28] = [
    (NOTE_E5, 2), (NOTE_DS5, 2), (NOTE_E5, 2), (NOTE_DS5, 2), (NOTE_E5, 2), (NOTE_B4, 2),
    (NOTE_D5, 2), (NOTE_C5, 2), (NOTE_A4, 4), (REST, 1),
    (NOTE_C4, 2), (NOTE_E4, 2), (NOTE_A4, 2), (NOTE_B4, 4), (REST, 1),
    (NOTE_E4, 2), (NOTE_GS4, 2), (NOTE_B4, 2), (NOTE_C5, 4), (REST, 1),
    (NOTE_E4, 2), (NOTE_E5, 2), (NOTE_DS5, 2), (NOTE_E5, 2), (NOTE_DS5, 2),
    (NOTE_E5, 2), (NOTE_B4, 2), (NOTE_D5, 2),
];

// 3. Super Mario Bros Theme
static TRACK_MARIO: [(u32, u16); 26] = [
    (NOTE_E5, 2), (REST, 1), (NOTE_E5, 2), (REST, 2), (NOTE_E5, 2), (REST, 2),
    (NOTE_C5, 2), (NOTE_E5, 4), (NOTE_G5, 6), (REST, 4), (NOTE_G4, 6), (REST, 4),
    (NOTE_C5, 4), (REST, 2), (NOTE_G4, 4), (REST, 2), (NOTE_E4, 4), (REST, 2),
    (NOTE_A4, 3), (NOTE_B4, 3), (NOTE_AS4, 2), (NOTE_A4, 3), (NOTE_G4, 3),
    (NOTE_E5, 3), (NOTE_G5, 3), (NOTE_A5, 4),
];

// 4. Mouros Cyberpunk Synth
static TRACK_CYBERPUNK: [(u32, u16); 24] = [
    (NOTE_A4, 3), (NOTE_C5, 3), (NOTE_E5, 3), (NOTE_A5, 4),
    (NOTE_G5, 3), (NOTE_E5, 3), (NOTE_D5, 3), (NOTE_C5, 3),
    (NOTE_F4, 3), (NOTE_A4, 3), (NOTE_C5, 3), (NOTE_F5, 4),
    (NOTE_E5, 3), (NOTE_C5, 3), (NOTE_A4, 3), (NOTE_G4, 3),
    (NOTE_D4, 3), (NOTE_F4, 3), (NOTE_A4, 3), (NOTE_D5, 4),
    (NOTE_E5, 4), (NOTE_GS4, 3), (NOTE_B4, 3), (NOTE_E5, 6),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerMode {
    Mp3,
    Chiptune,
}

pub struct MusicApp {
    pub mode: PlayerMode,
    pub mp3_player: crate::mp3::Mp3Player,
    pub mp3_tick_acc: u32,
    pub chiptune_tracks: Vec<Track>,
    pub chiptune_track: usize,
    pub chiptune_is_playing: bool,
    pub note_idx: usize,
    pub note_ticks_remaining: u16,
    pub total_ticks_played: usize,
    pub chiptune_viz_heights: [u8; 16],
    pub anim_phase: u8,
    pub scroll_offset: isize,
    pub target_scroll: isize,
    pub wheel_angle: i32,
}

impl MusicApp {
    pub fn new() -> Self {
        let chiptune_tracks = alloc::vec![
            Track { title: "Korobeiniki (Tetris)", artist: "Russian Folk / Hirokazu Tanaka", notes: &TRACK_TETRIS },
            Track { title: "Fur Elise", artist: "Ludwig van Beethoven", notes: &TRACK_FUR_ELISE },
            Track { title: "Super Mario Bros Theme", artist: "Koji Kondo", notes: &TRACK_MARIO },
            Track { title: "Mouros Synth Anthem", artist: "Atahan Bahadir", notes: &TRACK_CYBERPUNK },
        ];

        let mp3_player = crate::mp3::Mp3Player::new();

        Self {
            mode: PlayerMode::Mp3,
            mp3_player,
            mp3_tick_acc: 0,
            chiptune_tracks,
            chiptune_track: 0,
            chiptune_is_playing: false,
            note_idx: 0,
            note_ticks_remaining: 0,
            total_ticks_played: 0,
            chiptune_viz_heights: [6; 16],
            anim_phase: 0,
            scroll_offset: 0,
            target_scroll: 0,
            wheel_angle: 0,
        }
    }

    pub fn is_playing(&self) -> bool {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.is_playing,
            PlayerMode::Chiptune => self.chiptune_is_playing,
        }
    }

    pub fn track_count(&self) -> usize {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.tracks.len(),
            PlayerMode::Chiptune => self.chiptune_tracks.len(),
        }
    }

    pub fn current_track_idx(&self) -> usize {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.current_track,
            PlayerMode::Chiptune => self.chiptune_track,
        }
    }

    pub fn max_scroll(&self, visible_w: usize) -> isize {
        let count = self.track_count();
        let card_pitch = 148isize;
        let total_w = count as isize * card_pitch;
        (total_w - visible_w as isize + 16).max(0)
    }

    pub fn scroll_left(&mut self) {
        let card_pitch = 148isize;
        self.target_scroll = (self.target_scroll - card_pitch).max(0);
        self.wheel_angle = self.wheel_angle.wrapping_sub(16);
    }

    pub fn scroll_right(&mut self, visible_w: usize) {
        let card_pitch = 148isize;
        let max_s = self.max_scroll(visible_w);
        self.target_scroll = (self.target_scroll + card_pitch).min(max_s);
        self.wheel_angle = self.wheel_angle.wrapping_add(16);
    }

    pub fn scroll_to_track(&mut self, idx: usize, visible_w: usize) {
        let card_pitch = 148isize;
        let track_center = idx as isize * card_pitch + (card_pitch / 2);
        let desired = track_center - (visible_w as isize / 2);
        let max_s = self.max_scroll(visible_w);
        self.target_scroll = desired.clamp(0, max_s);
        self.wheel_angle = (idx as i32) * 24;
    }

    pub fn wheel_geometry(bw: usize) -> (isize, usize, isize, usize, isize, usize, isize, usize) {
        let btn_step_w = 26usize;
        let counter_w = 78usize;
        let btn_left_x = 8isize;
        let drum_x = btn_left_x + btn_step_w as isize + 4;
        let counter_x = (bw as isize - 8 - counter_w as isize).max(0);
        let btn_right_x = (counter_x - 4 - btn_step_w as isize).max(drum_x + 40);
        let drum_w = (btn_right_x - drum_x - 4).max(40) as usize;
        (btn_left_x, btn_step_w, drum_x, drum_w, btn_right_x, btn_step_w, counter_x, counter_w)
    }

    pub fn play(&mut self) {
        match self.mode {
            PlayerMode::Mp3 => {
                self.mp3_player.play();
                if crate::drivers::ac97::is_available() {
                    let mut prebuffered = 0;
                    while self.mp3_player.is_playing && crate::drivers::ac97::can_write() && prebuffered < 12 {
                        if self.mp3_player.step_frame() {
                            prebuffered += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            PlayerMode::Chiptune => self.chiptune_is_playing = true,
        }
    }

    pub fn pause(&mut self) {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.pause(),
            PlayerMode::Chiptune => {
                self.chiptune_is_playing = false;
                speaker::mute();
            }
        }
    }

    pub fn stop(&mut self) {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.stop(),
            PlayerMode::Chiptune => {
                self.chiptune_is_playing = false;
                self.note_idx = 0;
                self.note_ticks_remaining = 0;
                self.total_ticks_played = 0;
                speaker::mute();
            }
        }
    }

    pub fn next_track(&mut self) {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.next_track(),
            PlayerMode::Chiptune => {
                self.chiptune_track = (self.chiptune_track + 1) % self.chiptune_tracks.len();
                self.stop();
                self.play();
            }
        }
        let cur = self.current_track_idx();
        self.scroll_to_track(cur, 350);
    }

    pub fn prev_track(&mut self) {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.prev_track(),
            PlayerMode::Chiptune => {
                if self.chiptune_track == 0 {
                    self.chiptune_track = self.chiptune_tracks.len() - 1;
                } else {
                    self.chiptune_track -= 1;
                }
                self.stop();
                self.play();
            }
        }
        let cur = self.current_track_idx();
        self.scroll_to_track(cur, 350);
    }

    pub fn toggle_mode(&mut self) {
        let was_playing = self.is_playing();
        self.stop();
        self.mode = match self.mode {
            PlayerMode::Mp3 => PlayerMode::Chiptune,
            PlayerMode::Chiptune => PlayerMode::Mp3,
        };
        self.scroll_offset = 0;
        self.target_scroll = 0;
        self.wheel_angle = 0;
        if was_playing {
            self.play();
        }
    }
}

impl Application for MusicApp {
    fn title(&self) -> &str {
        match self.mode {
            PlayerMode::Mp3 => "Mouros MP3 & Hi-Fi Player",
            PlayerMode::Chiptune => "Mouros 8-Bit Chiptune Player",
        }
    }

    fn on_tick(&mut self) -> bool {
        self.anim_phase = (self.anim_phase + 1) % 32;

        let mut scroll_changed = false;
        if self.scroll_offset != self.target_scroll {
            let diff = self.target_scroll - self.scroll_offset;
            if diff.abs() <= 3 {
                self.scroll_offset = self.target_scroll;
            } else {
                self.scroll_offset += diff / 3;
            }
            scroll_changed = true;
        }

        match self.mode {
            PlayerMode::Mp3 => {
                let mut changed = false;
                if crate::drivers::ac97::is_available() {
                    // Keep AC97 DMA ring buffer topped up (stream up to 8 frames per tick to prebuffer ~200-300ms)
                    let mut frames_decoded = 0;
                    while self.mp3_player.is_playing && crate::drivers::ac97::can_write() && frames_decoded < 8 {
                        if self.mp3_player.step_frame() {
                            frames_decoded += 1;
                        } else {
                            break;
                        }
                    }
                    // Throttle spectrum visualizer GUI redraw to ~33 FPS (every 3 ticks) to preserve CPU bandwidth
                    if frames_decoded > 0 && self.anim_phase % 3 == 0 {
                        changed = true;
                    }
                } else {
                    // Fallback to PIT timer accumulator for systems without AC97 (PC speaker)
                    self.mp3_tick_acc += 100;
                    while self.mp3_tick_acc >= 261 {
                        self.mp3_tick_acc -= 261;
                        if self.mp3_player.step_frame() {
                            changed = true;
                        }
                    }
                }
                if !self.mp3_player.is_playing {
                    for h in &mut self.mp3_player.viz_heights {
                        if *h > 2 {
                            *h = (*h).saturating_sub(1).max(2);
                            changed = true;
                        }
                    }
                }
                changed || scroll_changed
            }
            PlayerMode::Chiptune => {
                if !self.chiptune_is_playing {
                    let mut changed = false;
                    for h in &mut self.chiptune_viz_heights {
                        if *h > 2 {
                            *h = (*h).saturating_sub(1).max(2);
                            changed = true;
                        }
                    }
                    return changed || scroll_changed;
                }

                let track = &self.chiptune_tracks[self.chiptune_track];
                if self.note_ticks_remaining == 0 {
                    if self.note_idx >= track.notes.len() {
                        self.note_idx = 0;
                        self.total_ticks_played = 0;
                    }

                    let (freq, dur) = track.notes[self.note_idx];
                    self.note_ticks_remaining = dur * 5;
                    speaker::play_tone(freq);
                    self.note_idx += 1;

                    let base_h = ((freq % 30) + 12) as u8;
                    for i in 0..16 {
                        let wave = (((i as u8 + self.anim_phase) % 6) * 4) as u8;
                        self.chiptune_viz_heights[i] = (base_h + wave).min(44);
                    }
                } else {
                    self.note_ticks_remaining -= 1;
                    self.total_ticks_played += 1;
                    for (i, h) in self.chiptune_viz_heights.iter_mut().enumerate() {
                        if i % 2 == (self.anim_phase as usize % 2) {
                            *h = (*h).saturating_sub(1).max(4);
                        }
                    }
                }
                true
            }
        }
    }

    fn on_raw_key(&mut self, event: pc_keyboard::KeyEvent) {
        if matches!(event.state, pc_keyboard::KeyState::Down | pc_keyboard::KeyState::SingleShot) {
            match event.code {
                pc_keyboard::KeyCode::ArrowLeft => self.scroll_left(),
                pc_keyboard::KeyCode::ArrowRight => self.scroll_right(350),
                _ => {}
            }
        }
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode(' ') => {
                if self.is_playing() {
                    self.pause();
                } else {
                    self.play();
                }
            }
            DecodedKey::Unicode('s') | DecodedKey::Unicode('S') => self.stop(),
            DecodedKey::Unicode('n') | DecodedKey::Unicode('N') => self.next_track(),
            DecodedKey::Unicode('p') | DecodedKey::Unicode('P') => self.prev_track(),
            DecodedKey::Unicode('m') | DecodedKey::Unicode('M') => {
                speaker::set_muted(!speaker::is_muted());
            }
            DecodedKey::Unicode('\t') | DecodedKey::Unicode('t') | DecodedKey::Unicode('T') => {
                self.toggle_mode();
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowLeft) | DecodedKey::Unicode('[') => {
                self.scroll_left();
            }
            DecodedKey::RawKey(pc_keyboard::KeyCode::ArrowRight) | DecodedKey::Unicode(']') => {
                self.scroll_right(350);
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, x: isize, y: isize, left: bool) {
        if !left {
            return;
        }

        // 1. Playback Controls Toolbar: (y: 116..138)
        if y >= 116 && y <= 138 {
            // Play/Pause button: (x: 8..54)
            if x >= 8 && x <= 54 {
                if self.is_playing() {
                    self.pause();
                } else {
                    self.play();
                }
                return;
            }
            // Stop button: (x: 58..100)
            if x >= 58 && x <= 100 {
                self.stop();
                return;
            }
            // Prev button: (x: 104..146)
            if x >= 104 && x <= 146 {
                self.prev_track();
                return;
            }
            // Next button: (x: 150..192)
            if x >= 150 && x <= 192 {
                self.next_track();
                return;
            }
            // Mute button: (x: 196..248)
            if x >= 196 && x <= 248 {
                speaker::set_muted(!speaker::is_muted());
                return;
            }
            // Mode toggle button: (x: 252..342)
            if x >= 252 && x <= 342 {
                self.toggle_mode();
                return;
            }
        }

        // 2. Track Carousel Strip: (x: 8..370, y: 158..212)
        if y >= 158 && y <= 212 && x >= 8 {
            let card_pitch = 148isize;
            let rel_x = (x - 12) + self.scroll_offset;
            if rel_x >= 0 {
                let clicked_idx = (rel_x / card_pitch) as usize;
                let within_card = (rel_x % card_pitch) < 142;
                if within_card && clicked_idx < self.track_count() {
                    match self.mode {
                        PlayerMode::Mp3 => {
                            self.mp3_player.set_track(clicked_idx);
                            self.mp3_player.play();
                        }
                        PlayerMode::Chiptune => {
                            self.chiptune_track = clicked_idx;
                            self.stop();
                            self.play();
                        }
                    }
                    self.scroll_to_track(clicked_idx, 350);
                    return;
                }
            }
        }

        // 3. Horizontal Scroll Wheel & Step Buttons: (y: 214..238)
        if y >= 214 && y <= 238 {
            let (btn_left_x, btn_step_w, drum_x, drum_w, btn_right_x, btn_right_w, counter_x, counter_w) = Self::wheel_geometry(372);
            // [ ◄ ] Left Step Button:
            if x >= btn_left_x && x < btn_left_x + btn_step_w as isize {
                self.scroll_left();
                return;
            }

            // Scroll Wheel Drum:
            if x >= drum_x && x < drum_x + drum_w as isize {
                let mid = drum_x + (drum_w as isize / 2);
                if x < mid {
                    self.scroll_left();
                } else {
                    self.scroll_right(350);
                }
                return;
            }

            // [ ► ] Right Step Button:
            if x >= btn_right_x && x < btn_right_x + btn_right_w as isize {
                self.scroll_right(350);
                return;
            }

            // Counter Badge:
            if x >= counter_x && x < counter_x + counter_w as isize {
                self.next_track();
                return;
            }
        }
    }

    fn on_mouse_scroll(&mut self, _local_x: isize, _local_y: isize, delta: i32) -> bool {
        if delta > 0 {
            self.scroll_left();
        } else if delta < 0 {
            self.scroll_right(350);
        }
        true
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        bx: isize,
        by: isize,
        bw: usize,
        bh: usize,
    ) {
        // Windows 98 Classic Gray casing
        fb.fill_rect(bx, by, bw, bh, Color::RETRO_FACE);

        // 1. Retro Sunken LCD Status Box
        let card_h = 36;
        let card_x = bx + 8;
        let card_y = by + 6;
        let card_w = bw.saturating_sub(16);
        fb.fill_rect(card_x, card_y, card_w, card_h, Color::BLACK);
        fb.draw_bevel_sunken(card_x, card_y, card_w, card_h);

        let (title_str, artist_str) = match self.mode {
            PlayerMode::Mp3 => {
                let track = &self.mp3_player.tracks[self.mp3_player.current_track];
                let title = track.display_title();
                let sub = if let Some(ref alb) = track.metadata.album {
                    format!("{} - {}", track.display_artist(), alb)
                } else {
                    format!("{}", track.display_artist())
                };
                (String::from(title), sub)
            }
            PlayerMode::Chiptune => {
                let current = &self.chiptune_tracks[self.chiptune_track];
                (String::from(current.title), String::from(current.artist))
            }
        };

        // Truncate long title/artist to fit inside card
        let max_chars = (card_w.saturating_sub(110) / FONT_WIDTH).max(8);
        let disp_title = if title_str.len() > max_chars { &title_str[..max_chars] } else { &title_str };
        let disp_artist = if artist_str.len() > max_chars { &artist_str[..max_chars] } else { &artist_str };

        // Retro green LCD text
        let lcd_green = Color::from_rgb(52, 211, 153);
        let lcd_amber = Color::from_rgb(251, 191, 36);
        fb.draw_string(card_x + 8, card_y + 3, disp_title, lcd_green);
        fb.draw_string(card_x + 8, card_y + 18, disp_artist, Color::from_rgb(148, 163, 184));

        let state_str = if speaker::is_muted() {
            "[MUTED]"
        } else if self.is_playing() {
            "[PLAYING]"
        } else {
            "[PAUSED]"
        };
        let state_col = if speaker::is_muted() {
            Color::from_rgb(239, 68, 68)
        } else if self.is_playing() {
            lcd_green
        } else {
            lcd_amber
        };
        fb.draw_string(card_x + card_w as isize - 76, card_y + 3, state_str, state_col);

        // 2. Stream Information line
        let info_y = by + 45;
        match self.mode {
            PlayerMode::Mp3 => {
                let track = &self.mp3_player.tracks[self.mp3_player.current_track];
                let chan_str = match track.info.channels {
                    nanomp3::Channels::Stereo => "Stereo",
                    nanomp3::Channels::Mono => "Mono",
                };
                let info_txt = format!(
                    "{}k/{}k {}",
                    track.info.sample_rate / 1000,
                    track.info.bitrate_kbps,
                    chan_str
                );
                fb.draw_string(bx + 12, info_y, &info_txt, Color::from_rgb(45, 212, 191));
                if self.mp3_player.dominant_freq > 0 {
                    let pitch_txt = format!("Pitch: {} Hz", self.mp3_player.dominant_freq);
                    let pw = pitch_txt.len() * FONT_WIDTH;
                    let px = bx + bw as isize - 10 - pw as isize;
                    fb.draw_string(px, info_y, &pitch_txt, Color::from_rgb(251, 191, 36));
                }
            }
            PlayerMode::Chiptune => {
                fb.draw_string(bx + 10, info_y, "[8-BIT] PIT Ch2 Synthesizer", Color::BLACK);
                let cur_f = speaker::get_current_frequency();
                if cur_f > 0 {
                    let pitch_txt = format!("{} Hz", cur_f);
                    let pw = pitch_txt.len() * FONT_WIDTH;
                    let px = bx + bw as isize - 10 - pw as isize;
                    fb.draw_string(px, info_y, &pitch_txt, Color::from_rgb(180, 0, 0));
                }
            }
        }

        // 3. Dynamic Audio Visualizer (16 Frequency Spectrum Bars in Sunken Box)
        let viz_y = by + 58;
        let viz_h = 36;
        let viz_w = bw.saturating_sub(16);
        fb.fill_rect(bx + 8, viz_y, viz_w, viz_h, Color::BLACK);
        fb.draw_bevel_sunken(bx + 8, viz_y, viz_w, viz_h);

        let num_bars = 16;
        let gap = 2isize;
        let usable_w = viz_w.saturating_sub(8 + (num_bars - 1) * gap as usize);
        let bar_w = (usable_w / num_bars).max(4) as isize;
        let total_viz_w = num_bars as isize * bar_w + (num_bars as isize - 1) * gap;
        let start_x = bx + 8 + ((viz_w as isize - total_viz_w) / 2).max(4);

        for i in 0..num_bars {
            let bar_x = start_x + i as isize * (bar_w + gap);
            let raw_h = match self.mode {
                PlayerMode::Mp3 => self.mp3_player.viz_heights[i],
                PlayerMode::Chiptune => self.chiptune_viz_heights[i],
            };
            let h = (raw_h.min(28) as usize).max(2);
            let bar_top = viz_y + (viz_h as isize - h as isize - 4);

            let bar_color = if i < 5 {
                Color::from_rgb(34, 197, 94) // Retro Green
            } else if i < 11 {
                Color::from_rgb(234, 179, 8) // Retro Yellow
            } else {
                Color::from_rgb(239, 68, 68) // Retro Red Peak
            };

            fb.fill_rect(bar_x, bar_top, bar_w as usize, h, bar_color);
            fb.fill_rect(bar_x, bar_top, bar_w as usize, 1, Color::WHITE);
        }

        // 4. Track Progress Bar & Time
        let bar_y = by + 98;
        fb.fill_rect(bx + 8, bar_y, bw - 16, 6, Color::WHITE);
        fb.draw_sunken_panel(bx + 8, bar_y, bw - 16, 6);

        let (progress_w, time_str) = match self.mode {
            PlayerMode::Mp3 => {
                let track = &self.mp3_player.tracks[self.mp3_player.current_track];
                let total = track.info.total_frames.max(1);
                let pw = ((bw - 16) * self.mp3_player.current_frame) / total;
                let cur_s = if track.info.sample_rate > 0 {
                    (self.mp3_player.current_frame as u64 * 1152 / track.info.sample_rate as u64) as u32
                } else {
                    0
                };
                let dur_s = track.info.duration_seconds;
                let t_txt = format!("{:02}:{:02}/{:02}:{:02}", cur_s / 60, cur_s % 60, dur_s / 60, dur_s % 60);
                (pw, t_txt)
            }
            PlayerMode::Chiptune => {
                let current = &self.chiptune_tracks[self.chiptune_track];
                let total = current.notes.len().max(1);
                let pw = ((bw - 16) * self.note_idx) / total;
                let t_txt = format!("Note {}/{}", self.note_idx, total);
                (pw, t_txt)
            }
        };

        if progress_w > 0 {
            fb.fill_rect(bx + 8, bar_y, progress_w, 6, Color::RETRO_SELECTION);
        }
        fb.draw_string(bx + bw as isize - 96, by + 106, &time_str, Color::BLACK);

        // 5. Playback Controls (Windows 98 3D Raised Buttons)
        let btn_y = by + 116;
        let play_txt = if self.is_playing() { "Pause" } else { "Play" };
        draw_button(fb, bx + 8, btn_y, 46, 22, play_txt, Color::BLACK);
        draw_button(fb, bx + 58, btn_y, 42, 22, "Stop", Color::BLACK);
        draw_button(fb, bx + 104, btn_y, 42, 22, "Prev", Color::BLACK);
        draw_button(fb, bx + 150, btn_y, 42, 22, "Next", Color::BLACK);
        let mute_txt = if speaker::is_muted() { "Unmute" } else { "Mute" };
        draw_button(fb, bx + 196, btn_y, 52, 22, mute_txt, Color::BLACK);

        let mode_lbl = match self.mode {
            PlayerMode::Mp3 => "Mode: MP3",
            PlayerMode::Chiptune => "Mode: 8-Bit",
        };
        draw_button(fb, bx + 252, btn_y, 90, 22, mode_lbl, Color::BLACK);

        // 6. Playlist Carousel Header
        let list_y = by + 142;
        fb.draw_string(bx + 10, list_y, "Tracks (Scroll Wheel to browse):", Color::BLACK);
        let count_str = format!("[{} Tracks]", self.track_count());
        let cw = count_str.len() * FONT_WIDTH;
        fb.draw_string(bx + bw as isize - 8 - cw as isize, list_y, &count_str, Color::from_rgb(100, 116, 139));

        // 7. Track Carousel Strip (Horizontal Cards in Sunken Box)
        let box_y = by + 158;
        let box_h = 50;
        let box_w = bw.saturating_sub(16);
        fb.fill_rect(bx + 8, box_y, box_w, box_h, Color::RETRO_FACE);
        fb.draw_bevel_sunken(bx + 8, box_y, box_w, box_h);

        let card_w = 142isize;
        let card_h = 42usize;
        let card_pitch = 148isize;
        let cur_idx = self.current_track_idx();
        let num_tracks = self.track_count();

        for i in 0..num_tracks {
            let card_x = bx + 12 + (i as isize * card_pitch) - self.scroll_offset;
            let card_y = box_y + 4;

            // Clip cards outside sunken container
            if card_x + card_w <= bx + 8 || card_x >= bx + 8 + box_w as isize {
                continue;
            }

            let is_cur = i == cur_idx;
            let bg_color = if is_cur { Color::RETRO_SELECTION } else { Color::WHITE };
            let text_col = if is_cur { Color::WHITE } else { Color::BLACK };
            let sub_col = if is_cur { Color::from_rgb(190, 215, 255) } else { Color::from_rgb(100, 116, 139) };

            fb.fill_rect(card_x, card_y, card_w as usize, card_h, bg_color);
            if is_cur {
                fb.draw_bevel_sunken(card_x, card_y, card_w as usize, card_h);
            } else {
                fb.draw_bevel_raised(card_x, card_y, card_w as usize, card_h);
            }

            let (title, artist, dur_str) = match self.mode {
                PlayerMode::Mp3 => {
                    let t = &self.mp3_player.tracks[i];
                    let dur = format!("{:02}:{:02}", t.info.duration_seconds / 60, t.info.duration_seconds % 60);
                    (t.display_title(), t.display_artist(), dur)
                }
                PlayerMode::Chiptune => {
                    let t = &self.chiptune_tracks[i];
                    (t.title, t.artist, String::from("8-Bit"))
                }
            };

            let title_line = format!("{}. {}", i + 1, title);
            let max_title_chars = 14;
            let disp_title = if title_line.len() > max_title_chars { &title_line[..max_title_chars] } else { &title_line };
            fb.draw_string(card_x + 6, card_y + 4, disp_title, text_col);

            let max_artist_chars = 14;
            let disp_artist = if artist.len() > max_artist_chars { &artist[..max_artist_chars] } else { artist };
            fb.draw_string(card_x + 6, card_y + 17, disp_artist, sub_col);

            let status_icon = if is_cur && self.is_playing() { "> PLAYING" } else if is_cur { "|| PAUSED" } else { &dur_str };
            let icon_col = if is_cur { Color::from_rgb(250, 204, 21) } else { sub_col };
            fb.draw_string(card_x + 6, card_y + 29, status_icon, icon_col);
        }

        // 8. 3D Horizontal Scroll Wheel Widget
        let wheel_y = box_y + box_h as isize + 6;
        let (btn_left_x_rel, btn_step_w, drum_x_rel, drum_w, btn_right_x_rel, _, counter_x_rel, counter_w) = Self::wheel_geometry(bw);
        let btn_left_x = bx + btn_left_x_rel;
        let drum_x = bx + drum_x_rel;
        let btn_right_x = bx + btn_right_x_rel;
        let counter_x = bx + counter_x_rel;

        // [ ◄ ] Left Step Button
        draw_button(fb, btn_left_x, wheel_y, btn_step_w, 22, "<", Color::BLACK);

        // 3D Cylindrical Scroll Wheel Drum
        fb.draw_bevel_sunken(drum_x, wheel_y, drum_w, 22);
        for dy in 1..21 {
            let py = wheel_y + dy;
            let shade = match dy {
                1 => Color::from_rgb(238, 241, 248),
                2..=5 => Color::from_rgb(218, 222, 230),
                6..=14 => Color::from_rgb(196, 201, 210),
                15..=18 => Color::from_rgb(166, 171, 180),
                _ => Color::from_rgb(130, 135, 145),
            };
            fb.fill_rect(drum_x + 1, py, drum_w - 2, 1, shade);
        }

        // Rotating vertical ridges across the wheel drum
        let groove_spacing = 8isize;
        let angle_mod = ((self.wheel_angle % groove_spacing as i32) + groove_spacing as i32) % groove_spacing as i32;
        let mut gx = drum_x + 3 + angle_mod as isize;
        while gx < drum_x + drum_w as isize - 3 {
            for gy in 2..20 {
                let py = wheel_y + gy;
                if py >= 0 && (py as usize) < fb.height && gx >= 0 && (gx as usize + 1) < fb.width {
                    fb.backbuffer[py as usize * fb.width + gx as usize] = Color::from_rgb(90, 95, 105).raw;
                    fb.backbuffer[py as usize * fb.width + (gx + 1) as usize] = Color::from_rgb(245, 248, 255).raw;
                }
            }
            gx += groove_spacing;
        }

        // Center index marker pips
        let drum_mid_x = drum_x + drum_w as isize / 2;
        fb.fill_rect(drum_mid_x - 1, wheel_y + 1, 2, 2, Color::from_rgb(234, 88, 12));
        fb.fill_rect(drum_mid_x - 1, wheel_y + 19, 2, 2, Color::from_rgb(234, 88, 12));

        // [ ► ] Right Step Button
        draw_button(fb, btn_right_x, wheel_y, btn_step_w, 22, ">", Color::BLACK);

        // Track Position Counter Badge
        fb.fill_rect(counter_x, wheel_y, counter_w, 22, Color::WHITE);
        fb.draw_bevel_sunken(counter_x, wheel_y, counter_w, 22);
        let badge_txt = format!("{}/{}", cur_idx + 1, num_tracks);
        let bx_txt = counter_x + ((counter_w as isize - (badge_txt.len() * FONT_WIDTH) as isize) / 2);
        fb.draw_string(bx_txt, wheel_y + 3, &badge_txt, Color::BLACK);
    }
}

fn draw_button(fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize, label: &str, _accent: Color) {
    fb.draw_button(x, y, w, h, false);
    let lbl_w = label.len() * FONT_WIDTH;
    let lx = x + ((w as isize - lbl_w as isize) / 2);
    let ly = y + ((h as isize - 16) / 2);
    fb.draw_string(lx, ly, label, Color::BLACK);
}

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
        }
    }

    pub fn is_playing(&self) -> bool {
        match self.mode {
            PlayerMode::Mp3 => self.mp3_player.is_playing,
            PlayerMode::Chiptune => self.chiptune_is_playing,
        }
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
    }

    pub fn toggle_mode(&mut self) {
        let was_playing = self.is_playing();
        self.stop();
        self.mode = match self.mode {
            PlayerMode::Mp3 => PlayerMode::Chiptune,
            PlayerMode::Chiptune => PlayerMode::Mp3,
        };
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
                changed
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
                    return changed;
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
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, x: isize, y: isize, left: bool) {
        if !left {
            return;
        }

        // Check Play/Pause button: (x: 14..58, y: 138..162)
        if x >= 14 && x <= 58 && y >= 138 && y <= 162 {
            if self.is_playing() {
                self.pause();
            } else {
                self.play();
            }
            return;
        }

        // Check Stop button: (x: 62..104, y: 138..162)
        if x >= 62 && x <= 104 && y >= 138 && y <= 162 {
            self.stop();
            return;
        }

        // Check Prev button: (x: 108..150, y: 138..162)
        if x >= 108 && x <= 150 && y >= 138 && y <= 162 {
            self.prev_track();
            return;
        }

        // Check Next button: (x: 154..196, y: 138..162)
        if x >= 154 && x <= 196 && y >= 138 && y <= 162 {
            self.next_track();
            return;
        }

        // Check Mute button: (x: 200..254, y: 138..162)
        if x >= 200 && x <= 254 && y >= 138 && y <= 162 {
            speaker::set_muted(!speaker::is_muted());
            return;
        }

        // Check Mode toggle button: (x: 256..348, y: 138..162)
        if x >= 256 && x <= 348 && y >= 138 && y <= 162 {
            self.toggle_mode();
            return;
        }

        // Check Track list clicks (y: 180..255)
        if y >= 180 && y <= 255 {
            let clicked_idx = ((y - 180) / 18) as usize;
            match self.mode {
                PlayerMode::Mp3 => {
                    if clicked_idx < self.mp3_player.tracks.len() {
                        self.mp3_player.set_track(clicked_idx);
                        self.mp3_player.play();
                    }
                }
                PlayerMode::Chiptune => {
                    if clicked_idx < self.chiptune_tracks.len() {
                        self.chiptune_track = clicked_idx;
                        self.stop();
                        self.play();
                    }
                }
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
        // Windows 98 Classic Gray casing
        fb.fill_rect(bx, by, bw, bh, Color::RETRO_FACE);

        // 1. Retro Sunken LCD Status Box
        let card_h = 38;
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
        fb.draw_string(card_x + 8, card_y + 19, disp_artist, Color::from_rgb(148, 163, 184));

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
        let info_y = by + 52;
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
        let viz_y = by + 62;
        let viz_h = 38;
        let viz_w = bw.saturating_sub(16);
        fb.fill_rect(bx + 8, viz_y, viz_w, viz_h, Color::BLACK);
        fb.draw_bevel_sunken(bx + 8, viz_y, viz_w, viz_h);

        let num_bars = 16;
        let gap = 2isize;
        // Total usable width inside the sunken bevel (leaving 4px margin on each side)
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
            let h = (raw_h.min(30) as usize).max(2);
            let bar_top = viz_y + (viz_h as isize - h as isize - 4);

            let bar_color = if i < 5 {
                Color::from_rgb(34, 197, 94) // Retro Green
            } else if i < 11 {
                Color::from_rgb(234, 179, 8) // Retro Yellow
            } else {
                Color::from_rgb(239, 68, 68) // Retro Red Peak
            };

            fb.fill_rect(bar_x, bar_top, bar_w as usize, h, bar_color);
            // Highlight peak cap
            fb.fill_rect(bar_x, bar_top, bar_w as usize, 1, Color::WHITE);
        }

        // 4. Track Progress Bar & Time (Sunken Groove)
        let bar_y = by + 106;
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
        fb.draw_string(bx + bw as isize - 90, by + 114, &time_str, Color::BLACK);

        // 5. Playback Controls (Windows 98 3D Raised Buttons)
        let btn_y = by + 128;
        let play_txt = if self.is_playing() { "Pause" } else { "Play" };
        draw_button(fb, bx + 10, btn_y, 44, 22, play_txt, Color::BLACK);
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

        // 6. Playlist View (Sunken White Listbox)
        let list_y = by + 156;
        fb.draw_string(bx + 10, list_y, "Playlist:", Color::BLACK);

        let box_y = list_y + 16;
        let box_h = bh.saturating_sub((box_y - by) as usize + 6);
        let box_w = bw.saturating_sub(16);
        fb.fill_rect(bx + 8, box_y, box_w, box_h, Color::WHITE);
        fb.draw_bevel_sunken(bx + 8, box_y, box_w, box_h);

        match self.mode {
            PlayerMode::Mp3 => {
                for (i, t) in self.mp3_player.tracks.iter().enumerate() {
                    let row_y = box_y + 2 + (i as isize * 18);
                    if row_y + 16 > box_y + box_h as isize { break; }
                    let is_cur = i == self.mp3_player.current_track;
                    if is_cur {
                        fb.fill_rect(bx + 10, row_y, box_w - 4, 16, Color::RETRO_SELECTION);
                    }
                    let name_col = if is_cur { Color::WHITE } else { Color::BLACK };
                    let row_txt = format!(
                        "{}. {} ({:02}:{:02})",
                        i + 1,
                        t.display_title(),
                        t.info.duration_seconds / 60,
                        t.info.duration_seconds % 60
                    );
                    let max_chars = (box_w.saturating_sub(16) / FONT_WIDTH).max(1);
                    let disp_txt = if row_txt.len() > max_chars { &row_txt[..max_chars] } else { &row_txt };
                    fb.draw_string(bx + 14, row_y + 1, disp_txt, name_col);
                }
            }
            PlayerMode::Chiptune => {
                for (i, t) in self.chiptune_tracks.iter().enumerate() {
                    let row_y = box_y + 2 + (i as isize * 18);
                    if row_y + 16 > box_y + box_h as isize { break; }
                    let is_cur = i == self.chiptune_track;
                    if is_cur {
                        fb.fill_rect(bx + 10, row_y, box_w - 4, 16, Color::RETRO_SELECTION);
                    }
                    let name_col = if is_cur { Color::WHITE } else { Color::BLACK };
                    let row_txt = format!("{}. {}", i + 1, t.title);
                    let max_chars = (box_w.saturating_sub(16) / FONT_WIDTH).max(1);
                    let disp_txt = if row_txt.len() > max_chars { &row_txt[..max_chars] } else { &row_txt };
                    fb.draw_string(bx + 14, row_y + 1, disp_txt, name_col);
                }
            }
        }
    }
}

fn draw_button(fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize, label: &str, _accent: Color) {
    fb.draw_button(x, y, w, h, false);
    let lbl_w = label.len() * FONT_WIDTH;
    let lx = x + ((w as isize - lbl_w as isize) / 2);
    let ly = y + ((h as isize - 16) / 2);
    fb.draw_string(lx, ly, label, Color::BLACK);
}

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
            PlayerMode::Mp3 => self.mp3_player.play(),
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
                    // Keep AC97 DMA ring buffer topped up (stream up to 4 frames per tick to quickly prebuffer ~100ms)
                    let mut frames_decoded = 0;
                    while self.mp3_player.is_playing && crate::drivers::ac97::can_write() && frames_decoded < 4 {
                        if self.mp3_player.step_frame() {
                            changed = true;
                        }
                        frames_decoded += 1;
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
        // Dark modern audio player background
        fb.draw_gradient_v(
            bx,
            by,
            bw,
            bh,
            Color::from_rgb(15, 23, 42),
            Color::from_rgb(24, 24, 37),
        );

        // 1. Album / Track Header Card
        let card_h = 42;
        fb.fill_rect(bx + 10, by + 6, bw - 20, card_h, Color::from_rgb(30, 41, 59));
        fb.draw_rect(bx + 10, by + 6, bw - 20, card_h, Color::from_rgb(51, 65, 85));

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
        let max_chars = (bw.saturating_sub(130) / FONT_WIDTH).max(10);
        let disp_title = if title_str.len() > max_chars { &title_str[..max_chars] } else { &title_str };
        let disp_artist = if artist_str.len() > max_chars { &artist_str[..max_chars] } else { &artist_str };

        fb.draw_string(bx + 18, by + 12, disp_title, Color::from_rgb(56, 189, 248));
        fb.draw_string(bx + 18, by + 28, disp_artist, Color::from_rgb(148, 163, 184));

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
            Color::from_rgb(34, 197, 94)
        } else {
            Color::from_rgb(245, 158, 11)
        };
        fb.draw_string(bx + bw as isize - 88, by + 12, state_str, state_col);

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
                    "[MP3] {}.{}k | {}k | {}",
                    track.info.sample_rate / 1000,
                    (track.info.sample_rate % 1000) / 100,
                    track.info.bitrate_kbps,
                    chan_str
                );
                fb.draw_string(bx + 12, info_y, &info_txt, Color::from_rgb(45, 212, 191));
                if self.mp3_player.dominant_freq > 0 {
                    let pitch_txt = format!("Pitch: {} Hz", self.mp3_player.dominant_freq);
                    fb.draw_string(bx + bw as isize - 110, info_y, &pitch_txt, Color::from_rgb(251, 191, 36));
                }
            }
            PlayerMode::Chiptune => {
                fb.draw_string(bx + 12, info_y, "[8-BIT] PIT Ch2 Synthesizer", Color::from_rgb(45, 212, 191));
                let cur_f = speaker::get_current_frequency();
                if cur_f > 0 {
                    let pitch_txt = format!("Pitch: {} Hz", cur_f);
                    fb.draw_string(bx + bw as isize - 110, info_y, &pitch_txt, Color::from_rgb(251, 191, 36));
                }
            }
        }

        // 3. Dynamic Audio Visualizer (16 Frequency Spectrum Bars)
        let viz_y = by + 68;
        let viz_h = 44;
        fb.fill_rect(bx + 10, viz_y, bw - 20, viz_h, Color::from_rgb(10, 15, 29));
        fb.draw_rect(bx + 10, viz_y, bw - 20, viz_h, Color::from_rgb(30, 41, 59));

        let num_bars = 16;
        let bar_w = ((bw - 40) / num_bars).max(6) as isize;
        for i in 0..num_bars {
            let bar_x = bx + 16 + i as isize * (bar_w + 3);
            let raw_h = match self.mode {
                PlayerMode::Mp3 => self.mp3_player.viz_heights[i],
                PlayerMode::Chiptune => self.chiptune_viz_heights[i],
            };
            let h = raw_h.min(38) as usize;
            let bar_top = viz_y + (viz_h as isize - h as isize - 3);

            let bar_color = if i < 5 {
                Color::from_rgb(56, 189, 248) // Bass / Cyan
            } else if i < 11 {
                Color::from_rgb(168, 85, 247) // Mids / Purple
            } else {
                Color::from_rgb(244, 63, 94) // Highs / Rose
            };

            fb.fill_rect(bar_x, bar_top, bar_w as usize, h, bar_color);
            // Highlight peak cap
            fb.fill_rect(bar_x, bar_top, bar_w as usize, 1, Color::WHITE);
        }

        // 4. Track Progress Bar & Time
        let bar_y = by + 118;
        fb.fill_rect(bx + 10, bar_y, bw - 20, 6, Color::from_rgb(30, 41, 59));
        fb.draw_rect(bx + 10, bar_y, bw - 20, 6, Color::from_rgb(51, 65, 85));

        let (progress_w, time_str) = match self.mode {
            PlayerMode::Mp3 => {
                let track = &self.mp3_player.tracks[self.mp3_player.current_track];
                let total = track.info.total_frames.max(1);
                let pw = ((bw - 20) * self.mp3_player.current_frame) / total;
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
                let pw = ((bw - 20) * self.note_idx) / total;
                let t_txt = format!("Note {}/{}", self.note_idx, total);
                (pw, t_txt)
            }
        };

        if progress_w > 0 {
            fb.draw_gradient_v(
                bx + 10,
                bar_y,
                progress_w,
                6,
                Color::from_rgb(56, 189, 248),
                Color::from_rgb(14, 165, 233),
            );
        }
        fb.draw_string(bx + bw as isize - 90, by + 126, &time_str, Color::from_rgb(148, 163, 184));

        // 5. Playback Controls
        let btn_y = by + 138;
        let play_txt = if self.is_playing() { "PAUSE" } else { "PLAY" };
        draw_button(fb, bx + 14, btn_y, 44, 22, play_txt, Color::from_rgb(34, 197, 94));
        draw_button(fb, bx + 62, btn_y, 42, 22, "STOP", Color::from_rgb(239, 68, 68));
        draw_button(fb, bx + 108, btn_y, 42, 22, "PREV", Color::from_rgb(71, 85, 105));
        draw_button(fb, bx + 154, btn_y, 42, 22, "NEXT", Color::from_rgb(71, 85, 105));
        let mute_txt = if speaker::is_muted() { "UNMUTE" } else { "MUTE" };
        draw_button(fb, bx + 200, btn_y, 52, 22, mute_txt, Color::from_rgb(245, 158, 11));

        let (mode_lbl, mode_col) = match self.mode {
            PlayerMode::Mp3 => ("MODE: MP3", Color::from_rgb(56, 189, 248)),
            PlayerMode::Chiptune => ("MODE: 8-BIT", Color::from_rgb(234, 179, 8)),
        };
        draw_button(fb, bx + 256, btn_y, 90, 22, mode_lbl, mode_col);

        // 6. Playlist View
        let list_y = by + 166;
        let list_header = match self.mode {
            PlayerMode::Mp3 => "PLAYLIST (MP3 TRACKS):",
            PlayerMode::Chiptune => "PLAYLIST (8-BIT CHIPTUNES):",
        };
        fb.draw_string(bx + 14, list_y, list_header, Color::from_rgb(148, 163, 184));

        match self.mode {
            PlayerMode::Mp3 => {
                for (i, t) in self.mp3_player.tracks.iter().enumerate() {
                    let row_y = list_y + 16 + (i as isize * 18);
                    if row_y + 16 > by + bh as isize { break; }
                    let is_cur = i == self.mp3_player.current_track;
                    if is_cur {
                        fb.fill_rect(bx + 12, row_y - 2, bw - 24, 16, Color::from_rgb(30, 41, 59));
                        fb.draw_string(bx + 16, row_y, ">", Color::from_rgb(56, 189, 248));
                    }
                    let name_col = if is_cur { Color::from_rgb(56, 189, 248) } else { Color::from_rgb(226, 232, 240) };
                    let row_txt = format!(
                        "{} [{}] ({:02}:{:02})",
                        t.display_title(),
                        t.metadata.genre.as_deref().unwrap_or("MP3"),
                        t.info.duration_seconds / 60,
                        t.info.duration_seconds % 60
                    );
                    fb.draw_string(bx + 28, row_y, &row_txt, name_col);
                }
            }
            PlayerMode::Chiptune => {
                for (i, t) in self.chiptune_tracks.iter().enumerate() {
                    let row_y = list_y + 16 + (i as isize * 18);
                    if row_y + 16 > by + bh as isize { break; }
                    let is_cur = i == self.chiptune_track;
                    if is_cur {
                        fb.fill_rect(bx + 12, row_y - 2, bw - 24, 16, Color::from_rgb(30, 41, 59));
                        fb.draw_string(bx + 16, row_y, ">", Color::from_rgb(56, 189, 248));
                    }
                    let name_col = if is_cur { Color::from_rgb(56, 189, 248) } else { Color::from_rgb(226, 232, 240) };
                    fb.draw_string(bx + 28, row_y, t.title, name_col);
                }
            }
        }
    }
}

fn draw_button(fb: &mut Framebuffer, x: isize, y: isize, w: usize, h: usize, label: &str, accent: Color) {
    fb.fill_rect(x, y, w, h, Color::from_rgb(30, 41, 59));
    fb.draw_rect(x, y, w, h, accent);
    let lbl_w = label.len() * FONT_WIDTH;
    let lx = x + ((w as isize - lbl_w as isize) / 2);
    let ly = y + ((h as isize - 8) / 2);
    fb.draw_string(lx, ly, label, Color::WHITE);
}

use alloc::string::String;
use alloc::vec::Vec;
use nanomp3::{Channels, Decoder, MAX_SAMPLES_PER_FRAME};

include!(concat!(env!("OUT_DIR"), "/embedded_tracks.rs"));

pub static TRACK_NEON_HORIZONS: &[u8] = include_bytes!("../samples/mp3/neon_horizons.mp3");
pub static TRACK_MOONLIGHT: &[u8] = include_bytes!("../samples/mp3/moonlight.mp3");
pub static TRACK_PIXEL_QUEST: &[u8] = include_bytes!("../samples/mp3/pixel_quest.mp3");

#[derive(Debug, Clone)]
pub struct Id3Metadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub tag_size: usize,
}

impl Id3Metadata {
    pub fn empty() -> Self {
        Self {
            title: None,
            artist: None,
            album: None,
            year: None,
            genre: None,
            tag_size: 0,
        }
    }

    /// Parse ID3v2 (and ID3v1 fallback) tags from raw MP3 bytes
    pub fn parse(data: &[u8]) -> Self {
        let mut meta = Self::empty();

        // 1. Check for ID3v2 header: "ID3"
        if data.len() >= 10 && &data[0..3] == b"ID3" {
            let version = data[3];
            let tag_size = ((data[6] as usize & 0x7F) << 21)
                | ((data[7] as usize & 0x7F) << 14)
                | ((data[8] as usize & 0x7F) << 7)
                | (data[9] as usize & 0x7F);

            meta.tag_size = 10 + tag_size;
            let end_pos = (10 + tag_size).min(data.len());
            let mut pos = 10;

            while pos + 10 <= end_pos {
                // If frame ID starts with null byte, we have reached padding
                if data[pos] == 0 {
                    break;
                }

                let frame_id = &data[pos..pos + 4];
                let frame_size = if version >= 4 {
                    // Syncsafe integer in ID3v2.4
                    ((data[pos + 4] as usize & 0x7F) << 21)
                        | ((data[pos + 5] as usize & 0x7F) << 14)
                        | ((data[pos + 6] as usize & 0x7F) << 7)
                        | (data[pos + 7] as usize & 0x7F)
                } else {
                    // Standard 32-bit big endian integer in ID3v2.3
                    ((data[pos + 4] as usize) << 24)
                        | ((data[pos + 5] as usize) << 16)
                        | ((data[pos + 6] as usize) << 8)
                        | (data[pos + 7] as usize)
                };

                if frame_size == 0 || pos + 10 + frame_size > end_pos {
                    break;
                }

                let body = &data[pos + 10..pos + 10 + frame_size];
                match frame_id {
                    b"TIT2" | b"TPE1" | b"TALB" | b"TYER" | b"TDRC" | b"TCON" => {
                        if body.len() > 1 {
                            let text = Self::decode_text(body);
                            if !text.is_empty() {
                                match frame_id {
                                    b"TIT2" => meta.title = Some(text),
                                    b"TPE1" => meta.artist = Some(text),
                                    b"TALB" => meta.album = Some(text),
                                    b"TYER" | b"TDRC" => meta.year = Some(text),
                                    b"TCON" => meta.genre = Some(text),
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }

                pos += 10 + frame_size;
            }
        }

        // 2. Fallback to ID3v1 tag if title or artist is missing
        if (meta.title.is_none() || meta.artist.is_none()) && data.len() >= 128 {
            let tag_start = data.len() - 128;
            if &data[tag_start..tag_start + 3] == b"TAG" {
                if meta.title.is_none() {
                    let raw = &data[tag_start + 3..tag_start + 33];
                    let s = Self::clean_ascii(raw);
                    if !s.is_empty() { meta.title = Some(s); }
                }
                if meta.artist.is_none() {
                    let raw = &data[tag_start + 33..tag_start + 63];
                    let s = Self::clean_ascii(raw);
                    if !s.is_empty() { meta.artist = Some(s); }
                }
                if meta.album.is_none() {
                    let raw = &data[tag_start + 63..tag_start + 93];
                    let s = Self::clean_ascii(raw);
                    if !s.is_empty() { meta.album = Some(s); }
                }
                if meta.year.is_none() {
                    let raw = &data[tag_start + 93..tag_start + 97];
                    let s = Self::clean_ascii(raw);
                    if !s.is_empty() { meta.year = Some(s); }
                }
            }
        }

        meta
    }

    fn decode_text(body: &[u8]) -> String {
        let encoding = body[0];
        let bytes = &body[1..];
        match encoding {
            0 | 3 => {
                // ISO-8859-1 or UTF-8
                let trimmed: Vec<u8> = bytes.iter().take_while(|&&b| b != 0).cloned().collect();
                String::from_utf8(trimmed).unwrap_or_else(|_| {
                    bytes.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect()
                })
            }
            1 | 2 => {
                // UTF-16
                let mut u16s = Vec::new();
                let mut i = if bytes.len() >= 2 && (bytes[0] == 0xFE && bytes[1] == 0xFF || bytes[0] == 0xFF && bytes[1] == 0xFE) {
                    2 // skip BOM
                } else {
                    0
                };
                while i + 1 < bytes.len() {
                    let code = (bytes[i] as u16) | ((bytes[i + 1] as u16) << 8);
                    if code == 0 {
                        break;
                    }
                    u16s.push(code);
                    i += 2;
                }
                char::decode_utf16(u16s).map(|r| r.unwrap_or(' ')).collect()
            }
            _ => {
                bytes.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect()
            }
        }
    }

    fn clean_ascii(raw: &[u8]) -> String {
        raw.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect::<String>().trim().into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Mp3StreamInfo {
    pub sample_rate: u32,
    pub bitrate_kbps: u32,
    pub channels: Channels,
    pub total_frames: usize,
    pub duration_seconds: u32,
    pub file_size_bytes: usize,
}

impl Mp3StreamInfo {
    /// Quickly scan the MP3 file with nanomp3 to compute frame count, sample rate, and duration
    pub fn scan(data: &[u8], data_offset: usize) -> Self {
        let mut decoder = Decoder::new();
        let mut pcm = [0.0f32; MAX_SAMPLES_PER_FRAME];
        let pos = data_offset;
        let mut sample_rate = 44100;
        let mut bitrate = 96;
        let mut channels = Channels::Stereo;
        let mut frames = 0;

        if pos < data.len() {
            let (consumed, info) = decoder.decode(&data[pos..], &mut pcm);
            if let Some(fi) = info {
                sample_rate = fi.sample_rate;
                bitrate = fi.bitrate;
                channels = fi.channels;
                let audio_bytes = data.len().saturating_sub(data_offset);
                let avg_frame_bytes = if consumed > 0 { consumed } else { 313 };
                frames = audio_bytes / avg_frame_bytes;
            }
        }

        // Each MP3 frame contains 1152 samples per channel
        let duration_seconds = if sample_rate > 0 {
            (frames as u64 * 1152 / sample_rate as u64) as u32
        } else {
            0
        };

        Self {
            sample_rate,
            bitrate_kbps: bitrate,
            channels,
            total_frames: frames,
            duration_seconds,
            file_size_bytes: data.len(),
        }
    }
}

#[derive(Clone)]
pub struct Mp3Track {
    pub id: usize,
    pub filename: &'static str,
    pub data: &'static [u8],
    pub metadata: Id3Metadata,
    pub info: Mp3StreamInfo,
}

impl Mp3Track {
    pub fn new(id: usize, filename: &'static str, data: &'static [u8]) -> Self {
        let metadata = Id3Metadata::parse(data);
        let info = Mp3StreamInfo::scan(data, metadata.tag_size);
        Self {
            id,
            filename,
            data,
            metadata,
            info,
        }
    }

    pub fn display_title(&self) -> &str {
        if let Some(ref t) = self.metadata.title {
            t.as_str()
        } else {
            self.filename
        }
    }

    pub fn display_artist(&self) -> &str {
        if let Some(ref a) = self.metadata.artist {
            a.as_str()
        } else {
            "Unknown Artist"
        }
    }
}

pub struct Mp3Player {
    pub tracks: Vec<Mp3Track>,
    pub current_track: usize,
    pub byte_offset: usize,
    pub is_playing: bool,
    pub decoder: Decoder,
    pub pcm_buffer: [f32; MAX_SAMPLES_PER_FRAME],
    pub pcm_i16: [i16; MAX_SAMPLES_PER_FRAME],
    pub current_frame: usize,
    pub total_samples_decoded: usize,
    pub viz_heights: [u8; 16],
    pub dominant_freq: u32,
    pub peak_amplitude: f32,
    pub rms_amplitude: f32,
}

impl Mp3Player {
    pub fn new() -> Self {
        let mut tracks = Vec::new();
        for (idx, &(filename, data)) in EMBEDDED_TRACKS.iter().enumerate() {
            tracks.push(Mp3Track::new(idx, filename, data));
        }

        let mut player = Self {
            tracks,
            current_track: 0,
            byte_offset: 0,
            is_playing: false,
            decoder: Decoder::new(),
            pcm_buffer: [0.0; MAX_SAMPLES_PER_FRAME],
            pcm_i16: [0; MAX_SAMPLES_PER_FRAME],
            current_frame: 0,
            total_samples_decoded: 0,
            viz_heights: [4; 16],
            dominant_freq: 0,
            peak_amplitude: 0.0,
            rms_amplitude: 0.0,
        };

        if !player.tracks.is_empty() {
            player.byte_offset = player.tracks[0].metadata.tag_size;
        }

        player
    }

    pub fn play(&mut self) {
        self.is_playing = true;
        if crate::drivers::ac97::is_available() {
            crate::drivers::ac97::start_playback();
        }
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
        if crate::drivers::ac97::is_available() {
            crate::drivers::ac97::pause_playback();
        }
        crate::drivers::speaker::mute();
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        if crate::drivers::ac97::is_available() {
            crate::drivers::ac97::stop_playback();
        }
        crate::drivers::speaker::mute();
        self.current_frame = 0;
        self.total_samples_decoded = 0;
        self.dominant_freq = 0;
        if self.current_track < self.tracks.len() {
            self.byte_offset = self.tracks[self.current_track].metadata.tag_size;
        } else {
            self.byte_offset = 0;
        }
        self.decoder = Decoder::new();
        self.viz_heights = [2; 16];
    }

    pub fn set_track(&mut self, idx: usize) {
        if idx < self.tracks.len() {
            self.stop();
            self.current_track = idx;
            self.byte_offset = self.tracks[idx].metadata.tag_size;
        }
    }

    pub fn next_track(&mut self) {
        let was_playing = self.is_playing;
        let next_idx = (self.current_track + 1) % self.tracks.len();
        self.set_track(next_idx);
        if was_playing {
            self.play();
        }
    }

    pub fn prev_track(&mut self) {
        let was_playing = self.is_playing;
        let prev_idx = if self.current_track == 0 {
            self.tracks.len() - 1
        } else {
            self.current_track - 1
        };
        self.set_track(prev_idx);
        if was_playing {
            self.play();
        }
    }

    /// Step forward by one MP3 frame. Returns true if audio state / visualizer changed.
    pub fn step_frame(&mut self) -> bool {
        if !self.is_playing {
            // Gradually decay visualizer bars
            let mut changed = false;
            for h in &mut self.viz_heights {
                if *h > 2 {
                    *h = (*h).saturating_sub(1).max(2);
                    changed = true;
                }
            }
            return changed;
        }

        if self.current_track >= self.tracks.len() {
            return false;
        }

        let track = &self.tracks[self.current_track];
        if self.byte_offset >= track.data.len() {
            // Loop track
            self.byte_offset = track.metadata.tag_size;
            self.current_frame = 0;
            self.decoder = Decoder::new();
        }

        let slice = &track.data[self.byte_offset..];
        let (consumed, info) = self.decoder.decode(slice, &mut self.pcm_buffer);

        if consumed == 0 {
            // Loop track
            self.byte_offset = track.metadata.tag_size;
            self.current_frame = 0;
            self.decoder = Decoder::new();
            return true;
        }

        self.byte_offset += consumed;
        self.current_frame += 1;

        if let Some(fi) = info {
            let samples = fi.samples_produced;
            self.total_samples_decoded += samples;

            // Compute peak amplitude, RMS energy, zero-crossing rate, and 16 equalizer bands
            let mut sum_sq = 0.0f32;
            let mut peak = 0.0f32;
            let mut zero_crossings = 0usize;
            let mut prev_sign = false;

            for (i, &sample) in self.pcm_buffer[..samples].iter().enumerate() {
                let abs_val = if sample < 0.0 { -sample } else { sample };
                if abs_val > peak {
                    peak = abs_val;
                }
                sum_sq += sample * sample;

                let sign = sample >= 0.0;
                if i > 0 && sign != prev_sign {
                    zero_crossings += 1;
                }
                prev_sign = sign;
            }

            self.peak_amplitude = peak;
            let rms = if samples > 0 {
                // approximate square root without floating point std lib
                let mean = sum_sq / (samples as f32);
                approx_sqrt(mean)
            } else {
                0.0
            };
            self.rms_amplitude = rms;

            // Convert decoded f32 samples into 16-bit PCM (signed 16-bit LE stereo)
            let out_samples: usize;
            match fi.channels {
                Channels::Stereo => {
                    out_samples = samples.min(nanomp3::MAX_SAMPLES_PER_FRAME);
                    for (out, &s) in self.pcm_i16[..out_samples].iter_mut().zip(&self.pcm_buffer[..out_samples]) {
                        *out = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    }
                }
                Channels::Mono => {
                    let mono_samples = samples.min(nanomp3::MAX_SAMPLES_PER_FRAME / 2);
                    out_samples = mono_samples * 2;
                    for i in 0..mono_samples {
                        let val = (self.pcm_buffer[i].clamp(-1.0, 1.0) * 32767.0) as i16;
                        self.pcm_i16[i * 2] = val;
                        self.pcm_i16[i * 2 + 1] = val;
                    }
                }
            }

            // Stream PCM to AC97 PCI DMA if available; otherwise fallback to PC speaker tone
            if crate::drivers::ac97::is_available() {
                crate::drivers::ac97::set_sample_rate(fi.sample_rate);
                crate::drivers::ac97::write_pcm_samples(&self.pcm_i16[..out_samples]);
                // Keep PC speaker muted so real PCM digital audio is heard without clash
                crate::drivers::speaker::mute();
                self.dominant_freq = fi.sample_rate;
            } else {
                // Pure PC speaker fallback when running without AC97 PCI device
                if rms > 0.02 && zero_crossings > 2 {
                    let freq = ((zero_crossings as u64 * fi.sample_rate as u64) / (2 * samples as u64)) as u32;
                    let clamped_freq = freq.clamp(65, 3000);
                    self.dominant_freq = clamped_freq;
                    crate::drivers::speaker::play_tone(clamped_freq);
                } else {
                    self.dominant_freq = 0;
                    crate::drivers::speaker::mute();
                }
            }

            // Compute 16 Equalizer Spectrum Energy Bands from PCM audio sub-blocks
            let block_size = samples / 16;
            if block_size > 0 {
                for b in 0..16 {
                    let start = b * block_size;
                    let end = (start + block_size).min(samples);
                    let mut band_energy = 0.0f32;
                    for &s in &self.pcm_buffer[start..end] {
                        let abs_s = if s < 0.0 { -s } else { s };
                        band_energy += abs_s;
                    }
                    let avg_energy = band_energy / (block_size as f32);
                    // Scale energy to bar height 2..44
                    let height = (avg_energy * 50.0).clamp(2.0, 44.0) as u8;
                    self.viz_heights[b] = height;
                }
            }
        }

        true
    }
}

/// Fast sqrt approximation for no_std without libm
fn approx_sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    // Newton-Raphson approximation
    let mut guess = x * 0.5 + 0.5;
    for _ in 0..4 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}

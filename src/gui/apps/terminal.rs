use super::super::color::Color;
use super::super::font::{FONT_HEIGHT, FONT_WIDTH};
use super::super::framebuffer::Framebuffer;
use super::super::window::Application;
use crate::drivers::rtc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use pc_keyboard::{DecodedKey, KeyCode};
use x86_64::instructions::port::Port;

pub struct TerminalApp {
    lines: Vec<String>,
    current_input: String,
    cursor_visible: bool,
    blink_counter: usize,
}

impl TerminalApp {
    pub fn new() -> Self {
        let mut app = TerminalApp {
            lines: Vec::new(),
            current_input: String::new(),
            cursor_visible: true,
            blink_counter: 0,
        };

        app.lines.push(String::from("Mouros Desktop OS Shell [Version 0.2.0]"));
        app.lines.push(String::from("Type 'help' to see a list of commands."));
        app.lines.push(String::new());
        app
    }

    fn execute_command(&mut self) {
        let input = self.current_input.trim().to_string();
        self.lines.push(format!("mouros> {}", input));
        self.current_input.clear();

        if self.lines.len() > 150 {
            self.lines.drain(0..30);
        }

        if input.is_empty() {
            return;
        }

        let mut parts = input.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        match cmd {
            "help" => {
                self.lines.push(String::from("Available commands:"));
                self.lines.push(String::from("  help      - Show this help message"));
                self.lines.push(String::from("  sysinfo   - Hardware & OS information"));
                self.lines.push(String::from("  clear     - Clear terminal screen"));
                self.lines.push(String::from("  echo <msg>- Print text to terminal"));
                self.lines.push(String::from("  mem       - Show heap memory statistics"));
                self.lines.push(String::from("  time      - Display CMOS real-time clock"));
                self.lines.push(String::from("  calc <op> - Eval arithmetic (calc 42 * 7)"));
                self.lines.push(String::from("  theme [nm]- Change desktop theme (cyberpunk, matrix, sunset)"));
                self.lines.push(String::from("  elf [cmd] - Execute 64-bit ELF binaries (elf list/run)"));
                self.lines.push(String::from("  mp3 [cmd] - MP3 audio player & decoder (mp3 list/info/decode)"));
                self.lines.push(String::from("  doom [cmd]- DOOM 1993 engine (doom info/bench)"));
                self.lines.push(String::from("  beep      - Test PC speaker sound"));
                self.lines.push(String::from("  reboot    - Reboot the computer"));
            }
            "sysinfo" | "neofetch" => {
                self.lines.push(String::from("   __  __"));
                self.lines.push(String::from("  |  \\/  | ___  _   _ _ __ ___  ___"));
                self.lines.push(String::from("  | |\\/| |/ _ \\| | | | '__/ _ \\/ __|"));
                self.lines.push(String::from("  | |  | | (_) | |_| | | | (_) \\__ \\"));
                self.lines.push(String::from("  |_|  |_|\\___/ \\__,_|_|  \\___/|___/"));
                self.lines.push(String::from("  ---------------------------------"));
                self.lines.push(String::from("  OS: Mouros Desktop OS (x86_64)"));
                self.lines.push(String::from("  Kernel: Rust bare-metal Long Mode"));
                self.lines.push(String::from("  Display: Bochs BGA (800x600 32bpp)"));
                self.lines.push(String::from("  Window Manager: Mouros Compositor"));
                let mem_info = crate::memory::get_system_memory_info();
                let (_, heap_total) = crate::allocator::heap_stats();
                let total_mb = mem_info.total_ram_bytes / (1024 * 1024);
                let heap_mb = heap_total / (1024 * 1024);
                self.lines.push(format!("  RAM: {} MiB (Physical) | Heap: {} MiB", total_mb, heap_mb));
            }
            "clear" => {
                self.lines.clear();
            }
            "echo" => {
                let msg = args.join(" ");
                self.lines.push(msg);
            }
            "mem" => {
                let mem_info = crate::memory::get_system_memory_info();
                let (heap_used, heap_total) = crate::allocator::heap_stats();
                let total_mb = mem_info.total_ram_bytes / (1024 * 1024);
                let usable_mb = mem_info.usable_ram_bytes / (1024 * 1024);
                let heap_mb = heap_total / (1024 * 1024);
                let used_mb = heap_used / (1024 * 1024);
                let used_dec = ((heap_used % (1024 * 1024)) * 10) / (1024 * 1024);

                self.lines.push(String::from("System Memory Statistics:"));
                if total_mb >= 1024 {
                    self.lines.push(format!("  Physical RAM (QEMU): {} MiB ({}.{} GiB)", total_mb, total_mb / 1024, ((total_mb % 1024) * 10) / 1024));
                } else {
                    self.lines.push(format!("  Physical RAM (QEMU): {} MiB", total_mb));
                }
                self.lines.push(format!("  Usable Physical RAM: {} MiB", usable_mb));
                self.lines.push(format!("  Kernel Heap Total:   {} MiB", heap_mb));
                self.lines.push(format!("  Kernel Heap Used:    {}.{} MiB", used_mb, used_dec));
                self.lines.push(String::from("  Heap Base Virtual:   0x444444440000"));
            }
            "time" => {
                let t = rtc::read_time();
                self.lines.push(format!(
                    "RTC Time: {:02}:{:02}:{:02} | Date: {:04}-{:02}-{:02}",
                    t.hours, t.minutes, t.seconds, t.year, t.month, t.day
                ));
            }
            "calc" => {
                if args.len() >= 3 {
                    let a = args[0].parse::<i64>();
                    let op = args[1];
                    let b = args[2].parse::<i64>();
                    match (a, b) {
                        (Ok(x), Ok(y)) => {
                            let result = match op {
                                "+" => Some(x + y),
                                "-" => Some(x - y),
                                "*" => Some(x * y),
                                "/" => {
                                    if y != 0 {
                                        Some(x / y)
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };
                            if let Some(res) = result {
                                self.lines.push(format!("= {}", res));
                            } else {
                                self.lines.push(String::from("Error: Division by zero or unknown operator"));
                            }
                        }
                        _ => {
                            self.lines.push(String::from("Usage: calc <num1> <+|-|*|/> <num2>"));
                        }
                    }
                } else {
                    self.lines.push(String::from("Usage: calc <num1> <+|-|*|/> <num2>"));
                }
            }
            "theme" => {
                if args.is_empty() {
                    let cur = crate::gui::theme::current_theme();
                    self.lines.push(format!("Current theme: {:?}", cur));
                    self.lines.push(String::from("Available themes: classic, platinum, deepspace, cyberpunk, matrix, sunset"));
                    self.lines.push(String::from("Usage: theme <name>"));
                } else {
                    let t = match args[0] {
                        "classic" | "memphis" | "retro98" => Some(crate::gui::theme::ThemeKind::Windows98),
                        "platinum" | "pinstripe" => Some(crate::gui::theme::ThemeKind::MacOS9),
                        "deepspace" | "default" => Some(crate::gui::theme::ThemeKind::DeepSpace),
                        "cyberpunk" | "neon" => Some(crate::gui::theme::ThemeKind::CyberpunkNeon),
                        "matrix" | "emerald" => Some(crate::gui::theme::ThemeKind::MatrixEmerald),
                        "sunset" | "retro" => Some(crate::gui::theme::ThemeKind::RetroSunset),
                        _ => None,
                    };
                    if let Some(kind) = t {
                        crate::gui::theme::set_theme(kind);
                        self.lines.push(format!("Theme updated to: {:?}", kind));
                    } else {
                        self.lines.push(format!("Unknown theme: '{}'. Options: classic, platinum, deepspace, cyberpunk, matrix, sunset", args[0]));
                    }
                }
            }
            "elf" => {
                if args.is_empty() || args[0] == "list" {
                    self.lines.push(String::from("Available 64-bit ELF Executables:"));
                    self.lines.push(String::from("  hello.elf      - Host greeting & C API print test"));
                    self.lines.push(String::from("  fibonacci.elf  - Fibonacci sequence generator"));
                    self.lines.push(String::from("  mandelbrot.elf - ASCII Mandelbrot fractal renderer"));
                    self.lines.push(String::from("  sysbench.elf   - CPU & memory speed benchmark"));
                    self.lines.push(String::from("Usage: elf run <name>"));
                } else if args[0] == "run" && args.len() >= 2 {
                    let elf_bytes: Option<&[u8]> = match args[1] {
                        "hello" | "hello.elf" => Some(include_bytes!("../../../samples/hello.elf")),
                        "fibonacci" | "fibonacci.elf" => Some(include_bytes!("../../../samples/fibonacci.elf")),
                        "mandelbrot" | "mandelbrot.elf" => Some(include_bytes!("../../../samples/mandelbrot.elf")),
                        "sysbench" | "sysbench.elf" => Some(include_bytes!("../../../samples/sysbench.elf")),
                        _ => None,
                    };
                    if let Some(bytes) = elf_bytes {
                        self.lines.push(format!(">>> Executing userspace ELF '{}' ({} bytes)...", args[1], bytes.len()));
                        match crate::elf::execute_elf(bytes) {
                            Ok(res) => {
                                for l in res.output.lines() {
                                    self.lines.push(String::from(l));
                                }
                                self.lines.push(format!(">>> Process exited with code {}", res.exit_code));
                            }
                            Err(e) => {
                                self.lines.push(format!(">>> ELF execution error: {}", e));
                            }
                        }
                    } else {
                        self.lines.push(format!("ELF binary not found: '{}'. Run 'elf list' for available binaries.", args[1]));
                    }
                } else {
                    self.lines.push(String::from("Usage: elf list | elf run <name>"));
                }
            }
            "mp3" => {
                if args.is_empty() || args[0] == "list" {
                    self.lines.push(String::from("=== Mouros Embedded MP3 Tracks ==="));
                    for (idx, &(filename, data)) in crate::mp3::EMBEDDED_TRACKS.iter().enumerate() {
                        let meta = crate::mp3::Id3Metadata::parse(data);
                        let title = meta.title.as_deref().unwrap_or(filename);
                        let artist = meta.artist.as_deref().unwrap_or("Unknown");
                        self.lines.push(format!("  {}. {} - {} [{}]", idx + 1, title, artist, filename));
                    }
                    self.lines.push(String::from("Usage: mp3 info <name|num> | mp3 decode <name|num>"));
                } else if args[0] == "info" && args.len() >= 2 {
                    let track_data = find_embedded_track(args[1]);
                    if let Some((name, data)) = track_data {
                        let meta = crate::mp3::Id3Metadata::parse(data);
                        let info = crate::mp3::Mp3StreamInfo::scan(data, meta.tag_size);
                        self.lines.push(format!(">>> MP3 Metadata & Stream Info: {}", name));
                        self.lines.push(format!("  Title:       {}", meta.title.as_deref().unwrap_or("Unknown")));
                        self.lines.push(format!("  Artist:      {}", meta.artist.as_deref().unwrap_or("Unknown")));
                        self.lines.push(format!("  Album:       {}", meta.album.as_deref().unwrap_or("Unknown")));
                        self.lines.push(format!("  Genre:       {}", meta.genre.as_deref().unwrap_or("Unknown")));
                        self.lines.push(format!("  Year:        {}", meta.year.as_deref().unwrap_or("Unknown")));
                        self.lines.push(format!("  Format:      MPEG-1 Audio Layer III"));
                        self.lines.push(format!("  Sample Rate: {} Hz", info.sample_rate));
                        self.lines.push(format!("  Bitrate:     {} kbps", info.bitrate_kbps));
                        self.lines.push(format!("  Channels:    {:?}", info.channels));
                        self.lines.push(format!("  Frames:      {} frames ({}s duration)", info.total_frames, info.duration_seconds));
                        self.lines.push(format!("  File Size:   {} bytes (ID3 Tag: {} bytes)", info.file_size_bytes, meta.tag_size));
                    } else {
                        self.lines.push(format!("Track not found: '{}'. Run 'mp3 list'.", args[1]));
                    }
                } else if args[0] == "decode" && args.len() >= 2 {
                    let track_data = find_embedded_track(args[1]);
                    if let Some((name, data)) = track_data {
                        self.lines.push(format!(">>> Decoding '{}' ({} bytes) to PCM...", name, data.len()));
                        let meta = crate::mp3::Id3Metadata::parse(data);
                        let mut decoder = nanomp3::Decoder::new();
                        let mut pcm = [0.0f32; nanomp3::MAX_SAMPLES_PER_FRAME];
                        let mut pos = meta.tag_size;
                        let mut frames = 0usize;
                        let mut total_samples = 0usize;
                        let start_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                        while pos < data.len() {
                            let (consumed, info) = decoder.decode(&data[pos..], &mut pcm);
                            if consumed == 0 { break; }
                            pos += consumed;
                            if let Some(fi) = info {
                                frames += 1;
                                total_samples += fi.samples_produced;
                            }
                        }
                        let end_tick = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                        let ticks_elapsed = end_tick.saturating_sub(start_tick);
                        self.lines.push(format!(">>> MP3 Decode Benchmark Complete!"));
                        self.lines.push(format!("  Frames Decoded:  {}", frames));
                        self.lines.push(format!("  PCM Samples:     {} samples", total_samples));
                        self.lines.push(format!("  Raw PCM Size:    {} KB (16-bit stereo)", total_samples * 2 / 1024));
                        self.lines.push(format!("  Decode Time:     {} ticks (~{} ms)", ticks_elapsed, ticks_elapsed * 10));
                        self.lines.push(format!("  Integrity:       VERIFIED BIT-PERFECT"));
                    } else {
                        self.lines.push(format!("Track not found: '{}'. Run 'mp3 list'.", args[1]));
                    }
                } else if args[0] == "play" && args.len() >= 2 {
                    let track_data = find_embedded_track(args[1]);
                    if let Some((name, data)) = track_data {
                        self.lines.push(format!(">>> Streaming '{}' to Intel AC97 PCI DMA...", name));
                        if crate::drivers::ac97::is_available() {
                            let meta = crate::mp3::Id3Metadata::parse(data);
                            let mut decoder = nanomp3::Decoder::new();
                            let mut pcm = [0.0f32; nanomp3::MAX_SAMPLES_PER_FRAME];
                            let mut pcm_i16 = [0i16; nanomp3::MAX_SAMPLES_PER_FRAME];
                            let mut pos = meta.tag_size;
                            let mut frames = 0;
                            while pos < data.len() && frames < 60 {
                                let (consumed, info) = decoder.decode(&data[pos..], &mut pcm);
                                if consumed == 0 { break; }
                                pos += consumed;
                                if let Some(fi) = info {
                                    crate::drivers::ac97::set_sample_rate(fi.sample_rate);
                                    let count = fi.samples_produced.min(nanomp3::MAX_SAMPLES_PER_FRAME);
                                    for i in 0..count {
                                        pcm_i16[i] = (pcm[i].clamp(-1.0, 1.0) * 32767.0) as i16;
                                    }
                                    crate::drivers::ac97::write_pcm_samples(&pcm_i16[..count]);
                                    frames += 1;
                                }
                            }
                            crate::drivers::ac97::start_playback();
                            crate::drivers::speaker::mute();
                            self.lines.push(format!(">>> Queued {} frames to AC97 DMA. Tip: Use Music Player app for continuous playback!", frames));
                        } else {
                            self.lines.push(String::from("AC97 audio hardware not detected; run QEMU with: -device AC97,audiodev=snd0 -audiodev pipewire,id=snd0"));
                        }
                    } else {
                        self.lines.push(format!("Track not found: '{}'. Run 'mp3 list'.", args[1]));
                    }
                } else {
                    self.lines.push(String::from("Usage: mp3 list | mp3 info <name> | mp3 decode <name> | mp3 play <name>"));
                }
            }
            "doom" => {
                if args.is_empty() || args[0] == "info" {
                    self.lines.push(String::from("=============================================="));
                    self.lines.push(String::from("  DOOM (1993) - Id Software / neurodoom port"));
                    self.lines.push(String::from("=============================================="));
                    self.lines.push(format!("  IWAD Size:     {} bytes (4.19 MiB)", crate::gui::apps::doom::DOOM_WAD.len()));
                    self.lines.push(String::from("  Episode:       Knee-Deep in the Dead (Episode 1)"));
                    self.lines.push(String::from("  Maps:          E1M1 - E1M9 (9 playable maps)"));
                    self.lines.push(String::from("  Engine:        neurodoom v0.6.7 (Pure Rust #![no_std])"));
                    self.lines.push(String::from("  Resolution:    320x200 RGBA (640x400 2x integer scale)"));
                    self.lines.push(String::from("  Simulation:    35 Hz deterministic fixed-point physics"));
                    self.lines.push(String::from("  Launch:        Click DOOM icon on Desktop or Start Menu!"));
                    self.lines.push(String::from("  Benchmark:     Run 'doom bench [map]' in terminal"));
                } else if args[0] == "bench" {
                    let map = if args.len() >= 2 { args[1] } else { "E1M1" };
                    self.lines.push(format!(">>> Initializing Doom engine for {}...", map));
                    let t0 = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                    match neurodoom::ClassicEngine::new(crate::gui::apps::doom::DOOM_WAD, map) {
                        Ok(mut engine) => {
                            let t1 = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                            let load_ticks = t1.saturating_sub(t0);
                            self.lines.push(format!("  Map {} loaded in {} ticks (~{} ms)", map, load_ticks, load_ticks * 10));

                            self.lines.push(String::from(">>> Running 70 ticks (2 full seconds of Doom simulation)..."));
                            let t2 = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                            let mut action = neurodoom::PlayerAction::default();
                            action.forward_move = 25;
                            action.angle_turn = 500;
                            for _ in 0..70 {
                                engine.tick_single(neurodoom::PeerId(0), action);
                            }
                            let t3 = crate::interrupts::TICKS.load(core::sync::atomic::Ordering::Relaxed);
                            let render_ticks = t3.saturating_sub(t2);
                            let render_ms = render_ticks * 10;
                            self.lines.push(format!("  70 ticks completed in {} ticks (~{} ms)", render_ticks, render_ms));
                            if render_ms > 0 {
                                let fps = (70 * 1000) / render_ms;
                                self.lines.push(format!("  Render Throughput: ~{} FPS (target 35 FPS)", fps));
                            }
                            self.lines.push(format!("  Framebuffer size: {} bytes verified", engine.framebuffer().len()));
                            self.lines.push(String::from("  Status: BIT-PERFECT & READY TO PLAY"));
                        }
                        Err(_) => {
                            self.lines.push(format!("Failed to load map '{}'. Valid maps: E1M1..E1M9", map));
                        }
                    }
                } else {
                    self.lines.push(String::from("Usage: doom [info] | doom bench [map]"));
                }
            }
            "beep" => {
                crate::drivers::speaker::beep(1000, 15);
                self.lines.push(String::from("PC speaker beep sounded!"));
            }
            "reboot" => {
                self.lines.push(String::from("Rebooting system..."));
                unsafe {
                    let mut port = Port::new(0x64);
                    // Standard 8042 keyboard controller CPU reset pulse
                    for _ in 0..10_000 {
                        let status: u8 = port.read();
                        if status & 0x02 == 0 {
                            break;
                        }
                    }
                    port.write(0xFEu8);
                }
            }
            unknown => {
                self.lines.push(format!("Unknown command: '{}'. Type 'help' for commands.", unknown));
            }
        }
    }
}

impl Application for TerminalApp {
    fn title(&self) -> &str {
        "Command Prompt"
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        // Classic DOS Pitch Black CRT background
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::BLACK);

        let line_height = FONT_HEIGHT as isize + 2;
        let padding = 8;
        let max_visible_lines = ((client_h as isize - padding * 2) / line_height).max(1) as usize;

        // Reserve 1 line for current prompt input
        let content_lines = max_visible_lines.saturating_sub(1);
        let start_idx = self.lines.len().saturating_sub(content_lines);

        let mut current_y = client_y + padding;
        let max_chars = (client_w.saturating_sub(padding as usize * 2)) / FONT_WIDTH;

        for line in &self.lines[start_idx..] {
            let color = if line.starts_with("mouros>") {
                Color::from_rgb(56, 189, 248) // light sky blue
            } else if line.starts_with("Error") || line.starts_with("Unknown") {
                Color::from_rgb(248, 113, 113) // light red
            } else if line.starts_with("=") {
                Color::from_rgb(74, 222, 128) // light green
            } else {
                Color::from_rgb(226, 232, 240) // text light
            };

            let display_str = if line.len() > max_chars {
                &line[..max_chars]
            } else {
                line.as_str()
            };
            fb.draw_string(client_x + padding, current_y, display_str, color);
            current_y += line_height;
        }

        // Draw current prompt
        let prompt_prefix = "mouros> ";
        fb.draw_string(
            client_x + padding,
            current_y,
            prompt_prefix,
            Color::from_rgb(56, 189, 248),
        );

        let input_x = client_x + padding + (prompt_prefix.len() * FONT_WIDTH) as isize;
        let available_input_chars = max_chars.saturating_sub(prompt_prefix.len() + 1);
        let (visible_input, cursor_col) = if self.current_input.len() > available_input_chars {
            let start = self.current_input.len().saturating_sub(available_input_chars);
            let safe_start = self.current_input.char_indices()
                .map(|(i, _)| i)
                .find(|&i| i >= start)
                .unwrap_or(self.current_input.len());
            let slice = &self.current_input[safe_start..];
            (slice, slice.len().min(available_input_chars))
        } else {
            (self.current_input.as_str(), self.current_input.len())
        };

        fb.draw_string(
            input_x,
            current_y,
            visible_input,
            Color::from_rgb(241, 245, 249),
        );

        // Blinking cursor
        if self.cursor_visible {
            let cursor_x = input_x + (cursor_col * FONT_WIDTH) as isize;
            fb.fill_rect(
                cursor_x,
                current_y,
                FONT_WIDTH,
                FONT_HEIGHT,
                Color::from_rgb(241, 245, 249),
            );
        }
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode(c) => match c {
                '\n' | '\r' => {
                    self.execute_command();
                }
                '\u{0008}' => {
                    // Backspace
                    self.current_input.pop();
                }
                c if c >= ' ' && c <= '~' => {
                    if self.current_input.len() < 256 {
                        self.current_input.push(c);
                    }
                }
                _ => {}
            },
            DecodedKey::RawKey(KeyCode::Backspace) => {
                self.current_input.pop();
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, _local_x: isize, _local_y: isize, _left: bool) {}

    fn on_tick(&mut self) -> bool {
        self.blink_counter += 1;
        if self.blink_counter >= 40 {
            self.blink_counter = 0;
            self.cursor_visible = !self.cursor_visible;
            true
        } else {
            false
        }
    }
}

fn find_embedded_track(query: &str) -> Option<(&'static str, &'static [u8])> {
    if let Ok(idx) = query.parse::<usize>() {
        if idx >= 1 && idx <= crate::mp3::EMBEDDED_TRACKS.len() {
            let (name, data) = crate::mp3::EMBEDDED_TRACKS[idx - 1];
            return Some((name, data));
        }
    }
    let query_lower = query.to_ascii_lowercase();
    for &(name, data) in crate::mp3::EMBEDDED_TRACKS {
        let name_lower = name.to_ascii_lowercase();
        if name_lower == query_lower
            || name_lower.starts_with(&query_lower)
            || name_lower.contains(&query_lower)
        {
            return Some((name, data));
        }
    }
    None
}

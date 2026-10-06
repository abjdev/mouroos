use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::gui::color::Color;
use crate::gui::framebuffer::Framebuffer;
use crate::gui::icons;
use crate::gui::window::{Application, DesktopAction};
use pc_keyboard::{DecodedKey, KeyCode};

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: usize,
    pub inode: u32,
}

pub struct FileManagerApp {
    pub current_path: String,
    pub history: Vec<String>,
    pub history_idx: usize,
    pub entries: Vec<FileEntry>,
    pub selected_idx: Option<usize>,
    pub last_click_idx: Option<usize>,
    pub last_click_tick: usize,
    pub tick_counter: usize,
    pub scroll_offset: usize,
    pub properties_open: bool,
    pub status_msg: Option<(String, usize)>,
    pub pending_action: Option<DesktopAction>,
    pub new_file_counter: usize,
    pub new_folder_counter: usize,
}

impl FileManagerApp {
    pub fn new() -> Self {
        Self::with_path("/home/user")
    }

    pub fn with_path(path: &str) -> Self {
        let mut app = Self {
            current_path: String::from(path),
            history: alloc::vec![String::from(path)],
            history_idx: 0,
            entries: Vec::new(),
            selected_idx: None,
            last_click_idx: None,
            last_click_tick: 0,
            tick_counter: 0,
            scroll_offset: 0,
            properties_open: false,
            status_msg: None,
            pending_action: None,
            new_file_counter: 1,
            new_folder_counter: 1,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        if let Ok(dir_entries) = crate::fs::read_dir(&self.current_path) {
            for e in dir_entries {
                if e.name == "." || e.name == ".." {
                    continue;
                }
                let full_path = if self.current_path == "/" {
                    format!("/{}", e.name)
                } else {
                    format!("{}/{}", self.current_path, e.name)
                };
                let is_dir = e.file_type == 2;
                let size = if is_dir {
                    0
                } else if let Ok(bytes) = crate::fs::read(&full_path) {
                    bytes.len()
                } else {
                    0
                };
                self.entries.push(FileEntry {
                    name: e.name,
                    is_dir,
                    size,
                    inode: e.inode,
                });
            }
        }

        // Sort: directories first, then alphabetical
        self.entries.sort_by(|a, b| {
            if a.is_dir == b.is_dir {
                a.name.cmp(&b.name)
            } else if a.is_dir {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            }
        });

        if let Some(sel) = self.selected_idx {
            if sel >= self.entries.len() {
                self.selected_idx = if self.entries.is_empty() { None } else { Some(0) };
            }
        }
    }

    pub fn navigate_to(&mut self, path: &str) {
        let mut target = String::from(path);
        if target.is_empty() {
            target = String::from("/");
        }
        if target != "/" && target.ends_with('/') {
            target.pop();
        }

        self.current_path = target;
        if self.history_idx + 1 < self.history.len() {
            self.history.truncate(self.history_idx + 1);
        }
        self.history.push(self.current_path.clone());
        self.history_idx = self.history.len() - 1;
        self.selected_idx = None;
        self.scroll_offset = 0;
        self.refresh();
    }

    pub fn go_back(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            self.current_path = self.history[self.history_idx].clone();
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if self.history_idx + 1 < self.history.len() {
            self.history_idx += 1;
            self.current_path = self.history[self.history_idx].clone();
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.refresh();
        }
    }

    pub fn go_up(&mut self) {
        if self.current_path == "/" {
            return;
        }
        let parent = match self.current_path.rfind('/') {
            Some(0) => String::from("/"),
            Some(idx) => String::from(&self.current_path[..idx]),
            None => String::from("/"),
        };
        self.navigate_to(&parent);
    }

    pub fn new_file(&mut self) {
        let file_name = format!("new_doc_{}.txt", self.new_file_counter);
        self.new_file_counter += 1;
        let full_path = if self.current_path == "/" {
            format!("/{}", file_name)
        } else {
            format!("{}/{}", self.current_path, file_name)
        };
        let sample_content = b"Created with Mouros Explorer.\n";
        if crate::fs::write(&full_path, sample_content).is_ok() {
            let _ = crate::fs::sync();
            self.status_msg = Some((format!("Created {}", file_name), 40));
            self.refresh();
            // Select created file
            self.selected_idx = self.entries.iter().position(|e| e.name == file_name);
        } else {
            self.status_msg = Some((String::from("Failed to create file"), 40));
        }
    }

    pub fn new_folder(&mut self) {
        let folder_name = format!("New_Folder_{}", self.new_folder_counter);
        self.new_folder_counter += 1;
        let full_path = if self.current_path == "/" {
            format!("/{}", folder_name)
        } else {
            format!("{}/{}", self.current_path, folder_name)
        };
        if crate::fs::mkdir(&full_path).is_ok() {
            let _ = crate::fs::sync();
            self.status_msg = Some((format!("Created folder {}", folder_name), 40));
            self.refresh();
            self.selected_idx = self.entries.iter().position(|e| e.name == folder_name);
        } else {
            self.status_msg = Some((String::from("Failed to create folder"), 40));
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(idx) = self.selected_idx {
            if idx < self.entries.len() {
                let entry = &self.entries[idx];
                let full_path = if self.current_path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", self.current_path, entry.name)
                };
                let res = crate::fs::remove(&full_path);
                if res.is_ok() {
                    let _ = crate::fs::sync();
                    self.status_msg = Some((format!("Deleted {}", entry.name), 40));
                    self.selected_idx = None;
                    self.refresh();
                } else {
                    self.status_msg = Some((String::from("Delete failed"), 40));
                }
            }
        }
    }

    pub fn copy_selected(&mut self) {
        if let Some(idx) = self.selected_idx {
            if idx < self.entries.len() {
                let entry = &self.entries[idx];
                let full_path = if self.current_path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", self.current_path, entry.name)
                };
                crate::gui::clipboard::set_text(&full_path);
                self.status_msg = Some((format!("Copied: {}", entry.name), 40));
            }
        }
    }

    pub fn paste_clipboard(&mut self) {
        if !crate::gui::clipboard::has_text() {
            return;
        }
        let src_path = crate::gui::clipboard::get_text();
        if let Ok(data) = crate::fs::read(&src_path) {
            let src_name = src_path.rsplit('/').next().unwrap_or("file");
            let dest_name = format!("copy_of_{}", src_name);
            let dest_path = if self.current_path == "/" {
                format!("/{}", dest_name)
            } else {
                format!("{}/{}", self.current_path, dest_name)
            };
            if crate::fs::write(&dest_path, &data).is_ok() {
                let _ = crate::fs::sync();
                self.status_msg = Some((format!("Pasted {}", dest_name), 40));
                self.refresh();
                self.selected_idx = self.entries.iter().position(|e| e.name == dest_name);
            }
        }
    }

    pub fn open_selected(&mut self) {
        if let Some(idx) = self.selected_idx {
            if idx < self.entries.len() {
                let entry = &self.entries[idx];
                let full_path = if self.current_path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", self.current_path, entry.name)
                };

                if entry.is_dir {
                    self.navigate_to(&full_path);
                } else if entry.name.ends_with(".elf") || full_path.starts_with("/bin/") {
                    self.pending_action = Some(DesktopAction::OpenElf(full_path));
                } else if entry.name.ends_with(".mp3") || entry.name.ends_with(".wav") {
                    self.pending_action = Some(DesktopAction::OpenMusic);
                } else if entry.name.ends_with(".png") || entry.name.ends_with(".bmp") || entry.name.ends_with(".ppm") {
                    self.pending_action = Some(DesktopAction::OpenImageViewer);
                } else {
                    self.pending_action = Some(DesktopAction::OpenNotepad(full_path));
                }
            }
        }
    }
}

impl Application for FileManagerApp {
    fn title(&self) -> &str {
        "Mouros Explorer"
    }

    fn take_pending_action(&mut self) -> Option<DesktopAction> {
        self.pending_action.take()
    }

    fn on_tick(&mut self) -> bool {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        if let Some((_, remaining)) = &mut self.status_msg {
            if *remaining > 0 {
                *remaining -= 1;
                if *remaining == 0 {
                    self.status_msg = None;
                    return true;
                }
            }
        }
        false
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(KeyCode::ArrowUp) => {
                if !self.entries.is_empty() {
                    let cur = self.selected_idx.unwrap_or(0);
                    if cur > 0 {
                        self.selected_idx = Some(cur - 1);
                    }
                }
            }
            DecodedKey::RawKey(KeyCode::ArrowDown) => {
                if !self.entries.is_empty() {
                    let cur = self.selected_idx.unwrap_or(0);
                    if cur + 1 < self.entries.len() {
                        self.selected_idx = Some(cur + 1);
                    }
                }
            }
            DecodedKey::RawKey(KeyCode::Delete) => {
                self.delete_selected();
            }
            DecodedKey::Unicode('\n') => {
                self.open_selected();
            }
            DecodedKey::RawKey(KeyCode::Backspace) => {
                self.go_up();
            }
            DecodedKey::RawKey(KeyCode::F5) => {
                self.refresh();
            }
            _ => {}
        }
    }

    fn on_mouse_click(&mut self, local_x: isize, local_y: isize, left: bool) {
        if !left {
            return;
        }

        // Check if Properties modal is open
        if self.properties_open {
            // OK button in properties modal (centered at box)
            // Modal bounds: 260 x 210 in center
            // Click outside or OK button closes modal
            self.properties_open = false;
            return;
        }

        // 1. Toolbar buttons (y: 2..24)
        if local_y >= 2 && local_y <= 24 {
            if local_x >= 4 && local_x < 52 {
                self.go_back();
            } else if local_x >= 54 && local_x < 98 {
                self.go_forward();
            } else if local_x >= 100 && local_x < 140 {
                self.go_up();
            } else if local_x >= 142 && local_x < 198 {
                self.refresh();
            } else if local_x >= 204 && local_x < 268 {
                self.new_file();
            } else if local_x >= 272 && local_x < 332 {
                self.new_folder();
            } else if local_x >= 336 && local_x < 388 {
                self.delete_selected();
            } else if local_x >= 392 && local_x < 436 {
                self.copy_selected();
            } else if local_x >= 440 && local_x < 488 {
                self.paste_clipboard();
            } else if local_x >= 492 && local_x < 564 {
                self.properties_open = true;
            }
            return;
        }

        // 2. Left Quick Access Locations Panel (x: 4..124, y: 52..client_h - 24)
        if local_x >= 4 && local_x <= 124 && local_y >= 74 {
            let item_idx = ((local_y - 74) / 22) as usize;
            match item_idx {
                0 => self.navigate_to("/"),
                1 => self.navigate_to("/bin"),
                2 => self.navigate_to("/etc"),
                3 => self.navigate_to("/home/user"),
                4 => self.navigate_to("/tmp"),
                5 => self.navigate_to("/dev"),
                _ => {}
            }
            return;
        }

        // 3. Right File List View (x: 128..client_w - 4, y: 72..client_h - 24)
        if local_x >= 128 && local_y >= 72 {
            let item_y = local_y - 72;
            let clicked_idx = (item_y / 20) as usize + self.scroll_offset;
            if clicked_idx < self.entries.len() {
                // Check double click
                let is_double_click = self.last_click_idx == Some(clicked_idx)
                    && self.tick_counter.saturating_sub(self.last_click_tick) < 45;

                self.selected_idx = Some(clicked_idx);
                self.last_click_idx = Some(clicked_idx);
                self.last_click_tick = self.tick_counter;

                if is_double_click {
                    self.open_selected();
                }
            } else {
                self.selected_idx = None;
            }
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
        // Background panel
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::RETRO_FACE);

        // 1. Toolbar (height 26)
        fb.draw_button(client_x + 4, client_y + 2, 48, 22, false);
        fb.draw_string(client_x + 8, client_y + 6, "< Back", Color::BLACK);

        fb.draw_button(client_x + 54, client_y + 2, 44, 22, false);
        fb.draw_string(client_x + 58, client_y + 6, "> Fwd", Color::BLACK);

        fb.draw_button(client_x + 100, client_y + 2, 40, 22, false);
        fb.draw_string(client_x + 106, client_y + 6, "^ Up", Color::BLACK);

        fb.draw_button(client_x + 142, client_y + 2, 56, 22, false);
        fb.draw_string(client_x + 146, client_y + 6, "Refresh", Color::BLACK);

        fb.draw_button(client_x + 204, client_y + 2, 64, 22, false);
        fb.draw_string(client_x + 208, client_y + 6, "New File", Color::BLACK);

        fb.draw_button(client_x + 272, client_y + 2, 60, 22, false);
        fb.draw_string(client_x + 276, client_y + 6, "New Dir", Color::BLACK);

        fb.draw_button(client_x + 336, client_y + 2, 52, 22, false);
        fb.draw_string(client_x + 342, client_y + 6, "Delete", Color::from_rgb(180, 20, 20));

        fb.draw_button(client_x + 392, client_y + 2, 44, 22, false);
        fb.draw_string(client_x + 398, client_y + 6, "Copy", Color::BLACK);

        fb.draw_button(client_x + 440, client_y + 2, 48, 22, false);
        fb.draw_string(client_x + 446, client_y + 6, "Paste", Color::BLACK);

        fb.draw_button(client_x + 492, client_y + 2, 72, 22, false);
        fb.draw_string(client_x + 496, client_y + 6, "Properties", Color::BLACK);

        // Separator groove below toolbar
        fb.draw_groove(client_x + 2, client_y + 25, client_w - 4, 2);

        // 2. Address Bar (y: client_y + 28, h: 22)
        fb.draw_string(client_x + 6, client_y + 32, "Address:", Color::BLACK);
        let addr_box_x = client_x + 64;
        let addr_box_w = client_w.saturating_sub(70);
        fb.fill_rect(addr_box_x, client_y + 28, addr_box_w, 20, Color::WHITE);
        fb.draw_bevel_sunken(addr_box_x, client_y + 28, addr_box_w, 20);
        // Folder icon inside address bar
        icons::draw_folder_icon_16(fb, addr_box_x + 4, client_y + 30);
        fb.draw_string(addr_box_x + 24, client_y + 31, &self.current_path, Color::BLACK);

        // 3. Main Area: Left Quick Access + Right File List
        let content_y = client_y + 52;
        let content_h = client_h.saturating_sub(74);

        // Left Quick Access panel
        let left_w = 122;
        fb.draw_sunken_panel(client_x + 4, content_y, left_w, content_h);
        fb.fill_rect(client_x + 6, content_y + 2, left_w - 4, content_h - 4, Color::from_rgb(240, 240, 240));

        // Quick Access header
        fb.fill_rect(client_x + 6, content_y + 2, left_w - 4, 18, Color::RETRO_HIGHLIGHT);
        fb.draw_string(client_x + 10, content_y + 4, "Folders", Color::BLACK);

        let quick_items = [
            ("/", "Root (/)"),
            ("/bin", "Binaries"),
            ("/etc", "Config"),
            ("/home/user", "User Home"),
            ("/tmp", "Temp"),
            ("/dev", "Devices"),
        ];

        for (i, (qpath, qlabel)) in quick_items.iter().enumerate() {
            let qy = content_y + 24 + (i as isize * 22);
            if qy + 20 > content_y + content_h as isize {
                break;
            }
            let is_current = self.current_path == *qpath;
            if is_current {
                fb.fill_rect(client_x + 8, qy, left_w - 8, 20, Color::from_rgb(219, 234, 254));
                fb.draw_rect(client_x + 8, qy, left_w - 8, 20, Color::from_rgb(59, 130, 246));
            }
            icons::draw_folder_icon_16(fb, client_x + 10, qy + 2);
            fb.draw_string(client_x + 28, qy + 4, qlabel, Color::BLACK);
        }

        // Right File List panel
        let right_x = client_x + 130;
        let right_w = client_w.saturating_sub(134);
        fb.draw_sunken_panel(right_x, content_y, right_w, content_h);
        fb.fill_rect(right_x + 2, content_y + 2, right_w - 4, content_h - 4, Color::WHITE);

        // List Column Headers
        let header_y = content_y + 2;
        fb.fill_rect(right_x + 2, header_y, right_w - 4, 18, Color::RETRO_FACE);
        fb.draw_button(right_x + 2, header_y, 160, 18, false);
        fb.draw_string(right_x + 8, header_y + 2, "Name", Color::BLACK);

        fb.draw_button(right_x + 162, header_y, 80, 18, false);
        fb.draw_string(right_x + 168, header_y + 2, "Size", Color::BLACK);

        let type_w = right_w.saturating_sub(242);
        fb.draw_button(right_x + 242, header_y, type_w, 18, false);
        fb.draw_string(right_x + 248, header_y + 2, "Type", Color::BLACK);

        // List Entries
        let list_y = content_y + 21;
        let visible_rows = (content_h.saturating_sub(22)) / 20;

        for (idx, entry) in self.entries.iter().enumerate().skip(self.scroll_offset).take(visible_rows) {
            let row_y = list_y + ((idx - self.scroll_offset) as isize * 20);
            let is_sel = self.selected_idx == Some(idx);

            if is_sel {
                fb.fill_rect(right_x + 2, row_y, right_w - 4, 20, Color::RETRO_SELECTION);
            } else if idx % 2 == 1 {
                fb.fill_rect(right_x + 2, row_y, right_w - 4, 20, Color::from_rgb(248, 250, 252));
            }

            let text_color = if is_sel { Color::WHITE } else { Color::BLACK };

            // Icon
            if entry.is_dir {
                icons::draw_folder_icon_16(fb, right_x + 6, row_y + 2);
            } else {
                icons::draw_file_icon_16(fb, right_x + 6, row_y + 2, &entry.name);
            }

            // Name
            let name_display = if entry.name.len() > 18 {
                &entry.name[..18]
            } else {
                &entry.name
            };
            fb.draw_string(right_x + 26, row_y + 3, name_display, text_color);

            // Size
            let size_str = if entry.is_dir {
                String::from("<DIR>")
            } else if entry.size >= 1024 {
                format!("{} KB", entry.size / 1024)
            } else {
                format!("{} B", entry.size)
            };
            fb.draw_string(right_x + 168, row_y + 3, &size_str, text_color);

            // Type
            let type_str = if entry.is_dir {
                "Folder"
            } else if entry.name.ends_with(".elf") || self.current_path == "/bin" {
                "ELF Executable"
            } else if entry.name.ends_with(".txt") || entry.name.ends_with(".md") {
                "Text Document"
            } else if entry.name.ends_with(".mp3") || entry.name.ends_with(".wav") {
                "Audio File"
            } else if entry.name.ends_with(".png") || entry.name.ends_with(".bmp") {
                "Image File"
            } else {
                "File"
            };
            fb.draw_string(right_x + 248, row_y + 3, type_str, text_color);
        }

        // 4. Status Bar at bottom (height 20)
        let status_y = client_y + client_h as isize - 20;
        fb.draw_sunken_panel(client_x + 2, status_y, client_w - 4, 18);

        let status_text = if let Some((msg, _)) = &self.status_msg {
            msg.clone()
        } else {
            format!("{} object(s) in {}", self.entries.len(), self.current_path)
        };
        fb.draw_string(client_x + 8, status_y + 2, &status_text, Color::BLACK);

        // 5. Modal Properties Dialog if open
        if self.properties_open {
            let pw = 280;
            let ph = 210;
            let px = client_x + (client_w as isize - pw as isize) / 2;
            let py = client_y + (client_h as isize - ph as isize) / 2;

            fb.fill_rect(px, py, pw, ph, Color::RETRO_FACE);
            fb.draw_bevel_raised(px, py, pw, ph);

            // Dialog titlebar
            fb.fill_rect(px + 2, py + 2, pw - 4, 18, Color::from_rgb(0, 0, 128));
            fb.draw_string(px + 8, py + 4, "Properties", Color::WHITE);

            // Close 'X'
            fb.draw_button(px + pw as isize - 18, py + 4, 14, 14, false);
            fb.draw_string(px + pw as isize - 14, py + 4, "X", Color::BLACK);

            if let Some(idx) = self.selected_idx {
                if idx < self.entries.len() {
                    let entry = &self.entries[idx];
                    let ey = py + 28;
                    // Icon and Name
                    if entry.is_dir {
                        icons::draw_folder_icon_16(fb, px + 12, ey);
                    } else {
                        icons::draw_file_icon_16(fb, px + 12, ey, &entry.name);
                    }
                    fb.draw_string(px + 34, ey + 2, &entry.name, Color::BLACK);
                    fb.draw_groove(px + 10, ey + 22, pw - 20, 2);

                    // Details
                    let kind = if entry.is_dir { "Folder" } else { "File" };
                    fb.draw_string(px + 12, ey + 30, &format!("Type: {}", kind), Color::BLACK);
                    fb.draw_string(px + 12, ey + 48, &format!("Location: {}", self.current_path), Color::BLACK);
                    let sz = if entry.is_dir {
                        String::from("Size: 4096 bytes (1 block)")
                    } else {
                        format!("Size: {} bytes", entry.size)
                    };
                    fb.draw_string(px + 12, ey + 66, &sz, Color::BLACK);
                    fb.draw_string(px + 12, ey + 84, &format!("Inode: #{}", entry.inode), Color::BLACK);
                    let perms = if entry.is_dir { "drwxr-xr-x (0755)" } else { "-rw-r--r-- (0644)" };
                    fb.draw_string(px + 12, ey + 102, &format!("Permissions: {}", perms), Color::BLACK);
                    fb.draw_string(px + 12, ey + 120, "Filesystem: MourosFS (ATA)", Color::BLACK);
                }
            } else {
                fb.draw_string(px + 12, py + 36, &format!("Folder: {}", self.current_path), Color::BLACK);
                fb.draw_string(px + 12, py + 56, &format!("Objects: {}", self.entries.len()), Color::BLACK);
                fb.draw_string(px + 12, py + 76, "Filesystem: MourosFS Inode VFS", Color::BLACK);
            }

            // OK button
            let btn_x = px + (pw as isize - 60) / 2;
            let btn_y = py + ph as isize - 28;
            fb.draw_button(btn_x, btn_y, 60, 22, false);
            fb.draw_string(btn_x + 20, btn_y + 4, "OK", Color::BLACK);
        }
    }
}

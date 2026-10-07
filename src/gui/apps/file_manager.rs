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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuTarget {
    Item(usize),
    EmptyArea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    Open,
    Copy,
    Paste,
    Delete,
    Rename,
    NewFile,
    NewFolder,
    Refresh,
    Properties,
}

#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub label: &'static str,
    pub is_separator: bool,
    pub action: ContextMenuAction,
}

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub target: ContextMenuTarget,
    pub items: Vec<ContextMenuItem>,
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
    pub context_menu: Option<ContextMenuState>,
    pub rename_dialog: Option<(usize, String)>,
    pub ctrl_pressed: bool,
    pub last_w: usize,
    pub last_h: usize,
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
            context_menu: None,
            rename_dialog: None,
            ctrl_pressed: false,
            last_w: 580,
            last_h: 380,
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
        self.context_menu = None;
        self.rename_dialog = None;
        self.refresh();
    }

    pub fn go_back(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            self.current_path = self.history[self.history_idx].clone();
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.context_menu = None;
            self.rename_dialog = None;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if self.history_idx + 1 < self.history.len() {
            self.history_idx += 1;
            self.current_path = self.history[self.history_idx].clone();
            self.selected_idx = None;
            self.scroll_offset = 0;
            self.context_menu = None;
            self.rename_dialog = None;
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

    pub fn confirm_rename(&mut self) {
        if let Some((idx, new_name)) = self.rename_dialog.take() {
            if idx < self.entries.len() && !new_name.is_empty() {
                let old_entry = &self.entries[idx];
                let old_path = if self.current_path == "/" {
                    format!("/{}", old_entry.name)
                } else {
                    format!("{}/{}", self.current_path, old_entry.name)
                };
                let new_path = if self.current_path == "/" {
                    format!("/{}", new_name)
                } else {
                    format!("{}/{}", self.current_path, new_name)
                };
                if crate::fs::rename(&old_path, &new_path).is_ok() {
                    let _ = crate::fs::sync();
                    self.status_msg = Some((format!("Renamed to {}", new_name), 40));
                    self.refresh();
                    self.selected_idx = self.entries.iter().position(|e| e.name == new_name);
                } else {
                    self.status_msg = Some((String::from("Rename failed"), 40));
                }
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

    fn on_raw_key(&mut self, event: pc_keyboard::KeyEvent) {
        match event.code {
            KeyCode::LControl | KeyCode::RControl => {
                self.ctrl_pressed = event.state == pc_keyboard::KeyState::Down;
            }
            _ => {}
        }
    }

    fn on_key(&mut self, key: DecodedKey) {
        // 1. Rename dialog active
        if let Some((_, ref mut name_buf)) = self.rename_dialog {
            match key {
                DecodedKey::Unicode('\n') | DecodedKey::Unicode('\r') => {
                    self.confirm_rename();
                }
                DecodedKey::Unicode('\x1b') => {
                    self.rename_dialog = None;
                }
                DecodedKey::Unicode('\u{0008}') => {
                    name_buf.pop();
                }
                DecodedKey::Unicode(c) if c >= ' ' && c <= '~' => {
                    if name_buf.len() < 32 {
                        name_buf.push(c);
                    }
                }
                DecodedKey::RawKey(KeyCode::Escape) => {
                    self.rename_dialog = None;
                }
                DecodedKey::RawKey(KeyCode::Backspace) => {
                    name_buf.pop();
                }
                _ => {}
            }
            return;
        }

        // 2. Normal key handling
        match key {
            DecodedKey::Unicode('\x1b') | DecodedKey::RawKey(KeyCode::Escape) => {
                // Dismiss context menu or properties
                if self.context_menu.is_some() {
                    self.context_menu = None;
                } else if self.properties_open {
                    self.properties_open = false;
                }
            }
            DecodedKey::Unicode('\x03') => {
                self.copy_selected();
            }
            DecodedKey::Unicode('\x16') => {
                self.paste_clipboard();
            }
            DecodedKey::Unicode(c) if self.ctrl_pressed && (c == 'c' || c == 'C') => {
                self.copy_selected();
            }
            DecodedKey::Unicode(c) if self.ctrl_pressed && (c == 'v' || c == 'V') => {
                self.paste_clipboard();
            }
            DecodedKey::RawKey(KeyCode::F2) => {
                if let Some(idx) = self.selected_idx {
                    if idx < self.entries.len() {
                        let name = self.entries[idx].name.clone();
                        self.rename_dialog = Some((idx, name));
                    }
                }
            }
            DecodedKey::RawKey(KeyCode::Delete) => {
                self.delete_selected();
            }
            DecodedKey::Unicode('\n') | DecodedKey::Unicode('\r') => {
                self.open_selected();
            }
            DecodedKey::RawKey(KeyCode::ArrowUp) => {
                if let Some(sel) = self.selected_idx {
                    if sel > 0 {
                        self.selected_idx = Some(sel - 1);
                    }
                } else if !self.entries.is_empty() {
                    self.selected_idx = Some(0);
                }
            }
            DecodedKey::RawKey(KeyCode::ArrowDown) => {
                if let Some(sel) = self.selected_idx {
                    if sel + 1 < self.entries.len() {
                        self.selected_idx = Some(sel + 1);
                    }
                } else if !self.entries.is_empty() {
                    self.selected_idx = Some(0);
                }
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
        let client_w = self.last_w.max(500);
        let client_h = self.last_h.max(340);

        // 1. Rename Dialog Hit-Testing
        if let Some((_, _)) = &self.rename_dialog {
            if left {
                let rw = 260;
                let rh = 110;
                let rx = (client_w as isize - rw as isize) / 2;
                let ry = (client_h as isize - rh as isize) / 2;
                let btn_y = ry + 74;

                // Close 'X' button
                if local_x >= rx + rw as isize - 18 && local_x < rx + rw as isize - 4 && local_y >= ry + 4 && local_y < ry + 18 {
                    self.rename_dialog = None;
                    return;
                }
                // OK button
                if local_x >= rx + 44 && local_x < rx + 114 && local_y >= btn_y && local_y < btn_y + 22 {
                    self.confirm_rename();
                    return;
                }
                // Cancel button
                if local_x >= rx + 144 && local_x < rx + 214 && local_y >= btn_y && local_y < btn_y + 22 {
                    self.rename_dialog = None;
                    return;
                }
                // Click outside modal cancels
                if local_x < rx || local_x >= rx + rw as isize || local_y < ry || local_y >= ry + rh as isize {
                    self.rename_dialog = None;
                    return;
                }
            }
            return;
        }

        // 2. Properties Dialog Hit-Testing
        if self.properties_open {
            if left {
                self.properties_open = false;
            }
            return;
        }

        // 3. Context Menu Hit-Testing
        if let Some(menu) = self.context_menu.take() {
            if left {
                let menu_w = menu.width as isize;
                let menu_h = (menu.items.len() * 20 + 6) as isize;
                let clamped_x = menu.x.clamp(4, (client_w.saturating_sub(menu.width + 4)) as isize);
                let clamped_y = menu.y.clamp(4, (client_h.saturating_sub(menu_h as usize + 4)) as isize);

                if local_x >= clamped_x && local_x < clamped_x + menu_w && local_y >= clamped_y && local_y < clamped_y + menu_h {
                    let rel_y = local_y - (clamped_y + 3);
                    if rel_y >= 0 {
                        let item_idx = (rel_y / 20) as usize;
                        if item_idx < menu.items.len() {
                            let item = &menu.items[item_idx];
                            if !item.is_separator {
                                match item.action {
                                    ContextMenuAction::Open => self.open_selected(),
                                    ContextMenuAction::Copy => self.copy_selected(),
                                    ContextMenuAction::Paste => self.paste_clipboard(),
                                    ContextMenuAction::Delete => self.delete_selected(),
                                    ContextMenuAction::Rename => {
                                        if let Some(idx) = self.selected_idx {
                                            if idx < self.entries.len() {
                                                let name = self.entries[idx].name.clone();
                                                self.rename_dialog = Some((idx, name));
                                            }
                                        }
                                    }
                                    ContextMenuAction::NewFile => self.new_file(),
                                    ContextMenuAction::NewFolder => self.new_folder(),
                                    ContextMenuAction::Refresh => self.refresh(),
                                    ContextMenuAction::Properties => self.properties_open = true,
                                }
                            }
                        }
                    }
                    return;
                }
            }
            // If click was outside, context menu is dismissed; proceed to handle click normally
        }

        // 4. Dispatch Click: Left vs Right Click
        if left {
            // A. Clean Top Toolbar buttons (y: 2..24)
            if local_y >= 2 && local_y <= 24 {
                if local_x >= 4 && local_x < 52 {
                    self.go_back();
                } else if local_x >= 54 && local_x < 98 {
                    self.go_forward();
                } else if local_x >= 100 && local_x < 140 {
                    self.go_up();
                } else if local_x >= 148 && local_x < 206 {
                    self.refresh();
                } else if local_x >= 214 && local_x < 288 {
                    self.properties_open = true;
                }
                return;
            }

            // B. Left Quick Access Locations Panel (x: 4..124, y: 74..)
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

            // C. Right File List View (x: 128..client_w - 4, y: 72..client_h - 24)
            if local_x >= 128 && local_y >= 72 {
                let item_y = local_y - 72;
                let clicked_idx = (item_y / 20) as usize + self.scroll_offset;
                if clicked_idx < self.entries.len() {
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
        } else {
            // RIGHT CLICK CONTEXT MENU!
            if local_x >= 128 && local_y >= 72 {
                let item_y = local_y - 72;
                let clicked_idx = (item_y / 20) as usize + self.scroll_offset;
                let menu_w = 136;

                if clicked_idx < self.entries.len() {
                    // Right click on file/folder entry
                    self.selected_idx = Some(clicked_idx);
                    let items = alloc::vec![
                        ContextMenuItem { label: "Open", is_separator: false, action: ContextMenuAction::Open },
                        ContextMenuItem { label: "", is_separator: true, action: ContextMenuAction::Open },
                        ContextMenuItem { label: "Copy", is_separator: false, action: ContextMenuAction::Copy },
                        ContextMenuItem { label: "Delete", is_separator: false, action: ContextMenuAction::Delete },
                        ContextMenuItem { label: "Rename", is_separator: false, action: ContextMenuAction::Rename },
                        ContextMenuItem { label: "", is_separator: true, action: ContextMenuAction::Open },
                        ContextMenuItem { label: "Properties", is_separator: false, action: ContextMenuAction::Properties },
                    ];
                    self.context_menu = Some(ContextMenuState {
                        x: local_x,
                        y: local_y,
                        width: menu_w,
                        target: ContextMenuTarget::Item(clicked_idx),
                        items,
                    });
                } else {
                    // Right click on blank/empty folder space
                    self.selected_idx = None;
                    let items = alloc::vec![
                        ContextMenuItem { label: "Refresh", is_separator: false, action: ContextMenuAction::Refresh },
                        ContextMenuItem { label: "", is_separator: true, action: ContextMenuAction::Refresh },
                        ContextMenuItem { label: "New File", is_separator: false, action: ContextMenuAction::NewFile },
                        ContextMenuItem { label: "New Folder", is_separator: false, action: ContextMenuAction::NewFolder },
                        ContextMenuItem { label: "", is_separator: true, action: ContextMenuAction::Refresh },
                        ContextMenuItem { label: "Paste", is_separator: false, action: ContextMenuAction::Paste },
                        ContextMenuItem { label: "Properties", is_separator: false, action: ContextMenuAction::Properties },
                    ];
                    self.context_menu = Some(ContextMenuState {
                        x: local_x,
                        y: local_y,
                        width: menu_w,
                        target: ContextMenuTarget::EmptyArea,
                        items,
                    });
                }
            }
        }
    }

    fn on_mouse_scroll(&mut self, _local_x: isize, _local_y: isize, delta: i32) -> bool {
        let max_rows = self.last_h.saturating_sub(96) / 20;
        let max_scroll = self.entries.len().saturating_sub(max_rows.max(1));
        if delta > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
        } else if delta < 0 {
            self.scroll_offset = (self.scroll_offset + 3).min(max_scroll);
        }
        true
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        self.last_w = client_w;
        self.last_h = client_h;

        // Background panel
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::RETRO_FACE);

        // 1. Clean Toolbar (height 26) - Essential buttons only!
        fb.draw_button(client_x + 4, client_y + 2, 48, 22, false);
        fb.draw_string(client_x + 8, client_y + 6, "< Back", Color::BLACK);

        fb.draw_button(client_x + 54, client_y + 2, 44, 22, false);
        fb.draw_string(client_x + 58, client_y + 6, "> Fwd", Color::BLACK);

        fb.draw_button(client_x + 100, client_y + 2, 40, 22, false);
        fb.draw_string(client_x + 106, client_y + 6, "^ Up", Color::BLACK);

        // Vertical separator groove
        fb.draw_groove(client_x + 144, client_y + 3, 2, 20);

        fb.draw_button(client_x + 148, client_y + 2, 58, 22, false);
        fb.draw_string(client_x + 152, client_y + 6, "Refresh", Color::BLACK);

        // Vertical separator groove
        fb.draw_groove(client_x + 210, client_y + 3, 2, 20);

        fb.draw_button(client_x + 214, client_y + 2, 74, 22, false);
        fb.draw_string(client_x + 218, client_y + 6, "Properties", Color::BLACK);

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

        // Quick Access Header
        fb.draw_string(client_x + 8, content_y + 6, "Folders", Color::BLACK);
        fb.draw_groove(client_x + 6, content_y + 20, left_w - 4, 2);

        let locations = [
            ("/", "Root (/)"),
            ("/bin", "Binaries"),
            ("/etc", "Config"),
            ("/home/user", "User Home"),
            ("/tmp", "Temp"),
            ("/dev", "Devices"),
        ];

        for (i, (loc_path, loc_label)) in locations.iter().enumerate() {
            let ly = content_y + 24 + (i as isize * 22);
            let is_active = self.current_path == *loc_path;
            if is_active {
                fb.fill_rect(client_x + 8, ly, left_w - 8, 20, Color::from_rgb(205, 230, 255));
                fb.draw_rect(client_x + 8, ly, left_w - 8, 20, Color::from_rgb(0, 100, 200));
            }
            icons::draw_folder_icon_16(fb, client_x + 10, ly + 2);
            fb.draw_string(client_x + 28, ly + 3, loc_label, Color::BLACK);
        }

        // Right File List Details View
        let right_x = client_x + left_w as isize + 6;
        let right_w = client_w.saturating_sub(left_w + 10);
        fb.draw_sunken_panel(right_x, content_y, right_w, content_h);
        fb.fill_rect(right_x + 2, content_y + 2, right_w - 4, content_h - 4, Color::WHITE);

        // Header Row (Name, Size, Type)
        let header_h = 18;
        fb.fill_rect(right_x + 2, content_y + 2, right_w - 4, header_h, Color::RETRO_FACE);
        fb.draw_bevel_raised(right_x + 2, content_y + 2, right_w - 4, header_h);
        fb.draw_string(right_x + 6, content_y + 4, "Name", Color::BLACK);
        fb.draw_groove(right_x + 160, content_y + 3, 2, 16);
        fb.draw_string(right_x + 168, content_y + 4, "Size", Color::BLACK);
        fb.draw_groove(right_x + 240, content_y + 3, 2, 16);
        fb.draw_string(right_x + 248, content_y + 4, "Type", Color::BLACK);

        // File rows
        let list_start_y = content_y + 22;
        let max_rows = (content_h.saturating_sub(24)) / 20;

        for (idx, entry) in self.entries.iter().skip(self.scroll_offset).take(max_rows).enumerate() {
            let actual_idx = idx + self.scroll_offset;
            let row_y = list_start_y + (idx as isize * 20);
            let is_selected = self.selected_idx == Some(actual_idx);

            if is_selected {
                fb.fill_rect(right_x + 3, row_y, right_w - 6, 20, Color::RETRO_SELECTION);
            }

            let text_color = if is_selected { Color::WHITE } else { Color::BLACK };

            // File Icon
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
            format!("{} object(s) in {} (Right-click for options)", self.entries.len(), self.current_path)
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

        // 6. Modal Rename Dialog if open
        if let Some((_, ref new_name)) = self.rename_dialog {
            let rw = 260;
            let rh = 110;
            let rx = client_x + (client_w as isize - rw as isize) / 2;
            let ry = client_y + (client_h as isize - rh as isize) / 2;

            fb.fill_rect(rx, ry, rw, rh, Color::RETRO_FACE);
            fb.draw_bevel_raised(rx, ry, rw, rh);

            // Dialog titlebar
            fb.fill_rect(rx + 2, ry + 2, rw - 4, 18, Color::from_rgb(0, 0, 128));
            fb.draw_string(rx + 8, ry + 4, "Rename Item", Color::WHITE);

            // Close 'X'
            fb.draw_button(rx + rw as isize - 18, ry + 4, 14, 14, false);
            fb.draw_string(rx + rw as isize - 14, ry + 4, "X", Color::BLACK);

            fb.draw_string(rx + 12, ry + 26, "Enter new name:", Color::BLACK);

            let input_x = rx + 12;
            let input_y = ry + 44;
            let input_w = rw - 24;
            fb.fill_rect(input_x, input_y, input_w, 20, Color::WHITE);
            fb.draw_bevel_sunken(input_x, input_y, input_w, 20);
            fb.draw_string(input_x + 4, input_y + 4, new_name, Color::BLACK);

            // Buttons: [ OK ] [ Cancel ]
            let btn_y = ry + 74;
            fb.draw_button(rx + 44, btn_y, 70, 22, false);
            fb.draw_string(rx + 68, btn_y + 4, "OK", Color::BLACK);

            fb.draw_button(rx + 144, btn_y, 70, 22, false);
            fb.draw_string(rx + 154, btn_y + 4, "Cancel", Color::BLACK);
        }

        // 7. Right-Click Context Menu (drawn on top of everything)
        if let Some(menu) = &self.context_menu {
            let menu_w = menu.width;
            let menu_h = menu.items.len() * 20 + 6;
            let clamped_x = menu.x.clamp(4, (client_w.saturating_sub(menu_w + 4)) as isize);
            let clamped_y = menu.y.clamp(4, (client_h.saturating_sub(menu_h + 4)) as isize);
            let mx = client_x + clamped_x;
            let my = client_y + clamped_y;

            // 3D raised frame
            fb.fill_rect(mx, my, menu_w, menu_h, Color::RETRO_FACE);
            fb.draw_bevel_raised(mx, my, menu_w, menu_h);

            let mouse_state = crate::drivers::mouse::get_mouse_state();
            let mut item_y = my + 3;

            for item in &menu.items {
                if item.is_separator {
                    fb.draw_groove(mx + 4, item_y + 9, menu_w - 8, 2);
                } else {
                    let is_hovered = mouse_state.x >= mx + 2
                        && mouse_state.x < mx + (menu_w as isize) - 2
                        && mouse_state.y >= item_y
                        && mouse_state.y < item_y + 20;

                    if is_hovered {
                        fb.fill_rect(mx + 2, item_y, menu_w - 4, 20, Color::RETRO_SELECTION);
                    }

                    let text_color = if is_hovered {
                        Color::WHITE
                    } else {
                        Color::BLACK
                    };

                    fb.draw_string(mx + 18, item_y + 4, item.label, text_color);
                }
                item_y += 20;
            }
        }
    }
}

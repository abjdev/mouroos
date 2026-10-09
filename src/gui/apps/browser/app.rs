use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use pc_keyboard::{DecodedKey, KeyCode};

use crate::gui::apps::browser::css::{compute_styles, CssRule};
use crate::gui::apps::browser::dom::DomTree;
use crate::gui::apps::browser::html::{compute_layout, layout_html, parse_html, ClickableAction, DocumentLayout, LayoutItem};
use crate::gui::apps::browser::js::JsContext;
use crate::gui::color::Color;
use crate::gui::font::{FONT_HEIGHT, FONT_WIDTH};
use crate::gui::framebuffer::Framebuffer;
use crate::gui::window::Application;
use crate::net::socket::TcpStream;
use crate::net::tls::TlsStream;

pub struct BrowserApp {
    pub url_input: String,
    pub current_url: String,
    pub history_back: Vec<String>,
    pub history_forward: Vec<String>,
    pub dom_tree: DomTree,
    pub stylesheets: Vec<CssRule>,
    pub js_context: JsContext,
    pub alert_dialog: Option<String>,
    pub layout: DocumentLayout,
    pub scroll_offset: usize,
    pub address_bar_focused: bool,
    pub cursor_visible: bool,
    pub cursor_col: usize,
    pub blink_counter: usize,
    pub status_text: String,
    pub warning_banner: Option<String>,
    pub page_title: String,
    pub last_width: usize,
    pub last_height: usize,
    pub pending_navigate: Option<String>,
}

impl BrowserApp {
    pub fn new() -> Self {
        Self::with_url("about:home")
    }

    pub fn with_url(initial_url: &str) -> Self {
        let mut app = BrowserApp {
            url_input: if initial_url == "about:home" {
                String::from("http://10.0.2.2:8000/")
            } else {
                String::from(initial_url)
            },
            current_url: String::from(initial_url),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            dom_tree: DomTree::new(),
            stylesheets: Vec::new(),
            js_context: JsContext::new(),
            alert_dialog: None,
            layout: DocumentLayout {
                title: String::from("Mouros Browser"),
                items: Vec::new(),
                links: Vec::new(),
                clickables: Vec::new(),
                total_height: 100,
            },
            scroll_offset: 0,
            address_bar_focused: false,
            cursor_visible: true,
            cursor_col: 0,
            blink_counter: 0,
            status_text: String::from("Ready"),
            warning_banner: None,
            page_title: String::from("Mouros Browser"),
            last_width: 620,
            last_height: 420,
            pending_navigate: None,
        };

        if initial_url == "about:home" || initial_url.is_empty() {
            app.load_welcome_page();
        } else {
            app.navigate_to(initial_url, false);
        }

        app
    }

    pub fn update_layout(&mut self) {
        let styles = compute_styles(&self.dom_tree, &self.stylesheets);
        let layout = compute_layout(&self.dom_tree, &styles, self.last_width.saturating_sub(40));
        self.page_title = format!("Mouros Browser - {}", layout.title);
        self.layout = layout;
    }

    pub fn load_html(&mut self, html_text: &str) {
        let doc = parse_html(html_text);
        self.dom_tree = doc.tree;
        self.stylesheets = doc.stylesheets;
        self.js_context = JsContext::new();

        // Execute top-level scripts
        for script in doc.scripts {
            self.js_context.eval_script(&mut self.dom_tree, &script);
        }

        if let Some(first_alert) = self.js_context.alerts.last() {
            self.alert_dialog = Some(first_alert.clone());
        }

        self.update_layout();
        self.scroll_offset = 0;
    }

    pub fn load_welcome_page(&mut self) {
        let welcome_html = concat!(
            "<html><head><title>Mouros Web Browser</title>",
            "<style>",
            "  body { background-color: #f0f4f8; color: #111; }",
            "  .header { background-color: #004488; color: #ffffff; padding: 10px; margin-bottom: 8px; border: 1px solid #002244; }",
            "  .card { background-color: #ffffff; padding: 8px; margin-bottom: 8px; border: 1px solid #cccccc; }",
            "  .btn { padding: 4px; font-weight: bold; }",
            "  .highlight { color: #cc0000; font-weight: bold; }",
            "</style>",
            "<script>",
            "  var counter = 0;",
            "  function incCount() {",
            "    counter = counter + 1;",
            "    document.getElementById('counter_val').innerText = 'Count: ' + counter;",
            "  }",
            "  function decCount() {",
            "    counter = counter - 1;",
            "    document.getElementById('counter_val').innerText = 'Count: ' + counter;",
            "  }",
            "  function toggleCardColor() {",
            "    var c = document.getElementById('demo_card');",
            "    if (c.style.backgroundColor == 'yellow') {",
            "      c.style.backgroundColor = 'white';",
            "    } else {",
            "      c.style.backgroundColor = 'yellow';",
            "    }",
            "  }",
            "  function triggerAlert() {",
            "    alert('JavaScript execution verified on Mouros OS!');",
            "  }",
            "</script>",
            "</head><body>",
            "<div class=\"header\">",
            "  <h1>Mouros Browser</h1>",
            "  <p>Bare-metal x86_64 OS with native CSS Styling and JavaScript Engine!</p>",
            "</div>",
            "<div class=\"card\" id=\"demo_card\" style=\"background-color: white;\">",
            "  <h2>Interactive DOM & JavaScript Showcase</h2>",
            "  <p>Test reactive JavaScript state manipulation and CSS updates in real-time:</p>",
            "  <p>",
            "    <button onclick=\"decCount()\"> - </button> ",
            "    <span id=\"counter_val\" style=\"color: blue; font-weight: bold;\">Count: 0</span> ",
            "    <button onclick=\"incCount()\"> + </button>",
            "  </p>",
            "  <p>",
            "    <button onclick=\"toggleCardColor()\">Toggle Card Color</button> ",
            "    <button onclick=\"triggerAlert()\">Trigger Alert</button>",
            "  </p>",
            "</div>",
            "<div class=\"card\">",
            "  <h2>Built-in Capabilities</h2>",
            "  <ul>",
            "    <li>Native CSS Engine: stylesheets, classes, IDs, cascade, colors, boxes</li>",
            "    <li>Native JavaScript Engine: variables, conditionals, loops, functions, DOM APIs</li>",
            "    <li>Reactive Layout: DOM mutations dynamically recompute layout and paint</li>",
            "    <li>Networking & Security: RTL8139 NIC, TCP/IP, DNS, DHCP, TLS 1.3 HTTPS</li>",
            "  </ul>",
            "</div>",
            "<div class=\"card\">",
            "  <h2>Quick Navigation Links</h2>",
            "  <ul>",
            "    <li><a href=\"https://abjdev.github.io/hw.html\">Live HTTPS Test (abjdev.github.io/hw.html)</a></li>",
            "    <li><a href=\"http://example.com/\">Example Domain (example.com)</a></li>",
            "    <li><a href=\"http://10.0.2.2:8000/\">Local Host Server (10.0.2.2:8000)</a></li>",
            "  </ul>",
            "</div>",
            "</body></html>"
        );
        self.load_html(welcome_html);
        self.status_text = String::from("Done (Welcome Page with CSS & JS)");
    }

    pub fn navigate_to(&mut self, url: &str, add_to_history: bool) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return;
        }

        self.warning_banner = None;

        if trimmed == "about:home" || trimmed == "about:blank" {
            self.load_welcome_page();
            self.url_input = String::from(trimmed);
            self.current_url = String::from(trimmed);
            return;
        }

        let full_url = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            format!("http://{}", trimmed)
        } else {
            String::from(trimmed)
        };

        let is_https = full_url.starts_with("https://");

        if add_to_history && !self.current_url.is_empty() && self.current_url != full_url {
            self.history_back.push(self.current_url.clone());
            self.history_forward.clear();
        }

        self.url_input = full_url.clone();
        self.current_url = full_url.clone();
        self.status_text = format!("Connecting to {}...", self.current_url);

        let without_proto = if is_https {
            &self.current_url[8..] // strip https://
        } else {
            &self.current_url[7..] // strip http://
        };

        let (host_port, path) = match without_proto.find('/') {
            Some(idx) => (&without_proto[..idx], &without_proto[idx..]),
            None => (without_proto, "/"),
        };

        let default_port = if is_https { 443 } else { 80 };
        let (host, port) = match host_port.find(':') {
            Some(idx) => {
                let h = &host_port[..idx];
                let p = host_port[idx + 1..].parse::<u16>().unwrap_or(default_port);
                (h, p)
            }
            None => (host_port, default_port),
        };

        self.status_text = format!("Resolving host {}...", host);

        let ip = match crate::net::resolve_hostname(host) {
            Some(addr) => addr,
            None => {
                self.status_text = format!("Error: Cannot resolve host '{}'", host);
                let err_html = format!(
                    "<html><head><title>DNS Error</title></head><body>\
                    <h1>Host Resolution Failed</h1>\
                    <p>Could not resolve host <strong>{}</strong>.</p>\
                    <p>Check network settings via <code>ifconfig</code> or <code>dns</code> in terminal.</p>\
                    </body></html>",
                    host
                );
                self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                self.page_title = String::from("Mouros Browser - DNS Error");
                return;
            }
        };

        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: MourosBrowser/1.0\r\nAccept: text/html,*/*\r\nConnection: close\r\n\r\n",
            path, host
        );

        let response_bytes = if is_https {
            self.status_text = format!("Connecting TLS 1.3 to {}:{}...", host, port);
            let mut stream = match TlsStream::connect(ip, port, host) {
                Ok(s) => s,
                Err(e) => {
                    self.status_text = format!("TLS Error: {}", e);
                    let err_html = format!(
                        "<html><head><title>TLS Error</title></head><body>\
                        <h1>TLS 1.3 Handshake Failed</h1>\
                        <p>Could not establish encrypted connection to <strong>{}:{}</strong>: {}</p>\
                        </body></html>",
                        host, port, e
                    );
                    self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                    self.page_title = String::from("Mouros Browser - TLS Error");
                    return;
                }
            };

            self.status_text = String::from("Sending encrypted HTTP GET request...");
            if let Err(e) = stream.write(request.as_bytes()) {
                self.status_text = format!("Send error: {}", e);
                let err_html = format!(
                    "<html><head><title>Network Send Error</title></head><body>\
                    <h1>Failed to Send Request</h1>\
                    <p>Could not send encrypted HTTP request to <strong>{}:{}</strong>: {}</p>\
                    </body></html>",
                    host, port, e
                );
                self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                self.page_title = String::from("Mouros Browser - Send Error");
                return;
            }

            self.status_text = String::from("Reading encrypted response...");
            match stream.read_to_end(65536) {
                Ok(b) => b,
                Err(e) => {
                    self.status_text = format!("Read error: {}", e);
                    let err_html = format!(
                        "<html><head><title>Network Read Error</title></head><body>\
                        <h1>Failed to Read Response</h1>\
                        <p>Error while reading response from <strong>{}:{}</strong>: {}</p>\
                        </body></html>",
                        host, port, e
                    );
                    self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                    self.page_title = String::from("Mouros Browser - Read Error");
                    return;
                }
            }
        } else {
            self.status_text = format!("Connecting to {}:{}...", ip, port);
            let mut stream = match TcpStream::connect(ip, port) {
                Ok(s) => s,
                Err(e) => {
                    self.status_text = format!("Error: {}", e);
                    let err_html = format!(
                        "<html><head><title>Connection Error</title></head><body>\
                        <h1>Failed to Connect</h1>\
                        <p>Could not establish TCP connection to <strong>{}:{}</strong>: {}</p>\
                        </body></html>",
                        ip, port, e
                    );
                    self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                    self.page_title = String::from("Mouros Browser - Connection Error");
                    return;
                }
            };

            self.status_text = String::from("Sending HTTP GET request...");
            if let Err(e) = stream.write(request.as_bytes()) {
                self.status_text = format!("Send error: {}", e);
                let err_html = format!(
                    "<html><head><title>Network Send Error</title></head><body>\
                    <h1>Failed to Send Request</h1>\
                    <p>Could not send HTTP request to <strong>{}:{}</strong>: {}</p>\
                    </body></html>",
                    ip, port, e
                );
                self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                self.page_title = String::from("Mouros Browser - Send Error");
                return;
            }

            self.status_text = String::from("Reading HTTP response...");
            match stream.read_to_end(65536) {
                Ok(b) => b,
                Err(e) => {
                    self.status_text = format!("Read error: {}", e);
                    let err_html = format!(
                        "<html><head><title>Network Read Error</title></head><body>\
                        <h1>Failed to Read Response</h1>\
                        <p>Error while reading response from <strong>{}:{}</strong>: {}</p>\
                        </body></html>",
                        ip, port, e
                    );
                    self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
                    self.page_title = String::from("Mouros Browser - Read Error");
                    return;
                }
            }
        };

        if response_bytes.is_empty() {
            self.status_text = String::from("Error: Empty response");
            let err_html = format!(
                "<html><head><title>Empty Response</title></head><body>\
                <h1>Empty Response From Server</h1>\
                <p>The host at <strong>{}</strong> ({}:{}) connected successfully but closed the connection without returning any response data.</p>\
                </body></html>",
                host, ip, port
            );
            self.layout = layout_html(&err_html, self.last_width.saturating_sub(40));
            self.page_title = String::from("Mouros Browser - Empty Response");
            return;
        }

        // Parse HTTP status & body
        let response_str = String::from_utf8_lossy(&response_bytes);
        let header_end = response_str.find("\r\n\r\n").or_else(|| response_str.find("\n\n"));

        let (headers, body) = match header_end {
            Some(idx) => {
                let h = &response_str[..idx];
                let b = if response_str[idx..].starts_with("\r\n\r\n") {
                    &response_str[idx + 4..]
                } else {
                    &response_str[idx + 2..]
                };
                (h, b)
            }
            None => ("", response_str.as_ref()),
        };

        // Extract status line
        let first_line = headers.lines().next().unwrap_or("HTTP/1.1 200 OK");
        let status_code: u16 = first_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);

        // Check for redirects (301, 302, 303, 307, 308)
        if status_code == 301 || status_code == 302 || status_code == 303 || status_code == 307 || status_code == 308 {
            for line in headers.lines() {
                if line.to_lowercase().starts_with("location:") {
                    let new_loc = line[9..].trim();
                    let target_url = if new_loc.starts_with("http://") || new_loc.starts_with("https://") {
                        String::from(new_loc)
                    } else if new_loc.starts_with("//") {
                        if is_https {
                            format!("https:{}", new_loc)
                        } else {
                            format!("http:{}", new_loc)
                        }
                    } else if new_loc.starts_with('/') {
                        let proto = if is_https { "https" } else { "http" };
                        format!("{}://{}{}", proto, host, new_loc)
                    } else {
                        let proto = if is_https { "https" } else { "http" };
                        format!("{}://{}/{}", proto, host, new_loc)
                    };

                    self.status_text = format!("Redirecting to {}...", target_url);
                    self.navigate_to(&target_url, add_to_history);
                    return;
                }
            }
        }

        let is_chunked = headers.lines().any(|l| {
            let lower = l.to_lowercase();
            lower.starts_with("transfer-encoding:") && lower.contains("chunked")
        });

        let decoded_body = if is_chunked {
            crate::net::socket::decode_chunked_body(body)
        } else {
            String::from(body)
        };

        self.load_html(&decoded_body);
        let proto_tag = if is_https { "HTTPS TLS 1.3" } else { "HTTP" };
        self.status_text = format!("Done ({}, {}, {} bytes)", proto_tag, first_line, decoded_body.len());
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.history_back.pop() {
            self.history_forward.push(self.current_url.clone());
            self.navigate_to(&prev, false);
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(next) = self.history_forward.pop() {
            self.history_back.push(self.current_url.clone());
            self.navigate_to(&next, false);
        }
    }

    pub fn reload(&mut self) {
        let url = self.current_url.clone();
        self.navigate_to(&url, false);
    }
}

impl Application for BrowserApp {
    fn title(&self) -> &str {
        &self.page_title
    }

    fn on_tick(&mut self) -> bool {
        self.blink_counter = self.blink_counter.wrapping_add(1);
        if self.blink_counter % 30 == 0 {
            self.cursor_visible = !self.cursor_visible;
            return true;
        }
        if let Some(url) = self.pending_navigate.take() {
            self.navigate_to(&url, true);
            return true;
        }
        false
    }

    fn on_key(&mut self, key: DecodedKey) {
        if self.address_bar_focused {
            match key {
                DecodedKey::Unicode('\n') | DecodedKey::Unicode('\r') => {
                    self.address_bar_focused = false;
                    let target = self.url_input.clone();
                    self.navigate_to(&target, true);
                }
                DecodedKey::Unicode('\x1b') | DecodedKey::RawKey(KeyCode::Escape) => {
                    self.address_bar_focused = false;
                    self.url_input = self.current_url.clone();
                }
                DecodedKey::Unicode('\u{0008}') | DecodedKey::RawKey(KeyCode::Backspace) => {
                    if self.cursor_col > 0 && self.cursor_col <= self.url_input.len() {
                        self.url_input.remove(self.cursor_col - 1);
                        self.cursor_col -= 1;
                    }
                }
                DecodedKey::RawKey(KeyCode::ArrowLeft) => {
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                    }
                }
                DecodedKey::RawKey(KeyCode::ArrowRight) => {
                    if self.cursor_col < self.url_input.len() {
                        self.cursor_col += 1;
                    }
                }
                DecodedKey::Unicode(c) if c >= ' ' && c <= '~' => {
                    if self.cursor_col <= self.url_input.len() {
                        self.url_input.insert(self.cursor_col, c);
                        self.cursor_col += 1;
                    }
                }
                _ => {}
            }
        } else {
            // Viewport scrolling keys
            match key {
                DecodedKey::RawKey(KeyCode::ArrowDown) => {
                    self.scroll_offset = self.scroll_offset.saturating_add(20);
                }
                DecodedKey::RawKey(KeyCode::ArrowUp) => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(20);
                }
                DecodedKey::RawKey(KeyCode::PageDown) => {
                    self.scroll_offset = self.scroll_offset.saturating_add(160);
                }
                DecodedKey::RawKey(KeyCode::PageUp) => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(160);
                }
                DecodedKey::RawKey(KeyCode::Home) => {
                    self.scroll_offset = 0;
                }
                DecodedKey::RawKey(KeyCode::End) => {
                    self.scroll_offset = self.layout.total_height;
                }
                _ => {}
            }
        }
    }

    fn on_mouse_click(&mut self, local_x: isize, local_y: isize, left: bool) {
        if !left {
            return;
        }

        let w = self.last_width;
        let h = self.last_height;

        if self.alert_dialog.is_some() {
            let dlg_w = 320;
            let dlg_h = 130;
            let dlg_x = (w as isize - dlg_w as isize) / 2;
            let dlg_y = (h as isize - dlg_h as isize) / 2;
            let ok_w = 64;
            let ok_h = 24;
            let ok_x = dlg_x + (dlg_w as isize - ok_w as isize) / 2;
            let ok_y = dlg_y + dlg_h as isize - 36;

            if local_x >= ok_x && local_x < ok_x + ok_w as isize && local_y >= ok_y && local_y < ok_y + ok_h as isize {
                self.alert_dialog = None;
            } else {
                self.alert_dialog = None;
            }
            return;
        }

        // 1. Top toolbar click (y: 4..28)
        if local_y >= 4 && local_y <= 28 {
            // Back button: 4..28
            if local_x >= 4 && local_x < 28 {
                self.go_back();
                return;
            }
            // Forward button: 32..56
            if local_x >= 32 && local_x < 56 {
                self.go_forward();
                return;
            }
            // Reload button: 60..84
            if local_x >= 60 && local_x < 84 {
                self.reload();
                return;
            }
            // Address bar: 92..w - 56
            let addr_w = w.saturating_sub(148);
            if local_x >= 92 && local_x < (92 + addr_w as isize) {
                self.address_bar_focused = true;
                let rel = local_x - 96;
                self.cursor_col = if rel > 0 {
                    (rel as usize / FONT_WIDTH).min(self.url_input.len())
                } else {
                    0
                };
                return;
            }
            // Go button: w - 50..w - 6
            if local_x >= (w as isize - 50) && local_x < (w as isize - 6) {
                self.address_bar_focused = false;
                let target = self.url_input.clone();
                self.navigate_to(&target, true);
                return;
            }
        }

        // 2. Viewport click
        let banner_offset: isize = if self.warning_banner.is_some() { 22 } else { 0 };
        let view_top = 34 + banner_offset;
        let view_bottom = h as isize - 24;

        if local_y >= view_top && local_y < view_bottom {
            let view_x = local_x - 8;
            let view_y = (local_y - view_top - 4) + self.scroll_offset as isize;

            if view_x >= 0 && view_y >= 0 {
                // Check if user clicked any interactive item in document
                for clickable in &self.layout.clickables {
                    let cx = clickable.x as isize;
                    let cy = clickable.y as isize;
                    let cw = clickable.width as isize;
                    let ch = clickable.height as isize;

                    if view_x >= cx.saturating_sub(2) && view_x < cx + cw + 2 && view_y >= cy.saturating_sub(2) && view_y < cy + ch + 2 {
                        match &clickable.action {
                            ClickableAction::Navigate(target_url) => {
                                let mut target = target_url.clone();
                                // Handle relative links
                                if !target.starts_with("http://") && !target.starts_with("https://") {
                                    if target.starts_with('/') {
                                        // Find host prefix of current url
                                        if let Some(idx) = self.current_url.find("://") {
                                            let rem = &self.current_url[idx + 3..];
                                            let host = rem.split('/').next().unwrap_or("");
                                            target = format!("http://{}{}", host, target);
                                        }
                                    } else {
                                        target = format!("http://{}", target);
                                    }
                                }
                                self.pending_navigate = Some(target);
                                return;
                            }
                            ClickableAction::JavaScript(code) => {
                                self.js_context.eval_script(&mut self.dom_tree, code);
                                if let Some(msg) = self.js_context.alerts.last() {
                                    self.alert_dialog = Some(msg.clone());
                                }
                                self.update_layout();
                                return;
                            }
                            ClickableAction::ButtonClick(node_id) => {
                                let onclick = self.dom_tree.get_node(*node_id).and_then(|n| n.onclick.clone());
                                if let Some(code) = onclick {
                                    self.js_context.eval_script(&mut self.dom_tree, &code);
                                    if let Some(msg) = self.js_context.alerts.last() {
                                        self.alert_dialog = Some(msg.clone());
                                    }
                                    self.update_layout();
                                }
                                return;
                            }
                        }
                    }
                }
            }

            // Check if clicked scrollbar track (rightmost 16px)
            let scrollbar_x = w as isize - 20;
            if local_x >= scrollbar_x && local_x < w as isize - 4 {
                if local_y < view_top + 16 {
                    // Up arrow
                    self.scroll_offset = self.scroll_offset.saturating_sub(30);
                } else if local_y > view_bottom - 16 {
                    // Down arrow
                    self.scroll_offset = self.scroll_offset.saturating_add(30);
                } else {
                    // Click in track
                    let track_h = (view_bottom - view_top - 32).max(1) as usize;
                    let click_rel = (local_y - (view_top + 16)) as usize;
                    let pct = click_rel * self.layout.total_height / track_h;
                    self.scroll_offset = pct;
                }
                return;
            }
        }
    }

    fn on_mouse_scroll(&mut self, _local_x: isize, _local_y: isize, delta: i32) -> bool {
        let step = 40 * delta.abs() as usize;
        let max_scroll = self.layout.total_height.saturating_sub(60);
        if delta > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(step);
        } else if delta < 0 {
            self.scroll_offset = (self.scroll_offset + step).min(max_scroll);
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
        self.last_width = client_w;
        self.last_height = client_h;

        // Background
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::RETRO_FACE);

        // 1. Top Toolbar (y: 2..32)
        let bar_y = client_y + 4;
        fb.draw_button(client_x + 4, bar_y, 24, 24, false);
        fb.draw_string(client_x + 11, bar_y + 4, "<", Color::BLACK);

        fb.draw_button(client_x + 32, bar_y, 24, 24, false);
        fb.draw_string(client_x + 39, bar_y + 4, ">", Color::BLACK);

        fb.draw_button(client_x + 60, bar_y, 24, 24, false);
        fb.draw_string(client_x + 67, bar_y + 4, "R", Color::BLACK);

        // Address Bar
        let addr_w = client_w.saturating_sub(148);
        fb.draw_sunken_panel(client_x + 92, bar_y, addr_w, 24);
        fb.fill_rect(client_x + 94, bar_y + 2, addr_w.saturating_sub(4), 20, Color::WHITE);

        let max_chars = addr_w.saturating_sub(12) / FONT_WIDTH;
        let disp_url = if self.url_input.len() > max_chars {
            &self.url_input[..max_chars]
        } else {
            &self.url_input
        };
        fb.draw_string(client_x + 98, bar_y + 4, disp_url, Color::BLACK);

        if self.address_bar_focused && self.cursor_visible && self.cursor_col <= max_chars {
            let cx = client_x + 98 + (self.cursor_col * FONT_WIDTH) as isize;
            fb.fill_rect(cx, bar_y + 4, 1, FONT_HEIGHT, Color::BLACK);
        }

        // Go Button
        let go_x = client_x + client_w as isize - 50;
        fb.draw_button(go_x, bar_y, 44, 24, false);
        fb.draw_string(go_x + 14, bar_y + 4, "Go", Color::BLACK);

        // 2. Optional Warning Banner
        let mut content_top = client_y + 34;
        if let Some(ref warn) = self.warning_banner {
            fb.fill_rect(client_x + 4, content_top, client_w.saturating_sub(8), 20, Color::from_rgb(255, 230, 150));
            fb.draw_sunken_panel(client_x + 4, content_top, client_w.saturating_sub(8), 20);
            fb.draw_string(client_x + 8, content_top + 2, warn, Color::from_rgb(140, 40, 0));
            content_top += 22;
        }

        // 3. Document Viewport
        let status_h = 22;
        let view_w = client_w.saturating_sub(8);
        let view_h = (client_y + client_h as isize - status_h as isize - content_top).max(20) as usize;

        fb.draw_sunken_panel(client_x + 4, content_top, view_w, view_h);
        fb.fill_rect(client_x + 6, content_top + 2, view_w.saturating_sub(20), view_h.saturating_sub(4), Color::WHITE);

        // Clip and render layout items
        let doc_viewport_h = view_h.saturating_sub(4);
        for item in &self.layout.items {
            match item {
                LayoutItem::Text { x, y, text, color, is_bold, scale } => {
                    if *y + 20 * scale < self.scroll_offset || *y > self.scroll_offset + doc_viewport_h {
                        continue;
                    }
                    let draw_y = content_top + 4 + (*y as isize - self.scroll_offset as isize);
                    let draw_x = client_x + 8 + *x as isize;

                    if *scale == 2 {
                        // Prominent 2x heading
                        fb.draw_string(draw_x, draw_y, text, *color);
                        fb.draw_string(draw_x + 1, draw_y, text, *color);
                    } else if *is_bold {
                        fb.draw_string(draw_x, draw_y, text, *color);
                        fb.draw_string(draw_x + 1, draw_y, text, *color);
                    } else {
                        fb.draw_string(draw_x, draw_y, text, *color);
                    }

                    // Underline links
                    if *color == Color::from_rgb(0, 80, 210) {
                        let text_w = text.len() * FONT_WIDTH * scale;
                        fb.fill_rect(draw_x, draw_y + (FONT_HEIGHT * scale) as isize, text_w, 1, *color);
                    }
                }
                LayoutItem::HorizontalRule { y, width } => {
                    if *y < self.scroll_offset || *y > self.scroll_offset + doc_viewport_h {
                        continue;
                    }
                    let draw_y = content_top + 4 + (*y as isize - self.scroll_offset as isize);
                    fb.draw_groove(client_x + 12, draw_y, *width, 2);
                }
                LayoutItem::Bullet { x, y } => {
                    if *y < self.scroll_offset || *y > self.scroll_offset + doc_viewport_h {
                        continue;
                    }
                    let draw_y = content_top + 4 + (*y as isize - self.scroll_offset as isize);
                    let draw_x = client_x + 8 + *x as isize;
                    fb.fill_rect(draw_x, draw_y + 4, 4, 4, Color::BLACK);
                }
                LayoutItem::ImagePlaceholder { x, y, width, height, alt } => {
                    if *y + height < self.scroll_offset || *y > self.scroll_offset + doc_viewport_h {
                        continue;
                    }
                    let draw_y = content_top + 4 + (*y as isize - self.scroll_offset as isize);
                    let draw_x = client_x + 8 + *x as isize;
                    fb.draw_sunken_panel(draw_x, draw_y, *width, *height);
                    fb.fill_rect(draw_x + 2, draw_y + 2, width.saturating_sub(4), height.saturating_sub(4), Color::from_rgb(230, 230, 230));
                    let alt_disp = if alt.len() > 6 { &alt[..6] } else { alt };
                    fb.draw_string(draw_x + 4, draw_y + 16, alt_disp, Color::from_rgb(100, 100, 100));
                }
                LayoutItem::Box { x, y, width, height, background, border_color, border_width } => {
                    if *y + height < self.scroll_offset || *y > self.scroll_offset + doc_viewport_h {
                        continue;
                    }
                    let draw_y = content_top + 4 + (*y as isize - self.scroll_offset as isize);
                    let draw_x = client_x + 8 + *x as isize;
                    if let Some(bg) = background {
                        fb.fill_rect(draw_x, draw_y, *width, *height, *bg);
                    }
                    if *border_width > 0 {
                        let bc = border_color.unwrap_or(Color::from_rgb(180, 180, 180));
                        fb.draw_rect(draw_x, draw_y, *width, *height, bc);
                    }
                }
                LayoutItem::Button { x, y, width, height, text, bg_color, text_color, .. } => {
                    if *y + height < self.scroll_offset || *y > self.scroll_offset + doc_viewport_h {
                        continue;
                    }
                    let draw_y = content_top + 4 + (*y as isize - self.scroll_offset as isize);
                    let draw_x = client_x + 8 + *x as isize;
                    if let Some(bg) = bg_color {
                        fb.fill_rect(draw_x, draw_y, *width, *height, *bg);
                        fb.draw_rect(draw_x, draw_y, *width, *height, Color::from_rgb(120, 120, 120));
                    } else {
                        fb.draw_button(draw_x, draw_y, *width, *height, false);
                    }
                    let tx = draw_x + (width.saturating_sub(text.len() * FONT_WIDTH)) as isize / 2;
                    let ty = draw_y + (height.saturating_sub(FONT_HEIGHT)) as isize / 2;
                    fb.draw_string(tx, ty, text, *text_color);
                }
            }
        }

        // Viewport scrollbar
        let sb_x = client_x + 4 + view_w as isize - 16;
        fb.fill_rect(sb_x, content_top + 2, 14, view_h.saturating_sub(4), Color::RETRO_LIGHT);
        fb.draw_button(sb_x, content_top + 2, 14, 14, false);
        fb.draw_string(sb_x + 3, content_top + 1, "^", Color::BLACK);

        fb.draw_button(sb_x, content_top + view_h as isize - 16, 14, 14, false);
        fb.draw_string(sb_x + 3, content_top + view_h as isize - 17, "v", Color::BLACK);

        // Scrollbar thumb
        let track_h = view_h.saturating_sub(36);
        let max_scroll = self.layout.total_height.max(1);
        let thumb_h = (track_h * doc_viewport_h / max_scroll).clamp(16, track_h);
        let thumb_y = content_top + 16 + (self.scroll_offset * (track_h - thumb_h) / max_scroll) as isize;
        fb.draw_button(sb_x, thumb_y, 14, thumb_h, false);

        // 4. Status Bar at Bottom
        let status_y = client_y + client_h as isize - status_h as isize;
        fb.draw_sunken_panel(client_x + 4, status_y, client_w.saturating_sub(8), 20);
        fb.draw_string(client_x + 8, status_y + 2, &self.status_text, Color::BLACK);

        // 5. JavaScript Alert Modal Dialog
        if let Some(ref alert_msg) = self.alert_dialog {
            let dlg_w = 320;
            let dlg_h = 130;
            let dlg_x = client_x + (client_w as isize - dlg_w as isize) / 2;
            let dlg_y = client_y + (client_h as isize - dlg_h as isize) / 2;

            // Shadow & window background
            fb.fill_rect(dlg_x + 4, dlg_y + 4, dlg_w, dlg_h, Color::from_rgb(60, 60, 60));
            fb.draw_raised_panel(dlg_x, dlg_y, dlg_w, dlg_h);

            // Title bar
            fb.fill_rect(dlg_x + 3, dlg_y + 3, dlg_w - 6, 20, Color::from_rgb(0, 0, 128));
            fb.draw_string(dlg_x + 8, dlg_y + 5, "JavaScript Alert", Color::WHITE);

            // Alert icon / message
            fb.draw_string(dlg_x + 16, dlg_y + 40, alert_msg, Color::BLACK);

            // OK button
            let ok_w = 64;
            let ok_h = 24;
            let ok_x = dlg_x + (dlg_w as isize - ok_w as isize) / 2;
            let ok_y = dlg_y + dlg_h as isize - 36;
            fb.draw_button(ok_x, ok_y, ok_w, ok_h, false);
            fb.draw_string(ok_x + 22, ok_y + 4, "OK", Color::BLACK);
        }
    }
}

use alloc::string::String;
use alloc::vec::Vec;
use crate::gui::color::Color;
use crate::gui::font::FONT_WIDTH;

#[derive(Clone, Debug)]
pub struct LayoutLink {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub href: String,
}

#[derive(Clone, Debug)]
pub enum LayoutItem {
    Text {
        x: usize,
        y: usize,
        text: String,
        color: Color,
        is_bold: bool,
        scale: usize, // 1 for normal/h3, 2 for h1
    },
    HorizontalRule {
        y: usize,
        width: usize,
    },
    Bullet {
        x: usize,
        y: usize,
    },
    ImagePlaceholder {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        alt: String,
    },
}

pub struct DocumentLayout {
    pub title: String,
    pub items: Vec<LayoutItem>,
    pub links: Vec<LayoutLink>,
    pub total_height: usize,
}

pub fn decode_entities(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi) = s[i..].find(';') {
                let entity = &s[i..i + semi + 1];
                match entity {
                    "&nbsp;" => out.push(' '),
                    "&lt;" => out.push('<'),
                    "&gt;" => out.push('>'),
                    "&amp;" => out.push('&'),
                    "&quot;" => out.push('"'),
                    "&apos;" => out.push('\''),
                    _ => out.push_str(entity),
                }
                i += semi + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[derive(Clone, Debug)]
enum Token<'a> {
    StartTag { name: &'a str, attrs: &'a str },
    EndTag(&'a str),
    Text(&'a str),
}

fn tokenize_html<'a>(html: &'a str) -> Vec<Token<'a>> {
    let mut tokens = Vec::new();
    let mut idx = 0;
    let bytes = html.as_bytes();

    while idx < bytes.len() {
        if bytes[idx] == b'<' {
            // Find closing '>'
            if let Some(end) = html[idx..].find('>') {
                let tag_content = &html[idx + 1..idx + end].trim();
                if tag_content.starts_with('/') {
                    let name = tag_content[1..].trim();
                    tokens.push(Token::EndTag(name));
                } else if tag_content.starts_with('!') {
                    // Comment or doctype, skip
                } else {
                    let parts: Vec<&str> = tag_content.splitn(2, |c: char| c.is_whitespace()).collect();
                    let name = parts[0];
                    let attrs = if parts.len() > 1 { parts[1] } else { "" };
                    tokens.push(Token::StartTag { name, attrs });
                }
                idx += end + 1;
                continue;
            }
        }

        // Text content until next '<'
        let end = html[idx..].find('<').unwrap_or(bytes.len() - idx);
        let text = &html[idx..idx + end];
        if !text.is_empty() {
            tokens.push(Token::Text(text));
        }
        idx += end;
    }

    tokens
}

fn extract_attr(attrs: &str, key: &str) -> Option<String> {
    let pattern = alloc::format!("{}=", key);
    let lower_attrs = attrs.to_lowercase();
    let pos = lower_attrs.find(&pattern)?;
    let rem = attrs[pos + pattern.len()..].trim_start();
    if rem.starts_with('"') {
        let closing = rem[1..].find('"')?;
        Some(String::from(&rem[1..1 + closing]))
    } else if rem.starts_with('\'') {
        let closing = rem[1..].find('\'')?;
        Some(String::from(&rem[1..1 + closing]))
    } else {
        let end = rem.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(rem.len());
        Some(String::from(&rem[..end]))
    }
}

pub fn layout_html(html: &str, max_width: usize) -> DocumentLayout {
    let tokens = tokenize_html(html);
    let mut items = Vec::new();
    let mut links = Vec::new();
    let mut doc_title = String::from("Mouros Browser");

    let margin_left = 12;
    let margin_right = 16;
    let usable_width = max_width.saturating_sub(margin_left + margin_right).max(100);

    let mut cursor_x = margin_left;
    let mut cursor_y = 12;

    let mut in_head = false;
    let mut in_title = false;
    let mut in_bold = false;
    let mut in_italic = false;
    let mut current_scale = 1;
    let mut current_link: Option<String> = None;
    let mut in_ul = false;
    let mut in_ol = false;
    let mut list_index = 1;

    for token in tokens {
        match token {
            Token::StartTag { name, attrs } => {
                let tag = name.to_lowercase();
                match tag.as_str() {
                    "head" => in_head = true,
                    "title" => in_title = true,
                    "h1" => {
                        if cursor_x > margin_left {
                            cursor_y += 18 * current_scale;
                            cursor_x = margin_left;
                        }
                        cursor_y += 12;
                        current_scale = 2;
                        in_bold = true;
                    }
                    "h2" => {
                        if cursor_x > margin_left {
                            cursor_y += 18 * current_scale;
                            cursor_x = margin_left;
                        }
                        cursor_y += 10;
                        current_scale = 1;
                        in_bold = true;
                    }
                    "h3" => {
                        if cursor_x > margin_left {
                            cursor_y += 18 * current_scale;
                            cursor_x = margin_left;
                        }
                        cursor_y += 8;
                        current_scale = 1;
                        in_bold = true;
                    }
                    "p" => {
                        if cursor_x > margin_left {
                            cursor_y += 18 * current_scale;
                            cursor_x = margin_left;
                        }
                        cursor_y += 8;
                    }
                    "br" => {
                        cursor_y += 18 * current_scale;
                        cursor_x = margin_left;
                    }
                    "hr" => {
                        if cursor_x > margin_left {
                            cursor_y += 18 * current_scale;
                            cursor_x = margin_left;
                        }
                        cursor_y += 6;
                        items.push(LayoutItem::HorizontalRule {
                            y: cursor_y,
                            width: usable_width,
                        });
                        cursor_y += 10;
                    }
                    "b" | "strong" => in_bold = true,
                    "i" | "em" => in_italic = true,
                    "a" => {
                        if let Some(href) = extract_attr(attrs, "href") {
                            current_link = Some(href);
                        }
                    }
                    "ul" => {
                        in_ul = true;
                        cursor_y += 6;
                        cursor_x = margin_left;
                    }
                    "ol" => {
                        in_ol = true;
                        list_index = 1;
                        cursor_y += 6;
                        cursor_x = margin_left;
                    }
                    "li" => {
                        if cursor_x > margin_left {
                            cursor_y += 18 * current_scale;
                        }
                        if in_ul {
                            items.push(LayoutItem::Bullet {
                                x: margin_left + 4,
                                y: cursor_y + 4,
                            });
                            cursor_x = margin_left + 16;
                        } else if in_ol {
                            let num_str = alloc::format!("{}. ", list_index);
                            list_index += 1;
                            items.push(LayoutItem::Text {
                                x: margin_left + 4,
                                y: cursor_y,
                                text: num_str.clone(),
                                color: Color::from_rgb(40, 40, 40),
                                is_bold: true,
                                scale: 1,
                            });
                            cursor_x = margin_left + 4 + num_str.len() * FONT_WIDTH;
                        } else {
                            cursor_x = margin_left + 12;
                        }
                    }
                    "img" => {
                        let alt = extract_attr(attrs, "alt").unwrap_or_else(|| String::from("Image"));
                        let img_w = 64;
                        let img_h = 48;
                        if cursor_x + img_w > margin_left + usable_width {
                            cursor_y += 18 * current_scale;
                            cursor_x = margin_left;
                        }
                        items.push(LayoutItem::ImagePlaceholder {
                            x: cursor_x,
                            y: cursor_y,
                            width: img_w,
                            height: img_h,
                            alt,
                        });
                        cursor_x += img_w + 8;
                    }
                    _ => {}
                }
            }
            Token::EndTag(name) => {
                let tag = name.to_lowercase();
                match tag.as_str() {
                    "head" => in_head = false,
                    "title" => in_title = false,
                    "h1" | "h2" | "h3" => {
                        cursor_y += 20 * current_scale;
                        cursor_x = margin_left;
                        current_scale = 1;
                        in_bold = false;
                    }
                    "p" => {
                        cursor_y += 18;
                        cursor_x = margin_left;
                    }
                    "b" | "strong" => in_bold = false,
                    "i" | "em" => in_italic = false,
                    "a" => current_link = None,
                    "ul" => {
                        in_ul = false;
                        cursor_y += 8;
                        cursor_x = margin_left;
                    }
                    "ol" => {
                        in_ol = false;
                        cursor_y += 8;
                        cursor_x = margin_left;
                    }
                    "li" => {
                        cursor_y += 18;
                        cursor_x = margin_left;
                    }
                    _ => {}
                }
            }
            Token::Text(raw_text) => {
                if in_head && in_title {
                    doc_title = decode_entities(raw_text.trim());
                    continue;
                }
                if in_head {
                    continue;
                }

                let decoded = decode_entities(raw_text);
                let words: Vec<&str> = decoded.split_whitespace().collect();
                if words.is_empty() {
                    continue;
                }

                let char_w = FONT_WIDTH * current_scale;
                let line_h = 18 * current_scale;

                for (w_idx, word) in words.iter().enumerate() {
                    let word_len = word.len() * char_w;
                    let space_len = if w_idx > 0 || cursor_x > margin_left { char_w } else { 0 };

                    if cursor_x + space_len + word_len > margin_left + usable_width && cursor_x > margin_left {
                        cursor_y += line_h;
                        cursor_x = if in_ul || in_ol { margin_left + 16 } else { margin_left };
                    } else if space_len > 0 && cursor_x > margin_left {
                        cursor_x += space_len;
                    }

                    let color = if current_link.is_some() {
                        Color::from_rgb(0, 80, 210) // Blue link
                    } else if in_bold {
                        Color::from_rgb(10, 10, 10)
                    } else if in_italic {
                        Color::from_rgb(50, 50, 50)
                    } else {
                        Color::from_rgb(30, 30, 30)
                    };

                    let start_x = cursor_x;
                    items.push(LayoutItem::Text {
                        x: start_x,
                        y: cursor_y,
                        text: String::from(*word),
                        color,
                        is_bold: in_bold,
                        scale: current_scale,
                    });

                    if let Some(ref href) = current_link {
                        links.push(LayoutLink {
                            x: start_x,
                            y: cursor_y,
                            width: word_len,
                            height: line_h,
                            href: href.clone(),
                        });
                    }

                    cursor_x += word_len;
                }
            }
        }
    }

    let total_height = cursor_y + 40;

    DocumentLayout {
        title: doc_title,
        items,
        links,
        total_height,
    }
}

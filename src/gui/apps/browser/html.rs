use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::gui::apps::browser::css::{compute_styles, parse_stylesheet, ComputedStyle, CssRule, DisplayMode};
use crate::gui::apps::browser::dom::DomTree;
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
pub enum ClickableAction {
    Navigate(String),
    JavaScript(String),
    ButtonClick(usize),
}

#[derive(Clone, Debug)]
pub struct LayoutClickable {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub action: ClickableAction,
}

#[derive(Clone, Debug)]
pub enum LayoutItem {
    Text {
        x: usize,
        y: usize,
        text: String,
        color: Color,
        is_bold: bool,
        scale: usize,
    },
    Box {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        background: Option<Color>,
        border_color: Option<Color>,
        border_width: usize,
    },
    Button {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        text: String,
        bg_color: Option<Color>,
        text_color: Color,
        node_id: usize,
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
    pub clickables: Vec<LayoutClickable>,
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
            if let Some(end) = html[idx..].find('>') {
                let tag_content = html[idx + 1..idx + end].trim();
                if tag_content.starts_with('/') {
                    let name = tag_content[1..].trim();
                    tokens.push(Token::EndTag(name));
                } else if tag_content.starts_with('!') {
                    // Comment or doctype, skip
                } else {
                    let parts: Vec<&str> = tag_content.splitn(2, |c: char| c.is_whitespace()).collect();
                    let name = parts[0].trim();
                    let attrs = if parts.len() > 1 { parts[1].trim() } else { "" };
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

fn parse_attributes(attrs_str: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let bytes = attrs_str.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        let key_start = i;
        while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' && bytes[i] != b'>' && bytes[i] != b'/' {
            i += 1;
        }
        let key = &attrs_str[key_start..i];
        if key.is_empty() {
            i += 1;
            continue;
        }

        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let mut val = String::new();
        if i < len && bytes[i] == b'=' {
            i += 1; // skip '='
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let val_start = i;
                while i < len && bytes[i] != quote {
                    i += 1;
                }
                val = String::from(&attrs_str[val_start..i]);
                if i < len {
                    i += 1; // skip closing quote
                }
            } else {
                let val_start = i;
                while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                val = String::from(&attrs_str[val_start..i]);
            }
        }

        attrs.push((String::from(key).to_lowercase(), val));
    }

    attrs
}

pub struct ParsedDocument {
    pub tree: DomTree,
    pub stylesheets: Vec<CssRule>,
    pub scripts: Vec<String>,
}

pub fn parse_html(html: &str) -> ParsedDocument {
    let mut tree = DomTree::new();
    let mut stylesheets = Vec::new();
    let mut scripts = Vec::new();

    let tokens = tokenize_html(html);
    let mut stack: Vec<usize> = alloc::vec![tree.root];

    for token in tokens {
        match token {
            Token::StartTag { name, attrs } => {
                let tag_lower = name.to_lowercase();
                let clean_tag = tag_lower.trim_end_matches('/');

                let node_id = tree.create_element(clean_tag);
                let parsed_attrs = parse_attributes(attrs);
                for (k, v) in parsed_attrs {
                    tree.set_attribute(node_id, &k, &v);
                }

                if let Some(&parent_id) = stack.last() {
                    tree.append_child(parent_id, node_id);
                }

                let is_self_closing = attrs.ends_with('/')
                    || matches!(
                        clean_tag,
                        "br" | "hr" | "img" | "input" | "meta" | "link"
                    );

                if !is_self_closing {
                    stack.push(node_id);
                }
            }
            Token::EndTag(name) => {
                let tag_lower = name.to_lowercase();
                // Pop back to matching element
                if let Some(pos) = stack.iter().rposition(|&id| {
                    if let Some(n) = tree.get_node(id) {
                        n.tag == tag_lower
                    } else {
                        false
                    }
                }) {
                    stack.truncate(pos);
                }
            }
            Token::Text(raw_text) => {
                let current_parent = *stack.last().unwrap_or(&tree.root);
                let parent_tag = tree
                    .get_node(current_parent)
                    .map(|n| n.tag.clone())
                    .unwrap_or_default();

                if parent_tag == "style" {
                    stylesheets.extend(parse_stylesheet(raw_text));
                } else if parent_tag == "script" {
                    scripts.push(String::from(raw_text));
                } else {
                    let decoded = decode_entities(raw_text);
                    let text_id = tree.create_text_node(&decoded);
                    tree.append_child(current_parent, text_id);
                }
            }
        }
    }

    ParsedDocument {
        tree,
        stylesheets,
        scripts,
    }
}

pub fn compute_layout(
    tree: &DomTree,
    styles: &[ComputedStyle],
    max_width: usize,
) -> DocumentLayout {
    let mut items = Vec::new();
    let mut links = Vec::new();
    let mut clickables = Vec::new();

    // Extract page title from <title> tag if present
    let mut doc_title = String::from("Mouros Browser");
    for node in &tree.nodes {
        if node.tag == "title" {
            let t = tree.get_inner_text(node.id).trim().to_string();
            if !t.is_empty() {
                doc_title = t;
            }
            break;
        }
    }

    let margin_left = 12;
    let margin_right = 16;
    let usable_width = max_width.saturating_sub(margin_left + margin_right).max(100);

    let mut cursor_x = margin_left;
    let mut cursor_y = 12;
    let mut list_index = 1;

    // Helper context for recursive DOM layout traversal
    layout_node(
        tree,
        styles,
        tree.root,
        margin_left,
        usable_width,
        &mut cursor_x,
        &mut cursor_y,
        &mut list_index,
        &mut items,
        &mut links,
        &mut clickables,
    );

    let total_height = cursor_y + 40;

    DocumentLayout {
        title: doc_title,
        items,
        links,
        clickables,
        total_height,
    }
}

fn layout_node(
    tree: &DomTree,
    styles: &[ComputedStyle],
    node_id: usize,
    margin_left: usize,
    usable_width: usize,
    cursor_x: &mut usize,
    cursor_y: &mut usize,
    list_index: &mut usize,
    items: &mut Vec<LayoutItem>,
    links: &mut Vec<LayoutLink>,
    clickables: &mut Vec<LayoutClickable>,
) {
    let node = match tree.get_node(node_id) {
        Some(n) => n,
        None => return,
    };

    let style = styles.get(node_id).cloned().unwrap_or_default();
    if style.display == DisplayMode::None {
        return;
    }

    // Skip root or head nodes directly
    if node.tag == "#document" || node.tag == "head" {
        for &child_id in &node.children {
            layout_node(
                tree,
                styles,
                child_id,
                margin_left,
                usable_width,
                cursor_x,
                cursor_y,
                list_index,
                items,
                links,
                clickables,
            );
        }
        return;
    }

    if node.is_text() {
        let text = &node.text;
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return;
        }

        let char_w = FONT_WIDTH * style.font_scale;
        let line_h = 18 * style.font_scale;

        for (w_idx, word) in words.iter().enumerate() {
            let word_len = word.len() * char_w;
            let space_len = if w_idx > 0 || *cursor_x > margin_left { char_w } else { 0 };

            if *cursor_x + space_len + word_len > margin_left + usable_width && *cursor_x > margin_left {
                *cursor_y += line_h;
                *cursor_x = margin_left;
            } else if space_len > 0 && *cursor_x > margin_left {
                *cursor_x += space_len;
            }

            let start_x = *cursor_x;
            items.push(LayoutItem::Text {
                x: start_x,
                y: *cursor_y,
                text: String::from(*word),
                color: style.color,
                is_bold: style.bold,
                scale: style.font_scale,
            });

            *cursor_x += word_len;
        }
        return;
    }

    // Container / Element nodes
    let is_block = style.display == DisplayMode::Block;
    if is_block && *cursor_x > margin_left {
        *cursor_y += 18;
        *cursor_x = margin_left;
    }

    *cursor_y += style.margin_top;

    let box_start_y = *cursor_y;
    let box_start_x = *cursor_x + style.margin_left;
    let box_width = style
        .width
        .unwrap_or(usable_width.saturating_sub(style.margin_left + style.margin_right));

    let content_margin_left = box_start_x + style.padding_left;
    let content_usable_width = box_width.saturating_sub(style.padding_left + style.padding_right);

    // Save item insert index so background box is placed behind children
    let box_item_idx = items.len();

    *cursor_y += style.padding_top;
    *cursor_x = content_margin_left;

    // Special element handling
    match node.tag.as_str() {
        "hr" => {
            *cursor_y += 4;
            items.push(LayoutItem::HorizontalRule {
                y: *cursor_y,
                width: usable_width,
            });
            *cursor_y += 10;
            *cursor_x = margin_left;
            return;
        }
        "br" => {
            *cursor_y += 18 * style.font_scale;
            *cursor_x = margin_left;
            return;
        }
        "button" => {
            let btn_text = tree.get_inner_text(node_id);
            let text_to_show = if btn_text.trim().is_empty() {
                String::from("Button")
            } else {
                btn_text.trim().to_string()
            };
            let text_len = text_to_show.len() * FONT_WIDTH;
            let btn_w = style.width.unwrap_or(text_len + style.padding_left + style.padding_right + 12);
            let btn_h = style.height.unwrap_or(24 + style.padding_top + style.padding_bottom);

            if *cursor_x + btn_w > margin_left + usable_width && *cursor_x > margin_left {
                *cursor_y += 28;
                *cursor_x = margin_left;
            }

            let btn_x = *cursor_x;
            let btn_y = *cursor_y;

            items.push(LayoutItem::Button {
                x: btn_x,
                y: btn_y,
                width: btn_w,
                height: btn_h,
                text: text_to_show,
                bg_color: style.background_color,
                text_color: style.color,
                node_id,
            });

            let action = if let Some(ref js_code) = node.onclick {
                ClickableAction::JavaScript(js_code.clone())
            } else {
                ClickableAction::ButtonClick(node_id)
            };

            clickables.push(LayoutClickable {
                x: btn_x,
                y: btn_y,
                width: btn_w,
                height: btn_h,
                action,
            });

            *cursor_x += btn_w + style.margin_right + 6;
            return;
        }
        "img" => {
            let alt = tree.get_attribute(node_id, "alt").unwrap_or("Image").to_string();
            let img_w = style.width.unwrap_or(64);
            let img_h = style.height.unwrap_or(48);

            if *cursor_x + img_w > margin_left + usable_width && *cursor_x > margin_left {
                *cursor_y += 18;
                *cursor_x = margin_left;
            }

            items.push(LayoutItem::ImagePlaceholder {
                x: *cursor_x,
                y: *cursor_y,
                width: img_w,
                height: img_h,
                alt,
            });

            *cursor_x += img_w + 8;
            return;
        }
        "li" => {
            let is_in_ol = node.parent.and_then(|p| tree.get_node(p)).map(|p| p.tag == "ol").unwrap_or(false);
            if is_in_ol {
                let num_str = format!("{}. ", *list_index);
                *list_index += 1;
                items.push(LayoutItem::Text {
                    x: *cursor_x,
                    y: *cursor_y,
                    text: num_str.clone(),
                    color: style.color,
                    is_bold: true,
                    scale: 1,
                });
                *cursor_x += num_str.len() * FONT_WIDTH;
            } else {
                items.push(LayoutItem::Bullet {
                    x: *cursor_x,
                    y: *cursor_y + 4,
                });
                *cursor_x += 16;
            }
        }
        "ol" => {
            *list_index = 1;
        }
        _ => {}
    }

    // Traverse children
    let link_start_x = *cursor_x;
    let link_start_y = *cursor_y;

    for &child_id in &node.children {
        layout_node(
            tree,
            styles,
            child_id,
            content_margin_left,
            content_usable_width,
            cursor_x,
            cursor_y,
            list_index,
            items,
            links,
            clickables,
        );
    }

    // If node was a link <a>, register its clickable area
    if node.tag == "a" {
        if let Some(href) = tree.get_attribute(node_id, "href") {
            let w = cursor_x.saturating_sub(link_start_x).max(FONT_WIDTH);
            let h = 18 * style.font_scale;
            links.push(LayoutLink {
                x: link_start_x,
                y: link_start_y,
                width: w,
                height: h,
                href: String::from(href),
            });
            clickables.push(LayoutClickable {
                x: link_start_x,
                y: link_start_y,
                width: w,
                height: h,
                action: ClickableAction::Navigate(String::from(href)),
            });
        }
    }

    // If node had onclick and is not a button, register clickable
    if let Some(ref js_code) = node.onclick {
        if node.tag != "button" {
            let w = cursor_x.saturating_sub(link_start_x).max(FONT_WIDTH * 2);
            let h = cursor_y.saturating_sub(link_start_y).max(18);
            clickables.push(LayoutClickable {
                x: link_start_x,
                y: link_start_y,
                width: w,
                height: h,
                action: ClickableAction::JavaScript(js_code.clone()),
            });
        }
    }

    // If inline content was rendered on the current line, advance to next line
    if is_block && *cursor_x > content_margin_left {
        *cursor_y += 18 * style.font_scale;
        *cursor_x = content_margin_left;
    }

    *cursor_y += style.padding_bottom;

    // Insert background/border box if styled
    if style.background_color.is_some() || style.border_width > 0 {
        let box_h = cursor_y.saturating_sub(box_start_y).max(18);
        items.insert(
            box_item_idx,
            LayoutItem::Box {
                x: box_start_x,
                y: box_start_y,
                width: box_width,
                height: box_h,
                background: style.background_color,
                border_color: style.border_color,
                border_width: style.border_width,
            },
        );
    }

    *cursor_y += style.margin_bottom;

    if is_block {
        *cursor_x = margin_left;
    }
}

pub fn layout_html(html: &str, max_width: usize) -> DocumentLayout {
    let doc = parse_html(html);
    let styles = compute_styles(&doc.tree, &doc.stylesheets);
    compute_layout(&doc.tree, &styles, max_width)
}

#[test_case]
fn test_welcome_page_scripts_and_click() {
    use crate::gui::apps::browser::js::{JsContext, JsValue};
    let html = "<html><head><script>var count = 0; function inc() { count = count + 1; document.getElementById('c').innerText = 'Count: ' + count; }</script></head><body><button onclick=\"inc()\">+</button><span id=\"c\">0</span></body></html>";
    let doc = parse_html(html);
    assert_eq!(doc.scripts.len(), 1);
    let mut ctx = JsContext::new();
    let mut tree = doc.tree;
    for s in &doc.scripts {
        ctx.eval_script(&mut tree, s);
    }
    assert!(ctx.get_var("inc") != JsValue::Undefined);
    ctx.eval_script(&mut tree, "inc()");
    assert_eq!(tree.get_inner_text(tree.get_element_by_id("c").unwrap()), "Count: 1");

    // Also test layout of the welcome page
    let app = crate::gui::apps::browser::BrowserApp::new();
    assert_eq!(app.layout.clickables.len(), 7);
}



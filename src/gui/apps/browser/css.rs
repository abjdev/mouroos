use alloc::string::String;
use alloc::vec::Vec;
use crate::gui::apps::browser::dom::{DomNode, DomTree};
use crate::gui::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayMode {
    Block,
    Inline,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
pub struct ComputedStyle {
    pub display: DisplayMode,
    pub color: Color,
    pub background_color: Option<Color>,
    pub font_size: usize,
    pub font_scale: usize,
    pub bold: bool,
    pub italic: bool,
    pub text_align: TextAlign,
    pub margin_top: usize,
    pub margin_bottom: usize,
    pub margin_left: usize,
    pub margin_right: usize,
    pub padding_top: usize,
    pub padding_bottom: usize,
    pub padding_left: usize,
    pub padding_right: usize,
    pub border_width: usize,
    pub border_color: Option<Color>,
    pub width: Option<usize>,
    pub height: Option<usize>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        ComputedStyle {
            display: DisplayMode::Block,
            color: Color::from_rgb(30, 30, 30),
            background_color: None,
            font_size: 12,
            font_scale: 1,
            bold: false,
            italic: false,
            text_align: TextAlign::Left,
            margin_top: 0,
            margin_bottom: 0,
            margin_left: 0,
            margin_right: 0,
            padding_top: 0,
            padding_bottom: 0,
            padding_left: 0,
            padding_right: 0,
            border_width: 0,
            border_color: None,
            width: None,
            height: None,
        }
    }
}

pub fn parse_color(val: &str) -> Option<Color> {
    let s = val.trim().to_lowercase();
    if s == "transparent" {
        return None;
    }
    match s.as_str() {
        "black" => return Some(Color::BLACK),
        "white" => return Some(Color::WHITE),
        "red" => return Some(Color::from_rgb(220, 20, 20)),
        "green" => return Some(Color::from_rgb(20, 160, 20)),
        "blue" => return Some(Color::from_rgb(20, 80, 220)),
        "yellow" => return Some(Color::from_rgb(240, 210, 0)),
        "orange" => return Some(Color::from_rgb(240, 120, 0)),
        "cyan" => return Some(Color::from_rgb(0, 190, 210)),
        "magenta" | "purple" => return Some(Color::from_rgb(160, 32, 240)),
        "gray" | "grey" => return Some(Color::from_rgb(128, 128, 128)),
        "lightgray" | "lightgrey" => return Some(Color::from_rgb(211, 211, 211)),
        "darkgray" | "darkgrey" => return Some(Color::from_rgb(60, 60, 60)),
        "navy" => return Some(Color::from_rgb(0, 0, 128)),
        "teal" => return Some(Color::from_rgb(0, 128, 128)),
        "maroon" => return Some(Color::from_rgb(128, 0, 0)),
        "silver" => return Some(Color::from_rgb(192, 192, 192)),
        _ => {}
    }

    if s.starts_with('#') {
        let hex = &s[1..];
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color::from_rgb(r, g, b));
        } else if hex.len() >= 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::from_rgb(r, g, b));
        }
    }

    if s.starts_with("rgb(") && s.ends_with(')') {
        let inner = &s[4..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            return Some(Color::from_rgb(r, g, b));
        }
    }

    None
}

fn parse_pixel_value(val: &str) -> Option<usize> {
    let s = val.trim();
    let num_str = s.trim_end_matches("px").trim();
    num_str.parse::<usize>().ok()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorKind {
    Universal,
    Tag(String),
    Class(String),
    Id(String),
}

#[derive(Clone, Debug)]
pub struct Selector {
    pub kind: SelectorKind,
    pub specificity: usize,
}

impl Selector {
    pub fn parse(s: &str) -> Option<Self> {
        let raw = s.trim();
        if raw == "*" {
            Some(Selector {
                kind: SelectorKind::Universal,
                specificity: 0,
            })
        } else if raw.starts_with('#') {
            Some(Selector {
                kind: SelectorKind::Id(String::from(&raw[1..])),
                specificity: 100,
            })
        } else if raw.starts_with('.') {
            Some(Selector {
                kind: SelectorKind::Class(String::from(&raw[1..])),
                specificity: 10,
            })
        } else if !raw.is_empty() {
            Some(Selector {
                kind: SelectorKind::Tag(String::from(raw).to_lowercase()),
                specificity: 1,
            })
        } else {
            None
        }
    }

    pub fn matches(&self, node: &DomNode) -> bool {
        match &self.kind {
            SelectorKind::Universal => true,
            SelectorKind::Tag(t) => node.tag == *t,
            SelectorKind::Class(c) => node.has_class(c),
            SelectorKind::Id(id) => node.id_attr.as_deref() == Some(id.as_str()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CssProperty {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct CssRule {
    pub selectors: Vec<Selector>,
    pub properties: Vec<CssProperty>,
}

pub fn parse_declarations(decl_str: &str) -> Vec<CssProperty> {
    let mut props = Vec::new();
    for part in decl_str.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(colon) = part.find(':') {
            let name = part[..colon].trim().to_lowercase();
            let value = part[colon + 1..].trim();
            props.push(CssProperty {
                name,
                value: String::from(value),
            });
        }
    }
    props
}

pub fn parse_stylesheet(css_text: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    // Strip comments
    let mut clean = String::new();
    let mut i = 0;
    let bytes = css_text.as_bytes();
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            if let Some(end) = css_text[i + 2..].find("*/") {
                i += end + 4;
                continue;
            } else {
                break;
            }
        }
        clean.push(bytes[i] as char);
        i += 1;
    }

    let mut pos = 0;
    let clean_str = clean.as_str();
    while pos < clean_str.len() {
        let brace_open = match clean_str[pos..].find('{') {
            Some(idx) => pos + idx,
            None => break,
        };
        let brace_close = match clean_str[brace_open + 1..].find('}') {
            Some(idx) => brace_open + 1 + idx,
            None => break,
        };

        let selector_part = clean_str[pos..brace_open].trim();
        let body_part = clean_str[brace_open + 1..brace_close].trim();

        let mut selectors = Vec::new();
        for s in selector_part.split(',') {
            if let Some(sel) = Selector::parse(s) {
                selectors.push(sel);
            }
        }

        let properties = parse_declarations(body_part);
        if !selectors.is_empty() && !properties.is_empty() {
            rules.push(CssRule {
                selectors,
                properties,
            });
        }

        pos = brace_close + 1;
    }

    rules
}

pub fn apply_property(style: &mut ComputedStyle, name: &str, val: &str) {
    let name = name.trim().to_lowercase();
    let val = val.trim();

    match name.as_str() {
        "color" => {
            if let Some(c) = parse_color(val) {
                style.color = c;
            }
        }
        "background-color" | "background" => {
            if let Some(c) = parse_color(val) {
                style.background_color = Some(c);
            } else if val == "none" || val == "transparent" {
                style.background_color = None;
            }
        }
        "font-size" => {
            if let Some(px) = parse_pixel_value(val) {
                style.font_size = px;
                style.font_scale = if px >= 20 { 2 } else { 1 };
            }
        }
        "font-weight" => {
            if val == "bold" || val == "700" || val == "800" || val == "900" {
                style.bold = true;
            } else if val == "normal" || val == "400" {
                style.bold = false;
            }
        }
        "font-style" => {
            if val == "italic" || val == "oblique" {
                style.italic = true;
            } else if val == "normal" {
                style.italic = false;
            }
        }
        "text-align" => {
            match val {
                "center" => style.text_align = TextAlign::Center,
                "right" => style.text_align = TextAlign::Right,
                _ => style.text_align = TextAlign::Left,
            }
        }
        "display" => {
            match val {
                "none" => style.display = DisplayMode::None,
                "inline" | "inline-block" => style.display = DisplayMode::Inline,
                _ => style.display = DisplayMode::Block,
            }
        }
        "margin" => {
            if let Some(px) = parse_pixel_value(val) {
                style.margin_top = px;
                style.margin_bottom = px;
                style.margin_left = px;
                style.margin_right = px;
            }
        }
        "margin-top" => {
            if let Some(px) = parse_pixel_value(val) {
                style.margin_top = px;
            }
        }
        "margin-bottom" => {
            if let Some(px) = parse_pixel_value(val) {
                style.margin_bottom = px;
            }
        }
        "margin-left" => {
            if let Some(px) = parse_pixel_value(val) {
                style.margin_left = px;
            }
        }
        "margin-right" => {
            if let Some(px) = parse_pixel_value(val) {
                style.margin_right = px;
            }
        }
        "padding" => {
            if let Some(px) = parse_pixel_value(val) {
                style.padding_top = px;
                style.padding_bottom = px;
                style.padding_left = px;
                style.padding_right = px;
            }
        }
        "padding-top" => {
            if let Some(px) = parse_pixel_value(val) {
                style.padding_top = px;
            }
        }
        "padding-bottom" => {
            if let Some(px) = parse_pixel_value(val) {
                style.padding_bottom = px;
            }
        }
        "padding-left" => {
            if let Some(px) = parse_pixel_value(val) {
                style.padding_left = px;
            }
        }
        "padding-right" => {
            if let Some(px) = parse_pixel_value(val) {
                style.padding_right = px;
            }
        }
        "border" => {
            // e.g. "1px solid #ccc" or "2px black"
            let parts: Vec<&str> = val.split_whitespace().collect();
            for p in parts {
                if let Some(px) = parse_pixel_value(p) {
                    style.border_width = px;
                } else if let Some(c) = parse_color(p) {
                    style.border_color = Some(c);
                }
            }
            if style.border_width == 0 {
                style.border_width = 1;
            }
        }
        "border-width" => {
            if let Some(px) = parse_pixel_value(val) {
                style.border_width = px;
            }
        }
        "border-color" => {
            style.border_color = parse_color(val);
        }
        "width" => {
            style.width = parse_pixel_value(val);
        }
        "height" => {
            style.height = parse_pixel_value(val);
        }
        _ => {}
    }
}

pub fn compute_styles(tree: &DomTree, rules: &[CssRule]) -> Vec<ComputedStyle> {
    let mut styles: Vec<ComputedStyle> = Vec::with_capacity(tree.nodes.len());

    // 1. Initialize default styles according to HTML element semantics
    for node in &tree.nodes {
        let mut st = ComputedStyle::default();
        match node.tag.as_str() {
            "h1" => {
                st.font_size = 24;
                st.font_scale = 2;
                st.bold = true;
                st.margin_top = 10;
                st.margin_bottom = 8;
            }
            "h2" => {
                st.font_size = 18;
                st.font_scale = 1;
                st.bold = true;
                st.margin_top = 8;
                st.margin_bottom = 6;
            }
            "h3" => {
                st.font_size = 14;
                st.font_scale = 1;
                st.bold = true;
                st.margin_top = 6;
                st.margin_bottom = 4;
            }
            "p" => {
                st.margin_top = 4;
                st.margin_bottom = 8;
            }
            "button" => {
                st.display = DisplayMode::Inline;
                st.padding_top = 4;
                st.padding_bottom = 4;
                st.padding_left = 10;
                st.padding_right = 10;
                st.margin_top = 2;
                st.margin_bottom = 2;
                st.margin_left = 2;
                st.margin_right = 4;
                st.bold = true;
            }
            "b" | "strong" => {
                st.display = DisplayMode::Inline;
                st.bold = true;
            }
            "i" | "em" => {
                st.display = DisplayMode::Inline;
                st.italic = true;
            }
            "a" => {
                st.display = DisplayMode::Inline;
                st.color = Color::from_rgb(0, 80, 210);
            }
            "span" => {
                st.display = DisplayMode::Inline;
            }
            "head" | "title" | "style" | "script" => {
                st.display = DisplayMode::None;
            }
            _ => {}
        }
        styles.push(st);
    }

    // 2. Apply CSS rules with specificity sorting
    // Collect (node_id, specificity, &CssProperty)
    let mut matched_props: Vec<(usize, usize, CssProperty)> = Vec::new();

    for rule in rules {
        for sel in &rule.selectors {
            for node in &tree.nodes {
                if sel.matches(node) {
                    for prop in &rule.properties {
                        matched_props.push((node.id, sel.specificity, prop.clone()));
                    }
                }
            }
        }
    }

    // Sort by specificity ascending so higher specificity applies last
    matched_props.sort_by_key(|item| item.1);

    for (node_id, _, prop) in matched_props {
        if node_id < styles.len() {
            apply_property(&mut styles[node_id], &prop.name, &prop.value);
        }
    }

    // 3. Apply inline styles (highest specificity: 1000)
    for node in &tree.nodes {
        if !node.inline_style.is_empty() {
            let decls = parse_declarations(&node.inline_style);
            for d in decls {
                if node.id < styles.len() {
                    apply_property(&mut styles[node.id], &d.name, &d.value);
                }
            }
        }
    }

    // 4. Inherit inheritable properties (color, font_size, font_scale, bold, italic) down the tree
    for node in &tree.nodes {
        if let Some(parent_id) = node.parent {
            if parent_id < styles.len() && node.id < styles.len() {
                // If child didn't explicitly set color, inherit from parent
                if styles[node.id].color == ComputedStyle::default().color && styles[parent_id].color != ComputedStyle::default().color {
                    styles[node.id].color = styles[parent_id].color;
                }
                // Inherit font_scale and font_size
                if styles[node.id].font_scale == 1 && styles[parent_id].font_scale > 1 {
                    styles[node.id].font_scale = styles[parent_id].font_scale;
                    styles[node.id].font_size = styles[parent_id].font_size;
                }
                // Inherit bold and italic
                if !styles[node.id].bold && styles[parent_id].bold {
                    styles[node.id].bold = true;
                }
                if !styles[node.id].italic && styles[parent_id].italic {
                    styles[node.id].italic = true;
                }
            }
        }
    }

    styles
}

#[test_case]
fn test_css_engine_parsing_and_cascade() {
    let css_str = "
        body { color: #333333; background-color: white; }
        .card { background-color: #f0f0f0; padding: 10px; }
        #counter { color: red; font-weight: bold; }
    ";
    let rules = parse_stylesheet(css_str);
    assert_eq!(rules.len(), 3);

    let mut tree = DomTree::new();
    let body = tree.create_element("body");
    tree.append_child(tree.root, body);

    let card = tree.create_element("div");
    tree.append_child(body, card);
    tree.set_attribute(card, "class", "card");

    let span = tree.create_element("span");
    tree.append_child(card, span);
    tree.set_attribute(span, "id", "counter");
    tree.set_attribute(span, "style", "font-size: 24px;");

    let styles = compute_styles(&tree, &rules);
    assert_eq!(styles[card].background_color, Some(Color::from_rgb(240, 240, 240)));
    assert_eq!(styles[card].padding_top, 10);
    assert_eq!(styles[span].color, Color::from_rgb(220, 20, 20)); // red
    assert_eq!(styles[span].bold, true);
    assert_eq!(styles[span].font_size, 24);
    assert_eq!(styles[span].font_scale, 2);
}


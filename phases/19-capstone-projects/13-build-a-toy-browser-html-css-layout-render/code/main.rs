// Build a Toy Browser (HTML/CSS layout + render)
// Run: rustc main.rs && ./main
//
// Architecture:
//   HTML text → DOM tree → CSS rules → Styled tree → Layout tree → ASCII paint
//
// Implements a complete browser pipeline: recursive-descent HTML parser,
// CSS parser with selector matching, block layout engine, and ASCII renderer.

use std::collections::HashMap;

// =============================================================================
// Step 1: HTML Parser — builds a DOM tree
// =============================================================================

#[derive(Debug, Clone)]
struct DOMNode {
    tag: String,
    attributes: HashMap<String, String>,
    children: Vec<DOMNode>,
    text: Option<String>,
}

impl DOMNode {
    fn element(tag: &str) -> Self {
        DOMNode { tag: tag.to_string(), attributes: HashMap::new(), children: Vec::new(), text: None }
    }
    fn text(content: &str) -> Self {
        DOMNode { tag: "#text".to_string(), attributes: HashMap::new(), children: Vec::new(), text: Some(content.to_string()) }
    }
}

struct HTMLParser { input: Vec<char>, pos: usize }

impl HTMLParser {
    fn new(input: &str) -> Self { HTMLParser { input: input.chars().collect(), pos: 0 } }
    fn peek(&self) -> Option<char> { self.input.get(self.pos).copied() }
    fn advance(&mut self) -> Option<char> { let ch = self.input.get(self.pos).copied()?; self.pos += 1; Some(ch) }
    fn skip_whitespace(&mut self) { while let Some(ch) = self.peek() { if ch.is_whitespace() { self.advance(); } else { break; } } }
    fn read_until(&mut self, delimiter: char) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() { if ch == delimiter { break; } s.push(ch); self.advance(); }
        s
    }
    fn parse_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(ch) = self.peek() { if ch.is_alphanumeric() || ch == '-' { name.push(ch); self.advance(); } else { break; } }
        name
    }
    fn parse_attributes(&mut self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some('>') || self.peek() == Some('/') { break; }
            let name = self.parse_tag_name();
            if name.is_empty() { break; }
            self.skip_whitespace();
            if self.peek() == Some('=') {
                self.advance(); self.skip_whitespace();
                let quote = self.advance().unwrap_or('"');
                let value = self.read_until(quote);
                self.advance();
                attrs.insert(name, value);
            } else { attrs.insert(name, String::new()); }
        }
        attrs
    }
    fn parse_node(&mut self) -> Option<DOMNode> {
        self.skip_whitespace();
        if self.pos >= self.input.len() { return None; }
        if self.peek() == Some('<') {
            self.advance();
            if self.peek() == Some('/') { self.read_until('>'); self.advance(); return None; }
            let tag = self.parse_tag_name();
            let attributes = self.parse_attributes();
            self.skip_whitespace();
            if self.peek() == Some('/') { self.advance(); self.advance(); return Some(DOMNode { tag, attributes, children: Vec::new(), text: None }); }
            self.advance();
            let mut children = Vec::new();
            loop {
                self.skip_whitespace();
                if self.pos >= self.input.len() { break; }
                if self.input[self.pos..].starts_with(&['<', '/']) { break; }
                if let Some(child) = self.parse_node() { children.push(child); } else { break; }
            }
            if self.peek() == Some('<') { self.advance(); if self.peek() == Some('/') { self.advance(); self.parse_tag_name(); self.skip_whitespace(); self.advance(); } }
            Some(DOMNode { tag, attributes, children, text: None })
        } else {
            let text = self.read_until('<');
            if text.is_empty() { return None; }
            Some(DOMNode::text(&text))
        }
    }
    fn parse(&mut self) -> DOMNode {
        let mut root = DOMNode::element("html");
        while let Some(node) = self.parse_node() { root.children.push(node); }
        root
    }
}

// =============================================================================
// Step 2: CSS Parser and Style Resolution
// =============================================================================

#[derive(Debug, Clone)]
struct CSSRule { selector: String, declarations: HashMap<String, String> }

struct CSSParser { input: Vec<char>, pos: usize }

impl CSSParser {
    fn new(input: &str) -> Self { CSSParser { input: input.chars().collect(), pos: 0 } }
    fn parse_rules(&mut self) -> Vec<CSSRule> {
        let mut rules = Vec::new();
        let content: String = self.input.iter().collect();
        for rule_str in content.split('}') {
            let parts: Vec<&str> = rule_str.splitn(2, '{').collect();
            if parts.len() != 2 { continue; }
            let selector = parts[0].trim().to_string();
            if selector.is_empty() { continue; }
            let mut declarations = HashMap::new();
            for decl in parts[1].split(';') {
                let kv: Vec<&str> = decl.splitn(2, ':').collect();
                if kv.len() == 2 { declarations.insert(kv[0].trim().to_string(), kv[1].trim().to_string()); }
            }
            rules.push(CSSRule { selector, declarations });
        }
        rules
    }
}

fn matches_selector(node: &DOMNode, selector: &str) -> bool {
    let selector = selector.trim();
    if selector.starts_with('.') {
        let class = &selector[1..];
        node.attributes.get("class").map(|c| c.split_whitespace().any(|c| c == class)).unwrap_or(false)
    } else if selector.starts_with('#') {
        let id = &selector[1..];
        node.attributes.get("id").map(|i| i.as_str()) == Some(id)
    } else { node.tag == selector }
}

#[derive(Debug, Clone)]
struct StyledNode { node: DOMNode, styles: HashMap<String, String>, children: Vec<StyledNode> }

fn resolve_styles(node: &DOMNode, rules: &[CSSRule], parent_styles: &HashMap<String, String>) -> StyledNode {
    let mut styles = parent_styles.clone();
    for rule in rules {
        if matches_selector(node, &rule.selector) {
            for (prop, value) in &rule.declarations { styles.insert(prop.clone(), value.clone()); }
        }
    }
    if let Some(style_attr) = node.attributes.get("style") {
        for decl in style_attr.split(';') {
            let kv: Vec<&str> = decl.splitn(2, ':').collect();
            if kv.len() == 2 { styles.insert(kv[0].trim().to_string(), kv[1].trim().to_string()); }
        }
    }
    let children: Vec<StyledNode> = node.children.iter()
        .map(|child| resolve_styles(child, rules, &styles)).collect();
    StyledNode { node: node.clone(), styles, children }
}

// =============================================================================
// Step 3: Layout Engine
// =============================================================================

#[derive(Debug, Clone)]
struct LayoutBox {
    x: f64, y: f64, width: f64, height: f64,
    styles: HashMap<String, String>,
    children: Vec<LayoutBox>,
    text: Option<String>,
}

fn layout(styled: &StyledNode, containing_width: f64, x: f64, y: f64) -> LayoutBox {
    let mut box_node = LayoutBox {
        x, y, width: containing_width, height: 0.0,
        styles: styled.styles.clone(), children: Vec::new(),
        text: styled.node.text.clone(),
    };
    if styled.node.text.is_some() { box_node.height = 20.0; return box_node; }
    let mut child_y = y;
    let padding = 10.0;
    for child in &styled.children {
        let child_box = layout(child, containing_width - 2.0 * padding, x + padding, child_y);
        child_y += child_box.height;
        box_node.children.push(child_box);
    }
    box_node.height = child_y - y + padding;
    box_node
}

// =============================================================================
// Step 4: ASCII Paint and Main
// =============================================================================

fn paint(box_node: &LayoutBox, canvas: &mut Vec<Vec<char>>, width: usize, height: usize) {
    let x = box_node.x as usize;
    let y = box_node.y as usize;
    if let Some(bg) = box_node.styles.get("background") {
        let bg_char = match bg.as_str() { "blue" => '#', "red" => '%', "green" => '&', _ => '.' };
        let w = box_node.width.min(width as f64) as usize;
        let h = box_node.height.min(height as f64) as usize;
        for dy in 0..h { for dx in 0..w {
            let (px, py) = (x + dx, y + dy);
            if px < width && py < height { canvas[py][px] = bg_char; }
        }}
    }
    if let Some(ref text) = box_node.text {
        for (i, ch) in text.chars().enumerate() {
            let px = x + i;
            if px < width && y < height { canvas[y][px] = ch; }
        }
    }
    for child in &box_node.children { paint(child, canvas, width, height); }
}

fn main() {
    let html = r#"
        <div class="card">
            <h1>Hello World</h1>
            <p>This is a paragraph.</p>
        </div>
    "#;
    let css = r#"
        .card { background: blue; }
        h1 { color: red; }
        p { color: green; }
    "#;

    let mut html_parser = HTMLParser::new(html);
    let dom = html_parser.parse();
    let mut css_parser = CSSParser::new(css);
    let rules = css_parser.parse_rules();

    let styled = resolve_styles(&dom, &rules, &HashMap::new());
    let layout_tree = layout(&styled, 80.0, 0.0, 0.0);

    let (width, height) = (80, 20);
    let mut canvas = vec![vec![' '; width]; height];
    paint(&layout_tree, &mut canvas, width, height);

    println!("=== Rendered Output ===");
    for row in &canvas {
        let line: String = row.iter().collect();
        println!("{}", line);
    }
}

# Build a Toy Browser (HTML/CSS layout + render)

> The browser pipeline is compact: bytes, DOM/CSSOM, style, layout, paint.

**Type:** Build
**Languages:** Rust, TypeScript
**Prerequisites:** Phase 19 lessons 01-12
**Time:** ~720 minutes

## Learning Objectives

- Parse a minimal HTML subset into a DOM tree.
- Parse a minimal CSS subset into rules.
- Match selectors and compute a style map.
- Build a vertical block layout tree and paint a text canvas.

## The Problem

Modern browsers are millions of lines of code. Chromium alone has over 25 million lines. Someone who tries to understand the full system at once gets lost in the complexity: the rendering engine, the JavaScript engine, the networking stack, the compositor, the GPU pipeline, accessibility, security sandboxes... the list goes on.

But the core rendering loop is compact and elegant: `bytes -> DOM/CSSOM -> style -> layout -> paint`. Every browser, from Netscape 1.0 to Chrome 120, follows this loop. The details changed (tables became flexbox became grid, single-threaded became multi-process), but the pipeline stayed the same.

Building a toy browser teaches you what each stage does, what invariants it maintains, and why some UI updates are cheap (repaint only) while others are expensive (reflow the entire layout). Once you understand the pipeline, you can reason about rendering performance, CSS specificity, and why `position: absolute` takes an element out of normal flow.

## The Concept

The browser rendering pipeline has five stages:

```
HTML bytes
    │
    ▼
┌──────────────┐
│ HTML Parser   │ → DOM tree (Document Object Model)
└──────────────┘
    │
CSS bytes       │
    │           │
    ▼           ▼
┌──────────────┐
│ CSS Parser    │ → CSSOM (CSS Object Model)
└──────────────┘
    │
    ▼
┌──────────────┐
│ Style         │ → Styled tree (each node has computed properties)
│ Resolution    │   Match selectors, resolve cascade, inherit
└──────────────┘
    │
    ▼
┌──────────────┐
│ Layout        │ → Layout tree (each node has x, y, w, h)
│ (Reflow)      │   Block/inline/flex positioning
└──────────────┘
    │
    ▼
┌──────────────┐
│ Paint         │ → Display list → pixels
│ (Repaint)     │   Draw backgrounds, text, borders
└──────────────┘
```

CSS specificity determines which rule wins when multiple rules match the same element. The specificity is a triple: (inline, id, class). Inline styles beat IDs, IDs beat classes, classes beat element selectors. When specificity is equal, the later rule wins.

## Build It

### Step 1: HTML Parser (Rust)

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct DOMNode {
    tag: String,
    attributes: HashMap<String, String>,
    children: Vec<DOMNode>,
    text: Option<String>,
}

impl DOMNode {
    fn element(tag: &str) -> Self {
        DOMNode {
            tag: tag.to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text: None,
        }
    }

    fn text(content: &str) -> Self {
        DOMNode {
            tag: "#text".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text: Some(content.to_string()),
        }
    }
}

struct HTMLParser {
    input: Vec<char>,
    pos: usize,
}

impl HTMLParser {
    fn new(input: &str) -> Self {
        HTMLParser {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() { self.advance(); } else { break; }
        }
    }

    fn read_until(&mut self, delimiter: char) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == delimiter { break; }
            s.push(ch);
            self.advance();
        }
        s
    }

    fn parse_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '-' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        name
    }

    fn parse_attributes(&mut self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some('>') || self.peek() == Some('/') {
                break;
            }
            let name = self.parse_tag_name();
            if name.is_empty() { break; }
            self.skip_whitespace();
            if self.peek() == Some('=') {
                self.advance();
                self.skip_whitespace();
                let quote = self.advance().unwrap_or('"');
                let value = self.read_until(quote);
                self.advance(); // closing quote
                attrs.insert(name, value);
            } else {
                attrs.insert(name, String::new());
            }
        }
        attrs
    }

    fn parse_node(&mut self) -> Option<DOMNode> {
        self.skip_whitespace();
        if self.pos >= self.input.len() { return None; }

        if self.peek() == Some('<') {
            self.advance(); // consume '<'

            if self.peek() == Some('/') {
                // Closing tag, skip
                self.read_until('>');
                self.advance();
                return None;
            }

            let tag = self.parse_tag_name();
            let attributes = self.parse_attributes();
            self.skip_whitespace();

            if self.peek() == Some('/') {
                // Self-closing tag
                self.advance(); // '/'
                self.advance(); // '>'
                return Some(DOMNode { tag, attributes, children: Vec::new(), text: None });
            }

            self.advance(); // consume '>'

            // Parse children
            let mut children = Vec::new();
            loop {
                self.skip_whitespace();
                if self.pos >= self.input.len() { break; }
                if self.input[self.pos..].starts_with(&['<', '/']) {
                    break; // Closing tag
                }
                if let Some(child) = self.parse_node() {
                    children.push(child);
                } else {
                    break;
                }
            }

            // Skip closing tag
            if self.peek() == Some('<') {
                self.advance();
                if self.peek() == Some('/') {
                    self.advance();
                    self.parse_tag_name();
                    self.skip_whitespace();
                    self.advance(); // '>'
                }
            }

            Some(DOMNode { tag, attributes, children, text: None })
        } else {
            // Text node
            let text = self.read_until('<');
            if text.is_empty() { return None; }
            Some(DOMNode::text(&text))
        }
    }

    fn parse(&mut self) -> DOMNode {
        let mut root = DOMNode::element("html");
        while let Some(node) = self.parse_node() {
            root.children.push(node);
        }
        root
    }
}
```

### Step 2: CSS Parser and Style Resolution

```rust
#[derive(Debug, Clone)]
struct CSSRule {
    selector: String,
    declarations: HashMap<String, String>,
}

struct CSSParser {
    input: Vec<char>,
    pos: usize,
}

impl CSSParser {
    fn new(input: &str) -> Self {
        CSSParser { input: input.chars().collect(), pos: 0 }
    }

    fn parse_rules(&mut self) -> Vec<CSSRule> {
        let mut rules = Vec::new();
        let content: String = self.input.iter().collect();
        // Simple: split on '}' and parse each rule
        for rule_str in content.split('}') {
            let parts: Vec<&str> = rule_str.splitn(2, '{').collect();
            if parts.len() != 2 { continue; }
            let selector = parts[0].trim().to_string();
            if selector.is_empty() { continue; }

            let mut declarations = HashMap::new();
            for decl in parts[1].split(';') {
                let kv: Vec<&str> = decl.splitn(2, ':').collect();
                if kv.len() == 2 {
                    declarations.insert(
                        kv[0].trim().to_string(),
                        kv[1].trim().to_string(),
                    );
                }
            }
            rules.push(CSSRule { selector, declarations });
        }
        rules
    }
}

// Match a CSS selector against a DOM node
fn matches_selector(node: &DOMNode, selector: &str) -> bool {
    let selector = selector.trim();
    if selector.starts_with('.') {
        // Class selector
        let class = &selector[1..];
        node.attributes.get("class")
            .map(|c| c.split_whitespace().any(|c| c == class))
            .unwrap_or(false)
    } else if selector.starts_with('#') {
        // ID selector
        let id = &selector[1..];
        node.attributes.get("id").map(|i| i.as_str()) == Some(id)
    } else {
        // Tag selector
        node.tag == selector
    }
}

// Resolve styles for all nodes
fn resolve_styles(node: &DOMNode, rules: &[CSSRule], parent_styles: &HashMap<String, String>) -> StyledNode {
    let mut styles = parent_styles.clone(); // Inherit from parent

    // Apply matching rules (in source order, later wins)
    for rule in rules {
        if matches_selector(node, &rule.selector) {
            for (prop, value) in &rule.declarations {
                styles.insert(prop.clone(), value.clone());
            }
        }
    }

    // Apply inline styles
    if let Some(style_attr) = node.attributes.get("style") {
        for decl in style_attr.split(';') {
            let kv: Vec<&str> = decl.splitn(2, ':').collect();
            if kv.len() == 2 {
                styles.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
            }
        }
    }

    let children: Vec<StyledNode> = node.children.iter()
        .map(|child| resolve_styles(child, rules, &styles))
        .collect();

    StyledNode {
        node: node.clone(),
        styles,
        children,
    }
}
```

### Step 3: Layout Engine

```rust
#[derive(Debug, Clone)]
struct StyledNode {
    node: DOMNode,
    styles: HashMap<String, String>,
    children: Vec<StyledNode>,
}

#[derive(Debug, Clone)]
struct LayoutBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    styles: HashMap<String, String>,
    children: Vec<LayoutBox>,
    text: Option<String>,
}

fn layout(styled: &StyledNode, containing_width: f64, x: f64, y: f64) -> LayoutBox {
    let mut box_node = LayoutBox {
        x, y,
        width: containing_width,
        height: 0.0,
        styles: styled.styles.clone(),
        children: Vec::new(),
        text: styled.node.text.clone(),
    };

    // Text contributes line height
    if styled.node.text.is_some() {
        box_node.height = 20.0; // Fixed line height
        return box_node;
    }

    // Block layout: stack children vertically
    let mut child_y = y;
    let padding = 10.0; // Simplified padding

    for child in &styled.children {
        let child_box = layout(child, containing_width - 2.0 * padding, x + padding, child_y);
        child_y += child_box.height;
        box_node.children.push(child_box);
    }

    box_node.height = child_y - y + padding;
    box_node
}
```

### Step 4: ASCII Paint

```rust
fn paint(box_node: &LayoutBox, canvas: &mut Vec<Vec<char>>, width: usize, height: usize) {
    let x = box_node.x as usize;
    let y = box_node.y as usize;

    // Paint background if specified
    if let Some(bg) = box_node.styles.get("background") {
        let bg_char = match bg.as_str() {
            "blue" => '#',
            "red" => '%',
            "green" => '&',
            _ => '.',
        };
        let w = box_node.width.min(width as f64) as usize;
        let h = box_node.height.min(height as f64) as usize;
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px < width && py < height {
                    canvas[py][px] = bg_char;
                }
            }
        }
    }

    // Paint text
    if let Some(ref text) = box_node.text {
        for (i, ch) in text.chars().enumerate() {
            let px = x + i;
            if px < width && y < height {
                canvas[y][px] = ch;
            }
        }
    }

    // Paint children
    for child in &box_node.children {
        paint(child, canvas, width, height);
    }
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

    // Parse
    let mut html_parser = HTMLParser::new(html);
    let dom = html_parser.parse();
    let mut css_parser = CSSParser::new(css);
    let rules = css_parser.parse_rules();

    // Style
    let styled = resolve_styles(&dom, &rules, &HashMap::new());

    // Layout
    let layout_tree = layout(&styled, 80.0, 0.0, 0.0);

    // Paint
    let width = 80;
    let height = 20;
    let mut canvas = vec![vec![' '; width]; height];
    paint(&layout_tree, &mut canvas, width, height);

    // Output
    println!("=== Rendered Output ===");
    for row in &canvas {
        let line: String = row.iter().collect();
        println!("{}", line);
    }
}
```

## Use It

The toy engine omits JavaScript, incremental layout, fonts, accessibility, and compositing, but the architecture matches production:

- **Chromium/Blink**: parses HTML into `Document`, CSS into style rules, produces layout objects, and paints display lists. The layout engine is in `third_party/blink/renderer/core/layout/`. The style system is in `third_party/blink/renderer/core/style/`.
- **Firefox/Gecko**: similar pipeline with Servo's style engine integration. The layout engine is in `layout/`. Firefox's Stylo (Servo's style system) parallelizes style resolution.
- **Servo**: Mozilla's research browser engine written in Rust. Designed for parallelism: style resolution and layout can run on multiple threads. The `style/` crate implements CSS matching; the `layout/` crate implements layout.

The key production lesson: **layout is the most expensive stage**. Changing a DOM node's style triggers style recalculation, then layout (reflow), then repaint. Changing only `color` triggers repaint without reflow. Understanding this pipeline is essential for optimizing web performance.

## Read the Source

- [Chromium layout source](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/layout/) — Production layout engine.
- [Servo source](https://github.com/servo/servo) — Rust browser engine with parallel layout.
- [How Browsers Work](https://www.html5rocks.com/en/tutorials/internals/howbrowserswork/) — Comprehensive overview of browser internals.

## Ship It

- `code/main.rs`: complete HTML parser, CSS parser, style resolver, block layout engine, and ASCII paint.
- `outputs/README.md`: browser capstone checklist covering DOM, CSSOM, style, layout, and paint.

## Exercises

1. **Easy** — Add `#id` selectors and selector specificity. Implement specificity calculation: (inline, id, class, tag). When two rules match the same property, the one with higher specificity wins. When specificity is equal, the later rule wins.
2. **Medium** — Add inline style attributes. Parse `style="color: red; background: blue"` on elements and apply those declarations with highest priority (inline > id > class > tag).
3. **Hard** — Add dirty-region repaint for changed nodes. Track which nodes changed since the last frame. Only repaint the changed nodes and their ancestors. Show that changing a leaf node's text doesn't require repainting the entire page.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| DOM | "HTML tree" | Document Object Model: a tree data structure representing the parsed HTML document. Each element, attribute, and text node is a DOM node. The DOM is the interface between HTML and JavaScript. |
| CSSOM | "styles tree" | CSS Object Model: the parsed representation of CSS rules. The style resolution phase matches CSSOM rules against DOM nodes to compute final styles. |
| Cascade | "CSS priority" | The algorithm that resolves conflicts when multiple CSS rules apply to the same element. Rules are ordered by origin (user, author, browser), specificity, and source order. |
| Layout | "box calculation" | The process of computing the geometric position and size of each element. Block layout stacks vertically; inline layout flows horizontally. Layout is triggered by style changes. |
| Paint | "drawing" | The process of converting layout boxes into pixels. The paint phase generates a display list of drawing commands (draw background, draw text, draw border) and executes them. |

## Further Reading

- [How Browsers Work](https://www.html5rocks.com/en/tutorials/internals/howbrowserswork/) — Comprehensive overview.
- [Servo](https://github.com/servo/servo) — Rust browser engine with parallel layout.
- [Chromium source](https://source.chromium.org/) — Searchable Chromium source code.

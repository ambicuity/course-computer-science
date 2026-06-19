use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword(String),
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    Op(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Assign,
    Eof,
    Error { msg: String, line: usize, col: usize },
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Keyword(kw) => write!(f, "KEYWORD({})", kw),
            Token::Ident(name) => write!(f, "IDENT({})", name),
            Token::Int(n) => write!(f, "INT({})", n),
            Token::Float(n) => write!(f, "FLOAT({})", n),
            Token::String(s) => write!(f, "STRING({:?})", s),
            Token::Op(op) => write!(f, "OP({})", op),
            Token::LParen => write!(f, "LPAREN"),
            Token::RParen => write!(f, "RPAREN"),
            Token::LBrace => write!(f, "LBRACE"),
            Token::RBrace => write!(f, "RBRACE"),
            Token::LBracket => write!(f, "LBRACKET"),
            Token::RBracket => write!(f, "RBRACKET"),
            Token::Semi => write!(f, "SEMI"),
            Token::Comma => write!(f, "COMMA"),
            Token::Assign => write!(f, "ASSIGN"),
            Token::Eof => write!(f, "EOF"),
            Token::Error { msg, line, col } => write!(f, "ERROR({}:{}: {})", line, col, msg),
        }
    }
}

pub struct Scanner {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    keywords: HashMap<String, String>,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        let mut keywords = HashMap::new();
        for &kw in &["if", "else", "while", "for", "return", "fn", "let", "mut", "true", "false"] {
            keywords.insert(kw.to_string(), kw.to_string());
        }
        Scanner {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            keywords,
        }
    }

    // ── character helpers ──────────────────────────────────────────────

    fn ch(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.ch()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    // ── whitespace & comments ─────────────────────────────────────────

    fn skip_trivia(&mut self) {
        while !self.at_end() {
            match self.ch() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek() == Some('/') => {
                    // single-line comment
                    while let Some(c) = self.advance() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                Some('/') if self.peek() == Some('*') => {
                    // multi-line comment
                    self.advance(); // /
                    self.advance(); // *
                    let start_line = self.line;
                    loop {
                        match self.ch() {
                            None => break,
                            Some('*') if self.peek() == Some('/') => {
                                self.advance();
                                self.advance();
                                break;
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                }
                _ => return,
            }
        }
    }

    // ── scanners ───────────────────────────────────────────────────────

    fn scan_identifier(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut buf = String::new();
        while let Some(c) = self.ch() {
            if c.is_alphanumeric() || c == '_' {
                buf.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if let Some(kw) = self.keywords.get(&buf) {
            Token::Keyword(kw.clone())
        } else {
            Token::Ident(buf)
        }
    }

    fn scan_number(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut buf = String::new();

        // prefix: 0x / 0b / 0o
        if self.ch() == Some('0') {
            buf.push('0');
            self.advance();
            match self.ch() {
                Some('x') | Some('X') => {
                    buf.push('x');
                    self.advance();
                    while let Some(c) = self.ch() {
                        if c.is_ascii_hexdigit() {
                            buf.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let val = i64::from_str_radix(&buf[2..], 16).unwrap_or(0);
                    return Token::Int(val);
                }
                Some('b') | Some('B') => {
                    buf.push(self.advance().unwrap());
                    while let Some(c) = self.ch() {
                        if c == '0' || c == '1' {
                            buf.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let val = i64::from_str_radix(&buf[2..], 2).unwrap_or(0);
                    return Token::Int(val);
                }
                Some('o') | Some('O') => {
                    buf.push(self.advance().unwrap());
                    while let Some(c) = self.ch() {
                        if ('0'..='7').contains(&c) {
                            buf.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let val = i64::from_str_radix(&buf[2..], 8).unwrap_or(0);
                    return Token::Int(val);
                }
                _ => {}
            }
        }

        // decimal part
        while let Some(c) = self.ch() {
            if c == '_' {
                // skip underscores in number literals
                self.advance();
                continue;
            }
            if c.is_ascii_digit() {
                buf.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // fractional part
        if self.ch() == Some('.') && self.peek().map_or(false, |c| c.is_ascii_digit()) {
            buf.push('.');
            self.advance();
            while let Some(c) = self.ch() {
                if c.is_ascii_digit() {
                    buf.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            // exponent
            if self.ch() == Some('e') || self.ch() == Some('E') {
                buf.push(self.advance().unwrap());
                if self.ch() == Some('+') || self.ch() == Some('-') {
                    buf.push(self.advance().unwrap());
                }
                while let Some(c) = self.ch() {
                    if c.is_ascii_digit() {
                        buf.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            return Token::Float(buf.parse().unwrap_or(0.0));
        }

        Token::Int(buf.parse().unwrap_or(0))
    }

    fn scan_string(&mut self, quote: char) -> Token {
        let line = self.line;
        let col = self.col;
        self.advance(); // opening quote
        let mut buf = String::new();
        loop {
            match self.ch() {
                None => {
                    return Token::Error {
                        msg: "unterminated string literal".into(),
                        line,
                        col,
                    };
                }
                Some(c) if c == quote => {
                    self.advance();
                    return Token::String(buf);
                }
                Some('\\') => {
                    self.advance();
                    match self.ch() {
                        Some('n') => { buf.push('\n'); self.advance(); }
                        Some('t') => { buf.push('\t'); self.advance(); }
                        Some('r') => { buf.push('\r'); self.advance(); }
                        Some('\\') => { buf.push('\\'); self.advance(); }
                        Some('"') => { buf.push('"'); self.advance(); }
                        Some('\'') => { buf.push('\''); self.advance(); }
                        Some('0') => { buf.push('\0'); self.advance(); }
                        Some(c) => {
                            buf.push('\\');
                            buf.push(c);
                            self.advance();
                        }
                        None => {
                            return Token::Error {
                                msg: "unterminated string literal".into(),
                                line,
                                col,
                            };
                        }
                    }
                }
                Some(c) => {
                    buf.push(c);
                    self.advance();
                }
            }
        }
    }

    fn scan_char_literal(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        self.advance(); // opening '
        let ch = match self.ch() {
            None => return Token::Error { msg: "unterminated char literal".into(), line, col },
            Some('\\') => {
                self.advance();
                match self.ch() {
                    Some('n') => { self.advance(); '\n' }
                    Some('t') => { self.advance(); '\t' }
                    Some('\\') => { self.advance(); '\\' }
                    Some('\'') => { self.advance(); '\'' }
                    Some('0') => { self.advance(); '\0' }
                    Some(c) => { self.advance(); c }
                    None => return Token::Error { msg: "unterminated char literal".into(), line, col },
                }
            }
            Some(c) => { self.advance(); c }
        };
        if self.ch() != Some('\'') {
            return Token::Error { msg: "expected closing ' for char literal".into(), line, col };
        }
        self.advance(); // closing '
        Token::Int(ch as i64)
    }

    fn scan_operator(&mut self) -> Token {
        let ch = self.advance().unwrap();
        let mut op = String::from(ch);
        // two-char operators
        match (ch, self.ch()) {
            ('=', Some('=')) | ('!', Some('=')) | ('<', Some('=')) | ('>', Some('=')) => {
                op.push(self.advance().unwrap());
            }
            ('&', Some('&')) | ('|', Some('|')) => {
                op.push(self.advance().unwrap());
            }
            _ => {}
        }
        Token::Op(op)
    }

    // ── main dispatch ──────────────────────────────────────────────────

    pub fn scan_token(&mut self) -> Token {
        self.skip_trivia();
        match self.ch() {
            None => Token::Eof,
            Some(c) if c.is_alphabetic() || c == '_' => self.scan_identifier(),
            Some(c) if c.is_ascii_digit() => self.scan_number(),
            Some('"') => self.scan_string('"'),
            Some('\'') => self.scan_char_literal(),
            Some('(') => { self.advance(); Token::LParen }
            Some(')') => { self.advance(); Token::RParen }
            Some('{') => { self.advance(); Token::LBrace }
            Some('}') => { self.advance(); Token::RBrace }
            Some('[') => { self.advance(); Token::LBracket }
            Some(']') => { self.advance(); Token::RBracket }
            Some(';') => { self.advance(); Token::Semi }
            Some(',') => { self.advance(); Token::Comma }
            Some('=') if self.peek() != Some('=') => { self.advance(); Token::Assign }
            Some('+') | Some('-') | Some('*') | Some('/') |
            Some('=') | Some('!') | Some('<') | Some('>') |
            Some('&') | Some('|') => self.scan_operator(),
            Some(c) => {
                let l = self.line;
                let co = self.col;
                self.advance();
                Token::Error { msg: format!("unexpected character: '{}'", c), line: l, col: co }
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.scan_token();
            let done = matches!(tok, Token::Eof);
            tokens.push(tok);
            if done {
                break;
            }
        }
        tokens
    }
}

fn main() {
    let source = r#"
fn fibonacci(n) {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

let result = fibonacci(10);
let hex_val = 0xDEAD;
let bin_val = 0b1101_0110;
let pi = 3.14159;
let message = "Hello,\nWorld!";
let ch = 'A';
// compute something
/* multi-line
   comment here */
if result == 55 && hex_val > 0 {
    true;
}
"#;

    let mut scanner = Scanner::new(source);
    let tokens = scanner.tokenize();
    for tok in &tokens {
        println!("{}", tok);
    }
}

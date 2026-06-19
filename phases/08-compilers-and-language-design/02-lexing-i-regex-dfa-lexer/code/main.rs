use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Keyword(String),
    // Identifiers
    Ident(String),
    // Numbers
    Int(i64),
    Float(f64),
    // Strings
    String(String),
    // Operators
    Op(String),
    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
    Comma,
    Assign,
    // Special
    Eof,
    // Error
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
            Token::Semi => write!(f, "SEMI"),
            Token::Comma => write!(f, "COMMA"),
            Token::Assign => write!(f, "ASSIGN"),
            Token::Eof => write!(f, "EOF"),
            Token::Error { msg, line, col } => write!(f, "ERROR({}:{}: {})", line, col, msg),
        }
    }
}

const KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "return", "fn", "let", "mut", "true", "false",
];

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek() == Some('/') {
                // Single-line comment
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn scan_identifier(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        let mut name = String::new();
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if KEYWORDS.contains(&name.as_str()) {
            Token::Keyword(name)
        } else {
            Token::Ident(name)
        }
    }

    fn scan_number(&mut self) -> Token {
        let mut num_str = String::new();
        let start_line = self.line;

        // Check for hex, octal, binary prefixes
        if self.current() == Some('0') {
            num_str.push('0');
            self.advance();
            match self.current() {
                Some('x') | Some('X') => {
                    num_str.push('x');
                    self.advance();
                    while let Some(ch) = self.current() {
                        if ch.is_ascii_hexdigit() {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return match i64::from_str_radix(&num_str[2..], 16) {
                        Ok(n) => Token::Int(n),
                        Err(_) => Token::Error {
                            msg: format!("invalid hex literal: {}", num_str),
                            line: start_line,
                            col: self.col,
                        },
                    };
                }
                Some('b') | Some('B') => {
                    num_str.push('b');
                    self.advance();
                    while let Some(ch) = self.current() {
                        if ch == '0' || ch == '1' {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return match i64::from_str_radix(&num_str[2..], 2) {
                        Ok(n) => Token::Int(n),
                        Err(_) => Token::Error {
                            msg: format!("invalid binary literal: {}", num_str),
                            line: start_line,
                            col: self.col,
                        },
                    };
                }
                Some('o') | Some('O') => {
                    num_str.push('o');
                    self.advance();
                    while let Some(ch) = self.current() {
                        if ('0'..='7').contains(&ch) {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return match i64::from_str_radix(&num_str[2..], 8) {
                        Ok(n) => Token::Int(n),
                        Err(_) => Token::Error {
                            msg: format!("invalid octal literal: {}", num_str),
                            line: start_line,
                            col: self.col,
                        },
                    };
                }
                _ => {}
            }
        }

        // Decimal digits
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Float: decimal point followed by digits
        if self.current() == Some('.') && self.peek().map_or(false, |c| c.is_ascii_digit()) {
            num_str.push('.');
            self.advance();
            while let Some(ch) = self.current() {
                if ch.is_ascii_digit() {
                    num_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            return match num_str.parse::<f64>() {
                Ok(n) => Token::Float(n),
                Err(_) => Token::Error {
                    msg: format!("invalid float literal: {}", num_str),
                    line: start_line,
                    col: self.col,
                },
            };
        }

        match num_str.parse::<i64>() {
            Ok(n) => Token::Int(n),
            Err(_) => Token::Error {
                msg: format!("invalid integer literal: {}", num_str),
                line: start_line,
                col: self.col,
            },
        }
    }

    fn scan_string(&mut self, quote: char) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        self.advance(); // consume opening quote
        let mut s = String::new();
        loop {
            match self.current() {
                None => {
                    return Token::Error {
                        msg: "unterminated string literal".to_string(),
                        line: start_line,
                        col: start_col,
                    };
                }
                Some(ch) if ch == quote => {
                    self.advance(); // consume closing quote
                    return Token::String(s);
                }
                Some('\\') => {
                    self.advance();
                    match self.current() {
                        Some('n') => { s.push('\n'); self.advance(); }
                        Some('t') => { s.push('\t'); self.advance(); }
                        Some('r') => { s.push('\r'); self.advance(); }
                        Some('\\') => { s.push('\\'); self.advance(); }
                        Some('"') => { s.push('"'); self.advance(); }
                        Some('\'') => { s.push('\''); self.advance(); }
                        Some('0') => { s.push('\0'); self.advance(); }
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                            self.advance();
                        }
                        None => {
                            return Token::Error {
                                msg: "unterminated string literal".to_string(),
                                line: start_line,
                                col: start_col,
                            };
                        }
                    }
                }
                Some(ch) => {
                    s.push(ch);
                    self.advance();
                }
            }
        }
    }

    fn scan_operator(&mut self) -> Token {
        let mut op = String::new();
        let ch = self.advance().unwrap();
        op.push(ch);
        // Two-character operators
        match (ch, self.current()) {
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

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        match self.current() {
            None => Token::Eof,
            Some(ch) if ch.is_alphabetic() || ch == '_' => self.scan_identifier(),
            Some(ch) if ch.is_ascii_digit() => self.scan_number(),
            Some('"') => self.scan_string('"'),
            Some('\'') => self.scan_string('\''),
            Some('(') => { self.advance(); Token::LParen }
            Some(')') => { self.advance(); Token::RParen }
            Some('{') => { self.advance(); Token::LBrace }
            Some('}') => { self.advance(); Token::RBrace }
            Some(';') => { self.advance(); Token::Semi }
            Some(',') => { self.advance(); Token::Comma }
            Some('=') if self.peek() != Some('=') => { self.advance(); Token::Assign }
            Some('+') | Some('-') | Some('*') | Some('/') |
            Some('=') | Some('!') | Some('<') | Some('>') |
            Some('&') | Some('|') => self.scan_operator(),
            Some(ch) => {
                let line = self.line;
                let col = self.col;
                self.advance();
                Token::Error {
                    msg: format!("unexpected character: '{}'", ch),
                    line,
                    col,
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token, Token::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

fn main() {
    let source = r#"
fn main() {
    let x = 42 + 0xFF;
    let pi = 3.14;
    let name = "hello\nworld";
    // This is a comment
    if x == 100 {
        return true;
    }
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    for token in &tokens {
        println!("{}", token);
    }
}

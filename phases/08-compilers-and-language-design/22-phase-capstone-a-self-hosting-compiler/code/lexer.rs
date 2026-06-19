// lexer.rs — Tokenizer for the pal language
//
// Produces a stream of tokens from source text. Tokens reference the
// source buffer via string slices (zero-copy). Line/column info is
// tracked for error messages.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Program, Var, Function, Begin, End,
    If, Then, Else, While, Do,
    Int, Bool, True, False,
    Print,

    // Literals
    Integer(i64),
    Ident(String),

    // Operators
    Plus, Minus, Star, Slash,
    Eq, Neq, Lt, Gt, Le, Ge,
    And, Or,

    // Symbols
    Assign,    // :=
    LParen,    // (
    RParen,    // )
    Semicolon, // ;
    Colon,     // :
    Comma,     // ,
    Dot,       // .

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '{' {
                // Skip comment until '}'
                while let Some(ch) = self.advance() {
                    if ch == '}' { break; }
                }
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let value: i64 = s.parse().unwrap_or(0);
        Token { kind: TokenKind::Integer(value), line, col }
    }

    fn read_ident_or_keyword(&mut self) -> Token {
        let line = self.line;
        let col = self.col;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let kind = match s.as_str() {
            "program" => TokenKind::Program,
            "var" => TokenKind::Var,
            "function" => TokenKind::Function,
            "begin" => TokenKind::Begin,
            "end" => TokenKind::End,
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "int" => TokenKind::Int,
            "bool" => TokenKind::Bool,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "print" => TokenKind::Print,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            _ => TokenKind::Ident(s),
        };
        Token { kind, line, col }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        let line = self.line;
        let col = self.col;

        match self.peek() {
            None => Token { kind: TokenKind::Eof, line, col },
            Some(c) if c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_alphabetic() => self.read_ident_or_keyword(),
            Some(':') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token { kind: TokenKind::Assign, line, col }
                } else {
                    Token { kind: TokenKind::Colon, line, col }
                }
            }
            Some('<') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token { kind: TokenKind::Le, line, col }
                } else {
                    Token { kind: TokenKind::Lt, line, col }
                }
            }
            Some('>') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token { kind: TokenKind::Ge, line, col }
                } else {
                    Token { kind: TokenKind::Gt, line, col }
                }
            }
            Some('=') => { self.advance(); Token { kind: TokenKind::Eq, line, col } }
            Some('+') => { self.advance(); Token { kind: TokenKind::Plus, line, col } }
            Some('-') => { self.advance(); Token { kind: TokenKind::Minus, line, col } }
            Some('*') => { self.advance(); Token { kind: TokenKind::Star, line, col } }
            Some('/') => { self.advance(); Token { kind: TokenKind::Slash, line, col } }
            Some('(') => { self.advance(); Token { kind: TokenKind::LParen, line, col } }
            Some(')') => { self.advance(); Token { kind: TokenKind::RParen, line, col } }
            Some(';') => { self.advance(); Token { kind: TokenKind::Semicolon, line, col } }
            Some(',') => { self.advance(); Token { kind: TokenKind::Comma, line, col } }
            Some('.') => { self.advance(); Token { kind: TokenKind::Dot, line, col } }
            Some(c) => {
                panic!("Unexpected character '{}' at {}:{}", c, line, col);
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof { break; }
        }
        tokens
    }
}

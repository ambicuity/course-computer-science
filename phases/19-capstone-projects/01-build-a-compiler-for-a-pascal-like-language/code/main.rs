// Build a Compiler for a Pascal-like Language
// Run: rustc main.rs && ./main
//
// Architecture:
//   Source text → Lexer → Token stream → Parser → AST → CodeGen → Stack instructions
//
// This implements a complete compiler pipeline for a small Pascal-like language
// supporting variable assignment, arithmetic expressions, and print statements.

// =============================================================================
// Step 1: The Lexer — converts source text into a stream of tokens
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Int(i64),
    Ident(String),
    Plus,
    Assign,
    Print,
    Semicolon,
    LParen,
    RParen,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    line: usize,
    col: usize,
}

struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied()?;
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
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> i64 {
        let mut n = 0i64;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                n = n * 10 + (ch as i64 - '0' as i64);
                self.advance();
            } else {
                break;
            }
        }
        n
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        let line = self.line;
        let col = self.col;

        match self.peek() {
            None => Token { kind: TokenKind::Eof, line, col },
            Some(ch) => match ch {
                '+' => { self.advance(); Token { kind: TokenKind::Plus, line, col } }
                '=' => { self.advance(); Token { kind: TokenKind::Assign, line, col } }
                ';' => { self.advance(); Token { kind: TokenKind::Semicolon, line, col } }
                '(' => { self.advance(); Token { kind: TokenKind::LParen, line, col } }
                ')' => { self.advance(); Token { kind: TokenKind::RParen, line, col } }
                _ if ch.is_ascii_digit() => {
                    let n = self.read_number();
                    Token { kind: TokenKind::Int(n), line, col }
                }
                _ if ch.is_alphabetic() => {
                    let s = self.read_ident();
                    let kind = match s.as_str() {
                        "print" => TokenKind::Print,
                        _ => TokenKind::Ident(s),
                    };
                    Token { kind, line, col }
                }
                _ => panic!("Unexpected character '{}' at {}:{}", ch, line, col),
            },
        }
    }

    fn tokenize(&mut self) -> Vec<Token> {
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

// =============================================================================
// Step 2: The Parser — converts tokens into an Abstract Syntax Tree (AST)
// =============================================================================

#[derive(Debug, Clone)]
enum Stmt {
    Assign { name: String, value: Expr },
    Print { value: Expr },
}

#[derive(Debug, Clone)]
enum Expr {
    Int(i64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Token {
        let tok = self.advance();
        if &tok.kind != expected {
            panic!("Expected {:?}, got {:?} at {}:{}", expected, tok.kind, tok.line, tok.col);
        }
        tok
    }

    fn parse_program(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while self.peek() != &TokenKind::Eof {
            stmts.push(self.parse_stmt());
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek().clone() {
            TokenKind::Print => {
                self.advance();
                let expr = self.parse_expr();
                self.expect(&TokenKind::Semicolon);
                Stmt::Print { value: expr }
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                self.expect(&TokenKind::Assign);
                let expr = self.parse_expr();
                self.expect(&TokenKind::Semicolon);
                Stmt::Assign { name, value: expr }
            }
            other => panic!("Unexpected token {:?} at statement start", other),
        }
    }

    fn parse_expr(&mut self) -> Expr {
        let mut left = self.parse_atom();
        while self.peek() == &TokenKind::Plus {
            self.advance();
            let right = self.parse_atom();
            left = Expr::Add(Box::new(left), Box::new(right));
        }
        left
    }

    fn parse_atom(&mut self) -> Expr {
        match self.peek().clone() {
            TokenKind::Int(n) => { self.advance(); Expr::Int(n) }
            TokenKind::Ident(name) => { self.advance(); Expr::Var(name) }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr();
                self.expect(&TokenKind::RParen);
                e
            }
            other => panic!("Expected expression, got {:?}", other),
        }
    }
}

// =============================================================================
// Step 3: Code Generation — emits stack-based instructions from the AST
// =============================================================================

struct CodeGen {
    instructions: Vec<String>,
    variables: Vec<String>,
}

impl CodeGen {
    fn new() -> Self {
        CodeGen { instructions: Vec::new(), variables: Vec::new() }
    }

    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Int(n) => self.instructions.push(format!("PUSH {}", n)),
            Expr::Var(name) => self.instructions.push(format!("LOAD {}", name)),
            Expr::Add(a, b) => {
                self.emit_expr(a);
                self.emit_expr(b);
                self.instructions.push("ADD".to_string());
            }
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign { name, value } => {
                self.emit_expr(value);
                self.instructions.push(format!("STORE {}", name));
                if !self.variables.contains(name) {
                    self.variables.push(name.clone());
                }
            }
            Stmt::Print { value } => {
                self.emit_expr(value);
                self.instructions.push("PRINT".to_string());
            }
        }
    }

    fn compile(&mut self, program: &[Stmt]) -> &[String] {
        for stmt in program {
            self.emit_stmt(stmt);
        }
        &self.instructions
    }
}

// =============================================================================
// Step 4: Full Pipeline — wire lexer, parser, and codegen together
// =============================================================================

fn main() {
    let source = r#"
        x = 10 + 20;
        y = x + 5;
        print y;
    "#;

    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    println!("Tokens: {} tokens", tokens.len());

    // Parse
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    println!("AST: {} statements", program.len());

    // Codegen
    let mut codegen = CodeGen::new();
    let instructions = codegen.compile(&program);
    println!("\nGenerated code:");
    for (i, inst) in instructions.iter().enumerate() {
        println!("  {:04}: {}", i, inst);
    }
}

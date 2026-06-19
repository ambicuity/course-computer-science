# Build a Compiler for a Pascal-like Language

> A compiler is a chain of trustworthy translations from syntax to behavior.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 18
**Time:** ~720 minutes

## Learning Objectives

- Design a staged compiler pipeline (lex, parse, analyze, lower, emit).
- Implement a minimal Pascal-like front end and tiny code generator.
- Build tests around syntax errors, symbol checks, and output consistency.
- Produce a maintainable project plan for a multi-week compiler build.

## The Problem

Large capstones fail when teams jump straight to full language features. Someone decides to build a compiler, starts with the parser, discovers they need a type checker, realizes the IR design is wrong, and restarts three times before giving up. The root cause: no incremental delivery plan.

A compiler is not one problem. It is five or six problems chained together: tokenization, parsing, semantic analysis, intermediate representation, optimization, and code generation. Each phase has its own data structures, its own invariants, and its own debugging story. When you blur the boundaries, you get bugs that could live in any phase and take hours to locate.

Successful compiler projects incrementally ship narrow vertical slices with clear interfaces between phases. The first milestone might be: take `print 1 + 2`, lex it, parse it into an AST, and emit three stack-machine instructions. That is a complete compiler for a tiny language. Every subsequent feature (variables, if-statements, loops) extends the pipeline without rewriting it.

## The Concept

A compiler is a pipeline of transformations, each consuming one representation and producing another. The key insight: each intermediate representation (IR) is a contract between phases. If the IR is well-designed, you can test each phase in isolation.

```
Source text
    │
    ▼
┌─────────┐
│  Lexer   │ ──→ Token stream
└─────────┘
    │
    ▼
┌─────────┐
│  Parser  │ ──→ Abstract Syntax Tree (AST)
└─────────┘
    │
    ▼
┌──────────────┐
│ Semantic     │ ──→ Annotated AST + symbol table
│ Analysis     │
└──────────────┘
    │
    ▼
┌──────────────┐
│ Lowering     │ ──→ Intermediate Representation (IR)
└──────────────┘
    │
    ▼
┌──────────────┐
│ Code Gen     │ ──→ Target output (bytecode, stack ops, C, etc.)
└──────────────┘
```

Each box is independently testable. The lexer has no idea what expressions mean. The parser doesn't check types. The code generator doesn't parse text. This separation is what makes compiler projects tractable.

Production compilers add more boxes: optimization passes between lowering and codegen, register allocation, instruction selection, linking. But the pipeline principle stays the same. LLVM's architecture is exactly this: a series of passes over IR, each transforming it toward the target.

## Build It

We build a compiler for a tiny Pascal-like language. The language supports integer literals, addition, variable assignment, and a `print` statement. The target is stack-machine pseudo-code: `PUSH`, `ADD`, `STORE`, `LOAD`, `PRINT`.

### Step 1: The Lexer

The lexer converts source text into a stream of tokens. Each token carries its kind and source location (line, column) for error reporting.

```rust
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
```

### Step 2: The Parser

The parser consumes tokens and produces an AST. We use recursive descent, which maps cleanly to grammar rules.

```rust
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
```

### Step 3: Code Generation

The code generator walks the AST and emits stack-machine instructions. Variables are stored in a named map; `STORE` writes to it, `LOAD` reads from it.

```rust
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
```

### Step 4: Full Pipeline

The main function ties everything together: lex, parse, compile, print.

```rust
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
```

Expected output for the source above:

```
Tokens: 20 tokens
AST: 3 statements

Generated code:
  0000: PUSH 10
  0001: PUSH 20
  0002: ADD
  0003: STORE x
  0004: LOAD x
  0005: PUSH 5
  0006: ADD
  0007: STORE y
  0008: LOAD y
  0009: PRINT
```

## Use It

Compiler architecture patterns here transfer to DSL compilers and static analyzers: clear IR boundaries and deterministic passes reduce complexity. The same pipeline structure shows up in:

- **LLVM**: the most widely used compiler infrastructure. Its pass-based architecture over SSA IR is the industrial version of what we just built. Every optimization is a pass over IR; backends translate IR to machine code.
- **rustc**: Rust's compiler uses a similar pipeline: token stream, HIR (high-level IR), MIR (mid-level IR), and LLVM IR. Each level is a complete, testable representation.
- **SQL query planners**: databases parse SQL into an AST, plan it into a relational algebra tree, optimize the tree, and execute it. Same pipeline, different domain.
- **Babel/SWC**: JavaScript transpilers parse source into an AST, transform it (lowering modern syntax to older syntax), and generate output code.

The key production lesson: **IR design determines everything**. LLVM's IR is its crown jewel. Once you have a good IR, adding a new language frontend or a new target backend is tractable. Without a clean IR, every change ripples through the whole system.

## Read the Source

- [Crafting Interpreters](https://craftinginterpreters.com/) by Robert Nystrom. The best introductory text on building interpreters and compilers. Part II (jlox) and Part III (clox) walk through lexing, parsing, and bytecode compilation step by step.
- [rustc dev guide](https://rustc-dev-guide.rust-lang.org/) — How rustc actually works. Follow the compilation pipeline from source to MIR to LLVM IR. The "Overview" chapter gives the high-level picture; "The parser" and "HIR lowering" chapters are directly relevant to this lesson.
- [LLVM Tutorial: Kaleidoscope](https://llvm.org/docs/tutorial/) — Build a language frontend that emits LLVM IR. The progression (lexer, parser, codegen, optimization, JIT) mirrors our pipeline exactly, but targets a real backend.

## Ship It

- `code/main.rs`: a complete lexer, parser, and stack-machine code generator for a Pascal-like language supporting `print`, assignment, integer literals, and addition.
- `outputs/README.md`: milestone roadmap and validation checklist for extending the compiler with if-statements, loops, and functions.

## Exercises

1. **Easy** — Add `if` expressions. The syntax is `if (condition) { body } else { body }`. For now, treat any non-zero integer as true. Emit `JUMP_IF_ZERO` and `JUMP` instructions for branching.
2. **Medium** — Add scoped variables with shadowing. When a variable is assigned inside a block, it shadows the outer variable. Emit warnings when a variable is used before assignment in the same scope.
3. **Hard** — Replace the pseudo-code backend with a real bytecode emitter. Define a `Chunk` type with a `Vec<u8>` of opcodes and a `Vec<Value>` constant pool. Write a simple VM that executes the bytecode with a stack and a globals map.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| AST | "syntax tree" | A tree data structure where each node represents a construct in the source language. The parser produces it; every later pass consumes it. |
| Lowering | "translation step" | Converting high-level AST constructs into a simpler intermediate representation that is closer to the target. Each lowering step makes the representation less abstract. |
| Symbol table | "name map" | A mapping from identifier names to their declarations (type, scope, location). Used during semantic analysis to resolve variable references and check types. |
| Backend | "code emitter" | The final compiler stage that translates IR into the target format: machine code, bytecode, or (in our case) stack-machine pseudo-instructions. |
| IR | "intermediate representation" | A representation between source and target. A good IR is language-independent and target-independent, allowing the same optimizer to serve multiple frontends and backends. |

## Further Reading

- [Crafting Interpreters](https://craftinginterpreters.com/) — Complete reference for building interpreters from scratch.
- [rustc dev guide](https://rustc-dev-guide.rust-lang.org/) — Official guide to rustc's internals.
- [Engineering a Compiler](https://www.elsevier.com/books/engineering-a-compiler/cooper/978-0-12-088478-5) — Cooper and Torczon. The standard textbook on compiler construction with coverage of SSA, register allocation, and instruction scheduling.

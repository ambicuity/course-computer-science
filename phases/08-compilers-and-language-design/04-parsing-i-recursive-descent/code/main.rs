// Lesson 04 — Parsing I: Recursive Descent
// Recursive descent parser for arithmetic expressions.
// Grammar:
//   expr   → term (('+' | '-') term)*
//   term   → factor (('*' | '/') factor)*
//   factor → '-' factor | atom
//   atom   → NUMBER | '(' expr ')'

use std::fmt;

// ── Token Types ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(i64),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{}", n),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
        }
    }
}

// ── AST Nodes ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(i64),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Paren(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
        }
    }
}

/// Pretty-print an AST with indentation.
pub fn pretty_print(expr: &Expr, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match expr {
        Expr::Number(n) => format!("{}Number({})", pad, n),
        Expr::BinOp { op, left, right } => {
            let mut s = format!("{}BinOp({})\n", pad, op);
            s.push_str(&pretty_print(left, indent + 1));
            s.push('\n');
            s.push_str(&pretty_print(right, indent + 1));
            s
        }
        Expr::UnaryOp { op: UnaryOp::Neg, operand } => {
            let mut s = format!("{}UnaryOp(-)\n", pad);
            s.push_str(&pretty_print(operand, indent + 1));
            s
        }
        Expr::Paren(inner) => {
            let mut s = format!("{}Paren\n", pad);
            s.push_str(&pretty_print(inner, indent + 1));
            s
        }
    }
}

// ── Lexer ───────────────────────────────────────────────────

/// Tokenize an input string into a vector of tokens, tracking byte offsets.
pub fn tokenize(input: &str) -> Result<Vec<(Token, usize)>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some(&(pos, ch)) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '+' => {
                tokens.push((Token::Plus, pos));
                chars.next();
            }
            '-' => {
                tokens.push((Token::Minus, pos));
                chars.next();
            }
            '*' => {
                tokens.push((Token::Star, pos));
                chars.next();
            }
            '/' => {
                tokens.push((Token::Slash, pos));
                chars.next();
            }
            '(' => {
                tokens.push((Token::LParen, pos));
                chars.next();
            }
            ')' => {
                tokens.push((Token::RParen, pos));
                chars.next();
            }
            '0'..='9' => {
                let start = pos;
                let mut num_str = String::new();
                while let Some(&(_, d)) = chars.peek() {
                    if d.is_ascii_digit() {
                        num_str.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let n: i64 = num_str.parse().map_err(|e| format!("Bad number at {}: {}", start, e))?;
                tokens.push((Token::Number(n), start));
            }
            _ => {
                return Err(format!("Unexpected character '{}' at position {}", ch, pos));
            }
        }
    }

    Ok(tokens)
}

// ── Parser ──────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<(Token, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, usize)>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&(Token, usize)> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<(Token, usize)> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    fn error(&self, msg: &str) -> String {
        let pos_str = self
            .current()
            .map(|(_, p)| p.to_string())
            .unwrap_or_else(|| "end".to_string());
        format!("Parse error at position {}: {}", pos_str, msg)
    }

    // ── Grammar rules ──

    /// expr → term (('+' | '-') term)*
    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;

        while let Some((tok, _)) = self.current() {
            let op = match tok {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// term → factor (('*' | '/') factor)*
    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;

        while let Some((tok, _)) = self.current() {
            let op = match tok {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// factor → '-' factor | atom
    fn parse_factor(&mut self) -> Result<Expr, String> {
        if let Some((Token::Minus, _)) = self.current() {
            self.advance();
            let operand = self.parse_factor()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            });
        }
        self.parse_atom()
    }

    /// atom → NUMBER | '(' expr ')'
    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.current() {
            Some((Token::Number(n), _)) => {
                let n = *n;
                self.advance();
                Ok(Expr::Number(n))
            }
            Some((Token::LParen, _)) => {
                self.advance();
                let inner = self.parse_expr()?;
                match self.current() {
                    Some((Token::RParen, _)) => {
                        self.advance();
                        Ok(Expr::Paren(Box::new(inner)))
                    }
                    _ => Err(self.error("expected ')'")),
                }
            }
            Some((tok, pos)) => Err(format!(
                "Parse error at position {}: expected number or '(', found '{}'",
                pos, tok
            )),
            None => Err("Parse error: unexpected end of input".to_string()),
        }
    }
}

/// Parse a token stream into an AST.
pub fn parse(tokens: Vec<(Token, usize)>) -> Result<Expr, String> {
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_expr()?;
    if parser.current().is_some() {
        return Err(parser.error("unexpected trailing tokens"));
    }
    Ok(ast)
}

// ── Main ────────────────────────────────────────────────────

fn main() {
    let examples = vec![
        "3 + 4 * 2",
        "(1 + 2) * 3",
        "-5 + 3",
        "((2 + 3) * (7 - 4)) / 3",
        "1 - -2",
    ];

    for input in examples {
        println!("Input:  {}", input);
        let tokens = match tokenize(input) {
            Ok(t) => t,
            Err(e) => {
                println!("  Lex error: {}\n", e);
                continue;
            }
        };
        let token_display: Vec<String> = tokens.iter().map(|(t, _)| t.to_string()).collect();
        println!("Tokens: [{}]", token_display.join(", "));
        match parse(tokens) {
            Ok(ast) => {
                println!("AST:\n{}", pretty_print(&ast, 0));
            }
            Err(e) => {
                println!("  Error: {}", e);
            }
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Expr {
        let tokens = tokenize(input).unwrap();
        parse(tokens).unwrap()
    }

    #[test]
    fn test_number() {
        assert_eq!(parse_str("42"), Expr::Number(42));
    }

    #[test]
    fn test_addition() {
        let ast = parse_str("1 + 2");
        assert!(matches!(ast, Expr::BinOp { op: BinOp::Add, .. }));
    }

    #[test]
    fn test_precedence() {
        // 1 + 2 * 3 → 1 + (2 * 3)
        let ast = parse_str("1 + 2 * 3");
        if let Expr::BinOp { op: BinOp::Add, right, .. } = ast {
            assert!(matches!(*right, Expr::BinOp { op: BinOp::Mul, .. }));
        } else {
            panic!("Expected Add at root");
        }
    }

    #[test]
    fn test_paren_override() {
        let ast = parse_str("(1 + 2) * 3");
        if let Expr::BinOp { op: BinOp::Mul, left, .. } = ast {
            assert!(matches!(*left, Expr::Paren(_)));
        } else {
            panic!("Expected Mul at root");
        }
    }

    #[test]
    fn test_unary() {
        let ast = parse_str("-3");
        assert!(matches!(ast, Expr::UnaryOp { op: UnaryOp::Neg, .. }));
    }

    #[test]
    fn test_error_missing_paren() {
        let tokens = tokenize("(1 + 2").unwrap();
        assert!(parse(tokens).is_err());
    }
}

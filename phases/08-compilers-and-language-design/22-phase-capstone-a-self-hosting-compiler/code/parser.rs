// parser.rs — Recursive descent parser for pal
//
// Consumes tokens from the lexer and builds an AST.
// Each grammar rule maps to a method.

use crate::lexer::{Lexer, Token, TokenKind};
use crate::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
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

    pub fn parse_program(&mut self) -> Program {
        self.expect(&TokenKind::Program);
        let name = self.expect_ident();
        self.expect(&TokenKind::Semicolon);

        let mut globals = Vec::new();
        let mut functions = Vec::new();

        loop {
            match self.peek() {
                TokenKind::Var => {
                    globals.push(self.parse_var_decl());
                }
                TokenKind::Function => {
                    functions.push(self.parse_func_decl());
                }
                _ => break,
            }
        }

        let body = self.parse_compound();
        self.expect(&TokenKind::Dot);

        Program { name, globals, functions, body }
    }

    fn parse_var_decl(&mut self) -> VarDecl {
        self.expect(&TokenKind::Var);
        let name = self.expect_ident();
        self.expect(&TokenKind::Colon);
        let ty = self.parse_type();
        self.expect(&TokenKind::Semicolon);
        VarDecl { name, ty }
    }

    fn parse_func_decl(&mut self) -> FuncDecl {
        self.expect(&TokenKind::Function);
        let name = self.expect_ident();
        self.expect(&TokenKind::LParen);

        let mut params = Vec::new();
        if *self.peek() != TokenKind::RParen {
            loop {
                let pname = self.expect_ident();
                self.expect(&TokenKind::Colon);
                let pty = self.parse_type();
                params.push(Param { name: pname, ty: pty });
                if *self.peek() != TokenKind::Semicolon { break; }
                self.advance(); // consume ';'
            }
        }
        self.expect(&TokenKind::RParen);
        self.expect(&TokenKind::Colon);
        let return_ty = self.parse_type();
        self.expect(&TokenKind::Semicolon);

        let mut locals = Vec::new();
        while *self.peek() == TokenKind::Var {
            locals.push(self.parse_var_decl());
        }

        let body = self.parse_compound();
        self.expect(&TokenKind::Semicolon);

        FuncDecl { name, params, return_ty, locals, body }
    }

    fn parse_type(&mut self) -> PalType {
        match self.peek() {
            TokenKind::Int => { self.advance(); PalType::Int }
            TokenKind::Bool => { self.advance(); PalType::Bool }
            _ => panic!("Expected type at line {}", self.tokens[self.pos].line),
        }
    }

    fn parse_compound(&mut self) -> Stmt {
        self.expect(&TokenKind::Begin);
        let mut stmts = Vec::new();
        stmts.push(self.parse_stmt());
        while *self.peek() == TokenKind::Semicolon {
            self.advance();
            if *self.peek() == TokenKind::End { break; }
            stmts.push(self.parse_stmt());
        }
        self.expect(&TokenKind::End);
        Stmt::Block(stmts)
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek() {
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Print => self.parse_print(),
            TokenKind::Begin => self.parse_compound(),
            TokenKind::Ident(_) => {
                let name = self.expect_ident();
                self.expect(&TokenKind::Assign);
                let expr = self.parse_expr();
                Stmt::Assign(name, expr)
            }
            _ => panic!("Unexpected token {:?} at line {}", self.peek(), self.tokens[self.pos].line),
        }
    }

    fn parse_if(&mut self) -> Stmt {
        self.expect(&TokenKind::If);
        let cond = self.parse_expr();
        self.expect(&TokenKind::Then);
        let then_stmt = Box::new(self.parse_stmt());
        let else_stmt = if *self.peek() == TokenKind::Else {
            self.advance();
            Some(Box::new(self.parse_stmt()))
        } else {
            None
        };
        Stmt::If(cond, then_stmt, else_stmt)
    }

    fn parse_while(&mut self) -> Stmt {
        self.expect(&TokenKind::While);
        let cond = self.parse_expr();
        self.expect(&TokenKind::Do);
        let body = Box::new(self.parse_stmt());
        Stmt::While(cond, body)
    }

    fn parse_print(&mut self) -> Stmt {
        self.expect(&TokenKind::Print);
        self.expect(&TokenKind::LParen);
        let expr = self.parse_expr();
        self.expect(&TokenKind::RParen);
        Stmt::Print(expr)
    }

    fn parse_expr(&mut self) -> Expr {
        let mut left = self.parse_comp();
        loop {
            let op = match self.peek() {
                TokenKind::And => { self.advance(); BinOp::And }
                TokenKind::Or => { self.advance(); BinOp::Or }
                _ => break,
            };
            let right = self.parse_comp();
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_comp(&mut self) -> Expr {
        let mut left = self.parse_arith();
        let op = match self.peek() {
            TokenKind::Eq => Some(BinOp::Eq),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::Le => Some(BinOp::Le),
            TokenKind::Ge => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let right = self.parse_arith();
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_arith(&mut self) -> Expr {
        let mut left = self.parse_term();
        loop {
            let op = match self.peek() {
                TokenKind::Plus => { self.advance(); BinOp::Add }
                TokenKind::Minus => { self.advance(); BinOp::Sub }
                _ => break,
            };
            let right = self.parse_term();
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_term(&mut self) -> Expr {
        let mut left = self.parse_factor();
        loop {
            let op = match self.peek() {
                TokenKind::Star => { self.advance(); BinOp::Mul }
                TokenKind::Slash => { self.advance(); BinOp::Div }
                _ => break,
            };
            let right = self.parse_factor();
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_factor(&mut self) -> Expr {
        match self.peek() {
            TokenKind::Integer(n) => { let n = *n; self.advance(); Expr::IntLit(n) }
            TokenKind::True => { self.advance(); Expr::BoolLit(true) }
            TokenKind::False => { self.advance(); Expr::BoolLit(false) }
            TokenKind::Ident(_) => {
                let name = self.expect_ident();
                if *self.peek() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.peek() != TokenKind::RParen {
                        args.push(self.parse_expr());
                        while *self.peek() == TokenKind::Comma {
                            self.advance();
                            args.push(self.parse_expr());
                        }
                    }
                    self.expect(&TokenKind::RParen);
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr();
                self.expect(&TokenKind::RParen);
                expr
            }
            _ => panic!("Unexpected token {:?} at line {}", self.peek(), self.tokens[self.pos].line),
        }
    }

    fn expect_ident(&mut self) -> String {
        match self.peek() {
            TokenKind::Ident(_) => {
                if let TokenKind::Ident(s) = self.advance().kind {
                    s
                } else { unreachable!() }
            }
            _ => panic!("Expected identifier at line {}", self.tokens[self.pos].line),
        }
    }
}

//! Intermediate Representation — Three-Address Code
//! Phase 08 — Compilers & Programming Language Design
//!
//! Lowering a simple AST to three-address code instructions.

use std::fmt;

// ── IR Instructions ────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Instr {
    Assign(String, String),                    // dest = src
    BinOp(String, String, BinOp, String),      // dest = src1 op src2
    UnaryOp(String, UnaryOp, String),          // dest = op src
    Label(String),                             // L:
    Goto(String),                              // goto L
    IfGoto(String, String),                    // if cond goto L
    Param(String),                             // param x
    Call(String, String, usize),               // dest = call f, n_params
    Return(String),                            // return x
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Eq  => write!(f, "=="),
            BinOp::Neq => write!(f, "!="),
            BinOp::Lt  => write!(f, "<"),
            BinOp::Gt  => write!(f, ">"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UnaryOp {
    Neg,
    Not,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::Assign(d, s)          => write!(f, "  {} = {}", d, s),
            Instr::BinOp(d, a, op, b)    => write!(f, "  {} = {} {} {}", d, a, op, b),
            Instr::UnaryOp(d, op, a)     => write!(f, "  {} = {}{}", d, op, a),
            Instr::Label(l)              => write!(f, "{}:", l),
            Instr::Goto(l)               => write!(f, "  goto {}", l),
            Instr::IfGoto(c, l)          => write!(f, "  if {} goto {}", c, l),
            Instr::Param(a)              => write!(f, "  param {}", a),
            Instr::Call(d, f_name, n)    => write!(f, "  {} = call {}, {}", d, f_name, n),
            Instr::Return(a)             => write!(f, "  return {}", a),
        }
    }
}

// ── AST ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Expr {
    IntLit(i64),
    Var(String),
    BinOp(Box<Expr>, AstBinOp, Box<Expr>),
    UnaryOp(AstUnaryOp, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
enum AstBinOp {
    Add, Sub, Mul, Div,
    Eq, Neq, Lt, Gt,
}

#[derive(Debug, Clone)]
enum AstUnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
enum Stmt {
    VarDecl(String),
    Assign(String, Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),  // cond, then, else
    While(Expr, Vec<Stmt>),
    Return(Expr),
    ExprStmt(Expr),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
struct FuncDecl {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
}

// ── IR Generator ───────────────────────────────────────────────────

struct IRGenerator {
    instructions: Vec<Instr>,
    temp_counter: usize,
    label_counter: usize,
}

impl IRGenerator {
    fn new() -> Self {
        IRGenerator {
            instructions: Vec::new(),
            temp_counter: 0,
            label_counter: 0,
        }
    }

    fn fresh_temp(&mut self) -> String {
        let name = format!("t{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    fn fresh_label(&mut self) -> String {
        let name = format!("L{}", self.label_counter);
        self.label_counter += 1;
        name
    }

    fn emit(&mut self, instr: Instr) {
        self.instructions.push(instr);
    }

    fn convert_binop(op: &AstBinOp) -> BinOp {
        match op {
            AstBinOp::Add => BinOp::Add,
            AstBinOp::Sub => BinOp::Sub,
            AstBinOp::Mul => BinOp::Mul,
            AstBinOp::Div => BinOp::Div,
            AstBinOp::Eq  => BinOp::Eq,
            AstBinOp::Neq => BinOp::Neq,
            AstBinOp::Lt  => BinOp::Lt,
            AstBinOp::Gt  => BinOp::Gt,
        }
    }

    fn convert_unop(op: &AstUnaryOp) -> UnaryOp {
        match op {
            AstUnaryOp::Neg => UnaryOp::Neg,
            AstUnaryOp::Not => UnaryOp::Not,
        }
    }

    /// Generate IR for an expression. Returns the name holding the result.
    fn gen_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::IntLit(n) => {
                let t = self.fresh_temp();
                self.emit(Instr::Assign(t.clone(), n.to_string()));
                t
            }
            Expr::Var(name) => {
                // Variables are already names; no instruction needed
                name.clone()
            }
            Expr::BinOp(l, op, r) => {
                let left = self.gen_expr(l);
                let right = self.gen_expr(r);
                let t = self.fresh_temp();
                self.emit(Instr::BinOp(
                    t.clone(),
                    left,
                    Self::convert_binop(op),
                    right,
                ));
                t
            }
            Expr::UnaryOp(op, e) => {
                let operand = self.gen_expr(e);
                let t = self.fresh_temp();
                self.emit(Instr::UnaryOp(t.clone(), Self::convert_unop(op), operand));
                t
            }
            Expr::Call(name, args) => {
                // Evaluate arguments and emit param instructions
                let mut arg_temps = Vec::new();
                for a in args {
                    arg_temps.push(self.gen_expr(a));
                }
                for at in &arg_temps {
                    self.emit(Instr::Param(at.clone()));
                }
                let t = self.fresh_temp();
                self.emit(Instr::Call(t.clone(), name.clone(), args.len()));
                t
            }
        }
    }

    /// Generate IR for a statement.
    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(_) => {
                // Declarations don't produce IR; they are tracked in the symbol table.
            }
            Stmt::Assign(name, expr) => {
                let src = self.gen_expr(expr);
                self.emit(Instr::Assign(name.clone(), src));
            }
            Stmt::If(cond, then_body, else_body) => {
                let c = self.gen_expr(cond);
                match else_body {
                    Some(else_stmts) => {
                        let l_else = self.fresh_label();
                        let l_end = self.fresh_label();
                        // if c == 0 goto L_else
                        let zero = self.fresh_temp();
                        self.emit(Instr::Assign(zero.clone(), "0".into()));
                        let test = self.fresh_temp();
                        self.emit(Instr::BinOp(test.clone(), c, BinOp::Eq, zero));
                        self.emit(Instr::IfGoto(test, l_else.clone()));

                        for s in then_body {
                            self.gen_stmt(s);
                        }
                        self.emit(Instr::Goto(l_end.clone()));

                        self.emit(Instr::Label(l_else));
                        for s in else_stmts {
                            self.gen_stmt(s);
                        }
                        self.emit(Instr::Label(l_end));
                    }
                    None => {
                        let l_end = self.fresh_label();
                        let zero = self.fresh_temp();
                        self.emit(Instr::Assign(zero.clone(), "0".into()));
                        let test = self.fresh_temp();
                        self.emit(Instr::BinOp(test.clone(), c, BinOp::Eq, zero));
                        self.emit(Instr::IfGoto(test, l_end.clone()));

                        for s in then_body {
                            self.gen_stmt(s);
                        }
                        self.emit(Instr::Label(l_end));
                    }
                }
            }
            Stmt::While(cond, body) => {
                let l_cond = self.fresh_label();
                let l_end = self.fresh_label();

                self.emit(Instr::Label(l_cond.clone()));
                let c = self.gen_expr(cond);
                let zero = self.fresh_temp();
                self.emit(Instr::Assign(zero.clone(), "0".into()));
                let test = self.fresh_temp();
                self.emit(Instr::BinOp(test.clone(), c, BinOp::Eq, zero));
                self.emit(Instr::IfGoto(test, l_end.clone()));

                for s in body {
                    self.gen_stmt(s);
                }
                self.emit(Instr::Goto(l_cond));
                self.emit(Instr::Label(l_end));
            }
            Stmt::Return(expr) => {
                let t = self.gen_expr(expr);
                self.emit(Instr::Return(t));
            }
            Stmt::ExprStmt(expr) => {
                self.gen_expr(expr);
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.gen_stmt(s);
                }
            }
        }
    }

    /// Generate IR for an entire function.
    fn generate(&mut self, func: &FuncDecl) {
        self.emit(Instr::Label(func.name.clone()));
        for s in &func.body {
            self.gen_stmt(s);
        }
    }

    fn dump(&self) {
        for instr in &self.instructions {
            println!("{}", instr);
        }
    }
}

// ── Demonstration ──────────────────────────────────────────────────

fn main() {
    println!("=== Lesson 12: Intermediate Representation — Three-Address Code ===\n");

    // --- Demo 1: Expression flattening ---
    println!("--- Demo 1: Flatten a * b + c * d ---");
    {
        // (a * b) + (c * d)
        let expr = Expr::BinOp(
            Box::new(Expr::BinOp(
                Box::new(Expr::Var("a".into())),
                AstBinOp::Mul,
                Box::new(Expr::Var("b".into())),
            )),
            AstBinOp::Add,
            Box::new(Expr::BinOp(
                Box::new(Expr::Var("c".into())),
                AstBinOp::Mul,
                Box::new(Expr::Var("d".into())),
            )),
        );
        let mut gen = IRGenerator::new();
        let result = gen.gen_expr(&expr);
        println!("  Result: {}", result);
        gen.dump();
    }

    // --- Demo 2: Function with if/else ---
    println!("\n--- Demo 2: Function with if/else ---");
    {
        let func = FuncDecl {
            name: "abs".into(),
            params: vec!["x".into()],
            body: vec![
                Stmt::If(
                    Expr::BinOp(
                        Box::new(Expr::Var("x".into())),
                        AstBinOp::Lt,
                        Box::new(Expr::IntLit(0)),
                    ),
                    vec![
                        Stmt::Return(Expr::UnaryOp(
                            AstUnaryOp::Neg,
                            Box::new(Expr::Var("x".into())),
                        )),
                    ],
                    Some(vec![
                        Stmt::Return(Expr::Var("x".into())),
                    ]),
                ),
            ],
        };
        let mut gen = IRGenerator::new();
        gen.generate(&func);
        gen.dump();
    }

    // --- Demo 3: While loop ---
    println!("\n--- Demo 3: While loop — factorial body ---");
    {
        // while (i > 1) { result = result * i; i = i - 1; }
        let func = FuncDecl {
            name: "factorial_loop".into(),
            params: vec!["n".into()],
            body: vec![
                Stmt::Assign("result".into(), Expr::IntLit(1)),
                Stmt::Assign("i".into(), Expr::Var("n".into())),
                Stmt::While(
                    Expr::BinOp(
                        Box::new(Expr::Var("i".into())),
                        AstBinOp::Gt,
                        Box::new(Expr::IntLit(1)),
                    ),
                    vec![
                        Stmt::Assign(
                            "result".into(),
                            Expr::BinOp(
                                Box::new(Expr::Var("result".into())),
                                AstBinOp::Mul,
                                Box::new(Expr::Var("i".into())),
                            ),
                        ),
                        Stmt::Assign(
                            "i".into(),
                            Expr::BinOp(
                                Box::new(Expr::Var("i".into())),
                                AstBinOp::Sub,
                                Box::new(Expr::IntLit(1)),
                            ),
                        ),
                    ],
                ),
                Stmt::Return(Expr::Var("result".into())),
            ],
        };
        let mut gen = IRGenerator::new();
        gen.generate(&func);
        gen.dump();
    }

    // --- Demo 4: Function call ---
    println!("\n--- Demo 4: Function call — add(3, 4) ---");
    {
        let func = FuncDecl {
            name: "main".into(),
            params: vec![],
            body: vec![
                Stmt::Assign(
                    "x".into(),
                    Expr::Call(
                        "add".into(),
                        vec![Expr::IntLit(3), Expr::IntLit(4)],
                    ),
                ),
                Stmt::Return(Expr::Var("x".into())),
            ],
        };
        let mut gen = IRGenerator::new();
        gen.generate(&func);
        gen.dump();
    }

    println!("\n=== Done ===");
}

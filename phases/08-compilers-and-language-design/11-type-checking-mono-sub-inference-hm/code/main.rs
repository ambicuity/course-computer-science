//! Type Checking — Monomorphic Type Checker
//! Phase 08 — Compilers & Programming Language Design
//!
//! A monomorphic type checker that walks an AST, maintains a type
//! environment, and reports type mismatches.

use std::collections::HashMap;

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Type {
    TInt,
    TBool,
    TFunc(Vec<Type>, Box<Type>), // param types → return type
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::TInt => write!(f, "int"),
            Type::TBool => write!(f, "bool"),
            Type::TFunc(params, ret) => {
                let ps: Vec<String> = params.iter().map(|t| t.to_string()).collect();
                write!(f, "fn({})->{}", ps.join(", "), ret)
            }
        }
    }
}

// ── AST ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Expr {
    IntLit(i64),
    BoolLit(bool),
    Var(String),
    BinOp(Box<Expr>, BinOpcode, Box<Expr>),
    Call(String, Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
enum BinOpcode {
    Add,
    Sub,
    Mul,
    Eq,
}

#[derive(Debug, Clone)]
enum Stmt {
    VarDecl(String, Type),
    FuncDecl(String, Vec<(String, Type)>, Type, Vec<Stmt>), // name, params, ret_type, body
    Return(Expr),
    ExprStmt(Expr),
}

// ── Type Environment ───────────────────────────────────────────────

type TypeEnv = HashMap<String, Type>;

// ── Type Errors ────────────────────────────────────────────────────

#[derive(Debug)]
struct TypeError {
    message: String,
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type error: {}", self.message)
    }
}

// ── Type Checker ───────────────────────────────────────────────────

struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    fn new() -> Self {
        TypeChecker {
            env: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn check_program(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.check_stmt(s);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(name, ty) => {
                self.env.insert(name.clone(), ty.clone());
            }
            Stmt::FuncDecl(name, params, ret_type, body) => {
                // Register the function type in the outer env
                let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                let func_type = Type::TFunc(param_types, Box::new(ret_type.clone()));
                self.env.insert(name.clone(), func_type);

                // Create inner env for the function body
                let saved_env = self.env.clone();
                for (pname, pty) in params {
                    self.env.insert(pname.clone(), pty.clone());
                }

                // Check body statements
                for s in body {
                    self.check_stmt(s);
                }

                self.env = saved_env;
            }
            Stmt::Return(expr) => {
                self.check_expr(expr);
            }
            Stmt::ExprStmt(expr) => {
                self.check_expr(expr);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::IntLit(_) => Some(Type::TInt),
            Expr::BoolLit(_) => Some(Type::TBool),

            Expr::Var(name) => match self.env.get(name) {
                Some(ty) => Some(ty.clone()),
                None => {
                    self.errors.push(TypeError {
                        message: format!("undeclared variable '{}'", name),
                    });
                    None
                }
            },

            Expr::BinOp(l, op, r) => {
                let lt = self.check_expr(l);
                let rt = self.check_expr(r);
                match op {
                    BinOpcode::Add | BinOpcode::Sub | BinOpcode::Mul => {
                        if let (Some(lt), Some(rt)) = (&lt, &rt) {
                            if lt != &Type::TInt || rt != &Type::TInt {
                                self.errors.push(TypeError {
                                    message: format!(
                                        "arithmetic operator requires int, got {} and {}",
                                        lt, rt
                                    ),
                                });
                            }
                        }
                        Some(Type::TInt)
                    }
                    BinOpcode::Eq => {
                        if let (Some(lt), Some(rt)) = (&lt, &rt) {
                            if lt != rt {
                                self.errors.push(TypeError {
                                    message: format!(
                                        "== requires matching types, got {} and {}",
                                        lt, rt
                                    ),
                                });
                            }
                        }
                        Some(Type::TBool)
                    }
                }
            }

            Expr::Call(name, args) => match self.env.get(name).cloned() {
                Some(Type::TFunc(param_types, ret_type)) => {
                    if args.len() != param_types.len() {
                        self.errors.push(TypeError {
                            message: format!(
                                "'{}' expects {} args, got {}",
                                name,
                                param_types.len(),
                                args.len()
                            ),
                        });
                    } else {
                        for (i, (arg, expected)) in
                            args.iter().zip(param_types.iter()).enumerate()
                        {
                            if let Some(actual) = self.check_expr(arg) {
                                if actual != *expected {
                                    self.errors.push(TypeError {
                                        message: format!(
                                            "argument {} of '{}': expected {}, got {}",
                                            i + 1, name, expected, actual
                                        ),
                                    });
                                }
                            }
                        }
                        return Some(*ret_type);
                    }
                    // Even on error, check remaining args
                    for a in args {
                        self.check_expr(a);
                    }
                    Some(*ret_type)
                }
                Some(other) => {
                    self.errors.push(TypeError {
                        message: format!("'{}' is not a function, it is {}", name, other),
                    });
                    for a in args {
                        self.check_expr(a);
                    }
                    None
                }
                None => {
                    self.errors.push(TypeError {
                        message: format!("undeclared function '{}'", name),
                    });
                    for a in args {
                        self.check_expr(a);
                    }
                    None
                }
            },

            Expr::If(cond, then_b, else_b) => {
                let ct = self.check_expr(cond);
                if let Some(ref ty) = ct {
                    if ty != &Type::TBool {
                        self.errors.push(TypeError {
                            message: format!("if condition must be bool, got {}", ty),
                        });
                    }
                }
                let tt = self.check_expr(then_b);
                let et = self.check_expr(else_b);
                match (tt, et) {
                    (Some(ref t), Some(ref e)) if t == e => Some(t.clone()),
                    (Some(t), Some(e)) => {
                        self.errors.push(TypeError {
                            message: format!(
                                "if/else branches have mismatched types: {} vs {}",
                                t, e
                            ),
                        });
                        None
                    }
                    _ => None,
                }
            }
        }
    }
}

// ── Demonstration ──────────────────────────────────────────────────

fn main() {
    println!("=== Lesson 11: Type Checking — Monomorphic Checker ===\n");

    // --- Demo 1: Well-typed program ---
    println!("--- Demo 1: Well-typed program ---");
    {
        let program = vec![
            Stmt::VarDecl("x".into(), Type::TInt),
            Stmt::VarDecl("y".into(), Type::TInt),
            Stmt::FuncDecl(
                "add".into(),
                vec![("a".into(), Type::TInt), ("b".into(), Type::TInt)],
                Type::TInt,
                vec![Stmt::Return(Expr::BinOp(
                    Box::new(Expr::Var("a".into())),
                    BinOpcode::Add,
                    Box::new(Expr::Var("b".into())),
                ))],
            ),
            Stmt::ExprStmt(Expr::Call(
                "add".into(),
                vec![Expr::Var("x".into()), Expr::Var("y".into())],
            )),
        ];
        let mut checker = TypeChecker::new();
        checker.check_program(&program);
        if checker.errors.is_empty() {
            println!("  No type errors.");
        }
    }

    // --- Demo 2: Type errors ---
    println!("\n--- Demo 2: Program with type errors ---");
    {
        let program = vec![
            Stmt::VarDecl("x".into(), Type::TInt),
            Stmt::VarDecl("flag".into(), Type::TBool),
            Stmt::FuncDecl(
                "double".into(),
                vec![("n".into(), Type::TInt)],
                Type::TInt,
                vec![Stmt::Return(Expr::BinOp(
                    Box::new(Expr::Var("n".into())),
                    BinOpcode::Mul,
                    Box::new(Expr::IntLit(2)),
                ))],
            ),
            // Error: adding int + bool
            Stmt::ExprStmt(Expr::BinOp(
                Box::new(Expr::Var("x".into())),
                BinOpcode::Add,
                Box::new(Expr::Var("flag".into())),
            )),
            // Error: wrong arg type
            Stmt::ExprStmt(Expr::Call(
                "double".into(),
                vec![Expr::BoolLit(true)],
            )),
            // Error: wrong arg count
            Stmt::ExprStmt(Expr::Call(
                "double".into(),
                vec![Expr::IntLit(1), Expr::IntLit(2)],
            )),
        ];
        let mut checker = TypeChecker::new();
        checker.check_program(&program);
        println!("  Found {} type errors:", checker.errors.len());
        for e in &checker.errors {
            println!("    - {}", e);
        }
    }

    // --- Demo 3: If/else type checking ---
    println!("\n--- Demo 3: If/else type checking ---");
    {
        // Well-typed: both branches are int
        let good = Expr::If(
            Box::new(Expr::BoolLit(true)),
            Box::new(Expr::IntLit(1)),
            Box::new(Expr::IntLit(2)),
        );
        let mut checker = TypeChecker::new();
        if let Some(t) = checker.check_expr(&good) {
            println!("  if true {{ 1 }} else {{ 2 }} : {}", t);
        }

        // Ill-typed: branches differ
        let bad = Expr::If(
            Box::new(Expr::IntLit(1)), // condition is int, not bool
            Box::new(Expr::IntLit(1)),
            Box::new(Expr::BoolLit(false)), // branches differ
        );
        let mut checker2 = TypeChecker::new();
        checker2.check_expr(&bad);
        println!("  Errors from mismatched if:");
        for e in &checker2.errors {
            println!("    - {}", e);
        }
    }

    println!("\n=== Done ===");
}

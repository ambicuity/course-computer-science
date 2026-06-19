//! Semantic Analysis — Symbol Tables, Scopes
//! Phase 08 — Compilers & Programming Language Design
//!
//! A complete symbol table with nested scopes and a semantic checker
//! that detects undeclared variables, duplicate declarations, and
//! function argument-count mismatches.

use std::collections::HashMap;

// ── Symbol Table Types ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SymbolKind {
    Var,
    Func,
    Type,
}

#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    kind: SymbolKind,
    type_info: String,
    scope_id: usize,
}

struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<usize>,
}

struct SymbolTable {
    scopes: Vec<Scope>,
    current: usize,
}

impl SymbolTable {
    fn new() -> Self {
        let global = Scope {
            symbols: HashMap::new(),
            parent: None,
        };
        SymbolTable {
            scopes: vec![global],
            current: 0,
        }
    }

    fn enter_scope(&mut self) {
        let parent = self.current;
        self.scopes.push(Scope {
            symbols: HashMap::new(),
            parent: Some(parent),
        });
        self.current = self.scopes.len() - 1;
    }

    fn exit_scope(&mut self) {
        let parent = self.scopes[self.current].parent;
        self.current = parent.expect("cannot exit global scope");
    }

    fn declare(&mut self, sym: Symbol) -> Result<(), String> {
        let scope = &mut self.scopes[self.current];
        if scope.symbols.contains_key(&sym.name) {
            return Err(format!("duplicate declaration of '{}'", sym.name));
        }
        scope.symbols.insert(sym.name.clone(), sym);
        Ok(())
    }

    fn resolve(&self, name: &str) -> Option<&Symbol> {
        let mut scope_id = Some(self.current);
        while let Some(id) = scope_id {
            if let Some(sym) = self.scopes[id].symbols.get(name) {
                return Some(sym);
            }
            scope_id = self.scopes[id].parent;
        }
        None
    }

    fn dump(&self) {
        for (i, scope) in self.scopes.iter().enumerate() {
            let parent_str = match scope.parent {
                Some(p) => format!("{}", p),
                None => "none".to_string(),
            };
            println!("  scope {} (parent={}):", i, parent_str);
            for (name, sym) in &scope.symbols {
                println!("    {:?} {} : {}", sym.kind, name, sym.type_info);
            }
        }
    }
}

// ── AST ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Expr {
    Var(String),
    IntLit(i64),
    BinOp(Box<Expr>, String, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
enum Stmt {
    VarDecl(String, String),
    FuncDecl(String, Vec<(String, String)>, Vec<Stmt>),
    Assign(String, Expr),
    ExprStmt(Expr),
    Block(Vec<Stmt>),
}

// ── Semantic Checker ────────────────────────────────────────────────

struct SemanticChecker {
    table: SymbolTable,
    errors: Vec<String>,
}

impl SemanticChecker {
    fn new() -> Self {
        SemanticChecker {
            table: SymbolTable::new(),
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
                let sym = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Var,
                    type_info: ty.clone(),
                    scope_id: self.table.current,
                };
                if let Err(e) = self.table.declare(sym) {
                    self.errors.push(e);
                }
            }
            Stmt::FuncDecl(name, params, body) => {
                let param_types: Vec<&str> =
                    params.iter().map(|(_, t)| t.as_str()).collect();
                let sig = format!("fn({})->?", param_types.join(","));
                let sym = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Func,
                    type_info: sig,
                    scope_id: self.table.current,
                };
                if let Err(e) = self.table.declare(sym) {
                    self.errors.push(e);
                    return;
                }
                self.table.enter_scope();
                for (pname, pty) in params {
                    let psym = Symbol {
                        name: pname.clone(),
                        kind: SymbolKind::Var,
                        type_info: pty.clone(),
                        scope_id: self.table.current,
                    };
                    let _ = self.table.declare(psym);
                }
                for s in body {
                    self.check_stmt(s);
                }
                self.table.exit_scope();
            }
            Stmt::Block(stmts) => {
                self.table.enter_scope();
                for s in stmts {
                    self.check_stmt(s);
                }
                self.table.exit_scope();
            }
            Stmt::Assign(name, expr) => {
                if self.table.resolve(name).is_none() {
                    self.errors.push(format!("undeclared variable '{}'", name));
                }
                self.check_expr(expr);
            }
            Stmt::ExprStmt(expr) => {
                self.check_expr(expr);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Var(name) => {
                if self.table.resolve(name).is_none() {
                    self.errors.push(format!("undeclared variable '{}'", name));
                }
            }
            Expr::IntLit(_) => {}
            Expr::BinOp(l, _op, r) => {
                self.check_expr(l);
                self.check_expr(r);
            }
            Expr::Call(name, args) => {
                match self.table.resolve(name) {
                    Some(sym) if sym.kind == SymbolKind::Func => {
                        let expected = if sym.type_info == "fn()->?" {
                            0
                        } else {
                            sym.type_info.matches(',').count() + 1
                        };
                        if args.len() != expected {
                            self.errors.push(format!(
                                "'{}' expects {} args, got {}",
                                name,
                                expected,
                                args.len()
                            ));
                        }
                    }
                    Some(sym) => {
                        self.errors.push(format!(
                            "'{}' is a {:?}, not a function",
                            name, sym.kind
                        ));
                    }
                    None => {
                        self.errors.push(format!("undeclared function '{}'", name));
                    }
                }
                for a in args {
                    self.check_expr(a);
                }
            }
        }
    }
}

// ── Demonstration ───────────────────────────────────────────────────

fn main() {
    println!("=== Lesson 10: Semantic Analysis — Symbol Tables, Scopes ===\n");

    // --- Demo 1: Symbol table basics ---
    println!("--- Demo 1: Symbol table with nested scopes ---");
    {
        let mut table = SymbolTable::new();
        let _ = table.declare(Symbol {
            name: "x".into(),
            kind: SymbolKind::Var,
            type_info: "int".into(),
            scope_id: 0,
        });
        let _ = table.declare(Symbol {
            name: "printf".into(),
            kind: SymbolKind::Func,
            type_info: "fn(str)->int".into(),
            scope_id: 0,
        });

        table.enter_scope();
        let _ = table.declare(Symbol {
            name: "y".into(),
            kind: SymbolKind::Var,
            type_info: "int".into(),
            scope_id: 1,
        });
        let _ = table.declare(Symbol {
            name: "x".into(), // shadowed
            kind: SymbolKind::Var,
            type_info: "bool".into(),
            scope_id: 1,
        });

        println!("  resolve 'x' from inner scope: {:?}", table.resolve("x").unwrap().type_info);
        println!("  resolve 'y' from inner scope: {:?}", table.resolve("y").unwrap().type_info);
        println!("  resolve 'printf' from inner scope: {:?}", table.resolve("printf").unwrap().type_info);
        println!("  resolve 'z': {:?}", table.resolve("z"));

        table.exit_scope();
        println!("  resolve 'x' after exiting inner scope: {:?}", table.resolve("x").unwrap().type_info);
        println!("\nSymbol table dump:");
        table.dump();
    }

    // --- Demo 2: Correct program ---
    println!("\n--- Demo 2: Checking a correct program ---");
    {
        let program = vec![
            Stmt::VarDecl("x".into(), "int".into()),
            Stmt::VarDecl("y".into(), "int".into()),
            Stmt::FuncDecl(
                "add".into(),
                vec![("a".into(), "int".into()), ("b".into(), "int".into())],
                vec![
                    Stmt::Assign(
                        "x".into(),
                        Expr::BinOp(
                            Box::new(Expr::Var("a".into())),
                            "+".into(),
                            Box::new(Expr::Var("b".into())),
                        ),
                    ),
                ],
            ),
            Stmt::Assign(
                "y".into(),
                Expr::Call(
                    "add".into(),
                    vec![Expr::Var("x".into()), Expr::IntLit(10)],
                ),
            ),
        ];
        let mut checker = SemanticChecker::new();
        checker.check_program(&program);
        if checker.errors.is_empty() {
            println!("  No errors — program is semantically valid.");
        } else {
            for e in &checker.errors {
                println!("  ERROR: {}", e);
            }
        }
    }

    // --- Demo 3: Program with errors ---
    println!("\n--- Demo 3: Checking a program with semantic errors ---");
    {
        let program = vec![
            Stmt::VarDecl("x".into(), "int".into()),
            Stmt::VarDecl("x".into(), "int".into()), // duplicate
            Stmt::Assign(
                "y".into(), // undeclared
                Expr::BinOp(
                    Box::new(Expr::Var("x".into())),
                    "+".into(),
                    Box::new(Expr::Var("z".into())), // undeclared
                ),
            ),
            Stmt::FuncDecl(
                "f".into(),
                vec![("a".into(), "int".into())],
                vec![],
            ),
            Stmt::ExprStmt(Expr::Call(
                "f".into(),
                vec![Expr::IntLit(1), Expr::IntLit(2)], // wrong arity
            )),
            Stmt::ExprStmt(Expr::Call(
                "g".into(), // undeclared function
                vec![],
            )),
        ];
        let mut checker = SemanticChecker::new();
        checker.check_program(&program);
        println!("  Found {} errors:", checker.errors.len());
        for e in &checker.errors {
            println!("    - {}", e);
        }
    }

    // --- Demo 4: Block scoping ---
    println!("\n--- Demo 4: Block scoping — variable not visible outside ---");
    {
        let program = vec![
            Stmt::VarDecl("a".into(), "int".into()),
            Stmt::Block(vec![
                Stmt::VarDecl("b".into(), "int".into()),
                Stmt::Assign(
                    "a".into(),
                    Expr::Var("b".into()),
                ),
            ]),
            Stmt::Assign(
                "a".into(),
                Expr::Var("b".into()), // b is out of scope
            ),
        ];
        let mut checker = SemanticChecker::new();
        checker.check_program(&program);
        println!("  Found {} errors:", checker.errors.len());
        for e in &checker.errors {
            println!("    - {}", e);
        }
    }

    println!("\n=== Done ===");
}

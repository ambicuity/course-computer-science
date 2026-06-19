// Lesson 09 — AST Design and Visitor Patterns
// AST for a mini-language + three visitor implementations:
//   PrettyPrinter, Interpreter, TypeChecker

use std::collections::HashMap;
use std::fmt;

// ── AST Nodes ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(i64),
    Str(String),
    Bool(bool),
    Var(String),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Option<Box<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    ExprStmt(Expr),
    VarDecl {
        name: String,
        value: Expr,
    },
    FuncDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Block(Vec<Stmt>),
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Print(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Neq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

// ── Value Type (for Interpreter) ────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
    Nil,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
        }
    }
}

// ── Type (for TypeChecker) ──────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Str,
    Bool,
    Nil,
    Func { params: Vec<Type>, ret: Box<Type> },
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Str => write!(f, "str"),
            Type::Bool => write!(f, "bool"),
            Type::Nil => write!(f, "nil"),
            Type::Func { params, ret } => {
                let param_str: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}) -> {}", param_str.join(", "), ret)
            }
        }
    }
}

// ── Visitor Trait ───────────────────────────────────────────

pub trait Visitor {
    type Output;
    type Error: fmt::Debug;

    fn visit_expr(&mut self, expr: &Expr) -> Result<Self::Output, Self::Error>;
    fn visit_stmt(&mut self, stmt: &Stmt) -> Result<Self::Output, Self::Error>;
}

// ── PrettyPrinter ──────────────────────────────────────────

pub struct PrettyPrinter {
    indent: usize,
}

impl PrettyPrinter {
    pub fn new() -> Self {
        PrettyPrinter { indent: 0 }
    }

    fn pad(&self) -> String {
        "  ".repeat(self.indent)
    }

    pub fn print_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Number(n) => n.to_string(),
            Expr::Str(s) => format!("\"{}\"", s),
            Expr::Bool(b) => b.to_string(),
            Expr::Var(name) => name.clone(),
            Expr::BinOp { op, left, right } => {
                format!("({} {} {})", self.print_expr(left), op, self.print_expr(right))
            }
            Expr::UnaryOp { op, operand } => {
                format!("({}{})", op, self.print_expr(operand))
            }
            Expr::Call { func, args } => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.print_expr(a)).collect();
                format!("{}({})", func, arg_strs.join(", "))
            }
            Expr::If { cond, then, else_ } => {
                let mut s = format!("if {} {{ {} }}", self.print_expr(cond), self.print_expr(then));
                if let Some(e) = else_ {
                    s.push_str(&format!(" else {{ {} }}", self.print_expr(e)));
                }
                s
            }
        }
    }

    pub fn print_stmt(&mut self, stmt: &Stmt) -> String {
        let p = self.pad();
        match stmt {
            Stmt::ExprStmt(expr) => format!("{}{};", p, self.print_expr(expr)),
            Stmt::VarDecl { name, value } => {
                format!("{}let {} = {};", p, name, self.print_expr(value))
            }
            Stmt::FuncDecl { name, params, body } => {
                let mut s = format!("{}fn {}({}) {{\n", p, name, params.join(", "));
                self.indent += 1;
                for stmt in body {
                    s.push_str(&format!("{}\n", self.print_stmt(stmt)));
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.pad()));
                s
            }
            Stmt::Return(expr) => match expr {
                Some(e) => format!("{}return {};", p, self.print_expr(e)),
                None => format!("{}return;", p),
            },
            Stmt::Block(stmts) => {
                let mut s = format!("{}{{\n", p);
                self.indent += 1;
                for stmt in stmts {
                    s.push_str(&format!("{}\n", self.print_stmt(stmt)));
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.pad()));
                s
            }
            Stmt::While { cond, body } => {
                let mut s = format!("{}while {} {{\n", p, self.print_expr(cond));
                self.indent += 1;
                for stmt in body {
                    s.push_str(&format!("{}\n", self.print_stmt(stmt)));
                }
                self.indent -= 1;
                s.push_str(&format!("{}}}", self.pad()));
                s
            }
            Stmt::Print(expr) => format!("{}print({});", p, self.print_expr(expr)),
        }
    }
}

// ── Interpreter ─────────────────────────────────────────────

#[derive(Debug)]
pub enum InterpError {
    UndefinedVar(String),
    TypeError(String),
    DivisionByZero,
    Return(Value),
    UndefinedFunc(String),
    ArityMismatch { expected: usize, got: usize },
}

pub struct Interpreter {
    env: Vec<HashMap<String, Value>>, // stack of scopes
    functions: HashMap<String, (Vec<String>, Vec<Stmt>)>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.env.pop();
    }

    fn define(&mut self, name: String, value: Value) {
        self.env.last_mut().unwrap().insert(name, value);
    }

    fn lookup(&self, name: &str) -> Result<Value, InterpError> {
        for scope in self.env.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Ok(val.clone());
            }
        }
        Err(InterpError::UndefinedVar(name.to_string()))
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, InterpError> {
        match expr {
            Expr::Number(n) => Ok(Value::Int(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Var(name) => self.lookup(name),
            Expr::BinOp { op, left, right } => {
                let lv = self.eval_expr(left)?;
                let rv = self.eval_expr(right)?;
                match op {
                    BinOp::Add => match (&lv, &rv) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                        (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                        _ => Err(InterpError::TypeError(format!("Cannot add {} and {}", lv, rv))),
                    },
                    BinOp::Sub => match (&lv, &rv) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                        _ => Err(InterpError::TypeError("Cannot subtract non-integers".into())),
                    },
                    BinOp::Mul => match (&lv, &rv) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                        _ => Err(InterpError::TypeError("Cannot multiply non-integers".into())),
                    },
                    BinOp::Div => match (&lv, &rv) {
                        (Value::Int(a), Value::Int(b)) => {
                            if *b == 0 {
                                Err(InterpError::DivisionByZero)
                            } else {
                                Ok(Value::Int(a / b))
                            }
                        }
                        _ => Err(InterpError::TypeError("Cannot divide non-integers".into())),
                    },
                    BinOp::Eq => Ok(Value::Bool(lv == rv)),
                    BinOp::Neq => Ok(Value::Bool(lv != rv)),
                    BinOp::Lt => match (&lv, &rv) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
                        _ => Err(InterpError::TypeError("Cannot compare non-integers with <".into())),
                    },
                    BinOp::Gt => match (&lv, &rv) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
                        _ => Err(InterpError::TypeError("Cannot compare non-integers with >".into())),
                    },
                    BinOp::And => match (&lv, &rv) {
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                        _ => Err(InterpError::TypeError("Cannot && non-booleans".into())),
                    },
                    BinOp::Or => match (&lv, &rv) {
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                        _ => Err(InterpError::TypeError("Cannot || non-booleans".into())),
                    },
                }
            }
            Expr::UnaryOp { op, operand } => {
                let v = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => match v {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        _ => Err(InterpError::TypeError("Cannot negate non-integer".into())),
                    },
                    UnaryOp::Not => match v {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err(InterpError::TypeError("Cannot ! non-boolean".into())),
                    },
                }
            }
            Expr::Call { func, args } => {
                let (params, body) = self
                    .functions
                    .get(func)
                    .ok_or_else(|| InterpError::UndefinedFunc(func.clone()))?
                    .clone();

                if args.len() != params.len() {
                    return Err(InterpError::ArityMismatch {
                        expected: params.len(),
                        got: args.len(),
                    });
                }

                let arg_vals: Result<Vec<Value>, InterpError> =
                    args.iter().map(|a| self.eval_expr(a)).collect();
                let arg_vals = arg_vals?;

                self.push_scope();
                for (param, val) in params.iter().zip(arg_vals) {
                    self.define(param.clone(), val);
                }

                let result = self.exec_block(&body);
                self.pop_scope();

                match result {
                    Err(InterpError::Return(v)) => Ok(v),
                    other => other,
                }
            }
            Expr::If { cond, then, else_ } => {
                let cv = self.eval_expr(cond)?;
                match cv {
                    Value::Bool(true) => self.eval_expr(then),
                    Value::Bool(false) => match else_ {
                        Some(e) => self.eval_expr(e),
                        None => Ok(Value::Nil),
                    },
                    _ => Err(InterpError::TypeError("If condition must be boolean".into())),
                }
            }
        }
    }

    pub fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Value, InterpError> {
        match stmt {
            Stmt::ExprStmt(expr) => self.eval_expr(expr),
            Stmt::VarDecl { name, value } => {
                let v = self.eval_expr(value)?;
                self.define(name.clone(), v);
                Ok(Value::Nil)
            }
            Stmt::FuncDecl { name, params, body } => {
                self.functions
                    .insert(name.clone(), (params.clone(), body.clone()));
                Ok(Value::Nil)
            }
            Stmt::Return(expr) => {
                let v = match expr {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Nil,
                };
                Err(InterpError::Return(v))
            }
            Stmt::Block(stmts) => self.exec_block(stmts),
            Stmt::While { cond, body } => {
                loop {
                    let cv = self.eval_expr(cond)?;
                    match cv {
                        Value::Bool(true) => {
                            self.exec_block(body)?;
                        }
                        Value::Bool(false) => break,
                        _ => return Err(InterpError::TypeError("While condition must be boolean".into())),
                    }
                }
                Ok(Value::Nil)
            }
            Stmt::Print(expr) => {
                let v = self.eval_expr(expr)?;
                println!("{}", v);
                Ok(Value::Nil)
            }
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Value, InterpError> {
        self.push_scope();
        for stmt in stmts {
            self.exec_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(Value::Nil)
    }

    pub fn run(&mut self, program: &[Stmt]) -> Result<Value, InterpError> {
        // First pass: register all function declarations
        for stmt in program {
            if let Stmt::FuncDecl { name, params, body } = stmt {
                self.functions
                    .insert(name.clone(), (params.clone(), body.clone()));
            }
        }
        // Second pass: execute non-function statements
        for stmt in program {
            match stmt {
                Stmt::FuncDecl { .. } => {} // already registered
                _ => {
                    self.exec_stmt(stmt)?;
                }
            }
        }
        Ok(Value::Nil)
    }
}

// ── TypeChecker ─────────────────────────────────────────────

#[derive(Debug)]
pub enum TypeError {
    Mismatch { expected: Type, got: Type, context: String },
    UndefinedVar(String),
    UndefinedFunc(String),
    ArityMismatch { expected: usize, got: usize },
    InvalidOp(String),
}

pub struct TypeChecker {
    env: Vec<HashMap<String, Type>>,
    functions: HashMap<String, (Vec<Type>, Type)>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            env: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.env.pop();
    }

    fn define(&mut self, name: String, ty: Type) {
        self.env.last_mut().unwrap().insert(name, ty);
    }

    fn lookup(&self, name: &str) -> Result<Type, TypeError> {
        for scope in self.env.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Ok(ty.clone());
            }
        }
        Err(TypeError::UndefinedVar(name.to_string()))
    }

    pub fn check_expr(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Number(_) => Ok(Type::Int),
            Expr::Str(_) => Ok(Type::Str),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Var(name) => self.lookup(name),
            Expr::BinOp { op, left, right } => {
                let lt = self.check_expr(left)?;
                let rt = self.check_expr(right)?;
                match op {
                    BinOp::Add => {
                        if lt == Type::Int && rt == Type::Int {
                            Ok(Type::Int)
                        } else if lt == Type::Str && rt == Type::Str {
                            Ok(Type::Str)
                        } else {
                            Err(TypeError::Mismatch {
                                expected: lt.clone(),
                                got: rt,
                                context: format!("binary {}", op),
                            })
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if lt == Type::Int && rt == Type::Int {
                            Ok(Type::Int)
                        } else {
                            Err(TypeError::InvalidOp(format!(
                                "Arithmetic operator requires int, got {} and {}",
                                lt, rt
                            )))
                        }
                    }
                    BinOp::Eq | BinOp::Neq => {
                        if lt == rt {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::Mismatch {
                                expected: lt,
                                got: rt,
                                context: format!("comparison {}", op),
                            })
                        }
                    }
                    BinOp::Lt | BinOp::Gt => {
                        if lt == Type::Int && rt == Type::Int {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidOp(format!(
                                "Comparison requires int, got {} and {}",
                                lt, rt
                            )))
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if lt == Type::Bool && rt == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidOp(format!(
                                "Logical operator requires bool, got {} and {}",
                                lt, rt
                            )))
                        }
                    }
                }
            }
            Expr::UnaryOp { op, operand } => {
                let t = self.check_expr(operand)?;
                match op {
                    UnaryOp::Neg => {
                        if t == Type::Int {
                            Ok(Type::Int)
                        } else {
                            Err(TypeError::InvalidOp(format!("Cannot negate {}", t)))
                        }
                    }
                    UnaryOp::Not => {
                        if t == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(TypeError::InvalidOp(format!("Cannot apply ! to {}", t)))
                        }
                    }
                }
            }
            Expr::Call { func, args } => {
                let (param_types, ret_type) = self
                    .functions
                    .get(func)
                    .ok_or_else(|| TypeError::UndefinedFunc(func.clone()))?
                    .clone();

                if args.len() != param_types.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: param_types.len(),
                        got: args.len(),
                    });
                }

                for (arg, expected) in args.iter().zip(&param_types) {
                    let actual = self.check_expr(arg)?;
                    if actual != *expected {
                        return Err(TypeError::Mismatch {
                            expected: expected.clone(),
                            got: actual,
                            context: format!("argument to {}", func),
                        });
                    }
                }

                Ok(ret_type)
            }
            Expr::If { cond, then, else_ } => {
                let ct = self.check_expr(cond)?;
                if ct != Type::Bool {
                    return Err(TypeError::Mismatch {
                        expected: Type::Bool,
                        got: ct,
                        context: "if condition".into(),
                    });
                }
                let tt = self.check_expr(then)?;
                match else_ {
                    Some(e) => {
                        let et = self.check_expr(e)?;
                        if tt == et {
                            Ok(tt)
                        } else {
                            Err(TypeError::Mismatch {
                                expected: tt,
                                got: et,
                                context: "if/else branches".into(),
                            })
                        }
                    }
                    None => Ok(Type::Nil),
                }
            }
        }
    }

    pub fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::ExprStmt(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
            Stmt::VarDecl { name, value } => {
                let ty = self.check_expr(value)?;
                self.define(name.clone(), ty);
                Ok(())
            }
            Stmt::FuncDecl { name, params, body } => {
                // For simplicity, assume all params are int
                let param_types: Vec<Type> = params.iter().map(|_| Type::Int).collect();
                self.functions
                    .insert(name.clone(), (param_types.clone(), Type::Int));

                self.push_scope();
                for (param, ty) in params.iter().zip(&param_types) {
                    self.define(param.clone(), ty.clone());
                }
                for stmt in body {
                    self.check_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.check_expr(e)?;
                }
                Ok(())
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                for stmt in stmts {
                    self.check_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::While { cond, body } => {
                let ct = self.check_expr(cond)?;
                if ct != Type::Bool {
                    return Err(TypeError::Mismatch {
                        expected: Type::Bool,
                        got: ct,
                        context: "while condition".into(),
                    });
                }
                for stmt in body {
                    self.check_stmt(stmt)?;
                }
                Ok(())
            }
            Stmt::Print(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    pub fn check_program(&mut self, program: &[Stmt]) -> Result<(), TypeError> {
        // Register functions
        for stmt in program {
            if let Stmt::FuncDecl { name, params, .. } = stmt {
                let param_types: Vec<Type> = params.iter().map(|_| Type::Int).collect();
                self.functions
                    .insert(name.clone(), (param_types, Type::Int));
            }
        }
        // Check all statements
        for stmt in program {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }
}

// ── Demo Program ────────────────────────────────────────────

fn build_demo_program() -> Vec<Stmt> {
    vec![
        // fn add(a, b) { return a + b; }
        Stmt::FuncDecl {
            name: "add".into(),
            params: vec!["a".into(), "b".into()],
            body: vec![Stmt::Return(Some(Expr::BinOp {
                op: BinOp::Add,
                left: Box::new(Expr::Var("a".into())),
                right: Box::new(Expr::Var("b".into())),
            }))],
        },
        // fn factorial(n) { if n <= 1 { return 1; } else { return n * factorial(n - 1); } }
        Stmt::FuncDecl {
            name: "factorial".into(),
            params: vec!["n".into()],
            body: vec![Stmt::Return(Some(Expr::If {
                cond: Box::new(Expr::BinOp {
                    op: BinOp::Eq,
                    left: Box::new(Expr::Var("n".into())),
                    right: Box::new(Expr::Number(1)),
                }),
                then: Box::new(Expr::Number(1)),
                else_: Some(Box::new(Expr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(Expr::Var("n".into())),
                    right: Box::new(Expr::Call {
                        func: "factorial".into(),
                        args: vec![Expr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(Expr::Var("n".into())),
                            right: Box::new(Expr::Number(1)),
                        }],
                    }),
                })),
            }))],
        },
        // let result = add(3, 4);
        Stmt::VarDecl {
            name: "result".into(),
            value: Expr::Call {
                func: "add".into(),
                args: vec![Expr::Number(3), Expr::Number(4)],
            },
        },
        // print(result);
        Stmt::Print(Expr::Var("result".into())),
        // print(factorial(5));
        Stmt::Print(Expr::Call {
            func: "factorial".into(),
            args: vec![Expr::Number(5)],
        }),
    ]
}

// ── Main ────────────────────────────────────────────────────

fn main() {
    let program = build_demo_program();

    // PrettyPrinter demo
    println!("=== PrettyPrinter ===\n");
    let mut printer = PrettyPrinter::new();
    for stmt in &program {
        println!("{}", printer.print_stmt(stmt));
        println!();
    }

    // TypeChecker demo
    println!("=== TypeChecker ===\n");
    let mut checker = TypeChecker::new();
    match checker.check_program(&program) {
        Ok(()) => println!("All types check out!\n"),
        Err(e) => println!("Type error: {:?}\n", e),
    }

    // Interpreter demo
    println!("=== Interpreter ===\n");
    let mut interp = Interpreter::new();
    match interp.run(&program) {
        Ok(_) => println!("\nProgram finished."),
        Err(e) => println!("Runtime error: {:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_print_number() {
        let mut p = PrettyPrinter::new();
        assert_eq!(p.print_expr(&Expr::Number(42)), "42");
    }

    #[test]
    fn test_pretty_print_binop() {
        let mut p = PrettyPrinter::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            left: Box::new(Expr::Number(1)),
            right: Box::new(Expr::Number(2)),
        };
        assert_eq!(p.print_expr(&expr), "(1 + 2)");
    }

    #[test]
    fn test_interpreter_arithmetic() {
        let mut interp = Interpreter::new();
        let expr = Expr::BinOp {
            op: BinOp::Mul,
            left: Box::new(Expr::Number(3)),
            right: Box::new(Expr::Number(4)),
        };
        assert_eq!(interp.eval_expr(&expr).unwrap(), Value::Int(12));
    }

    #[test]
    fn test_interpreter_add_function() {
        let mut interp = Interpreter::new();
        let program = vec![
            Stmt::FuncDecl {
                name: "add".into(),
                params: vec!["a".into(), "b".into()],
                body: vec![Stmt::Return(Some(Expr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(Expr::Var("a".into())),
                    right: Box::new(Expr::Var("b".into())),
                }))],
            },
            Stmt::VarDecl {
                name: "x".into(),
                value: Expr::Call {
                    func: "add".into(),
                    args: vec![Expr::Number(10), Expr::Number(20)],
                },
            },
        ];
        interp.run(&program).unwrap();
        assert_eq!(interp.lookup("x").unwrap(), Value::Int(30));
    }

    #[test]
    fn test_type_checker_int_add() {
        let mut checker = TypeChecker::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            left: Box::new(Expr::Number(1)),
            right: Box::new(Expr::Number(2)),
        };
        assert_eq!(checker.check_expr(&expr).unwrap(), Type::Int);
    }

    #[test]
    fn test_type_checker_bool_and() {
        let mut checker = TypeChecker::new();
        let expr = Expr::BinOp {
            op: BinOp::And,
            left: Box::new(Expr::Bool(true)),
            right: Box::new(Expr::Bool(false)),
        };
        assert_eq!(checker.check_expr(&expr).unwrap(), Type::Bool);
    }

    #[test]
    fn test_type_checker_error() {
        let mut checker = TypeChecker::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            left: Box::new(Expr::Number(1)),
            right: Box::new(Expr::Bool(true)),
        };
        assert!(checker.check_expr(&expr).is_err());
    }

    #[test]
    fn test_if_expression() {
        let mut interp = Interpreter::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Bool(true)),
            then: Box::new(Expr::Number(1)),
            else_: Some(Box::new(Expr::Number(2))),
        };
        assert_eq!(interp.eval_expr(&expr).unwrap(), Value::Int(1));
    }
}

// typecheck.rs — Type checker for pal
//
// Walks the AST, checks variable declarations, function signatures,
// and expression types. Builds a symbol table with scope stacking.

use std::collections::HashMap;
use crate::ast::*;

pub struct TypeError {
    pub message: String,
}

impl TypeError {
    fn new(msg: &str) -> Self {
        TypeError { message: msg.to_string() }
    }
}

struct Scope {
    vars: HashMap<String, PalType>,
    funcs: HashMap<String, (Vec<PalType>, PalType)>,
}

pub struct TypeChecker {
    scopes: Vec<Scope>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            scopes: vec![Scope {
                vars: HashMap::new(),
                funcs: HashMap::new(),
            }],
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            funcs: HashMap::new(),
        });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: &str, ty: PalType) -> Result<(), TypeError> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.vars.contains_key(name) {
            return Err(TypeError::new(&format!("Variable '{}' already declared", name)));
        }
        scope.vars.insert(name.to_string(), ty);
        Ok(())
    }

    fn lookup_var(&self, name: &str) -> Result<&PalType, TypeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.vars.get(name) {
                return Ok(ty);
            }
        }
        Err(TypeError::new(&format!("Undefined variable '{}'", name)))
    }

    fn lookup_func(&self, name: &str) -> Result<&(Vec<PalType>, PalType), TypeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(sig) = scope.funcs.get(name) {
                return Ok(sig);
            }
        }
        Err(TypeError::new(&format!("Undefined function '{}'", name)))
    }

    pub fn check_program(&mut self, prog: &Program) -> Result<(), TypeError> {
        for g in &prog.globals {
            self.declare_var(&g.name, g.ty.clone())?;
        }

        for f in &prog.functions {
            let param_types: Vec<PalType> = f.params.iter().map(|p| p.ty.clone()).collect();
            let scope = self.scopes.last_mut().unwrap();
            scope.funcs.insert(f.name.clone(), (param_types, f.return_ty.clone()));
        }

        for f in &prog.functions {
            self.check_func(f)?;
        }

        self.check_stmt(&prog.body)?;
        Ok(())
    }

    fn check_func(&mut self, f: &FuncDecl) -> Result<(), TypeError> {
        self.push_scope();

        // Function name is available as a variable (for return assignment)
        self.declare_var(&f.name, f.return_ty.clone())?;

        for p in &f.params {
            self.declare_var(&p.name, p.ty.clone())?;
        }
        for l in &f.locals {
            self.declare_var(&l.name, l.ty.clone())?;
        }

        self.check_stmt(&f.body)?;
        self.pop_scope();
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::Assign(name, expr) => {
                let expr_ty = self.check_expr(expr)?;
                let var_ty = self.lookup_var(name)?;
                if std::mem::discriminant(&expr_ty) != std::mem::discriminant(var_ty) {
                    return Err(TypeError::new(&format!(
                        "Type mismatch in assignment to '{}'", name
                    )));
                }
                Ok(())
            }
            Stmt::If(cond, then_stmt, else_stmt) => {
                let cond_ty = self.check_expr(cond)?;
                if let PalType::Bool = cond_ty {} else {
                    return Err(TypeError::new("If condition must be bool"));
                }
                self.check_stmt(then_stmt)?;
                if let Some(e) = else_stmt {
                    self.check_stmt(e)?;
                }
                Ok(())
            }
            Stmt::While(cond, body) => {
                let cond_ty = self.check_expr(cond)?;
                if let PalType::Bool = cond_ty {} else {
                    return Err(TypeError::new("While condition must be bool"));
                }
                self.check_stmt(body)
            }
            Stmt::Print(expr) => {
                let _ = self.check_expr(expr)?;
                Ok(())
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.check_stmt(s)?;
                }
                Ok(())
            }
        }
    }

    pub fn check_expr(&mut self, expr: &Expr) -> Result<PalType, TypeError> {
        match expr {
            Expr::IntLit(_) => Ok(PalType::Int),
            Expr::BoolLit(_) => Ok(PalType::Bool),
            Expr::Var(name) => Ok(self.lookup_var(name)?.clone()),
            Expr::BinOp(left, op, right) => {
                let lt = self.check_expr(left)?;
                let rt = self.check_expr(right)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        match (&lt, &rt) {
                            (PalType::Int, PalType::Int) => Ok(PalType::Int),
                            _ => Err(TypeError::new("Arithmetic requires int operands")),
                        }
                    }
                    BinOp::Eq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        if std::mem::discriminant(&lt) == std::mem::discriminant(&rt) {
                            Ok(PalType::Bool)
                        } else {
                            Err(TypeError::new("Comparison operands must match types"))
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        match (&lt, &rt) {
                            (PalType::Bool, PalType::Bool) => Ok(PalType::Bool),
                            _ => Err(TypeError::new("Logical operators require bool operands")),
                        }
                    }
                }
            }
            Expr::Call(name, args) => {
                let sig = self.lookup_func(name)?.clone();
                if args.len() != sig.0.len() {
                    return Err(TypeError::new(&format!(
                        "Function '{}' expects {} args, got {}",
                        name, sig.0.len(), args.len()
                    )));
                }
                for (i, (arg, param_ty)) in args.iter().zip(sig.0.iter()).enumerate() {
                    let arg_ty = self.check_expr(arg)?;
                    if std::mem::discriminant(&arg_ty) != std::mem::discriminant(param_ty) {
                        return Err(TypeError::new(&format!(
                            "Argument {} of '{}' has wrong type", i, name
                        )));
                    }
                }
                Ok(sig.1)
            }
        }
    }
}

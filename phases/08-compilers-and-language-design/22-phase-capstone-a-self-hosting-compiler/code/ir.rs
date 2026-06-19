// ir.rs — IR generation for pal (3-address code)
//
// Translates the AST into a flat list of three-address instructions.
// Temporaries are numbered t0, t1, ... Labels are L0, L1, ...

#[derive(Debug, Clone)]
pub enum IrOp {
    Add, Sub, Mul, Div,
    Eq, Lt, Gt, Le, Ge,
}

#[derive(Debug, Clone)]
pub enum IrVal {
    Const(i64),
    Var(String),
    Temp(usize),
}

#[derive(Debug, Clone)]
pub enum IrInstr {
    Assign(IrVal, IrVal),                   // x = y
    BinOp(IrVal, IrOp, IrVal, IrVal),      // t = a op b
    Label(String),
    IfGoto(IrVal, String),                  // if x goto L
    Goto(String),
    Call(String, Vec<IrVal>, IrVal),        // t = f(args)
    Return(IrVal),
    Print(IrVal),
}

pub struct IrGenerator {
    instrs: Vec<IrInstr>,
    temp_counter: usize,
    label_counter: usize,
}

impl IrGenerator {
    pub fn new() -> Self {
        IrGenerator {
            instrs: Vec::new(),
            temp_counter: 0,
            label_counter: 0,
        }
    }

    fn fresh_temp(&mut self) -> IrVal {
        let t = self.temp_counter;
        self.temp_counter += 1;
        IrVal::Temp(t)
    }

    fn fresh_label(&mut self) -> String {
        let l = format!("L{}", self.label_counter);
        self.label_counter += 1;
        l
    }

    pub fn emit(&mut self, instr: IrInstr) {
        self.instrs.push(instr);
    }

    pub fn generate_program(&mut self, prog: &crate::ast::Program) -> Vec<IrInstr> {
        for f in &prog.functions {
            self.emit(IrInstr::Label(format!("func_{}", f.name)));
            self.generate_stmt(&f.body);
        }

        self.emit(IrInstr::Label("main".to_string()));
        self.generate_stmt(&prog.body);

        std::mem::take(&mut self.instrs)
    }

    fn generate_stmt(&mut self, stmt: &crate::ast::Stmt) {
        match stmt {
            crate::ast::Stmt::Assign(name, expr) => {
                let val = self.generate_expr(expr);
                self.emit(IrInstr::Assign(IrVal::Var(name.clone()), val));
            }
            crate::ast::Stmt::If(cond, then_stmt, else_stmt) => {
                let cond_val = self.generate_expr(cond);
                if let Some(e) = else_stmt {
                    let else_label = self.fresh_label();
                    let end_label = self.fresh_label();
                    self.emit(IrInstr::IfGoto(cond_val, else_label.clone()));
                    // Inverted: if cond is false, goto else
                    self.generate_stmt(then_stmt);
                    self.emit(IrInstr::Goto(end_label.clone()));
                    self.emit(IrInstr::Label(else_label));
                    self.generate_stmt(e);
                    self.emit(IrInstr::Label(end_label));
                } else {
                    let end_label = self.fresh_label();
                    let neg = self.fresh_temp();
                    self.emit(IrInstr::BinOp(neg.clone(), IrOp::Eq, cond_val, IrVal::Const(0)));
                    self.emit(IrInstr::IfGoto(neg, end_label.clone()));
                    self.generate_stmt(then_stmt);
                    self.emit(IrInstr::Label(end_label));
                }
            }
            crate::ast::Stmt::While(cond, body) => {
                let loop_label = self.fresh_label();
                let end_label = self.fresh_label();
                self.emit(IrInstr::Label(loop_label.clone()));
                let cond_val = self.generate_expr(cond);
                let neg = self.fresh_temp();
                self.emit(IrInstr::BinOp(neg.clone(), IrOp::Eq, cond_val, IrVal::Const(0)));
                self.emit(IrInstr::IfGoto(neg, end_label.clone()));
                self.generate_stmt(body);
                self.emit(IrInstr::Goto(loop_label));
                self.emit(IrInstr::Label(end_label));
            }
            crate::ast::Stmt::Print(expr) => {
                let val = self.generate_expr(expr);
                self.emit(IrInstr::Print(val));
            }
            crate::ast::Stmt::Block(stmts) => {
                for s in stmts {
                    self.generate_stmt(s);
                }
            }
        }
    }

    fn generate_expr(&mut self, expr: &crate::ast::Expr) -> IrVal {
        match expr {
            crate::ast::Expr::IntLit(n) => IrVal::Const(*n),
            crate::ast::Expr::BoolLit(b) => IrVal::Const(if *b { 1 } else { 0 }),
            crate::ast::Expr::Var(name) => IrVal::Var(name.clone()),
            crate::ast::Expr::BinOp(left, op, right) => {
                let lv = self.generate_expr(left);
                let rv = self.generate_expr(right);
                let ir_op = match op {
                    crate::ast::BinOp::Add => IrOp::Add,
                    crate::ast::BinOp::Sub => IrOp::Sub,
                    crate::ast::BinOp::Mul => IrOp::Mul,
                    crate::ast::BinOp::Div => IrOp::Div,
                    crate::ast::BinOp::Eq => IrOp::Eq,
                    crate::ast::BinOp::Lt => IrOp::Lt,
                    crate::ast::BinOp::Gt => IrOp::Gt,
                    crate::ast::BinOp::Le => IrOp::Le,
                    crate::ast::BinOp::Ge => IrOp::Ge,
                    _ => IrOp::Eq, // and/or handled at stmt level
                };
                let tmp = self.fresh_temp();
                self.emit(IrInstr::BinOp(tmp.clone(), ir_op, lv, rv));
                tmp
            }
            crate::ast::Expr::Call(name, args) => {
                let ir_args: Vec<IrVal> = args.iter().map(|a| self.generate_expr(a)).collect();
                let tmp = self.fresh_temp();
                self.emit(IrInstr::Call(name.clone(), ir_args, tmp.clone()));
                tmp
            }
        }
    }
}

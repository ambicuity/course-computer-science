// ast.rs — AST node definitions for pal
//
// Algebraic data types representing the program structure after parsing.
// Expr and Stmt are enums, enabling exhaustive pattern matching in later stages.

#[derive(Debug, Clone)]
pub enum PalType {
    Int,
    Bool,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    BoolLit(bool),
    Var(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Lt, Gt, Le, Ge,
    And, Or,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(String, Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Print(Expr),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub ty: PalType,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: PalType,
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: PalType,
    pub locals: Vec<VarDecl>,
    pub body: Stmt,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub name: String,
    pub globals: Vec<VarDecl>,
    pub functions: Vec<FuncDecl>,
    pub body: Stmt,
}

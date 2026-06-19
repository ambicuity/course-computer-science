// Build a Static Analyzer for C with Abstract Interpretation
// Run: rustc main.rs && ./main
//
// Architecture:
//   Source → CFG Builder → Abstract Domain (Interval) → Fixpoint Solver → Warnings
//
// Implements interval abstract domain, control-flow graph types, transfer functions,
// and a worklist-based fixpoint solver with widening for convergence.

use std::collections::HashMap;

// =============================================================================
// Step 1: Abstract Interval Domain
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
struct Interval { lo: i64, hi: i64 }

const TOP: Interval = Interval { lo: i64::MIN, hi: i64::MAX };
const BOTTOM: Interval = Interval { lo: 1, hi: 0 }; // lo > hi = empty

impl Interval {
    fn new(lo: i64, hi: i64) -> Self { Interval { lo, hi } }
    fn is_bottom(&self) -> bool { self.lo > self.hi }
    fn is_top(&self) -> bool { self.lo == i64::MIN && self.hi == i64::MAX }
    fn contains_zero(&self) -> bool { self.lo <= 0 && self.hi >= 0 }

    fn join(&self, other: &Interval) -> Interval {
        if self.is_bottom() { return *other; }
        if other.is_bottom() { return *self; }
        Interval::new(self.lo.min(other.lo), self.hi.max(other.hi))
    }

    fn widen(&self, other: &Interval) -> Interval {
        if self.is_bottom() { return *other; }
        if other.is_bottom() { return *self; }
        let lo = if other.lo < self.lo { i64::MIN } else { self.lo };
        let hi = if other.hi > self.hi { i64::MAX } else { self.hi };
        Interval::new(lo, hi)
    }

    fn add_const(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        Interval::new(self.lo.saturating_add(c), self.hi.saturating_add(c))
    }

    fn mul_const(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        let products = [self.lo.saturating_mul(c), self.hi.saturating_mul(c)];
        Interval::new(*products.iter().min().unwrap(), *products.iter().max().unwrap())
    }

    fn narrow_lt(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        Interval::new(self.lo, self.hi.min(c - 1))
    }

    fn narrow_ge(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        Interval::new(self.lo.max(c), self.hi)
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_bottom() { write!(f, "⊥") }
        else if self.is_top() { write!(f, "⊤") }
        else { write!(f, "[{}, {}]", self.lo, self.hi) }
    }
}

// =============================================================================
// Step 2: CFG and Transfer Functions
// =============================================================================

#[derive(Debug, Clone)]
enum Stmt {
    Assign(String, Expr),
    If(Expr),
    While(Expr),
}

#[derive(Debug, Clone)]
enum Expr {
    Var(String),
    Const(i64),
    Add(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
}

type State = HashMap<String, Interval>;

fn eval_expr(expr: &Expr, state: &State) -> Interval {
    match expr {
        Expr::Var(name) => state.get(name).copied().unwrap_or(TOP),
        Expr::Const(c) => Interval::new(*c, *c),
        Expr::Add(a, b) => {
            let (la, lb) = (eval_expr(a, state), eval_expr(b, state));
            if la.is_bottom() || lb.is_bottom() { BOTTOM }
            else { Interval::new(la.lo.saturating_add(lb.lo), la.hi.saturating_add(lb.hi)) }
        }
        Expr::Div(a, b) => {
            let divisor = eval_expr(b, state);
            if divisor.contains_zero() {
                println!("  WARNING: possible division by zero (denominator: {})", divisor);
            }
            if divisor.is_bottom() || divisor == Interval::new(0, 0) { BOTTOM } else { TOP }
        }
        Expr::Lt(a, b) => {
            let (la, lb) = (eval_expr(a, state), eval_expr(b, state));
            if la.hi < lb.lo { Interval::new(1, 1) }
            else if la.lo >= lb.hi { Interval::new(0, 0) }
            else { Interval::new(0, 1) }
        }
    }
}

fn transfer(stmt: &Stmt, state: &State) -> State {
    let mut new_state = state.clone();
    match stmt {
        Stmt::Assign(name, expr) => { new_state.insert(name.clone(), eval_expr(expr, state)); }
        Stmt::If(_) | Stmt::While(_) => {} // Simplified
    }
    new_state
}

// =============================================================================
// Step 3: Worklist Solver
// =============================================================================

struct Analyzer { stmts: Vec<Stmt> }

impl Analyzer {
    fn new(stmts: Vec<Stmt>) -> Self { Analyzer { stmts } }

    fn analyze(&self) -> State {
        let mut state: State = HashMap::new();
        let mut prev_state: State;
        for iteration in 0..20 {
            prev_state = state.clone();
            for stmt in &self.stmts { state = transfer(stmt, &state); }
            if state == prev_state {
                println!("  Fixpoint reached at iteration {}", iteration);
                break;
            }
            for (var, interval) in state.iter_mut() {
                if let Some(prev) = prev_state.get(var) {
                    *interval = prev.widen(interval);
                }
            }
        }
        state
    }
}

fn main() {
    println!("=== Static Analysis Demo ===\n");

    // Analyze: x = 5; while (x < 10) { x = x + 1; }; y = 10 / x;
    let stmts = vec![
        Stmt::Assign("x".into(), Expr::Const(5)),
        Stmt::While(Expr::Lt(Box::new(Expr::Var("x")), Box::new(Expr::Const(10)))),
        Stmt::Assign("x".into(), Expr::Add(Box::new(Expr::Var("x")), Box::new(Expr::Const(1)))),
        Stmt::Assign("y".into(), Expr::Div(Box::new(Expr::Const(10)), Box::new(Expr::Var("x")))),
    ];

    let analyzer = Analyzer::new(stmts);
    let final_state = analyzer.analyze();

    println!("\nFinal abstract state:");
    for (var, interval) in &final_state { println!("  {} = {}", var, interval); }

    // Analyze a potentially unsafe division
    println!("\n=== Unsafe Division ===\n");
    let unsafe_stmts = vec![
        Stmt::Assign("x".into(), Expr::Const(0)),
        Stmt::Assign("y".into(), Expr::Div(Box::new(Expr::Const(10)), Box::new(Expr::Var("x")))),
    ];

    let analyzer2 = Analyzer::new(unsafe_stmts);
    let state2 = analyzer2.analyze();
    println!("\nFinal abstract state:");
    for (var, interval) in &state2 { println!("  {} = {}", var, interval); }
}

# Build a Static Analyzer for C with Abstract Interpretation

> Static analysis over-approximates behavior so you catch bugs before they run.

**Type:** Build
**Languages:** Rust, OCaml
**Prerequisites:** Phase 19 lessons 01-13
**Time:** ~720 minutes

## Learning Objectives

- Implement interval abstract domains for integer variables.
- Build a worklist-based fixpoint solver over a control-flow graph.
- Detect potential division-by-zero and uninitialized reads.
- Understand widening for termination guarantees.

## The Problem

Dynamic testing can miss paths. A test might exercise the happy path but miss the edge case where a variable is zero when it shouldn't be, or where a pointer is used after being freed. Static analysis over-approximates all possible executions: if there exists any input that causes a division by zero, the analyzer reports it, even if no test found it.

Abstract interpretation gives a disciplined way to trade precision for scalability. Instead of tracking every possible concrete value of a variable (which could be millions), we track an abstract property: the range of possible values. If a variable's range includes zero and it's used as a denominator, we report a potential division by zero.

The key insight: abstract domains must be finite to guarantee termination. Concrete integers are infinite (there are infinitely many integers). The interval domain `[lo, hi]` is finite for a fixed bit width. When a loop iterates, the interval grows until it reaches a fixpoint (it stops changing) or we force convergence with widening.

## The Concept

A static analyzer has three components:

```
Source code
    │
    ▼
┌───────────────┐
│ 1. Parser      │  Build CFG (control-flow graph)
│  (CFG builder) │  Blocks + edges
└───────────────┘
    │
    ▼
┌───────────────┐
│ 2. Abstract    │  Interval domain [lo, hi]
│  Domain        │  Join, widen, transfer functions
└───────────────┘
    │
    ▼
┌───────────────┐
│ 3. Fixpoint    │  Worklist iteration
│  Solver        │  Propagate until stable
└───────────────┘
```

Abstract domain operations:
- **Join** (⊔): combine two states at a CFG merge point. For intervals: `[a,b] ⊔ [c,d] = [min(a,c), max(b,d)]`
- **Widen** (∇): force convergence in loops. For intervals: if the bound is increasing, set it to +∞
- **Transfer**: compute the abstract effect of one statement. `x = x + 1` on `[3,5]` gives `[4,6]`

```
Example CFG:

    x = 5          ← x: [5,5]
    │
    ▼
  ┌─while x < 10─┐
  │               │
  │  x = x + 1   │  ← x: [5,10] (after widening)
  │               │
  └───────────────┘
    │
    ▼
  y = 10 / x      ← x: [5,10], but after loop x: [10,10]
                     Safe: denominator is always ≥ 10
```

## Build It

### Step 1: Abstract Interval Domain (Rust)

```rust
use std::collections::{HashMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Interval {
    lo: i64,
    hi: i64,
}

const TOP: Interval = Interval { lo: i64::MIN, hi: i64::MAX };
const BOTTOM: Interval = Interval { lo: 1, hi: 0 }; // lo > hi = empty

impl Interval {
    fn new(lo: i64, hi: i64) -> Self { Interval { lo, hi } }
    fn is_bottom(&self) -> bool { self.lo > self.hi }
    fn is_top(&self) -> bool { self.lo == i64::MIN && self.hi == i64::MAX }
    fn contains_zero(&self) -> bool { self.lo <= 0 && self.hi >= 0 }

    // Join (least upper bound)
    fn join(&self, other: &Interval) -> Interval {
        if self.is_bottom() { return *other; }
        if other.is_bottom() { return *self; }
        Interval::new(self.lo.min(other.lo), self.hi.max(other.hi))
    }

    // Widening: force convergence
    fn widen(&self, other: &Interval) -> Interval {
        if self.is_bottom() { return *other; }
        if other.is_bottom() { return *self; }
        let lo = if other.lo < self.lo { i64::MIN } else { self.lo };
        let hi = if other.hi > self.hi { i64::MAX } else { self.hi };
        Interval::new(lo, hi)
    }

    // Transfer: add constant
    fn add_const(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        Interval::new(
            self.lo.saturating_add(c),
            self.hi.saturating_add(c),
        )
    }

    // Transfer: multiply by constant
    fn mul_const(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        let products = [
            self.lo.saturating_mul(c),
            self.hi.saturating_mul(c),
        ];
        Interval::new(*products.iter().min().unwrap(), *products.iter().max().unwrap())
    }

    // Compare: is this interval possibly less than c?
    fn possibly_less_than(&self, c: i64) -> bool {
        !self.is_bottom() && self.lo < c
    }

    // Narrow the interval based on a comparison
    fn narrow_lt(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        Interval::new(self.lo, self.hi.min(c - 1))
    }

    fn narrow_ge(&self, c: i64) -> Interval {
        if self.is_bottom() { return BOTTOM; }
        Interval::new(self.lo.max(c), self.hi)
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_bottom() { write!(f, "⊥") }
        else if self.is_top() { write!(f, "⊤") }
        else { write!(f, "[{}, {}]", self.lo, self.hi) }
    }
}
```

### Step 2: CFG and Transfer Functions

```rust
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
            let la = eval_expr(a, state);
            let lb = eval_expr(b, state);
            if la.is_bottom() || lb.is_bottom() { BOTTOM }
            else { Interval::new(la.lo.saturating_add(lb.lo), la.hi.saturating_add(lb.hi)) }
        }
        Expr::Div(a, b) => {
            let divisor = eval_expr(b, state);
            if divisor.contains_zero() {
                println!("  WARNING: possible division by zero (denominator: {})", divisor);
            }
            if divisor.is_bottom() || divisor == Interval::new(0, 0) { BOTTOM }
            else { TOP } // Simplified
        }
        Expr::Lt(a, b) => {
            let la = eval_expr(a, state);
            let lb = eval_expr(b, state);
            if la.hi < lb.lo { Interval::new(1, 1) }      // Definitely true
            else if la.lo >= lb.hi { Interval::new(0, 0) } // Definitely false
            else { Interval::new(0, 1) }                    // Unknown
        }
    }
}

fn transfer(stmt: &Stmt, state: &State) -> State {
    let mut new_state = state.clone();
    match stmt {
        Stmt::Assign(name, expr) => {
            let val = eval_expr(expr, state);
            new_state.insert(name.clone(), val);
        }
        Stmt::If(expr) => {
            // Simplified: doesn't refine based on condition
        }
        Stmt::While(expr) => {
            // Simplified
        }
    }
    new_state
}
```

### Step 3: Worklist Solver

```rust
struct Analyzer {
    stmts: Vec<Stmt>,
    // Simplified: linear CFG (no real branching for this demo)
}

impl Analyzer {
    fn new(stmts: Vec<Stmt>) -> Self {
        Analyzer { stmts }
    }

    fn analyze(&self) -> State {
        let mut state: State = HashMap::new();
        let mut prev_state: State;

        // Simple iterative analysis (not full worklist for demo)
        for iteration in 0..20 {
            prev_state = state.clone();

            for stmt in &self.stmts {
                state = transfer(stmt, &state);
            }

            // Check fixpoint
            if state == prev_state {
                println!("  Fixpoint reached at iteration {}", iteration);
                break;
            }

            // Apply widening for stability
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
        Stmt::Assign("x".into(), Expr::Add(
            Box::new(Expr::Var("x")),
            Box::new(Expr::Const(1)),
        )),
        Stmt::Assign("y".into(), Expr::Div(
            Box::new(Expr::Const(10)),
            Box::new(Expr::Var("x")),
        )),
    ];

    let analyzer = Analyzer::new(stmts);
    let final_state = analyzer.analyze();

    println!("\nFinal abstract state:");
    for (var, interval) in &final_state {
        println!("  {} = {}", var, interval);
    }

    // Analyze a potentially unsafe division
    println!("\n=== Unsafe Division ===\n");
    let unsafe_stmts = vec![
        Stmt::Assign("x".into(), Expr::Const(0)),
        Stmt::Assign("y".into(), Expr::Div(
            Box::new(Expr::Const(10)),
            Box::new(Expr::Var("x")),
        )),
    ];

    let analyzer2 = Analyzer::new(unsafe_stmts);
    let state2 = analyzer2.analyze();
    println!("\nFinal abstract state:");
    for (var, interval) in &state2 {
        println!("  {} = {}", var, interval);
    }
}
```

## Use It

Production analyzers use richer domains and path sensitivity, but rely on the same backbone:

- **Clang Static Analyzer**: uses symbolic execution (a form of abstract interpretation) to detect null pointer dereferences, memory leaks, and use-after-free in C/C++ code. The core is in `clang/lib/StaticAnalyzer/Core/`.
- **Infer (Facebook)**: uses separation logic (a specialized abstract domain) to detect null pointer dereferences and resource leaks in Java, C, and Objective-C. Runs incrementally on code changes.
- **Frama-C**: a C analysis framework with multiple abstract domains (intervals, congruences, zones). The Eva plugin implements a configurable value analysis.

The key production lesson: **widen early, narrow later**. Without widening, loops never converge: the interval keeps growing (5→6→7→...→10→11→...). Widening forces convergence by jumping to the extreme (5→∞). You can then narrow: if the loop condition is `x < 10`, you know `x` is at most 10 after the loop.

## Read the Source

- [Clang Static Analyzer](https://clang.llvm.org/docs/ClangStaticAnalyzer.html) — Documentation and tutorials.
- [Frama-C Eva](https://frama-c.com/fc-versions/copper/2019/01/01/eva.html) — Value analysis plugin with configurable abstract domains.
- [Principles of Program Analysis](https://www.springer.com/gp/book/9783540654100) — Nielson, Nielson, Hankin. The standard textbook on abstract interpretation.

## Ship It

- `code/main.rs`: interval domain, CFG transfer functions, worklist solver with widening.
- `outputs/README.md`: analyzer report format with one sample program and warning output.

## Exercises

1. **Easy** — Add `x - c` and `x * c` transfers. Implement subtraction and multiplication in the interval domain. Verify that `[5,10] - 3 = [2,7]` and `[3,4] * 2 = [6,8]`.
2. **Medium** — Track relational facts (`x < y`) with a lightweight domain. When the analyzer sees `if (x < 5)`, refine the interval for `x` in the true branch to `[-∞, 4]` and in the false branch to `[5, +∞]`.
3. **Hard** — Add source locations and suppressions. Track line numbers for each statement. When a warning is reported, include the source line. Allow users to suppress warnings with a comment directive: `// analyzer:suppress division-by-zero`.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Abstract domain | "value model" | A finite representation of infinite concrete states. The interval domain `[lo, hi]` represents all integers in the range. The sign domain `{negative, zero, positive}` is coarser but faster. |
| Transfer function | "statement semantics" | The abstract effect of one program step. `x = x + 1` transforms interval `[3,5]` to `[4,6]`. Each statement type has its own transfer function. |
| Join | "merge" | The least upper bound of two abstract states. At a CFG merge point (after an if-else), the join combines the states from both branches. For intervals: `[a,b] ⊔ [c,d] = [min(a,c), max(b,d)]`. |
| Fixpoint | "stable result" | The state where further propagation doesn't change anything. The worklist solver iterates until the abstract state stabilizes. For loops, widening ensures termination. |
| Widening | "loop accelerator" | An operator that guarantees termination by over-approximating. For intervals, if the bound is increasing, widening sets it to infinity. This loses precision but ensures the fixpoint is reached in finite steps. |

## Further Reading

- [Clang Static Analyzer](https://clang.llvm.org/docs/ClangStaticAnalyzer.html) — Production static analyzer for C/C++.
- [Abstract Interpretation](https://www.di.ens.fr/~cousot/AI/) — Cousot's original papers on abstract interpretation theory.
- [Frama-C](https://frama-c.com/) — A C analysis framework with multiple abstract domains.

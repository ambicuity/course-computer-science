# Lesson 09 — AST Design and Visitor Patterns

## What Is an AST?

An **Abstract Syntax Tree** (AST) is the compiler's internal representation of a program's structure. After the parser produces a parse tree (or concrete syntax tree), the compiler transforms it into an AST — a cleaner, more compact tree that drops syntactic noise and focuses on meaning.

```
Source → Lexer → tokens → Parser → Parse Tree → AST → type check → codegen
```

The word **"abstract"** means we remove concrete syntax details: parentheses, semicolons, commas, whitespace. The word **"syntax"** means it still reflects the grammatical structure of the program. The word **"tree"** means it's a hierarchical data structure.

## Concrete vs. Abstract Syntax Tree

Consider the expression `(1 + 2) * 3`:

**Parse tree (concrete):**
```
Expr
├── Term
│   ├── Factor
│   │   ├── '('
│   │   ├── Expr
│   │   │   └── Term
│   │   │       └── Factor
│   │   │           └── Number(1)
│   │   ├── '+'
│   │   ├── Term
│   │   │   └── Factor
│   │   │       └── Number(2)
│   │   └── ')'
│   ├── '*'
│   └── Factor
│       └── Number(3)
```

**AST (abstract):**
```
BinOp(*, BinOp(+, Number(1), Number(2)), Number(3))
```

The AST drops parentheses (grouping is implicit in tree structure), intermediate non-terminals (`Expr`, `Term`, `Factor`), and whitespace. It's half the size and directly usable for interpretation or code generation.

## AST Design: What to Include, What to Drop

**Include:**
- Operators and their operands
- Precedence groups (the tree structure enforces precedence)
- Declarations (variables, functions, types)
- Control flow (if/else, while, for, return)
- Literals (numbers, strings, booleans)

**Drop:**
- Parentheses (grouping is structural)
- Semicolons and commas (delimiters)
- Whitespace and comments
- Intermediate non-terminals from the grammar
- Redundant grouping (e.g., `(x)` → just `x`)

**Design choices that matter:**

1. **Flat vs. nested binary operations**: `1 + 2 + 3` can be `BinOp(+, BinOp(+, 1, 2), 3)` (left-associative tree) or `NaryOp(+, [1, 2, 3])` (flat list). Flat is simpler for some optimizations but loses associativity information.

2. **Statement vs. expression**: Many languages distinguish statements (variable declarations, control flow) from expressions (values, function calls). The AST must reflect this, typically with separate `Stmt` and `Expr` enums.

3. **Location tracking**: Production ASTs store source spans (start line, start column, end line, end column) for error messages. Our simplified version omits this.

## Algebraic Data Types as AST Representation

Rust's `enum` is the natural AST representation. Each variant is a node type, and nested `Box<T>` enables recursive structures:

```rust
enum Expr {
    Number(i64),
    BinOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { func: String, args: Vec<Expr> },
    If { cond: Box<Expr>, then: Box<Expr>, else_: Option<Box<Expr>> },
}
```

This is an **algebraic data type** (ADT) — a sum type (enum) of product types (structs). Pattern matching on ADTs is exhaustive: the compiler warns you if you forget a case.

## The Visitor Pattern

The **visitor pattern** decouples tree traversal from the operations performed on each node. Instead of adding methods like `type_check()` and `codegen()` to every AST node, you define a `Visitor` trait:

```rust
trait Visitor<T> {
    fn visit_number(&mut self, n: i64) -> T;
    fn visit_binop(&mut self, op: BinOp, left: T, right: T) -> T;
    fn visit_call(&mut self, func: &str, args: Vec<T>) -> T;
    // ...
}
```

Each compiler pass is a visitor:
- **PrettyPrinter** — `Visitor<String>`: converts AST back to source code.
- **Interpreter** — `Visitor<Value>`: evaluates the AST directly.
- **TypeChecker** — `Visitor<Type>`: annotates or validates types.
- **CodeGenerator** — `Visitor<Instruction>`: emits machine code or IR.

The tree traversal logic lives in one place (the AST nodes), and operations are plugged in via visitors.

## The Expression Problem

The visitor pattern exposes a fundamental tension known as the **expression problem** (Philip Wadler, 1998):

- **Adding new operations** (e.g., a new optimization pass): Easy with visitors — just implement a new `Visitor`.
- **Adding new node types** (e.g., a `Switch` statement): Hard with visitors — you must modify every existing visitor to handle the new case.

The reverse is true for the **extensible visitor** approach (type classes, open methods): new node types are easy, but new operations require modifying existing code.

Languages handle this differently:
- **Rust/OCaml/Haskell**: ADTs + pattern matching. New operations = new functions. New variants = modify the enum + update all match arms.
- **Java/C++**: Class hierarchy + virtual methods. New node types = new subclasses. New operations = add methods to every class.
- **Clojure/C#**: Multimethods or extension methods try to solve both sides.

In practice for compilers, new passes are more common than new node types, so ADT + visitor is the dominant pattern.

## Visitor Implementation in Rust

In Rust, the visitor pattern is implemented through traits and recursive traversal:

```rust
trait Visitor {
    type Output;
    fn visit_expr(&mut self, expr: &Expr) -> Self::Output;
    fn visit_stmt(&mut self, stmt: &Stmt) -> Self::Output;
}
```

Each `visit_*` method calls `visit` on child nodes, combining results. The AST nodes themselves implement a `walk` method that calls the visitor's methods in the correct order.

## Build It: AST + Visitor for a Mini-Language

We'll build an AST for a mini-language with variables, arithmetic, comparisons, conditionals, while loops, let bindings, functions, and returns. Then implement three visitors:

1. **PrettyPrinter** — reconstructs source code from the AST.
2. **Interpreter** — evaluates the AST with an environment.
3. **TypeChecker** — infers and validates types.

## Use It

Production AST examples:

- **Clang**: Defines `Stmt`, `Expr`, `Decl` hierarchies in `clang/AST/Stmt.h`, `clang/AST/Expr.h`. Uses a CRTP-based visitor (`RecursiveASTVisitor`) with `VisitStmt`, `VisitExpr`, etc.
- **rustc**: Has multiple IR stages — HIR (high-level IR, after macro expansion), MIR (mid-level IR, for borrow checking and optimization), each with its own AST type and visitors.
- **Go**: `go/ast` package defines `Expr`, `Stmt`, `Decl` interfaces with concrete types like `BinaryExpr`, `CallExpr`, `IfStmt`. Visitors are function-based (walk with `ast.Inspect`).
- **TypeScript compiler**: `ts.Node` hierarchy with `forEachChild` traversal and visitor callbacks.

## Read the Source

- Clang AST: `clang/include/clang/AST/Stmt.h` — the core AST node hierarchy for C/C++.
- rustc HIR: `compiler/rustc_hir/src/hir.rs` — Rust's high-level intermediate representation.
- Go `go/ast`: The Go standard library's AST package — clean, idiomatic design.

## Ship It

The artifact is an AST library with a visitor framework. Given a program's AST, you can pretty-print it, interpret it, or type-check it by implementing a visitor.

## Exercises

**Level 1 — Warm-Up:**
Add a `Println` statement to the AST and interpreter. `println(expr)` should evaluate the expression and print its value to stdout.

**Level 2 — Intermediate:**
Add an `Optimizer` visitor that performs constant folding: `1 + 2` → `3`, `x * 1` → `x`, `x * 0` → `0`. The visitor should transform the AST in-place (or return a new, simplified AST).

**Level 3 — Challenge:**
Add a `CodeGen` visitor that compiles the AST to a simple stack machine. Define bytecode instructions (`Push`, `Add`, `Mul`, `Jump`, `JumpIfFalse`, `Call`, `Return`), and have the visitor emit a `Vec<Instruction>`. Execute the bytecode on a stack machine to verify correctness.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| AST | "Abstract Syntax Tree" | A tree representation of a program that drops syntactic noise (parentheses, delimiters) and captures semantic structure |
| Concrete syntax tree | "Parse tree" | A tree that retains every token and grammar rule used during parsing — faithful to the source but verbose |
| Visitor pattern | "Separate operations from traversal" | A design pattern where a `Visitor` trait defines operations for each node type; traversal logic lives in the AST, operations live in the visitor |
| Expression problem | "Adding operations vs. adding types" | The tension between easily adding new operations (easy with visitors) and easily adding new node types (hard with visitors) |
| ADT | "Algebraic Data Type" | A type built from sum (enum) and product (struct) combinators — the natural way to represent ASTs in Rust, OCaml, Haskell |

## Further Reading

- Wadler, "The Expression Problem" (1998) — the classic paper on extensibility trade-offs
- Clang AST documentation: `clang.llvm.org/docs/InternalsManual.html`
- rustc dev guide: `rustc-dev-guide.rust-lang.org/`
- Nystrom, *Crafting Interpreters* — a complete, accessible interpreter implementation with AST + tree-walk evaluation

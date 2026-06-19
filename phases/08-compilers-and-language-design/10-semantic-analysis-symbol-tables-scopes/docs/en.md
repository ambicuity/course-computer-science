# Lesson 10: Semantic Analysis — Symbol Tables, Scopes

## Overview

Parsing tells you whether a program is syntactically valid. Semantic analysis tells you whether it **means** anything. This lesson builds a symbol table with nested scopes and a semantic checker that catches undeclared variables, duplicate definitions, and argument-count mismatches — the errors that parse trees cannot detect.

---

## The Problem

Consider this fragment, which parses perfectly:

```
x = y + 1;
```

The parser sees an assignment with a binary expression. It cannot know whether `y` was ever declared, whether `x` was already defined as a function, or whether `+` is valid for the types involved. Semantic analysis fills that gap.

---

## Symbol Table

A **symbol table** maps names to their declarations. Each entry records:

- **name** — the identifier string
- **kind** — variable, function, type alias, …
- **type information** — declared type or signature
- **scope id** — which scope the symbol belongs to

---

## Scopes

A **scope** is a region of source text where a name binding is visible. Most languages use **lexical scoping** — scope is determined by program structure, not runtime flow.

| Scope | Created by | Example |
|-------|-----------|---------|
| Block | `{ }` | Local variables inside a function body |
| Function | Function declaration | Parameter names, local bindings |
| Module | File / namespace | Top-level definitions, imports |

**Lookup chain:** scopes nest. Reference a name → search innermost scope first, then walk outward through parents. If no scope contains it, the name is **undeclared**.

```
module scope          ← search here last
  └─ function scope   ← search here second
       └─ block scope ← search here first
```

**Shadowing:** an inner scope may re-declare a name from an outer scope. The inner binding shadows the outer one.

---

## Semantic Checks

1. **Declared-before-use** — every referenced name must exist in some enclosing scope.
2. **No duplicate declarations** — a name cannot be declared twice in the same scope.
3. **Function argument count** — call-site arity must match the declaration.
4. **Type compatibility** — operations receive compatible types (covered in Lesson 11).

**Name resolution** binds each identifier occurrence in the AST to its declaration entry.

---

## Build It

### Step 1: Symbol Table with Nested Scopes

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SymbolKind { Var, Func, Type }

struct Symbol {
    name: String, kind: SymbolKind, type_info: String, scope_id: usize,
}

struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<usize>,
}

struct SymbolTable { scopes: Vec<Scope>, current: usize }
```

`enter_scope()` pushes a new scope whose parent is the current one. `exit_scope()` restores the parent. `declare()` inserts into the current scope, rejecting duplicates. `resolve()` walks the parent chain:

```rust
fn resolve(&self, name: &str) -> Option<&Symbol> {
    let mut scope_id = Some(self.current);
    while let Some(id) = scope_id {
        if let Some(sym) = self.scopes[id].symbols.get(name) { return Some(sym); }
        scope_id = self.scopes[id].parent;
    }
    None
}
```

### Step 2: Semantic Checker

A `SemanticChecker` walks the AST, building the symbol table and collecting errors. For `FuncDecl`, it registers the function, enters a new scope for parameters, checks the body, then exits. For `Assign` and `Var` expressions, it calls `resolve()` and reports undeclared names. For `Call` expressions, it verifies the callee is a function and checks argument count.

See `code/main.rs` for the complete implementation with demonstrations of correct programs, duplicate declarations, undeclared variables, wrong argument counts, and block-scoping violations.

---

## Use It

- **GCC** uses a symbol table in `c-decl.c`. It checks scope rules and warns about shadowed variables (`-Wshadow`).
- **Clang** builds a `Sema` layer (`clang/lib/Sema/`) that creates `Decl` objects in an `ASTContext` scope tree. Name lookup walks from the innermost `DeclContext` outward.
- **rustc** uses `rustc_resolve` to bind every identifier to a `Def` via `DefId`. It catches use-before-declaration and duplicate definitions.

### Read the Source

- `rustc/compiler/rustc_resolve/src/lib.rs` — rustc's main resolver entry point
- `clang/lib/Sema/SemaDecl.cpp` — Clang's declaration handling and scope entry

---

## Ship It

The reusable artifact from this lesson is a symbol-table library supporting nested scopes, parent-chain lookup, declaration with duplicate checking, and pluggable error accumulation.

---

## Exercises

### Level 1: Extend

Add `lookup_in_current_scope()` — returns a symbol only in the current scope (useful when a language disallows shadowing).

### Level 2: For-Loop Scope

Extend the AST with `For(init, cond, update, body)`. The `init` should share a scope with the body. The loop variable must not be visible outside.

### Level 3: Overloading

Modify the symbol table to support function overloading by signature. Store a list of `Symbol` per name in the function namespace. `f(int)` and `f(bool)` coexist; `f(42)` resolves via basic argument-type matching.

---

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Symbol table | "The compiler's dictionary" | Data structure mapping identifiers to declaration metadata |
| Scope | "A block of visibility" | Region where a name binding is active |
| Lexical scoping | "Scope follows code structure" | Name lookup by textual nesting, not runtime call stack |
| Name resolution | "Figuring out what a name means" | Binding each identifier reference to its declaration |
| Shadowing | "The inner x hides the outer x" | Inner declaration with same name as an outer one |
| Arity | "How many arguments" | Number of parameters a function expects |

## Further Reading

- Aho, Lam, Sethi, Ullman — *Compilers: Principles, Techniques, and Tools*, Chapter 3: Symbol Tables
- Robert Nystrom — *Crafting Interpreters*, Chapters 8–11 (excellent resolver walk-through)

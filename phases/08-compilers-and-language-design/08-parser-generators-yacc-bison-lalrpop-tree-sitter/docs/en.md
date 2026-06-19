# Lesson 08 — Parser Generators (yacc/bison/lalrpop/tree-sitter)

## The Idea: Write a Grammar, Get a Parser

A **parser generator** takes a formal grammar specification and produces parser source code automatically. Instead of hand-writing recursive descent functions or LR state tables, you declare the grammar rules and the generator emits the parsing machinery.

This is the compiler-writer's version of "don't repeat yourself" — the grammar is the single source of truth.

## Yacc/Bison: The Classic

**Yacc** (Yet Another Compiler Compiler, 1975) and its GNU successor **Bison** generate **LALR(1)** parsers from grammar files. The workflow:

1. Write a `.y` file with grammar rules and embedded C actions.
2. Run `bison parser.y` → produces `parser.c`.
3. Compile `parser.c` with your lexer output.

A Bison grammar file looks like:

```yacc
%{
#include <stdio.h>
%}

%token NUMBER
%left '+' '-'
%left '*' '/'

%%

expr: expr '+' expr   { $$ = $1 + $3; }
    | expr '-' expr   { $$ = $1 - $3; }
    | expr '*' expr   { $$ = $1 * $3; }
    | expr '/' expr   { $$ = $1 / $3; }
    | '(' expr ')'    { $$ = $2; }
    | NUMBER          { $$ = $1; }
    ;
%%
```

Key features:

- **`%left`, `%right`, `%nonassoc`**: Declare operator precedence and associativity. Bison resolves shift-reduce conflicts using these declarations.
- **`$$`** is the result value; **`$1`, `$2`, ...** are the values of the right-hand side symbols.
- **Shift-reduce conflicts** arise when Bison can't decide between shifting a token or reducing a production. The dangling-else is the canonical example.
- **`%glr-parser`** enables GLR mode for ambiguous grammars.

## Shift-Reduce Conflicts: The Dangling Else

Given:

```yacc
stmt: IF expr THEN stmt
    | IF expr THEN stmt ELSE stmt
    ;
```

After parsing `IF E THEN S`, the parser sees `ELSE`. Should it reduce `IF E THEN S` to `stmt`, or shift `ELSE`? This is a **shift-reduce conflict**. Bison's default: **shift wins** (the `else` binds to the nearest `if`). This is almost always the desired behavior.

You can also resolve conflicts with **`%prec`** — assign an explicit precedence level to a production.

## LALRPOP: Rust-Native Parser Generation

**LALRPOP** is a Rust parser generator that produces type-safe Rust code. Key differences from Bison:

- **Type-safe**: Grammar rules have Rust types. The parser generator checks that your actions return the right type.
- **No embedded language**: Actions are Rust expressions, not C code in a different language.
- **Compile-time generation**: `.lalrpop` files are processed by a build script, producing Rust modules at compile time.
- **Ergonomic error handling**: LALRPOP generates parsers that integrate with Rust's `Result` type.

A LALRPOP grammar looks like:

```lalrpop
pub Expr: i32 = {
    <l:Expr> "+" <r:Term> => l + r,
    <l:Expr> "-" <r:Term> => l - r,
    Term,
};

Term: i32 = {
    <l:Term> "*" <r:Atom> => l * r,
    <l:Term> "/" <r:Atom> => l / r,
    Atom,
};

Atom: i32 = {
    <n:"[0-9]+"> => n.parse().unwrap(),
    "(" <e:Expr> ")" => e,
};
```

LALRPOP infers types from the actions and checks them at generation time. This catches many bugs that would only appear as runtime type errors in Bison.

## Tree-sitter: Incremental GLR for Editors

**Tree-sitter** is a parser generator designed for **editor integration**. Key properties:

- **Incremental**: When you edit one line of a file, tree-sitter re-parses only the changed region, not the entire file. This enables real-time syntax highlighting and code navigation.
- **GLR**: Uses Generalized LR parsing, so it handles **ambiguous grammars** gracefully — producing all valid parse trees.
- **Concrete syntax tree**: Unlike most parsers that produce an AST (dropping whitespace, parentheses, comments), tree-sitter preserves **every token** including whitespace and comments. This is essential for editor tooling.
- **Error recovery**: Tree-sitter always produces a complete tree, even for invalid input. It inserts error nodes where parsing fails.

Tree-sitter grammars are defined in JavaScript (a `grammar.js` file) and compiled to C at build time:

```javascript
module.exports = grammar({
  name: 'my_lang',
  rules: {
    source_file: $ => repeat($._statement),
    _statement: $ => choice(
      $.expression_statement,
      $.variable_declaration,
    ),
    expression_statement: $ => seq($.expression, ';'),
    expression: $ => choice(
      $.number,
      $.binary_expression,
    ),
    binary_expression: $ => prec.left(seq(
      $.expression,
      choice('+', '-', '*', '/'),
      $.expression,
    )),
    number: $ => /\d+/,
    variable_declaration: $ => seq('let', $.identifier, '=', $.expression, ';'),
    identifier: $ => /[a-zA-Z_]\w*/,
  }
});
```

The `prec()`, `prec.left()`, `prec.right()` functions control precedence and associativity — analogous to Bison's `%left` and `%right`.

## Comparison

| Feature | Yacc/Bison | LALRPOP | Tree-sitter |
|---------|-----------|---------|-------------|
| Algorithm | LALR(1) | LALR(1) | GLR |
| Type safety | No | Yes | No |
| Incremental | No | No | Yes |
| Main use | Compilers | Rust DSLs | Editors |

## Build It: Grammar + Parser Simulation

Since we can't run LALRPOP directly in this lesson, we'll simulate the parser generator workflow: define grammar rules as Rust data structures and build a parser that interprets them. This demonstrates how grammar specifications translate to parsing tables.

## Use It

- **GCC/Clang**: Hand-written recursive descent (no parser generator).
- **VS Code, Neovim, GitHub**: tree-sitter for syntax highlighting and code folding.
- **Rust DSLs**: LALRPOP is popular for type-safe parser generation.
- **Python**: CPython 3.9+ uses a PEG parser (switched from LL(1)).

## Read the Source

- Bison manual: `bison.gnu.org/manual/` — the definitive reference for LALR(1) parser generation.
- LALRPOP guide: `lalrpop.github.io/lalrpop/` — Rust parser generator documentation.
- Tree-sitter: `github.com/tree-sitter/tree-sitter` — the `lib/src/parser.c` file shows the GLR implementation.

## Ship It

The artifact is a grammar-based parser. Given a `.lalrpop`-style grammar (expressed as Rust data structures), our code demonstrates how rules, conflicts, and precedence interact.

## Exercises

**Level 1 — Warm-Up:**
Write a Bison-style grammar rule set for a language that supports `if-then-else` statements and `while` loops. Identify where shift-reduce conflicts would arise.

**Level 2 — Intermediate:**
Take the grammar from Level 1 and rewrite it in LALRPOP syntax. Add type annotations so that each rule produces an AST node (e.g., `Stmt` enum). Show how LALRPOP's type system catches a type error in the actions.

**Level 3 — Challenge:**
Define a tree-sitter grammar for a simple language with variables, arithmetic, `if-else`, and `while`. Use `prec.left()` and `prec.right()` to handle operator precedence correctly. Explain why tree-sitter's concrete syntax tree preserves information that an AST drops.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Parser generator | "Tool that writes parsers for you" | Takes a grammar file as input, produces parser source code (C, Rust, etc.) as output |
| LALR(1) | "Lookahead LR" | An LR parsing variant that merges states with the same core, producing compact tables — the algorithm behind Yacc/Bison |
| Shift-reduce conflict | "Parser can't decide" | A state where the parser could either shift the next token or reduce a production. Resolved by precedence declarations or default (shift wins) |
| GLR | "Generalized LR" | An LR variant that forks the parser at conflicts, exploring all paths in parallel. Used by tree-sitter |
| Incremental parsing | "Re-parse only what changed" | After an edit, only re-analyze the affected subtree. Tree-sitter's key advantage for editors |
| Concrete syntax tree | "Preserves all tokens" | A parse tree that retains whitespace, comments, and punctuation — unlike an AST which drops them |

## Further Reading

- Johnson, "Yacc: Yet Another Compiler Compiler" (1975) — the original paper
- LALRPOP documentation: `github.com/lalrpop/lalrpop`
- Tree-sitter: `tree-sitter.github.io/tree-sitter/`
- Aho, Lam, Sethi, Ullman, *Compilers: Principles, Techniques, and Tools* (Dragon Book), Chapter 4

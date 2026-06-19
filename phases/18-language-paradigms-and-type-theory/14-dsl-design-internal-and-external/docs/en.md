# DSL Design — Internal and External

> A good DSL compresses intent while preserving debuggability.

**Type:** Learn
**Languages:** Rust, TypeScript
**Prerequisites:** Phase 18 lessons 01-13
**Time:** ~60 minutes

## Learning Objectives

- Compare internal DSLs and external DSLs tradeoffs.
- Design a tiny expression DSL and evaluator.
- Evaluate parser/tooling costs vs embedding advantages.
- Identify DSL failure modes (ambiguity, hidden semantics).

## The Problem

A configuration file uses YAML with stringly-typed rules:

```yaml
rules:
  - condition: "user.age > 18 and user.country == 'US'"
    action: "allow"
  - condition: "user.score < 50"
    action: "deny"
```

This works until someone writes `user.age > "eighteen"` (string comparison, silently wrong) or `user.coutnry` (typo, never caught). The YAML parser doesn't understand your domain. It just sees strings.

A DSL (domain-specific language) gives your domain its own syntax and semantics. An internal DSL borrows the host language's syntax (TypeScript, Rust). An external DSL defines its own grammar and parser. Both trade generality for domain focus: the language knows about users, scores, and actions, not just strings and numbers.

The tradeoff: internal DSLs reuse the host language's tooling (IDE, debugger, type checker) but are constrained by its syntax. External DSLs have full syntactic freedom but require building a parser, editor support, and error reporting from scratch.

## The Concept

### Internal DSL: embedded in the host language

The host language's syntax becomes the DSL's syntax through clever use of method chaining, operator overloading, or macros.

**TypeScript method chaining:**
```typescript
const rule = Rule
  .when(user => user.age > 18)
  .and(user => user.country === 'US')
  .then('allow')
  .otherwise('deny');
```

**Rust macro:**
```rust
let rule = rule! {
    when user.age > 18 && user.country == "US" => "allow"
    otherwise => "deny"
};
```

**Strengths**: IDE support, type checking, debugger works, no parser to write, easy to mix with host language code.

**Weaknesses**: constrained by host syntax, can be hard to read if the DSL fights the host language's idioms.

### External DSL: separate syntax and parser

You define a grammar, write a parser, and interpret or compile the results.

**Grammar (BNF):**
```
rule    := "when" condition "then" action ("otherwise" action)?
condition := expr ("and" expr)*
expr    := IDENT COMP_OP VALUE
action  := STRING
```

**Source:**
```
when user.age > 18 and user.country == "US" then "allow" otherwise "deny"
```

**Strengths**: full syntactic freedom, can be designed for non-programmers, clear separation from host language.

**Weaknesses**: parser development cost, no IDE support without extra work, error reporting is your responsibility, another language to maintain.

### When to choose which

| Factor | Internal DSL | External DSL |
|--------|-------------|-------------|
| Users | Developers | Non-technical domain experts |
| Complexity | Simple expressions | Complex grammar |
| Tooling | Free (IDE, debugger) | Must build or integrate |
| Error messages | Host language errors | Custom, domain-specific |
| Embedding | Seamless with host code | Separate file/process |
| Evolution | Track host language | Independent grammar |

### Parser combinators (middle ground)

Parser combinators let you build parsers as composable functions in the host language. You get external DSL syntax with internal DSL tooling:

```typescript
// TypeScript parser combinator (conceptual)
const rule = seq(
  keyword('when'),
  condition,
  keyword('then'),
  action,
  optional(seq(keyword('otherwise'), action))
);
```

Libraries: `nom` (Rust), `parsec` (Haskell), `pest` (Rust, PEG), `tree-sitter` (incremental parsing).

### Common DSL patterns

| Pattern | Example | Domain |
|---------|---------|--------|
| Query DSL | SQL, GraphQL, Datalog | Data retrieval |
| Build DSL | Makefile, Bazel, Gradle | Build rules |
| Config DSL | Nginx, HAProxy, Terraform | Infrastructure |
| Rule DSL | Drools, CLIPS, jq | Business rules |
| Template DSL | Jinja, Handlebars, ERB | Text generation |

## Build It

### Step 1: Internal DSL in TypeScript

```typescript
// Builder pattern DSL for query construction
class QueryBuilder {
  private _table: string = '';
  private _conditions: string[] = [];
  private _limit: number | null = null;

  from(table: string): this {
    this._table = table;
    return this;
  }

  where(condition: string): this {
    this._conditions.push(condition);
    return this;
  }

  limit(n: number): this {
    this._limit = n;
    return this;
  }

  build(): string {
    let q = `SELECT * FROM ${this._table}`;
    if (this._conditions.length) {
      q += ` WHERE ${this._conditions.join(' AND ')}`;
    }
    if (this._limit !== null) {
      q += ` LIMIT ${this._limit}`;
    }
    return q;
  }
}

function select(): QueryBuilder {
  return new QueryBuilder();
}

// Usage: reads like a DSL
const query = select()
  .from('users')
  .where('age > 18')
  .where('country = "US"')
  .limit(10)
  .build();

console.log(query);
// SELECT * FROM users WHERE age > 18 AND country = "US" LIMIT 10
```

### Step 2: External DSL in Rust (expression evaluator)

```rust
#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Var(String),
}

fn eval(expr: &Expr, vars: &std::collections::HashMap<String, f64>) -> f64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a, vars) + eval(b, vars),
        Expr::Mul(a, b) => eval(a, vars) * eval(b, vars),
        Expr::Var(name) => *vars.get(name).unwrap_or(&0.0),
    }
}

// Parser (simplified recursive descent)
fn parse_expr(input: &str) -> Option<(Expr, &str)> {
    let (mut left, mut rest) = parse_term(input)?;
    while rest.starts_with('+') {
        let (right, r) = parse_term(&rest[1..])?;
        left = Expr::Add(Box::new(left), Box::new(right));
        rest = r;
    }
    Some((left, rest))
}

fn parse_term(input: &str) -> Option<(Expr, &str)> {
    let (mut left, mut rest) = parse_atom(input)?;
    while rest.starts_with('*') {
        let (right, r) = parse_atom(&rest[1..])?;
        left = Expr::Mul(Box::new(left), Box::new(right));
        rest = r;
    }
    Some((left, rest))
}

fn parse_atom(input: &str) -> Option<(Expr, &str)> {
    let input = input.trim_start();
    if input.starts_with('(') {
        let (expr, rest) = parse_expr(&input[1..])?;
        let rest = rest.trim_start().strip_prefix('')?;
        Some((expr, rest))
    } else if let Some(c) = input.chars().next().filter(|c| c.is_ascii_digit()) {
        let end = input.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(input.len());
        let n: f64 = input[..end].parse().ok()?;
        Some((Expr::Num(n), &input[end..]))
    } else {
        let end = input.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(input.len());
        Some((Expr::Var(input[..end].to_string()), &input[end..]))
    }
}

fn main() {
    let expr = parse_expr("x + 2 * 3").unwrap().0;
    let mut vars = std::collections::HashMap::new();
    vars.insert("x".to_string(), 10.0);
    println!("{:?} = {}", expr, eval(&expr, &vars));  // Add(Var("x"), Mul(Num(2.0), Num(3.0))) = 16
}
```

### Step 3: Compare ergonomics

```typescript
// Internal: reuses TypeScript types, IDE autocomplete, debugger
interface Rule {
  condition: (user: User) => boolean;
  action: string;
}

const rules: Rule[] = [
  { condition: u => u.age > 18, action: 'allow' },
  { condition: u => u.score < 50, action: 'deny' },
];

// External: custom syntax, but needs parser
// when user.age > 18 then "allow"
// when user.score < 50 then "deny"
```

## Use It

DSLs are common in:

- **Build systems**: Makefile targets, Bazel rules, Cargo.toml.
- **Query layers**: SQL, GraphQL, Datalog, jq.
- **Policy engines**: OPA/Rego, AWS IAM policy language.
- **Config languages**: Nginx, Terraform HCL, Caddyfile.
- **Template engines**: Jinja2, Handlebars, ERB.
- **Macro systems**: Rust's `macro_rules!`, Lisp macros, Scala macros.

The most successful DSLs start internal (leveraging the host language) and go external only when the syntax constraints become limiting.

## Read the Source

- [Language Workbenches](https://martinfowler.com/articles/languageWorkbench.html) — Martin Fowler on DSL design.
- [Crafting Interpreters](https://craftinginterpreters.com/) — building external DSLs from scratch.
- [nom](https://docs.rs/nom/) — Rust parser combinator library.
- [External DSLs in Practice](https://martinfowler.com/books/dsl.html) — Fowler's DSL book.

## Ship It

- `code/main.ts` and `code/main.rs`: expression DSL evaluators.
- `outputs/README.md`: DSL adoption checklist.

## Quiz

**Q1 (Pre).** What's the main advantage of an internal DSL over an external one?

- A) It's always shorter.
- B) It reuses the host language's tooling: IDE, debugger, type checker, and package manager.
- C) It has better performance.
- D) It doesn't need a parser.

**Answer: B.** Internal DSLs borrow the host language's syntax and infrastructure. You get autocomplete, type checking, error messages, and debugging for free. The cost is syntactic constraints: you can only express what the host language's grammar allows.

**Q2 (Pre).** When would you choose an external DSL?

- A) Always; external DSLs are more powerful.
- B) When the users are non-programmers, the grammar is complex, or the syntax needs to be very different from any host language.
- C) Never; internal DSLs are always better.
- D) Only for configuration files.

**Answer: B.** External DSLs make sense when: users aren't developers (so host language syntax is confusing), the grammar has unique requirements (e.g., math notation), or the DSL needs to be stored as data (files, database). They cost parser development and tooling effort.

**Q3 (Post).** What are parser combinators?

- A) A type of external DSL.
- B) Composable functions that build parsers in the host language, combining internal DSL tooling with external DSL syntax.
- C) A replacement for regular expressions.
- D) A parser generator like yacc.

**Answer: B.** Parser combinators are higher-order functions that combine smaller parsers into larger ones. You write them in the host language (Rust, Haskell, TypeScript), so you get IDE support and type checking. But the resulting parser can handle any grammar. Libraries like `nom` (Rust) and `parsec` (Haskell) implement this pattern.

**Q4 (Post).** What's a common failure mode of internal DSLs?

- A) They're too slow.
- B) The DSL syntax fights the host language's idioms, making code hard to read.
- C) They can't express complex domains.
- D) They require a separate build step.

**Answer: B.** Internal DSLs are constrained by the host language's grammar. Method chaining can become unreadable when chains are long. Operator overloading can create surprising semantics. The DSL may look like host language code but behave differently, confusing users.

**Q5 (Post).** Why do successful DSLs often start internal and go external later?

- A) External DSLs are always better.
- B) Internal DSLs are faster to build and iterate on; external DSLs are worth the cost only when the domain stabilizes and syntax constraints become limiting.
- C) Internal DSLs can't handle complex domains.
- D) External DSLs are required for production use.

**Answer: B.** Internal DSLs let you iterate quickly with full tooling support. When the domain stabilizes and the host language's syntax can't express the domain naturally, investing in an external DSL parser and tooling becomes worthwhile. The transition path is: prototype internal, mature external.

## Exercises

1. **Easy.** Add variable bindings to the expression DSL. Support `let x = 5 in x + 1`.
2. **Medium.** Add a pretty-printer and error reporting with source locations to the Rust evaluator.
3. **Hard.** Evaluate the migration path from an ad-hoc configuration format (e.g., stringly-typed YAML rules) to an internal DSL. What are the costs and benefits?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Internal DSL | "embedded API" | Domain-specific constructs encoded in the host language's syntax and types |
| External DSL | "own language" | Separate grammar and parser for domain-specific notation |
| AST | "syntax tree" | Structured representation of parsed expressions |
| Semantic drift | "spec mismatch" | DSL usage diverges from intended meaning over time |
| Parser combinator | "parser function" | Composable function building parsers in the host language |

## Further Reading

- [Language Workbenches](https://martinfowler.com/articles/languageWorkbench.html)
- [Crafting Interpreters](https://craftinginterpreters.com/)
- [Domain-Specific Languages](https://martinfowler.com/books/dsl.html) (Fowler)
- [nom: Rust parser combinators](https://docs.rs/nom/)

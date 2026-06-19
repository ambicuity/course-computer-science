// Lesson 08 — Parser Generators (yacc/bison/lalrpop/tree-sitter)
// Simulates a parser generator: define grammar rules as Rust data structures,
// demonstrate how conflicts arise and how precedence resolves them.
//
// This is not a full LALRPOP implementation — it shows the concepts.

use std::collections::{HashMap, HashSet};
use std::fmt;

// ── Grammar Representation ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol {
    NonTerminal(String),
    Terminal(String),
    Epsilon,
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::NonTerminal(name) => write!(f, "{}", name),
            Symbol::Terminal(tok) => write!(f, "'{}'", tok),
            Symbol::Epsilon => write!(f, "ε"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Production {
    pub lhs: String,
    pub rhs: Vec<Symbol>,
    pub action: String, // Description of the semantic action
}

impl fmt::Display for Production {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} →", self.lhs)?;
        for sym in &self.rhs {
            write!(f, " {}", sym)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Grammar {
    pub name: String,
    pub productions: Vec<Production>,
    pub start: String,
    pub precedence: Vec<(Associativity, Vec<String>)>, // (assoc, operators)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Associativity {
    Left,
    Right,
    NonAssoc,
}

impl fmt::Display for Associativity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Associativity::Left => write!(f, "left"),
            Associativity::Right => write!(f, "right"),
            Associativity::NonAssoc => write!(f, "nonassoc"),
        }
    }
}

impl Grammar {
    pub fn new(name: &str, start: &str) -> Self {
        Grammar {
            name: name.to_string(),
            productions: Vec::new(),
            start: start.to_string(),
            precedence: Vec::new(),
        }
    }

    pub fn add_production(&mut self, lhs: &str, rhs: Vec<Symbol>, action: &str) {
        self.productions.push(Production {
            lhs: lhs.to_string(),
            rhs,
            action: action.to_string(),
        });
    }

    pub fn set_precedence(&mut self, assoc: Associativity, operators: Vec<&str>) {
        self.precedence.push((assoc, operators.iter().map(|s| s.to_string()).collect()));
    }

    pub fn non_terminals(&self) -> HashSet<String> {
        self.productions.iter().map(|p| p.lhs.clone()).collect()
    }

    pub fn terminals(&self) -> HashSet<String> {
        let mut terms = HashSet::new();
        for p in &self.productions {
            for sym in &p.rhs {
                if let Symbol::Terminal(t) = sym {
                    terms.insert(t.clone());
                }
            }
        }
        terms
    }

    pub fn first_sets(&self) -> HashMap<String, HashSet<String>> {
        let nts = self.non_terminals();
        let mut first: HashMap<String, HashSet<String>> = HashMap::new();

        for nt in &nts {
            first.insert(nt.clone(), HashSet::new());
        }

        let mut changed = true;
        while changed {
            changed = false;
            for prod in &self.productions {
                let old_size = first[&prod.lhs].len();
                if prod.rhs.is_empty() {
                    first.get_mut(&prod.lhs).unwrap().insert("ε".to_string());
                } else {
                    for sym in &prod.rhs {
                        match sym {
                            Symbol::Terminal(t) => {
                                first.get_mut(&prod.lhs).unwrap().insert(t.clone());
                                break;
                            }
                            Symbol::NonTerminal(nt) => {
                                let sym_first = first.get(nt).cloned().unwrap_or_default();
                                let lhs_set = first.get_mut(&prod.lhs).unwrap();
                                for s in &sym_first {
                                    if s != "ε" {
                                        lhs_set.insert(s.clone());
                                    }
                                }
                                if !sym_first.contains("ε") {
                                    break;
                                }
                            }
                            Symbol::Epsilon => {
                                first.get_mut(&prod.lhs).unwrap().insert("ε".to_string());
                            }
                        }
                    }
                }
                if first[&prod.lhs].len() > old_size {
                    changed = true;
                }
            }
        }
        first
    }

    pub fn follow_sets(&self) -> HashMap<String, HashSet<String>> {
        let nts = self.non_terminals();
        let first = self.first_sets();
        let mut follow: HashMap<String, HashSet<String>> = HashMap::new();

        for nt in &nts {
            follow.insert(nt.clone(), HashSet::new());
        }
        follow.get_mut(&self.start).unwrap().insert("$".to_string());

        let mut changed = true;
        while changed {
            changed = false;
            for prod in &self.productions {
                for (i, sym) in prod.rhs.iter().enumerate() {
                    if let Symbol::NonTerminal(nt) = sym {
                        let old_size = follow[nt].len();

                        // FIRST of remaining symbols
                        let mut rest_has_epsilon = true;
                        for remaining in &prod.rhs[i + 1..] {
                            match remaining {
                                Symbol::Terminal(t) => {
                                    follow.get_mut(nt).unwrap().insert(t.clone());
                                    rest_has_epsilon = false;
                                    break;
                                }
                                Symbol::NonTerminal(rest_nt) => {
                                    for s in &first.get(rest_nt).cloned().unwrap_or_default() {
                                        if s != "ε" {
                                            follow.get_mut(nt).unwrap().insert(s.clone());
                                        }
                                    }
                                    if !first.get(rest_nt).map_or(false, |f| f.contains("ε")) {
                                        rest_has_epsilon = false;
                                        break;
                                    }
                                }
                                Symbol::Epsilon => {}
                            }
                        }

                        if rest_has_epsilon {
                            let lhs_follow = follow.get(&prod.lhs).cloned().unwrap_or_default();
                            for s in lhs_follow {
                                follow.get_mut(nt).unwrap().insert(s);
                            }
                        }

                        if follow[nt].len() > old_size {
                            changed = true;
                        }
                    }
                }
            }
        }
        follow
    }
}

// ── Conflict Detection ──────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Conflict {
    ShiftReduce {
        state_description: String,
        production: String,
        lookahead: String,
    },
    ReduceReduce {
        state_description: String,
        prod1: String,
        prod2: String,
        lookahead: String,
    },
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Conflict::ShiftReduce {
                state_description,
                production,
                lookahead,
            } => write!(
                f,
                "Shift-Reduce conflict in [{}]: shift '{}' vs reduce {}",
                state_description, lookahead, production
            ),
            Conflict::ReduceReduce {
                state_description,
                prod1,
                prod2,
                lookahead,
            } => write!(
                f,
                "Reduce-Reduce conflict in [{}]: {} vs {} on '{}'",
                state_description, prod1, prod2, lookahead
            ),
        }
    }
}

/// Detect potential conflicts in a grammar (simplified analysis).
pub fn detect_conflicts(grammar: &Grammar) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    // Look for productions that could cause shift-reduce conflicts:
    // A production whose RHS is a prefix of another production's RHS,
    // or when a non-terminal can derive a sequence that starts with
    // a terminal that also appears later in another production.

    // Example: dangling else
    // stmt → IF expr THEN stmt
    // stmt → IF expr THEN stmt ELSE stmt
    // After parsing IF expr THEN stmt, seeing ELSE causes shift-reduce conflict.

    for (i, p1) in grammar.productions.iter().enumerate() {
        for (j, p2) in grammar.productions.iter().enumerate() {
            if i >= j {
                continue;
            }
            // Check if p1.rhs is a prefix of p2.rhs (or vice versa)
            if p1.rhs.len() < p2.rhs.len() && p2.rhs.starts_with(&p1.rhs) {
                if let Some(next_sym) = p2.rhs.get(p1.rhs.len()) {
                    conflicts.push(Conflict::ShiftReduce {
                        state_description: format!("after matching {}", p1),
                        production: p1.to_string(),
                        lookahead: next_sym.to_string(),
                    });
                }
            } else if p2.rhs.len() < p1.rhs.len() && p1.rhs.starts_with(&p2.rhs) {
                if let Some(next_sym) = p1.rhs.get(p2.rhs.len()) {
                    conflicts.push(Conflict::ShiftReduce {
                        state_description: format!("after matching {}", p2),
                        production: p2.to_string(),
                        lookahead: next_sym.to_string(),
                    });
                }
            }
        }
    }

    conflicts
}

/// Resolve shift-reduce conflicts using precedence declarations.
pub fn resolve_with_precedence(
    grammar: &Grammar,
    conflicts: &[Conflict],
) -> Vec<(Conflict, String)> {
    let mut resolutions = Vec::new();

    for conflict in conflicts {
        match conflict {
            Conflict::ShiftReduce {
                lookahead,
                production,
                ..
            } => {
                // Find precedence level of lookahead and production
                let tok_prec = find_precedence(grammar, lookahead);
                let prod_prec = find_production_precedence(grammar, production);

                match (tok_prec, prod_prec) {
                    (Some((tok_level, tok_assoc)), Some(prod_level)) => {
                        if tok_level > prod_level {
                            resolutions.push((conflict.clone(), "shift (higher precedence)".to_string()));
                        } else if prod_level > tok_level {
                            resolutions.push((conflict.clone(), "reduce (higher precedence)".to_string()));
                        } else {
                            // Same precedence — use associativity
                            let resolution = match tok_assoc {
                                Associativity::Left => "reduce (left-associative)",
                                Associativity::Right => "shift (right-associative)",
                                Associativity::NonAssoc => "error (non-associative)",
                            };
                            resolutions.push((conflict.clone(), resolution.to_string()));
                        }
                    }
                    _ => {
                        // Default: shift wins
                        resolutions.push((conflict.clone(), "shift (default)".to_string()));
                    }
                }
            }
            Conflict::ReduceReduce { .. } => {
                resolutions.push((conflict.clone(), "reduce by earlier production (default)".to_string()));
            }
        }
    }

    resolutions
}

fn find_precedence<'a>(grammar: &'a Grammar, terminal: &str) -> Option<(usize, &'a Associativity)> {
    for (level, (assoc, ops)) in grammar.precedence.iter().enumerate() {
        if ops.iter().any(|op| op == terminal) {
            return Some((level, assoc));
        }
    }
    None
}

fn find_production_precedence(grammar: &Grammar, production_desc: &str) -> Option<usize> {
    // In a real parser generator, precedence of a production is determined
    // by the last terminal in its RHS (or an explicit %prec declaration).
    // Simplified: extract the last terminal from the production description.
    for (level, (_, ops)) in grammar.precedence.iter().enumerate() {
        for op in ops {
            if production_desc.contains(&format!("'{}'", op)) {
                return Some(level);
            }
        }
    }
    None
}

// ── Simulated Parser Table ──────────────────────────────────

#[derive(Debug)]
pub struct ParseAction {
    pub action_type: String,
    pub target: String,
}

/// Generate a simplified parse table showing what actions would be taken.
pub fn generate_parse_table(grammar: &Grammar) -> HashMap<String, HashMap<String, ParseAction>> {
    let mut table: HashMap<String, HashMap<String, ParseAction>> = HashMap::new();
    let follow = grammar.follow_sets();

    // For each non-terminal and each terminal in its FOLLOW set,
    // we'd have a reduce action for each complete production.
    for prod in &grammar.productions {
        let follow_set = follow.get(&prod.lhs).cloned().unwrap_or_default();
        let state_key = format!("reduce_by_{}", prod.lhs);

        for terminal in &follow_set {
            let action = ParseAction {
                action_type: "reduce".to_string(),
                target: prod.to_string(),
            };
            table
                .entry(state_key.clone())
                .or_default()
                .insert(terminal.clone(), action);
        }
    }

    // Shift actions for terminals in production RHS
    for prod in &grammar.productions {
        for sym in &prod.rhs {
            if let Symbol::Terminal(t) = sym {
                let state_key = format!("state_{}", prod.lhs);
                let action = ParseAction {
                    action_type: "shift".to_string(),
                    target: format!("goto after {}", t),
                };
                table
                    .entry(state_key)
                    .or_default()
                    .insert(t.clone(), action);
            }
        }
    }

    table
}

// ── Display ─────────────────────────────────────────────────

pub fn print_grammar(grammar: &Grammar) {
    println!("Grammar: {}", grammar.name);
    println!("Start symbol: {}\n", grammar.start);

    println!("Productions:");
    for (i, prod) in grammar.productions.iter().enumerate() {
        println!("  {}: {}", i, prod);
    }

    if !grammar.precedence.is_empty() {
        println!("\nPrecedence (lowest to highest):");
        for (level, (assoc, ops)) in grammar.precedence.iter().enumerate() {
            println!("  Level {}: {} {:?}", level, assoc, ops);
        }
    }

    let first = grammar.first_sets();
    println!("\nFIRST sets:");
    let mut keys: Vec<_> = first.keys().collect();
    keys.sort();
    for key in keys {
        let mut items: Vec<_> = first[key].iter().collect();
        items.sort();
        println!("  FIRST({}) = {{ {} }}", key, items.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }

    let follow = grammar.follow_sets();
    println!("\nFOLLOW sets:");
    let mut keys: Vec<_> = follow.keys().collect();
    keys.sort();
    for key in keys {
        let mut items: Vec<_> = follow[key].iter().collect();
        items.sort();
        println!("  FOLLOW({}) = {{ {} }}", key, items.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
    }
}

// ── Demo: Arithmetic Grammar ────────────────────────────────

fn demo_arithmetic_grammar() {
    println!("=== Arithmetic Expression Grammar (LALRPOP-style) ===\n");

    let mut grammar = Grammar::new("Arithmetic", "Expr");

    // Productions
    grammar.add_production("Expr", vec![Symbol::NonTerminal("Expr".into()), Symbol::Terminal("+".into()), Symbol::NonTerminal("Term".into())], "$1 + $3");
    grammar.add_production("Expr", vec![Symbol::NonTerminal("Expr".into()), Symbol::Terminal("-".into()), Symbol::NonTerminal("Term".into())], "$1 - $3");
    grammar.add_production("Expr", vec![Symbol::NonTerminal("Term".into())], "$1");
    grammar.add_production("Term", vec![Symbol::NonTerminal("Term".into()), Symbol::Terminal("*".into()), Symbol::NonTerminal("Factor".into())], "$1 * $3");
    grammar.add_production("Term", vec![Symbol::NonTerminal("Term".into()), Symbol::Terminal("/".into()), Symbol::NonTerminal("Factor".into())], "$1 / $3");
    grammar.add_production("Term", vec![Symbol::NonTerminal("Factor".into())], "$1");
    grammar.add_production("Factor", vec![Symbol::Terminal("NUM".into())], "$1");
    grammar.add_production("Factor", vec![Symbol::Terminal("(".into()), Symbol::NonTerminal("Expr".into()), Symbol::Terminal(")".into())], "$2");

    // Precedence
    grammar.set_precedence(Associativity::Left, vec!["+", "-"]);
    grammar.set_precedence(Associativity::Left, vec!["*", "/"]);

    print_grammar(&grammar);

    let conflicts = detect_conflicts(&grammar);
    println!("\nConflicts detected: {}", conflicts.len());
    for c in &conflicts {
        println!("  {}", c);
    }

    println!();
}

// ── Demo: Dangling Else Grammar ─────────────────────────────

fn demo_dangling_else() {
    println!("=== Dangling Else Grammar (Shift-Reduce Conflict) ===\n");

    let mut grammar = Grammar::new("DanglingElse", "Stmt");

    grammar.add_production("Stmt", vec![Symbol::Terminal("IF".into()), Symbol::NonTerminal("Expr".into()), Symbol::Terminal("THEN".into()), Symbol::NonTerminal("Stmt".into())], "if $2 then $4");
    grammar.add_production("Stmt", vec![Symbol::Terminal("IF".into()), Symbol::NonTerminal("Expr".into()), Symbol::Terminal("THEN".into()), Symbol::NonTerminal("Stmt".into()), Symbol::Terminal("ELSE".into()), Symbol::NonTerminal("Stmt".into())], "if $2 then $4 else $6");
    grammar.add_production("Stmt", vec![Symbol::Terminal("ID".into())], "$1");
    grammar.add_production("Expr", vec![Symbol::Terminal("ID".into())], "$1");

    // Precedence — ELSE binds tighter than THEN
    grammar.set_precedence(Associativity::NonAssoc, vec!["THEN"]);
    grammar.set_precedence(Associativity::NonAssoc, vec!["ELSE"]);

    print_grammar(&grammar);

    let conflicts = detect_conflicts(&grammar);
    println!("\nConflicts detected: {}", conflicts.len());
    for c in &conflicts {
        println!("  {}", c);
    }

    if !conflicts.is_empty() {
        println!("\nResolution:");
        let resolutions = resolve_with_precedence(&grammar, &conflicts);
        for (conflict, resolution) in &resolutions {
            println!("  {} => {}", conflict, resolution);
        }
    }

    println!();
}

// ── Demo: LALRPOP-style Type Annotations ────────────────────

fn demo_lalrpop_style() {
    println!("=== LALRPOP-style Grammar (Type-Safe Actions) ===\n");

    // In LALRPOP, each production has a Rust type.
    // We simulate this by annotating our productions.

    #[derive(Debug)]
    struct TypedProduction {
        lhs: String,
        lhs_type: String,
        rhs: Vec<(Symbol, Option<String>)>, // (symbol, type annotation)
        action: String,
    }

    impl fmt::Display for TypedProduction {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}: {} = ", self.lhs, self.lhs_type)?;
            for (i, (sym, ty)) in self.rhs.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                match ty {
                    Some(t) => write!(f, "<{}:{}>", sym, t)?,
                    None => write!(f, "{}", sym)?,
                }
            }
            write!(f, " => {}", self.action)
        }
    }

    let productions = vec![
        TypedProduction {
            lhs: "Expr".into(),
            lhs_type: "i32".into(),
            rhs: vec![
                (Symbol::NonTerminal("Expr".into()), Some("l:i32".into())),
                (Symbol::Terminal("+".into()), None),
                (Symbol::NonTerminal("Term".into()), Some("r:i32".into())),
            ],
            action: "l + r".into(),
        },
        TypedProduction {
            lhs: "Expr".into(),
            lhs_type: "i32".into(),
            rhs: vec![(Symbol::NonTerminal("Term".into()), None)],
            action: "Term".into(),
        },
        TypedProduction {
            lhs: "Term".into(),
            lhs_type: "i32".into(),
            rhs: vec![
                (Symbol::NonTerminal("Term".into()), Some("l:i32".into())),
                (Symbol::Terminal("*".into()), None),
                (Symbol::NonTerminal("Atom".into()), Some("r:i32".into())),
            ],
            action: "l * r".into(),
        },
        TypedProduction {
            lhs: "Term".into(),
            lhs_type: "i32".into(),
            rhs: vec![(Symbol::NonTerminal("Atom".into()), None)],
            action: "Atom".into(),
        },
        TypedProduction {
            lhs: "Atom".into(),
            lhs_type: "i32".into(),
            rhs: vec![(Symbol::Terminal("NUM".into()), Some("n:String".into()))],
            action: "n.parse::<i32>().unwrap()".into(),
        },
    ];

    for prod in &productions {
        println!("  {}", prod);
    }

    println!("\nLALRPOP checks at generation time:");
    println!("  - Expr: i32  ✓ (all actions return i32)");
    println!("  - Term: i32  ✓ (all actions return i32)");
    println!("  - Atom: i32  ✓ (parse returns i32)");
    println!("\nIf an action returned e.g. String where i32 is expected,");
    println!("LALRPOP would report a type error at generation time —");
    println!("not at runtime, as Bison would.");

    println!();
}

// ── Demo: Tree-sitter Concepts ──────────────────────────────

fn demo_treesitter() {
    println!("=== Tree-sitter Concepts ===\n");

    println!("Tree-sitter differences from Bison/LALRPOP:");
    println!("  1. GLR parsing — forks at conflicts instead of failing");
    println!("  2. Concrete syntax tree — preserves ALL tokens (ws, comments, parens)");
    println!("  3. Incremental — re-parses only changed region on edit");
    println!("  4. Error recovery — always produces a complete tree");
    println!();

    // Simulate concrete vs abstract syntax tree
    println!("Input: let x = 1 + 2;");
    println!();
    println!("AST (what Bison/LALRPOP produces):");
    println!("  VarDecl");
    println!("    name: \"x\"");
    println!("    value: BinOp(+, Num(1), Num(2))");
    println!();
    println!("CST (what tree-sitter produces):");
    println!("  source_file");
    println!("    variable_declaration");
    println!("      \"let\"");
    println!("      \" \"");
    println!("      identifier: \"x\"");
    println!("      \" \"");
    println!("      \"=\"");
    println!("      \" \"");
    println!("      binary_expression");
    println!("        number: \"1\"");
    println!("        \" \"");
    println!("        \"+\"");
    println!("        \" \"");
    println!("        number: \"2\"");
    println!("      \";\"");
    println!();
    println!("The CST preserves whitespace and punctuation.");
    println!("The AST drops them — simpler for compilation, but useless for editors.");
}

// ── Main ────────────────────────────────────────────────────

fn main() {
    demo_arithmetic_grammar();
    demo_dangling_else();
    demo_lalrpop_style();
    demo_treesitter();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_grammar() -> Grammar {
        let mut g = Grammar::new("Simple", "S");
        g.add_production("S", vec![Symbol::NonTerminal("A".into()), Symbol::NonTerminal("B".into())], "$1 $2");
        g.add_production("A", vec![Symbol::Terminal("a".into())], "$1");
        g.add_production("B", vec![Symbol::Terminal("b".into())], "$1");
        g
    }

    #[test]
    fn test_non_terminals() {
        let g = simple_grammar();
        let nts = g.non_terminals();
        assert!(nts.contains("S"));
        assert!(nts.contains("A"));
        assert!(nts.contains("B"));
    }

    #[test]
    fn test_terminals() {
        let g = simple_grammar();
        let terms = g.terminals();
        assert!(terms.contains("a"));
        assert!(terms.contains("b"));
    }

    #[test]
    fn test_first_sets() {
        let g = simple_grammar();
        let first = g.first_sets();
        assert!(first["A"].contains("a"));
        assert!(first["B"].contains("b"));
        assert!(first["S"].contains("a"));
    }

    #[test]
    fn test_follow_sets() {
        let g = simple_grammar();
        let follow = g.follow_sets();
        assert!(follow["A"].contains("b"));
        assert!(follow["S"].contains("$"));
    }

    #[test]
    fn test_dangling_else_conflict() {
        let mut g = Grammar::new("DE", "Stmt");
        g.add_production("Stmt", vec![Symbol::Terminal("IF".into()), Symbol::NonTerminal("Expr".into()), Symbol::Terminal("THEN".into()), Symbol::NonTerminal("Stmt".into())], "if-then");
        g.add_production("Stmt", vec![Symbol::Terminal("IF".into()), Symbol::NonTerminal("Expr".into()), Symbol::Terminal("THEN".into()), Symbol::NonTerminal("Stmt".into()), Symbol::Terminal("ELSE".into()), Symbol::NonTerminal("Stmt".into())], "if-then-else");

        let conflicts = detect_conflicts(&g);
        assert!(!conflicts.is_empty(), "Should detect shift-reduce conflict");
    }

    #[test]
    fn test_precedence_resolution() {
        let mut g = Grammar::new("Prec", "E");
        g.add_production("E", vec![Symbol::NonTerminal("E".into()), Symbol::Terminal("+".into()), Symbol::NonTerminal("E".into())], "add");
        g.add_production("E", vec![Symbol::NonTerminal("E".into()), Symbol::Terminal("*".into()), Symbol::NonTerminal("E".into())], "mul");
        g.add_production("E", vec![Symbol::Terminal("NUM".into())], "num");
        g.set_precedence(Associativity::Left, vec!["+", "-"]);
        g.set_precedence(Associativity::Left, vec!["*", "/"]);

        let first = g.first_sets();
        assert!(first["E"].contains("NUM"));
    }
}

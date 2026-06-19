// Lesson 07 — PEG Parsers and Packrat
// A PEG parser combinator library with packrat memoization.
//
// Combinators: literal, seq, choice, optional, many, many1, not, and_then, map
// Demo: arithmetic expression grammar using PEG combinators.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

// ── Parse Result ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ParseResult<T> {
    Success { value: T, pos: usize },
    Failure { pos: usize },
}

impl<T> ParseResult<T> {
    fn map<U, F: FnOnce(T) -> U>(self, f: F) -> ParseResult<U> {
        match self {
            ParseResult::Success { value, pos } => ParseResult::Success {
                value: f(value),
                pos,
            },
            ParseResult::Failure { pos } => ParseResult::Failure { pos },
        }
    }
}

// ── Parser Trait ────────────────────────────────────────────

pub trait Parser<T> {
    fn parse(&self, input: &str, pos: usize) -> ParseResult<T>;
}

impl<F, T> Parser<T> for F
where
    F: Fn(&str, usize) -> ParseResult<T>,
{
    fn parse(&self, input: &str, pos: usize) -> ParseResult<T> {
        self(input, pos)
    }
}

// ── Combinators ─────────────────────────────────────────────

/// Match an exact string literal at the current position.
pub fn literal(expected: &'static str) -> impl Parser<String> {
    move |input: &str, pos: usize| {
        if input[pos..].starts_with(expected) {
            ParseResult::Success {
                value: expected.to_string(),
                pos: pos + expected.len(),
            }
        } else {
            ParseResult::Failure { pos }
        }
    }
}

/// Match a sequence: parse p1, then p2. Return both results as a tuple.
pub fn seq<T: 'static, U: 'static>(
    p1: impl Parser<T> + 'static,
    p2: impl Parser<U> + 'static,
) -> impl Parser<(T, U)> {
    move |input: &str, pos: usize| match p1.parse(input, pos) {
        ParseResult::Success { value: v1, pos: pos2 } => match p2.parse(input, pos2) {
            ParseResult::Success { value: v2, pos: pos3 } => ParseResult::Success {
                value: (v1, v2),
                pos: pos3,
            },
            ParseResult::Failure { pos } => ParseResult::Failure { pos },
        },
        ParseResult::Failure { pos } => ParseResult::Failure { pos },
    }
}

/// Ordered choice: try p1 first; if it fails, try p2.
pub fn choice<T: 'static>(
    p1: impl Parser<T> + 'static,
    p2: impl Parser<T> + 'static,
) -> impl Parser<T> {
    move |input: &str, pos: usize| match p1.parse(input, pos) {
        ParseResult::Success { .. } => p1.parse(input, pos),
        ParseResult::Failure { .. } => p2.parse(input, pos),
    }
}

/// Optional: match zero or one occurrence. Returns Some(value) or None.
pub fn optional<T: 'static + Clone>(
    p: impl Parser<T> + 'static,
) -> impl Parser<Option<T>> {
    move |input: &str, pos: usize| match p.parse(input, pos) {
        ParseResult::Success { value, pos: new_pos } => ParseResult::Success {
            value: Some(value),
            pos: new_pos,
        },
        ParseResult::Failure { .. } => ParseResult::Success {
            value: None,
            pos,
        },
    }
}

/// Zero-or-more: match p as many times as possible.
pub fn many<T: 'static>(p: impl Parser<T> + 'static) -> impl Parser<Vec<T>> {
    move |input: &str, pos: usize| {
        let mut results = Vec::new();
        let mut current_pos = pos;
        loop {
            match p.parse(input, current_pos) {
                ParseResult::Success { value, pos: new_pos } => {
                    if new_pos == current_pos {
                        // No progress — avoid infinite loop
                        break;
                    }
                    results.push(value);
                    current_pos = new_pos;
                }
                ParseResult::Failure { .. } => break,
            }
        }
        ParseResult::Success {
            value: results,
            pos: current_pos,
        }
    }
}

/// One-or-more: match p at least once.
pub fn many1<T: 'static>(p: impl Parser<T> + 'static) -> impl Parser<Vec<T>> {
    move |input: &str, pos: usize| match p.parse(input, pos) {
        ParseResult::Success { value: first, pos: pos2 } => {
            let mut results = vec![first];
            let mut current_pos = pos2;
            loop {
                match p.parse(input, current_pos) {
                    ParseResult::Success { value, pos: new_pos } => {
                        if new_pos == current_pos {
                            break;
                        }
                        results.push(value);
                        current_pos = new_pos;
                    }
                    ParseResult::Failure { .. } => break,
                }
            }
            ParseResult::Success {
                value: results,
                pos: current_pos,
            }
        }
        ParseResult::Failure { pos } => ParseResult::Failure { pos },
    }
}

/// Not-predicate: succeed if p fails, without consuming input.
pub fn not<T: 'static>(p: impl Parser<T> + 'static) -> impl Parser<()> {
    move |input: &str, pos: usize| match p.parse(input, pos) {
        ParseResult::Success { .. } => ParseResult::Failure { pos },
        ParseResult::Failure { .. } => ParseResult::Success { value: (), pos },
    }
}

/// And-predicate: succeed if p succeeds, without consuming input.
pub fn and_then<T: 'static>(p: impl Parser<T> + 'static) -> impl Parser<()> {
    move |input: &str, pos: usize| match p.parse(input, pos) {
        ParseResult::Success { .. } => ParseResult::Success { value: (), pos },
        ParseResult::Failure { pos } => ParseResult::Failure { pos },
    }
}

/// Map: transform the result of a parser.
pub fn map<T: 'static, U: 'static>(
    p: impl Parser<T> + 'static,
    f: impl Fn(T) -> U + 'static,
) -> impl Parser<U> {
    move |input: &str, pos: usize| p.parse(input, pos).map(&f)
}

/// Match a single character satisfying a predicate.
pub fn satisfy(pred: impl Fn(char) -> bool + 'static) -> impl Parser<char> {
    move |input: &str, pos: usize| {
        if pos >= input.len() {
            return ParseResult::Failure { pos };
        }
        let ch = input[pos..].chars().next().unwrap();
        if pred(ch) {
            ParseResult::Success {
                value: ch,
                pos: pos + ch.len_utf8(),
            }
        } else {
            ParseResult::Failure { pos }
        }
    }
}

// ── Packrat Cache ───────────────────────────────────────────

/// A memoization cache for packrat parsing.
/// Key: (rule_id, input_position)
/// Value: (result_position, optional_value_as_string)
pub struct PackratCache {
    entries: HashMap<(usize, usize), (usize, bool)>,
    hits: usize,
    misses: usize,
}

impl PackratCache {
    pub fn new() -> Self {
        PackratCache {
            entries: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, rule_id: usize, pos: usize) -> Option<(usize, bool)> {
        if let Some(&result) = self.entries.get(&(rule_id, pos)) {
            self.hits += 1;
            Some(result)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, rule_id: usize, pos: usize, end_pos: usize, success: bool) {
        self.entries.insert((rule_id, pos), (end_pos, success));
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.hits, self.misses)
    }
}

/// A packrat-enabled parser that wraps a combinator with caching.
pub struct PackratParser<T> {
    rule_id: usize,
    parser: Rc<dyn Parser<T>>,
}

impl<T: Clone + fmt::Debug> PackratParser<T> {
    pub fn new(rule_id: usize, parser: impl Parser<T> + 'static) -> Self {
        PackratParser {
            rule_id,
            parser: Rc::new(parser),
        }
    }

    pub fn parse_cached(&self, input: &str, pos: usize, cache: &mut PackratCache) -> ParseResult<T> {
        if let Some((end_pos, success)) = cache.get(self.rule_id, pos) {
            if success {
                // Re-parse to get the value (in a real packrat, we'd store the value too)
                // For demo purposes, we just re-run — the cache tracks hit rate
                let result = self.parser.parse(input, pos);
                return result;
            } else {
                return ParseResult::Failure { pos: end_pos };
            }
        }

        let result = self.parser.parse(input, pos);
        match &result {
            ParseResult::Success { pos: new_pos, .. } => {
                cache.insert(self.rule_id, pos, *new_pos, true);
            }
            ParseResult::Failure { pos: fail_pos } => {
                cache.insert(self.rule_id, pos, *fail_pos, false);
            }
        }
        result
    }
}

// ── Arithmetic Expression Grammar via PEG ───────────────────
//
// Grammar (PEG):
//   expr   ← term (('+' / '-') term)*
//   term   ← factor (('*' / '/') factor)*
//   factor ← '-' factor / '(' expr ')' / number
//   number ← [0-9]+

#[derive(Debug, Clone, PartialEq)]
pub enum ArithExpr {
    Num(i64),
    BinOp {
        op: char,
        left: Box<ArithExpr>,
        right: Box<ArithExpr>,
    },
    Neg(Box<ArithExpr>),
}

impl fmt::Display for ArithExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArithExpr::Num(n) => write!(f, "{}", n),
            ArithExpr::BinOp { op, left, right } => write!(f, "({} {} {})", left, op, right),
            ArithExpr::Neg(inner) => write!(f, "(-{})", inner),
        }
    }
}

fn parse_digit(input: &str, pos: usize) -> ParseResult<char> {
    satisfy(|c| c.is_ascii_digit()).parse(input, pos)
}

fn parse_number(input: &str, pos: usize) -> ParseResult<ArithExpr> {
    let digits = many1(parse_digit);
    match digits.parse(input, pos) {
        ParseResult::Success { value, pos: new_pos } => {
            let s: String = value.iter().collect();
            let n: i64 = s.parse().unwrap();
            ParseResult::Success {
                value: ArithExpr::Num(n),
                pos: new_pos,
            }
        }
        ParseResult::Failure { pos } => ParseResult::Failure { pos },
    }
}

fn parse_whitespace(input: &str, pos: usize) -> ParseResult<Vec<char>> {
    many(satisfy(|c| c == ' ' || c == '\t')).parse(input, pos)
}

fn skip_ws(inner_pos: usize, input: &str) -> usize {
    match parse_whitespace(input, inner_pos) {
        ParseResult::Success { pos, .. } => pos,
        _ => inner_pos,
    }
}

fn parse_expr(input: &str, pos: usize) -> ParseResult<ArithExpr> {
    let pos = skip_ws(pos, input);
    match parse_term(input, pos) {
        ParseResult::Success { value: mut left, pos: mut current_pos } => {
            loop {
                let p = skip_ws(current_pos, input);
                if p >= input.len() {
                    break;
                }
                let ch = input[p..].chars().next();
                if ch == Some('+') || ch == Some('-') {
                    let op = ch.unwrap();
                    let after_op = skip_ws(p + 1, input);
                    match parse_term(input, after_op) {
                        ParseResult::Success { value: right, pos: new_pos } => {
                            left = ArithExpr::BinOp {
                                op,
                                left: Box::new(left),
                                right: Box::new(right),
                            };
                            current_pos = new_pos;
                        }
                        ParseResult::Failure { .. } => break,
                    }
                } else {
                    break;
                }
            }
            ParseResult::Success {
                value: left,
                pos: current_pos,
            }
        }
        ParseResult::Failure { pos } => ParseResult::Failure { pos },
    }
}

fn parse_term(input: &str, pos: usize) -> ParseResult<ArithExpr> {
    match parse_factor(input, pos) {
        ParseResult::Success { value: mut left, pos: mut current_pos } => {
            loop {
                let p = skip_ws(current_pos, input);
                if p >= input.len() {
                    break;
                }
                let ch = input[p..].chars().next();
                if ch == Some('*') || ch == Some('/') {
                    let op = ch.unwrap();
                    let after_op = skip_ws(p + 1, input);
                    match parse_factor(input, after_op) {
                        ParseResult::Success { value: right, pos: new_pos } => {
                            left = ArithExpr::BinOp {
                                op,
                                left: Box::new(left),
                                right: Box::new(right),
                            };
                            current_pos = new_pos;
                        }
                        ParseResult::Failure { .. } => break,
                    }
                } else {
                    break;
                }
            }
            ParseResult::Success {
                value: left,
                pos: current_pos,
            }
        }
        ParseResult::Failure { pos } => ParseResult::Failure { pos },
    }
}

fn parse_factor(input: &str, pos: usize) -> ParseResult<ArithExpr> {
    let pos = skip_ws(pos, input);
    if pos >= input.len() {
        return ParseResult::Failure { pos };
    }
    let ch = input[pos..].chars().next().unwrap();

    // Unary negation
    if ch == '-' {
        let after = skip_ws(pos + 1, input);
        match parse_factor(input, after) {
            ParseResult::Success { value, pos: new_pos } => {
                return ParseResult::Success {
                    value: ArithExpr::Neg(Box::new(value)),
                    pos: new_pos,
                };
            }
            ParseResult::Failure { pos } => return ParseResult::Failure { pos },
        }
    }

    // Parenthesized expression
    if ch == '(' {
        let after = skip_ws(pos + 1, input);
        match parse_expr(input, after) {
            ParseResult::Success { value, pos: inner_pos } => {
                let inner_pos = skip_ws(inner_pos, input);
                if inner_pos < input.len() && input[inner_pos..].starts_with(')') {
                    return ParseResult::Success {
                        value,
                        pos: inner_pos + 1,
                    };
                }
                ParseResult::Failure { pos: inner_pos }
            }
            ParseResult::Failure { pos } => ParseResult::Failure { pos },
        }
    } else {
        parse_number(input, pos)
    }
}

// ── Combinator Demo ─────────────────────────────────────────

fn demo_combinators() {
    println!("=== PEG Combinator Demo ===\n");

    // literal
    let p = literal("hello");
    println!("literal(\"hello\").parse(\"hello world\"):");
    println!("  {:?}\n", p.parse("hello world", 0));

    println!("literal(\"hello\").parse(\"goodbye\"):");
    println!("  {:?}\n", p.parse("goodbye", 0));

    // satisfy
    let digit = satisfy(|c| c.is_ascii_digit());
    println!("satisfy(digit).parse(\"42abc\"):");
    println!("  {:?}\n", digit.parse("42abc", 0));

    // seq
    let p = seq(literal("ab"), literal("cd"));
    println!("seq(\"ab\", \"cd\").parse(\"abcd\"):");
    println!("  {:?}\n", p.parse("abcd", 0));

    // choice
    let p = choice(literal("cat"), literal("car"));
    println!("choice(\"cat\", \"car\").parse(\"car\"):");
    println!("  {:?}\n", p.parse("car", 0));

    // optional
    let p = optional(literal("maybe"));
    println!("optional(\"maybe\").parse(\"maybe_not\"):");
    println!("  {:?}\n", p.parse("maybe_not", 0));

    println!("optional(\"maybe\").parse(\"nothing\"):");
    println!("  {:?}\n", p.parse("nothing", 0));

    // many
    let p = many(literal("ab"));
    println!("many(\"ab\").parse(\"abababxyz\"):");
    println!("  {:?}\n", p.parse("abababxyz", 0));

    // many1
    let p = many1(literal("ab"));
    println!("many1(\"ab\").parse(\"xyz\"):");
    println!("  {:?}\n", p.parse("xyz", 0));

    // not
    let p = not(literal("forbidden"));
    println!("not(\"forbidden\").parse(\"okay\"):");
    println!("  {:?}\n", p.parse("okay", 0));

    println!("not(\"forbidden\").parse(\"forbidden\"):");
    println!("  {:?}\n", p.parse("forbidden", 0));

    // and_then
    let p = and_then(literal("hello"));
    println!("and_then(\"hello\").parse(\"hello world\"):");
    println!("  {:?}\n", p.parse("hello world", 0));

    // map
    let p = map(many1(satisfy(|c| c.is_ascii_digit())), |digits: Vec<char>| {
        let s: String = digits.iter().collect();
        s.parse::<i64>().unwrap()
    });
    println!("map(many1(digit), to_i64).parse(\"12345\"):");
    println!("  {:?}\n", p.parse("12345", 0));
}

// ── Packrat Demo ────────────────────────────────────────────

fn demo_packrat() {
    println!("=== Packrat Parsing Demo ===\n");

    let inputs = vec![
        "3 + 4",
        "2 * 3 + 1",
        "10 / 2 - 1",
        "(1 + 2) * 3",
        "42",
        "-5 + 3",
        "((2 + 3) * (7 - 4)) / 3",
    ];

    for input in inputs {
        println!("Input:  {}", input);
        let mut cache = PackratCache::new();

        // Use packrat-wrapped parsers for expression rules
        let expr_parser = PackratParser::new(0, |input: &str, pos: usize| parse_expr(input, pos));

        match expr_parser.parse_cached(input, 0, &mut cache) {
            ParseResult::Success { value, pos } => {
                let (hits, misses) = cache.stats();
                println!("AST:    {}", value);
                println!("Parsed: {} chars, cache hits: {}, misses: {}", pos, hits, misses);
            }
            ParseResult::Failure { pos } => {
                println!("FAILED at position {}", pos);
            }
        }
        println!();
    }
}

// ── Main ────────────────────────────────────────────────────

fn main() {
    demo_combinators();
    println!();
    demo_packrat();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_match() {
        let p = literal("foo");
        assert_eq!(
            p.parse("foobar", 0),
            ParseResult::Success {
                value: "foo".to_string(),
                pos: 3
            }
        );
    }

    #[test]
    fn test_literal_fail() {
        let p = literal("foo");
        assert_eq!(p.parse("bar", 0), ParseResult::Failure { pos: 0 });
    }

    #[test]
    fn test_choice_first() {
        let p = choice(literal("a"), literal("b"));
        assert_eq!(
            p.parse("abc", 0),
            ParseResult::Success {
                value: "a".to_string(),
                pos: 1
            }
        );
    }

    #[test]
    fn test_choice_fallback() {
        let p = choice(literal("a"), literal("b"));
        assert_eq!(
            p.parse("bc", 0),
            ParseResult::Success {
                value: "b".to_string(),
                pos: 1
            }
        );
    }

    #[test]
    fn test_many() {
        let p = many(literal("ab"));
        assert_eq!(
            p.parse("ababab", 0),
            ParseResult::Success {
                value: vec!["ab".to_string(), "ab".to_string(), "ab".to_string()],
                pos: 6
            }
        );
    }

    #[test]
    fn test_not_predicate() {
        let p = not(literal("x"));
        assert_eq!(p.parse("y", 0), ParseResult::Success { value: (), pos: 0 });
        assert_eq!(p.parse("x", 0), ParseResult::Failure { pos: 0 });
    }

    #[test]
    fn test_arithmetic_addition() {
        match parse_expr("3 + 4", 0) {
            ParseResult::Success { value, .. } => {
                assert_eq!(value.to_string(), "(3 + 4)");
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_arithmetic_precedence() {
        match parse_expr("2 + 3 * 4", 0) {
            ParseResult::Success { value, .. } => {
                // Should parse as 2 + (3 * 4)
                assert_eq!(value.to_string(), "(2 + (3 * 4))");
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_arithmetic_parentheses() {
        match parse_expr("(2 + 3) * 4", 0) {
            ParseResult::Success { value, .. } => {
                assert_eq!(value.to_string(), "((2 + 3) * 4)");
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_packrat_cache() {
        let mut cache = PackratCache::new();
        cache.insert(0, 0, 5, true);
        assert_eq!(cache.get(0, 0), Some((5, true)));
        assert_eq!(cache.get(0, 1), None);
    }
}

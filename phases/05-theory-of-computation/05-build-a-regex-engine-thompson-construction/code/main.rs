/// Thompson Construction Regex Engine (Rust)
/// ==========================================
/// Builds an NFA from a regex, then simulates it to find all matches.
/// Supports: literal, ., *, +, ?, |, (), [abc], [^abc]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Lit(char),       // literal or '.' (stored as '.')
    Dot,             // '.' matches any char
    Alt(Box<Node>, Box<Node>),
    Concat(Vec<Node>),
    Star(Box<Node>),
    Plus(Box<Node>),
    Opt(Box<Node>),
    CharClass { chars: HashSet<char>, negated: bool },
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Char(char),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Star,
    Plus,
    Question,
    Pipe,
    Dot,
    Eof,
}

fn lex(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        chars.next();
        match c {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    chars.next();
                    tokens.push(Token::Char(next));
                }
            }
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '[' => tokens.push(Token::LBracket),
            ']' => tokens.push(Token::RBracket),
            '*' => tokens.push(Token::Star),
            '+' => tokens.push(Token::Plus),
            '?' => tokens.push(Token::Question),
            '|' => tokens.push(Token::Pipe),
            '.' => tokens.push(Token::Dot),
            _ => tokens.push(Token::Char(c)),
        }
    }
    tokens.push(Token::Eof);
    tokens
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn parse(&mut self) -> Node {
        let node = self.alt();
        assert_eq!(*self.peek(), Token::Eof, "Unexpected trailing tokens");
        node
    }

    fn alt(&mut self) -> Node {
        let mut node = self.concat();
        while *self.peek() == Token::Pipe {
            self.advance();
            let right = self.concat();
            node = Node::Alt(Box::new(node), Box::new(right));
        }
        node
    }

    fn concat(&mut self) -> Node {
        let mut nodes = Vec::new();
        while !matches!(self.peek(), Token::Eof | Token::Pipe | Token::RParen) {
            nodes.push(self.repeat());
        }
        assert!(!nodes.is_empty(), "Empty concatenation");
        if nodes.len() == 1 {
            nodes.remove(0)
        } else {
            Node::Concat(nodes)
        }
    }

    fn repeat(&mut self) -> Node {
        let mut node = self.atom();
        loop {
            match self.peek() {
                Token::Star => { self.advance(); node = Node::Star(Box::new(node)); }
                Token::Plus => { self.advance(); node = Node::Plus(Box::new(node)); }
                Token::Question => { self.advance(); node = Node::Opt(Box::new(node)); }
                _ => break,
            }
        }
        node
    }

    fn atom(&mut self) -> Node {
        match self.peek().clone() {
            Token::LParen => {
                self.advance();
                let node = self.alt();
                assert_eq!(*self.peek(), Token::RParen, "Missing )");
                self.advance();
                node
            }
            Token::LBracket => self.char_class(),
            Token::Dot => { self.advance(); Node::Dot }
            Token::Char(c) => { self.advance(); Node::Lit(c) }
            other => panic!("Unexpected token: {:?}", other),
        }
    }

    fn char_class(&mut self) -> Node {
        self.advance(); // consume [
        let negated = if *self.peek() == Token::Char('^') {
            self.advance();
            true
        } else {
            false
        };
        let mut chars = HashSet::new();
        while *self.peek() != Token::RBracket {
            match self.peek().clone() {
                Token::Char(c) => { self.advance(); chars.insert(c); }
                Token::Dot => { self.advance(); chars.insert('.'); }
                Token::Eof => panic!("Unclosed character class"),
                _ => { panic!("Unexpected in char class"); }
            }
        }
        self.advance(); // consume ]
        Node::CharClass { chars, negated }
    }
}

fn parse_regex(input: &str) -> Node {
    let tokens = lex(input);
    Parser::new(tokens).parse()
}

// ---------------------------------------------------------------------------
// NFA
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct State(usize);

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s{}", self.0)
    }
}

struct StateFactory { next: usize }

impl StateFactory {
    fn new() -> Self { Self { next: 0 } }
    fn create(&mut self) -> State {
        let s = State(self.next);
        self.next += 1;
        s
    }
}

struct NFA {
    start: State,
    accept: State,
    transitions: HashMap<(State, String), HashSet<State>>,
}

impl NFA {
    fn add(&mut self, src: State, label: &str, dst: State) {
        self.transitions
            .entry((src, label.to_string()))
            .or_default()
            .insert(dst);
    }

    fn next_states(&self, state: State, label: &str) -> HashSet<State> {
        self.transitions
            .get(&(state, label.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    fn eps_closure(&self, states: &HashSet<State>) -> HashSet<State> {
        let mut closure = states.clone();
        let mut queue: VecDeque<State> = states.iter().copied().collect();
        while let Some(s) = queue.pop_front() {
            for &t in &self.next_states(s, "") {
                if closure.insert(t) {
                    queue.push_back(t);
                }
            }
        }
        closure
    }
}

// ---------------------------------------------------------------------------
// Thompson Construction
// ---------------------------------------------------------------------------

fn thompson(node: &Node, factory: &mut StateFactory) -> NFA {
    match node {
        Node::Lit(c) => {
            let start = factory.create();
            let accept = factory.create();
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            nfa.add(start, &c.to_string(), accept);
            nfa
        }
        Node::Dot => {
            let start = factory.create();
            let accept = factory.create();
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            nfa.add(start, ".", accept);
            nfa
        }
        Node::CharClass { chars, negated } => {
            let start = factory.create();
            let accept = factory.create();
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            if *negated {
                let mut sorted: Vec<char> = chars.iter().copied().collect();
                sorted.sort();
                let label = format!("[^{}]", sorted.iter().collect::<String>());
                nfa.add(start, &label, accept);
            } else {
                for c in chars {
                    nfa.add(start, &c.to_string(), accept);
                }
            }
            nfa
        }
        Node::Alt(left, right) => {
            let nfa_left = thompson(left, factory);
            let nfa_right = thompson(right, factory);
            let start = factory.create();
            let accept = factory.create();
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            // Merge transitions
            for (k, v) in nfa_left.transitions.iter() {
                nfa.transitions.entry(k.clone()).or_default().extend(v);
            }
            for (k, v) in nfa_right.transitions.iter() {
                nfa.transitions.entry(k.clone()).or_default().extend(v);
            }
            nfa.add(start, "", nfa_left.start);
            nfa.add(start, "", nfa_right.start);
            nfa.add(nfa_left.accept, "", accept);
            nfa.add(nfa_right.accept, "", accept);
            nfa
        }
        Node::Concat(nodes) => {
            let nfas: Vec<NFA> = nodes.iter().map(|n| thompson(n, factory)).collect();
            // Link accept of each to start of next
            let mut extra: Vec<(State, State)> = Vec::new();
            for i in 0..nfas.len() - 1 {
                extra.push((nfas[i].accept, nfas[i + 1].start));
            }
            let start = nfas[0].start;
            let accept = nfas.last().unwrap().accept;
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            for nf in &nfas {
                for (k, v) in nf.transitions.iter() {
                    nfa.transitions.entry(k.clone()).or_default().extend(v);
                }
            }
            for (a, b) in extra {
                nfa.add(a, "", b);
            }
            nfa
        }
        Node::Star(inner) => {
            let inner_nfa = thompson(inner, factory);
            let start = factory.create();
            let accept = factory.create();
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            for (k, v) in inner_nfa.transitions.iter() {
                nfa.transitions.entry(k.clone()).or_default().extend(v);
            }
            nfa.add(start, "", inner_nfa.start);
            nfa.add(start, "", accept);
            nfa.add(inner_nfa.accept, "", inner_nfa.start);
            nfa.add(inner_nfa.accept, "", accept);
            nfa
        }
        Node::Plus(inner) => {
            // A+ = A A*
            let inner_nfa = thompson(inner, factory);
            let star_start = factory.create();
            let star_accept = factory.create();
            let mut nfa = NFA { start: inner_nfa.start, accept: star_accept, transitions: HashMap::new() };
            for (k, v) in inner_nfa.transitions.iter() {
                nfa.transitions.entry(k.clone()).or_default().extend(v);
            }
            nfa.add(inner_nfa.accept, "", star_start);
            nfa.add(star_start, "", inner_nfa.start);
            nfa.add(inner_nfa.accept, "", star_accept);
            nfa
        }
        Node::Opt(inner) => {
            let inner_nfa = thompson(inner, factory);
            let start = factory.create();
            let accept = factory.create();
            let mut nfa = NFA { start, accept, transitions: HashMap::new() };
            for (k, v) in inner_nfa.transitions.iter() {
                nfa.transitions.entry(k.clone()).or_default().extend(v);
            }
            nfa.add(start, "", inner_nfa.start);
            nfa.add(start, "", accept);
            nfa.add(inner_nfa.accept, "", accept);
            nfa
        }
    }
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

fn simulate(nfa: &NFA, text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut matches = Vec::new();

    for start_pos in 0..chars.len() {
        let mut current = nfa.eps_closure(&[nfa.start].into_iter().collect());
        let mut found = false;

        for i in start_pos..chars.len() {
            let mut next: HashSet<State> = HashSet::new();
            for &s in &current {
                // Exact char
                for &t in &nfa.next_states(s, &chars[i].to_string()) {
                    next.insert(t);
                }
                // Dot wildcard
                for &t in &nfa.next_states(s, ".") {
                    next.insert(t);
                }
                // Negated char classes
                for ((src, label), dsts) in &nfa.transitions {
                    if *src == s && label.starts_with("[^") && label.ends_with(']') {
                        let excluded: HashSet<char> = label[2..label.len()-1].chars().collect();
                        if !excluded.contains(&chars[i]) {
                            next.extend(dsts);
                        }
                    }
                }
            }
            current = nfa.eps_closure(&next);
            if current.contains(&nfa.accept) {
                matches.push((start_pos, i + 1));
                found = true;
                break;
            }
            if current.is_empty() {
                break;
            }
        }
        let _ = found;
    }

    matches.sort();
    matches.dedup();
    matches
}

fn match_regex(pattern: &str, text: &str) -> Vec<(usize, usize)> {
    let ast = parse_regex(pattern);
    let mut factory = StateFactory::new();
    let nfa = thompson(&ast, &mut factory);
    simulate(&nfa, text)
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

fn demo() {
    println!("{}", "=".repeat(60));
    println!("Thompson Construction Regex Engine (Rust)");
    println!("{}", "=".repeat(60));

    let tests: Vec<(&str, &str, &str)> = vec![
        ("a", "banana", "Single literal"),
        ("a*", "aaab", "Kleene star"),
        ("a|b", "cabd", "Union"),
        ("(a|b)*abb", "aabbababbab", "Classic example"),
        ("a+b", "aaab ab", "One-or-more"),
        ("colou?r", "color colour", "Optional"),
        (".*end", "the end", "Dot wildcard"),
        ("[abc]+", "aabccba", "Character class"),
    ];

    for (pattern, text, desc) in &tests {
        println!("\nPattern: {:?}  Text: {:?}  ({})", pattern, text, desc);
        let results = match_regex(pattern, text);
        if results.is_empty() {
            println!("  No match");
        } else {
            for (s, e) in &results {
                let matched: String = text.chars().skip(*s).take(e - s).collect();
                println!("  Match [{}:{}] = {:?}", s, e, matched);
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Engine supports: literal, ., *, +, ?, |, (), [abc], [^abc]");
    println!("{}", "=".repeat(60));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 {
        let results = match_regex(&args[1], &args[2]);
        if results.is_empty() {
            println!("No match");
        } else {
            for (s, e) in &results {
                let matched: String = args[2].chars().skip(*s).take(e - s).collect();
                println!("Match [{}:{}]: {}", s, e, matched);
            }
        }
    } else {
        demo();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal() {
        assert_eq!(match_regex("a", "banana"), vec![(1, 2), (3, 4), (5, 6)]);
    }

    #[test]
    fn test_star() {
        assert_eq!(match_regex("a*", "aaab"), vec![(0, 3)]);
    }

    #[test]
    fn test_union() {
        assert_eq!(match_regex("a|b", "cabd"), vec![(1, 2), (2, 3)]);
    }

    #[test]
    fn test_classic() {
        assert_eq!(match_regex("(a|b)*abb", "aabbab"), vec![(0, 4), (3, 6)]);
    }

    #[test]
    fn test_plus() {
        assert_eq!(match_regex("a+b", "aaab"), vec![(0, 4)]);
    }

    #[test]
    fn test_optional() {
        assert_eq!(match_regex("colou?r", "colour"), vec![(0, 6)]);
        assert_eq!(match_regex("colou?r", "color"), vec![(0, 5)]);
    }

    #[test]
    fn test_dot() {
        assert_eq!(match_regex(".*end", "the end"), vec![(0, 7)]);
    }

    #[test]
    fn test_char_class() {
        assert_eq!(match_regex("[abc]+", "aabccba"), vec![(0, 7)]);
    }
}

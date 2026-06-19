# Lesson 05: Build a Regex Engine — Thompson Construction

## Overview

Regular expressions are everywhere — grep, text editors, log parsers, input validators.
But how does a regex engine actually work? This lesson builds one from scratch using
**Thompson's construction**, the algorithm behind POSIX `grep -E` and libraries like RE2.

By the end, you will implement a regex engine that converts patterns like `(a|b)*abb`
into a nondeterministic finite automaton (NFA), simulates it, and returns match positions.

## Build It: Regex Engine from Scratch

### Step 1: Regex → AST (Parser)

A regex is a small language with its own grammar:

```
regex   ::= alt
alt     ::= concat ('|' concat)*
concat  ::= repeat+
repeat  ::= atom ('*' | '+' | '?')?
atom    ::= char | '.' | '(' alt ')' | '[' char* ']'
```

The parser uses recursive descent. Input `(a|b)*abb` produces:

```
Concat(
  Repeat(Alt(Lit('a'), Lit('b')), '*'),
  Concat(Lit('a'), Lit('b'), Lit('b'))
)
```

### Step 2: AST → NFA (Thompson Construction)

Thompson's construction (1968) converts any regex AST into an NFA with:
- Exactly one start state and one accept state
- Transitions labeled by characters or ε (epsilon = empty string)
- At most two outgoing ε-transitions from any state

**Base cases:**

| Pattern | NFA |
|---------|-----|
| `a` (literal) | Start →[a]→ Accept |
| `ε` | Start →[ε]→ Accept |
| `∅` | Start Accept (no path) |

**Inductive cases:**

**Concatenation — `AB`:**
Connect Accept(A) to Start(B) via ε.
Start state comes from A; accept state from B.

**Union — `A|B`:**
Create new start state with ε to Start(A) and Start(B).
Create new accept state with ε from Accept(A) and Accept(B).

**Kleene star — `A*`:**
Create new start and accept states.
ε from new start to Start(A) and to new accept.
ε from Accept(A) back to Start(A).
ε from Accept(A) to new accept.

Every construction adds at most O(1) states and transitions. Total NFA size = O(|regex|).

### Step 3: NFA Simulation

The NFA may be in multiple states simultaneously. We simulate it with **state sets**:

```
function simulate(nfa, text):
    current = ε-closure({nfa.start})
    for each char c in text:
        next = ∅
        for each state s in current:
            for each transition s --c--> t:
                next = next ∪ ε-closure({t})
        current = next
    if current ∩ nfa.accept ≠ ∅:
        return MATCH
```

**ε-closure(S)** = all states reachable from S by following zero or more ε-transitions.
Computed via BFS/DFS in O(|states|) time.

### Matching Substrings

To find all matches in a text, run the simulator starting at every position.
For `(a|b)*abb` against `"aabbab"`:
- Position 0: `"aabb"` matches (states reach accept)
- Position 1: no match
- Position 2: `"abb"` matches
- Position 5: no match

## NFA vs DFA: Why Thompson NFA?

**Thompson NFA** (this lesson):
- Guaranteed linear time construction: O(|regex|) states
- Simulation is O(|text| × |states|) — polynomial, predictable
- Used by RE2, Google's safe regex engine

**DFA engines** (like classical grep):
- DFA can be exponentially larger: 2^n states for n-state NFA
- Matching is O(|text|) — linear in text
- Preprocessing cost can be enormous

**Backtracking engines** (PCRE, Python `re`):
- Exponential worst case on patterns like `(a|a)*` with certain inputs
- Catastrophic backtracking: `aaaaaaaaab` against `(a+)+b` takes minutes
- Thompson NFA never has this problem

## Use It: grep -E Uses Thompson NFA

When you run `grep -E 'pattern' file`, the implementation typically:
1. Parses the extended regex
2. Applies Thompson's construction to build an NFA
3. Converts to a DFA (subset construction) for speed
4. Simulates the DFA on each line

The POSIX standard requires leftmost-longest matching, which the NFA approach supports naturally.

## Ship It: Regex Engine CLI

```bash
# Search for pattern in text
python main.py '(a|b)*abb' 'aabbababbab'

# Output:
# Match at [0..4]: aabb
# Match at [4..7]: abb
# Match at [7..10]: abb

# Pipe with echo
echo "hello world 123" | python main.py '[0-9]+'
# Output:
# Match at [12..15]: 123
```

## Implementation Notes

### Handling Character Classes `[abc]`

Character classes expand to a union of literals:
`[abc]` → `Alt(Lit('a'), Alt(Lit('b'), Lit('c')))`

Negated classes `[^abc]` match any character NOT in the set.
The dot `.` is equivalent to `[^]` (any character).

### Handling `+` and `?`

These are syntactic sugar, desugared during parsing:
- `A+` → `A A*` (one or more)
- `A?` → `(A | ε)` (zero or one)

## Exercises

### Level 1 — Trace
For regex `a|b*`:
1. Draw the NFA produced by Thompson's construction
2. Trace the simulation on input `"aab"`
3. List all state sets visited at each step

### Level 2 — Implement
Extend the engine to support:
- Backreferences: `(a)\1` matches `aa` but not `ab`
- Why is this NOT possible with Thompson NFA? (Hint: requires memory)
- Implement backreferences using a different approach

### Level 3 — Prove
1. Prove that Thompson NFA construction produces O(|regex|) states
2. Prove that NFA simulation runs in O(|text| × |states|) time
3. Show that converting the NFA to DFA can produce 2^|states| states
   by constructing a specific regex where this bound is tight

## Summary

Thompson's construction turns any regex into an NFA with linear-size overhead.
NFA simulation via ε-closure avoids the exponential blowup of DFA conversion
and the catastrophic backtracking of recursive engines. This is the foundation
of safe, predictable regex engines like RE2.

Key takeaways:
- Regex → AST → NFA → simulation = matching
- Thompson NFA: O(|regex|) construction, O(|text|·|states|) matching
- No catastrophic backtracking — every match attempt is bounded
- Used in production: RE2, POSIX grep, Rust `regex` crate

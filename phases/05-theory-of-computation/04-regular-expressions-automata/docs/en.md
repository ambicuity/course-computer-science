# Lesson 04: Regular Expressions ↔ Automata

## What You'll Learn

- Regular expressions as an algebraic language: union, concatenation, Kleene star
- The equivalence: regex ⇔ NFA ⇔ DFA ⇔ regular language
- Thompson construction: building an NFA from a regex
- State elimination: extracting a regex from an NFA
- The pumping lemma: why {aⁿbⁿ} is not regular

---

## 1. Regular Expressions: A Formal Language

A **regular expression** over alphabet Σ defines a language (set of strings):

| Operation | Notation | Meaning |
|-----------|----------|---------|
| **Union** | R₁ \| R₂ | Strings matching R₁ **or** R₂ |
| **Concatenation** | R₁ R₂ | R₁ followed by R₂ |
| **Kleene star** | R* | Zero or more repetitions of R |

Base cases: `a` → {a}, `ε` → {""}, `∅` → {}.

| Regex | Language |
|-------|----------|
| `a*` | {ε, a, aa, aaa, …} |
| `(a\|b)*` | All strings over {a, b} |
| `a(a\|b)*b` | Starts with a, ends with b |
| `(ab)*` | {ε, ab, abab, …} |

---

## 2. The Equivalence Theorem

```
Regular Expression  ⟺  NFA  ⟺  DFA  ⟺  Regular Language
```

All four formalisms describe exactly the same class of languages:

- Regex → NFA via Thompson construction
- NFA → DFA via subset construction (Lesson 03)
- DFA → regex via state elimination
- DFA can always be minimized (Hopcroft's algorithm)

This equivalence makes regex engines possible: a programmer writes a regex,
the engine converts it to an automaton, and the automaton does the matching.

---

## 3. Thompson Construction: Regex → NFA

**Thompson's construction** builds an NFA from a regex by composing small NFA
fragments using three rules, one per operation.

### Base: Single Symbol

For symbol `a`, two states with a single transition:

```
──→ (q₀) ──a─→ ((q₁))
```

### Union: R₁ | R₂

New start/accept with ε-transitions branching to both sub-NFAs:

```
         ┌─ε→ [NFA for R₁] ─ε─┐
──→ (s) ─┤                    ├→ ((f))
         └─ε→ [NFA for R₂] ─ε─┘
```

### Concatenation: R₁ R₂

Accept states of R₁ connect via ε to start of R₂:

```
──→ [NFA for R₁] ──ε──→ [NFA for R₂] ──→
```

### Kleene Star: R*

New start/accept with ε-loops for zero-or-more:

```
          ┌──────────ε──────────┐
          ▼                     │
──→ (s) ─ε→ [NFA for R] ──ε─→ ((f))
          │                     ▲
          └──────────ε──────────┘
```

The result always has exactly **one start** and **one accept** state, enabling
composition.

---

## 4. State Elimination: NFA → Regex

The reverse direction uses **state elimination**: add new start/accept states,
then repeatedly remove internal states, updating edge labels to regex
expressions capturing all paths through the removed state. When only start
and accept remain, the edge label is the equivalent regex.

---

## 5. The Pumping Lemma (Preview)

The pumping lemma proves a language is **not** regular. If L is regular with
pumping length p, every string s ∈ L with |s| ≥ p can be written as s = xyz
where |xy| ≤ p, |y| > 0, and xyⁱz ∈ L for all i ≥ 0.

**Intuition:** In a DFA with p states, any string longer than p visits some
state twice. The substring y can be "pumped."

**Example:** L = {aⁿbⁿ} is not regular. Take s = aᵖbᵖ. Since |xy| ≤ p, y
consists only of a's. Pumping down (i = 0) gives fewer a's than b's, so
xy⁰z ∉ L. Contradiction. ∎

---

## 6. Build It: Regex Parser + NFA Construction

See `code/main.py` for:

- Recursive-descent parser supporting `|`, `*`, concatenation, parentheses
- Thompson construction building an NFA from the parse tree
- `regex_to_nfa(pattern)` — one call to get an NFA
- Verification against Python's `re` module

---

## 7. Use It

- **grep / egrep**: Thompson NFA simulation with BFS (no backtracking).
- **awk / sed**: Stream processing guided by regex automata.
- **Programming languages**: Python `re`, JS RegExp, Java `Pattern` — all
  automata-based or backtracking engines.
- **Lexical analysis**: `flex` compiles regex specs to DFA table-lookup code.

## 8. Ship It

1. Parse a regex string into an AST
2. Apply Thompson construction to produce an NFA
3. Optionally subset-construct a DFA
4. Test strings against both the automaton and Python's `re`

## 9. Exercises

### Level 1 — By Hand

1. Draw the NFA from Thompson construction for `a*b|c`.
2. Convert `(ab)*` to an NFA. How many states does it have?
3. Apply state elimination to the NFA for {aa, ab} to recover a regex.

### Level 2 — Pumping Lemma

4. Prove that {aⁿbⁿ | n ≥ 0} is not regular.
5. Prove that the language of palindromes over {a, b} is not regular.
6. Is {w ∈ {a, b}* | w has equal a's and b's} regular? Prove or disprove.

### Level 3 — Implementation

7. Extend the regex parser to support `+` (one or more) and `?` (zero or one),
   rewriting them as concatenation/Kleene star before building the NFA.
8. Implement `nfa_to_regex(nfa)` using state elimination.
9. Benchmark `regex_to_nfa` against Python's `re` on 1000 random strings.

## Summary

Regex, NFAs, DFAs, and regular languages are four faces of the same concept.
Thompson bridges regex and NFA; subset construction bridges NFA and DFA. The
pumping lemma proves certain languages fall outside this class. Together, these
results form the foundation of pattern matching and lexical analysis.

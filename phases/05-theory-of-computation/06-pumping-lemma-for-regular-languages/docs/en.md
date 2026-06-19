# Lesson 06: Pumping Lemma for Regular Languages

## Overview

The pumping lemma gives us a **necessary condition** for a language to be regular:
if a language is regular, then all sufficiently long strings in it can be "pumped"
(repeated) at a middle segment and still remain in the language. When pumping fails,
the language is **not regular** — a powerful negative result.

## Build It: Pumping Lemma Proof Assistant

### Statement

If L is a regular language, then there exists a pumping length p ≥ 1 such that
every string w ∈ L with |w| ≥ p can be written as w = xyz where:

1. |xy| ≤ p (the pumped part is near the beginning)
2. |y| ≥ 1 (the pumped part is non-empty)
3. xyⁱz ∈ L for all i ≥ 0 (pumping y any number of times stays in L)

### Proof Idea: Pigeonhole on DFA States

Suppose L is regular with a DFA of p states. Take any w ∈ L with |w| ≥ p.
As the DFA reads w, it visits p+1 states (including start). By the pigeonhole
principle, some state repeats: the path forms a loop.

```
w = x · y · z
     ↓   ↓   ↓
  start → q → q → ... → accept
          ↑___↓
         the loop is y
```

- `x` = prefix before the loop enters state q
- `y` = the loop (at least one character, since states must repeat)
- `z` = suffix from second visit to q to accept

Pumping y means going around the loop 0, 2, 3, ... times, each still reaching accept.

## How to Use It: Proving Non-Regularity

### Example 1: {aⁿbⁿ | n ≥ 0} is NOT Regular

**Proof by contradiction:**

1. Assume L = {aⁿbⁿ | n ≥ 0} is regular with pumping length p.
2. Choose w = aᵖbᵖ ∈ L. Note |w| = 2p ≥ p.
3. By the pumping lemma, w = xyz with |xy| ≤ p and |y| ≥ 1.
4. Since |xy| ≤ p, both x and y consist entirely of a's.
5. Pump i = 0: xz = aᵖ⁻|y|bᵖ. This has fewer a's than b's.
6. So xz ∉ L — contradiction.

Therefore L is not regular. ∎

### Example 2: {ww | w ∈ Σ*} is NOT Regular

**Proof by contradiction:**

1. Assume L = {ww | w ∈ {a,b}*} is regular with pumping length p.
2. Choose w = aᵖaᵖ = a²ᵖ ∈ L. Note |w| = 2p ≥ p.
3. By the pumping lemma, w = xyz with |xy| ≤ p and |y| ≥ 1.
4. Since |xy| ≤ p, y consists entirely of a's, say y = aᵏ with k ≥ 1.
5. Pump i = 0: xz = a²ᵖ⁻ᵏ. Now 2p − k is odd, so xz cannot equal vv
   for any v (since |vv| = 2|v| is always even).
6. So xz ∉ L — contradiction.

Therefore L is not regular. ∎

### The Key Technique

The pumping lemma is a **necessary** condition. To prove L is not regular:
1. Assume L is regular (for contradiction)
2. State the pumping lemma with pumping length p
3. **Choose** a specific w ∈ L with |w| ≥ p (this is the critical step)
4. **Show** that for every valid partition w = xyz, some pumped version xyⁱz ∉ L
5. Conclude contradiction

### What the Pumping Lemma Cannot Do

The pumping lemma does **not** say "if pumping works, then L is regular."
It is necessary but not sufficient. For example:
- L = {aⁱbʲcᵏ | i,j,k ≥ 0} is regular (it equals a*b*c*)
- L = {aⁿbⁿ | n ≥ 1} is NOT regular

Both look like counting languages, but only the first is regular.
The pumping lemma helps distinguish them.

## Use It: Proving Language Non-Regularity

Languages you can prove non-regular with pumping:
- {wwᴿ | w ∈ Σ*} — palindromes (even-length, mirrored)
- {aⁿ | n is prime} — primes cannot be pumped
- {aⁿbⁿ | n ≥ 0} — balanced a's and b's
- {ww | w ∈ Σ*} — duplicated strings
- {aⁿbᵐ | n < m} — strict inequality on counts

## Ship It: Pumping Lemma Checker

```bash
python main.py
# Demonstrates pumping on regular language a*b*
# Shows pumping fails for {a^n b^n}
# Shows pumping fails for {ww}
```

## Summary

The pumping lemma turns DFA theory into a practical proof tool. The pigeonhole
principle guarantees that long strings must revisit DFA states, creating loops
that can be pumped. When pumping fails for a carefully chosen string, the language
is not regular.

Key takeaways:
- Pumping lemma: necessary condition for regularity
- Proof technique: assume regular, choose w, show pumping fails
- |xy| ≤ p restricts where the loop can be — use this to your advantage
- The lemma does NOT give sufficiency — some non-regular languages still pump

## Exercises

### Level 1 — Trace
Show that pumping works for the regular language L = a*b*:
1. Choose p = 2
2. Take w = aaabb ∈ L (|w| = 5 ≥ 2)
3. Show there exists a valid xyz decomposition where all xyⁱz ∈ L

### Level 2 — Prove
Prove that L = {aⁿbᵐ | n > m ≥ 0} is not regular:
1. State the pumping lemma assumption
2. Choose an appropriate w
3. Show pumping fails for every valid partition

### Level 3 — Extend
1. Prove the pumping lemma holds for all regular languages (full proof using pigeonhole)
2. Show that L = {aⁿbⁿ | n is prime} is not context-free using the pumping lemma for CFLs
3. Is there a language that satisfies the pumping lemma but is not context-free? (Hint: Ogden's lemma)

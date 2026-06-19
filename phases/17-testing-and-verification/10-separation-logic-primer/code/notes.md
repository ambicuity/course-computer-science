# Separation Logic Notes

## Predicate snippets

- `x |-> v` : address `x` stores value `v`
- `emp` : empty heap
- `P * Q` : disjoint heap decomposition

## Frame rule shape

From `{P} C {Q}` infer `{P * R} C {Q * R}` if `C` does not touch `R` footprint.

## Why this matters

- Local proofs scale better than whole-heap reasoning.
- Aliasing bugs show up as unsatisfiable ownership assumptions.

## Mini linked-list predicate

`list(x)`
- `x = null` implies `emp`
- else `exists v,n. x |-> (v,n) * list(n)`

## Practical checklist

1. Declare ownership boundaries.
2. State footprint of each command.
3. Apply frame rule for untouched regions.
4. Re-check alias assumptions.

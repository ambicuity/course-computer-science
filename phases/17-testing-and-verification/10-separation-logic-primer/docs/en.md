# Separation Logic Primer

> Local reasoning about memory works when ownership is explicit.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 17 lessons 01-09
**Time:** ~75 minutes

## Learning Objectives

- Understand heap assertions and separating conjunction `*`.
- Use ownership framing to reason locally about pointer-manipulating code.
- Distinguish aliasing-safe and aliasing-unsafe specs.
- Connect separation logic ideas to modern borrow/ownership systems.

## The Problem

Traditional Hoare logic works beautifully for programs that manipulate local
variables. But the moment you add pointers, things fall apart.

Consider this C code:

```c
void swap(int *x, int *y) {
    int tmp = *x;
    *x = *y;
    *y = tmp;
}
```

A Hoare logic spec might say: `{*x = a ∧ *y = b} swap(x, y) {*x = b ∧ *y = a}`.
That looks correct. But what if `x == y`? Then after `swap`, both `*x` and `*y`
equal `b`, not `a`. The spec is wrong when the pointers alias.

Traditional assertions can't express "x and y point to *different* memory
locations." The conjunction `*x = a ∧ *y = b` is satisfied even when `x == y`
and `a == b`. You have no way to say "these are disjoint."

This is the **aliasing problem**. In any program with mutable heap data, local
reasoning breaks down because updating one pointer might invalidate assumptions
about another pointer that happens to share the same memory.

Separation logic solves this with a new connective: the **separating
conjunction** `*`, which explicitly requires disjointness.

## The Concept

### The Separating Conjunction

In classical logic, `P ∧ Q` means "both P and Q hold." In separation logic,
`P * Q` means "the heap can be split into two disjoint parts, one satisfying P
and the other satisfying Q."

```
    Classical:  P ∧ Q
                "Both true in the same world"
    
    Separating: P * Q
                "Heap splits into two parts, P holds in one, Q in the other"
    
    ┌─────────────────────────────┐
    │         Full Heap           │
    │  ┌──────────┐ ┌──────────┐ │
    │  │  P part  │ │  Q part  │ │
    │  │ (owns    │ │ (owns    │ │
    │  │  these   │ │  those   │ │
    │  │  cells)  │ │  cells)  │ │
    │  └──────────┘ └──────────┘ │
    │     Disjoint regions        │
    └─────────────────────────────┘
```

### Points-To Assertions

The basic heap assertion is `l ↦ v` ("l points to v"), meaning location `l`
holds value `v` in the heap.

```
    Heap:
    ┌─────┬─────┬─────┬─────┬─────┐
    │  0  │  1  │  2  │  3  │  4  │  Location
    ├─────┼─────┼─────┼─────┼─────┤
    │ 42  │ 17  │  ?  │  99 │  ?  │  Value
    └─────┴─────┴─────┴─────┴─────┘
    
    1 ↦ 17       "location 1 holds 17"
    3 ↦ 99       "location 3 holds 99"
    1 ↦ 17 * 3 ↦ 99    "locations 1 and 3 hold these values, and they're different"
```

The expression `1 ↦ 17 * 3 ↦ 99` asserts that locations 1 and 3 are *distinct*
and hold the stated values. If you wrote `1 ↦ 17 ∧ 3 ↦ 99`, you'd allow `1 == 3`
(which would be inconsistent since they'd need to hold both 17 and 99). The `*`
connective prevents this by requiring disjointness.

### The Frame Rule

This is the key insight that makes separation logic practical:

```
    {P} C {Q}
    ─────────────────────────── (Frame)
    {P * R} C {Q * R}
```

If command `C` transforms `P` into `Q`, then `C` also transforms `P * R` into
`Q * R`, *provided that `C` does not touch any location in `R`*.

The frame rule lets you reason about a piece of code *locally*: you only need
to know about the memory the code actually accesses (`P`). Everything else (`R`)
stays unchanged. You don't need to re-prove global properties after every
local update.

```
    Before C:                    After C:
    ┌──────────┬──────────┐     ┌──────────┬──────────┐
    │  P part  │  R part  │     │  Q part  │  R part  │
    │ (accessed│ (untouchd│     │ (changed │ (still   │
    │  by C)   │  by C)   │     │  by C)   │  same)   │
    └──────────┴──────────┘     └──────────┴──────────┘
    
    C only modifies P's region. R stays valid without re-proof.
```

This is why separation logic scales. Without the frame rule, verifying a
function that modifies one pointer would require re-verifying every property
of every other pointer in the program. With the frame rule, you only re-verify
the part the function actually touches.

### Example: List Node Update

Consider a linked list node at location `n`:

```
    n ↦ node{value: v, next: p}
```

To update the value:

```
    {n ↦ node{value: v, next: p} * p ↦ node{value: w, next: q}}
    n.value := 42
    {n ↦ node{value: 42, next: p} * p ↦ node{value: w, next: q}}
```

The frame rule lets us keep `p ↦ ...` in the spec without worrying about it.
The assignment `n.value := 42` only touches location `n`. The assertion about
`p` is framed out and remains valid.

### Aliasing Breaks Separation

If `x == y`, then `x ↦ a * y ↦ b` is *unsatisfiable* (you can't split the
heap into disjoint parts when both point to the same cell). This is a feature,
not a bug: it means separation logic *detects* aliasing at the spec level.

```
    x ↦ a * y ↦ b    — asserts x ≠ y (disjoint cells)
    x ↦ a ∧ y ↦ b    — allows x = y (classical conjunction)
```

When you write a spec with `*` and it's unsatisfiable, you've discovered that
your code has an aliasing problem. The spec catches it before the code runs.

### Connection to Rust's Ownership

Rust's ownership model is a *syntactic* version of separation logic's ideas:

| Separation Logic | Rust |
|---|---|
| `x ↦ v` (owns cell) | `let x = v;` (owns value) |
| Separating conjunction `*` | Each value has exactly one owner |
| Frame rule (local reasoning) | Borrow checker ensures no aliasing under `&mut` |
| Ownership transfer | `move` semantics |
| Borrowing (`&`) | Shared reference (aliasing allowed, mutation forbidden) |
| Mutable borrow (`&mut`) | Exclusive reference (no aliasing, mutation allowed) |

Rust doesn't use formal separation logic proofs, but its borrow checker enforces
similar invariants: you can't have two mutable references to the same data. This
is the practical engineering version of "disjoint ownership."

## Build It

### Step 1: Heap-cell predicates for a linked list

Specify a singly linked list `a -> b -> c`:

```
    list(a, [v1, v2, v3]) =
        a ↦ node{value: v1, next: b} *
        b ↦ node{value: v2, next: c} *
        c ↦ node{value: v3, next: null}
```

Each `*` asserts that `a`, `b`, and `c` are distinct locations. If any two are
equal, the assertion is unsatisfiable.

### Step 2: Prove a head-pointer update

```c
void push(Node **head, int value) {
    Node *new_node = malloc(sizeof(Node));
    new_node->value = value;
    new_node->next = *head;
    *head = new_node;
}
```

Spec:

```
    {head ↦ p * list(p, vs)}
    push(head, v)
    {head ↦ q * q ↦ node{value: v, next: p} * list(p, vs)}
```

The frame rule applies: `list(p, vs)` is untouched by `push`. We only reason
about the new node and the head pointer.

### Step 3: Show how disjoint ownership avoids global re-proofs

Without separation logic, updating the head of a list would require re-verifying
every property of every other list node. With the frame rule, the tail of the
list (`list(p, vs)`) is framed out and stays valid automatically.

```
    Without frame rule:
    Update head → re-verify entire list → O(n) proof work per operation
    
    With frame rule:
    Update head → frame out tail → O(1) proof work per operation
```

This is why separation logic scales to real programs: proof effort is proportional
to the code being verified, not the entire program state.

## Use It

Separation logic powers several major verification tools and influenced modern
language design:

- **Verified Software Toolchain (VST):** Proves C programs correct using
  separation logic. Used to verify parts of the CompCert compiler.
- **Facebook Infer:** Uses separation logic (specifically bi-abduction) to
  find memory bugs in C, C++, Java, and Objective-C. Runs on every code
  change at Facebook/Meta.
- **Rust's borrow checker:** Implements ownership and borrowing rules that
  embody separation logic principles without requiring formal proofs.
- **seL4:** The verified microkernel uses separation logic-style reasoning
  for its correctness proofs.

The key insight: separation logic makes *local reasoning* about heap-manipulating
code possible. You can verify a function by looking at only the memory it
accesses, not the entire program state.

## Read the Source

- John Reynolds, "Separation Logic: A Logic for Shared Mutable Data Structures"
  (2002) — the foundational paper.
- Peter O'Hearn, "Separation Logic" (CACM 2019) — accessible survey.
- [Verified Software Toolchain](https://vst.cs.princeton.edu/) — practical
  formal verification using separation logic.
- [Facebook Infer](https://fbinfer.com/) — industrial static analysis based
  on separation logic.

## Ship It

This lesson ships:

- `code/notes.md`: compact cheat sheet of predicates and frame rule workflow.
- `outputs/README.md`: local-reasoning checklist for pointer-manipulating code.

## Quiz

**Pre-questions:**

**Q1.** What does `P * Q` mean in separation logic?

- A) Both P and Q hold in the same heap.
- B) The heap can be split into two disjoint parts, one satisfying P and one Q.
- C) P implies Q.
- D) P or Q holds.

**Answer: B.** The separating conjunction `*` asserts that the heap can be
partitioned into two non-overlapping regions: one satisfying `P` and the other
satisfying `Q`. This is stronger than classical `∧` because it requires
disjointness.

**Q2.** Why does the frame rule matter for scalability?

- A) It makes proofs shorter.
- B) It lets you reason about code locally, ignoring unchanged memory.
- C) It eliminates the need for invariants.
- D) It works only for functional programs.

**Answer: B.** The frame rule says: if `{P} C {Q}`, then `{P * R} C {Q * R}`
(provided C doesn't touch R's locations). This means you verify `C` by looking
only at the memory it accesses (`P`). Everything else (`R`) stays valid
without re-proof. This is what makes separation logic scale to real programs.

**Post-questions:**

**Q3.** You write `x ↦ 5 * y ↦ 10`. Later you discover `x == y`. What
happens?

- A) The assertion is satisfied with x = y = 5.
- B) The assertion is unsatisfiable; it requires x and y to be different locations.
- C) The assertion is satisfied with x = y = 10.
- D) The assertion is satisfied with x = y = 7.5 (average).

**Answer: B.** The separating conjunction `*` requires disjoint heap regions.
If `x == y`, you can't split the heap into two parts with different values at
the same location. The assertion is unsatisfiable, which reveals the aliasing
problem early.

**Q4.** How does Rust's borrow checker relate to separation logic?

- A) Rust uses formal separation logic proofs at compile time.
- B) Rust's ownership model enforces similar invariants: one mutable reference
   at a time, preventing aliasing under mutation.
- C) Rust has no connection to separation logic.
- D) Rust uses separation logic only for unsafe code.

**Answer: B.** Rust's borrow checker enforces that mutable references are
exclusive (no aliasing) and shared references prevent mutation. This mirrors
separation logic's disjoint ownership without requiring formal proofs. The
ideas are the same; the mechanism is syntactic (compile-time checking) rather
than semantic (proof-based).

**Q5.** What is the "aliasing problem" that separation logic solves?

- A) Two variables having the same value.
- B) Two pointers to the same memory, making local reasoning about one
   invalidate assumptions about the other.
- C) Two functions with the same name.
- D) Two threads accessing the same CPU cache line.

**Answer: B.** When two pointers alias (point to the same memory), updating
one can silently invalidate assumptions about the other. Traditional logic
can't express "these pointers are disjoint." Separation logic's `*` connective
makes disjointness explicit, enabling local reasoning even in the presence of
mutable heap data.

## Exercises

**Easy:** Specify a push operation on a singly linked stack using separation
logic. Write the precondition, the code, and the postcondition using `*` and
`↦` assertions.

**Medium:** Show why aliasing breaks a disjointness assumption. Write a
`swap(int *x, int *y)` function. Give a separation logic spec that works when
`x ≠ y`. Show what goes wrong when `x == y` and how the spec catches it.

**Hard:** Encode ownership transfer in an API contract. Design a
`transfer_ownership(Node *src, Node **dst)` function that moves a list node
from one pointer to another. Write the separation logic spec showing that
ownership of the node transfers from `src`'s region to `dst`'s region.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Separating conjunction | "and" | `P * Q`: heap splits into disjoint parts satisfying P and Q |
| Frame rule | "ignore unchanged parts" | Preserve untouched owned assertions during local reasoning |
| Ownership | "who can mutate" | Rights over specific memory region in proof state |
| Footprint | "used memory" | Heap region a command actually accesses |
| Points-to | "cell assertion" | `l ↦ v`: location l holds value v in the heap |
| Bi-abduction | "guess the frame" | Technique used by Infer to automatically discover preconditions and frames |
| Linear logic | "use exactly once" | Logic where propositions are resources that must be consumed exactly once |

## Further Reading

- [Separation Logic (CACM)](https://cacm.acm.org/research/separation-logic/) — Peter O'Hearn's accessible survey.
- [VST](https://vst.cs.princeton.edu/) — Verified Software Toolchain for C program verification.
- [Facebook Infer](https://fbinfer.com/) — industrial static analysis based on separation logic.
- [Separation Logic Primer](https://www.cs.cmu.edu/~jcr/seplogic.pdf) — concise introduction by Reynolds.
- [Rust and Separation Logic](https://blog.rust-lang.org/2015/04/10/Fearless-Concurrency.html) — how Rust's ownership model relates to separation logic ideas.

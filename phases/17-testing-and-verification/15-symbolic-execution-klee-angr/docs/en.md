# Symbolic Execution - KLEE, angr

> Replace concrete inputs with symbolic values to explore path logic systematically.

**Type:** Learn
**Languages:** Python, C
**Prerequisites:** Phase 17 lessons 01-14
**Time:** ~75 minutes

## Learning Objectives

- Explain path constraints and solver-backed path exploration.
- Contrast symbolic execution with fuzzing and unit testing.
- Understand path explosion and mitigation strategies.
- Build a mini symbolic-style checker for branch reachability intuition.

## The Problem

A parser has nested branches and hidden corner conditions. Hand tests miss rare
combinations. Fuzzing might eventually find them, but can struggle with deep
path guards. Symbolic execution can derive constraints for reaching branches
directly.

Consider this code:

```c
int process(int x, int y) {
    if (x > 0) {
        if (y > 0) {
            if (x + y == 100) {
                if (x * y == 2100) {
                    // Bug here: only reachable with x=30, y=70 or x=70, y=30
                    crash();
                }
            }
        }
    }
    return 0;
}
```

A fuzzer would need to randomly guess `x=30, y=70` to reach the crash. The
probability of hitting this with random inputs is tiny. Symbolic execution
treats `x` and `y` as symbols, collects path constraints, and uses a solver
to derive `x=30, y=70` directly.

## The Concept

### Concrete vs Symbolic Execution

```
    Concrete Execution:              Symbolic Execution:
    
    Input: x=5, y=10                Input: x=α, y=β (symbols)
    
    if (x > 0) {                    if (α > 0) {
        // true, x=5                   // constraint: α > 0
        if (y > 0) {                  if (β > 0) {
            // true, y=10              // constraint: β > 0
            if (x+y == 100) {         if (α+β == 100) {
                // false, 5+10≠100     // constraint: α+β = 100
                // PATH STOPS          if (α*β == 2100) {
            }                             // constraint: α*β = 2100
        }                                 // Solver: α=30, β=70 ✓
    }                                 }
    return 0;                         }
```

Concrete execution follows one path. Symbolic execution explores *all* paths
by treating inputs as symbols and collecting constraints at each branch.

### The Symbolic Execution Loop

1. **Treat inputs as symbolic variables.** Instead of concrete values, `x` and
   `y` are algebraic symbols.

2. **Execute the program.** At each operation, compute with symbols. `x + y`
   becomes `α + β`.

3. **Fork on branches.** At `if (x > 0)`, fork into two states: one with
   constraint `α > 0` (true branch) and one with `α ≤ 0` (false branch).

4. **Ask the solver.** For each path, ask Z3 (or another SMT solver) whether
   the path constraints are satisfiable. If SAT, the solver produces a concrete
   input that reaches that path.

5. **Replay with concrete inputs.** Use the solver's model to run the program
   with concrete values, confirming the behavior.

```
    Symbolic Execution Tree:
    
                    Start
                   /     \
            α > 0         α ≤ 0
           /     \            |
      β > 0      β ≤ 0    return 0
      /    \        |
  α+β=100  α+β≠100 return 0
    /   \
α*β=2100 α*β≠2100
   |        |
crash()  return 0
    
    Solver finds: α=30, β=70 for the crash path
```

### Path Explosion

The number of paths grows exponentially with the number of branches. A function
with 20 `if` statements has up to 2^20 ≈ 1 million paths. This is the **path
explosion problem**.

Mitigation strategies:

- **Bounded exploration:** Limit path depth or number of explored paths.
- **Path prioritization:** Explore likely-buggy paths first (e.g., error
  handling paths).
- **State merging:** Merge similar path states to reduce the tree.
- **Concolic execution:** Mix concrete and symbolic execution. Run with a
  concrete input, then symbolically negate one branch condition at a time.

### Environment Modeling

Symbolic execution tools must model the environment (file system, network,
system calls). This is hard because:

- System calls have complex semantics.
- External state is infinite.
- Concurrency introduces exponential interleavings.

KLEE handles this by providing models for common libc functions. angr handles
binary code and uses VEX IR for architecture-independent analysis.

## Build It

We build a mini symbolic execution engine in Python that explores branch paths
and generates witness inputs.

### Step 1: Define a branchy function

```python
def target(x: int, y: int) -> str:
    """A function with nested branches for symbolic exploration."""
    if x > 10:
        if y > 10:
            if x + y == 50:
                return "path_A"  # reachable: x=25, y=25
            else:
                return "path_B"
        else:
            return "path_C"
    else:
        if y < 0:
            return "path_D"  # reachable: x=0, y=-1
        else:
            return "path_E"
```

### Step 2: Build a symbolic explorer

```python
from z3 import *

def explore_symbolically():
    """Explore all paths through target() using Z3."""
    
    x = Int('x')
    y = Int('y')
    
    paths = []
    
    # Path A: x > 10 AND y > 10 AND x + y == 50
    s = Solver()
    s.add(x > 10, y > 10, x + y == 50)
    if s.check() == sat:
        m = s.model()
        paths.append(("path_A", m[x].as_long(), m[y].as_long()))
    
    # Path B: x > 10 AND y > 10 AND x + y != 50
    s = Solver()
    s.add(x > 10, y > 10, x + y != 50)
    if s.check() == sat:
        m = s.model()
        paths.append(("path_B", m[x].as_long(), m[y].as_long()))
    
    # Path C: x > 10 AND NOT (y > 10)
    s = Solver()
    s.add(x > 10, y <= 10)
    if s.check() == sat:
        m = s.model()
        paths.append(("path_C", m[x].as_long(), m[y].as_long()))
    
    # Path D: NOT (x > 10) AND y < 0
    s = Solver()
    s.add(x <= 10, y < 0)
    if s.check() == sat:
        m = s.model()
        paths.append(("path_D", m[x].as_long(), m[y].as_long()))
    
    # Path E: NOT (x > 10) AND NOT (y < 0)
    s = Solver()
    s.add(x <= 10, y >= 0)
    if s.check() == sat:
        m = s.model()
        paths.append(("path_E", m[x].as_long(), m[y].as_long()))
    
    return paths

paths = explore_symbolically()
print("Reachable paths and witness inputs:")
for path_name, x_val, y_val in paths:
    result = target(x_val, y_val)
    print(f"  {path_name}: x={x_val}, y={y_val} -> {result}")
```

### Step 3: Verify with concrete execution

```python
# Verify each witness input reaches the expected path
for path_name, x_val, y_val in paths:
    result = target(x_val, y_val)
    assert result == path_name, f"Expected {path_name}, got {result}"
    print(f"  Verified: target({x_val}, {y_val}) = {result}")
```

### Step 4: Show unreachable paths

```python
def target_with_dead_code(x: int) -> str:
    """Function with an unreachable branch."""
    if x > 0:
        if x < 0:  # Dead code: x can't be both > 0 and < 0
            return "unreachable"
        return "positive"
    return "non_positive"

# Try to reach the "unreachable" path
s = Solver()
x = Int('x')
s.add(x > 0, x < 0)
if s.check() == unsat:
    print("Confirmed: 'unreachable' path is dead code (UNSAT)")
else:
    print("Bug: path is reachable with", s.model())
```

## Use It

KLEE and angr are used for:

- **Finding assertion failures and crashes.** Symbolic execution generates
  inputs that trigger specific program paths, including error-handling paths
  that fuzzers miss.
- **Generating high-coverage test cases.** For each feasible path, the solver
  produces a concrete input. This gives path-complete test suites.
- **Reasoning about binary code.** angr works on compiled binaries, letting you
  analyze code without source access.
- **Security analysis.** Find inputs that reach vulnerable code paths (buffer
  overflows, format strings, etc.).

Production references:

- KLEE found bugs in GNU Coreutils, SQLite, and other C programs.
- angr is used in CTF competitions and binary analysis research.
- Microsoft's SAGE (now part of IntelliTest) uses concolic execution for test
  generation at scale.

## Read the Source

- [KLEE](https://klee.github.io/) — KLEE documentation and papers.
- [angr](https://angr.io/) — angr documentation and examples.
- [KLEE OSDI paper](https://www.usenix.org/legacy/event/osdi08/tech/full_papers/cadar/cadar.pdf) — the foundational KLEE paper.

## Ship It

This lesson ships:

- `code/main.py`: path-constraint exploration example with Z3.
- `code/main.c`: branchy target suitable for symbolic analysis demos.
- `outputs/README.md`: symbolic execution triage checklist.

```bash
pip install z3-solver
python code/main.py
```

## Quiz

**Pre-questions:**

**Q1.** How does symbolic execution differ from fuzzing?

- A) Symbolic execution is faster.
- B) Symbolic execution treats inputs as symbols and uses a solver to derive
   inputs for each path; fuzzing generates random or mutated inputs.
- C) Fuzzing is more thorough.
- D) They're the same technique.

**Answer: B.** Fuzzing generates concrete inputs (random or mutated) and
observes which paths they hit. Symbolic execution treats inputs as algebraic
symbols, collects path constraints, and uses an SMT solver to derive concrete
inputs that reach specific paths. Symbolic execution is more systematic but
suffers from path explosion.

**Q2.** What is a "path constraint"?

- A) A limit on how many paths the program can take.
- B) A conjunction of branch conditions that must be true to reach a specific
   program point.
- C) A performance constraint on execution time.
- D) A constraint on the number of variables.

**Answer: B.** A path constraint is a formula describing the conditions under
which a specific execution path is taken. At each branch, the condition (for
the true branch) or its negation (for the false branch) is added to the
constraint. The solver checks if the accumulated constraints are satisfiable.

**Post-questions:**

**Q3.** A function has 30 `if` statements. What's the theoretical maximum
number of paths?

- A) 30.
- B) 60.
- C) 2^30 ≈ 1 billion.
- D) 30^2.

**Answer: C.** Each `if` statement creates two branches (true/false). With 30
independent `if` statements, the maximum number of paths is 2^30, which is
about 1 billion. This is the path explosion problem. In practice, many paths
are infeasible (constraints are UNSAT), but the theoretical maximum is
exponential.

**Q4.** What is "concolic execution"?

- A) Concrete-only execution.
- B) A hybrid approach: run with a concrete input, then symbolically negate
   one branch at a time to explore nearby paths.
- C) Symbolic execution with no solver.
- D) Execution with concurrent threads.

**Answer: B.** Concolic (concrete + symbolic) execution runs the program with
a concrete input, collecting symbolic constraints along the path. It then
negates one constraint at a time and asks the solver for a new input that
takes a different branch. This avoids the full path explosion by exploring
paths one at a time.

**Q5.** Z3 returns UNSAT for a path constraint. What does this mean?

- A) The path has a bug.
- B) The path is unreachable: no concrete input can execute that path.
- C) Z3 can't solve the constraint.
- D) The path is always taken.

**Answer: B.** If the path constraints are UNSAT, no concrete input exists that
satisfies all the branch conditions needed to reach that program point. The
path is dead code. Symbolic execution can prove unreachability, which is useful
for finding dead code and verifying that error-handling paths are actually
reachable.

## Exercises

**Easy:** Add another guarded branch to the `target` function (e.g.,
`if x * y > 1000`). Derive a witness input that reaches the new branch.

**Medium:** Show one path made unreachable by contradictory constraints. Write
a function where one branch requires `x > 100` and `x < 50` simultaneously.
Use Z3 to prove the path is unreachable.

**Hard:** Compare symbolic-derived inputs with random fuzz outcomes. Write a
function with 5 nested branches. Run 10,000 random fuzz inputs and count how
many paths are reached. Then use symbolic execution to generate inputs for
all paths. Compare coverage.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Symbolic value | "unknown input" | Variable treated as algebraic symbol during execution |
| Path constraint | "branch formula" | Conjunction of branch decisions required to reach a path |
| Path explosion | "too many states" | Exponential growth of branch combinations |
| Witness input | "generated test" | Concrete assignment satisfying a path constraint |
| Concolic execution | "hybrid symbolic" | Mix of concrete and symbolic execution for scalability |
| Dead code | "unreachable" | Code that no concrete input can reach (provably UNSAT) |
| State merging | "combine paths" | Technique to reduce path explosion by merging similar states |

## Further Reading

- [KLEE](https://klee.github.io/) — documentation, papers, and tutorials.
- [angr](https://angr.io/) — documentation and examples for binary analysis.
- [KLEE OSDI paper](https://www.usenix.org/legacy/event/osdi08/tech/full_papers/cadar/cadar.pdf) — the foundational paper.
- [SAGE](https://www.microsoft.com/en-us/research/publication/sage-automatic-generation-of-high-coverage-tests-via-concolic-execution/) — Microsoft's concolic execution at scale.
- [Symbolic Execution Survey](https://www.cis.upenn.edu/~alur/Survey.pdf) — academic survey of techniques.

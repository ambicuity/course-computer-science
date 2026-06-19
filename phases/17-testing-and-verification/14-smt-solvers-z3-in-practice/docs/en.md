# SMT Solvers - Z3 in Practice

> Encode constraints; let the solver find the impossible assumptions.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 17 lessons 01-13
**Time:** ~75 minutes

## Learning Objectives

- Translate program constraints into SMT formulas.
- Use satisfiable/unsatisfiable results to validate assumptions.
- Extract models and unsat cores conceptually.
- Apply solver checks to input validation and safety properties.

## The Problem

Engineers often reason informally about constraints (ranges, ordering,
consistency) and miss combinations that are impossible or unsafe. A configuration
system has 5 parameters, each with a valid range. The ranges overlap in ways
that create 3 impossible combinations. Nobody notices until a customer hits one
in production.

A scheduling system must assign tasks to workers. Each task has a duration, a
deadline, and a skill requirement. Each worker has hours and skills. Manually
verifying that a valid schedule exists for all inputs is error-prone. An SMT
solver can check this in milliseconds.

SMT (Satisfiability Modulo Theories) solvers provide precise satisfiability
checks over arithmetic, logic, arrays, and bit-vectors. You encode your
constraints as a formula, and the solver tells you: SAT (here's a satisfying
assignment) or UNSAT (no assignment exists, and here's why).

## The Concept

### SMT Workflow

```
    SMT Solver Workflow:
    
    ┌──────────────────┐
    │ Declare variables│  x: Int, y: Int
    └────────┬─────────┘
             │
    ┌────────▼─────────┐
    │ Add constraints  │  x > 0, y > 0, x + y = 10
    └────────┬─────────┘
             │
    ┌────────▼─────────┐
    │ Ask: SAT/UNSAT?  │
    └────────┬─────────┘
             │
     ┌───────┴───────┐
     │               │
     ▼               ▼
    ┌─────┐      ┌──────┐
    │ SAT │      │ UNSAT│
    │     │      │      │
    │Model:│      │Core: │
    │x=3, │      │x > 0 │
    │y=7  │      │x < 0 │
    └─────┘      └──────┘
    
    SAT: here's a concrete assignment satisfying all constraints.
    UNSAT: no assignment exists; here's a minimal conflicting subset.
```

### Z3 Python API

Z3 is the most widely used SMT solver. Its Python API makes constraint
programming accessible:

```python
from z3 import *

# Declare symbolic variables
x = Int('x')
y = Int('y')

# Add constraints
s = Solver()
s.add(x > 0)
s.add(y > 0)
s.add(x + y == 10)

# Check satisfiability
if s.check() == sat:
    m = s.model()
    print(f"x = {m[x]}, y = {m[y]}")
    # Output: x = 3, y = 7 (or any other satisfying assignment)
```

### Theories

SMT solvers support multiple theories (mathematical domains):

| Theory | What it handles | Example |
|---|---|---|
| Linear integer arithmetic | `+`, `-`, `*` on integers | `x + y = 10` |
| Linear real arithmetic | `+`, `-`, `*` on reals | `x / 3.0 > 1.5` |
| Bit-vectors | Fixed-width bit operations | `x & 0xFF == 0x42` |
| Arrays | Read/write on arrays | `a[i] = 5` |
| Uninterpreted functions | Function equality | `f(x) = f(y)` |

Z3 combines theories automatically. You can mix integers, arrays, and
bit-vectors in a single formula.

### SAT vs UNSAT

**SAT:** The solver found a concrete assignment satisfying all constraints.
You can inspect the model to understand what values work.

**UNSAT:** No assignment exists. The solver can produce an **unsat core**: a
minimal subset of constraints that are mutually contradictory. This tells you
which assumptions conflict.

```python
from z3 import *

x = Int('x')
s = Solver()

s.add(x > 0, name="positive")
s.add(x < 0, name="negative")

if s.check() == unsat:
    print("Conflicting constraints:", s.unsat_core())
    # Output: [positive, negative]
```

### Optimization

Z3 can also optimize (minimize/maximize) objectives:

```python
from z3 import *

x = Int('x')
o = Optimize()
o.add(x >= 0)
o.add(x <= 100)
o.minimize(x)

if o.check() == sat:
    print(f"Minimum x = {o.model()[x]}")
    # Output: Minimum x = 0
```

## Build It

We model a scheduling constraint set and demonstrate SAT/UNSAT results.

### Step 1: Declare variables and constraints

```python
from z3 import *

def check_schedule():
    """Check if a valid schedule exists for 3 tasks and 2 workers."""
    
    # Task durations and deadlines
    tasks = {
        "t1": {"duration": 3, "deadline": 10, "skill": "python"},
        "t2": {"duration": 5, "deadline": 15, "skill": "rust"},
        "t3": {"duration": 2, "deadline": 8,  "skill": "python"},
    }
    
    # Worker availability and skills
    workers = {
        "w1": {"hours": 8, "skills": ["python", "rust"]},
        "w2": {"hours": 6, "skills": ["python"]},
    }
    
    # Decision variables: which worker does which task, and when
    assignment = {t: Int(f"assign_{t}") for t in tasks}  # worker id (1 or 2)
    start_time = {t: Int(f"start_{t}") for t in tasks}
    
    s = Solver()
    
    # Each task assigned to a valid worker
    for t in tasks:
        s.add(Or(assignment[t] == 1, assignment[t] == 2))
    
    # Worker skill constraints
    for t, info in tasks.items():
        if info["skill"] == "rust":
            s.add(assignment[t] == 1)  # Only w1 has rust skill
    
    # Worker hour constraints
    for w in [1, 2]:
        worker_tasks = [t for t in tasks if True]  # simplified
        total = Sum([If(assignment[t] == w, tasks[t]["duration"], 0) 
                     for t in tasks])
        s.add(total <= (8 if w == 1 else 6))
    
    # Deadline constraints
    for t, info in tasks.items():
        s.add(start_time[t] >= 0)
        s.add(start_time[t] + info["duration"] <= info["deadline"])
    
    # Non-overlapping tasks on same worker
    for t1 in tasks:
        for t2 in tasks:
            if t1 < t2:
                s.add(Implies(
                    assignment[t1] == assignment[t2],
                    Or(
                        start_time[t1] + tasks[t1]["duration"] <= start_time[t2],
                        start_time[t2] + tasks[t2]["duration"] <= start_time[t1]
                    )
                ))
    
    return s

s = check_schedule()
if s.check() == sat:
    m = s.model()
    print("Valid schedule found:")
    for v in m:
        print(f"  {v} = {m[v]}")
else:
    print("No valid schedule exists")
    print("Conflicting constraints:", s.unsat_core())
```

### Step 2: Demonstrate UNSAT with conflicting constraints

```python
from z3 import *

def impossible_config():
    """Show UNSAT with conflicting configuration constraints."""
    
    cpu_cores = Int('cpu_cores')
    memory_gb = Int('memory_gb')
    
    s = Solver()
    
    # System requirements
    s.add(cpu_cores >= 1, name="min_cores")
    s.add(memory_gb >= 1, name="min_memory")
    
    # Application constraints
    s.add(cpu_cores * 2 <= memory_gb, name="app_ratio")
    
    # Deployment constraint (too restrictive)
    s.add(memory_gb <= 2, name="deploy_limit")
    s.add(cpu_cores >= 4, name="app_requirement")
    
    if s.check() == unsat:
        print("Configuration impossible!")
        print("Conflicting constraints:", s.unsat_core())
        # Output: [app_ratio, deploy_limit, app_requirement]
    else:
        print("Valid configuration:", s.model())

impossible_config()
```

### Step 3: Auto-generate edge tests

```python
from z3 import *

def generate_edge_cases(func_constraints, num_cases=5):
    """Use Z3 to generate inputs at constraint boundaries."""
    
    x = Int('x')
    s = Solver()
    
    # Function constraints (e.g., 0 <= x <= 100)
    s.add(x >= 0)
    s.add(x <= 100)
    
    cases = []
    for _ in range(num_cases):
        if s.check() == sat:
            m = s.model()
            val = m[x].as_long()
            cases.append(val)
            # Block this value to find different ones
            s.add(x != val)
        else:
            break
    
    return cases

# Generate boundary-adjacent values
edge_cases = generate_edge_cases(None)
print("Edge cases:", edge_cases)
# Output might include: [0, 100, 50, 1, 99]
```

## Use It

In production, solvers are used for:

- **Test input generation:** Use Z3 to find inputs that exercise specific code
  paths or hit boundary conditions.
- **Static analysis:** Encode program properties as constraints and check for
  violations.
- **Configuration validation:** Verify that configuration parameters have at
  least one valid combination.
- **Scheduling and planning:** Check feasibility of task assignments, resource
  allocations, or deployment configurations.
- **Protocol verification:** Encode protocol state machines and check for
  reachable error states.

Production references:

- Microsoft's Z3 is used in Dafny, P, and SAW for program verification.
- Amazon uses Z3 for AWS IAM policy analysis.
- KLEE uses Z3 as its constraint solver for symbolic execution.

## Read the Source

- [Z3 Guide](https://microsoft.github.io/z3guide/) — comprehensive Z3
  tutorial with Python examples.
- [SMT-LIB](https://smtlib.cs.uiowa.edu/) — the standard language for SMT
  solvers.
- [Z3 GitHub](https://github.com/Z3Prover/z3) — source code and examples.

## Ship It

This lesson ships:

- `code/main.py`: scheduling constraint checker with SAT/UNSAT demos.
- `outputs/README.md`: SMT integration checklist.

```bash
pip install z3-solver
python code/main.py
```

## Quiz

**Pre-questions:**

**Q1.** What does "SAT" mean in the context of SMT solvers?

- A) The formula is always true.
- B) There exists at least one assignment of values to variables that satisfies
   all constraints.
- C) The solver is satisfied with the result.
- D) The formula is satisfiable for all inputs.

**Answer: B.** SAT means the solver found a concrete assignment of values to
variables that makes all constraints true simultaneously. It doesn't mean the
formula is always true (that would be "valid"). It means at least one satisfying
assignment exists.

**Q2.** What is an "unsat core"?

- A) The hardest constraint to satisfy.
- B) A minimal subset of constraints that are mutually contradictory.
- C) The solver's internal representation of the problem.
- D) The set of all constraints.

**Answer: B.** When a formula is UNSAT, the unsat core is a minimal set of
constraints that together are contradictory. Removing any one of them would make
the formula satisfiable. This tells you exactly which assumptions conflict.

**Post-questions:**

**Q3.** You encode `x > 0` and `x < 0` as constraints. Z3 returns UNSAT. What
does this mean?

- A) Z3 has a bug.
- B) No integer value of x satisfies both constraints simultaneously.
- C) x must be zero.
- D) The constraints are too complex.

**Answer: B.** `x > 0` requires x to be positive. `x < 0` requires x to be
negative. No value can be both positive and negative. The formula is
unsatisfiable, meaning no assignment exists.

**Q4.** How can Z3 help with test generation?

- A) Z3 writes tests automatically.
- B) Z3 finds inputs that satisfy specific path constraints, generating
   test cases that exercise particular code paths.
- C) Z3 replaces the need for testing.
- D) Z3 only works with integer inputs.

**Answer: B.** You encode the conditions for reaching a specific code path as
constraints. Z3 finds a satisfying assignment (an input that takes that path).
This generates targeted test cases without manual reasoning about which inputs
trigger which branches.

**Q5.** What's the difference between SAT and SMT?

- A) SAT is for booleans; SMT extends SAT with theories (arithmetic, arrays,
   bit-vectors, etc.).
- B) SAT is faster; SMT is more accurate.
- C) They're the same thing.
- D) SAT is for satisfiability; SMT is for optimization.

**Answer: A.** SAT (Boolean Satisfiability) works only with boolean variables
and logical connectives. SMT (Satisfiability Modulo Theories) extends SAT with
theories like integer arithmetic, arrays, and bit-vectors. SMT solvers use SAT
solvers internally but add theory-specific reasoning.

## Exercises

**Easy:** Add resource capacity constraints to the scheduling model. Each
worker has a maximum of 8 hours. Check if a valid schedule exists when you add
a 4th task that requires 3 hours of Rust work.

**Medium:** Add objective minimization using Z3's `Optimize` module. Minimize
the total completion time (makespan) of all tasks. Compare the optimal schedule
with the feasible one.

**Hard:** Use Z3 to auto-generate edge test cases for a function with multiple
constraints. Write a function `classify(x, y)` with several branches. Encode
each branch's path condition as a Z3 constraint. Use the solver to generate
inputs that reach each branch, including hard-to-reach ones.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| SAT | "possible" | Constraints have at least one satisfying assignment |
| UNSAT | "contradictory" | No assignment satisfies all constraints |
| Model | "answer" | Concrete satisfying assignment from solver |
| Theory | "math domain" | Supported logic domain (ints, arrays, bit-vectors, etc.) |
| Unsat core | "conflict set" | Minimal subset of mutually contradictory constraints |
| SMT | "SAT with theories" | Satisfiability Modulo Theories: SAT extended with mathematical domains |
| Optimize | "minimize/maximize" | Find the best satisfying assignment according to an objective |

## Further Reading

- [Z3 Guide](https://microsoft.github.io/z3guide/) — comprehensive tutorial
  with Python examples.
- [SMT-LIB](https://smtlib.cs.uiowa.edu/) — standard language for SMT solvers.
- [Z3 GitHub](https://github.com/Z3Prover/z3) — source and examples.
- [Decision Procedures](https://www.decision-procedures.org/) — textbook on
  SAT/SMT theory.
- [Dafny](https://dafny.org/) — programming language that uses Z3 for
  verification.

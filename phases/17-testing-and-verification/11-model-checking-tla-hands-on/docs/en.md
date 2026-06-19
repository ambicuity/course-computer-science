# Model Checking - TLA+ Hands-On

> Test examples; model-check state spaces.

**Type:** Learn
**Languages:** TLA+
**Prerequisites:** Phase 17 lessons 01-10
**Time:** ~120 minutes

## Learning Objectives

- Build a small TLA+ state-machine model and invariants.
- Distinguish safety and liveness properties.
- Use bounded model checking to find counterexample traces.
- Translate counterexample traces into engineering fixes.

## The Problem

Distributed bugs often require rare interleavings no unit suite covers. Model
checking explores all transitions of a bounded abstraction and finds traces that
violate invariants. This is one of the fastest ways to discover protocol design
flaws before implementation.

Consider a simple lock service. Two clients request a shared lock. You write a
test where client A gets the lock, releases it, then client B gets it. Test
passes. But what if both clients request the lock at the exact same time? What
if a release message is delayed? What if a client crashes while holding the
lock? Your unit test covers one interleaving. The model checker covers *all*
interleavings in a bounded state space.

Amazon uses TLA+ to verify DynamoDB, S3, EBS, and the AWS Lambda runtime. They
found bugs in every protocol they modeled, bugs that survived years of testing
and code review. The investment paid for itself on the first model.

## The Concept

### State Machines in TLA+

TLA+ models systems as state machines. A state is an assignment of values to
variables. A transition is a relation between old and new states.

```
    State Machine:
    
    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
    │ lock = ""    │────▶│ lock = "A"  │────▶│ lock = ""    │
    │ A = "idle"   │     │ A = "held"  │     │ A = "idle"   │
    │ B = "idle"   │     │ B = "idle"  │     │ B = "idle"   │
    └─────────────┘     └─────────────┘     └─────────────┘
         │                                        │
         │              ┌─────────────┐           │
         └─────────────▶│ lock = "B"  │◀──────────┘
                        │ A = "idle"  │
                        │ B = "held"  │
                        └─────────────┘
    
    Each arrow is a transition. The model checker explores ALL paths.
```

### Model Ingredients

A TLA+ specification has:

1. **Variables:** The state components (e.g., `lock`, `pc_A`, `pc_B`).
2. **Init:** The initial state predicate (what values variables start with).
3. **Next:** The next-state relation (what transitions are allowed).
4. **Safety invariant:** A predicate that must hold in every reachable state.
5. **Optional liveness:** Properties about infinite behaviors (eventual progress).

### Safety vs Liveness

**Safety:** "Nothing bad happens." Formally: a predicate true in all reachable
states. Example: "at most one client holds the lock."

**Liveness:** "Something good eventually happens." Formally: a property over
infinite behaviors. Example: "every client that requests the lock eventually
gets it."

Safety violations produce finite counterexample traces. Liveness violations
require infinite stuttering traces (the system gets stuck forever).

### The Lock Protocol

We model a simple mutual exclusion protocol:

```
    Client A                    Lock Service                 Client B
    ────────                    ────────────                 ────────
    Request(lock) ──────────▶  lock := "A"               ◀── Request(lock)
                                │                           │
    Acquired(lock) ◀───────────┘                           │
                                │                           │
    [critical section]          │                           [waiting]
                                │                           │
    Release(lock) ──────────▶  lock := "" ──────────────▶ Acquired(lock)
                                │                           │
                                │                      [critical section]
                                │                           │
                                ◀───────────────────── Release(lock)
```

### TLA+ Specification

```tla
----------------------------- MODULE Lock -----------------------------
EXTENDS Naturals

CONSTANTS Clients

VARIABLES lock, state

TypeOK ==
    /\ lock \in Clients \cup {""}
    /\ state \in [Clients -> {"idle", "waiting", "held"}]

Init ==
    /\ lock = ""
    /\ state = [c \in Clients |-> "idle"]

Request(c) ==
    /\ state[c] = "idle"
    /\ state' = [state EXCEPT ![c] = "waiting"]
    /\ UNCHANGED lock

Acquire(c) ==
    /\ state[c] = "waiting"
    /\ lock = ""
    /\ lock' = c
    /\ state' = [state EXCEPT ![c] = "held"]

Release(c) ==
    /\ state[c] = "held"
    /\ lock' = ""
    /\ state' = [state EXCEPT ![c] = "idle"]

Next ==
    \E c \in Clients:
        \/ Request(c)
        \/ Acquire(c)
        \/ Release(c)

Spec == Init /\ [][Next]_<<lock, state>>

\* Safety: at most one client holds the lock
MutualExclusion ==
    \A c1, c2 \in Clients:
        (state[c1] = "held" /\ state[c2] = "held") => c1 = c2

\* Safety: lock matches holder
LockConsistency ==
    (lock /= "") => state[lock] = "held"
====================================================================
```

### TLC Configuration

```tla
SPECIFICATION Spec
CONSTANTS Clients = {"A", "B"}
INVARIANT MutualExclusion LockConsistency TypeOK
```

TLC explores all reachable states. With 2 clients and 3 states each plus lock,
the state space is small (~20 states). TLC checks every one.

## Build It

### Step 1: Write the TLA+ specification

Create `code/Lock.tla` with the specification above. The key elements:

- `Clients` is a constant set (parameterized, so you can test with 2, 3, or more).
- `lock` tracks who holds the lock (or "" if free).
- `state` tracks each client's local state.
- `Next` defines all possible transitions.
- `MutualExclusion` is the safety invariant.

### Step 2: Run TLC

```bash
# Install TLA+ Toolbox or use the VS Code extension
# Run TLC with the config file
tlc Lock.tla
```

TLC reports: "Invariant MutualExclusion is violated." It found a
counterexample trace.

### Step 3: Inspect the counterexample

TLC produces a trace like:

```
State 1: Init
  lock = ""
  state = [A |-> "idle", B |-> "idle"]

State 2: Request(A)
  lock = ""
  state = [A |-> "waiting", B |-> "idle"]

State 3: Acquire(A)
  lock = "A"
  state = [A |-> "held", B |-> "idle"]

State 4: Request(B)
  lock = "A"
  state = [A |-> "held", B |-> "waiting"]

State 5: Release(A)
  lock = ""
  state = [A |-> "idle", B |-> "waiting"]

State 6: Acquire(B)
  lock = "B"
  state = [A |-> "idle", B |-> "held"]
```

This trace shows the protocol working correctly. If we had a bug (e.g.,
allowing `Acquire` when lock is not ""), TLC would show a trace where both
clients end up in "held" state.

### Step 4: Introduce a bug and re-check

Modify `Acquire` to not check `lock = ""`:

```tla
AcquireBug(c) ==
    /\ state[c] = "waiting"
    \* Missing: /\ lock = ""
    /\ lock' = c
    /\ state' = [state EXCEPT ![c] = "held"]
```

TLC now finds a trace where both A and B are in "held" state simultaneously.
The counterexample shows exactly which interleaving triggers the bug.

## Use It

Practical approach for using TLA+ in engineering:

1. **Model before code** for complex concurrency/distributed flows. The model
   is 50 lines; the implementation is 5000. Bugs in the model cost hours;
   bugs in the code cost weeks.

2. **Keep the model intentionally small and abstract.** Don't model network
   byte formats or serialization. Model the state transitions and invariants.

3. **Iterate:** Failing trace → design/spec fix → re-check. Each cycle takes
   minutes, not days.

4. **Check in the model as documentation.** The TLA+ spec is a precise,
   machine-checkable description of the protocol's intended behavior.

Production references:

- Amazon's use of TLA+ on DynamoDB, S3, and EBS.
- Microsoft's use of TLA+ on Azure Cosmos DB.
- The Raft consensus protocol has a TLA+ spec that found bugs during design.

## Read the Source

- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) — Lamport's canonical learning resource.
- [Learn TLA+](https://learntla.com/) — practical tutorial flow with examples.
- [Specifying Systems](https://lamport.azurewebsites.net/tla/book.html) — Lamport's TLA+ textbook (free PDF).

## Ship It

This lesson ships:

- `code/Lock.tla`: lock model with mutual exclusion invariant.
- `code/Lock.cfg`: TLC configuration file.
- `outputs/README.md`: TLC run checklist and trace triage flow.

```bash
# Run the model checker
tlc code/Lock.tla -config code/Lock.cfg
```

## Quiz

**Pre-questions:**

**Q1.** What does a model checker do that a unit test doesn't?

- A) Runs faster.
- B) Explores all possible state transitions in a bounded model.
- C) Generates random inputs.
- D) Proves the code is correct.

**Answer: B.** A unit test checks one specific execution path. A model checker
explores *all* reachable states in a bounded abstraction. If a bug requires a
rare interleaving of events, the model checker finds it by exhaustive search,
while unit tests only find it if you happen to write the right test case.

**Q2.** What is a safety property in TLA+?

- A) A property that guarantees termination.
- B) A predicate that must hold in every reachable state.
- C) A property about infinite behaviors.
- D) A performance guarantee.

**Answer: B.** Safety properties assert that "nothing bad happens." They're
predicates over states: "at every point in time, this holds." Mutual exclusion
is a safety property: at no point do two clients hold the lock simultaneously.

**Post-questions:**

**Q3.** TLC reports a counterexample trace of 6 states. What should you do?

- A) Delete the invariant that was violated.
- B) Read the trace to understand which interleaving causes the bug, then fix
   the protocol or spec.
- C) Increase the state space bound and re-run.
- D) Add more invariants to prevent the trace.

**Answer: B.** A counterexample trace is a *feature*: it shows exactly which
sequence of transitions leads to the violation. Read it, understand the
interleaving, then decide whether the bug is in the protocol (fix the design)
or in the spec (fix the model). Don't delete invariants to make the model pass.

**Q4.** Why does TLA+ use a "state-machine" model rather than pseudocode?

- A) State machines are easier to read.
- B) State machines make all nondeterminism explicit, which is essential for
   exploring all interleavings.
- C) Pseudocode can't express invariants.
- D) TLA+ doesn't support pseudocode.

**Answer: B.** In a state machine, every nondeterministic choice (e.g., which
client acts next) is explicit in the `Next` relation. This lets TLC explore
all possible interleavings. Pseudocode hides nondeterminism behind control
flow, making it impossible to systematically explore all execution orders.

**Q5.** You model a protocol with 3 clients and TLC checks 500 states. You add
a 4th client and TLC checks 50,000 states. What's happening?

- A) TLC has a bug.
- B) The state space grows exponentially with the number of clients.
   This is the state explosion problem.
- C) You need a faster computer.
- D) The protocol is wrong.

**Answer: B.** Model checking suffers from state explosion: the number of
reachable states grows exponentially with the number of components. With 3
clients × 3 states each × lock values, you get ~27 states. With 4 clients,
~256. This is why model checking uses *bounded* analysis: you check small
instances thoroughly, trusting that protocol bugs manifest in small cases.

## Exercises

**Easy:** Add a timeout transition to the lock model. If a client is in
"waiting" state for too long, it transitions back to "idle" (gives up). Check
whether the protocol still satisfies mutual exclusion.

**Medium:** Add a queue discipline. Instead of allowing any waiting client to
acquire the lock, require FIFO ordering. Define an invariant that the queue is
always in order. Check that mutual exclusion and fairness both hold.

**Hard:** Model a two-phase commit sketch. Define coordinator and participant
states. Check the invariant that all participants agree (either all commit or
all abort). Add a crash transition for the coordinator and see what invariants
break.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Safety | "nothing bad happens" | Invariant true in all reachable states |
| Liveness | "something good eventually happens" | Progress property over infinite behaviors |
| Counterexample trace | "bug replay" | Sequence of transitions violating a property |
| Abstraction | "simplification" | Reduced state model preserving relevant behavior |
| State explosion | "too many states" | Exponential growth of reachable states with model size |
| Model checking | "exhaustive search" | Algorithm that explores all reachable states of a finite model |
| TLC | "the checker" | The TLA+ model checker that evaluates specifications |
| Stuttering | "doing nothing" | A step where all variables remain unchanged; allows infinite "no progress" |

## Further Reading

- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) — canonical learning resource by Lamport.
- [Learn TLA+](https://learntla.com/) — practical tutorial with examples.
- [Specifying Systems](https://lamport.azurewebsites.net/tla/book.html) — Lamport's TLA+ textbook.
- [Amazon's use of TLA+](https://www.amazon.science/publications/how-amazon-web-services-uses-formal-methods) — real-world impact stories.
- [TLA+ VS Code Extension](https://marketplace.visualstudio.com/items?itemName=alygin.vscode-tlaplus) — IDE support for writing and checking specs.

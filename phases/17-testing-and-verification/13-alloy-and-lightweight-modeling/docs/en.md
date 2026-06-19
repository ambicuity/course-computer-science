# Alloy and Lightweight Modeling

> Small models expose big design contradictions.

**Type:** Learn
**Languages:** Alloy
**Prerequisites:** Phase 17 lessons 01-12
**Time:** ~75 minutes

## Learning Objectives

- Build relational models and constraints in Alloy.
- Check consistency and find counterexample instances quickly.
- Encode acyclic and uniqueness constraints.
- Use bounded analysis as design sanity check.

## The Problem

Many design documents describe relationships informally and miss contradictions.
A microservices architecture says "Service A depends on Service B" and "Service
B depends on Service A." That's a cycle, but nobody notices until a deployment
deadlock in production.

A database schema says "every order has exactly one customer" and "every customer
has at most one active order." These constraints sound reasonable separately but
together they prevent a customer from having two orders, which is probably wrong.

Alloy lets you formalize these relationships and automatically search for
counterexamples in small scopes. It's not a theorem prover; it's a model finder.
You describe what *must* be true, and Alloy either shows you a valid instance
(where all constraints hold) or a counterexample (where some constraint is
violated). This catches design contradictions before you write code.

## The Concept

### Alloy Is Relational and Bounded

Alloy models the world in terms of **signatures** (sets of atoms), **fields**
(relations between atoms), and **facts** (constraints that must hold).

```
    Alloy Model Structure:
    
    ┌─────────────────────────────────────┐
    │  sig Node {                         │  "Node" is a set of atoms
    │    edges: set Node                  │  "edges" is a relation: Node → Node
    │  }                                  │
    ├─────────────────────────────────────┤
    │  fact no_self_loops {               │  Constraint: no node has an edge
    │    no n: Node | n in n.edges        │  to itself
    │  }                                  │
    ├─────────────────────────────────────┤
    │  assert acyclic {                   │  Property to check: the graph
    │    no n: Node | n in n.^edges       │  has no cycles (^ = transitive closure)
    │  }                                  │
    ├─────────────────────────────────────┤
    │  check acyclic for 5               │  Check for up to 5 atoms
    └─────────────────────────────────────┘
```

Key Alloy features:

- **Signatures (`sig`):** Define sets of atoms. `sig Node {}` creates a set
  of Node atoms.
- **Fields:** Relations between signatures. `edges: set Node` is a binary
  relation from Node to Node.
- **Facts (`fact`):** Constraints that must hold in every instance.
- **Assertions (`assert`):** Properties to check by searching for counterexamples.
- **Scope (`for N`):** Bounds the search to at most N atoms per signature.

### Bounded vs Unbounded

Alloy's bounded analysis is powerful but not a proof. It checks all instances
up to a given scope. If you check `for 5`, Alloy examines every possible
arrangement of up to 5 atoms per signature. If no counterexample exists in
that scope, the assertion *might* hold in general, or a counterexample might
need 6 atoms.

This is a pragmatic trade-off: Alloy finds bugs fast (seconds for small scopes)
but can't prove universal properties. For design-level sanity checks, this is
usually enough. Most design bugs manifest in small instances.

### The Analyzer

The Alloy Analyzer is a SAT-based model finder. It translates your model into
a boolean satisfiability problem and uses a SAT solver to find instances. This
is fast: even with hundreds of atoms, the analyzer returns results in seconds.

```
    Alloy Workflow:
    
    ┌──────────┐     ┌──────────┐     ┌──────────┐
    │  Write   │────▶│  Alloy   │────▶│  SAT     │
    │  Model   │     │  translates│    │  Solver  │
    └──────────┘     └──────────┘     └────┬─────┘
                                           │
                    ┌──────────┐           │
                    │  Visual  │◀──────────┘
                    │  Instance│  (valid or counterexample)
                    └──────────┘
```

### Common Constraint Patterns

**Acyclicity:** No node can reach itself through edges.
```alloy
fact acyclic {
    no n: Node | n in n.^edges
}
```

**Uniqueness:** Each element has exactly one parent.
```alloy
fact unique_parent {
    all n: Node | one n.parent
}
```

**Totality:** Every pair of nodes is related.
```alloy
fact total {
    all n1, n2: Node | n1 -> n2 in edges or n2 -> n1 in edges
}
```

**Cardinality:** At most N elements in a set.
```alloy
fact at_most_three_children {
    all n: Node | #(n.children) <= 3
}
```

## Build It

We model a service dependency graph with constraints that catch common
architectural mistakes.

### Step 1: Define the model

```alloy
module ServiceGraph

sig Service {
    depends_on: set Service,
    owned_by: one Team
}

sig Team {}

// No service depends on itself
fact no_self_dependency {
    no s: Service | s in s.depends_on
}

// Dependency graph is acyclic
fact acyclic {
    no s: Service | s in s.^depends_on
}

// Each team owns at least one service
fact team_owns_service {
    all t: Team | some s: Service | s.owned_by = t
}

// Exactly one gateway service (no incoming dependencies)
one sig Gateway extends Service {}

fact gateway_no_incoming {
    no s: Service | Gateway in s.depends_on
}

// Gateway depends on other services (it's the entry point)
fact gateway_has_outgoing {
    some Gateway.depends_on
}
```

### Step 2: Define assertions

```alloy
// Check: no circular dependencies
assert no_cycles {
    no s: Service | s in s.^depends_on
}

// Check: every service is reachable from gateway
assert all_reachable {
    all s: Service | s in Gateway.*depends_on
}

// Check: no team owns two services that depend on each other
assert no_circular_ownership {
    no s1, s2: Service |
        s1.owned_by = s2.owned_by and
        s1 in s2.depends_on and
        s2 in s1.depends_on
}

// Run the checks
check no_cycles for 8
check all_reachable for 8
check no_circular_ownership for 8
```

### Step 3: Run the analyzer

```bash
# Using Alloy Analyzer GUI or CLI
alloy ServiceGraph.als
```

Alloy either finds a counterexample (showing a concrete instance where the
assertion fails) or reports that no counterexample exists in the given scope.

### Step 4: Interpret results

If `all_reachable` fails, Alloy shows an instance with a service that isn't
reachable from the Gateway. This reveals an architectural gap: a service exists
but has no dependency path from the entry point.

If `no_circular_ownership` fails, Alloy shows two services owned by the same
team with a circular dependency. This is a design smell: the team is creating
tightly coupled components.

## Use It

Use Alloy for architecture and schema constraints before implementation:

- **Microservices dependency graphs:** Check for cycles, unreachable services,
  and ownership conflicts.
- **Database schemas:** Check cardinality constraints, foreign key consistency,
  and normalization violations.
- **API contracts:** Check that request/response schemas are consistent across
  versions.
- **Access control:** Check that permission assignments don't create privilege
  escalation paths.

When assertions fail, Alloy gives a concrete instance demonstrating the flaw.
This is much more useful than a text description: you can visualize the exact
configuration that breaks your design.

Production references:

- Daniel Jackson's book "Software Abstractions" uses Alloy extensively for
  software design.
- Microsoft used Alloy to verify aspects of the Xbox Live architecture.
- Alloy has been used to find bugs in network configurations, file systems,
  and access control policies.

## Read the Source

- [Alloy Tools](https://alloytools.org/) — download and documentation.
- [Software Abstractions](https://mitpress.mit.edu/9780262526545/software-abstractions-revised-edition/) — Daniel Jackson's book on Alloy.
- [Alloy documentation](https://alloytools.org/documentation.html) — language reference and examples.

## Ship It

This lesson ships:

- `code/ServiceGraph.als`: service dependency model with checks.
- `outputs/README.md`: lightweight modeling checklist.

```bash
alloy code/ServiceGraph.als
```

## Quiz

**Pre-questions:**

**Q1.** What does Alloy do when an assertion fails?

- A) Proves the assertion is false for all possible instances.
- B) Shows a concrete counterexample instance where the assertion is violated.
- C) Reports an error without details.
- D) Fixes the model automatically.

**Answer: B.** Alloy is a model finder, not a theorem prover. When an assertion
fails, it produces a concrete instance (a specific arrangement of atoms and
relations) that demonstrates the violation. You can visualize this instance to
understand exactly what goes wrong.

**Q2.** What does "bounded analysis" mean in Alloy?

- A) Alloy can only analyze small programs.
- B) Alloy checks all instances up to a given scope (number of atoms per
   signature), but can't prove universal properties.
- C) Alloy requires bounded memory.
- D) Alloy only works with bounded loops.

**Answer: B.** Bounded analysis means Alloy exhaustively checks all possible
instances within a scope limit (e.g., `for 5` means up to 5 atoms per
signature). If no counterexample exists in that scope, the property *might*
hold in general, but Alloy can't guarantee it. This is a pragmatic trade-off:
fast bug-finding at the cost of completeness.

**Post-questions:**

**Q3.** You model a dependency graph with `fact acyclic { no n: Node | n in
n.^edges }`. What does `^edges` mean?

- A) The complement of edges.
- B) The transitive closure of edges (all nodes reachable through one or more edges).
- C) The inverse of edges.
- D) The reflexive closure of edges.

**Answer: B.** The `^` operator computes the transitive closure: `n.^edges`
gives all nodes reachable from `n` by following one or more edges. If `n` is
in `n.^edges`, there's a cycle back to `n`. This is how Alloy encodes
acyclicity constraints.

**Q4.** You check an assertion `for 5` and Alloy finds no counterexample. Can
you conclude the assertion holds in general?

- A) Yes, Alloy proved it.
- B) No, a counterexample might exist with 6 or more atoms.
- C) Yes, if the model is small enough.
- D) Only if you also check `for 10`.

**Answer: B.** Bounded analysis only guarantees no counterexample exists within
the scope. A counterexample might require more atoms than the scope allows.
For design-level sanity checks, small scopes are usually sufficient (most bugs
manifest early), but you can't claim a universal proof.

**Q5.** What's the difference between Alloy and TLA+?

- A) Alloy is for distributed systems; TLA+ is for data structures.
- B) Alloy uses relational logic and bounded SAT-based analysis; TLA+ uses
   temporal logic and state-space exploration.
- C) Alloy is faster but less expressive.
- D) They're the same tool with different syntax.

**Answer: B.** Alloy models relationships between entities using relational
logic and checks properties by SAT-solving bounded instances. TLA+ models
state machines using temporal logic and checks properties by exploring
reachable states. Alloy is better for structural/relational models; TLA+ is
better for behavioral/protocol models.

## Exercises

**Easy:** Add environment-specific dependency restrictions. Model that
production services cannot depend on development services. Check for violations.

**Medium:** Add optional failover dependencies. A service can have a primary
dependency and a failover dependency. Constrain that the failover cannot create
a cycle with the primary. Check for violations.

**Hard:** Model ownership rules between teams and services. Each team owns
services, and services depend on other services. Add a constraint that if
service A depends on service B, the team owning A must have permission to
depend on B's team. Check for permission violations.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Signature | "class" | Set of atoms in a model universe |
| Fact | "always true rule" | Constraint required in every instance |
| Assertion | "property to verify" | Claim checked by searching for counterexample |
| Scope | "search size" | Bound on atoms considered by analyzer |
| Transitive closure | "reachability" | `^r`: all pairs connected by one or more steps of relation `r` |
| Instance | "example" | A concrete arrangement of atoms satisfying all facts |
| Counterexample | "bug witness" | An instance violating an assertion |
| SAT solver | "constraint solver" | Algorithm that finds satisfying assignments for boolean formulas |

## Further Reading

- [Alloy Tools](https://alloytools.org/) — download, documentation, and examples.
- [Software Abstractions](https://mitpress.mit.edu/9780262526545/software-abstractions-revised-edition/) — Daniel Jackson's foundational book on Alloy.
- [Alloy documentation](https://alloytools.org/documentation.html) — language reference.
- [Lightweight Formal Methods](https://www.cs.cmu.edu/~jcr/alloy.html) — Daniel Jackson's papers on Alloy's approach.

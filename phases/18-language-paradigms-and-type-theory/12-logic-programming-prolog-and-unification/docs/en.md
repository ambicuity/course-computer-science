# Logic Programming — Prolog and Unification

> Describe facts and relations; let search derive answers.

**Type:** Learn
**Languages:** Prolog
**Prerequisites:** Phase 18 lessons 01-11
**Time:** ~75 minutes

## Learning Objectives

- Understand unification and backtracking as execution mechanisms.
- Model domain relationships with facts and rules.
- Write queries that leverage logical inference.
- Recognize where declarative logic style is effective.

## The Problem

You have a family tree. You want to know: "Who are all the ancestors of Alice?" In an imperative language, you'd write a graph traversal with a visited set, a stack, and a loop. In Prolog, you write two rules:

```prolog
ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
```

And query: `?- ancestor(X, alice).` The engine finds all answers by searching. You didn't write the traversal. You described the relationship, and the engine derived the consequences.

This is the power of logic programming: you describe what is true, not how to compute it. The engine's search (unification + backtracking) does the computation. For relationship-heavy domains (family trees, type inference, configuration rules, knowledge graphs), this can be dramatically more concise than imperative code.

The cost: you lose control over execution strategy. Prolog's depth-first search can loop on infinite branches. Performance depends on clause order and cut placement. Debugging requires understanding the search tree, not just the code.

## The Concept

### Prolog fundamentals

A Prolog program is a set of **facts** and **rules**.

```prolog
% Facts: base truths
parent(tom, bob).
parent(tom, liz).
parent(bob, ann).
parent(bob, pat).

% Rules: derived truths
grandparent(X, Z) :- parent(X, Y), parent(Y, Z).
sibling(X, Y) :- parent(Z, X), parent(Z, Y), X \= Y.
```

A **query** asks if a goal can be satisfied:

```prolog
?- parent(tom, bob).        % true
?- parent(tom, X).          % X = bob ; X = liz
?- grandparent(tom, ann).   % true
?- sibling(bob, pat).       % true
```

### Unification

Unification is pattern matching with variable binding. Two terms unify if there's a substitution making them identical:

```prolog
% Constants unify with themselves
?- foo = foo.               % true

% Variables unify with anything
?- X = 5.                   % X = 5
?- X = foo(bar).            % X = foo(bar)

% Structured terms unify if functors match and args unify
?- foo(X, b) = foo(a, Y).  % X = a, Y = b

% Occurs check (in standard Prolog, often disabled)
?- X = f(X).                % loops or error (depending on implementation)
```

### Backtracking

When a goal can be satisfied in multiple ways, Prolog tries them in order. If a later goal fails, it backtracks and tries the next alternative:

```prolog
likes(alice, bob).
likes(alice, charlie).
likes(bob, alice).

friend(X, Y) :- likes(X, Y), likes(Y, X).

?- friend(alice, Y).
% Try: likes(alice, bob) → Y=bob, check likes(bob, alice) → true → Y=bob
% On backtrack: likes(alice, charlie) → Y=charlie, check likes(charlie, alice) → fail
% No more alternatives
```

The search tree:

```
friend(alice, Y)
├── likes(alice, Y)    Y=bob
│   └── likes(bob, alice)    true ✓
└── likes(alice, Y)    Y=charlie
    └── likes(charlie, alice)    fail ✗
```

### The cut (!)

The cut `!` commits to the current choice, preventing backtracking:

```prolog
max(X, Y, X) :- X >= Y, !.
max(_, Y, Y).
```

Without the cut, `max(3, 5, 3)` would succeed (first clause fails, second succeeds). With the cut, once `X >= Y` succeeds, the first clause commits. This is Prolog's way of expressing "if-then-else."

### Lists and recursion

```prolog
% List membership
member(X, [X|_]).
member(X, [_|T]) :- member(X, T).

% Append
append([], L, L).
append([H|T], L, [H|R]) :- append(T, L, R).

% Query: find X such that append(X, Y, [1,2,3])
?- append(X, Y, [1,2,3]).
% X=[], Y=[1,2,3]
% X=[1], Y=[2,3]
% X=[1,2], Y=[3]
% X=[1,2,3], Y=[]
```

### Unification as type inference

Prolog-style unification is exactly what Hindley-Milner type inference uses. Type variables unify with types, function types unify structurally:

```prolog
% Type inference as Prolog (conceptual)
typeof(Var, Type) :- lookup(Var, Type, Context).
typeof(app(F, A), T) :- typeof(F, arrow(T1, T)), typeof(A, T1).
typeof(lam(X, B), arrow(T1, T2)) :- typeof(B, T2) in context [X:T1].
```

## Build It

### Step 1: Family relationships

```prolog
% Facts
parent(tom, bob).
parent(tom, liz).
parent(bob, ann).
parent(bob, pat).
parent(liz, joe).

% Rules
grandparent(X, Z) :- parent(X, Y), parent(Y, Z).
sibling(X, Y) :- parent(Z, X), parent(Z, Y), X \= Y.
ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

% Queries
% ?- ancestor(tom, ann).      true
% ?- ancestor(tom, X).        X=ann ; X=pat ; X=joe
% ?- grandparent(X, ann).     X=tom
% ?- sibling(ann, pat).       true
```

### Step 2: List operations

```prolog
% Length
len([], 0).
len([_|T], N) :- len(T, N1), N is N1 + 1.

% Reverse (accumulator style)
rev(L, R) :- rev(L, [], R).
rev([], Acc, Acc).
rev([H|T], Acc, R) :- rev(T, [H|Acc], R).

% Quicksort
qs([], []).
qs([P|Xs], Sorted) :-
    partition(Xs, P, Lows, Highs),
    qs(Lows, LS),
    qs(Highs, HS),
    append(LS, [P|HS], Sorted).

partition([], _, [], []).
partition([X|Xs], P, [X|Lows], Highs) :- X =< P, partition(Xs, P, Lows, Highs).
partition([X|Xs], P, Lows, [X|Highs]) :- X > P, partition(Xs, P, Lows, Highs).
```

### Step 3: Graph reachability

```prolog
% Directed graph
edge(a, b).
edge(b, c).
edge(c, d).
edge(d, a).  % cycle

% Reachable with cycle guard
reachable(X, Y) :- reachable(X, Y, [X]).
reachable(X, Y, _) :- edge(X, Y).
reachable(X, Y, Visited) :-
    edge(X, Z),
    \+ member(Z, Visited),
    reachable(Z, Y, [Z|Visited]).

% ?- reachable(a, d).    true
% ?- reachable(a, a).    true (via cycle)
```

### Step 4: Constraint solving

```prolog
% SEND + MORE = MONEY puzzle
:- use_module(library(clpfd)).

solve([S,E,N,D,M,O,R,Y]) :-
    Vars = [S,E,N,D,M,O,R,Y],
    Vars ins 0..9,
    all_different(Vars),
    S #\= 0, M #\= 0,
    1000*S + 100*E + 10*N + D +
    1000*M + 100*O + 10*R + E #=
    10000*M + 1000*O + 100*N + 10*E + Y,
    label(Vars).

% ?- solve(V).  V = [9,5,6,7,1,0,8,2]
```

## Use It

Logic programming is effective for:

- **Rule engines**: business rules as Prolog-style facts and rules (Drools, CLIPS).
- **Type inference**: Hindley-Milner unification is Prolog-style search.
- **Knowledge graphs**: RDF/SPARQL queries over ontologies.
- **Configuration**: "find a valid configuration given these constraints."
- **Natural language parsing**: definite clause grammars (DCGs) in Prolog.
- **Database queries**: Datalog (a Prolog subset) for recursive queries over data.

SWI-Prolog is the most widely used implementation. It has libraries for HTTP, JSON, and RDF, making it practical for knowledge-base applications.

## Read the Source

- [SWI-Prolog Manual](https://www.swi-prolog.org/pldoc/) — comprehensive reference.
- *The Art of Prolog* (Sterling, Shapiro) — deep treatment of logic programming.
- *Learn Prolog Now!* — accessible introduction.
- [Datalog papers](https://datalog.org/) — the database-friendly Prolog subset.

## Ship It

- `code/main.pl`: family relation and ancestor logic.
- `outputs/README.md`: logic modeling checklist.

## Quiz

**Q1 (Pre).** What is unification in Prolog?

- A) Type checking.
- B) Pattern matching that binds variables to make two terms structurally identical.
- C) String concatenation.
- D) Boolean comparison.

**Answer: B.** Unification finds a substitution making two terms equal. `foo(X, b) = foo(a, Y)` unifies with `X=a, Y=b`. It's the same operation used in type inference (lesson 05). Variables can bind to any term, including other variables.

**Q2 (Pre).** What does backtracking do in Prolog?

- A) Reverses the program.
- B) When a goal fails, the engine returns to the last choice point and tries the next alternative.
- C) Restarts from the beginning.
- D) Skips failed goals.

**Answer: B.** Prolog's execution explores a search tree. When a branch fails (a goal can't be satisfied), the engine backtracks to the most recent choice point with unexplored alternatives and tries the next one. This is how multiple solutions are found.

**Q3 (Post).** How does Prolog's `append` function work when called with `append(X, Y, [1,2,3])`?

- A) It's an error; append needs all three arguments.
- B) It finds all pairs `(X, Y)` such that X ++ Y = [1,2,3] by backtracking through the recursive definition.
- C) It returns [1,2,3] for both X and Y.
- D) It requires a cut to work.

**Answer: B.** Because Prolog uses unification and backtracking, calling `append` with two free variables generates all possible splits: `([], [1,2,3])`, `([1], [2,3])`, `([1,2], [3])`, `([1,2,3], [])`. The same code works as append, split, and prefix check, depending on which arguments are bound.

**Q4 (Post).** Why might Prolog's depth-first search be a problem?

- A) It's always slow.
- B) It can loop infinitely on branches that have no solution, even if a solution exists on another branch.
- C) It can't find multiple solutions.
- D) It requires explicit loop constructs.

**Answer: B.** Prolog explores depth-first: it goes as deep as possible before trying alternatives. If a branch is infinite (e.g., left-recursive grammar), it loops forever, even though a solution exists on a different branch. Breadth-first search would avoid this but uses more memory.

**Q5 (Post).** How is Prolog-style unification related to type inference?

- A) They're unrelated.
- B) Hindley-Milner type inference uses the same unification algorithm to solve type constraints.
- C) Type inference uses a different algorithm.
- D) Prolog doesn't support unification.

**Answer: B.** The `unify` function in a type checker (lesson 05) is the same operation as Prolog's unification. Type variables unify with types, arrow types unify structurally. Many type checkers are implemented as Prolog-style logic programs.

## Exercises

1. **Easy.** Add `sibling` and `cousin` rules to the family tree. Query for all siblings of `bob`.
2. **Medium.** Add a cycle guard to graph reachability. Test with a graph that has cycles.
3. **Hard.** Implement a type checker for a small expression language in Prolog, using unification for type inference.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Unification | "matching" | Variable-binding process making two terms structurally identical |
| Backtracking | "try alternatives" | Systematic exploration of rule/goal branches on failure |
| Fact | "base relation" | Ground statement accepted as true without proof |
| Rule | "derived relation" | Implication defining new truths from goals (`Head :- Body`) |
| Cut (!) | "commit" | Prevents backtracking past this point, committing to the current choice |

## Further Reading

- [SWI-Prolog Manual](https://www.swi-prolog.org/pldoc/)
- [Learn Prolog Now](http://www.learnprolognow.org/)
- *The Art of Prolog* (Sterling, Shapiro)
- [Datalog: A Prolog for Databases](https://datalog.org/)

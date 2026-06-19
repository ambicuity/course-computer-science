# Markov Chains & Random Walks (Discrete)

> A process where the next state depends only on the current one. Once you accept that, you can analyze PageRank, MCMC, and load-balancing schemes by raising a single matrix to a power.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 19
**Time:** ~60 minutes

## Learning Objectives

- Define a discrete-time Markov chain (states, transition matrix, initial distribution).
- Compute n-step transition probabilities by matrix exponentiation; recognize stationary distributions as fixed points of the transition matrix.
- Apply the random-walk-on-a-graph framing to PageRank and to mixing-time analyses.
- Recognize the conditions for ergodicity (irreducibility + aperiodicity) and what they buy you (convergence to a unique stationary distribution).

## The Problem

The "next state depends only on the current state" property — the **Markov property** — applies to a surprising number of CS systems:

- A web surfer randomly clicking links → PageRank.
- A cache evicting via LRU → state-machine analysis of hit ratios.
- A Metropolis-Hastings MCMC sampler → posterior estimation.
- A randomized algorithm with retry on failure → expected restart-count.
- A network's queue length under random arrivals → birth-death chain.

Each is a Markov chain; the same linear-algebra toolkit answers "what's the long-run distribution?" and "how fast do we converge?"

## The Concept

### Definition

A **discrete-time Markov chain** is a sequence of random variables `X₀, X₁, X₂, …` taking values in a (finite, in this lesson) state space S, with the property:

```
P(X_{t+1} = j | X_t = i, X_{t-1}, ..., X_0) = P(X_{t+1} = j | X_t = i) = P_{ij}
```

The transition probabilities `P_{ij}` form an n × n **transition matrix** P, where rows sum to 1.

### n-step transitions

The probability of being in state j after n steps, given start at i, is `(Pⁿ)_{ij}` — the iₜₕ-row, jₜₕ-column entry of P raised to the nₜₕ power.

If π₀ is the initial probability row-vector, then `πₙ = π₀ Pⁿ` is the distribution at time n.

### Stationary distribution

A row vector π is **stationary** if `π P = π`. Equivalently, π is a left eigenvector of P with eigenvalue 1. For a finite chain, at least one stationary distribution always exists.

For most "nice" chains (irreducible + aperiodic), there's a *unique* stationary π and `πₙ → π` regardless of the starting distribution. The chain **mixes**.

### Ergodicity conditions

- **Irreducible**: every state can reach every other state (the chain's graph is strongly connected).
- **Aperiodic**: gcd of cycle lengths through any state is 1.

An irreducible + aperiodic chain is **ergodic**: unique stationary π, and `Pⁿ → 𝟙 πᵀ` (every row of Pⁿ converges to π) — geometrically fast.

### Random walk on a graph

Given an undirected graph G = (V, E), the **simple random walk** moves to a uniformly random neighbor each step. The stationary distribution is:

```
π_v = deg(v) / (2|E|)
```

For a regular graph (all vertices same degree), π is uniform.

### PageRank

PageRank treats the web as a directed graph and computes the stationary distribution of a random surfer:

```
π = α · π M + (1 - α) · 𝟙/n
```

where M is a row-stochastic transition matrix derived from the link graph and α ≈ 0.85 is the damping factor. The `(1-α)·𝟙/n` term handles "teleport with probability 1-α to a uniformly random page" — fixing periodicity and reducibility issues.

PageRank converges in ~50 power-iteration steps on the live web. The full algorithm in Phase 11.

### Mixing time

How fast does πₙ → π? Mixing time tₘᵢₓ(ε) is the smallest n with `||πₙ − π||₁ ≤ ε`. Tied to the **spectral gap** of P: tₘᵢₓ = O((1 / spectral_gap) · log(1/ε)).

For expander graphs (constant spectral gap), random walks mix in O(log n) steps. For "narrow" graphs (think a long cycle), mixing is slow — O(n²).

## Build It

Open `code/main.py`.

### Step 1: Weather chain

States: 'sunny', 'rainy'. P = [[0.9, 0.1], [0.5, 0.5]]. Compute P², P¹⁰, P¹⁰⁰. Observe row-convergence.

### Step 2: Stationary distribution

Two methods agree: power iteration and the eigenvector with eigenvalue 1.

### Step 3: Random walk on a small graph

Stationary π_v = deg(v) / (2|E|); verify by power iteration on the walk matrix.

### Step 4: PageRank on a tiny directed graph

A 4-node web; damping α = 0.85; pages with more inbound links rank higher.

### Step 5: Mixing-time comparison

A 6-cycle vs K₆: count steps until distribution is ε-close to π.

## Use It

- **PageRank** (Phase 11): the original Google ranking, still used as a component.
- **MCMC** (Phase 17, ML): sample from intractable posteriors by running a chain whose stationary is the target distribution.
- **Cache analysis**: hit-ratio of LRU is computed via a state-space Markov model.
- **Load balancing**: power-of-two-choices, random-walk-based hash distribution.
- **Reliability**: birth-death chains for queues and failure analysis.

## Read the Source

- *Markov Chains and Mixing Times* by Levin, Peres, Wilmer — modern, CS-friendly textbook (free draft online).
- *Probability and Computing* (Mitzenmacher & Upfal), Chapter 7 — MCMC and random walks.
- [The original PageRank paper](http://infolab.stanford.edu/pub/papers/google.pdf) — Brin & Page 1998, easy read.

## Ship It

This lesson ships **`outputs/markov.py`** — `stationary(P)` (eigenvector or power-iteration), `walk_sim(P, start, n)`, `pagerank(adj, damping=0.85)`.

## Exercises

1. **Easy.** Given the weather chain, compute the long-run probability of "sunny" by raising P to a high power. Compare with the eigenvector method.
2. **Medium.** A random walk on a 4-vertex cycle (square). Compute the stationary distribution by deg/2E. Confirm with simulation (1 million steps).
3. **Hard.** Implement PageRank on the graph of US states (state borders). Find the top-5 highest-PageRank states. (Hint: large central states beat edge states.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Markov chain | "Memoryless process" | Sequence X₀, X₁, ... where each transition depends only on the current state |
| Transition matrix | "P_{ij}" | n × n row-stochastic matrix; (Pⁿ)_{ij} is the n-step transition probability |
| Stationary distribution | "Long-run fraction" | A row vector π with πP = π; for ergodic chains, unique and reached by any start |
| Ergodic | "Mixing" | Irreducible + aperiodic; guarantees unique stationary + convergence from any start |
| Mixing time | "Steps to converge" | Smallest n with πₙ ≈ π to within ε; controlled by the spectral gap of P |

## Further Reading

- [Persi Diaconis's classic paper on card shuffling](https://www.jstor.org/stable/2241075) — random walks on the symmetric group; how many riffle shuffles you need (~7).
- *MCMC in Practice* by Gilks et al. — practitioner's guide for stats/Bayesian inference.
- [Markov Chain Monte Carlo notes, MIT 18.405J](https://ocw.mit.edu/courses/18-405j-advanced-complexity-theory-fall-2001/) — high-level CS perspective.

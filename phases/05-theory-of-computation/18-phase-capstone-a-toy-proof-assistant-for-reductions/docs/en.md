# Phase Capstone — A Toy Proof Assistant for Reductions

> Every formal system starts with primitives and inference rules. This capstone gives you both — and a CLI that checks your work.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–17
**Time:** ~120 minutes

## Learning Objectives

- Represent computational models (DFA, NFA, PDA, TM) in a uniform specification format and verify language membership programmatically.
- Guide and verify pumping lemma proofs for regular and context-free languages by encoding the adversarial decomposition logic.
- Verify polynomial-time reductions between NP-complete problems by checking the correctness conditions automatically.
- Build a CLI proof assistant that ties together the entire Phase 5 curriculum.

## The Problem

Phase 5 covered automata, grammars, Turing machines, decidability, and complexity. Each lesson built a piece, but the pieces are disconnected: your DFA simulator, TM simulator, pumping lemma intuition, and NP-completeness toolkit live in separate files. A proof assistant — even a toy one — unifies them:

- **It forces precision.** "The language of strings with an even number of 1s" is informal. A DFA is precise. Encoding problems in the assistant requires you to be exact.
- **It checks your reasoning.** When you claim "A reduces to B," the assistant verifies the reduction preserves yes/no answers on test instances.
- **It mirrors real tools.** Lean, Coq, and Isabelle are industrial proof assistants. They verify hardware designs, compiler correctness, and mathematical theorems. This toy version captures the same workflow: define objects, state claims, provide evidence, get verification.

Without this integration, you know the pieces but cannot assemble them into a proof.

## The Concept

### Uniform model representation

Every computational model reduces to: a set of states, an alphabet, a transition function, a start state, and accept/reject criteria. We use a dictionary-based spec:

```python
# DFA
{"type": "dfa", "states": {0,1}, "alphabet": {"0","1"},
 "transitions": {(0,"0"): 0, (0,"1"): 1, (1,"0"): 1, (1,"1"): 0},
 "start": 0, "accept": {0}}

# NFA (transitions map to SETS of states)
{"type": "nfa", ... "transitions": {(0,"a"): {1,2}} ...}

# PDA (transitions include stack operations)
{"type": "pda", ... "transitions": {(0,"a","Z"): (1, ["A","Z"])} ...}

# TM (transitions include head movement)
{"type": "tm", ... "transitions": {("q0","0"): ("q0","1","R")} ...}
```

The assistant detects the model type and dispatches to the correct simulator.

### Pumping lemma verification

The pumping lemma says: if L is regular, ∃p such that ∀w ∈ L with |w| ≥ p, we can write w = xyz with |xy| ≤ p, |y| ≥ 1, and ∀i ≥ 0, xyⁱz ∈ L.

To prove L is *not* regular, you exhibit a w and show that for *every* valid split xyz, some pumping power i breaks membership. The assistant:
1. Takes your chosen w and the claimed p.
2. Enumerates all valid (x, y, z) splits.
3. Checks that for each split, some i ≥ 0 makes xyⁱz ∉ L.

### Reduction verification

A reduction A ≤_p B requires: (1) a polynomial-time function f mapping A-instances to B-instances, and (2) x ∈ A ⟺ f(x) ∈ B. The assistant runs f on test instances and checks the equivalence.

## Build It

### The `ProofAssistant` class

```python
class ProofAssistant:
    def __init__(self):
        self.automata = {}
        self.reductions = {}

    def define_automaton(self, name, spec):
        """Register a DFA/NFA/PDA/TM under a name."""
        self.automata[name] = spec

    def verify_membership(self, name, string):
        """Test if string is accepted by the named automaton."""
        spec = self.automata[name]
        if spec["type"] == "dfa":
            return self._run_dfa(spec, string)
        elif spec["type"] == "nfa":
            return self._run_nfa(spec, string)
        # ... PDA and TM

    def pumping_lemma(self, automaton_name, p, witness, alphabet):
        """Verify a pumping lemma proof that a language is not regular."""
        # Enumerate all valid splits of witness
        # Check that for each split, some pumping power breaks membership

    def verify_reduction(self, name_a, name_b, reduction_fn, test_instances):
        """Verify a poly-time reduction preserves membership."""
        for x in test_instances:
            f_x = reduction_fn(x)
            a_answer = self.verify_membership(name_a, x)
            b_answer = self.verify_membership(name_b, f_x)
            if a_answer != b_answer:
                return False, f"Counterexample: {x}"
        return True, "Reduction verified on all test instances"
```

### DFA / NFA simulation

```python
def _run_dfa(self, spec, string):
    state = spec["start"]
    for ch in string:
        state = spec["transitions"].get((state, ch))
        if state is None:
            return False
    return state in spec["accept"]

def _run_nfa(self, spec, string):
    current = {spec["start"]}
    for ch in string:
        next_states = set()
        for s in current:
            next_states |= spec["transitions"].get((s, ch), set())
            next_states |= spec["transitions"].get((s, ""), set())  # epsilon
        current = next_states
    return bool(current & spec["accept"])
```

Run `python3 code/main.py` to launch the interactive CLI.

## Use It

The production equivalents of this toy:

- **Lean 4** (leanprover.github.io): used to verify mathematical theorems and software correctness. The Mathlib library has formalized much of undergraduate mathematics.
- **Coq** (coq.inria.fr): used to verify compiler correctness (CompCert — a verified C compiler) and cryptographic protocols.
- **Isabelle/HOL** (isabelle.in.tum.de): used for verifying security protocols and hardware designs.
- **Z3 / SMT solvers**: automated theorem provers that combine SAT with theory reasoning (integers, arrays, bitvectors).

All of these share the same workflow you see here: define objects, state properties, provide evidence, get machine-checked verification.

## Read the Source

- [The Hitchhiker's Guide to Logical Verification](https://lean-forward.github.io/logical-verification/) — Lean 4 tutorial covering DFA verification.
- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — Coq textbook that builds from propositional logic to programming language semantics.
- [Z3 tutorial](https://theory.stanford.edu/~nikolaj/programmingz3.html) — the practical path: encode, solve, extract.

## Ship It

This lesson ships **`outputs/proof_assistant.py`** — a clean, runnable CLI script. Launch it to define automata, test membership, run pumping lemma proofs, and verify reductions interactively.

## Exercises

1. **Easy.** Define a DFA for "strings over {a, b} with exactly two a's" and verify membership on five test strings (three should be accepted, two rejected).
2. **Medium.** Use the pumping lemma verifier to prove that {aⁿbⁿ | n ≥ 0} is not regular. Choose a witness, set p, and confirm every split fails.
3. **Hard.** Define the 3-SAT → Vertex Cover reduction as a reduction object in the proof assistant. Supply three test 3-SAT instances and verify that satisfiable instances produce Vertex Cover instances with covers of the target size.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Proof assistant | "Tool that checks proofs" | Software where humans provide proof steps, machine checks each inference |
| Formal verification | "Mathematical guarantee" | Proving properties of systems using symbolic logic, not testing |
| Pumping lemma | "Pumping argument" | A structural regularity guarantee: regular languages contain repeating subpatterns |
| Reduction (≤_p) | "Transformation" | A poly-time function preserving yes/no membership |
| Model | "Automaton / machine" | A formal specification of a computational process |

## Further Reading

- Sipser, *Introduction to the Theory of Computation* — the textbook that covers everything this capstone touches.
- Pierce et al., *Software Foundations* (Volume 1: Logical Foundations) — the gentlest introduction to proof assistants, written in Coq.
- de Moura & Bjørner, "Z3: An Efficient SMT Solver" (TACAS 2008) — the paper behind the solver used everywhere in industry.

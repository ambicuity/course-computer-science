# Verified Software - seL4, CompCert, etc.

> Verification is most valuable where failure cost is highest.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 17 lessons 01-16
**Time:** ~60 minutes

## Learning Objectives

- Understand what "verified software" means in practice.
- Compare theorem-proved components with extensively tested ones.
- Identify cost/benefit tradeoffs for verification investment.
- Build a decision rubric for adopting formal verification.

## The Problem

Formal verification has strong outcomes but real costs. Teams struggle to decide
where to invest. Over-applying verification slows delivery; under-applying misses
high-assurance opportunities.

A medical device runs software that controls drug dosage. A bug could harm
patients. Testing found 99.9% of known bugs. But the 0.1% that escaped could
be catastrophic. Is it worth spending 3 years and $10M to prove the software
correct? For a video game, no. For a pacemaker, probably yes.

The question isn't "should we verify everything?" It's "which components justify
the cost of verification, and which are adequately served by testing?" This
lesson examines real verified systems and builds a decision framework.

## The Concept

### What "Verified" Means

A verified component has a machine-checked proof that its implementation matches
a formal specification. This is not the same as "well-tested" or "bug-free."

```
    Assurance Spectrum:
    
    Testing          Model Checking       Proof Assistants
    ───────          ──────────────       ────────────────
    "We tried        "We checked all      "We proved it
     many inputs"     states in a          for all inputs
                      bounded model"       mathematically"
    
    ████░░░░░░░      ████████░░░          ████████████
    Evidence          Strong evidence      Mathematical
    (inductive)       (exhaustive,         guarantee
                       bounded)            (within spec)
    
    Cost: Low         Cost: Medium         Cost: Very High
    Coverage: Partial Coverage: Bounded    Coverage: Total
```

"Verified" means: given the formal specification is correct, and the trusted
computing base (TCB) is correct, the implementation satisfies the specification
for all inputs. The proof is machine-checked: no human reasoning gap.

What "verified" does NOT mean:

- The specification is correct (that's a human judgment).
- The hardware is correct (cosmic rays, manufacturing defects).
- The TCB is correct (the proof checker, the OS, the compiler used to build
  the verified software).
- The system is secure (security properties require separate proofs).

### seL4: The Verified Microkernel

seL4 is a microkernel with a machine-checked proof of functional correctness.
It's the world's first OS kernel with a complete, formal proof of implementation
correctness.

```
    seL4 Proof Architecture:
    
    ┌─────────────────────────────────────┐
    │  Abstract Specification             │  What the kernel SHOULD do
    │  (high-level Haskell model)         │
    └──────────────┬──────────────────────┘
                   │  Refinement proof
                   │  (Isabelle/HOL)
    ┌──────────────▼──────────────────────┐
    │  Executable Specification           │  Haskell model compiled to C
    │  (Haskell → C translation)          │
    └──────────────┬──────────────────────┘
                   │  Refinement proof
                   │  (Isabelle/HOL)
    ┌──────────────▼──────────────────────┐
    │  C Implementation                   │  Actual C code running on hardware
    │  (binary code on ARM)               │
    └─────────────────────────────────────┘
    
    Proofs guarantee:
    - Functional correctness (C matches abstract spec)
    - Integrity (no memory safety violations)
    - Confidentiality (information flow)
    - Worst-case execution time (for real-time variants)
```

The proof covers:

- **Functional correctness:** The C implementation behaves exactly as the
  abstract specification says.
- **Integrity and confidentiality:** No buffer overflows, no null pointer
  dereferences, no information leaks between security domains.
- **Absence of common bugs:** No arithmetic overflow, no use-after-free, no
  memory leaks in the kernel.

What the proof does NOT cover:

- Hardware correctness.
- User-space code running on the kernel.
- Performance properties (though worst-case timing is proven for some variants).

### CompCert: The Verified Compiler

CompCert is a C compiler with a machine-checked proof that compilation preserves
program semantics. If the source program has behavior X, the compiled program
has behavior X.

```
    CompCert Proof:
    
    Source C program
         │
         ▼
    ┌─────────────────────────────────────┐
    │  Frontend: C → Cminor               │
    │  Proof: semantics preserved         │
    └──────────────┬──────────────────────┘
                   │
    ┌──────────────▼──────────────────────┐
    │  Backend: Cminor → RTL → Mach → ASM │
    │  Proof: semantics preserved         │
    └──────────────┬──────────────────────┘
                   │
    ▼
    Assembly code (ARM, PowerPC, x86, RISC-V)
    
    Theorem: If source program has behavior B,
    then compiled program has behavior B.
```

This eliminates **miscompilation bugs**: cases where the compiler introduces
behavior not present in the source code. In unverified compilers (GCC, LLVM),
these bugs exist and are found regularly. CompCert's proof guarantees they
can't happen.

CompCert's scope:

- Covers: C11 language (without concurrency), ARM/PowerPC/x86/RISC-V backends.
- Doesn't cover: Inline assembly, linker behavior, hardware-specific properties.
- TCB: The Coq proof checker, the OCaml runtime, the operating system.

### Verified Cryptographic Libraries

Several crypto libraries have formal proofs:

- **HACL***: Verified C implementation of crypto primitives (AES, ChaCha20,
  Poly1305, Curve25519, SHA-2, etc.). Proofs in F* and Vale.
- **Jasmin**: Verified assembly-level implementations of crypto primitives.
- **TLS 1.3 proofs**: Formal proofs that TLS 1.3 handshake provides key
  secrecy and authentication.

These proofs catch implementation bugs that testing misses: timing side channels,
incorrect constant-time operations, and subtle mathematical errors.

### The Trusted Computing Base

Every proof rests on assumptions. The TCB includes:

- The proof checker (Coq, Isabelle, Lean).
- The compiler used to build the proof checker (typically unverified).
- The operating system running the proof checker.
- The hardware running everything.

If the TCB has a bug, the proof might be invalid. Minimizing the TCB is a key
goal of verification projects.

## Build It

Create a decision matrix for when to invest in formal verification:

### Decision Rubric

| Factor | Low (Test) | Medium (Model Check) | High (Prove) |
|---|---|---|---|
| **Failure impact** | Inconvenience | Data loss | Safety/security breach |
| **Component criticality** | Utility function | Core business logic | Safety-critical path |
| **Interface stability** | Changing weekly | Stable for months | Stable for years |
| **Proof tooling expertise** | None on team | Some experience | Dedicated experts |
| **Regulatory pressure** | None | Industry standard | Required by law |
| **Input space** | Small, enumerable | Medium, bounded | Infinite, needs proof |

### Scoring

Score each factor 1-3. Sum the scores:

- **6-10:** Testing is sufficient. Focus on good test practices.
- **11-14:** Consider model checking for critical protocols. Use TLA+ or Alloy.
- **15-18:** Formal verification is justified. Start with proof assistants for
  the most critical components.

### Real-World Examples

| System | Verification Level | Justification |
|---|---|---|
| Video game physics | Testing only | Failure is inconvenient, not dangerous |
| Web application | Testing + fuzzing | Data loss possible but recoverable |
| Database engine | Model checking + fuzzing | Data integrity is critical |
| Flight control | Full verification | Safety-critical, regulatory requirement |
| Medical device | Full verification | Patient safety, regulatory requirement |
| Crypto library | Targeted verification | Security-critical, subtle bugs |

## Use It

Use verification for components that are:

- **Safety-critical:** Bugs could harm people (medical, aviation, automotive).
- **Security-critical:** Bugs could compromise confidentiality or integrity
  (crypto, access control, kernels).
- **Highly stable in interface and semantics:** The specification doesn't
  change frequently, so proofs don't break constantly.

Don't use verification for:

- Components with rapidly changing requirements.
- Components where testing provides adequate confidence.
- Components where the cost of proof exceeds the cost of failure.

Production references:

- seL4 is used in military drones, autonomous vehicles, and secure
  communication systems.
- CompCert is used in avionics and safety-critical embedded systems.
- Amazon uses TLA+ (model checking, not full proof) for AWS services.
- Google uses formal methods for Android's security-critical components.

## Read the Source

- [seL4](https://sel4.systems/) — seL4 project publications and proof artifacts.
- [CompCert](https://compcert.org/) — CompCert overview and proof scope.
- [HACL*](https://hacl-star.github.io/) — verified crypto library.
- [Amazon's TLA+ experience](https://www.amazon.science/publications/how-amazon-web-services-uses-formal-methods) — model checking at scale.

## Ship It

This lesson ships:

- `code/notes.md`: adoption rubric and comparative notes.
- `outputs/README.md`: verification decision checklist.

## Quiz

**Pre-questions:**

**Q1.** What does it mean for software to be "verified"?

- A) It has been tested extensively.
- B) It has a machine-checked proof that its implementation matches a formal
   specification for all inputs.
- C) It has no bugs.
- D) It passed code review.

**Answer: B.** Verified means there's a mathematical proof, checked by a
machine, that the implementation satisfies a formal specification. This is
stronger than testing (which covers specific inputs) but depends on the
specification being correct and the trusted computing base being sound.

**Q2.** What is the "trusted computing base" (TCB)?

- A) The entire system.
- B) The minimal set of components whose correctness the proofs depend on
   (proof checker, compiler, OS, hardware).
- C) The tested components.
- D) The unverified components only.

**Answer: B.** The TCB is everything the proof assumes to be correct. If the
proof checker has a bug, the proof might be invalid. If the hardware has a
cosmic ray error, the proof doesn't help. Minimizing and trusting the TCB is
a key concern in verification projects.

**Post-questions:**

**Q3.** seL4 has a proof of functional correctness. What does this proof
NOT cover?

- A) The C implementation matches the abstract specification.
- B) Hardware correctness and user-space code.
- C) Absence of buffer overflows.
- D) Memory safety.

**Answer: B.** The proof covers the kernel's C implementation against its
abstract specification. It doesn't prove hardware is correct, user-space
programs are correct, or the system is secure against all attacks. These
require separate proofs or are outside the scope of formal methods.

**Q4.** When should you NOT invest in formal verification?

- A) When the component is safety-critical.
- B) When the interface changes frequently and proofs would break constantly.
- C) When the component handles security-critical operations.
- D) When regulatory requirements demand it.

**Answer: B.** Formal proofs are expensive to maintain. When the specification
changes frequently, proofs must be rewritten each time. The cost of proof
maintenance can exceed the benefit. Verification works best for stable,
well-specified components where the cost of failure justifies the investment.

**Q5.** CompCert proves that compilation preserves program semantics. What
class of bugs does this eliminate?

- A) Bugs in the source program.
- B) Miscompilation bugs: cases where the compiler introduces behavior not
   present in the source code.
- C) Hardware bugs.
- D) Bugs in the operating system.

**Answer: B.** CompCert's proof guarantees that if the source program has
behavior X, the compiled program has behavior X. This eliminates the class of
bugs where the compiler itself introduces incorrect behavior (miscompilation).
Unverified compilers (GCC, LLVM) have such bugs; CompCert provably doesn't.

## Exercises

**Easy:** Score one module in your current project using the verification
decision rubric. What assurance level does it justify?

**Medium:** Identify one area in your stack where testing is sufficient and
one where proof is justified. Explain the reasoning for each, considering
failure impact, stability, and cost.

**Hard:** Define the integration assumptions between verified and unverified
layers. If you have a verified kernel (seL4) running unverified user-space
code, what properties does the kernel guarantee regardless of user-space
behavior? What properties require user-space verification?

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Verified component | "bug-free" | Correct with respect to formal spec under explicit assumptions |
| Trusted computing base | "core system" | Minimal components whose correctness assumptions matter most |
| Refinement proof | "equivalence" | Mapping low-level behavior to high-level specification |
| Assurance case | "confidence argument" | Structured evidence and assumptions for dependability claim |
| Functional correctness | "does what it should" | Implementation matches specification's input-output behavior |
| Miscompilation | "compiler bug" | Compiler introduces behavior not present in source code |
| Information flow | "data leakage" | Proof that secret data doesn't leak to public channels |

## Further Reading

- [seL4](https://sel4.systems/) — seL4 project, publications, and proof artifacts.
- [CompCert](https://compcert.org/) — CompCert compiler and proof scope.
- [HACL*](https://hacl-star.github.io/) — verified crypto library.
- [sel4 White Paper](https://sel4.systems/About/seL4-whitepaper.pdf) — accessible overview of seL4's verification.
- [CompCert Overview](https://compcert.org/compcert-C.pdf) — CompCert's approach to verified compilation.
- [Isabelle/HOL](https://isabelle.in.tum.de/) — the proof assistant used for seL4's verification.

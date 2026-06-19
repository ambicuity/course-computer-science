# Computer Science Glossary

A growing list of CS terms with their honest definitions. Each entry: what people commonly
mean, what it actually means, and (when useful) why it's called that.

## A

### Abstraction
- **What people say:** "Hiding complexity."
- **What it actually means:** Choosing what to ignore. Every abstraction is a deliberate
  decision about which details cross the interface and which don't. A leaky abstraction is
  one where the hidden details still affect users.

### Algorithm
- **What people say:** "A recipe for solving a problem."
- **What it actually means:** A finite, unambiguous procedure that terminates on every
  valid input and produces a correct output. The unambiguity matters more than the
  recipe metaphor — pseudocode is not an algorithm if a step says "guess wisely."

### Amortized complexity
- **What people say:** "Average cost."
- **What it actually means:** Worst-case total cost of a sequence of operations divided
  by the number of operations, with **no probabilistic assumptions**. Different from
  average-case, which is probabilistic.

### Atomicity
- **What people say:** "Indivisible."
- **What it actually means:** From the outside, an operation appears to happen entirely
  or not at all — there is no observable intermediate state. The implementation may
  still have many steps; atomicity is about what observers can see.

## B

### Big-O
- **What people say:** "How fast an algorithm runs."
- **What it actually means:** An asymptotic upper bound on growth rate. `O(n²)` says
  "for sufficiently large n, the cost is at most some constant times n²." It does **not**
  say anything about small inputs or constant factors.

### B-Tree
- **What people say:** "Like a binary tree but wider."
- **What it actually means:** A self-balancing search tree designed for block-oriented
  storage where reading one node costs the same as reading one disk page (or cache line).
  The branching factor is set to maximize page utilization, not to minimize comparisons.

## C

### Cache
- **What people say:** "A place to store things for later."
- **What it actually means:** A fixed-size, low-latency store positioned between a fast
  consumer and a slow producer, with a replacement policy (e.g., LRU) that decides what
  to evict when full. Without an eviction policy, it's just a buffer.

### Consensus
- **What people say:** "Everyone agrees."
- **What it actually means:** A property of a distributed protocol: every correct
  process eventually decides on the same value, and that value was proposed by some
  process. FLP says you can't always achieve this in an async network with even one
  crash failure — Raft and Paxos sidestep it by assuming partial synchrony.

## D

### Deadlock
- **What people say:** "Two threads stuck."
- **What it actually means:** A circular wait on a set of resources where no participant
  can make progress. Specifically requires four conditions (Coffman): mutual exclusion,
  hold-and-wait, no preemption, circular wait. Break any one and deadlock can't form.

### Denormal number
- **What people say:** "A weird tiny float."
- **What it actually means:** A floating-point value with the smallest possible exponent
  and a leading zero in the mantissa. Exists to maintain gradual underflow. On many
  CPUs, hitting denormals tanks performance because they fall through to a slow path.

## H

### Hash function
- **What people say:** "Turns a key into a number."
- **What it actually means:** A deterministic function from arbitrary-length input to
  fixed-length output, with good distribution. "Good distribution" means different
  things in different contexts: cryptographic hashes also need preimage resistance.

## I

### Interface
- **What people say:** "What you can call."
- **What it actually means:** A contract specifying the set of operations, their
  signatures, and (often implicitly) their semantics — including invariants the caller
  can rely on. An interface without specified semantics is a syntax document, not an
  interface.

## M

### Memory model
- **What people say:** "How the CPU orders memory ops."
- **What it actually means:** The set of guarantees the hardware (or language runtime)
  provides about the visibility and ordering of memory operations across threads. C++,
  Java, and JVM each have written specs; x86 is roughly sequential consistency for
  aligned scalar ops; ARM and RISC-V are much weaker.

## N

### NP-complete
- **What people say:** "Impossible to solve fast."
- **What it actually means:** In NP **and** every problem in NP reduces to it in
  polynomial time. We don't know if P=NP, so we can't say it's "impossible to solve
  fast" — we can say no polynomial-time algorithm is currently known and finding one
  would solve every NP problem too.

## P

### Pointer
- **What people say:** "An address."
- **What it actually means:** A value that uniquely identifies a memory location in a
  particular address space, along with (in typed languages) a static promise about what
  lives at that location. In C, the *type* of a pointer participates in pointer
  arithmetic — `p+1` advances by `sizeof(*p)` bytes, not one byte.

### Process
- **What people say:** "A running program."
- **What it actually means:** An OS-managed unit of resource allocation: an address
  space, a set of open file descriptors, a credential, and one or more threads of
  execution. The "running program" framing under-emphasizes the address space, which is
  what `fork()` actually copies.

## R

### Race condition
- **What people say:** "Two threads collide."
- **What it actually means:** Observable behavior depends on the relative timing of
  events. A data race is a specific kind of race condition involving concurrent
  unsynchronized access to a shared memory location where at least one is a write.

## T

### Thread
- **What people say:** "A lightweight process."
- **What it actually means:** An independent flow of control within a process, sharing
  the address space and most resources with its peer threads. "Lightweight" usually just
  means "context switches don't flush the address space."

## V

### Virtual memory
- **What people say:** "Pretending you have more RAM."
- **What it actually means:** A per-process address space mapped through a page table
  to physical memory (or to disk, or to nothing). Buys isolation, demand paging,
  copy-on-write, and shared mappings. The "pretending you have more RAM" framing only
  covers swap, which is one feature of many.

---

_This glossary grows as lessons reference terms. Lessons add their own "Key Terms"
table; entries that recur three or more times get promoted here._

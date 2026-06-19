# Singly and Doubly Linked Lists

> Pointer chasing is the structure. Understand when that is a feature and when it is a fatal flaw.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** P02 (pointers, ownership)
**Time:** ~60 minutes

## Learning Objectives

- Implement singly- and doubly-linked lists with push/pop at both ends, insert/remove at arbitrary positions, and iteration.
- Apply the **intrusive linked list** pattern used in real kernels (Linux's `list_head`).
- Compare with `Vec`: when does pointer-chasing beat contiguous arrays, and when does cache locality crush the linked-list O(1) insert win?
- Express linked-list ownership in Rust (single owner → safe `Option<Box<Node>>`; many owners → `Rc<RefCell<…>>` or hand-written `unsafe`).

## The Problem

The linked list is the textbook data structure that, in production, is almost never the right answer — yet you must understand it because it appears inside every kernel scheduler, every memory allocator's free-list, every fast queue.

It exists for one reason: **constant-time insert/remove at a known position**. Arrays shift O(n) bytes on insert; linked lists rewire two pointers. If you have a pointer to the node, you're done.

The cost: every traversal step is a pointer load that may miss cache. A `Vec` of n integers fits 16 to a cache line; a linked list of n integers fits 1 per cache line. For dense data, the array wins by 10-100×.

The rule of thumb: **arrays for iteration, lists for splice**. Most workloads are iteration. Hence the textbook answer ("use a linked list!") is usually wrong outside specific contexts.

## The Concept

### Singly linked list (SLL)

```c
typedef struct Node {
    int data;
    struct Node *next;
} Node;
```

Operations:

| Op | Cost | Why |
|----|------|-----|
| `push_front(x)` | O(1) | New node → head's old position |
| `pop_front()` | O(1) | Advance head |
| `push_back(x)` | O(n) without tail pointer, O(1) with one | Need last node |
| `find(v)` | O(n) | Linear scan |
| `remove(node)` | O(1) given pred, O(n) without | Need predecessor's `next` |
| Iterate | O(n) | Pointer-chase each node |

### Doubly linked list (DLL)

```c
typedef struct Node {
    int data;
    struct Node *prev, *next;
} Node;
```

Adds `prev`. Now `remove(node)` is O(1) without a predecessor pointer: rewire `node->prev->next = node->next` and `node->next->prev = node->prev`. This is what makes DLLs the data structure of choice for LRU caches, scheduler queues, and free-list tracking.

Costs DLL adds:
- 1 extra pointer per node (memory)
- 2 writes per modification instead of 1
- 2 invariants to maintain (`prev` and `next` must agree)

### Intrusive lists

In C kernels (Linux, BSD, FreeRTOS), the list node is *inside* the user struct:

```c
struct task {
    int pid;
    char name[64];
    struct list_head tasks;     /* the node */
};
```

`list_head` has only `prev` and `next` pointers — no `data` field. To traverse, you take a `list_head*` and use the `container_of` macro to recover the `task*`. Benefit: no separate node allocation; the user struct IS the node. Tasks can be in multiple lists simultaneously (add a second `list_head`).

This is the pattern Linux uses everywhere. Look at `include/linux/list.h` — every kernel data structure links through it.

### Rust ownership for linked lists

Rust's borrow checker makes naïve linked lists awkward. Three approaches:

1. **Safe, single-owner: `Option<Box<Node>>`**. Each node owns its `next`. Works for SLL.
2. **Multi-owner: `Rc<RefCell<Node>>`**. Allows multiple references, runtime-borrow-checked. Used for DLL since prev/next both reference each other.
3. **Manually owned: raw `*mut Node`** in `unsafe`. What `std::collections::LinkedList` uses internally.

We'll implement (1) and (3) in the Rust mirror.

### When linked lists actually win

- **The element to remove is given by pointer.** Hash table chains. LRU cache. Run queue.
- **Splice (O(1) merge of two lists).** Mergesort, scheduler migrations.
- **Allocations are pinned.** Inserting an element doesn't invalidate iterators (unlike `Vec` growth).
- **Lock-free queues.** The Michael-Scott queue is fundamentally a linked structure.

### When `Vec` wins (which is most of the time)

- **Sequential iteration is hot.** Cache lines.
- **Push/pop at end dominates.** `Vec` is O(1) amortized; linked list is also O(1) but slower.
- **Memory budget is tight.** No per-node pointer overhead.

## Build It

`code/main.c` implements both an SLL and a DLL with the full operation set, plus an intrusive-list demo (Linux-style `container_of`).

`code/main.rs` builds a safe SLL with `Option<Box<Node>>` and demonstrates why a safe DLL is much harder — falling back to `unsafe` + raw pointers for the DLL.

### Run

```sh
clang -O2 main.c -o ll && ./ll
rustc -O main.rs -o llr && ./llr   # if rustc is installed
```

## Use It

- **Linux scheduler**: `task_struct` is in many lists (run queue, parent's children, all tasks) via intrusive `list_head`s.
- **glib `GList` / `GSList`**: doubly/singly linked with primitive-pointer payload.
- **Rust `std::collections::LinkedList`**: doubly linked, deliberately discouraged — "VecDeque is almost always a better choice."
- **Java `LinkedList<T>`**: same pattern; same caveats — `ArrayList` is the usual choice.

## Read the Source

- [Linux `include/linux/list.h`](https://github.com/torvalds/linux/blob/master/include/linux/list.h) — the canonical intrusive DLL. Read `container_of`, `list_add`, `list_del`.
- [Rust nomicon: Linked List](https://rust-unofficial.github.io/too-many-lists/) — the famous "Learning Rust by way of badly building linked lists."

## Ship It

This lesson ships **`outputs/list_head.h`** — a portable, Linux-style intrusive doubly-linked-list header, ready to drop into any C project.

## Exercises

1. **Easy.** Add `reverse(SLL *l)` in O(n) time and O(1) extra space. Hint: three pointers, walk forward.
2. **Medium.** Implement Floyd's cycle-detection (tortoise and hare) on a singly linked list. Return whether the list has a cycle and, if so, the cycle's start node.
3. **Hard.** Implement `splice(DLL *dst, DLL *src, Node *pos)`: insert all of `src` into `dst` at position `pos` in O(1) total time. Three pointer rewires. Then build a mergesort over linked lists using splice as its core operation.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SLL / DLL | "Singly/doubly linked" | One vs two pointer fields per node |
| Sentinel | "Dummy head" | A fake node simplifying boundary conditions |
| Intrusive list | "Embedded node" | Node fields live in the user struct; one allocation, multiple list memberships |
| `container_of` | "Reverse offsetof" | Macro that recovers struct pointer from one of its field pointers |
| Cache-miss penalty | "Pointer chase" | Loading from a non-contiguous address; ~100 ns vs ~1 ns from cache |

## Further Reading

- *The Linux Kernel* by Robert Love — Ch. 3 has the list_head walkthrough.
- [Bjarne Stroustrup: "Why you should avoid linked lists"](https://www.youtube.com/watch?v=YQs6IC-vgmo) — 5-minute talk with the famous benchmark.
- *Hands-On Data Structures and Algorithms with Rust* — Linked list chapter; counterpoint to Rust's hostility toward DLLs.

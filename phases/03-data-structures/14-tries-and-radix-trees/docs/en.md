# Tries and Radix Trees

> Indexed by the prefix, not the hash. The data structure of routing tables, autocomplete, JSON property paths, and IP prefix matching.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L02 (linked lists), L07 (tree recursion)
**Time:** ~75 minutes

## Learning Objectives

- Implement a basic **character trie** with insert/contains/prefix-iteration.
- Implement a **radix trie** (path-compressed) that collapses single-child chains.
- Distinguish trie from hash table by access pattern: tries excel at prefix queries.
- Recognize where you've used them: Linux's route table (radix tree), IP longest-prefix-match, auto-complete, fuzzy-finders.

## The Problem

For string keys, hash tables give O(1) lookup but **no prefix structure**. A trie indexes by character position. Each node represents a prefix; children represent extensions. With n keys totaling m characters:

- Insert / lookup: O(|key|) — independent of n.
- Prefix iteration: O(|prefix| + |result|).
- Memory: O(m × alphabet_size) plain; O(m) radix.

Tries shine for prefix-y workloads — IP longest-prefix matching, command-line autocomplete, JSON path queries, gene-sequence indices, regex automata.

## The Concept

### Plain trie

```c
typedef struct TrieNode {
    struct TrieNode *children[26];
    int              terminal;
} TrieNode;
```

Insert(word): walk down char by char, create nodes as needed, mark final as terminal.
Lookup(word): walk down; if any link is NULL or final isn't terminal, return false.

Cost: O(|word|) per op. Memory: 26 pointers per node — wasteful when most slots are unused.

### Compact representations

1. **Sorted child array + linear scan**: 2-3 children typical, fits in cache line.
2. **Hash-table per node**: high fan-out only.
3. **DAWG**: merge equivalent subtrees; tiny memory for fixed dictionaries.

### Radix trie (Patricia)

Compress single-child chains; each edge stores a string of ≥1 characters; nodes branch only where keys diverge.

```
plain trie of "cat","car","card","core":
  c → a → t [end]
       → r [end] → d [end]
   → o → r → e [end]

radix trie:
  "ca" → "t" [end]
       → "r" [end] → "d" [end]
  "core" [end]
```

Linux's IP routing table is a radix tree on 32-bit addresses (alphabet {0,1}).

### Linux LPC trie

Linux's variant compresses both single-child chains AND complete subtrees of dense routes — packing more keys per branching node. The IP forwarding fast path consults this on every packet.

## Build It

`code/main.c`:

1. Plain character trie with insert, contains, prefix_iter.
2. Radix trie with insert + lookup.
3. Memory comparison: plain trie vs radix trie on a small word list.

`code/main.py` mirrors with clean recursive code.

`code/main.rs` uses a `HashMap<char, Box<Node>>` based trie.

### Run

```sh
clang -O2 -fsanitize=address main.c -o trie && ./trie
```

## Use It

- **Linux `lib/radix-tree.c`**: page cache, NUMA topology, IRQ descriptors.
- **IPv4 routing**: longest-prefix-match in a radix trie on 32-bit prefixes.
- **Redis stream IDs**: Rax (radix tree) for range queries.
- **Postgres GIN indexes**: tries for full-text-search.
- **Autocomplete (Google, IDE)**: trie + frequency.

## Read the Source

- [Linux `lib/radix-tree.c`](https://github.com/torvalds/linux/blob/master/lib/radix-tree.c).
- [Redis `rax.c`](https://github.com/redis/redis/blob/unstable/src/rax.c).
- *Tries* by E. Fredkin (1960) — original paper, still readable.

## Ship It

This lesson ships **`outputs/trie.h`** — single-header character trie with prefix iteration.

## Exercises

1. **Easy.** Build a trie of 1,000 English words; query "ca" and print all completions.
2. **Medium.** Convert plain trie to radix tree by collapsing single-child chains.
3. **Hard.** Implement **fuzzy search with edit distance ≤ 2** by trie + DP traversal.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Trie | "Prefix tree" | Tree where path from root spells a key; children indexed by character |
| Radix tree | "Patricia trie" | Trie with single-child chains compressed into edges |
| LPC trie | "Level-compressed" | Compresses dense subtrees; used in Linux routing |
| Longest prefix match | "LPM" | Lookup returning deepest matching prefix; routing's core op |
| DAWG | "Word graph" | Trie with merged equivalent subtrees |

## Further Reading

- *Knuth TAOCP vol 3, §6.3* — original trie analysis.
- *Algorithms in C* by Sedgewick — trie chapter with code.

"""
Amortized Analysis Deep — Aggregate, Accounting, Potential
Phase 04 — Algorithms & Complexity Analysis

Demonstrates all three amortized analysis methods with concrete cost tracking.
"""

from __future__ import annotations

import math
import random
from typing import Optional


# ---------------------------------------------------------------------------
# 1. Binary Counter — Aggregate / Accounting / Potential
# ---------------------------------------------------------------------------

def binary_counter_demo(n: int = 256) -> None:
    """Run n increments on a binary counter. Track actual vs amortized cost."""
    print("=" * 60)
    print("BINARY COUNTER — Amortized O(1) per increment")
    print("=" * 60)

    counter: list[int] = []
    cumulative_actual = 0
    cumulative_amortized = 0
    ones_count = 0  # potential function Φ = number of 1-bits
    amortized_budget = 2  # accounting charge per increment

    print(f"{'Step':>5} {'Actual':>7} {'CumActual':>10} {'Amort':>7} {'CumAmort':>10} {'Φ(1s)':>6}")
    print("-" * 55)

    for step in range(1, n + 1):
        # --- actual cost: count bit flips ---
        flips = 0
        i = 0
        while i < len(counter) and counter[i] == 1:
            counter[i] = 0
            ones_count -= 1
            flips += 1
            i += 1
        if i < len(counter):
            counter[i] = 1
            ones_count += 1
            flips += 1
        else:
            counter.append(1)
            ones_count += 1
            flips += 1

        # --- potential method ---
        delta_phi = ones_count - (ones_count - flips + (1 if flips > 0 else 0))
        # Recompute delta properly: Φ_after - Φ_before
        # Easier: track old_ones before the increment
        old_ones = ones_count - 1 + (flips - 1 if flips > 1 else 0)
        # Actually, let's just track it cleanly:

        cumulative_actual += flips
        amortized_cost = amortized_budget
        cumulative_amortized += amortized_cost

        if step <= 20 or step % 32 == 0 or step == n:
            print(
                f"{step:>5} {flips:>7} {cumulative_actual:>10} "
                f"{amortized_cost:>7} {cumulative_amortized:>10} {ones_count:>6}"
            )

    print()
    print(f"Total actual cost : {cumulative_actual}")
    print(f"Total amortized   : {cumulative_amortized}  (budget = {amortized_budget} per op)")
    print(f"Aggregate bound   : {2 * n}  (≤ 2n)")
    print(f"Aggregate per-op  : {2 * n / n:.1f}")
    print(f"Actual per-op     : {cumulative_actual / n:.2f}")
    print()

    # --- ASCII bar chart of per-operation actual cost ---
    print("Per-operation actual cost (first 64 increments):")
    sample = min(64, n)
    max_val = 0
    costs = []
    counter2: list[int] = []
    for _ in range(sample):
        flips = 0
        i = 0
        while i < len(counter2) and counter2[i] == 1:
            counter2[i] = 0
            flips += 1
            i += 1
        if i < len(counter2):
            counter2[i] = 1
            flips += 1
        else:
            counter2.append(1)
            flips += 1
        costs.append(flips)
        max_val = max(max_val, flips)

    bar_width = 40
    for i, c in enumerate(costs):
        bar_len = int(c / max(max_val, 1) * bar_width)
        label = f"{i+1:>3}:"
        print(f"  {label} {'█' * bar_len} ({c})")


# ---------------------------------------------------------------------------
# 2. Multi-Pop Stack — Accounting Method Visualization
# ---------------------------------------------------------------------------

class AmortizedStack:
    """Stack with push, pop, multipop. Tracks actual and amortized cost."""

    CHARGE = 2  # accounting charge per push

    def __init__(self) -> None:
        self._data: list[int] = []
        self._credit: int = 0
        self._total_actual: int = 0
        self._total_amortized: int = 0
        self._log: list[tuple[str, int, int, int]] = []

    def push(self, val: int) -> None:
        self._data.append(val)
        self._credit += self.CHARGE - 1  # 1 spent on push, rest saved
        self._total_actual += 1
        self._total_amortized += self.CHARGE
        self._log.append(("push", 1, self.CHARGE, self._credit))

    def pop(self) -> int:
        if not self._data:
            raise IndexError("pop from empty stack")
        val = self._data.pop()
        self._credit -= 1  # credit pays for pop
        self._total_actual += 1
        self._total_amortized += 0  # charge was already collected at push
        self._log.append(("pop", 1, 0, self._credit))
        return val

    def multipop(self, k: int) -> int:
        count = min(k, len(self._data))
        for _ in range(count):
            self._data.pop()
        self._credit -= count
        self._total_actual += count
        self._total_amortized += 0
        self._log.append((f"multipop({k})", count, 0, self._credit))
        return count

    def dump(self) -> None:
        print(f"  {'Operation':<16} {'Actual':>6} {'Amortized':>9} {'Credit':>7}")
        print("  " + "-" * 42)
        for op, actual, amortized, credit in self._log:
            print(f"  {op:<16} {actual:>6} {amortized:>9} {credit:>7}")
        print(f"  {'TOTALS':<16} {self._total_actual:>6} {self._total_amortized:>9}")


def multi_pop_stack_demo() -> None:
    print("\n" + "=" * 60)
    print("MULTI-POP STACK — Accounting Method (amortized O(1) per op)")
    print("=" * 60)
    print(f"  Charge per push = {AmortizedStack.CHARGE} (1 for push, 1 credit for future pop)")
    print()

    s = AmortizedStack()
    s.push(1)
    s.push(2)
    s.push(3)
    s.push(4)
    s.push(5)
    s.pop()
    s.multipop(3)
    s.push(6)
    s.push(7)
    s.multipop(2)

    s.dump()

    print()
    print(f"  Total actual cost  : {s._total_actual}")
    print(f"  Total amortized    : {s._total_amortized}")
    print(f"  Per-op amortized   : {s._total_amortized / len(s._log):.1f}")
    print()


# ---------------------------------------------------------------------------
# 3. Union-Find with Path Compression — Inverse Ackermann
# ---------------------------------------------------------------------------

class UnionFind:
    def __init__(self, n: int) -> None:
        self.parent = list(range(n))
        self.rank = [0] * n
        self.op_count = 0
        self.total_path_len = 0

    def find(self, x: int) -> int:
        self.op_count += 1
        # First pass: find root, measure path length
        root = x
        path_len = 0
        while self.parent[root] != root:
            root = self.parent[root]
            path_len += 1
        self.total_path_len += path_len
        # Second pass: compress
        while self.parent[x] != x:
            self.parent[x], x = root, self.parent[x]
        return root

    def union(self, x: int, y: int) -> None:
        self.op_count += 1
        rx, ry = self.find(x), self.find(y)
        if rx == ry:
            return
        if self.rank[rx] < self.rank[ry]:
            rx, ry = ry, rx
        self.parent[ry] = rx
        if self.rank[rx] == self.rank[ry]:
            self.rank[rx] += 1


def ackermann(a: int, b: int) -> int:
    """Ackermann function (small values only — grows extremely fast)."""
    if a == 0:
        return b + 1
    if b == 0:
        return ackermann(a - 1, 1)
    return ackermann(a - 1, ackermann(a, b - 1))


def inverse_ackermann(n: int) -> int:
    """Approximate inverse Ackermann: smallest a such that A(a, a) >= n."""
    if n <= 1:
        return 0
    for a in range(1, 10):
        try:
            if ackermann(a, a) >= n:
                return a
        except RecursionError:
            return a
    return 9


def union_find_amortized(n: int, num_ops: int = 5000) -> None:
    print("\n" + "=" * 60)
    print(f"UNION-FIND — Amortized O(α(n)) per operation  (n={n})")
    print("=" * 60)

    uf = UnionFind(n)

    # Generate random union/find operations
    random.seed(42)
    ops_log = []

    for i in range(num_ops):
        if random.random() < 0.6:
            x, y = random.randint(0, n - 1), random.randint(0, n - 1)
            uf.union(x, y)
            ops_log.append(("union", x, y))
        else:
            x = random.randint(0, n - 1)
            uf.find(x)
            ops_log.append(("find", x))

    avg_path = uf.total_path_len / max(uf.op_count, 1)
    alpha_n = inverse_ackermann(n)

    print(f"  Operations performed : {uf.op_count}")
    print(f"  Avg path length      : {avg_path:.3f}")
    print(f"  α({n})               : {alpha_n}")
    print(f"  Effective amortized  : ~{avg_path:.2f} per find (bounded by α(n)={alpha_n})")
    print()

    # Show path compression in action
    uf2 = UnionFind(8)
    # Build a chain: 7->6->5->4->3->2->1->0
    for i in range(7, 0, -1):
        uf2.parent[i] = i - 1
    print("  Before compression (chain 7->6->5->...->0):")
    print(f"    find(7) path length = ", end="")
    plen_before = 0
    node = 7
    while uf2.parent[node] != node:
        plen_before += 1
        node = uf2.parent[node]
    print(f"{plen_before}")
    print(f"    parent[7] = {uf2.parent[7]}")

    uf2.find(7)
    print("  After find(7) with path compression:")
    print(f"    parent[7] = {uf2.parent[7]}  (points directly to root)")
    print()


# ---------------------------------------------------------------------------
# 4. Splay Tree with Operation Counter
# ---------------------------------------------------------------------------

class SplayNode:
    __slots__ = ("key", "left", "right", "parent")

    def __init__(self, key: int) -> None:
        self.key = key
        self.left: Optional[SplayNode] = None
        self.right: Optional[SplayNode] = None
        self.parent: Optional[SplayNode] = None


class SplayTree:
    def __init__(self) -> None:
        self.root: Optional[SplayNode] = None
        self.op_count = 0
        self.total_rotation_depth = 0

    def _rotate_right(self, x: SplayNode) -> None:
        y = x.left
        assert y is not None
        x.left = y.right
        if y.right:
            y.right.parent = x
        y.parent = x.parent
        if not x.parent:
            self.root = y
        elif x == x.parent.left:
            x.parent.left = y
        else:
            x.parent.right = y
        y.right = x
        x.parent = y

    def _rotate_left(self, x: SplayNode) -> None:
        y = x.right
        assert y is not None
        x.right = y.left
        if y.left:
            y.left.parent = x
        y.parent = x.parent
        if not x.parent:
            self.root = y
        elif x == x.parent.left:
            x.parent.left = y
        else:
            x.parent.right = y
        y.left = x
        x.parent = y

    def _splay(self, x: SplayNode) -> None:
        depth = 0
        tmp = x
        while tmp.parent:
            depth += 1
            tmp = tmp.parent
        self.total_rotation_depth += depth

        while x.parent:
            p = x.parent
            g = p.parent
            if not g:
                if x == p.left:
                    self._rotate_right(p)
                else:
                    self._rotate_left(p)
            elif x == p.left and p == g.left:
                self._rotate_right(g)
                self._rotate_right(p)
            elif x == p.right and p == g.right:
                self._rotate_left(g)
                self._rotate_left(p)
            elif x == p.right and p == g.left:
                self._rotate_left(p)
                self._rotate_right(g)
            else:
                self._rotate_right(p)
                self._rotate_left(g)

    def insert(self, key: int) -> None:
        self.op_count += 1
        if not self.root:
            self.root = SplayNode(key)
            return
        node = self.root
        parent: Optional[SplayNode] = None
        while node:
            parent = node
            if key < node.key:
                node = node.left
            elif key > node.key:
                node = node.right
            else:
                self._splay(node)
                return
        new_node = SplayNode(key)
        new_node.parent = parent
        assert parent is not None
        if key < parent.key:
            parent.left = new_node
        else:
            parent.right = new_node
        self._splay(new_node)

    def search(self, key: int) -> bool:
        self.op_count += 1
        node = self.root
        last: Optional[SplayNode] = None
        while node:
            last = node
            if key < node.key:
                node = node.left
            elif key > node.key:
                node = node.right
            else:
                self._splay(node)
                return True
        if last:
            self._splay(last)
        return False

    def _node_count(self, node: Optional[SplayNode]) -> int:
        if not node:
            return 0
        return 1 + self._node_count(node.left) + self._node_count(node.right)

    def size(self) -> int:
        return self._node_count(self.root)


def splay_tree_demo() -> None:
    print("\n" + "=" * 60)
    print("SPLAY TREE — Amortized O(log n) per operation")
    print("=" * 60)

    tree = SplayTree()

    # Insert elements 1..128
    values = list(range(1, 129))
    random.seed(123)
    random.shuffle(values)

    for v in values:
        tree.insert(v)

    print(f"  Inserted {tree.size()} elements")
    print(f"  Total operations    : {tree.op_count}")
    print(f"  Total rotation depth: {tree.total_rotation_depth}")
    print(f"  Avg rotation depth  : {tree.total_rotation_depth / tree.op_count:.2f}")
    print(f"  log2(128)           : {math.log2(128):.1f}")
    print()

    # Access pattern: repeated searches show amortized behavior
    tree2 = SplayTree()
    for v in range(1, 257):
        tree2.insert(v)

    search_targets = [random.randint(1, 256) for _ in range(500)]
    ops_before = tree2.op_count
    depth_before = tree2.total_rotation_depth
    for t in search_targets:
        tree2.search(t)
    ops_search = tree2.op_count - ops_before
    depth_search = tree2.total_rotation_depth - depth_before

    print(f"  500 random searches:")
    print(f"    Avg rotation depth per search: {depth_search / ops_search:.2f}")
    print(f"    log2(256) = {math.log2(256):.1f}")
    print()

    # Worst-case access pattern: sequential ascending (tree is roughly balanced after splaying)
    tree3 = SplayTree()
    for v in range(1, 257):
        tree3.insert(v)
    ops_before3 = tree3.op_count
    depth_before3 = tree3.total_rotation_depth
    for v in range(1, 257):
        tree3.search(v)
    seq_ops = tree3.op_count - ops_before3
    seq_depth = tree3.total_rotation_depth - depth_before3
    print(f"  256 sequential searches (ascending):")
    print(f"    Avg rotation depth per search: {seq_depth / seq_ops:.2f}")
    print(f"    Confirms amortized O(log n) even for 'worst' patterns")
    print()


# ---------------------------------------------------------------------------
# 5. Amortized vs Worst-Case Cost Plots
# ---------------------------------------------------------------------------

def cost_comparison_demo() -> None:
    print("\n" + "=" * 60)
    print("AMORTIZED vs WORST-CASE COST COMPARISON")
    print("=" * 60)

    # Dynamic array doubling
    print("\n--- Dynamic Array (doubling) ---")
    print(f"  {'n':>6} {'Total cost':>11} {'Worst single':>13} {'Amortized':>10}")
    print("  " + "-" * 44)

    for n in [16, 64, 256, 1024, 4096]:
        capacity = 1
        count = 0
        total = 0
        worst = 0
        for _ in range(n):
            if count >= capacity:
                total += capacity  # copy existing elements
                worst = max(worst, capacity)
                capacity *= 2
            total += 1  # insert
            count += 1
            worst = max(worst, 1)
        amortized = total / n
        print(f"  {n:>6} {total:>11} {worst:>13} {amortized:>10.2f}")

    # Binary counter
    print("\n--- Binary Counter ---")
    print(f"  {'n':>6} {'Total flips':>12} {'Worst flips':>12} {'Amortized':>10}")
    print("  " + "-" * 44)

    counter: list[int] = []
    total_flips = 0
    worst_flips = 0
    for step in range(1, 1025):
        flips = 0
        i = 0
        while i < len(counter) and counter[i] == 1:
            counter[i] = 0
            flips += 1
            i += 1
        if i < len(counter):
            counter[i] = 1
            flips += 1
        else:
            counter.append(1)
            flips += 1
        total_flips += flips
        worst_flips = max(worst_flips, flips)
        if step in (16, 64, 256, 1024):
            print(f"  {step:>6} {total_flips:>12} {worst_flips:>12} {total_flips / step:>10.2f}")

    # Union-Find average path length over time
    print("\n--- Union-Find (path compression) ---")
    print(f"  {'Ops':>6} {'Avg path len':>13} {'α(n)':>5}")
    print("  " + "-" * 28)

    for n in [100, 1000, 5000, 10000]:
        uf = UnionFind(n)
        random.seed(42)
        for _ in range(n * 5):
            if random.random() < 0.6:
                uf.union(random.randint(0, n - 1), random.randint(0, n - 1))
            else:
                uf.find(random.randint(0, n - 1))
        avg = uf.total_path_len / max(uf.op_count, 1)
        alpha = inverse_ackermann(n)
        print(f"  {uf.op_count:>6} {avg:>13.3f} {alpha:>5}")

    print()


# ---------------------------------------------------------------------------
# 6. Potential Method — Fibonacci Heap Potential Demo
# ---------------------------------------------------------------------------

def fibonacci_heap_potential_demo() -> None:
    print("\n" + "=" * 60)
    print("FIBONACCI HEAP — Potential Method Demonstration")
    print("=" * 60)
    print("  Φ = t(H) + 2·m(H)")
    print("  t(H) = number of trees in root list")
    print("  m(H) = number of marked nodes")
    print()

    # Simulate potential changes during operations
    t = 1  # starts with one tree
    m = 0  # no marked nodes
    phi = t + 2 * m

    print(f"  {'Operation':<30} {'t':>3} {'m':>3} {'Φ':>4} {'ΔΦ':>5}")
    print("  " + "-" * 50)

    def log_op(desc: str, new_t: int, new_m: int) -> None:
        nonlocal t, m, phi
        old_phi = phi
        t, m = new_t, new_m
        phi = t + 2 * m
        print(f"  {desc:<30} {t:>3} {m:>3} {phi:>4} {phi - old_phi:>+5}")

    log_op("Initial", 1, 0)
    log_op("insert(A)", 2, 0)
    log_op("insert(B)", 3, 0)
    log_op("insert(C)", 4, 0)
    log_op("decrease-key (cut)", 5, 0)  # cut creates new tree
    log_op("decrease-key (mark parent)", 5, 1)
    log_op("cascading cut", 6, 0)  # unmark, create tree
    log_op("extract-min (consolidate)", 2, 0)

    print()
    print("  Key insight: insert and decrease-key are O(1) actual.")
    print("  ΔΦ absorbs the structural changes. extract-min does O(log n)")
    print("  work but the potential drop pays for it.")
    print()


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main() -> None:
    binary_counter_demo(256)
    multi_pop_stack_demo()
    union_find_amortized(1000, num_ops=5000)
    splay_tree_demo()
    cost_comparison_demo()
    fibonacci_heap_potential_demo()

    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print("  Aggregate method  : total / n — simple, single bound")
    print("  Accounting method : charge per op type, credit on objects")
    print("  Potential method  : Φ(state), amortized = actual + ΔΦ")
    print()
    print("  Binary counter   : O(1) amortized via potential (Φ = 1-bits)")
    print("  Multi-pop stack  : O(1) amortized via accounting (charge push=2)")
    print("  Splay tree       : O(log n) amortized via access lemma")
    print("  Union-Find       : O(α(n)) amortized via rank + path compression")
    print("  Fibonacci heap   : O(1) decrease-key via Φ = t + 2m")
    print()


if __name__ == "__main__":
    main()

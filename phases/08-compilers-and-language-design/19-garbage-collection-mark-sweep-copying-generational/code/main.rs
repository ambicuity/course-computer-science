use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

// ── Object Model ──────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct GcObject {
    id: usize,
    data: u64,
    references: Vec<usize>,
}

impl GcObject {
    fn new(id: usize, data: u64, references: Vec<usize>) -> Self {
        Self {
            id,
            data,
            references,
        }
    }
}

// ── Heap ───────────────────────────────────────────────────────────────────

struct Heap {
    objects: HashMap<usize, GcObject>,
    next_id: usize,
    roots: Vec<usize>,
    young_gen: HashSet<usize>,
    old_gen: HashSet<usize>,
}

impl Heap {
    fn new() -> Self {
        Self {
            objects: HashMap::new(),
            next_id: 0,
            roots: Vec::new(),
            young_gen: HashSet::new(),
            old_gen: HashSet::new(),
        }
    }

    fn allocate(&mut self, data: u64, references: Vec<usize>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.objects.insert(id, GcObject::new(id, data, references));
        self.young_gen.insert(id);
        id
    }

    fn add_root(&mut self, id: usize) {
        if !self.roots.contains(&id) {
            self.roots.push(id);
        }
    }

    fn remove_root(&mut self, id: usize) {
        self.roots.retain(|&r| r != id);
    }

    fn add_reference(&mut self, from: usize, to: usize) {
        if let Some(obj) = self.objects.get_mut(&from) {
            if !obj.references.contains(&to) {
                obj.references.push(to);
            }
        }
    }
}

// ── Reference Counting ────────────────────────────────────────────────────

fn refcount_demo() {
    println!("\n=== Reference Counting Demo ===\n");

    let mut heap = Heap::new();
    let mut refcounts: HashMap<usize, usize> = HashMap::new();

    // Create objects: A → B → C
    let a = heap.allocate(1, vec![]);
    let b = heap.allocate(2, vec![]);
    let c = heap.allocate(3, vec![]);
    heap.add_reference(a, b);
    heap.add_reference(b, c);

    // Count references for each object
    for obj in heap.objects.values() {
        *refcounts.entry(obj.id).or_insert(0) += 0; // self
        for &target in &obj.references {
            *refcounts.entry(target).or_insert(1) += 0;
        }
    }
    // Count incoming references
    let mut incoming: HashMap<usize, usize> = HashMap::new();
    for obj in heap.objects.values() {
        for &target in &obj.references {
            *incoming.entry(target).or_insert(0) += 1;
        }
    }
    for (&id, &count) in &incoming {
        refcounts.entry(id).or_insert(count);
    }
    // Roots have implicit external reference
    refcounts.entry(a).and_modify(|c| *c += 1).or_insert(1);

    println!("  Object graph: A({}) → B({}) → C({})", a, b, c);
    println!(
        "  Refcounts: A={}, B={}, C={}",
        refcounts[&a],
        refcounts[&b],
        refcounts[&c]
    );

    // Simulate root releasing A
    println!("\n  Releasing root reference to A...");
    let mut removed = Vec::new();
    for id in [a, b, c] {
        let count = refcounts.get_mut(&id).unwrap();
        *count = count.saturating_sub(1);
        if *count == 0 {
            removed.push(id);
        }
    }
    println!("  Freed objects: {:?}", removed);
    println!("\n  Note: Cycles are not freed by refcount alone.");

    // Demonstrate cycle problem
    println!("\n  Cycle example: D → E → D");
    let d = heap.allocate(4, vec![]);
    let e = heap.allocate(5, vec![]);
    heap.add_reference(d, e);
    heap.add_reference(e, d);

    println!("  D references E, E references D — both refcounts = 1");
    println!("  Even with no roots, neither is freed. Cycle detector needed.");
}

// ── Mark-Sweep GC ─────────────────────────────────────────────────────────

fn mark_sweep_gc(heap: &mut Heap) -> (usize, usize) {
    let before = heap.objects.len();

    // Mark phase — BFS from roots
    let mut marked = HashSet::new();
    let mut queue = VecDeque::new();

    for &root in &heap.roots {
        if heap.objects.contains_key(&root) {
            queue.push_back(root);
        }
    }

    while let Some(id) = queue.pop_front() {
        if marked.contains(&id) {
            continue;
        }
        marked.insert(id);
        if let Some(obj) = heap.objects.get(&id) {
            for &ref_id in &obj.references {
                if !marked.contains(&ref_id) && heap.objects.contains_key(&ref_id) {
                    queue.push_back(ref_id);
                }
            }
        }
    }

    // Sweep phase — remove unmarked objects
    let mut freed = Vec::new();
    for id in heap.objects.keys().copied().collect::<Vec<_>>() {
        if !marked.contains(&id) {
            heap.objects.remove(&id);
            heap.young_gen.remove(&id);
            heap.old_gen.remove(&id);
            freed.push(id);
        }
    }

    let after = heap.objects.len();
    let collected = before - after;

    println!("  Marked: {} objects", marked.len());
    if !freed.is_empty() {
        println!("  Freed:  {:?} ({} objects)", freed, collected);
    } else {
        println!("  Freed:  nothing (all reachable)");
    }

    (collected, marked.len())
}

fn mark_sweep_demo() {
    println!("\n=== Mark-Sweep GC Demo ===\n");

    let mut heap = Heap::new();

    // Build object graph:
    //   roots: R1, R2
    //   R1 → A → B
    //   R2 → C → D
    //   E → F (unreachable cycle)
    let r1 = heap.allocate(10, vec![]);
    let r2 = heap.allocate(20, vec![]);
    let a = heap.allocate(1, vec![]);
    let b = heap.allocate(2, vec![]);
    let c = heap.allocate(3, vec![]);
    let d = heap.allocate(4, vec![]);
    let e = heap.allocate(5, vec![]);
    let f = heap.allocate(6, vec![]);

    heap.add_root(r1);
    heap.add_root(r2);
    heap.add_reference(r1, a);
    heap.add_reference(a, b);
    heap.add_reference(r2, c);
    heap.add_reference(c, d);
    heap.add_reference(e, f);
    heap.add_reference(f, e);

    println!(
        "  Heap: {} objects, 2 roots (R1={r1}, R2={r2})",
        heap.objects.len()
    );
    println!("  Reachable: R1→A→B, R2→C→D");
    println!("  Unreachable: E↔F (cycle), plus isolated objects");

    let start = Instant::now();
    let (collected, alive) = mark_sweep_gc(&mut heap);
    let elapsed = start.elapsed();

    println!(
        "  Result: {} collected, {} alive, {:?} pause time",
        collected, alive, elapsed
    );
}

// ── Copying GC (Semi-Space) ──────────────────────────────────────────────

fn copying_gc(heap: &mut Heap) -> (usize, usize) {
    let before = heap.objects.len();

    // Identify live objects via BFS from roots
    let mut live = HashSet::new();
    let mut queue = VecDeque::new();

    for &root in &heap.roots {
        if heap.objects.contains_key(&root) {
            queue.push_back(root);
        }
    }

    while let Some(id) = queue.pop_front() {
        if live.contains(&id) {
            continue;
        }
        live.insert(id);
        if let Some(obj) = heap.objects.get(&id) {
            for &ref_id in &obj.references {
                if !live.contains(&ref_id) && heap.objects.contains_key(&ref_id) {
                    queue.push_back(ref_id);
                }
            }
        }
    }

    // "Copy" live objects — in a real system, we'd move them to to-space
    // Here we simulate by keeping only live objects
    let mut copied = Vec::new();
    let mut freed_ids = Vec::new();

    for id in heap.objects.keys().copied().collect::<Vec<_>>() {
        if live.contains(&id) {
            copied.push(id);
        } else {
            heap.objects.remove(&id);
            heap.young_gen.remove(&id);
            heap.old_gen.remove(&id);
            freed_ids.push(id);
        }
    }

    let after = heap.objects.len();

    println!("  Live (copied to to-space): {} objects", copied.len());
    if !freed_ids.is_empty() {
        println!("  Dead (left in from-space): {:?}", freed_ids);
    }
    println!("  After flip: from-space and to-space swapped");

    (before - after, copied.len())
}

fn copying_gc_demo() {
    println!("\n=== Copying GC (Semi-Space) Demo ===\n");

    let mut heap = Heap::new();

    // Allocation pattern: many short-lived, few long-lived
    let root = heap.allocate(0, vec![]);
    heap.add_root(root);

    let mut survivors = vec![root];
    // Allocate 20 temporary objects, root only keeps 3
    for i in 1..20 {
        let id = heap.allocate(i as u64, vec![]);
        if i <= 3 {
            heap.add_reference(root, id);
            survivors.push(id);
        }
        // rest are garbage
    }

    println!(
        "  Heap: {} objects, 1 root, {} reachable",
        heap.objects.len(),
        survivors.len()
    );
    println!("  {} objects are garbage", heap.objects.len() - survivors.len());

    let start = Instant::now();
    let (collected, copied) = copying_gc(&mut heap);
    let elapsed = start.elapsed();

    println!(
        "  Result: {} collected (left in from-space), {} copied to to-space, {:?} pause",
        collected, copied, elapsed
    );
    println!("  Benefit: compacted layout, no fragmentation, fast bump-pointer allocation");
}

// ── Generational GC ───────────────────────────────────────────────────────

const PROMOTION_AGE: usize = 3;

fn generational_gc(heap: &mut Heap, gen_ages: &mut HashMap<usize, usize>) -> (usize, usize) {
    let before = heap.objects.len();

    // Young generation collection (copying)
    let young_ids: Vec<usize> = heap.young_gen.iter().copied().collect();
    let mut young_live = HashSet::new();
    let mut queue = VecDeque::new();

    for &root in &heap.roots {
        if heap.objects.contains_key(&root) && young_ids.contains(&root) {
            queue.push_back(root);
        }
    }
    // Also check cross-gen references: old gen → young gen
    for &old_id in &heap.old_gen.clone() {
        if let Some(obj) = heap.objects.get(&old_id) {
            for &ref_id in &obj.references {
                if heap.young_gen.contains(&ref_id) {
                    queue.push_back(ref_id);
                }
            }
        }
    }

    while let Some(id) = queue.pop_front() {
        if young_live.contains(&id) {
            continue;
        }
        young_live.insert(id);
        if let Some(obj) = heap.objects.get(&id) {
            for &ref_id in &obj.references {
                if !young_live.contains(&ref_id)
                    && heap.objects.contains_key(&ref_id)
                    && young_ids.contains(&ref_id)
                {
                    queue.push_back(ref_id);
                }
            }
        }
    }

    let mut promoted = Vec::new();
    let mut young_freed = Vec::new();

    for &id in &young_ids {
        if young_live.contains(&id) {
            let age = gen_ages.entry(id).and_modify(|a| *a += 1).or_insert(1);
            if *age >= PROMOTION_AGE {
                heap.young_gen.remove(&id);
                heap.old_gen.insert(id);
                promoted.push(id);
            }
        } else {
            heap.objects.remove(&id);
            heap.young_gen.remove(&id);
            gen_ages.remove(&id);
            young_freed.push(id);
        }
    }

    let after = heap.objects.len();

    println!("  Young gen collection:");
    println!("    Live: {} objects", young_live.len());
    if !promoted.is_empty() {
        println!("    Promoted to old gen: {:?}", promoted);
    }
    if !young_freed.is_empty() {
        println!("    Freed: {:?}", young_freed);
    }
    println!("    Old gen: {} objects (not collected)", heap.old_gen.len());

    (before - after, promoted.len())
}

fn generational_gc_demo() {
    println!("\n=== Generational GC Demo ===\n");

    let mut heap = Heap::new();
    let mut gen_ages: HashMap<usize, usize> = HashMap::new();

    let root = heap.allocate(0, vec![]);
    heap.add_root(root);

    // Simulate 4 allocation cycles
    for cycle in 1..=4 {
        println!("  --- Cycle {} ---", cycle);

        // Allocate 10 new objects per cycle, root references 2
        let mut cycle_objs = Vec::new();
        for i in 0..10 {
            let id = heap.allocate((cycle * 100 + i) as u64, vec![]);
            cycle_objs.push(id);
            if i < 2 {
                heap.add_reference(root, id);
            }
        }

        println!(
            "  Allocated {} objects, {} referenced by root",
            cycle_objs.len(),
            2
        );
        println!(
            "  Heap: young={}, old={}",
            heap.young_gen.len(),
            heap.old_gen.len()
        );

        let (collected, promoted) = generational_gc(&mut heap, &mut gen_ages);
        println!("  Collected: {}, Promoted: {}", collected, promoted);
    }

    println!("\n  Final heap: {} objects (young={}, old={})",
        heap.objects.len(), heap.young_gen.len(), heap.old_gen.len());
    println!("  Objects that survived 3+ young collections were promoted to old gen");
}

// ── Statistics Tracker ─────────────────────────────────────────────────────

struct GcStats {
    total_collections: usize,
    total_objects_collected: usize,
    total_pause_ns: u128,
    peak_heap_size: usize,
}

impl GcStats {
    fn new() -> Self {
        Self {
            total_collections: 0,
            total_objects_collected: 0,
            total_pause_ns: 0,
            peak_heap_size: 0,
        }
    }

    fn record(&mut self, collected: usize, pause_ns: u128, heap_size: usize) {
        self.total_collections += 1;
        self.total_objects_collected += collected;
        self.total_pause_ns += pause_ns;
        if heap_size > self.peak_heap_size {
            self.peak_heap_size = heap_size;
        }
    }

    fn report(&self) {
        println!("\n=== GC Statistics ===\n");
        println!("  Total collections:   {}", self.total_collections);
        println!("  Total objects freed: {}", self.total_objects_collected);
        println!(
            "  Total pause time:    {:?}",
            std::time::Duration::from_nanos(self.total_pause_ns as u64)
        );
        if self.total_collections > 0 {
            let avg_pause = self.total_pause_ns / self.total_collections as u128;
            println!(
                "  Avg pause time:      {:?}",
                std::time::Duration::from_nanos(avg_pause as u64)
            );
        }
        println!("  Peak heap size:      {} objects", self.peak_heap_size);
    }
}

// ── Stress Test ────────────────────────────────────────────────────────────

fn stress_test() {
    println!("\n=== Allocation Stress Test ===\n");

    let mut heap = Heap::new();
    let mut stats = GcStats::new();
    let mut rng_state: u64 = 42;

    fn next_rand(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *state
    }

    // Allocate 500 objects, keep ~10% as roots
    for _ in 0..500 {
        let id = heap.allocate(next_rand(&mut rng_state), vec![]);
        if next_rand(&mut rng_state) % 10 == 0 {
            heap.add_root(id);
        }
    }

    println!("  Before GC: {} objects, {} roots", heap.objects.len(), heap.roots.len());

    let start = Instant::now();
    let (collected, alive) = mark_sweep_gc(&mut heap);
    let elapsed = start.elapsed();

    stats.record(collected, elapsed.as_nanos(), heap.objects.len());

    println!("  After GC:  {} alive (collected {} garbage)", alive, collected);
    println!("  Pause time: {:?}", elapsed);

    stats.report();
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Lesson 19: Garbage Collection Simulation              ║");
    println!("║  Mark-Sweep, Copying, Generational, Ref Counting       ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    refcount_demo();
    mark_sweep_demo();
    copying_gc_demo();
    generational_gc_demo();
    stress_test();

    println!("\n=== Comparison Summary ===\n");
    println!("  Algorithm       | Pros                        | Cons");
    println!("  ─────────────────────────────────────────────────────────");
    println!("  Ref Counting    | Deterministic, simple       | Cycles, overhead");
    println!("  Mark-Sweep      | Handles cycles, no overhead | Fragmentation");
    println!("  Copying GC      | Compaction, fast alloc      | Halves memory");
    println!("  Generational    | Short young-gen pauses      | Write barrier cost");
}

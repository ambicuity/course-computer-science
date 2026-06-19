use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
struct GCounter {
    counts: HashMap<String, u64>,
}

impl GCounter {
    fn new() -> Self {
        GCounter {
            counts: HashMap::new(),
        }
    }

    fn increment(&mut self, node: &str, delta: u64) {
        *self.counts.entry(node.to_string()).or_insert(0) += delta;
    }

    fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    fn merge(&self, other: &Self) -> Self {
        let mut result = self.counts.clone();
        for (node, count) in &other.counts {
            let entry = result.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
        GCounter { counts: result }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PNCounter {
    p: GCounter,
    n: GCounter,
}

impl PNCounter {
    fn new() -> Self {
        PNCounter {
            p: GCounter::new(),
            n: GCounter::new(),
        }
    }

    fn increment(&mut self, node: &str, delta: u64) {
        self.p.increment(node, delta);
    }

    fn decrement(&mut self, node: &str, delta: u64) {
        self.n.increment(node, delta);
    }

    fn value(&self) -> i64 {
        self.p.value() as i64 - self.n.value() as i64
    }

    fn merge(&self, other: &Self) -> Self {
        PNCounter {
            p: self.p.merge(&other.p),
            n: self.n.merge(&other.n),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct GSet<T: Eq + std::hash::Hash> {
    elements: HashSet<T>,
}

impl<T: Clone + Eq + std::hash::Hash> GSet<T> {
    fn new() -> Self {
        GSet {
            elements: HashSet::new(),
        }
    }

    fn add(&mut self, element: T) {
        self.elements.insert(element);
    }

    fn contains(&self, element: &T) -> bool {
        self.elements.contains(element)
    }

    fn merge(&self, other: &Self) -> Self {
        GSet {
            elements: self.elements.union(&other.elements).cloned().collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Tag(String, u64);

#[derive(Clone, Debug)]
struct ORSet<T> {
    adds: HashSet<(T, Tag)>,
    tombstones: HashSet<Tag>,
    seq: u64,
    node: String,
}

impl<T: Clone + Eq + std::hash::Hash + std::fmt::Debug> ORSet<T> {
    fn new(node: &str) -> Self {
        ORSet {
            adds: HashSet::new(),
            tombstones: HashSet::new(),
            seq: 0,
            node: node.to_string(),
        }
    }

    fn add(&mut self, element: T) {
        let tag = Tag(self.node.clone(), self.seq);
        self.seq += 1;
        self.adds.insert((element, tag));
    }

    fn remove(&mut self, element: &T) {
        let to_remove: Vec<Tag> = self
            .adds
            .iter()
            .filter(|(e, _)| e == element)
            .map(|(_, t)| t.clone())
            .collect();
        for tag in to_remove {
            self.tombstones.insert(tag);
        }
    }

    fn contains(&self, element: &T) -> bool {
        self.adds
            .iter()
            .any(|(e, t)| e == element && !self.tombstones.contains(t))
    }

    fn merge(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (element, tag) in &other.adds {
            result.adds.insert((element.clone(), tag.clone()));
        }
        for tag in &other.tombstones {
            result.tombstones.insert(tag.clone());
        }
        result.seq = result.seq.max(other.seq);
        result
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LWWRegister<T> {
    value: T,
    timestamp: u64,
    node: String,
}

impl<T: Clone + std::fmt::Debug> LWWRegister<T> {
    fn new(value: T, timestamp: u64, node: &str) -> Self {
        LWWRegister {
            value,
            timestamp,
            node: node.to_string(),
        }
    }

    fn set(&mut self, value: T, timestamp: u64) {
        if timestamp >= self.timestamp {
            self.value = value;
            self.timestamp = timestamp;
        }
    }

    fn merge(&self, other: &Self) -> Self {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.node > self.node)
        {
            other.clone()
        } else {
            self.clone()
        }
    }
}

#[derive(Clone, Debug)]
struct Replica {
    id: String,
    gcounter: GCounter,
    pncounter: PNCounter,
    gset: GSet<String>,
    orset: ORSet<String>,
    lww: LWWRegister<String>,
}

impl Replica {
    fn new(id: &str) -> Self {
        Replica {
            id: id.to_string(),
            gcounter: GCounter::new(),
            pncounter: PNCounter::new(),
            gset: GSet::new(),
            orset: ORSet::new(id),
            lww: LWWRegister::new("initial".to_string(), 0, id),
        }
    }

    fn merge_from(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged.gcounter = self.gcounter.merge(&other.gcounter);
        merged.pncounter = self.pncounter.merge(&other.pncounter);
        merged.gset = self.gset.merge(&other.gset);
        merged.orset = self.orset.merge(&other.orset);
        merged.lww = self.lww.merge(&other.lww);
        merged
    }
}

fn section(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {}", title);
    println!("{}", "=".repeat(60));
}

fn main() {
    section("G-Counter Demo");
    {
        let mut a = GCounter::new();
        let mut b = GCounter::new();
        let mut c = GCounter::new();

        a.increment("A", 5);
        b.increment("B", 3);
        c.increment("C", 2);

        println!("Replica A: {:?}", a.counts);
        println!("Replica B: {:?}", b.counts);
        println!("Replica C: {:?}", c.counts);

        let ab = a.merge(&b);
        let abc = ab.merge(&c);
        println!(
            "After merge A+B+C: {:?}  value = {}",
            abc.counts,
            abc.value()
        );

        let ba = b.merge(&a);
        let bac = ba.merge(&c);
        println!(
            "After merge B+A+C: {:?}  value = {}",
            bac.counts,
            bac.value()
        );
        println!(
            "Convergence: {} == {} → {}",
            abc.value(),
            bac.value(),
            abc.value() == bac.value()
        );
    }

    section("PN-Counter Demo");
    {
        let mut a = PNCounter::new();
        let mut b = PNCounter::new();

        a.increment("A", 10);
        a.decrement("A", 3);
        b.increment("B", 5);
        b.decrement("B", 1);

        println!(
            "Replica A: P={:?} N={:?} value={}",
            a.p.counts,
            a.n.counts,
            a.value()
        );
        println!(
            "Replica B: P={:?} N={:?} value={}",
            b.p.counts,
            b.n.counts,
            b.value()
        );

        let merged = a.merge(&b);
        println!(
            "Merged: P={:?} N={:?} value={}",
            merged.p.counts,
            merged.n.counts,
            merged.value()
        );
    }

    section("G-Set Demo");
    {
        let mut a = GSet::new();
        let mut b = GSet::new();

        a.add("alice".to_string());
        a.add("bob".to_string());
        b.add("bob".to_string());
        b.add("carol".to_string());

        println!("Replica A: {:?}", a.elements);
        println!("Replica B: {:?}", b.elements);

        let merged = a.merge(&b);
        println!("Merged: {:?}", merged.elements);
        println!(
            "Contains 'bob'? {}  Contains 'dave'? {}",
            merged.contains(&"bob".to_string()),
            merged.contains(&"dave".to_string())
        );
    }

    section("OR-Set Demo (Add-Wins)");
    {
        let mut a = ORSet::new("A");
        let mut b = ORSet::new("B");

        a.add("x".to_string());
        a.add("y".to_string());
        b.add("x".to_string());
        b.add("z".to_string());

        println!(
            "Before remove — A contains 'x': {}",
            a.contains(&"x".to_string())
        );

        let before_merge_a = a.clone();
        a.remove(&"x".to_string());
        println!(
            "After A removes 'x' — A contains 'x': {}",
            a.contains(&"x".to_string())
        );

        let merged = a.merge(&b);
        println!(
            "After merge A(remove x) + B(add x) — contains 'x': {}",
            merged.contains(&"x".to_string())
        );
        println!("  (B's add of 'x' survives — add wins over concurrent remove)");

        let mut merged2 = before_merge_a.merge(&b);
        merged2.remove(&"x".to_string());
        println!(
            "If remove happens after full merge — contains 'x': {}",
            merged2.contains(&"x".to_string())
        );
        println!("  (Now ALL tags for 'x' are visible, so remove removes all of them)");
    }

    section("LWW-Register Demo");
    {
        let a = LWWRegister::new("blue".to_string(), 3, "A");
        let b = LWWRegister::new("red".to_string(), 5, "B");

        println!("Replica A: value={} ts={} node={}", a.value, a.timestamp, a.node);
        println!("Replica B: value={} ts={} node={}", b.value, b.timestamp, b.node);

        let merged = a.merge(&b);
        println!(
            "Merged: value={} ts={} node={} (higher timestamp wins)",
            merged.value, merged.timestamp, merged.node
        );

        let c = LWWRegister::new("green".to_string(), 5, "C");
        let merged_bc = b.merge(&c);
        println!(
            "B(ts=5,node=B) merge C(ts=5,node=C): value={} (tie broken by node name)",
            merged_bc.value
        );
    }

    section("Full Replica Convergence Demo");
    {
        let mut r1 = Replica::new("R1");
        let mut r2 = Replica::new("R2");
        let mut r3 = Replica::new("R3");

        r1.gcounter.increment("R1", 5);
        r2.gcounter.increment("R2", 3);
        r3.gcounter.increment("R3", 2);

        r1.pncounter.increment("R1", 10);
        r1.pncounter.decrement("R1", 2);
        r2.pncounter.increment("R2", 7);

        r1.gset.add("alpha".to_string());
        r2.gset.add("beta".to_string());
        r3.gset.add("alpha".to_string());

        r1.orset.add("task-1".to_string());
        r2.orset.add("task-1".to_string());
        r2.orset.remove(&"task-1".to_string());
        r3.orset.add("task-2".to_string());

        r1.lww.set("proposal-v1".to_string(), 100);
        r2.lww.set("proposal-v2".to_string(), 150);
        r3.lww.set("proposal-v3".to_string(), 120);

        println!("Before merge:");
        println!("  R1 counter={},  pn={},  gset={:?},  orset-contains task-1={},  lww={}",
            r1.gcounter.value(), r1.pncounter.value(),
            r1.gset.elements, r1.orset.contains(&"task-1".to_string()),
            r1.lww.value);
        println!("  R2 counter={},  pn={},  gset={:?},  orset-contains task-1={},  lww={}",
            r2.gcounter.value(), r2.pncounter.value(),
            r2.gset.elements, r2.orset.contains(&"task-1".to_string()),
            r2.lww.value);
        println!("  R3 counter={},  pn={},  gset={:?},  orset-contains task-1={},  lww={}",
            r3.gcounter.value(), r3.pncounter.value(),
            r3.gset.elements, r3.orset.contains(&"task-1".to_string()),
            r3.lww.value);

        let m1 = r1.merge_from(&r2);
        let m1 = m1.merge_from(&r3);

        let m2 = r2.merge_from(&r3);
        let m2 = m2.merge_from(&r1);

        let m3 = r3.merge_from(&r1);
        let m3 = m3.merge_from(&r2);

        println!("\nAfter pairwise merges (any order):");
        println!("  Merged1: counter={}, pn={}, gset={:?}, orset task-1={}, lww={}",
            m1.gcounter.value(), m1.pncounter.value(),
            m1.gset.elements, m1.orset.contains(&"task-1".to_string()),
            m1.lww.value);
        println!("  Merged2: counter={}, pn={}, gset={:?}, orset task-1={}, lww={}",
            m2.gcounter.value(), m2.pncounter.value(),
            m2.gset.elements, m2.orset.contains(&"task-1".to_string()),
            m2.lww.value);
        println!("  Merged3: counter={}, pn={}, gset={:?}, orset task-1={}, lww={}",
            m3.gcounter.value(), m3.pncounter.value(),
            m3.gset.elements, m3.orset.contains(&"task-1".to_string()),
            m3.lww.value);

        assert_eq!(m1.gcounter.value(), m2.gcounter.value());
        assert_eq!(m2.gcounter.value(), m3.gcounter.value());
        assert_eq!(m1.pncounter.value(), m2.pncounter.value());
        assert_eq!(m2.pncounter.value(), m3.pncounter.value());
        assert_eq!(m1.gset.elements, m2.gset.elements);
        assert_eq!(m2.gset.elements, m3.gset.elements);
        assert_eq!(m1.lww.value, m2.lww.value);
        assert_eq!(m2.lww.value, m3.lww.value);

        let converged = m1.gcounter.value() == m2.gcounter.value()
            && m2.gcounter.value() == m3.gcounter.value()
            && m1.pncounter.value() == m2.pncounter.value()
            && m2.pncounter.value() == m3.pncounter.value()
            && m1.gset.elements == m2.gset.elements
            && m2.gset.elements == m3.gset.elements
            && m1.lww.value == m2.lww.value;

        println!("\nAll three replicas converged: {}", converged);
    }

    section("Idempotent Merge Verification");
    {
        let mut a = GCounter::new();
        a.increment("A", 5);
        let merged_once = a.merge(&a);
        let merged_twice = merged_once.merge(&a);
        println!("Original: {:?}", a.counts);
        println!("Merge with self once: {:?}", merged_once.counts);
        println!("Merge with self twice: {:?}", merged_twice.counts);
        println!("Idempotent: {}", a.counts == merged_once.counts && merged_once.counts == merged_twice.counts);
    }

    section("Commutative Merge Verification");
    {
        let mut a = GCounter::new();
        let mut b = GCounter::new();
        a.increment("A", 3);
        b.increment("B", 7);

        let ab = a.merge(&b);
        let ba = b.merge(&a);
        println!("merge(A, B) = {:?}", ab.counts);
        println!("merge(B, A) = {:?}", ba.counts);
        println!("Commutative: {}", ab.counts == ba.counts);
    }

    section("Associative Merge Verification");
    {
        let mut a = GCounter::new();
        let mut b = GCounter::new();
        let mut c = GCounter::new();
        a.increment("A", 2);
        b.increment("B", 4);
        c.increment("C", 6);

        let ab_then_c = a.merge(&b).merge(&c);
        let a_then_bc = a.merge(&b.merge(&c));
        println!("merge(merge(A,B), C) = {:?}", ab_then_c.counts);
        println!("merge(A, merge(B,C)) = {:?}", a_then_bc.counts);
        println!("Associative: {}", ab_then_c.counts == a_then_bc.counts);
    }
}
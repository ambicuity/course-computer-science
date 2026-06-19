use std::collections::VecDeque;

fn rr(mut q: VecDeque<(&'static str, u32)>, quantum: u32) {
    while let Some((name, rem)) = q.pop_front() {
        let run = rem.min(quantum);
        println!("run {} for {}", name, run);
        if rem > quantum {
            q.push_back((name, rem - quantum));
        }
    }
}

fn main() {
    let q = VecDeque::from([("A", 5), ("B", 3), ("C", 7)]);
    rr(q, 2);
}

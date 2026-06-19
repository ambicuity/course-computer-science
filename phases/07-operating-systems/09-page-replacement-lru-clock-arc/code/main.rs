use std::collections::VecDeque;

fn lru_faults(cap: usize, refs: &[i32]) -> usize {
    let mut frames: VecDeque<i32> = VecDeque::new();
    let mut faults = 0;
    for &p in refs {
        if let Some(pos) = frames.iter().position(|&x| x == p) {
            let v = frames.remove(pos).unwrap_or(p);
            frames.push_back(v);
        } else {
            faults += 1;
            if frames.len() == cap {
                frames.pop_front();
            }
            frames.push_back(p);
        }
    }
    faults
}

fn main() {
    let refs = [1,2,3,1,4,5,2,1,2,3,4,5];
    println!("LRU faults: {}", lru_faults(3, &refs));
}

use std::time::Instant;

fn insertion_sort(v: &mut [i32]) {
    for i in 1..v.len() {
        let key = v[i];
        let mut j = i;
        while j > 0 && v[j - 1] > key {
            v[j] = v[j - 1];
            j -= 1;
        }
        v[j] = key;
    }
}

fn main() {
    let mut data: Vec<i32> = (0..5000).rev().collect();
    let t0 = Instant::now();
    insertion_sort(&mut data);
    let dt = t0.elapsed();
    println!("sorted={} elapsed_ms={}", data.windows(2).all(|w| w[0] <= w[1]), dt.as_millis());
}

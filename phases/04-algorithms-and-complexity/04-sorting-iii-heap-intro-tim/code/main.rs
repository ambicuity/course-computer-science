//! Sorting III — Heap, Intro, Tim
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Heap sort with generics + simplified Tim sort skeleton.

// ---------------------------------------------------------------------------
// Heap Sort
// ---------------------------------------------------------------------------

/// Bottom-up heapify then repeated extraction. O(n log n), in-place.
pub fn heap_sort<T: Ord>(arr: &mut [T]) {
    let n = arr.len();
    if n <= 1 {
        return;
    }

    // Bottom-up heapify: O(n)
    for i in (0..n / 2).rev() {
        sift_down(arr, i, n);
    }

    // Extract max: O(n log n)
    for end in (1..n).rev() {
        arr.swap(0, end);
        sift_down(arr, 0, end);
    }
}

fn sift_down<T: Ord>(arr: &mut [T], start: usize, end: usize) {
    let mut root = start;
    loop {
        let left = 2 * root + 1;
        if left >= end {
            break;
        }
        let right = left + 1;
        let mut largest = root;
        if arr[left] > arr[largest] {
            largest = left;
        }
        if right < end && arr[right] > arr[largest] {
            largest = right;
        }
        if largest == root {
            break;
        }
        arr.swap(root, largest);
        root = largest;
    }
}

// ---------------------------------------------------------------------------
// Simplified Tim Sort (skeleton)
// ---------------------------------------------------------------------------

/// Insertion sort on arr[lo..=hi].
fn insertion_sort_range<T: Ord>(arr: &mut [T], lo: usize, hi: usize) {
    for i in (lo + 1)..=hi {
        let mut j = i;
        while j > lo && arr[j] < arr[j - 1] {
            arr.swap(j, j - 1);
            j -= 1;
        }
    }
}

/// Merge arr[lo..=mid] and arr[mid+1..=hi].
fn merge<T: Ord + Clone>(arr: &mut [T], lo: usize, mid: usize, hi: usize) {
    let left: Vec<T> = arr[lo..=mid].to_vec();
    let right: Vec<T> = arr[(mid + 1)..=hi].to_vec();
    let mut i = 0;
    let mut j = 0;
    let mut k = lo;
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            arr[k] = left[i].clone();
            i += 1;
        } else {
            arr[k] = right[j].clone();
            j += 1;
        }
        k += 1;
    }
    while i < left.len() {
        arr[k] = left[i].clone();
        i += 1;
        k += 1;
    }
    while j < right.len() {
        arr[k] = right[j].clone();
        j += 1;
        k += 1;
    }
}

/// Compute min_run for Tim sort.
fn compute_min_run(n: usize) -> usize {
    let mut n = n;
    let mut r = 0;
    while n >= 32 {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

/// Simplified Tim sort: detect ascending runs, extend short ones, merge.
pub fn tim_sort_simplified<T: Ord + Clone>(arr: &mut [T]) {
    let n = arr.len();
    if n <= 1 {
        return;
    }
    let min_run = compute_min_run(n);

    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;

    while i < n {
        let run_start = i;

        // Detect descending run — reverse it
        if i + 1 < n && arr[i] > arr[i + 1] {
            while i + 1 < n && arr[i] > arr[i + 1] {
                i += 1;
            }
            arr[run_start..=i].reverse();
        } else {
            while i + 1 < n && arr[i] <= arr[i + 1] {
                i += 1;
            }
        }

        let mut run_end = i;
        // Extend short runs
        if run_end - run_start + 1 < min_run {
            let force_end = std::cmp::min(run_start + min_run - 1, n - 1);
            insertion_sort_range(arr, run_start, force_end);
            run_end = force_end;
        }

        runs.push((run_start, run_end));
        i = run_end + 1;
    }

    // Merge runs pairwise until one remains
    while runs.len() > 1 {
        let mut new_runs = Vec::new();
        let mut j = 0;
        while j < runs.len() {
            if j + 1 < runs.len() {
                let (lo, mid) = runs[j];
                let (_, hi) = runs[j + 1];
                merge(arr, lo, mid, hi);
                new_runs.push((lo, hi));
                j += 2;
            } else {
                new_runs.push(runs[j]);
                j += 1;
            }
        }
        runs = new_runs;
    }
}

// ---------------------------------------------------------------------------
// Tests & Demo
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_sort_empty() {
        let mut a: Vec<i32> = vec![];
        heap_sort(&mut a);
        assert_eq!(a, vec![]);
    }

    #[test]
    fn heap_sort_sorted() {
        let mut a = vec![5, 4, 3, 2, 1];
        heap_sort(&mut a);
        assert_eq!(a, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn heap_sort_random() {
        let mut a = vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
        let expected = sorted(&a);
        heap_sort(&mut a);
        assert_eq!(a, expected);
    }

    #[test]
    fn tim_sort_empty() {
        let mut a: Vec<i32> = vec![];
        tim_sort_simplified(&mut a);
        assert_eq!(a, vec![]);
    }

    #[test]
    fn tim_sort_sorted() {
        let mut a = vec![5, 4, 3, 2, 1];
        tim_sort_simplified(&mut a);
        assert_eq!(a, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn tim_sort_random() {
        let mut a = vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
        let expected = sorted(&a);
        tim_sort_simplified(&mut a);
        assert_eq!(a, expected);
    }

    fn sorted(a: &[i32]) -> Vec<i32> {
        let mut b = a.to_vec();
        b.sort();
        b
    }
}

fn main() {
    let mut data = vec![5, 1, 4, 2, 8, 3, 7, 6, 0, 9];
    println!("Original:  {:?}", data);

    let mut heap = data.clone();
    heap_sort(&mut heap);
    println!("Heap sort: {:?}", heap);

    let mut tim = data.clone();
    tim_sort_simplified(&mut tim);
    println!("Tim sort:  {:?}", tim);

    assert_eq!(heap, tim);
    println!("Both match.");

    println!("\nRun `cargo test` to verify correctness.");
}

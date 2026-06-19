//! Tiny state-vector simulator: H + CNOT on 2 qubits.

fn zero_state(n: usize) -> Vec<(f64, f64)> {
    let mut s = vec![(0.0, 0.0); 1 << n];
    s[0] = (1.0, 0.0);
    s
}

fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) { (a.0 + b.0, a.1 + b.1) }
fn sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) { (a.0 - b.0, a.1 - b.1) }
fn scale(a: (f64, f64), k: f64) -> (f64, f64) { (a.0 * k, a.1 * k) }

fn apply_h(state: &[(f64, f64)], target: usize, n: usize) -> Vec<(f64, f64)> {
    let mut out = state.to_vec();
    let stride = 1 << target;
    let span = stride << 1;
    let inv = 1.0 / 2.0_f64.sqrt();
    for base in (0..(1 << n)).step_by(span) {
        for i in 0..stride {
            let a = state[base + i];
            let b = state[base + i + stride];
            out[base + i] = scale(add(a, b), inv);
            out[base + i + stride] = scale(sub(a, b), inv);
        }
    }
    out
}

fn apply_cnot(state: &[(f64, f64)], control: usize, target: usize, n: usize) -> Vec<(f64, f64)> {
    let mut out = state.to_vec();
    for idx in 0..(1 << n) {
        if ((idx >> control) & 1) == 1 {
            let f = idx ^ (1 << target);
            out[f] = state[idx];
            out[idx] = state[f];
        }
    }
    out
}

fn prob(a: (f64, f64)) -> f64 { a.0 * a.0 + a.1 * a.1 }

fn main() {
    let n = 2;
    let mut s = zero_state(n);
    s = apply_h(&s, 0, n);
    s = apply_cnot(&s, 0, 1, n);

    for (i, amp) in s.iter().copied().enumerate() {
        println!("|{:02b}>: {:.3}", i, prob(amp));
    }
}

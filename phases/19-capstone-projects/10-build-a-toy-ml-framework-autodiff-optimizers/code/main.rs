#[derive(Debug, Clone, Copy)]
struct Param {
    v: f64,
    g: f64,
}

fn sgd_step(p: &mut Param, lr: f64) {
    p.v -= lr * p.g;
    p.g = 0.0;
}

fn main() {
    let mut w = Param { v: 1.0, g: -16.0 }; // toy gradient
    println!("before={:?}", w);
    sgd_step(&mut w, 0.1);
    println!("after={:?}", w);
}

fn consume(s: String) -> usize {
    s.len()
}

fn main() {
    let a = String::from("owned");
    let n = consume(a);
    println!("len={}", n);

    let b = String::from("borrowed");
    let r = &b;
    println!("ref={}", r);
    println!("owner still usable={}", b);
}

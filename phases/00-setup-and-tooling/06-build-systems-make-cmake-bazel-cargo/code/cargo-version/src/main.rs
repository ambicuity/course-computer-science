fn greet(who: &str) {
    let prefix = std::env::var("GREET_PREFIX").unwrap_or_else(|_| "hello".to_string());
    println!("{prefix}, {who}");
}

fn main() {
    greet("world");
}

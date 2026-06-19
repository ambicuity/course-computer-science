use std::collections::HashMap;

fn main() {
    let mut cache: HashMap<&str, &str> = HashMap::new();
    let key = "compile:src/main.c:-O2";

    match cache.get(key) {
        Some(v) => println!("cache hit {}", v),
        None => {
            cache.insert(key, "obj/main.o");
            println!("cache miss -> store obj/main.o");
        }
    }
}

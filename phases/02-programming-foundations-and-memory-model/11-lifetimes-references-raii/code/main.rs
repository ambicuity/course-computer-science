//! Lifetimes and RAII in Rust.
//! Build: rustc -O main.rs -o m && ./m

use std::rc::Rc;
use std::sync::Arc;

/// Explicit-lifetime function: returned reference is bounded by the shorter of s1, s2.
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.len() > s2.len() { s1 } else { s2 }
}

/// Lifetime elision: single input lifetime → output gets it.
fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// Struct holding a borrowed field — must declare the lifetime.
struct Token<'a> {
    text: &'a str,
    kind: &'static str,
}

impl<'a> Token<'a> {
    fn describe(&self) -> String {
        format!("{}({:?})", self.kind, self.text)
    }
}

struct LoudResource {
    label: String,
}

impl Drop for LoudResource {
    fn drop(&mut self) {
        println!("  ↓ Dropping LoudResource({:?})", self.label);
    }
}

fn main() {
    println!("== Annotated lifetime: longest of two ==");
    let s1 = String::from("longer string");
    let s2 = String::from("hi");
    let result = longest(&s1, &s2);
    println!("  longest = {:?}", result);

    println!("\n== Elided lifetime ==");
    let sentence = String::from("the quick brown fox");
    let first = first_word(&sentence);
    println!("  first word of {:?} = {:?}", sentence, first);

    println!("\n== Struct with borrowed field ==");
    let src = String::from("identifier:42");
    let parts: Vec<&str> = src.splitn(2, ':').collect();
    let tok = Token { text: parts[0], kind: "Ident" };
    println!("  {}", tok.describe());

    println!("\n== Box<T> = unique_ptr ==");
    {
        let boxed = Box::new(LoudResource { label: "heap-allocated".to_string() });
        println!("  boxed.label = {:?}", boxed.label);
    } // drop fires here

    println!("\n== Rc<T> shared ownership (single thread) ==");
    {
        let a = Rc::new(LoudResource { label: "shared".to_string() });
        let b = Rc::clone(&a);     // refcount = 2
        let c = Rc::clone(&a);     // refcount = 3
        println!("  refcount = {} after 3 owners", Rc::strong_count(&a));
        drop(b);
        drop(c);
        println!("  refcount = {} after 2 dropped", Rc::strong_count(&a));
    } // drop a here; refcount → 0; underlying value dropped

    println!("\n== Arc<T> shared across threads (preview) ==");
    let shared = Arc::new(42i32);
    let s2 = Arc::clone(&shared);
    println!("  Arc value: {} (strong_count = {})", *s2, Arc::strong_count(&shared));
}

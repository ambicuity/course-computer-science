//! Ownership, moves, borrows, Drop, Clone — runnable demos.
//! Build: rustc -O main.rs -o m && ./m

struct Loud(String);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("  ↓ Dropping Loud({:?})", self.0);
    }
}

fn read(s: &String)         { println!("  read: {}", s); }
fn append(s: &mut String)   { s.push_str(" world"); }

fn main() {
    println!("== Move semantics (non-Copy types) ==");
    let s1 = String::from("hi");
    let s2 = s1;              // move
    println!("  s2 = {}  (s1 is moved; using s1 would not compile)", s2);

    println!("\n== Copy semantics (Copy types) ==");
    let a: i32 = 5;
    let b = a;                // copy — i32 is Copy
    println!("  a = {}, b = {}  (both valid; i32 is Copy)", a, b);

    println!("\n== Borrowing: many shared XOR one exclusive ==");
    let mut s = String::from("hello");
    read(&s);
    read(&s);                 // multiple shared borrows OK
    append(&mut s);           // exclusive borrow; other refs ended
    read(&s);
    /* The following would fail to compile (NLL: Non-Lexical Lifetimes):
        let r1 = &s;
        let r2 = &mut s;       // ← error: cannot borrow `s` as mutable
        println!("{}", r1);
    */

    println!("\n== Clone when you need a duplicate ==");
    let original = String::from("data");
    let copy = original.clone();
    println!("  original = {:?}, copy = {:?}  (both heap-allocated, independent)",
             original, copy);

    println!("\n== Drop fires at scope end (RAII) ==");
    {
        let _x = Loud(String::from("scope-local"));
        println!("  inside inner scope");
    }   // _x dropped here
    println!("  outside scope (Loud already dropped above)");

    println!("\n== Ownership transferred into a function ==");
    let owned = String::from("eaten");
    consume(owned);
    // println!("{}", owned);  // would error: value moved into consume
    println!("  caller can no longer use the value");
}

fn consume(s: String) {
    println!("  consume took ownership of {:?}", s);
    // s is dropped here, at end of `consume`
}

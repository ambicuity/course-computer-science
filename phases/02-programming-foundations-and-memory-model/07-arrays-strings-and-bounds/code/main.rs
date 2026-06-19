//! Rust slice safety: the (pointer, length) pair C arrays don't carry.
//! Build:  rustc -O main.rs -o m && ./m

fn sum(s: &[i32]) -> i32 {
    s.iter().sum()
}

fn main() {
    println!("== Slices carry length ==");
    let arr = [10, 20, 30, 40, 50];
    let s: &[i32] = &arr;
    println!("  arr.len() = {}, slice.len() = {}", arr.len(), s.len());
    println!("  sum(&arr) = {}", sum(&arr));      // works for any-length array

    println!("\n== Byte vs char vs grapheme length ==");
    let cafe = "café";              // 'é' is 2 bytes in UTF-8
    let emoji = "👨‍👩‍👧";          // family emoji: many bytes, many codepoints, one grapheme
    println!("  cafe  = \"{}\"  bytes={}, chars={}",
             cafe, cafe.len(), cafe.chars().count());
    println!("  emoji = \"{}\"  bytes={}, chars={}",
             emoji, emoji.len(), emoji.chars().count());

    println!("\n== Bounds-checked indexing ==");
    let v = vec![1, 2, 3];
    println!("  v[1] = {}  (in-bounds)", v[1]);
    // Uncommenting the next line would PANIC at runtime:
    //   thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 5'
    // let _bad = v[5];

    // Safe alternative: .get returns Option<&T>
    println!("  v.get(5) = {:?}  (None instead of panic)", v.get(5));

    println!("\n== String slicing must respect UTF-8 boundaries ==");
    let s = String::from("café");
    println!("  s = {:?},  &s[0..2] = {:?}", s, &s[0..2]);  // "ca" — OK
    // &s[0..3] would PANIC: byte index 3 lies in the middle of the 'é' codepoint
}

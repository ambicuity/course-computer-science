use std::collections::HashMap;

fn main() {
    let mut index: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();
    index.insert("consensus", vec![("d1", 1), ("d3", 1)]);
    index.insert("store", vec![("d3", 1)]);

    println!("index terms={}", index.len());
    println!("consensus postings={:?}", index.get("consensus"));
}

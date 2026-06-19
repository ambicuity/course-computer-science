// Build a SQL Database with B-Tree Index
// Run: rustc main.rs && ./main
//
// Architecture:
//   Row data → Table (schema + primary key index) → B+-Tree (custom implementation)
//   Supports point lookup and range scan via a hand-built B+-tree with order 4.

use std::collections::HashMap;

// =============================================================================
// Step 1: B+-Tree Node Structure and Core Operations
// =============================================================================

const ORDER: usize = 4;
const MAX_KEYS: usize = ORDER;

#[derive(Debug, Clone)]
enum Node {
    Internal {
        keys: Vec<i64>,
        children: Vec<Box<Node>>,
    },
    Leaf {
        keys: Vec<i64>,
        values: Vec<String>,
        next: Option<usize>,
    },
}

struct BPlusTree {
    root: Box<Node>,
    leaf_count: usize,
}

impl BPlusTree {
    fn new() -> Self {
        BPlusTree {
            root: Box::new(Node::Leaf {
                keys: Vec::new(),
                values: Vec::new(),
                next: None,
            }),
            leaf_count: 1,
        }
    }

    fn get(&self, key: i64) -> Option<String> {
        self.search_node(&self.root, key)
    }

    fn search_node(&self, node: &Node, key: i64) -> Option<String> {
        match node {
            Node::Leaf { keys, values, .. } => {
                match keys.binary_search(&key) {
                    Ok(idx) => Some(values[idx].clone()),
                    Err(_) => None,
                }
            }
            Node::Internal { keys, children } => {
                let idx = keys.partition_point(|&k| k <= key);
                self.search_node(&children[idx], key)
            }
        }
    }

    fn insert(&mut self, key: i64, value: String) {
        let split = self.insert_node(&mut self.root, key, value);
        if let Some((sep_key, new_node)) = split {
            let old_root = std::mem::replace(
                &mut self.root,
                Box::new(Node::Internal {
                    keys: vec![sep_key],
                    children: vec![],
                }),
            );
            if let Node::Internal { keys: _, ref mut children } = *self.root {
                children.push(old_root);
                children.push(new_node);
            }
        }
    }

    fn insert_node(&mut self, node: &mut Box<Node>, key: i64, value: String)
        -> Option<(i64, Box<Node>)>
    {
        match node.as_mut() {
            Node::Leaf { keys, values, .. } => {
                match keys.binary_search(&key) {
                    Ok(idx) => {
                        values[idx] = value;
                        None
                    }
                    Err(idx) => {
                        keys.insert(idx, key);
                        values.insert(idx, value);
                        if keys.len() > MAX_KEYS {
                            let split_idx = keys.len() / 2;
                            let right_keys = keys.split_off(split_idx);
                            let right_values = values.split_off(split_idx);
                            let sep = right_keys[0];
                            let new_leaf = Box::new(Node::Leaf {
                                keys: right_keys,
                                values: right_values,
                                next: None,
                            });
                            Some((sep, new_leaf))
                        } else {
                            None
                        }
                    }
                }
            }
            Node::Internal { keys, children } => {
                let idx = keys.partition_point(|&k| k <= key);
                if let Some((sep, new_child)) = self.insert_node(&mut children[idx], key, value) {
                    keys.insert(idx, sep);
                    children.insert(idx + 1, new_child);
                    if keys.len() > MAX_KEYS {
                        let split_idx = keys.len() / 2;
                        let right_keys = keys.split_off(split_idx + 1);
                        let up_key = keys.pop().unwrap();
                        let right_children = children.split_off(split_idx + 1);
                        let new_internal = Box::new(Node::Internal {
                            keys: right_keys,
                            children: right_children,
                        });
                        Some((up_key, new_internal))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    fn range_scan(&self, lo: i64, hi: i64) -> Vec<(i64, String)> {
        let mut results = Vec::new();
        self.collect_range(&self.root, lo, hi, &mut results);
        results
    }

    fn collect_range(&self, node: &Node, lo: i64, hi: i64, results: &mut Vec<(i64, String)>) {
        match node {
            Node::Leaf { keys, values, .. } => {
                for (k, v) in keys.iter().zip(values.iter()) {
                    if *k >= lo && *k <= hi {
                        results.push((*k, v.clone()));
                    }
                }
            }
            Node::Internal { keys, children } => {
                for (i, child) in children.iter().enumerate() {
                    if i < keys.len() && keys[i] < lo { continue; }
                    if i > 0 && keys[i-1] > hi { break; }
                    self.collect_range(child, lo, hi, results);
                }
            }
        }
    }
}

// =============================================================================
// Step 2: Row Serialization and Table Layer
// =============================================================================

#[derive(Debug, Clone)]
struct Column {
    name: String,
    col_type: String,
}

#[derive(Debug, Clone)]
struct TableSchema {
    name: String,
    columns: Vec<Column>,
    primary_key: String,
}

fn serialize_row(row: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = row.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    parts.sort();
    parts.join("|")
}

fn deserialize_row(data: &str) -> HashMap<String, String> {
    data.split('|')
        .filter_map(|part| {
            let mut split = part.splitn(2, '=');
            Some((split.next()?.to_string(), split.next()?.to_string()))
        })
        .collect()
}

struct Table {
    schema: TableSchema,
    index: BPlusTree,
}

impl Table {
    fn new(schema: TableSchema) -> Self {
        Table { schema, index: BPlusTree::new() }
    }

    fn insert_row(&mut self, row: HashMap<String, String>) -> Result<(), String> {
        let pk = row.get(&self.schema.primary_key)
            .ok_or("Missing primary key")?
            .parse::<i64>()
            .map_err(|_| "Primary key must be integer")?;
        let serialized = serialize_row(&row);
        self.index.insert(pk, serialized);
        Ok(())
    }

    fn get_by_key(&self, key: i64) -> Option<HashMap<String, String>> {
        self.index.get(key).map(|data| deserialize_row(&data))
    }

    fn scan_range(&self, lo: i64, hi: i64) -> Vec<HashMap<String, String>> {
        self.index.range_scan(lo, hi)
            .into_iter()
            .map(|(_, data)| deserialize_row(&data))
            .collect()
    }
}

// =============================================================================
// Step 3: Demo
// =============================================================================

fn main() {
    let schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            Column { name: "id".to_string(), col_type: "INTEGER".to_string() },
            Column { name: "name".to_string(), col_type: "TEXT".to_string() },
            Column { name: "age".to_string(), col_type: "INTEGER".to_string() },
        ],
        primary_key: "id".to_string(),
    };

    let mut table = Table::new(schema);

    for (id, name, age) in &[
        (10, "Alice", 30),
        (20, "Bob", 25),
        (30, "Charlie", 35),
        (40, "Diana", 28),
        (50, "Eve", 32),
    ] {
        let mut row = HashMap::new();
        row.insert("id".to_string(), id.to_string());
        row.insert("name".to_string(), name.to_string());
        row.insert("age".to_string(), age.to_string());
        table.insert_row(row).unwrap();
    }

    println!("=== Point lookup: id=30 ===");
    if let Some(row) = table.get_by_key(30) {
        println!("  {:?}", row);
    }

    println!("\n=== Range scan: id in [20, 40] ===");
    for row in table.scan_range(20, 40) {
        println!("  {:?}", row);
    }
}

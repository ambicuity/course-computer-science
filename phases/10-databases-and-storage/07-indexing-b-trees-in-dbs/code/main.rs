use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
enum Node<K, V> {
    Internal {
        keys: Vec<K>,
        children: Vec<Box<Node<K, V>>>,
    },
    Leaf {
        keys: Vec<K>,
        vals: Vec<V>,
        next: Option<Box<Node<K, V>>>,
    },
}

impl<K: Clone, V: Clone> Clone for Node<K, V> {
    fn clone(&self) -> Self {
        match self {
            Node::Internal { keys, children } => Node::Internal {
                keys: keys.clone(),
                children: children.clone(),
            },
            Node::Leaf { keys, vals, next } => Node::Leaf {
                keys: keys.clone(),
                vals: vals.clone(),
                next: next.clone(),
            },
        }
    }
}

enum InsertResult<K, V> {
    Done(Box<Node<K, V>>),
    Split {
        left: Box<Node<K, V>>,
        median: K,
        right: Box<Node<K, V>>,
    },
}

enum DeleteResult<K, V> {
    Done(Box<Node<K, V>>),
    Removed,
    NotFound(Box<Node<K, V>>),
}

pub struct BPlusTree<K: Ord + Clone, V: Clone> {
    order: usize,
    root: Option<Box<Node<K, V>>>,
}

impl<K: Ord + Clone, V: Clone> BPlusTree<K, V> {
    pub fn new(order: usize) -> Self {
        assert!(order >= 2, "B+ tree order must be >= 2");
        BPlusTree { order, root: None }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let node = self.root.as_ref()?;
        search_in_node(node, key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        let old_root = self.root.take();
        match insert_in_node(old_root, key, value, self.order) {
            InsertResult::Done(node) => self.root = Some(node),
            InsertResult::Split { left, median, right } => {
                self.root = Some(Box::new(Node::Internal {
                    keys: vec![median],
                    children: vec![left, right],
                }));
            }
        }
    }

    pub fn delete(&mut self, key: &K) -> bool {
        if self.root.is_none() {
            return false;
        }
        let old_root = self.root.take();
        match delete_from_node(old_root, key, self.order) {
            DeleteResult::Done(node) => {
                self.root = Some(node);
                true
            }
            DeleteResult::Removed => {
                self.root = None;
                true
            }
            DeleteResult::NotFound(node) => {
                self.root = Some(node);
                false
            }
        }
    }

    pub fn range_scan(&self, start: &K, end: &K) -> Vec<(K, V)> {
        let mut results = Vec::new();
        if let Some(ref root) = self.root {
            collect_range(root, start, end, &mut results);
        }
        results
    }
}

fn search_in_node<'a, K: Ord, V>(node: &'a Box<Node<K, V>>, key: &K) -> Option<&'a V> {
    match node.as_ref() {
        Node::Internal { keys, children } => {
            let idx = match keys.binary_search(key) {
                Ok(i) => i + 1,
                Err(i) => i,
            };
            search_in_node(&children[idx], key)
        }
        Node::Leaf { keys, vals, .. } => keys.binary_search(key).ok().map(|i| &vals[i]),
    }
}

fn insert_in_node<K: Ord + Clone, V: Clone>(
    node: Option<Box<Node<K, V>>>,
    key: K,
    value: V,
    order: usize,
) -> InsertResult<K, V> {
    let node = match node {
        Some(n) => n,
        None => {
            return InsertResult::Done(Box::new(Node::Leaf {
                keys: vec![key],
                vals: vec![value],
                next: None,
            }));
        }
    };

    match *node {
        Node::Leaf { mut keys, mut vals, next } => {
            match keys.binary_search(&key) {
                Ok(idx) => {
                    vals[idx] = value;
                    return InsertResult::Done(Box::new(Node::Leaf { keys, vals, next }));
                }
                Err(idx) => {
                    keys.insert(idx, key);
                    vals.insert(idx, value);
                }
            }

            if keys.len() <= order {
                return InsertResult::Done(Box::new(Node::Leaf { keys, vals, next }));
            }

            let split = keys.len() / 2;
            let right_keys = keys.split_off(split);
            let right_vals = vals.split_off(split);
            let median = right_keys[0].clone();

            let right = Box::new(Node::Leaf {
                keys: right_keys,
                vals: right_vals,
                next,
            });
            let left = Box::new(Node::Leaf {
                keys,
                vals,
                next: Some(right.clone()),
            });

            InsertResult::Split { left, median, right }
        }
        Node::Internal { mut keys, mut children } => {
            let idx = match keys.binary_search(&key) {
                Ok(i) => i + 1,
                Err(i) => i,
            };
            let child = children.remove(idx);

            match insert_in_node(Some(child), key, value, order) {
                InsertResult::Done(new_child) => {
                    children.insert(idx, new_child);
                    InsertResult::Done(Box::new(Node::Internal { keys, children }))
                }
                InsertResult::Split { left, median, right } => {
                    keys.insert(idx, median);
                    children.insert(idx, left);
                    children.insert(idx + 1, right);

                    if keys.len() <= order {
                        return InsertResult::Done(Box::new(Node::Internal { keys, children }));
                    }

                    let mid = keys.len() / 2;
                    let promoted = keys[mid].clone();
                    let right_keys: Vec<K> = keys.drain(mid + 1..).collect();
                    keys.pop();
                    let right_children: Vec<Box<Node<K, V>>> =
                        children.drain(mid + 1..).collect();

                    let left_node = Box::new(Node::Internal {
                        keys,
                        children,
                    });
                    let right_node = Box::new(Node::Internal {
                        keys: right_keys,
                        children: right_children,
                    });

                    InsertResult::Split {
                        left: left_node,
                        median: promoted,
                        right: right_node,
                    }
                }
            }
        }
    }
}

fn delete_from_node<K: Ord, V>(
    node: Option<Box<Node<K, V>>>,
    key: &K,
    order: usize,
) -> DeleteResult<K, V> {
    let node = node.unwrap();

    match *node {
        Node::Leaf { mut keys, mut vals, next } => {
            let idx = match keys.binary_search(key) {
                Ok(i) => i,
                Err(_) => {
                    return DeleteResult::NotFound(Box::new(Node::Leaf { keys, vals, next }));
                }
            };
            keys.remove(idx);
            vals.remove(idx);
            if keys.is_empty() {
                DeleteResult::Removed
            } else {
                DeleteResult::Done(Box::new(Node::Leaf { keys, vals, next }))
            }
        }
        Node::Internal { mut keys, mut children } => {
            let idx = match keys.binary_search(key) {
                Ok(i) => i + 1,
                Err(i) => i,
            };
            let child = children.remove(idx);

            match delete_from_node(Some(child), key, order) {
                DeleteResult::NotFound(restored_child) => {
                    children.insert(idx, restored_child);
                    DeleteResult::NotFound(Box::new(Node::Internal { keys, children }))
                }
                DeleteResult::Done(new_child) => {
                    children.insert(idx, new_child);
                    DeleteResult::Done(Box::new(Node::Internal { keys, children }))
                }
                DeleteResult::Removed => {
                    if keys.is_empty() {
                        return if children.is_empty() {
                            DeleteResult::Removed
                        } else if children.len() == 1 {
                            DeleteResult::Done(children.remove(0))
                        } else {
                            DeleteResult::Done(Box::new(Node::Internal { keys, children }))
                        };
                    }
                    let sep_idx = idx.min(keys.len() - 1);
                    keys.remove(sep_idx);

                    if keys.is_empty() && children.len() == 1 {
                        DeleteResult::Done(children.remove(0))
                    } else {
                        DeleteResult::Done(Box::new(Node::Internal { keys, children }))
                    }
                }
            }
        }
    }
}

fn collect_range<K: Ord + Clone, V: Clone>(
    node: &Box<Node<K, V>>,
    start: &K,
    end: &K,
    results: &mut Vec<(K, V)>,
) {
    match node.as_ref() {
        Node::Internal { keys, children } => {
            let idx = match keys.binary_search(start) {
                Ok(i) => i,
                Err(i) => i,
            };
            for child in children.iter().skip(idx) {
                collect_range(child, start, end, results);
            }
        }
        Node::Leaf { keys, vals, .. } => {
            for (k, v) in keys.iter().zip(vals.iter()) {
                if *k >= *start && *k <= *end {
                    results.push((k.clone(), v.clone()));
                }
            }
        }
    }
}

impl<K: Display + Ord + Clone, V: Display + Clone> Display for BPlusTree<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.root {
            None => write!(f, "Empty tree"),
            Some(ref root) => display_node(root.as_ref(), f, 0),
        }
    }
}

fn display_node<K: Display, V: Display>(
    node: &Node<K, V>,
    f: &mut Formatter<'_>,
    depth: usize,
) -> FmtResult {
    let indent = "  ".repeat(depth);
    match node {
        Node::Internal { keys, children } => {
            writeln!(
                f,
                "{}Internal: [{}]",
                indent,
                keys.iter()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
            for child in children {
                display_node(child.as_ref(), f, depth + 1)?;
            }
            Ok(())
        }
        Node::Leaf { keys, vals, .. } => {
            let pairs: Vec<String> = keys
                .iter()
                .zip(vals.iter())
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            writeln!(f, "{}{}", indent, pairs.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get_integer() {
        let mut tree = BPlusTree::new(4);
        tree.insert(10, "ten");
        tree.insert(20, "twenty");
        tree.insert(5, "five");
        tree.insert(15, "fifteen");

        assert_eq!(tree.get(&5), Some(&"five"));
        assert_eq!(tree.get(&10), Some(&"ten"));
        assert_eq!(tree.get(&15), Some(&"fifteen"));
        assert_eq!(tree.get(&20), Some(&"twenty"));
        assert_eq!(tree.get(&25), None);
    }

    #[test]
    fn test_insert_triggers_split() {
        let mut tree = BPlusTree::new(3);
        tree.insert(10, "a");
        tree.insert(20, "b");
        tree.insert(5, "c");
        tree.insert(15, "d");

        assert_eq!(tree.get(&5), Some(&"c"));
        assert_eq!(tree.get(&10), Some(&"a"));
        assert_eq!(tree.get(&15), Some(&"d"));
        assert_eq!(tree.get(&20), Some(&"b"));
    }

    #[test]
    fn test_replace_value() {
        let mut tree = BPlusTree::new(4);
        tree.insert(10, "old");
        tree.insert(10, "new");
        assert_eq!(tree.get(&10), Some(&"new"));
    }

    #[test]
    fn test_range_scan() {
        let mut tree = BPlusTree::new(4);
        for i in (0..100).step_by(10) {
            tree.insert(i, i.to_string());
        }
        let results = tree.range_scan(&20, &60);
        let keys: Vec<i32> = results.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec![20, 30, 40, 50, 60]);
    }

    #[test]
    fn test_range_scan_no_results() {
        let mut tree = BPlusTree::new(4);
        tree.insert(10, "a");
        tree.insert(20, "b");
        let results = tree.range_scan(&30, &50);
        assert!(results.is_empty());
    }

    #[test]
    fn test_delete_leaf() {
        let mut tree = BPlusTree::new(4);
        tree.insert(10, "ten");
        tree.insert(20, "twenty");
        assert!(tree.delete(&10));
        assert_eq!(tree.get(&10), None);
        assert_eq!(tree.get(&20), Some(&"twenty"));
    }

    #[test]
    fn test_delete_not_found() {
        let mut tree = BPlusTree::new(4);
        tree.insert(10, "ten");
        assert!(!tree.delete(&5));
    }

    #[test]
    fn test_string_keys() {
        let mut tree = BPlusTree::new(4);
        tree.insert("cat", 1);
        tree.insert("dog", 2);
        tree.insert("bird", 3);
        tree.insert("zebra", 4);
        assert_eq!(tree.get(&"bird"), Some(&3));
        assert_eq!(tree.get(&"dog"), Some(&2));
    }

    #[test]
    fn test_large_insert() {
        let mut tree = BPlusTree::new(6);
        let n = 1000;
        for i in 0..n {
            tree.insert(i, i * 2);
        }
        for i in 0..n {
            assert_eq!(tree.get(&i), Some(&(i * 2)));
        }
    }

    #[test]
    fn test_delete_all() {
        let mut tree = BPlusTree::new(4);
        tree.insert(10, "a");
        tree.insert(20, "b");
        tree.insert(30, "c");
        assert!(tree.delete(&10));
        assert!(tree.delete(&20));
        assert!(tree.delete(&30));
        assert!(tree.is_empty());
    }

    #[test]
    fn test_range_scan_strings() {
        let mut tree = BPlusTree::new(4);
        tree.insert("apple", 1);
        tree.insert("banana", 2);
        tree.insert("cherry", 3);
        tree.insert("date", 4);
        let results = tree.range_scan(&"banana", &"date");
        let keys: Vec<&str> = results.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["banana", "cherry", "date"]);
    }

    #[test]
    fn test_internal_node_split() {
        let mut tree = BPlusTree::new(3);
        for i in 0..20 {
            tree.insert(i, i);
        }
        for i in 0..20 {
            assert_eq!(tree.get(&i), Some(&i));
        }
    }
}

fn main() {
    let mut tree: BPlusTree<i32, &str> = BPlusTree::new(4);
    tree.insert(10, "ten");
    tree.insert(5, "five");
    tree.insert(15, "fifteen");
    tree.insert(3, "three");
    tree.insert(7, "seven");
    tree.insert(12, "twelve");
    tree.insert(18, "eighteen");

    println!("B+ Tree:\n{}", tree);
    println!("\nGet key 7: {:?}", tree.get(&7));
    println!("Range scan [5, 15]: {:?}", tree.range_scan(&5, &15));
    println!("Delete key 10: {}", tree.delete(&10));
    println!("After delete:\n{}", tree);
}

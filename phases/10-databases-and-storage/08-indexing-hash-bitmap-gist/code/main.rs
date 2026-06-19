//! Indexing — Hash, Bitmap, GiST
//! Phase 10 — Databases & Storage Systems
//!
//! Implementes extendible hashing with directory doubling and bucket splitting.

use std::fmt::Debug;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// Bucket
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Bucket<K: Clone + Eq + Hash, V: Clone> {
    depth: u32,
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Clone + Eq + Hash, V: Clone> Bucket<K, V> {
    fn new(depth: u32) -> Self {
        Bucket { depth, keys: Vec::new(), values: Vec::new() }
    }

    fn capacity(&self) -> usize {
        2
    }

    fn is_full(&self) -> bool {
        self.keys.len() >= self.capacity()
    }

    fn search(&self, key: &K) -> Option<&V> {
        self.keys.iter().position(|k| k == key).map(|i| &self.values[i])
    }

    fn upsert(&mut self, key: K, value: V) -> bool {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            self.values[i] = value;
            return false;
        }
        if self.is_full() {
            return false;
        }
        self.keys.push(key);
        self.values.push(value);
        true
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(i) = self.keys.iter().position(|k| k == key) {
            self.keys.swap_remove(i);
            self.values.swap_remove(i);
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------------------
// ExtendibleHash
// ---------------------------------------------------------------------------

type BucketPtr<K, V> = *mut Bucket<K, V>;

pub struct ExtendibleHash<K: Clone + Eq + Hash, V: Clone> {
    global_depth: u32,
    directory: Vec<BucketPtr<K, V>>,
}

unsafe impl<K: Clone + Eq + Hash + Send, V: Clone + Send> Send for ExtendibleHash<K, V> {}
unsafe impl<K: Clone + Eq + Hash + Sync, V: Clone + Sync> Sync for ExtendibleHash<K, V> {}

impl<K: Clone + Eq + Hash, V: Clone> ExtendibleHash<K, V> {
    pub fn new() -> Self {
        let b = Box::into_raw(Box::new(Bucket::new(1)));
        ExtendibleHash { global_depth: 1, directory: vec![b, b] }
    }

    fn hash_key(key: &K) -> u64 {
        let mut h = std::hash::DefaultHasher::new();
        key.hash(&mut h);
        h.finish()
    }

    fn mask(depth: u32) -> u32 {
        (1 << depth) - 1
    }

    fn dir_index(hash: u64, depth: u32) -> usize {
        (hash as u32 & Self::mask(depth)) as usize
    }

    pub fn search(&self, key: &K) -> Option<&V> {
        let h = Self::hash_key(key);
        let idx = Self::dir_index(h, self.global_depth);
        unsafe { (*self.directory[idx]).search(key) }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let h = Self::hash_key(&key);
        let idx = Self::dir_index(h, self.global_depth);
        let bp = self.directory[idx];
        unsafe {
            if !(*bp).is_full() || (*bp).search(&key).is_some() {
                (*bp).upsert(key, value);
                return;
            }
        }
        self.split(idx, key, value, h);
    }

    fn split(&mut self, idx: usize, key: K, value: V, hash: u64) {
        let old_depth;
        unsafe { old_depth = (*self.directory[idx]).depth; }
        let new_depth = old_depth + 1;

        if new_depth > self.global_depth {
            self.double_directory();
        }

        let mut b0 = Box::new(Bucket::new(new_depth));
        let mut b1 = Box::new(Bucket::new(new_depth));

        let mask_bit = 1 << old_depth;

        unsafe {
            let old = &mut *self.directory[idx];
            let keys = std::mem::take(&mut old.keys);
            let values = std::mem::take(&mut old.values);
            for (k, v) in keys.into_iter().zip(values.into_iter()) {
                let hk = Self::hash_key(&k);
                if (Self::dir_index(hk, new_depth) & mask_bit) == 0 {
                    b0.keys.push(k);
                    b0.values.push(v);
                } else {
                    b1.keys.push(k);
                    b1.values.push(v);
                }
            }
        }

        if (Self::dir_index(hash, new_depth) & mask_bit) == 0 {
            b0.keys.push(key);
            b0.values.push(value);
        } else {
            b1.keys.push(key);
            b1.values.push(value);
        }

        let bp0 = Box::into_raw(b0);
        let bp1 = Box::into_raw(b1);

        let old_bucket_ptr = self.directory[idx];
        for i in 0..self.directory.len() {
            if self.directory[i] == old_bucket_ptr {
                if (i as u32 & mask_bit as u32) == 0 {
                    self.directory[i] = bp0;
                } else {
                    self.directory[i] = bp1;
                }
            }
        }
    }

    fn double_directory(&mut self) {
        let old = self.directory.clone();
        self.directory.reserve(old.len());
        for i in 0..old.len() {
            self.directory.push(old[i]);
        }
        self.global_depth += 1;
    }

    pub fn remove(&mut self, key: &K) -> bool {
        let h = Self::hash_key(key);
        let idx = Self::dir_index(h, self.global_depth);
        unsafe { (*self.directory[idx]).remove(key) }
    }
}

impl<K: Clone + Eq + Hash, V: Clone> Drop for ExtendibleHash<K, V> {
    fn drop(&mut self) {
        let mut freed = std::collections::HashSet::new();
        for &p in &self.directory {
            if freed.insert(p as usize) {
                unsafe { drop(Box::from_raw(p)); }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Demo
// ---------------------------------------------------------------------------

fn demo_extendible_hash() {
    let mut idx = ExtendibleHash::new();
    let pairs = [(10, 100), (22, 200), (1, 10), (7, 70),
                 (15, 150), (3, 30), (31, 310), (9, 90)];

    for &(k, v) in &pairs {
        idx.insert(k, v);
    }

    println!("global_depth={}", idx.global_depth);
    println!("directory_size={}", idx.directory.len());

    let tests = [10, 22, 1, 7, 15, 3, 31, 9, 99];
    for &k in &tests {
        match idx.search(&k) {
            Some(v) => println!("  search({:3}) -> {}", k, v),
            None => println!("  search({:3}) -> not found", k),
        }
    }
}

fn main() {
    println!("=== Extendible Hash Index (Rust) ===");
    demo_extendible_hash();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let mut idx = ExtendibleHash::new();
        idx.insert(1, 10);
        idx.insert(2, 20);
        idx.insert(3, 30);
        assert_eq!(idx.search(&1), Some(&10));
        assert_eq!(idx.search(&2), Some(&20));
        assert_eq!(idx.search(&3), Some(&30));
        assert_eq!(idx.search(&4), None);
    }

    #[test]
    fn test_update_existing() {
        let mut idx = ExtendibleHash::new();
        idx.insert(5, 50);
        idx.insert(5, 500);
        assert_eq!(idx.search(&5), Some(&500));
    }

    #[test]
    fn test_remove() {
        let mut idx = ExtendibleHash::new();
        idx.insert(42, 420);
        assert!(idx.search(&42).is_some());
        assert!(idx.remove(&42));
        assert!(idx.search(&42).is_none());
        assert!(!idx.remove(&42));
    }

    #[test]
    fn test_directory_doubling() {
        let mut idx = ExtendibleHash::new();
        for i in 0..100 {
            idx.insert(i, i * 10);
        }
        for i in 0..100 {
            assert_eq!(idx.search(&i), Some(&(i * 10)));
        }
    }

    #[test]
    fn test_string_keys() {
        let mut idx = ExtendibleHash::new();
        idx.insert("hello".to_string(), 1);
        idx.insert("world".to_string(), 2);
        assert_eq!(idx.search(&"hello".to_string()), Some(&1));
        assert_eq!(idx.search(&"world".to_string()), Some(&2));
    }
}

// Build a Distributed KV Store with Raft + MVCC (Rust version)
// Run: rustc main.rs && ./main
//
// This implements a multi-version key-value store in Rust using BTreeMap
// for sorted version chains and RwLock for thread safety.

use std::collections::BTreeMap;
use std::sync::RwLock;

#[derive(Clone, Debug)]
struct Version {
    value: Option<String>, // None = tombstone
    created_at: u64,
}

struct MVCCStore {
    versions: RwLock<BTreeMap<String, Vec<Version>>>,
    global_version: RwLock<u64>,
}

impl MVCCStore {
    fn new() -> Self {
        MVCCStore {
            versions: RwLock::new(BTreeMap::new()),
            global_version: RwLock::new(0),
        }
    }

    fn apply_put(&self, key: &str, value: &str) -> u64 {
        let mut gv = self.global_version.write().unwrap();
        *gv += 1;
        let v = Version {
            value: Some(value.to_string()),
            created_at: *gv,
        };
        let mut versions = self.versions.write().unwrap();
        versions.entry(key.to_string()).or_default().push(v);
        *gv
    }

    fn apply_delete(&self, key: &str) -> u64 {
        let mut gv = self.global_version.write().unwrap();
        *gv += 1;
        let v = Version {
            value: None,
            created_at: *gv,
        };
        let mut versions = self.versions.write().unwrap();
        versions.entry(key.to_string()).or_default().push(v);
        *gv
    }

    fn get_at(&self, key: &str, read_version: u64) -> Option<String> {
        let versions = self.versions.read().unwrap();
        let chain = versions.get(key)?;
        let idx = chain.partition_point(|v| v.created_at <= read_version);
        if idx == 0 { return None; }
        chain[idx - 1].value.clone()
    }
}

fn main() {
    let store = MVCCStore::new();
    store.apply_put("name", "Alice");
    store.apply_put("age", "30");
    store.apply_put("name", "Bob");
    store.apply_delete("age");

    println!("Latest name: {:?}", store.get_at("name", u64::MAX));
    println!("Name at v1: {:?}", store.get_at("name", 1));
    println!("Age at v2: {:?}", store.get_at("age", 2));
}

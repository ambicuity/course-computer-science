//! Phase Capstone — Build an MVCC KV Store with a SQL Frontend
//! Phase 10 — Databases & Storage Systems
//!
//! Run:   cargo build && cargo run
//! Test:  cargo test
//!
//! This is a complete embedded database with:
//! - Slotted page storage (Lesson 05)
//! - Buffer pool with Clock eviction (Lesson 06)
//! - B+ Tree index (Lesson 07)
//! - LSM-Tree write path (Lesson 09)
//! - MVCC transaction manager (Lessons 13-15)
//! - WAL / ARIES crash recovery (Lesson 16)
//! - SQL parser + planner + executor (Lessons 02-03, 10-12)

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

// ============================================================================
// Constants
// ============================================================================

const PAGE_SIZE: usize = 4096;
const HEADER_SIZE: usize = 24;
const SLOT_ENTRY_SIZE: usize = 4;
const MAX_FRAMES: usize = 16;
const LSM_THRESHOLD: usize = 1024;
const WAL_MAGIC: u32 = 0x57414c4c;

// ============================================================================
// Binary I/O Helpers
// ============================================================================

fn put_u16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}
fn get_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}
fn put_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}
fn get_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}
fn put_u64(buf: &mut [u8], off: usize, v: u64) {
    buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
}
fn get_u64(buf: &[u8], off: usize) -> u64 {
    u64::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3],
                        buf[off + 4], buf[off + 5], buf[off + 6], buf[off + 7]])
}

fn serialize_row(values: &[Value], buf: &mut Vec<u8>) {
    for v in values {
        match v {
            Value::Null => { buf.push(0); }
            Value::Int(n) => { buf.push(1); buf.extend_from_slice(&n.to_le_bytes()); }
            Value::Text(s) => {
                buf.push(2);
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
        }
    }
}

fn deserialize_row(buf: &[u8], types: &[DataType]) -> Vec<Value> {
    let mut off = 0;
    let mut row = Vec::new();
    for t in types {
        if off >= buf.len() { row.push(Value::Null); continue; }
        let tag = buf[off];
        off += 1;
        match (tag, t) {
            (0, _) => row.push(Value::Null),
            (1, _) => {
                let n = i64::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3],
                                            buf[off+4], buf[off+5], buf[off+6], buf[off+7]]);
                off += 8;
                row.push(Value::Int(n));
            }
            (2, _) => {
                let len = u32::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3]]) as usize;
                off += 4;
                let s = String::from_utf8_lossy(&buf[off..off+len]).to_string();
                off += len;
                row.push(Value::Text(s));
            }
            _ => row.push(Value::Null),
        }
    }
    row
}

// ============================================================================
// Data Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Null,
    Int(i64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
enum DataType {
    Int,
    Text,
}

#[derive(Debug, Clone, PartialEq)]
struct ColumnDef {
    name: String,
    dtype: DataType,
}

#[derive(Debug, Clone)]
struct Schema {
    name: String,
    columns: Vec<ColumnDef>,
    pk_idx: usize,
}

impl Schema {
    fn col_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }
    fn data_types(&self) -> Vec<DataType> {
        self.columns.iter().map(|c| c.dtype.clone()).collect()
    }
    fn data_size_hint(&self) -> usize {
        let mut sz = 16;
        for c in &self.columns {
            match c.dtype {
                DataType::Int => sz += 9,
                DataType::Text => sz += 13,
            }
        }
        sz
    }
}

impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::Null => "NULL".to_string(),
            Value::Int(n) => n.to_string(),
            Value::Text(s) => s.clone(),
        }
    }
}

// ============================================================================
// Slotted Page (Lesson 05)
// ============================================================================

struct SlottedPage {
    buffer: [u8; PAGE_SIZE],
}

impl SlottedPage {
    fn new(page_id: u32) -> Self {
        let mut sp = SlottedPage { buffer: [0u8; PAGE_SIZE] };
        put_u32(&mut sp.buffer, 0, page_id);
        put_u16(&mut sp.buffer, 4, HEADER_SIZE as u16);
        put_u16(&mut sp.buffer, 6, PAGE_SIZE as u16);
        put_u16(&mut sp.buffer, 8, 0);
        put_u16(&mut sp.buffer, 10, 0);
        sp
    }

    fn page_id(&self) -> u32 { get_u32(&self.buffer, 0) }
    fn free_start(&self) -> u16 { get_u16(&self.buffer, 4) }
    fn data_end(&self) -> u16 { get_u16(&self.buffer, 6) }
    fn slot_count(&self) -> u16 { get_u16(&self.buffer, 8) }
    fn lsn(&self) -> u16 { get_u16(&self.buffer, 10) }

    fn set_free_start(&mut self, v: u16) { put_u16(&mut self.buffer, 4, v); }
    fn set_data_end(&mut self, v: u16) { put_u16(&mut self.buffer, 6, v); }
    fn set_slot_count(&mut self, v: u16) { put_u16(&mut self.buffer, 8, v); }
    fn set_lsn(&mut self, v: u16) { put_u16(&mut self.buffer, 10, v); }

    fn slot_off(&self, idx: u16) -> u16 {
        get_u16(&self.buffer, HEADER_SIZE + idx as usize * SLOT_ENTRY_SIZE)
    }
    fn slot_len(&self, idx: u16) -> u16 {
        get_u16(&self.buffer, HEADER_SIZE + idx as usize * SLOT_ENTRY_SIZE + 2)
    }
    fn set_slot(&mut self, idx: u16, off: u16, len: u16) {
        let pos = HEADER_SIZE + idx as usize * SLOT_ENTRY_SIZE;
        put_u16(&mut self.buffer, pos, off);
        put_u16(&mut self.buffer, pos + 2, len);
    }

    fn free_space(&self) -> usize {
        (self.data_end() as usize).saturating_sub(self.free_start() as usize)
    }

    fn insert(&mut self, data: &[u8]) -> Option<u16> {
        let needed = data.len() + SLOT_ENTRY_SIZE;
        if needed > self.free_space() {
            self.defragment();
            if needed > self.free_space() { return None; }
        }
        let slot = self.slot_count();
        let new_de = self.data_end() as usize - data.len();
        self.buffer[new_de..new_de + data.len()].copy_from_slice(data);
        self.set_slot(slot, new_de as u16, data.len() as u16);
        self.set_data_end(new_de as u16);
        self.set_slot_count(slot + 1);
        self.set_free_start((HEADER_SIZE + (slot + 1) as usize * SLOT_ENTRY_SIZE) as u16);
        Some(slot)
    }

    fn get(&self, slot: u16) -> Option<&[u8]> {
        if slot >= self.slot_count() { return None; }
        let off = self.slot_off(slot);
        let len = self.slot_len(slot);
        if off == 0 && len == 0 { return None; }
        Some(&self.buffer[off as usize..off as usize + len as usize])
    }

    fn delete(&mut self, slot: u16) {
        if slot >= self.slot_count() { return; }
        self.set_slot(slot, 0, 0);
    }

    fn update(&mut self, slot: u16, data: &[u8]) -> bool {
        if slot >= self.slot_count() { return false; }
        let old_len = self.slot_len(slot) as usize;
        if data.len() <= old_len {
            let off = self.slot_off(slot) as usize;
            self.buffer[off..off + data.len()].copy_from_slice(data);
            self.set_slot(slot, off as u16, data.len() as u16);
            return true;
        }
        self.delete(slot);
        match self.insert(data) {
            Some(_) => true,
            None => false,
        }
    }

    fn has_slot(&self, slot: u16) -> bool {
        if slot >= self.slot_count() { return false; }
        let off = self.slot_off(slot);
        off != 0 || self.slot_len(slot) != 0
    }

    fn defragment(&mut self) {
        let count = self.slot_count() as usize;
        let mut live: Vec<(usize, Vec<u8>)> = Vec::new();
        for i in 0..count {
            if self.has_slot(i as u16) {
                if let Some(data) = self.get(i as u16) {
                    live.push((i, data.to_vec()));
                }
            }
        }
        self.set_data_end(PAGE_SIZE as u16);
        self.set_free_start((HEADER_SIZE + count * SLOT_ENTRY_SIZE) as u16);
        for (slot, data) in &live {
            let new_de = self.data_end() as usize - data.len();
            self.buffer[new_de..new_de + data.len()].copy_from_slice(data);
            self.set_slot(*slot as u16, new_de as u16, data.len() as u16);
            self.set_data_end(new_de as u16);
        }
    }
}

// ============================================================================
// Buffer Pool with Clock Eviction (Lesson 06)
// ============================================================================

struct Frame {
    page_id: u32,
    pin_count: u32,
    dirty: bool,
    ref_bit: bool,
    data: Vec<u8>,
}

impl Frame {
    fn new(page_id: u32) -> Self {
        let mut data = vec![0u8; PAGE_SIZE];
        put_u32(&mut data, 0, page_id);
        put_u16(&mut data, 4, HEADER_SIZE as u16);
        put_u16(&mut data, 6, PAGE_SIZE as u16);
        Frame { page_id, pin_count: 0, dirty: false, ref_bit: true, data }
    }
}

struct BufferPool {
    frames: Vec<Option<Frame>>,
    hand: usize,
    num_pages: u32,
}

impl BufferPool {
    fn new() -> Self {
        let mut frames = Vec::with_capacity(MAX_FRAMES);
        for _ in 0..MAX_FRAMES { frames.push(None); }
        BufferPool { frames, hand: 0, num_pages: 0 }
    }

    fn pin(&mut self, page_id: u32, data_file: &mut HeapFile) -> &mut [u8] {
        for i in 0..MAX_FRAMES {
            if let Some(ref f) = self.frames[i] {
                if f.page_id == page_id {
                    let frame = self.frames[i].as_mut().unwrap();
                    frame.pin_count += 1;
                    frame.ref_bit = true;
                    return &mut frame.data;
                }
            }
        }
        let evicted = self.evict(data_file);
        let slot = self.frames[evicted].as_mut().unwrap();
        slot.pin_count = 1;
        slot.dirty = false;
        slot.ref_bit = true;
        slot.page_id = page_id;
        if data_file.page_count() > page_id as u64 {
            slot.data = data_file.read_page(page_id).unwrap_or_else(|| {
                let mut d = vec![0u8; PAGE_SIZE];
                put_u32(&mut d, 0, page_id);
                put_u16(&mut d, 4, HEADER_SIZE as u16);
                put_u16(&mut d, 6, PAGE_SIZE as u16);
                d
            });
        } else {
            let mut d = vec![0u8; PAGE_SIZE];
            put_u32(&mut d, 0, page_id);
            put_u16(&mut d, 4, HEADER_SIZE as u16);
            put_u16(&mut d, 6, PAGE_SIZE as u16);
            slot.data = d;
        }
        &mut slot.data
    }

    fn unpin(&mut self, page_id: u32, dirty: bool) {
        for i in 0..MAX_FRAMES {
            if let Some(ref mut f) = self.frames[i] {
                if f.page_id == page_id && f.pin_count > 0 {
                    f.pin_count -= 1;
                    if dirty { f.dirty = true; }
                    return;
                }
            }
        }
    }

    fn evict(&mut self, data_file: &mut HeapFile) -> usize {
        loop {
            for _ in 0..MAX_FRAMES {
                let idx = self.hand % MAX_FRAMES;
                self.hand = (self.hand + 1) % MAX_FRAMES;
                if let Some(ref mut f) = self.frames[idx] {
                    if f.pin_count > 0 { continue; }
                    if f.ref_bit {
                        f.ref_bit = false;
                        continue;
                    }
                    if f.dirty {
                        data_file.write_page(f.page_id, &f.data);
                        f.dirty = false;
                    }
                    return idx;
                } else {
                    self.frames[idx] = Some(Frame::new(u32::MAX));
                    return idx;
                }
            }
        }
    }

    fn flush_all(&mut self, data_file: &mut HeapFile) {
        for i in 0..MAX_FRAMES {
            if let Some(ref mut f) = self.frames[i] {
                if f.dirty {
                    data_file.write_page(f.page_id, &f.data);
                    f.dirty = false;
                }
            }
        }
    }
}

// ============================================================================
// Heap File (flat page storage on disk)
// ============================================================================

struct HeapFile {
    path: String,
    file: File,
    pages: u64,
}

impl HeapFile {
    fn open(path: &str) -> Self {
        let file = OpenOptions::new().read(true).write(true).create(true)
            .open(path).expect("cannot open heap file");
        let pages = file.metadata().map(|m| m.len() / PAGE_SIZE as u64).unwrap_or(0);
        HeapFile { path: path.to_string(), file, pages }
    }

    fn page_count(&self) -> u64 { self.pages }

    fn allocate_page(&mut self) -> u32 {
        let pid = self.pages as u32;
        let mut buf = vec![0u8; PAGE_SIZE];
        put_u32(&mut buf, 0, pid);
        put_u16(&mut buf, 4, HEADER_SIZE as u16);
        put_u16(&mut buf, 6, PAGE_SIZE as u16);
        self.file.seek(SeekFrom::End(0)).expect("seek end");
        self.file.write_all(&buf).expect("write page");
        self.pages += 1;
        pid
    }

    fn read_page(&mut self, page_id: u32) -> Option<Vec<u8>> {
        if page_id as u64 >= self.pages { return None; }
        let mut buf = vec![0u8; PAGE_SIZE];
        self.file.seek(SeekFrom::Start(page_id as u64 * PAGE_SIZE as u64)).ok()?;
        self.file.read_exact(&mut buf).ok()?;
        Some(buf)
    }

    fn write_page(&mut self, page_id: u32, data: &[u8]) {
        self.file.seek(SeekFrom::Start(page_id as u64 * PAGE_SIZE as u64))
            .expect("seek for write");
        self.file.write_all(&data[..PAGE_SIZE]).expect("write page");
    }

    fn sync(&mut self) {
        self.file.flush().expect("flush");
    }

    fn close(&mut self) {
        self.sync();
    }
}

// ============================================================================
// B+ Tree Index (simplified, on-disk) — Lesson 07
// ============================================================================

#[derive(Clone)]
struct BTreeEntry {
    key: Vec<u8>,
    page_id: u32,
    slot: u16,
    begin_ts: u64,
    end_ts: u64,
}

struct BTree {
    entries: Vec<BTreeEntry>,
    modified: bool,
}

impl BTree {
    fn new() -> Self {
        BTree { entries: Vec::new(), modified: false }
    }

    fn insert(&mut self, key: Vec<u8>, page_id: u32, slot: u16, begin_ts: u64) {
        let pos = self.entries.binary_search_by(|e| e.key.as_slice().cmp(&key));
        match pos {
            Ok(idx) => {
                self.entries[idx].page_id = page_id;
                self.entries[idx].slot = slot;
                self.entries[idx].begin_ts = begin_ts;
                self.entries[idx].end_ts = 0;
            }
            Err(idx) => {
                self.entries.insert(idx, BTreeEntry {
                    key, page_id, slot, begin_ts, end_ts: 0,
                });
            }
        }
        self.modified = true;
    }

    fn mark_deleted(&mut self, key: &[u8], end_ts: u64) {
        if let Ok(idx) = self.entries.binary_search_by(|e| e.key.as_slice().cmp(key)) {
            self.entries[idx].end_ts = end_ts;
            self.modified = true;
        }
    }

    fn search(&self, key: &[u8]) -> Option<&BTreeEntry> {
        if let Ok(idx) = self.entries.binary_search_by(|e| e.key.as_slice().cmp(key)) {
            Some(&self.entries[idx])
        } else {
            None
 }
    }

    fn range_scan(&self, start: &[u8], end: &[u8]) -> Vec<&BTreeEntry> {
        let start_idx = match self.entries.binary_search_by(|e| e.key.as_slice().cmp(start)) {
            Ok(i) => i,
            Err(i) => i,
        };
        let mut results = Vec::new();
        for e in self.entries[start_idx..].iter() {
            if e.key.as_slice() > end { break; }
            results.push(e);
        }
        results
    }

    fn load(path: &str) -> Self {
        let mut bt = BTree::new();
        let path_obj = Path::new(path);
        if !path_obj.exists() { return bt; }
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => return bt,
        };
        if data.len() < 4 { return bt; }
        let count = get_u32(&data, 0) as usize;
        let mut off = 4;
        for _ in 0..count {
            if off + 16 > data.len() { break; }
            let key_len = get_u32(&data, off) as usize;
            off += 4;
            if off + key_len + 16 > data.len() { break; }
            let key = data[off..off + key_len].to_vec();
            off += key_len;
            let page_id = get_u32(&data, off);
            off += 4;
            let slot = get_u16(&data, off);
            off += 2;
            let begin_ts = get_u64(&data, off);
            off += 8;
            let end_ts = get_u64(&data, off);
            off += 8;
            bt.entries.push(BTreeEntry { key, page_id, slot, begin_ts, end_ts });
        }
        bt
    }

    fn save(&self, path: &str) {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for e in &self.entries {
            data.extend_from_slice(&(e.key.len() as u32).to_le_bytes());
            data.extend_from_slice(&e.key);
            data.extend_from_slice(&e.page_id.to_le_bytes());
            data.extend_from_slice(&e.slot.to_le_bytes());
            data.extend_from_slice(&e.begin_ts.to_le_bytes());
            data.extend_from_slice(&e.end_ts.to_le_bytes());
        }
        fs::write(path, &data).expect("write index");
    }
}

// ============================================================================
// LSM-Tree Engine (Lesson 09)
// ============================================================================

struct LSMTree {
    memtable: BTreeMap<Vec<u8>, Vec<u8>>,
    path: String,
    sstable_count: usize,
}

impl LSMTree {
    fn new(path: &str) -> Self {
        LSMTree {
            memtable: BTreeMap::new(),
            path: path.to_string(),
            sstable_count: 0,
        }
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.memtable.insert(key, value);
        if self.memtable.len() > LSM_THRESHOLD {
            self.flush_memtable();
        }
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if let Some(v) = self.memtable.get(key) {
            return Some(v.clone());
        }
        for i in (0..self.sstable_count).rev() {
            let sst_path = format!("{}.sst.{}", self.path, i);
            if let Some(v) = SSTable::read(&sst_path, key) {
                return Some(v);
            }
        }
        None
    }

    fn flush_memtable(&mut self) {
        if self.memtable.is_empty() { return; }
        let sst_path = format!("{}.sst.{}", self.path, self.sstable_count);
        let entries: Vec<(Vec<u8>, Vec<u8>)> = self.memtable.iter()
            .map(|(k, v)| (k.clone(), v.clone())).collect();
        SSTable::write(&sst_path, &entries);
        self.sstable_count += 1;
        self.memtable.clear();
    }

    fn flush_all(&mut self) {
        self.flush_memtable();
    }
}

struct SSTable;

impl SSTable {
    fn write(path: &str, entries: &[(Vec<u8>, Vec<u8>)]) {
        let mut data = Vec::new();
        data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (k, v) in entries {
            data.extend_from_slice(&(k.len() as u32).to_le_bytes());
            data.extend_from_slice(k);
            data.extend_from_slice(&(v.len() as u32).to_le_bytes());
            data.extend_from_slice(v);
        }
        fs::write(path, &data).expect("write SSTable");
    }

    fn read(path: &str, key: &[u8]) -> Option<Vec<u8>> {
        let data = fs::read(path).ok()?;
        if data.len() < 4 { return None; }
        let count = get_u32(&data, 0) as usize;
        let mut off = 4;
        for _ in 0..count {
            if off + 4 > data.len() { return None; }
            let klen = get_u32(&data, off) as usize;
            off += 4;
            if off + klen + 4 > data.len() { return None; }
            let k = &data[off..off + klen];
            off += klen;
            let vlen = get_u32(&data, off) as usize;
            off += 4;
            if off + vlen > data.len() { return None; }
            if k == key {
                return Some(data[off..off + vlen].to_vec());
            }
            off += vlen;
        }
        None
    }
}

// ============================================================================
// WAL — Write-Ahead Log + ARIES Recovery (Lesson 16)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum LogType {
    Begin,
    Insert,
    UpdateDelete,
    Commit,
    Abort,
    CLR,
}

#[derive(Debug, Clone)]
struct LogRecord {
    lsn: u64,
    prev_lsn: u64,
    txn_id: u64,
    log_type: LogType,
    page_id: u32,
    payload: Vec<u8>,
}

struct WalLog {
    records: Vec<LogRecord>,
    next_lsn: u64,
    path: String,
    txn_lsn_map: BTreeMap<u64, u64>,
}

impl WalLog {
    fn open(path: &str) -> Self {
        let exists = Path::new(path).exists();
        let recs = if exists {
            Self::load_records(path)
        } else {
            Vec::new()
        };
        WalLog {
            records: recs,
            next_lsn: 1,
            path: path.to_string(),
            txn_lsn_map: BTreeMap::new(),
        }
    }

    fn load_records(path: &str) -> Vec<LogRecord> {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        let mut records = Vec::new();
        let mut off = 0;
        while off + 8 <= data.len() {
            if get_u32(&data, off) != WAL_MAGIC { break; }
            let lsn = get_u64(&data, off + 4);
            let prev_lsn = get_u64(&data, off + 12);
            let txn_id = get_u64(&data, off + 20);
            let lt = data[off + 28];
            let page_id = get_u32(&data, off + 29);
            let plen = get_u32(&data, off + 33) as usize;
            off += 37;
            let payload = if plen > 0 && off + plen <= data.len() {
                data[off..off + plen].to_vec()
            } else {
                Vec::new()
            };
            off += plen;
            let log_type = match lt {
                0 => LogType::Begin,
                1 => LogType::Insert,
                2 => LogType::UpdateDelete,
                3 => LogType::Commit,
                4 => LogType::Abort,
                5 => LogType::CLR,
                _ => continue,
            };
            records.push(LogRecord { lsn, prev_lsn, txn_id, log_type, page_id, payload });
        }
        records
    }

    fn append(&mut self, txn_id: u64, log_type: LogType, page_id: u32, payload: &[u8]) -> u64 {
        let lsn = self.next_lsn;
        self.next_lsn += 1;
        let prev_lsn = self.txn_lsn_map.get(&txn_id).copied().unwrap_or(0);
        self.txn_lsn_map.insert(txn_id, lsn);
        self.records.push(LogRecord {
            lsn, prev_lsn, txn_id, log_type, page_id, payload: payload.to_vec(),
        });
        let mut buf = Vec::new();
        buf.extend_from_slice(&WAL_MAGIC.to_le_bytes());
        buf.extend_from_slice(&lsn.to_le_bytes());
        buf.extend_from_slice(&prev_lsn.to_le_bytes());
        buf.extend_from_slice(&txn_id.to_le_bytes());
        let lt_byte: u8 = match log_type {
            LogType::Begin => 0,
            LogType::Insert => 1,
            LogType::UpdateDelete => 2,
            LogType::Commit => 3,
            LogType::Abort => 4,
            LogType::CLR => 5,
        };
        buf.push(lt_byte);
        buf.extend_from_slice(&page_id.to_le_bytes());
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(payload);
        let mut f = OpenOptions::new().create(true).append(true)
            .open(&self.path).expect("open WAL");
        f.write_all(&buf).expect("write WAL");
        f.flush().expect("flush WAL");
        lsn
    }

    fn analyze(&self) -> (BTreeMap<u64, (u64, bool)>, BTreeMap<u32, u64>, u64) {
        let mut txn_table: BTreeMap<u64, (u64, bool)> = BTreeMap::new();
        let mut dirty_pages: BTreeMap<u32, u64> = BTreeMap::new();
        let mut max_lsn = 0;
        for r in &self.records {
            if r.lsn > max_lsn { max_lsn = r.lsn; }
            match r.log_type {
                LogType::Begin => { txn_table.entry(r.txn_id).or_insert((r.lsn, true)); }
                LogType::Commit | LogType::Abort => {
                    txn_table.insert(r.txn_id, (r.lsn, false));
                }
                _ => {
                    txn_table.entry(r.txn_id).or_insert((r.lsn, true));
                    dirty_pages.entry(r.page_id).or_insert(r.lsn);
                }
            }
        }
        (txn_table, dirty_pages, max_lsn)
    }

    fn redo(&self, pool: &mut BufferPool, data_file: &mut HeapFile, min_lsn: u64) {
        for r in &self.records {
            if r.lsn < min_lsn { continue; }
            match r.log_type {
                LogType::Insert | LogType::UpdateDelete => {
                    let page = pool.pin(r.page_id, data_file);
                    let page_lsn = get_u16(page, 10) as u64;
                    if page_lsn < r.lsn {
                        let slotted = SlottedPage::new(r.page_id);
                        let data_end = slotted.data_end();
                        let slot_count = slotted.slot_count();
                        put_u16(page, 6, data_end);
                        put_u16(page, 8, slot_count);
                        page[24..24 + r.payload.len()].copy_from_slice(&r.payload);
                        put_u16(page, 10, r.lsn as u16);
                    }
                    pool.unpin(r.page_id, true);
                }
                _ => {}
            }
        }
    }

    fn undo(&mut self, _pool: &mut BufferPool, _data_file: &mut HeapFile) {
        let (txn_table, _, _) = self.analyze();
        let active: Vec<u64> = txn_table.iter()
            .filter(|(_, (_, active))| *active)
            .map(|(id, _)| *id)
            .collect();
        if active.is_empty() { return; }

        let mut undo_stack: Vec<(u64, u64)> = Vec::new();
        for tid in &active {
            let mut prev_lsn = self.txn_lsn_map.get(tid).copied().unwrap_or(0);
            while prev_lsn > 0 {
                if let Some(rec) = self.records.iter().find(|r| r.lsn == prev_lsn) {
                    if rec.log_type == LogType::Insert || rec.log_type == LogType::UpdateDelete {
                        undo_stack.push((prev_lsn, rec.txn_id));
                    }
                    prev_lsn = rec.prev_lsn;
                } else {
                    break;
                }
            }
        }
        undo_stack.sort_by(|a, b| b.0.cmp(&a.0));
        for (lsn, tid) in &undo_stack {
            if let Some(rec) = self.records.iter().find(|r| r.lsn == *lsn) {
                if rec.log_type == LogType::UpdateDelete && rec.payload.len() >= 8 {
                    let txn_bytes = tid.to_le_bytes();
                    self.append(*tid, LogType::CLR, rec.page_id, &txn_bytes);
                }
            }
        }
        for tid in &active {
            self.append(*tid, LogType::Abort, 0, &[]);
        }
    }

    fn recover(&mut self, pool: &mut BufferPool, data_file: &mut HeapFile) {
        let (_, dirty_pages, _) = self.analyze();
        let min_lsn = dirty_pages.values().min().copied().unwrap_or(0);
        self.redo(pool, data_file, min_lsn);
        self.undo(pool, data_file);
    }
}

// ============================================================================
// MVCC Transaction Manager (Lessons 13-15)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum TxnStatus { Active, Committed, Aborted }

struct Transaction {
    id: u64,
    snapshot_ts: u64,
    status: TxnStatus,
    modified_pages: Vec<(u32, u16, Vec<u8>)>,
}

struct TransactionManager {
    next_id: u64,
    committed_txns: Vec<(u64, u64)>,
}

impl TransactionManager {
    fn new() -> Self {
        TransactionManager { next_id: 1, committed_txns: Vec::new() }
    }

    fn begin(&mut self) -> Transaction {
        let id = self.next_id;
        self.next_id += 1;
        let max_committed = self.committed_txns.iter()
            .map(|(_, ct)| *ct).max().unwrap_or(0);
        Transaction {
            id,
            snapshot_ts: max_committed,
            status: TxnStatus::Active,
            modified_pages: Vec::new(),
        }
    }

    fn commit(&mut self, txn: &mut Transaction) -> bool {
        if txn.status != TxnStatus::Active { return false; }
        txn.status = TxnStatus::Committed;
        self.committed_txns.push((txn.id, txn.id));
        self.committed_txns.retain(|(_, ct)| {
            let cutoff = txn.id.saturating_sub(100);
            *ct > cutoff
        });
        true
    }

    fn rollback(&mut self, txn: &mut Transaction) {
        txn.status = TxnStatus::Aborted;
    }

    fn is_visible(&self, begin_ts: u64, end_ts: u64, snapshot_ts: u64, txn_id: u64) -> bool {
        if begin_ts == txn_id { return true; }
        let begin_committed = begin_ts <= snapshot_ts
            || self.committed_txns.iter().any(|(id, _)| *id == begin_ts && begin_ts <= snapshot_ts);
        if !begin_committed { return false; }
        if end_ts == 0 { return true; }
        let end_is_txn = end_ts == txn_id;
        let end_committed = self.committed_txns.iter()
            .any(|(_, ct)| *ct == end_ts);
        !end_committed || end_is_txn
    }
}

// ============================================================================
// SQL Parser (Lessons 02-03)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Create, Table, Into, Values, Select, From, Where,
    Insert, Delete, Update, Set, Begin, Commit, Rollback,
    Transaction, Start, Index, On,
    And, Or, Not, Null,
    Int, Text, Primary, Key,
    Star, Eq, Lt, Gt, Le, Ge, Ne,
    LParen, RParen, Comma, Semicolon,
    Ident(String), Number(i64), String(String),
    Exit, Unknown(String),
}

fn tokenize(sql: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch.is_whitespace() { i += 1; continue; }
        if ch == '-' && i + 1 < bytes.len() && bytes[i + 1] as char == '-' {
            while i < bytes.len() && bytes[i] as char != '\n' { i += 1; }
            continue;
        }
        if ch == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] as char == '*' && bytes[i + 1] as char == '/') { i += 1; }
            i += 2;
            continue;
        }
        match ch {
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            ',' => { tokens.push(Token::Comma); i += 1; }
            ';' => { tokens.push(Token::Semicolon); i += 1; }
            '*' => { tokens.push(Token::Star); i += 1; }
            '=' => { tokens.push(Token::Eq); i += 1; }
            '<' => {
                if i + 1 < bytes.len() && bytes[i + 1] as char == '>' {
                    tokens.push(Token::Ne); i += 2;
                } else if i + 1 < bytes.len() && bytes[i + 1] as char == '=' {
                    tokens.push(Token::Le); i += 2;
                } else {
                    tokens.push(Token::Lt); i += 1;
                }
            }
            '>' => {
                if i + 1 < bytes.len() && bytes[i + 1] as char == '=' {
                    tokens.push(Token::Ge); i += 2;
                } else {
                    tokens.push(Token::Gt); i += 1;
                }
            }
            '\'' => {
                let start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] as char != '\'' { i += 1; }
                let s = String::from_utf8_lossy(&bytes[start..i]).to_string();
                tokens.push(Token::String(s));
                if i < bytes.len() { i += 1; }
            }
            ch if ch.is_ascii_digit() || (ch == '-' && i + 1 < bytes.len() && (bytes[i+1] as char).is_ascii_digit()) => {
                let start = i;
                if bytes[i] as char == '-' { i += 1; }
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() { i += 1; }
                let n: i64 = String::from_utf8_lossy(&bytes[start..i]).parse().unwrap_or(0);
                tokens.push(Token::Number(n));
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let start = i;
                while i < bytes.len() && ((bytes[i] as char).is_alphanumeric() || bytes[i] as char == '_') { i += 1; }
                let word = String::from_utf8_lossy(&bytes[start..i]).to_string().to_uppercase();
                let tok = match word.as_str() {
                    "CREATE" => Token::Create, "TABLE" => Token::Table,
                    "INTO" => Token::Into, "VALUES" => Token::Values,
                    "SELECT" => Token::Select, "FROM" => Token::From,
                    "WHERE" => Token::Where, "INSERT" => Token::Insert,
                    "DELETE" => Token::Delete, "UPDATE" => Token::Update,
                    "SET" => Token::Set, "BEGIN" => Token::Begin,
                    "COMMIT" => Token::Commit, "ROLLBACK" => Token::Rollback,
                    "TRANSACTION" => Token::Transaction,
                    "START" => Token::Start, "AND" => Token::And,
                    "OR" => Token::Or, "NOT" => Token::Not,
                    "NULL" => Token::Null, "INT" => Token::Int,
                    "TEXT" => Token::Text, "PRIMARY" => Token::Primary,
                    "KEY" => Token::Key, "INDEX" => Token::Index,
                    "ON" => Token::On, "EXIT" => Token::Exit,
                    _ => Token::Ident(String::from_utf8_lossy(&bytes[start..i]).to_string()),
                };
                tokens.push(tok);
            }
            _ => {
                tokens.push(Token::Unknown(ch.to_string()));
                i += 1;
            }
        }
    }
    tokens
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Column(String),
    Value(Value),
    BinOp(Box<Expr>, Op, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum Op { Eq, Ne, Lt, Gt, Le, Ge }

#[derive(Debug, Clone, PartialEq)]
enum Statement {
    CreateTable { name: String, columns: Vec<ColumnDef>, pk_col: Option<usize> },
    Insert { table: String, values: Vec<Vec<Value>> },
    Select { columns: Vec<String>, table: String, where_clause: Option<Expr>, table_alias: Option<String> },
    Update { table: String, assignments: Vec<(String, Value)>, where_clause: Option<Expr> },
    Delete { table: String, where_clause: Option<Expr> },
    Begin,
    Commit,
    Rollback,
    CreateIndex { table: String, column: String },
    Exit,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, String> {
        match self.advance() {
            Some(tok) if tok == *expected => Ok(tok),
            Some(tok) => Err(format!("expected {:?}, got {:?}", expected, tok)),
            None => Err(format!("expected {:?}, got EOF", expected)),
        }
    }

    fn parse(&mut self) -> Result<Statement, String> {
        let tok = self.peek().ok_or("empty statement")?.clone();
        match tok {
            Token::Create => {
                self.advance();
                match self.peek() {
                    Some(Token::Table) => self.parse_create_table(),
                    Some(Token::Index) => self.parse_create_index(),
                    Some(t) => Err(format!("expected TABLE or INDEX after CREATE, got {:?}", t)),
                    None => Err("unexpected EOF after CREATE".to_string()),
                }
            }
            Token::Insert => { self.advance(); self.parse_insert() }
            Token::Select => { self.advance(); self.parse_select() }
            Token::Update => { self.advance(); self.parse_update() }
            Token::Delete => { self.advance(); self.parse_delete() }
            Token::Begin => { self.advance(); self.parse_begin() }
            Token::Commit => { self.advance(); self.parse_commit() }
            Token::Rollback => { self.advance(); self.parse_rollback() }
            Token::Exit => { self.advance(); Ok(Statement::Exit) }
            _ => Err(format!("unexpected token {:?}", tok)),
        }
    }

    fn parse_create_table(&mut self) -> Result<Statement, String> {
        self.expect(&Token::Table)?;
        let name = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected table name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        self.expect(&Token::LParen)?;
        let mut columns = Vec::new();
        let mut pk_col = None;
        loop {
            let next = self.peek().cloned();
            match next {
                Some(Token::RParen) => { self.advance(); break; }
                Some(Token::Comma) => { self.advance(); continue; }
                Some(Token::Primary) => {
                    self.advance();
                    self.expect(&Token::Key)?;
                    self.expect(&Token::LParen)?;
                    let pk_name = match self.advance() {
                        Some(Token::Ident(s)) => s,
                        Some(t) => return Err(format!("expected column name in PK, got {:?}", t)),
                        None => return Err("unexpected EOF".to_string()),
                    };
                    pk_col = columns.iter().position(|c: &ColumnDef| c.name == pk_name);
                    self.expect(&Token::RParen)?;
                }
                Some(Token::Ident(col_name)) => {
                    self.advance();
                    let next2 = self.peek().cloned();
                    let dtype = match next2 {
                        Some(Token::Int) => { self.advance(); DataType::Int }
                        Some(Token::Text) => { self.advance(); DataType::Text }
                        Some(t) => return Err(format!("expected INT or TEXT, got {:?}", t)),
                        None => return Err("unexpected EOF".to_string()),
                    };
                    let col_def = ColumnDef { name: col_name, dtype };
                    columns.push(col_def);
                }
                Some(t) => return Err(format!("unexpected token in CREATE TABLE: {:?}", t)),
                None => return Err("unexpected EOF".to_string()),
            }
        }
        if pk_col.is_none() && !columns.is_empty() {
            pk_col = Some(0);
        }
        Ok(Statement::CreateTable { name, columns, pk_col })
    }

    fn parse_create_index(&mut self) -> Result<Statement, String> {
        self.expect(&Token::Index)?;
        let _idx_name = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected index name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        self.expect(&Token::On)?;
        let table = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected table name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        self.expect(&Token::LParen)?;
        let column = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected column name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        self.expect(&Token::RParen)?;
        Ok(Statement::CreateIndex { table, column })
    }

    fn parse_insert(&mut self) -> Result<Statement, String> {
        self.expect(&Token::Into)?;
        let table = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected table name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        self.expect(&Token::Values)?;
        self.expect(&Token::LParen)?;
        let mut values = Vec::new();
        loop {
            let next = self.peek().cloned();
            match next {
                Some(Token::RParen) => { self.advance(); break; }
                Some(Token::Comma) => { self.advance(); }
                Some(Token::Number(n)) => { self.advance(); values.push(Value::Int(n)); }
                Some(Token::String(s)) => { self.advance(); values.push(Value::Text(s)); }
                Some(Token::Null) => { self.advance(); values.push(Value::Null); }
                Some(t) => return Err(format!("unexpected token in VALUES: {:?}", t)),
                None => return Err("unexpected EOF".to_string()),
            }
        }
        Ok(Statement::Insert { table, values: vec![values] })
    }

    fn parse_select(&mut self) -> Result<Statement, String> {
        let mut columns = Vec::new();
        loop {
            let next = self.peek().cloned();
            match next {
                Some(Token::Star) => { self.advance(); columns.push("*".to_string()); }
                Some(Token::Ident(s)) => {
                    self.advance();
                    if s.to_uppercase() == "DISTINCT" { continue; }
                    columns.push(s);
                }
                Some(_) => return Err(format!("expected column list, got {:?}", self.peek())),
                None => return Err("unexpected EOF".to_string()),
            }
            let next = self.peek().cloned();
            match next {
                Some(Token::Comma) => { self.advance(); }
                Some(Token::From) => { break; }
                Some(t) => return Err(format!("expected FROM or comma, got {:?}", t)),
                None => return Err("unexpected EOF".to_string()),
            }
        }
        self.expect(&Token::From)?;
        let table = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected table name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        let mut table_alias = None;
        let alias_tok = self.peek().cloned();
        if let Some(Token::Ident(ref a)) = alias_tok {
            let uc = a.to_uppercase();
            if uc != "WHERE" && uc != "ORDER" && uc != "GROUP" && uc != "LIMIT" && uc != "JOIN" && uc != "INNER" && uc != "LEFT" {
                self.advance();
                table_alias = Some(a.clone());
            }
        }
        let mut where_clause = None;
        if let Some(Token::Where) = self.peek() {
            self.advance();
            where_clause = Some(self.parse_expr()?);
        }
        Ok(Statement::Select { columns, table, where_clause, table_alias })
    }

    fn parse_update(&mut self) -> Result<Statement, String> {
        let table = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected table name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        self.expect(&Token::Set)?;
        let mut assignments = Vec::new();
        loop {
            let col = match self.advance() {
                Some(Token::Ident(s)) => s,
                Some(t) => return Err(format!("expected column name, got {:?}", t)),
                None => return Err("unexpected EOF".to_string()),
            };
            self.expect(&Token::Eq)?;
            let val = match self.advance() {
                Some(Token::Number(n)) => Value::Int(n),
                Some(Token::String(s)) => Value::Text(s),
                Some(Token::Null) => Value::Null,
                Some(t) => return Err(format!("expected value, got {:?}", t)),
                None => return Err("unexpected EOF".to_string()),
            };
            assignments.push((col, val));
            match self.peek() {
                Some(Token::Comma) => { self.advance(); }
                _ => { break; }
            }
        }
        let mut where_clause = None;
        if let Some(Token::Where) = self.peek() {
            self.advance();
            where_clause = Some(self.parse_expr()?);
        }
        Ok(Statement::Update { table, assignments, where_clause })
    }

    fn parse_delete(&mut self) -> Result<Statement, String> {
        self.expect(&Token::From)?;
        let table = match self.advance() {
            Some(Token::Ident(s)) => s,
            Some(t) => return Err(format!("expected table name, got {:?}", t)),
            None => return Err("unexpected EOF".to_string()),
        };
        let mut where_clause = None;
        if let Some(Token::Where) = self.peek() {
            self.advance();
            where_clause = Some(self.parse_expr()?);
        }
        Ok(Statement::Delete { table, where_clause })
    }

    fn parse_begin(&mut self) -> Result<Statement, String> {
        if let Some(Token::Transaction) = self.peek() { self.advance(); }
        Ok(Statement::Begin)
    }

    fn parse_commit(&mut self) -> Result<Statement, String> {
        Ok(Statement::Commit)
    }

    fn parse_rollback(&mut self) -> Result<Statement, String> {
        Ok(Statement::Rollback)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_comparand()?;
        if let Some(op) = self.peek() {
            let op = match op {
                Token::Eq => { self.advance(); Op::Eq }
                Token::Ne => { self.advance(); Op::Ne }
                Token::Lt => { self.advance(); Op::Lt }
                Token::Gt => { self.advance(); Op::Gt }
                Token::Le => { self.advance(); Op::Le }
                Token::Ge => { self.advance(); Op::Ge }
                _ => return Ok(left),
            };
            let right = self.parse_comparand()?;
            Ok(Expr::BinOp(Box::new(left), op, Box::new(right)))
        } else {
            Ok(left)
        }
    }

    fn parse_comparand(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(Expr::Column(s)),
            Some(Token::Number(n)) => Ok(Expr::Value(Value::Int(n))),
            Some(Token::String(s)) => Ok(Expr::Value(Value::Text(s))),
            Some(Token::Null) => Ok(Expr::Value(Value::Null)),
            Some(t) => Err(format!("expected expression, got {:?}", t)),
            None => Err("unexpected EOF in expression".to_string()),
        }
    }
}

// ============================================================================
// Database — Unified Storage + Transaction + Query Engine
// ============================================================================

struct Database {
    data_dir: String,
    schemas: BTreeMap<String, Schema>,
    btrees: BTreeMap<String, BTree>,
    lsm_trees: BTreeMap<String, LSMTree>,
    wal: WalLog,
    txn_mgr: TransactionManager,
    current_txn: Option<Transaction>,
    pool: BufferPool,
    data_files: BTreeMap<String, HeapFile>,
}

impl Database {
    fn new(data_dir: &str) -> Self {
        fs::create_dir_all(data_dir).ok();
        let wal = WalLog::open(&format!("{}/wal.log", data_dir));
        let mut db = Database {
            data_dir: data_dir.to_string(),
            schemas: BTreeMap::new(),
            btrees: BTreeMap::new(),
            lsm_trees: BTreeMap::new(),
            wal,
            txn_mgr: TransactionManager::new(),
            current_txn: None,
            pool: BufferPool::new(),
            data_files: BTreeMap::new(),
        };
        db.load_catalog();
        db
    }

    fn load_catalog(&mut self) {
        let cat_path = format!("{}/catalog.dat", self.data_dir);
        if !Path::new(&cat_path).exists() { return; }
        let data = match fs::read(&cat_path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let count = get_u32(&data, 0) as usize;
        let mut off = 4;
        for _ in 0..count {
            if off + 4 > data.len() { break; }
            let name_len = get_u32(&data, off) as usize;
            off += 4;
            if off + name_len + 4 > data.len() { break; }
            let name = String::from_utf8_lossy(&data[off..off + name_len]).to_string();
            off += name_len;
            let col_count = get_u32(&data, off) as usize;
            off += 4;
            let mut columns = Vec::new();
            let mut pk_idx = 0;
            for _ in 0..col_count {
                if off + 4 > data.len() { break; }
                let cname_len = get_u32(&data, off) as usize;
                off += 4;
                if off + cname_len + 1 > data.len() { break; }
                let cname = String::from_utf8_lossy(&data[off..off + cname_len]).to_string();
                off += cname_len;
                let dtype = if data[off] == 0 { DataType::Int } else { DataType::Text };
                off += 1;
                if off >= data.len() { break; }
                let is_pk = data[off] != 0;
                off += 1;
                if is_pk { pk_idx = columns.len(); }
                columns.push(ColumnDef { name: cname, dtype });
            }
            let schema_name = name.clone();
            let schema = Schema { name, columns, pk_idx };
            let idx_path = self.index_path(&schema.name);
            let btree = BTree::load(&idx_path);
            let lsm_path = format!("{}/{}", self.data_dir, schema.name);
            let lsm_tree = LSMTree::new(&lsm_path);
            let data_path = self.data_path(&schema.name);
            let data_file = HeapFile::open(&data_path);
            self.schemas.insert(schema_name.clone(), schema);
            self.btrees.insert(schema_name.clone(), btree);
            self.lsm_trees.insert(schema_name.clone(), lsm_tree);
            self.data_files.insert(schema_name, data_file);
        }
    }

    fn save_catalog(&self) {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.schemas.len() as u32).to_le_bytes());
        for (name, schema) in &self.schemas {
            data.extend_from_slice(&(name.len() as u32).to_le_bytes());
            data.extend_from_slice(name.as_bytes());
            data.extend_from_slice(&(schema.columns.len() as u32).to_le_bytes());
            for (i, col) in schema.columns.iter().enumerate() {
                data.extend_from_slice(&(col.name.len() as u32).to_le_bytes());
                data.extend_from_slice(col.name.as_bytes());
                data.push(match col.dtype { DataType::Int => 0, DataType::Text => 1 });
                data.push(if i == schema.pk_idx { 1 } else { 0 });
            }
        }
        fs::write(format!("{}/catalog.dat", self.data_dir), &data).expect("save catalog");
    }

    fn save_indexes(&self) {
        for (name, btree) in &self.btrees {
            if btree.modified {
                btree.save(&self.index_path(name));
            }
        }
    }

    fn close(&mut self) {
        self.save_indexes();
        self.save_catalog();
        for df in self.data_files.values_mut() {
            df.close();
        }
    }

    fn data_path(&self, name: &str) -> String {
        format!("{}/{}.data", self.data_dir, name)
    }

    fn index_path(&self, name: &str) -> String {
        format!("{}/{}.idx", self.data_dir, name)
    }

    fn data_file(&mut self, name: &str) -> &mut HeapFile {
        self.data_files.get_mut(name).expect("unknown table")
    }

    #[allow(unused)]
    fn ensure_txn(&mut self) {
        if self.current_txn.is_none() {
            let txn = self.txn_mgr.begin();
            self.wal.append(txn.id, LogType::Begin, 0, &[]);
            self.current_txn = Some(txn);
        }
    }

    fn execute(&mut self, stmt: &Statement) -> String {
        let is_dml = matches!(stmt,
            Statement::Insert{..} | Statement::Select{..} |
            Statement::Update{..} | Statement::Delete{..}
        );
        let is_txn_ctrl = matches!(stmt,
            Statement::Begin | Statement::Commit | Statement::Rollback
        );
        let auto_start = if (is_dml || is_txn_ctrl) && self.current_txn.is_none() && !is_txn_ctrl {
            let txn = self.txn_mgr.begin();
            self.wal.append(txn.id, LogType::Begin, 0, &[]);
            self.current_txn = Some(txn);
            true
        } else {
            false
        };

        let result = match stmt {
            Statement::CreateTable { name, columns, pk_col } => {
                if self.schemas.contains_key(name) {
                    return format!("Error: table '{}' already exists", name);
                }
                let pk = pk_col.unwrap_or(0);
                let schema = Schema {
                    name: name.clone(),
                    columns: columns.clone(),
                    pk_idx: pk,
                };
                let df = HeapFile::open(&self.data_path(name));
                self.data_files.insert(name.clone(), df);
                self.btrees.insert(name.clone(), BTree::new());
                self.lsm_trees.insert(name.clone(), LSMTree::new(&format!("{}/{}", self.data_dir, name)));
                self.schemas.insert(name.clone(), schema);
                self.save_catalog();
                "OK".to_string()
            }
            Statement::CreateIndex { table, column } => {
                let schema = match self.schemas.get(table) {
                    Some(s) => s,
                    None => return format!("Error: table '{}' not found", table),
                };
                if schema.col_index(column).is_none() {
                    return format!("Error: column '{}' not found in table '{}'", column, table);
                }
                format!("OK (index on {}.{})", table, column)
            }
            Statement::Insert { table, values } => {
                let schema = match self.schemas.get(table) {
                    Some(s) => s.clone(),
                    None => return format!("Error: table '{}' not found", table),
                };
                let _ = &schema;
                let txn_id = self.current_txn.as_ref().unwrap().id;
                let mut results = Vec::new();
                for row_values in values {
                    let mut row_buf = Vec::new();
                    serialize_row(row_values, &mut row_buf);
                    let mut mvcc_buf = Vec::new();
                    mvcc_buf.extend_from_slice(&txn_id.to_le_bytes());
                    mvcc_buf.extend_from_slice(&0u64.to_le_bytes());
                    mvcc_buf.extend_from_slice(&row_buf);
                    let pid = {
                        let df = self.data_files.get_mut(table).unwrap();
                        df.allocate_page()
                    };
                    {
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        let page_copy = page_data.to_vec();
                        self.pool.unpin(pid, true);
                        let mut sp = SlottedPage::new(pid);
                        sp.buffer.copy_from_slice(&page_copy);
                        let _slot = sp.insert(&mvcc_buf).unwrap();
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        page_data.copy_from_slice(&sp.buffer);
                        self.pool.unpin(pid, true);
                    }
                    let lsm = self.lsm_trees.get_mut(table).unwrap();
                    let lsm_key = txn_id.to_le_bytes().to_vec();
                    lsm.put(lsm_key, mvcc_buf.clone());
                    self.wal.append(txn_id, LogType::Insert, pid, &mvcc_buf);
                    results.push(format!("{} row(s) inserted", 1));
                }
                results.join(", ")
            }
            Statement::Select { columns, table, where_clause, table_alias: _ } => {
                let schema = match self.schemas.get(table) {
                    Some(s) => s.clone(),
                    None => return format!("Error: table '{}' not found", table),
                };
                let snapshot_ts = self.current_txn.as_ref().unwrap().snapshot_ts;
                let txn_id = self.current_txn.as_ref().unwrap().id;
                let mut output_rows = Vec::new();
                let count = {
                    let df = self.data_files.get_mut(table).unwrap();
                    df.page_count()
                };
                for pid in 0..count as u32 {
                    let buf_copy = {
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        let buf = page_data.to_vec();
                        self.pool.unpin(pid, false);
                        buf
                    };
                    let mut sp = SlottedPage::new(pid);
                    sp.buffer.copy_from_slice(&buf_copy);
                    for slot in 0..sp.slot_count() {
                        if !sp.has_slot(slot) { continue; }
                        if let Some(data) = sp.get(slot) {
                            if data.len() < 16 { continue; }
                            let begin_ts = get_u64(data, 0);
                            let end_ts = get_u64(data, 8);
                            if !self.txn_mgr.is_visible(begin_ts, end_ts, snapshot_ts, txn_id) {
                                continue;
                            }
                            let row_data = &data[16..];
                            let types = schema.data_types();
                            let values = deserialize_row(row_data, &types);
                            if let Some(ref where_expr) = where_clause {
                                if !Self::eval_expr(where_expr, &values, &schema) {
                                    continue;
                                }
                            }
                            let display: Vec<String> = if columns.len() == 1 && columns[0] == "*" {
                                values.iter().map(|v| v.to_string()).collect()
                            } else {
                                let mut sel = Vec::new();
                                for col_name in columns {
                                    if col_name == "*" {
                                        sel.extend(values.iter().map(|v| v.to_string()));
                                    } else if let Some(idx) = schema.col_index(col_name) {
                                        sel.push(values[idx].to_string());
                                    }
                                }
                                sel
                            };
                            output_rows.push(display.join(" | "));
                        }
                    }
                }
                if output_rows.is_empty() {
                    "(no rows)".to_string()
                } else {
                    output_rows.join("\n")
                }
            }
             Statement::Update { table, assignments, where_clause } => {
                let schema = match self.schemas.get(table) {
                    Some(s) => s.clone(),
                    None => return format!("Error: table '{}' not found", table),
                };
                let txn_id = self.current_txn.as_ref().unwrap().id;
                let snapshot_ts = self.current_txn.as_ref().unwrap().snapshot_ts;
                let mut updated = 0;
                let count = {
                    let df = self.data_files.get_mut(table).unwrap();
                    df.page_count()
                };
                for pid in 0..count as u32 {
                    let (slot_updates, old_buf) = {
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        let buf_copy = page_data.to_vec();
                        self.pool.unpin(pid, false);
                        let mut sp = SlottedPage::new(pid);
                        sp.buffer.copy_from_slice(&buf_copy);
                        let mut updates = Vec::new();
                        for slot in 0..sp.slot_count() {
                            if !sp.has_slot(slot) { continue; }
                            if let Some(data) = sp.get(slot) {
                                if data.len() < 16 { continue; }
                                let begin_ts = get_u64(data, 0);
                                let end_ts = get_u64(data, 8);
                                if !self.txn_mgr.is_visible(begin_ts, end_ts, snapshot_ts, txn_id) {
                                    continue;
                                }
                                let row_data = &data[16..];
                                let types = schema.data_types();
                                let values = deserialize_row(row_data, &types);
                                let matches = match where_clause {
                                    Some(ref expr) => Self::eval_expr(expr, &values, &schema),
                                    None => true,
                                };
                                if !matches { continue; }
                                let mut new_values = values.clone();
                                for (col_name, val) in assignments {
                                    if let Some(idx) = schema.col_index(col_name) {
                                        if idx < new_values.len() {
                                            new_values[idx] = val.clone();
                                        }
                                    }
                                }
                                updates.push((slot, new_values));
                            }
                        }
                        (updates, buf_copy)
                    };
                    let mut modified = Vec::new();
                    for (slot, new_values) in slot_updates.iter() {
                        let mut new_row_buf = Vec::new();
                        serialize_row(new_values, &mut new_row_buf);
                        let mut mvcc_buf = Vec::new();
                        mvcc_buf.extend_from_slice(&txn_id.to_le_bytes());
                        mvcc_buf.extend_from_slice(&0u64.to_le_bytes());
                        mvcc_buf.extend_from_slice(&new_row_buf);
                        let (new_pid, new_slot) = {
                            let np = {
                                let df = self.data_files.get_mut(table).unwrap();
                                df.allocate_page()
                            };
                            let ns = {
                                let df = self.data_files.get_mut(table).unwrap();
                                let new_page = self.pool.pin(np, df);
                                let page_copy = new_page.to_vec();
                                self.pool.unpin(np, true);
                                let mut new_sp = SlottedPage::new(np);
                                new_sp.buffer.copy_from_slice(&page_copy);
                                let s = new_sp.insert(&mvcc_buf).unwrap();
                                let df = self.data_files.get_mut(table).unwrap();
                                let new_page = self.pool.pin(np, df);
                                new_page.copy_from_slice(&new_sp.buffer);
                                self.pool.unpin(np, true);
                                s
                            };
                            (np, ns)
                        };
                        let pk_key = match &new_values[schema.pk_idx] {
                            Value::Int(n) => n.to_le_bytes().to_vec(),
                            Value::Text(s) => s.as_bytes().to_vec(),
                            Value::Null => vec![0],
                        };
                        let btree = self.btrees.get_mut(table).unwrap();
                        btree.mark_deleted(&pk_key, txn_id);
                        btree.insert(pk_key, new_pid, new_slot, txn_id);
                        self.wal.append(txn_id, LogType::UpdateDelete, pid, &txn_id.to_le_bytes());
                        updated += 1;
                        modified.push(*slot);
                    }
                    if !modified.is_empty() {
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        let mut new_buf = old_buf.clone();
                        for slot in &modified {
                            let pos = HEADER_SIZE + *slot as usize * SLOT_ENTRY_SIZE;
                            put_u16(&mut new_buf, pos, 0);
                            put_u16(&mut new_buf, pos + 2, 0);
                        }
                        page_data.copy_from_slice(&new_buf);
                        self.pool.unpin(pid, true);
                    }
                }
                format!("{} row(s) updated", updated)
            }
            Statement::Delete { table, where_clause } => {
                let schema = match self.schemas.get(table) {
                    Some(s) => s.clone(),
                    None => return format!("Error: table '{}' not found", table),
                };
                let txn_id = self.current_txn.as_ref().unwrap().id;
                let snapshot_ts = self.current_txn.as_ref().unwrap().snapshot_ts;
                let mut deleted = 0;
                let count = {
                    let df = self.data_files.get_mut(table).unwrap();
                    df.page_count()
                };
                for pid in 0..count as u32 {
                    let (slot_deletes, old_buf) = {
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        let buf_copy = page_data.to_vec();
                        self.pool.unpin(pid, false);
                        let mut sp = SlottedPage::new(pid);
                        sp.buffer.copy_from_slice(&buf_copy);
                        let mut deletes = Vec::new();
                        for slot in 0..sp.slot_count() {
                            if !sp.has_slot(slot) { continue; }
                            if let Some(data) = sp.get(slot) {
                                if data.len() < 16 { continue; }
                                let begin_ts = get_u64(data, 0);
                                let end_ts = get_u64(data, 8);
                                if !self.txn_mgr.is_visible(begin_ts, end_ts, snapshot_ts, txn_id) {
                                    continue;
                                }
                                let row_data = &data[16..];
                                let types = schema.data_types();
                                let values = deserialize_row(row_data, &types);
                                let matches = match where_clause {
                                    Some(ref expr) => Self::eval_expr(expr, &values, &schema),
                                    None => true,
                                };
                                if !matches { continue; }
                                let pk_key = match &values[schema.pk_idx] {
                                    Value::Int(n) => n.to_le_bytes().to_vec(),
                                    Value::Text(s) => s.as_bytes().to_vec(),
                                    Value::Null => vec![0],
                                };
                                deletes.push((slot, pk_key));
                            }
                        }
                        (deletes, buf_copy)
                    };
                    if !slot_deletes.is_empty() {
                        let mut new_buf = old_buf.clone();
                        for &(slot, ref pk_key) in &slot_deletes {
                            let pos = HEADER_SIZE + slot as usize * SLOT_ENTRY_SIZE;
                            put_u16(&mut new_buf, pos, 0);
                            put_u16(&mut new_buf, pos + 2, 0);
                            let btree = self.btrees.get_mut(table).unwrap();
                            btree.mark_deleted(pk_key, txn_id);
                            self.wal.append(txn_id, LogType::UpdateDelete, pid, &txn_id.to_le_bytes());
                            deleted += 1;
                        }
                        let df = self.data_files.get_mut(table).unwrap();
                        let page_data = self.pool.pin(pid, df);
                        page_data.copy_from_slice(&new_buf);
                        self.pool.unpin(pid, true);
                    }
                }
                format!("{} row(s) deleted", deleted)
            }
            Statement::Begin => {
                if self.current_txn.is_some() {
                    return "Warning: already in a transaction".to_string();
                }
                let txn = self.txn_mgr.begin();
                self.wal.append(txn.id, LogType::Begin, 0, &[]);
                self.current_txn = Some(txn);
                "OK".to_string()
            }
            Statement::Commit => {
                let mut txn = match self.current_txn.take() {
                    Some(t) => t,
                    None => return "Error: no active transaction".to_string(),
                };
                if self.txn_mgr.commit(&mut txn) {
                    self.wal.append(txn.id, LogType::Commit, 0, &[]);
                    if let Some(df) = self.data_files.values_mut().next() {
                        self.pool.flush_all(df);
                    }
                    "OK".to_string()
                } else {
                    "Error: commit failed".to_string()
                }
            }
            Statement::Rollback => {
                let mut txn = match self.current_txn.take() {
                    Some(t) => t,
                    None => return "Error: no active transaction".to_string(),
                };
                self.txn_mgr.rollback(&mut txn);
                self.wal.append(txn.id, LogType::Abort, 0, &[]);
                "OK".to_string()
            }
            Statement::Exit => "Goodbye!".to_string(),
        };

        if auto_start && is_dml {
            if let Some(mut txn) = self.current_txn.take() {
                if self.txn_mgr.commit(&mut txn) {
                    self.wal.append(txn.id, LogType::Commit, 0, &[]);
                }
            }
        }

        result
    }

    fn eval_expr(expr: &Expr, values: &[Value], schema: &Schema) -> bool {
        match expr {
            Expr::BinOp(left, op, right) => {
                let lv = Self::resolve_expr(left, values, schema);
                let rv = Self::resolve_expr(right, values, schema);
                match (lv, rv) {
                    (Value::Int(a), Value::Int(b)) => match op {
                        Op::Eq => a == b, Op::Ne => a != b, Op::Lt => a < b,
                        Op::Gt => a > b, Op::Le => a <= b, Op::Ge => a >= b,
                    },
                    (Value::Text(a), Value::Text(b)) => match op {
                        Op::Eq => a == b, Op::Ne => a != b, Op::Lt => a < b,
                        Op::Gt => a > b, Op::Le => a <= b, Op::Ge => a >= b,
                    },
                    _ => false,
                }
            }
            Expr::Column(_) | Expr::Value(_) => true,
        }
    }

    fn resolve_expr(expr: &Expr, values: &[Value], schema: &Schema) -> Value {
        match expr {
            Expr::Column(name) => {
                if let Some(idx) = schema.col_index(name) {
                    values.get(idx).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            Expr::Value(v) => v.clone(),
            Expr::BinOp(_, _, _) => Value::Null,
        }
    }
}

// ============================================================================
// CLI REPL
// ============================================================================

fn main() {
    let data_dir = std::env::var("DB_DIR").unwrap_or_else(|_| "mydb".to_string());
    let mut db = Database::new(&data_dir);
    if Path::new(&format!("{}/wal.log", data_dir)).exists() {
        println!("Recovering from WAL...");
        let mut hf = HeapFile::open(&format!("{}/recovery.tmp", data_dir));
        db.wal.recover(&mut db.pool, &mut hf);
        println!("Recovery complete.");
    }

    let stdin = std::io::stdin();
    println!("MVCC-SQL v0.1 — Phase 10 Capstone");
    println!("Enter SQL statements (EXIT to quit):");
    loop {
        print!("db> ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let tokens = tokenize(trimmed);
        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(Statement::Exit) => {
                println!("Goodbye!");
                db.close();
                break;
            }
            Ok(stmt) => {
                let result = db.execute(&stmt);
                println!("{}", result);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    db.close();
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- SlottedPage Tests ---

    #[test]
    fn test_slotted_page_insert_and_get() {
        let mut sp = SlottedPage::new(0);
        let data = b"hello, world";
        let slot = sp.insert(data).expect("insert");
        let retrieved = sp.get(slot).expect("get");
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_slotted_page_multiple_inserts() {
        let mut sp = SlottedPage::new(0);
        let s1 = sp.insert(b"aaa").expect("insert a");
        let s2 = sp.insert(b"bbb").expect("insert b");
        let s3 = sp.insert(b"ccc").expect("insert c");
        assert_eq!(sp.get(s1), Some(&b"aaa"[..]));
        assert_eq!(sp.get(s2), Some(&b"bbb"[..]));
        assert_eq!(sp.get(s3), Some(&b"ccc"[..]));
    }

    #[test]
    fn test_slotted_page_delete() {
        let mut sp = SlottedPage::new(0);
        let slot = sp.insert(b"delete me").expect("insert");
        assert!(sp.has_slot(slot));
        sp.delete(slot);
        assert!(!sp.has_slot(slot));
    }

    #[test]
    fn test_slotted_page_defrag() {
        let mut sp = SlottedPage::new(0);
        let s1 = sp.insert(b"aaaa").expect("insert a");
        let s2 = sp.insert(b"bbbb").expect("insert b");
        sp.delete(s1);
        let used_before = sp.free_space();
        sp.defragment();
        assert!(sp.free_space() >= used_before);
        assert_eq!(sp.get(s2), Some(&b"bbbb"[..]));
    }

    #[test]
    fn test_slotted_page_update_grow() {
        let mut sp = SlottedPage::new(0);
        let slot = sp.insert(b"short").expect("insert");
        assert_eq!(sp.get(slot), Some(&b"short"[..]));
        let success = sp.update(slot, b"much longer data");
        assert!(success);
        assert!(!sp.has_slot(slot));
        assert_eq!(sp.slot_count(), 2);
        let new_slot = if sp.has_slot(1) { 1 } else { 0 };
        assert_eq!(sp.get(new_slot), Some(&b"much longer data"[..]));
    }

    // --- BufferPool Tests ---

    #[test]
    fn test_buffer_pool_pin_unpin() {
        let mut hf = HeapFile::open("/tmp/test_bp_pin.data");
        let mut bp = BufferPool::new();
        let pid = hf.allocate_page();
        let _data = bp.pin(pid, &mut hf);
        bp.unpin(pid, false);
        let _data2 = bp.pin(pid, &mut hf);
        bp.unpin(pid, false);
        hf.close();
        fs::remove_file("/tmp/test_bp_pin.data").ok();
    }

    #[test]
    fn test_buffer_pool_multiple_pages() {
        let mut hf = HeapFile::open("/tmp/test_bp_multi.data");
        let mut bp = BufferPool::new();
        let pages: Vec<u32> = (0..5).map(|_| hf.allocate_page()).collect();
        for &pid in &pages {
            let data = bp.pin(pid, &mut hf);
            let id = get_u32(data, 0);
            assert_eq!(id, pid);
            bp.unpin(pid, false);
        }
        hf.close();
        fs::remove_file("/tmp/test_bp_multi.data").ok();
    }

    // --- BTree Tests ---

    #[test]
    fn test_btree_insert_and_search() {
        let mut bt = BTree::new();
        bt.insert(b"alice".to_vec(), 1, 0, 1);
        bt.insert(b"bob".to_vec(), 2, 1, 1);
        bt.insert(b"charlie".to_vec(), 3, 2, 1);
        let e = bt.search(b"bob").expect("find bob");
        assert_eq!(e.page_id, 2);
        assert_eq!(e.slot, 1);
        assert!(bt.search(b"dave").is_none());
    }

    #[test]
    fn test_btree_mark_deleted() {
        let mut bt = BTree::new();
        bt.insert(b"key1".to_vec(), 1, 0, 1);
        bt.mark_deleted(b"key1", 2);
        let e = bt.search(b"key1").unwrap();
        assert_eq!(e.end_ts, 2);
    }

    // --- MVCC Tests ---

    #[test]
    fn test_mvcc_begin_and_commit() {
        let mut mgr = TransactionManager::new();
        let mut txn = mgr.begin();
        assert_eq!(txn.status, TxnStatus::Active);
        assert!(mgr.commit(&mut txn));
        assert_eq!(txn.status, TxnStatus::Committed);
    }

    #[test]
    fn test_mvcc_visibility_own_changes() {
        let mut mgr = TransactionManager::new();
        let mut txn = mgr.begin();
        let tid = txn.id;
        mgr.commit(&mut txn);
        assert!(mgr.is_visible(tid, 0, txn.snapshot_ts, tid));
    }

    #[test]
    fn test_mvcc_visibility_other_txn_uncommitted() {
        let mut mgr = TransactionManager::new();
        let txn = mgr.begin();
        let other_id = 99;
        assert!(!mgr.is_visible(other_id, 0, txn.snapshot_ts, txn.id));
    }

    #[test]
    fn test_mvcc_visibility_deleted_row() {
        let mut mgr = TransactionManager::new();
        let mut txn1 = mgr.begin();
        let id1 = txn1.id;
        mgr.commit(&mut txn1);
        let txn2 = mgr.begin();
        assert!(!mgr.is_visible(id1, id1, txn2.snapshot_ts, txn2.id));
    }

    // --- WAL Tests ---

    #[test]
    fn test_wal_append_and_recover() {
        let wal_path = "/tmp/test_wal.log";
        fs::remove_file(wal_path).ok();
        let mut wal = WalLog::open(wal_path);
        wal.append(1, LogType::Begin, 0, &[]);
        wal.append(1, LogType::Insert, 5, b"test payload");
        wal.append(1, LogType::Commit, 0, &[]);
        assert!(wal.records.len() >= 3);
        let (txn_table, _, _) = wal.analyze();
        let entry = txn_table.get(&1);
        assert!(entry.is_some());
        assert!(!entry.unwrap().1);
        fs::remove_file(wal_path).ok();
    }

    #[test]
    fn test_wal_analyze_active_txn() {
        let wal_path = "/tmp/test_wal_active.log";
        let _ = fs::remove_file(wal_path);
        let mut wal = WalLog::open(wal_path);
        wal.append(10, LogType::Begin, 0, &[]);
        wal.append(20, LogType::Begin, 0, &[]);
        wal.append(10, LogType::Insert, 3, b"data");
        wal.append(10, LogType::Commit, 0, &[]);
        let (txn_table, _, _) = wal.analyze();
        let e1 = txn_table.get(&10);
        let e2 = txn_table.get(&20);
        assert!(e1.is_some(), "txn 10 should be in table");
        assert!(!e1.unwrap().1, "txn 10 should be committed");
        assert!(e2.is_some(), "txn 20 should be in table");
        assert!(e2.unwrap().1, "txn 20 should still be active");
        let _ = fs::remove_file(wal_path);
    }

    // --- SQL Parser Tests ---

    #[test]
    fn test_parse_create_table() {
        let tokens = tokenize("CREATE TABLE users (id INT, name TEXT)");
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().expect("parse");
        match stmt {
            Statement::CreateTable { name, columns, .. } => {
                assert_eq!(name, "users");
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0].name, "id");
                assert_eq!(columns[0].dtype, DataType::Int);
                assert_eq!(columns[1].name, "name");
                assert_eq!(columns[1].dtype, DataType::Text);
            }
            _ => panic!("expected CreateTable"),
        }
    }

    #[test]
    fn test_parse_insert() {
        let tokens = tokenize("INSERT INTO users VALUES (1, 'Alice')");
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().expect("parse");
        match stmt {
            Statement::Insert { table, values } => {
                assert_eq!(table, "users");
                assert_eq!(values.len(), 1);
                assert_eq!(values[0][0], Value::Int(1));
                assert_eq!(values[0][1], Value::Text("Alice".to_string()));
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn test_parse_select() {
        let tokens = tokenize("SELECT id, name FROM users WHERE id = 1");
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().expect("parse");
        match stmt {
            Statement::Select { columns, table, where_clause, .. } => {
                assert_eq!(columns, vec!["id", "name"]);
                assert_eq!(table, "users");
                assert!(where_clause.is_some());
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn test_parse_select_star() {
        let tokens = tokenize("SELECT * FROM orders");
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().expect("parse");
        match stmt {
            Statement::Select { columns, table, .. } => {
                assert_eq!(columns, vec!["*"]);
                assert_eq!(table, "orders");
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn test_parse_begin_commit() {
        let tokens = tokenize("BEGIN");
        let mut parser = Parser::new(tokens);
        assert_eq!(parser.parse().ok(), Some(Statement::Begin));

        let tokens = tokenize("COMMIT");
        let mut parser = Parser::new(tokens);
        assert_eq!(parser.parse().ok(), Some(Statement::Commit));

        let tokens = tokenize("ROLLBACK");
        let mut parser = Parser::new(tokens);
        assert_eq!(parser.parse().ok(), Some(Statement::Rollback));
    }

    #[test]
    fn test_parse_update() {
        let tokens = tokenize("UPDATE users SET name = 'Bob' WHERE id = 1");
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().expect("parse");
        match stmt {
            Statement::Update { table, assignments, where_clause } => {
                assert_eq!(table, "users");
                assert_eq!(assignments.len(), 1);
                assert_eq!(assignments[0].0, "name");
                assert_eq!(assignments[0].1, Value::Text("Bob".to_string()));
                assert!(where_clause.is_some());
            }
            _ => panic!("expected Update"),
        }
    }

    #[test]
    fn test_parse_delete() {
        let tokens = tokenize("DELETE FROM users WHERE id = 5");
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse().expect("parse");
        match stmt {
            Statement::Delete { table, where_clause } => {
                assert_eq!(table, "users");
                assert!(where_clause.is_some());
            }
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn test_tokenize_complex() {
        let tokens = tokenize("SELECT * FROM t WHERE x >= 10 AND y <> 'foo'");
        assert!(tokens.contains(&Token::Star));
        assert!(tokens.contains(&Token::Ge));
        assert!(tokens.contains(&Token::Ne));
        assert!(tokens.contains(&Token::And));
    }

    #[test]
    fn test_tokenize_comments() {
        let tokens = tokenize("SELECT 1; -- this is a comment\nSELECT 2");
        assert_eq!(tokens.len(), 5);
    }

    // --- Serialization Tests ---

    #[test]
    fn test_serialize_deserialize_row() {
        let values = vec![Value::Int(42), Value::Text("hello".to_string()), Value::Null];
        let types = vec![DataType::Int, DataType::Text, DataType::Int];
        let mut buf = Vec::new();
        serialize_row(&values, &mut buf);
        let decoded = deserialize_row(&buf, &types);
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], Value::Int(42));
        assert_eq!(decoded[1], Value::Text("hello".to_string()));
        assert_eq!(decoded[2], Value::Null);
    }

    // --- Integration Test ---

    #[test]
    fn test_database_crud() {
        let dir = "/tmp/test_db_crud";
        fs::remove_dir_all(dir).ok();
        let mut db = Database::new(dir);

        let r1 = db.execute(&Statement::CreateTable {
            name: "test".to_string(),
            columns: vec![
                ColumnDef { name: "id".to_string(), dtype: DataType::Int },
                ColumnDef { name: "val".to_string(), dtype: DataType::Text },
            ],
            pk_col: Some(0),
        });
        assert_eq!(r1, "OK");

        let r2 = db.execute(&Statement::Insert {
            table: "test".to_string(),
            values: vec![vec![Value::Int(1), Value::Text("one".to_string())]],
        });
        assert!(r2.contains("inserted"));

        let r3 = db.execute(&Statement::Select {
            columns: vec!["*".to_string()],
            table: "test".to_string(),
            where_clause: None,
            table_alias: None,
        });
        assert!(r3.contains("1"));
        assert!(r3.contains("one"));

        let r4 = db.execute(&Statement::Begin);
        assert_eq!(r4, "OK");

        let r5 = db.execute(&Statement::Insert {
            table: "test".to_string(),
            values: vec![vec![Value::Int(2), Value::Text("two".to_string())]],
        });
        assert!(r5.contains("inserted"));

        let r6 = db.execute(&Statement::Commit);
        assert_eq!(r6, "OK");

        let r7 = db.execute(&Statement::Select {
            columns: vec!["*".to_string()],
            table: "test".to_string(),
            where_clause: None,
            table_alias: None,
        });
        assert!(r7.contains("2"));

        let r8 = db.execute(&Statement::Rollback);
        assert_eq!(r8, "Error: no active transaction");

        db.close();
        let mut db2 = Database::new(dir);
        let r9 = db2.execute(&Statement::Select {
            columns: vec!["*".to_string()],
            table: "test".to_string(),
            where_clause: None,
            table_alias: None,
        });
        assert!(r9.contains("1"), "persisted data: {:?}", r9);
        db2.close();
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_select_with_where() {
        let dir = "/tmp/test_db_where";
        fs::remove_dir_all(dir).ok();
        let mut db = Database::new(dir);

        db.execute(&Statement::CreateTable {
            name: "items".to_string(),
            columns: vec![
                ColumnDef { name: "id".to_string(), dtype: DataType::Int },
                ColumnDef { name: "price".to_string(), dtype: DataType::Int },
            ],
            pk_col: Some(0),
        });

        db.execute(&Statement::Insert {
            table: "items".to_string(),
            values: vec![vec![Value::Int(1), Value::Int(100)]],
        });
        db.execute(&Statement::Insert {
            table: "items".to_string(),
            values: vec![vec![Value::Int(2), Value::Int(200)]],
        });
        db.execute(&Statement::Insert {
            table: "items".to_string(),
            values: vec![vec![Value::Int(3), Value::Int(50)]],
        });

        let and_tokens = tokenize("SELECT * FROM items WHERE price > 100");
        let mut parser = Parser::new(and_tokens);
        let stmt = parser.parse().unwrap();
        let result = db.execute(&stmt);
        assert!(result.contains("200"));

        db.close();
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_mvcc_transaction_isolation() {
        let dir = "/tmp/test_db_mvcc";
        fs::remove_dir_all(dir).ok();
        let mut db = Database::new(dir);

        db.execute(&Statement::Begin);
        db.execute(&Statement::CreateTable {
            name: "accounts".to_string(),
            columns: vec![
                ColumnDef { name: "id".to_string(), dtype: DataType::Int },
                ColumnDef { name: "balance".to_string(), dtype: DataType::Int },
            ],
            pk_col: Some(0),
        });
        db.execute(&Statement::Insert {
            table: "accounts".to_string(),
            values: vec![vec![Value::Int(1), Value::Int(1000)]],
        });
        db.execute(&Statement::Commit);

        db.execute(&Statement::Begin);
        db.execute(&Statement::Insert {
            table: "accounts".to_string(),
            values: vec![vec![Value::Int(2), Value::Int(2000)]],
        });
        let r1 = db.execute(&Statement::Select {
            columns: vec!["*".to_string()],
            table: "accounts".to_string(),
            where_clause: None,
            table_alias: None,
        });
        assert!(r1.contains("2000"), "sees own insert: {:?}", r1);
        db.execute(&Statement::Commit);

        db.close();
        fs::remove_dir_all(dir).ok();
    }
}

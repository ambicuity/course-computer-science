use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MEMTABLE_SIZE: usize = 4 * 1024 * 1024; // 4 MB — flush threshold
const BLOCK_SIZE: usize = 4096;               // 4 KB — SSTable data block
const SSTABLE_MAGIC: u32 = 0xDEAD_BEEF;
const COMPACTION_THRESHOLD_BASE: usize = 4;   // base for per-level thresholds

// ---------------------------------------------------------------------------
// ValueEntry — stored in memtable and during compaction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum ValueEntry {
    Live(Vec<u8>),
    Tombstone,
}

// ---------------------------------------------------------------------------
// MemTable (C0) — in-memory sorted write buffer
// ---------------------------------------------------------------------------

struct MemTable {
    map: BTreeMap<Vec<u8>, ValueEntry>,
    approx_size: usize,
}

impl MemTable {
    fn new() -> Self {
        MemTable { map: BTreeMap::new(), approx_size: 0 }
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let cost = key.len() + value.len();
        if let Some(old) = self.map.insert(key, ValueEntry::Live(value)) {
            if let ValueEntry::Live(ref v) = old {
                self.approx_size = self.approx_size.saturating_sub(v.len());
            }
        }
        self.approx_size += cost;
    }

    fn delete(&mut self, key: Vec<u8>) {
        let cost = key.len();
        if let Some(old) = self.map.insert(key, ValueEntry::Tombstone) {
            if let ValueEntry::Live(ref v) = old {
                self.approx_size = self.approx_size.saturating_sub(v.len());
            }
        }
        self.approx_size += cost;
    }

    fn get(&self, key: &[u8]) -> Option<&ValueEntry> {
        self.map.get(key)
    }

    fn is_full(&self) -> bool {
        self.approx_size >= MEMTABLE_SIZE
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &ValueEntry)> {
        self.map.iter()
    }
}

// ---------------------------------------------------------------------------
// BloomFilter — probabilistic membership test
// ---------------------------------------------------------------------------

struct BloomFilter {
    bits: Vec<u64>,
    num_hashes: usize,
}

impl BloomFilter {
    fn new(num_entries: usize, fp_rate: f64) -> Self {
        let n = num_entries.max(1);
        let ln2 = std::f64::consts::LN_2;
        let num_bits = (-(n as f64) * fp_rate.ln() / (ln2 * ln2)).ceil() as usize;
        let num_words = (num_bits + 63) / 64;
        let num_hashes = ((num_bits as f64 / n as f64) * ln2).round() as usize;
        BloomFilter {
            bits: vec![0; num_words],
            num_hashes: num_hashes.max(1),
        }
    }

    fn hash_key(key: &[u8], seed: u64) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        h.write_u64(seed);
        h.write(key);
        h.finish()
    }

    fn insert(&mut self, key: &[u8]) {
        for i in 0..self.num_hashes {
            let h = Self::hash_key(key, i as u64);
            let idx = (h as usize) % (self.bits.len() * 64);
            self.bits[idx / 64] |= 1 << (idx % 64);
        }
    }

    fn might_contain(&self, key: &[u8]) -> bool {
        for i in 0..self.num_hashes {
            let h = Self::hash_key(key, i as u64);
            let idx = (h as usize) % (self.bits.len() * 64);
            if self.bits[idx / 64] & (1 << (idx % 64)) == 0 {
                return false;
            }
        }
        true
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 8 + self.bits.len() * 8];
        buf[0..4].copy_from_slice(&(self.num_hashes as u32).to_le_bytes());
        buf[4..8].copy_from_slice(&(self.bits.len() as u32).to_le_bytes());
        for (i, w) in self.bits.iter().enumerate() {
            buf[8 + i * 8..16 + i * 8].copy_from_slice(&w.to_le_bytes());
        }
        buf
    }

    fn deserialize(buf: &[u8]) -> Self {
        let nh = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        let bl = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
        let mut bits = vec![0u64; bl];
        for (i, w) in bits.iter_mut().enumerate() {
            *w = u64::from_le_bytes(buf[8 + i * 8..16 + i * 8].try_into().unwrap());
        }
        BloomFilter { bits, num_hashes: nh }
    }
}

// ---------------------------------------------------------------------------
// SSTable Builder
// ---------------------------------------------------------------------------

struct SSTableBuilder {
    block: Vec<(Vec<u8>, ValueEntry)>,
    block_bytes: usize,
    blocks: Vec<Vec<u8>>,
    index: Vec<(Vec<u8>, u64)>,
    bloom: BloomFilter,
}

impl SSTableBuilder {
    fn new(num_entries: usize) -> Self {
        SSTableBuilder {
            block: Vec::new(),
            block_bytes: 0,
            blocks: Vec::new(),
            index: Vec::new(),
            bloom: BloomFilter::new(num_entries, 0.01),
        }
    }

    fn add(&mut self, key: &[u8], entry: &ValueEntry) {
        let sz = key.len()
            + 8
            + match entry {
                ValueEntry::Live(v) => v.len(),
                ValueEntry::Tombstone => 0,
            };
        self.block.push((key.to_vec(), entry.clone()));
        self.block_bytes += sz;
        self.bloom.insert(key);
        if self.block_bytes >= BLOCK_SIZE {
            self.flush_block();
        }
    }

    fn flush_block(&mut self) {
        if self.block.is_empty() {
            return;
        }
        let mut raw = Vec::new();
        raw.extend_from_slice(&(self.block.len() as u32).to_le_bytes());
        for (k, v) in &self.block {
            raw.extend_from_slice(&(k.len() as u32).to_le_bytes());
            raw.extend_from_slice(k);
            match v {
                ValueEntry::Live(val) => {
                    raw.extend_from_slice(&(val.len() as u32).to_le_bytes());
                    raw.extend_from_slice(val);
                }
                ValueEntry::Tombstone => {
                    raw.extend_from_slice(&u32::MAX.to_le_bytes());
                }
            }
        }
        let off: u64 = self.blocks.iter().map(|b| b.len() as u64).sum();
        let last = self.block.last().unwrap().0.clone();
        self.index.push((last, off));
        self.blocks.push(raw);
        self.block.clear();
        self.block_bytes = 0;
    }

    fn build(mut self, path: &Path) -> std::io::Result<()> {
        self.flush_block();
        let mut f = fs::File::create(path)?;

        for b in &self.blocks {
            f.write_all(b)?;
        }
        let data_end = f.stream_position()?;

        let index_off = data_end;
        f.write_all(&(self.index.len() as u32).to_le_bytes())?;
        for (lk, off) in &self.index {
            f.write_all(&(lk.len() as u32).to_le_bytes())?;
            f.write_all(lk)?;
            f.write_all(&off.to_le_bytes())?;
        }
        let bloom_off = f.stream_position()?;

        let bloom_bytes = self.bloom.serialize();
        f.write_all(&bloom_bytes)?;

        f.write_all(&bloom_off.to_le_bytes())?;
        f.write_all(&index_off.to_le_bytes())?;
        f.write_all(&SSTABLE_MAGIC.to_le_bytes())?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SSTable Reader
// ---------------------------------------------------------------------------

struct SSTableReader {
    data: Vec<u8>,
    data_end: usize,
    bloom: BloomFilter,
    index: Vec<(Vec<u8>, u64)>,
}

impl SSTableReader {
    fn open(path: &Path) -> std::io::Result<Self> {
        let data = fs::read(path)?;
        if data.len() < 24 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file too small",
            ));
        }
        let fs_ = data.len() - 24;
        let bloom_off =
            u64::from_le_bytes(data[fs_..fs_ + 8].try_into().unwrap()) as usize;
        let index_off =
            u64::from_le_bytes(data[fs_ + 8..fs_ + 16].try_into().unwrap()) as usize;
        let magic =
            u32::from_le_bytes(data[fs_ + 16..fs_ + 20].try_into().unwrap());
        if magic != SSTABLE_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad magic",
            ));
        }

        let bloom = BloomFilter::deserialize(&data[bloom_off..]);

        let mut c = index_off;
        let nidx = u32::from_le_bytes(data[c..c + 4].try_into().unwrap()) as usize;
        c += 4;
        let mut index = Vec::with_capacity(nidx);
        for _ in 0..nidx {
            let kl = u32::from_le_bytes(data[c..c + 4].try_into().unwrap()) as usize;
            c += 4;
            let k = data[c..c + kl].to_vec();
            c += kl;
            let off = u64::from_le_bytes(data[c..c + 8].try_into().unwrap());
            c += 8;
            index.push((k, off));
        }

        Ok(SSTableReader {
            data,
            data_end: index_off,
            bloom,
            index,
        })
    }

    fn might_contain(&self, key: &[u8]) -> bool {
        self.bloom.might_contain(key)
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if self.index.is_empty() {
            return None;
        }

        let mut lo = 0usize;
        let mut hi = self.index.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if key <= self.index[mid].0.as_slice() {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        if lo >= self.index.len() {
            lo = self.index.len() - 1;
        }

        let start = self.index[lo].1 as usize;
        let end = if lo + 1 < self.index.len() {
            self.index[lo + 1].1 as usize
        } else {
            self.data_end
        };

        self.scan_block(key, start, end)
    }

    fn scan_block(&self, key: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
        if start + 4 > end {
            return None;
        }
        let n = u32::from_le_bytes(self.data[start..start + 4].try_into().unwrap()) as usize;
        let mut c = start + 4;
        for _ in 0..n {
            if c + 4 > end {
                return None;
            }
            let kl = u32::from_le_bytes(self.data[c..c + 4].try_into().unwrap()) as usize;
            c += 4;
            if c + kl > end {
                return None;
            }
            let k = &self.data[c..c + kl];
            c += kl;
            if c + 4 > end {
                return None;
            }
            let vl = u32::from_le_bytes(self.data[c..c + 4].try_into().unwrap());
            c += 4;
            if k == key {
                if vl == u32::MAX {
                    return None;
                }
                if c + vl as usize > end {
                    return None;
                }
                return Some(self.data[c..c + vl as usize].to_vec());
            }
            if vl != u32::MAX {
                c += vl as usize;
            }
        }
        None
    }

    fn scan_range_into(
        &self,
        start: &[u8],
        end: &[u8],
        out: &mut BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    ) {
        for i in 0..self.index.len() {
            if start > self.index[i].0.as_slice() {
                continue;
            }
            let bs = self.index[i].1 as usize;
            let be = if i + 1 < self.index.len() {
                self.index[i + 1].1 as usize
            } else {
                self.data_end
            };
            self.scan_block_into(start, end, bs, be, out);
        }
    }

    fn scan_block_into(
        &self,
        start: &[u8],
        end: &[u8],
        bs: usize,
        be: usize,
        out: &mut BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    ) {
        if bs + 4 > be {
            return;
        }
        let n =
            u32::from_le_bytes(self.data[bs..bs + 4].try_into().unwrap()) as usize;
        let mut c = bs + 4;
        for _ in 0..n {
            if c + 4 > be {
                return;
            }
            let kl =
                u32::from_le_bytes(self.data[c..c + 4].try_into().unwrap()) as usize;
            c += 4;
            if c + kl > be {
                return;
            }
            let k = &self.data[c..c + kl];
            c += kl;
            if c + 4 > be {
                return;
            }
            let vl =
                u32::from_le_bytes(self.data[c..c + 4].try_into().unwrap());
            c += 4;
            if k >= start && k < end {
                let entry = out.entry(k.to_vec());
                if let std::collections::btree_map::Entry::Vacant(e) = entry {
                    if vl == u32::MAX {
                        e.insert(None);
                    } else {
                        if c + vl as usize > be {
                            return;
                        }
                        e.insert(Some(self.data[c..c + vl as usize].to_vec()));
                    }
                }
            }
            if vl != u32::MAX {
                c += vl as usize;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// LSMTree — orchestrates memtable flushes, compactions, reads
// ---------------------------------------------------------------------------

struct LSMTree {
    memtable: MemTable,
    immutable: Option<MemTable>,
    levels: Vec<Vec<PathBuf>>,
    dir: PathBuf,
    seq: u64,
}

impl LSMTree {
    fn new(dir: &Path) -> std::io::Result<Self> {
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        fs::create_dir_all(dir)?;
        Ok(LSMTree {
            memtable: MemTable::new(),
            immutable: None,
            levels: vec![Vec::new()],
            dir: dir.to_path_buf(),
            seq: 0,
        })
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::io::Result<()> {
        self.memtable.put(key, value);
        self.try_freeze()
    }

    fn delete(&mut self, key: Vec<u8>) -> std::io::Result<()> {
        self.memtable.delete(key);
        self.try_freeze()
    }

    fn try_freeze(&mut self) -> std::io::Result<()> {
        if self.memtable.is_full() {
            self.freeze_and_flush()?;
        }
        Ok(())
    }

    fn freeze_and_flush(&mut self) -> std::io::Result<()> {
        if self.memtable.len() == 0 {
            return Ok(());
        }
        let old = std::mem::replace(&mut self.memtable, MemTable::new());
        self.immutable = Some(old);
        self.flush_immutable()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.freeze_and_flush()
    }

    fn flush_immutable(&mut self) -> std::io::Result<()> {
        if let Some(imm) = self.immutable.take() {
            if imm.len() == 0 {
                return Ok(());
            }
            let n = imm.len();
            let path = self.dir.join(format!("L0-{}.sst", self.seq));
            self.seq += 1;
            let mut b = SSTableBuilder::new(n);
            for (k, v) in imm.iter() {
                b.add(k, v);
            }
            b.build(&path)?;
            self.levels[0].push(path);
            if self.levels[0].len() >= COMPACTION_THRESHOLD_BASE {
                self.compact(0)?;
            }
        }
        Ok(())
    }

    fn get(&self, key: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
        if let Some(e) = self.memtable.get(key) {
            return Ok(match e {
                ValueEntry::Live(v) => Some(v.clone()),
                _ => None,
            });
        }
        if let Some(ref imm) = self.immutable {
            if let Some(e) = imm.get(key) {
                return Ok(match e {
                    ValueEntry::Live(v) => Some(v.clone()),
                    _ => None,
                });
            }
        }
        for lvl in &self.levels {
            for p in lvl.iter().rev() {
                let reader = SSTableReader::open(p)?;
                if !reader.might_contain(key) {
                    continue;
                }
                if let Some(v) = reader.get(key) {
                    return Ok(Some(v));
                }
            }
        }
        Ok(None)
    }

    fn compact(&mut self, level: usize) -> std::io::Result<()> {
        while level >= self.levels.len() {
            self.levels.push(Vec::new());
        }
        let ssts = std::mem::take(&mut self.levels[level]);
        if ssts.len() < COMPACTION_THRESHOLD_BASE {
            self.levels[level] = ssts;
            return Ok(());
        }

        let mut all: BTreeMap<Vec<u8>, ValueEntry> = BTreeMap::new();
        for p in &ssts {
            let data = fs::read(p)?;
            let mut c = 0usize;
            let data_end = {
                if data.len() < 24 {
                    continue;
                }
                let fs_ = data.len() - 24;
                u64::from_le_bytes(data[fs_ + 8..fs_ + 16].try_into().unwrap())
                    as usize
            };
            while c < data_end {
                if c + 4 > data_end {
                    break;
                }
                let n =
                    u32::from_le_bytes(data[c..c + 4].try_into().unwrap()) as usize;
                c += 4;
                if n == 0 || n > 1_000_000 {
                    break;
                }
                for _ in 0..n {
                    if c + 4 > data_end {
                        break;
                    }
                    let kl = u32::from_le_bytes(data[c..c + 4].try_into().unwrap())
                        as usize;
                    c += 4;
                    if c + kl > data_end {
                        break;
                    }
                    let k = data[c..c + kl].to_vec();
                    c += kl;
                    if c + 4 > data_end {
                        break;
                    }
                    let vl = u32::from_le_bytes(data[c..c + 4].try_into().unwrap());
                    c += 4;
                    if vl == u32::MAX {
                        all.insert(k, ValueEntry::Tombstone);
                    } else {
                        if c + vl as usize > data_end {
                            break;
                        }
                        let v = data[c..c + vl as usize].to_vec();
                        c += vl as usize;
                        all.insert(k, ValueEntry::Live(v));
                    }
                }
            }
        }

        let out_path = self.dir.join(format!("L{}-c-{}.sst", level + 1, self.seq));
        self.seq += 1;
        let mut b = SSTableBuilder::new(all.len());
        for (k, v) in &all {
            b.add(k, v);
        }
        b.build(&out_path)?;
        self.levels[level + 1].push(out_path);

        for p in &ssts {
            let _ = fs::remove_file(p);
        }

        let threshold = COMPACTION_THRESHOLD_BASE.pow(level as u32 + 2);
        if self.levels[level + 1].len() >= threshold {
            self.compact(level + 1)?;
        }

        Ok(())
    }

    fn scan(
        &self,
        start: &[u8],
        end: &[u8],
    ) -> std::io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut out: BTreeMap<Vec<u8>, Option<Vec<u8>>> = BTreeMap::new();

        for (k, v) in self.memtable.iter() {
            if k.as_slice() >= start && k.as_slice() < end {
                match v {
                    ValueEntry::Live(val) => {
                        out.entry(k.clone()).or_insert(Some(val.clone()));
                    }
                    ValueEntry::Tombstone => {
                        out.entry(k.clone()).or_insert(None);
                    }
                }
            }
        }

        if let Some(ref imm) = self.immutable {
            for (k, v) in imm.iter() {
                if k.as_slice() >= start && k.as_slice() < end {
                    match v {
                        ValueEntry::Live(val) => {
                            out.entry(k.clone()).or_insert(Some(val.clone()));
                        }
                        ValueEntry::Tombstone => {
                            out.entry(k.clone()).or_insert(None);
                        }
                    }
                }
            }
        }

        for lvl in &self.levels {
            for p in lvl.iter().rev() {
                if let Ok(reader) = SSTableReader::open(p) {
                    reader.scan_range_into(start, end, &mut out);
                }
            }
        }

        Ok(out
            .into_iter()
            .filter_map(|(k, v)| v.map(|vv| (k, vv)))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Main — demo, test, and example usage
// ---------------------------------------------------------------------------

fn main() -> std::io::Result<()> {
    let path = Path::new("test_lsm_db");
    let mut db = LSMTree::new(path)?;

    // ── Phase 1: All in-memory (no flush yet) ──

    db.put(b"alpha".to_vec(), b"first".to_vec())?;
    db.put(b"beta".to_vec(), b"second".to_vec())?;
    db.put(b"gamma".to_vec(), b"third".to_vec())?;

    assert_eq!(db.get(b"alpha")?, Some(b"first".to_vec()));
    assert_eq!(db.get(b"delta")?, None);

    db.delete(b"beta".to_vec())?;
    assert_eq!(db.get(b"beta")?, None);
    assert_eq!(db.get(b"gamma")?, Some(b"third".to_vec()));

    db.put(b"alpha".to_vec(), b"updated".to_vec())?;
    assert_eq!(db.get(b"alpha")?, Some(b"updated".to_vec()));

    // ── Phase 2: Flush to SSTable ──

    db.put(b"k-001".to_vec(), b"v-001".to_vec())?;
    db.put(b"k-003".to_vec(), b"v-003".to_vec())?;
    db.put(b"k-005".to_vec(), b"v-005".to_vec())?;

    db.flush()?;
    // Data now in L0-0.sst. Memtable is empty.

    assert_eq!(db.get(b"alpha")?, Some(b"updated".to_vec()));
    assert_eq!(db.get(b"gamma")?, Some(b"third".to_vec()));
    assert_eq!(db.get(b"beta")?, None);
    assert_eq!(db.get(b"k-003")?, Some(b"v-003".to_vec()));

    // Write to the now-empty memtable and flush again
    db.put(b"k-002".to_vec(), b"v-002".to_vec())?;
    db.put(b"k-004".to_vec(), b"v-004".to_vec())?;
    db.flush()?;
    // L0 now has 2 SSTables: [L0-0.sst, L0-1.sst]

    assert_eq!(db.get(b"k-002")?, Some(b"v-002".to_vec()));

    // ── Phase 3: Range scan across memtable + multiple SSTables ──
    // Add results in memtable too (some newer than SSTable data)
    db.put(b"k-001".to_vec(), b"memtable-override".to_vec())?;

    let results = db.scan(b"k-001", b"k-006")?;
    assert_eq!(results.len(), 5);
    // k-001 should be the memtable version (newest)
    assert_eq!(&results[0].1[..], b"memtable-override");
    assert_eq!(&results[1].1[..], b"v-002");
    assert_eq!(&results[2].1[..], b"v-003");
    assert_eq!(&results[3].1[..], b"v-004");
    assert_eq!(&results[4].1[..], b"v-005");

    // ── Phase 4: Tombstones across flush boundaries ──
    db.delete(b"k-003".to_vec())?;
    db.flush()?;
    // L0 now has 3 SSTables

    let results = db.scan(b"k-001", b"k-006")?;
    assert_eq!(results.len(), 4);
    assert_eq!(&results[0].1[..], b"memtable-override");
    assert_eq!(&results[1].1[..], b"v-002");
    assert_eq!(&results[2].1[..], b"v-004");
    assert_eq!(&results[3].1[..], b"v-005");

    // ── Phase 5: Force compaction (L0 threshold = 4) ──
    db.put(b"k-010".to_vec(), b"v-010".to_vec())?;
    db.flush()?;
    // L0 now has 4 SSTables → compact(0) is triggered inside flush_immutable
    // After compaction: L0 empty, L1 has 1 merged SSTable

    // Verify all data survives compaction
    assert_eq!(db.get(b"alpha")?, Some(b"updated".to_vec()));
    assert_eq!(db.get(b"k-001")?, Some(b"memtable-override".to_vec()));
    assert_eq!(db.get(b"k-003")?, None);
    assert_eq!(db.get(b"k-010")?, Some(b"v-010".to_vec()));

    // Scan after compaction
    let results = db.scan(b"k-001", b"k-006")?;
    assert_eq!(results.len(), 4);

    // ── Phase 6: Tombstone shadowing across flushes ──
    // Put a key, flush it (now in SSTable), delete it (in memtable),
    // flush again (tombstone in new SSTable)
    db.put(b"shadow-test".to_vec(), b"original".to_vec())?;
    db.flush()?;
    db.delete(b"shadow-test".to_vec())?;
    db.flush()?;
    assert_eq!(db.get(b"shadow-test")?, None);

    // ── Phase 7: Empty scan ──
    let results = db.scan(b"nonexistent-a", b"nonexistent-z")?;
    assert!(results.is_empty());

    // ── Phase 8: Overwrite and scan ──
    db.put(b"k-050".to_vec(), b"v-050".to_vec())?;
    db.put(b"k-050".to_vec(), b"v-050-overwrite".to_vec())?;
    assert_eq!(db.get(b"k-050")?, Some(b"v-050-overwrite".to_vec()));

    println!("All LSM-Tree engine tests passed.");
    Ok(())
}

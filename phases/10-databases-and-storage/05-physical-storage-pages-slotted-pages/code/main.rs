//! Physical Storage — Pages, Slotted Pages
//! Phase 10 — Databases & Storage Systems
//!
//! Run:   cargo build && cargo run
//! Test:  cargo test

const PAGE_SIZE: usize = 4096;
const HEADER_SIZE: usize = 24;
const SLOT_ENTRY_SIZE: usize = 4;

fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn write_u16(buf: &mut [u8], offset: usize, val: u16) {
    let b = val.to_le_bytes();
    buf[offset] = b[0];
    buf[offset + 1] = b[1];
}

fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}

fn write_u32(buf: &mut [u8], offset: usize, val: u32) {
    let b = val.to_le_bytes();
    buf[offset] = b[0];
    buf[offset + 1] = b[1];
    buf[offset + 2] = b[2];
    buf[offset + 3] = b[3];
}

/// A slotted page — fixed-size buffer with variable-length records.
///
/// Layout:
/// ```text
/// [Header 24B] [Slot Array → ... ] ← Free Space → ... [← Record Data] [← PAGE_SIZE]
/// ```
///
/// Header fields:
///   [0..4)  page_id      (u32)
///   [4..6)  free_start   (u16)  — first byte of free space (end of slot array)
///   [6..8)  data_end     (u16)  — first byte before record data (grows down)
///   [8..10) slot_count   (u16)
///   [10..24) reserved
///
/// Slot entry (4 bytes per slot):
///   offset (u16) — byte offset of record data in the buffer
///   length (u16) — byte length of record data
///   An entry of (0, 0) indicates a deleted slot.
pub struct SlottedPage {
    buffer: [u8; PAGE_SIZE],
}

impl SlottedPage {
    pub fn new(page_id: u32) -> Self {
        let mut buffer = [0u8; PAGE_SIZE];
        write_u32(&mut buffer, 0, page_id);
        write_u16(&mut buffer, 4, HEADER_SIZE as u16);
        write_u16(&mut buffer, 6, PAGE_SIZE as u16);
        SlottedPage { buffer }
    }

    // --- header accessors ---

    fn free_start(&self) -> u16 {
        read_u16(&self.buffer, 4)
    }
    fn set_free_start(&mut self, v: u16) {
        write_u16(&mut self.buffer, 4, v);
    }
    fn data_end(&self) -> u16 {
        read_u16(&self.buffer, 6)
    }
    fn set_data_end(&mut self, v: u16) {
        write_u16(&mut self.buffer, 6, v);
    }
    fn slot_count(&self) -> u16 {
        read_u16(&self.buffer, 8)
    }
    fn set_slot_count(&mut self, v: u16) {
        write_u16(&mut self.buffer, 8, v);
    }

    fn free_space(&self) -> usize {
        self.data_end() as usize - self.free_start() as usize
    }

    fn slot_entry_base(&self, slot: u16) -> usize {
        HEADER_SIZE + slot as usize * SLOT_ENTRY_SIZE
    }
    fn slot_off(&self, slot: u16) -> u16 {
        let b = self.slot_entry_base(slot);
        read_u16(&self.buffer, b)
    }
    fn set_slot_off(&mut self, slot: u16, v: u16) {
        let b = self.slot_entry_base(slot);
        write_u16(&mut self.buffer, b, v);
    }
    fn slot_len(&self, slot: u16) -> u16 {
        let b = self.slot_entry_base(slot);
        read_u16(&self.buffer, b + 2)
    }
    fn set_slot_len(&mut self, slot: u16, v: u16) {
        let b = self.slot_entry_base(slot);
        write_u16(&mut self.buffer, b + 2, v);
    }

    // --- public API ---

    pub fn page_id(&self) -> u32 {
        read_u32(&self.buffer, 0)
    }

    pub fn utilization(&self) -> f64 {
        let used = PAGE_SIZE - self.free_space();
        used as f64 / PAGE_SIZE as f64
    }

    pub fn insert_record(&mut self, data: &[u8]) -> Result<u16, &str> {
        let needed = data.len() + SLOT_ENTRY_SIZE;
        if needed > self.free_space() {
            self.defragment();
            if needed > self.free_space() {
                return Err("page full");
            }
        }
        let slot = self.slot_count();
        let new_de = self.data_end() - data.len() as u16;
        self.buffer[new_de as usize..][..data.len()].copy_from_slice(data);
        self.set_slot_off(slot, new_de);
        self.set_slot_len(slot, data.len() as u16);
        self.set_data_end(new_de);
        self.set_slot_count(slot + 1);
        self.set_free_start(HEADER_SIZE as u16 + self.slot_count() * SLOT_ENTRY_SIZE as u16);
        Ok(slot)
    }

    pub fn get_record(&self, slot: u16) -> Option<&[u8]> {
        if slot >= self.slot_count() {
            return None;
        }
        let off = self.slot_off(slot);
        let len = self.slot_len(slot);
        if off == 0 && len == 0 {
            return None;
        }
        Some(&self.buffer[off as usize..][..len as usize])
    }

    pub fn delete_record(&mut self, slot: u16) {
        if slot >= self.slot_count() {
            return;
        }
        self.set_slot_off(slot, 0);
        self.set_slot_len(slot, 0);
    }

    pub fn update_record(&mut self, slot: u16, data: &[u8]) -> Result<(), &str> {
        if slot >= self.slot_count() {
            return Err("invalid slot");
        }
        let old_off = self.slot_off(slot);
        let old_len = self.slot_len(slot);
        if old_off == 0 && old_len == 0 {
            return Err("slot is deleted");
        }
        // Shrink: copy in place, update length
        if data.len() <= old_len as usize {
            let start = old_off as usize;
            self.buffer[start..][..data.len()].copy_from_slice(data);
            if data.len() < old_len as usize {
                self.buffer[start + data.len()..][..old_len as usize - data.len()].fill(0);
            }
            self.set_slot_len(slot, data.len() as u16);
            return Ok(());
        }
        // Grow: relocate to end
        self.set_slot_off(slot, 0);
        self.set_slot_len(slot, 0);
        let needed = data.len() + SLOT_ENTRY_SIZE;
        if needed > self.free_space() {
            self.defragment();
            if needed > self.free_space() {
                return Err("page full after defrag");
            }
        }
        let new_de = self.data_end() - data.len() as u16;
        self.buffer[new_de as usize..][..data.len()].copy_from_slice(data);
        self.set_slot_off(slot, new_de);
        self.set_slot_len(slot, data.len() as u16);
        self.set_data_end(new_de);
        Ok(())
    }

    pub fn defragment(&mut self) {
        let count = self.slot_count() as usize;
        if count == 0 {
            self.set_data_end(PAGE_SIZE as u16);
            return;
        }
        // Collect live records
        struct Rec {
            slot: usize,
            data: Vec<u8>,
        }
        let mut live: Vec<Rec> = Vec::with_capacity(count);
        for i in 0..count {
            let off = self.slot_off(i as u16);
            let len = self.slot_len(i as u16);
            if off != 0 || len != 0 {
                let slice = &self.buffer[off as usize..][..len as usize];
                live.push(Rec { slot: i, data: slice.to_vec() });
            }
        }
        self.set_data_end(PAGE_SIZE as u16);
        for rec in &live {
            let new_off = self.data_end() - rec.data.len() as u16;
            self.buffer[new_off as usize..][..rec.data.len()].copy_from_slice(&rec.data);
            self.set_slot_off(rec.slot as u16, new_off);
            self.set_slot_len(rec.slot as u16, rec.data.len() as u16);
            self.set_data_end(new_off);
        }
    }
}

/// A heap file — flat collection of slotted pages.
/// Allocates new pages when existing pages are full.
pub struct HeapFile {
    pages: Vec<SlottedPage>,
    next_page_id: u32,
}

impl HeapFile {
    pub fn new() -> Self {
        HeapFile { pages: Vec::new(), next_page_id: 0 }
    }

    pub fn insert_record(&mut self, data: &[u8]) -> (u32, u16) {
        for page in self.pages.iter_mut() {
            if let Ok(slot) = page.insert_record(data) {
                return (page.page_id(), slot);
            }
        }
        let mut page = SlottedPage::new(self.next_page_id);
        self.next_page_id += 1;
        let slot = page.insert_record(data).unwrap();
        let pid = page.page_id();
        self.pages.push(page);
        (pid, slot)
    }

    pub fn get_record(&self, pid: u32, slot: u16) -> Option<&[u8]> {
        self.pages.iter().find(|p| p.page_id() == pid)
            .and_then(|p| p.get_record(slot))
    }

    pub fn delete_record(&mut self, pid: u32, slot: u16) {
        if let Some(page) = self.pages.iter_mut().find(|p| p.page_id() == pid) {
            page.delete_record(slot);
        }
    }

    pub fn num_pages(&self) -> usize {
        self.pages.len()
    }
}

fn main() {
    let mut page = SlottedPage::new(1);

    let s0 = page.insert_record(b"hello").unwrap();
    let s1 = page.insert_record(b"world").unwrap();
    let s2 = page.insert_record(b"slotted page").unwrap();

    println!("page_id={} slots={} util={:.2}%",
        page.page_id(), page.slot_count(), page.utilization() * 100.0);

    for s in [s0, s1, s2] {
        if let Some(data) = page.get_record(s) {
            println!("  slot {}: {:?}", s, std::str::from_utf8(data).unwrap());
        }
    }

    page.delete_record(s1);
    println!("after delete slot {}: {:?}", s1, page.get_record(s1));

    page.update_record(s0, b"HELLO AGAIN").unwrap();
    println!("after update slot {}: {:?}", s0, std::str::from_utf8(page.get_record(s0).unwrap()).unwrap());

    page.defragment();
    println!("after defrag: util={:.2}%", page.utilization() * 100.0);
    for s in [s0, s1, s2] {
        if let Some(data) = page.get_record(s) {
            println!("  slot {}: {:?}", s, std::str::from_utf8(data).unwrap());
        }
    }

    let mut hf = HeapFile::new();
    let (pid, _) = hf.insert_record(b"heap file record");
    println!("\nheap file: page_id={}, num_pages={}", pid, hf.num_pages());
    assert_eq!(hf.get_record(pid, 0), Some(&b"heap file record"[..]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut p = SlottedPage::new(0);
        let s = p.insert_record(b"hello").unwrap();
        assert_eq!(p.get_record(s), Some(&b"hello"[..]));
    }

    #[test]
    fn test_delete() {
        let mut p = SlottedPage::new(0);
        let s = p.insert_record(b"hello").unwrap();
        p.delete_record(s);
        assert_eq!(p.get_record(s), None);
    }

    #[test]
    fn test_update_shrink() {
        let mut p = SlottedPage::new(0);
        let s = p.insert_record(b"longer data").unwrap();
        p.update_record(s, b"hi").unwrap();
        assert_eq!(p.get_record(s), Some(&b"hi"[..]));
    }

    #[test]
    fn test_update_grow() {
        let mut p = SlottedPage::new(0);
        let s = p.insert_record(b"hi").unwrap();
        p.update_record(s, b"longer data here").unwrap();
        assert_eq!(p.get_record(s), Some(&b"longer data here"[..]));
    }

    #[test]
    fn test_defrag_leaves_slot_numbers_stable() {
        let mut p = SlottedPage::new(0);
        let s0 = p.insert_record(b"aaa").unwrap();
        let s1 = p.insert_record(b"bbb").unwrap();
        let s2 = p.insert_record(b"ccc").unwrap();
        p.delete_record(s1);
        p.defragment();
        assert_eq!(p.get_record(s0), Some(&b"aaa"[..]));
        assert_eq!(p.get_record(s1), None);
        assert_eq!(p.get_record(s2), Some(&b"ccc"[..]));
    }

    #[test]
    fn test_page_full() {
        let mut p = SlottedPage::new(0);
        let big = vec![0u8; PAGE_SIZE - HEADER_SIZE - SLOT_ENTRY_SIZE];
        assert!(p.insert_record(&big).is_ok());
        assert!(p.insert_record(b"x").is_err());
    }

    #[test]
    fn test_heap_file_spills() {
        let mut hf = HeapFile::new();
        let big = vec![0u8; PAGE_SIZE - HEADER_SIZE - SLOT_ENTRY_SIZE];
        let (p1, _) = hf.insert_record(&big);
        let (p2, _) = hf.insert_record(b"spill");
        assert_ne!(p1, p2);
        assert_eq!(hf.num_pages(), 2);
    }

    #[test]
    fn test_utilization_nonzero() {
        let mut p = SlottedPage::new(0);
        assert!(p.utilization() > 0.0);
        let _ = p.insert_record(b"data");
        let u = p.utilization();
        assert!(u > 0.0 && u < 1.0);
    }
}

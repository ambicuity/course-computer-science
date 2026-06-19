#![allow(dead_code)]

use std::collections::HashMap;

type LSN = u64;
type PageID = u64;
type TransID = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
enum RecordType {
    Begin,
    Update,
    Commit,
    Abort,
    CLR,
    Checkpoint,
}

#[derive(Clone, Debug)]
struct LogRecord {
    lsn: LSN,
    prev_lsn: LSN,
    trans_id: TransID,
    rtype: RecordType,
    page_id: PageID,
    before_image: Vec<u8>,
    after_image: Vec<u8>,
    undo_next_lsn: LSN,
}

impl LogRecord {
    fn begin(trans_id: TransID, prev_lsn: LSN) -> Self {
        LogRecord {
            lsn: 0,
            prev_lsn,
            trans_id,
            rtype: RecordType::Begin,
            page_id: 0,
            before_image: vec![],
            after_image: vec![],
            undo_next_lsn: 0,
        }
    }

    fn update(trans_id: TransID, page_id: PageID, before: &str, after: &str, prev_lsn: LSN) -> Self {
        LogRecord {
            lsn: 0,
            prev_lsn,
            trans_id,
            rtype: RecordType::Update,
            page_id,
            before_image: before.as_bytes().to_vec(),
            after_image: after.as_bytes().to_vec(),
            undo_next_lsn: 0,
        }
    }

    fn commit(trans_id: TransID, prev_lsn: LSN) -> Self {
        LogRecord {
            lsn: 0,
            prev_lsn,
            trans_id,
            rtype: RecordType::Commit,
            page_id: 0,
            before_image: vec![],
            after_image: vec![],
            undo_next_lsn: 0,
        }
    }

    fn abort(trans_id: TransID, prev_lsn: LSN) -> Self {
        LogRecord {
            lsn: 0,
            prev_lsn,
            trans_id,
            rtype: RecordType::Abort,
            page_id: 0,
            before_image: vec![],
            after_image: vec![],
            undo_next_lsn: 0,
        }
    }
}

struct LogManager {
    records: Vec<LogRecord>,
    next_lsn: LSN,
}

impl LogManager {
    fn new() -> Self {
        LogManager {
            records: vec![],
            next_lsn: 1,
        }
    }

    fn append(&mut self, mut r: LogRecord) -> LSN {
        r.lsn = self.next_lsn;
        self.next_lsn += 1;
        self.records.push(r);
        self.next_lsn - 1
    }

    fn into_records(self) -> Vec<LogRecord> {
        self.records
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TransStatus {
    InProgress,
    Committed,
    Aborted,
}

#[derive(Clone, Debug)]
struct TransState {
    status: TransStatus,
    last_lsn: LSN,
}

#[derive(Clone, Debug)]
struct Page {
    page_id: PageID,
    data: Vec<u8>,
    page_lsn: LSN,
}

impl Page {
    fn new(page_id: PageID, data: &str) -> Self {
        Page {
            page_id,
            data: data.as_bytes().to_vec(),
            page_lsn: 0,
        }
    }

    fn data_as_str(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }
}

struct Recovery {
    log: Vec<LogRecord>,
}

impl Recovery {
    fn new(log: Vec<LogRecord>) -> Self {
        Recovery { log }
    }

    fn recover(&self, disk: &mut HashMap<PageID, Page>) {
        let (trans_table, dpt) = self.analysis();
        self.redo(&dpt, disk);
        self.undo(&trans_table, disk);
    }

    fn analysis(&self) -> (HashMap<TransID, TransState>, HashMap<PageID, LSN>) {
        let mut tt: HashMap<TransID, TransState> = HashMap::new();
        let mut dpt: HashMap<PageID, LSN> = HashMap::new();

        for r in &self.log {
            match r.rtype {
                RecordType::Begin => {
                    tt.insert(
                        r.trans_id,
                        TransState {
                            status: TransStatus::InProgress,
                            last_lsn: r.lsn,
                        },
                    );
                }
                RecordType::Update | RecordType::CLR => {
                    if let Some(t) = tt.get_mut(&r.trans_id) {
                        t.last_lsn = r.lsn;
                    }
                    dpt.entry(r.page_id).or_insert(r.lsn);
                }
                RecordType::Commit => {
                    if let Some(t) = tt.get_mut(&r.trans_id) {
                        t.status = TransStatus::Committed;
                        t.last_lsn = r.lsn;
                    }
                }
                RecordType::Abort => {
                    if let Some(t) = tt.get_mut(&r.trans_id) {
                        t.status = TransStatus::Aborted;
                        t.last_lsn = r.lsn;
                    }
                }
                RecordType::Checkpoint => {}
            }
        }

        (tt, dpt)
    }

    fn redo(&self, dpt: &HashMap<PageID, LSN>, disk: &mut HashMap<PageID, Page>) {
        let min_lsn = match dpt.values().min() {
            Some(&lsn) => lsn,
            None => return,
        };

        for r in &self.log {
            if r.lsn < min_lsn {
                continue;
            }
            if r.rtype != RecordType::Update && r.rtype != RecordType::CLR {
                continue;
            }

            let page = disk
                .entry(r.page_id)
                .or_insert_with(|| Page::new(r.page_id, ""));
            if page.page_lsn < r.lsn {
                page.data = r.after_image.clone();
                page.page_lsn = r.lsn;
            }
        }
    }

    fn undo(&self, trans_table: &HashMap<TransID, TransState>, disk: &mut HashMap<PageID, Page>) {
        let losers: Vec<TransID> = trans_table
            .iter()
            .filter(|(_, s)| s.status == TransStatus::InProgress)
            .map(|(&id, _)| id)
            .collect();

        let mut to_undo: Vec<(LSN, &LogRecord)> = Vec::new();
        for &tid in &losers {
            let mut lsn = trans_table[&tid].last_lsn;
            while lsn > 0 {
                let r = &self.log[(lsn - 1) as usize];
                match r.rtype {
                    RecordType::Update => {
                        to_undo.push((lsn, r));
                        lsn = r.prev_lsn;
                    }
                    RecordType::CLR => {
                        lsn = r.undo_next_lsn;
                    }
                    _ => break,
                }
            }
        }

        to_undo.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, r) in &to_undo {
            let page = disk
                .entry(r.page_id)
                .or_insert_with(|| Page::new(r.page_id, ""));
            page.data = r.before_image.clone();
        }
    }
}

fn print_disk(disk: &HashMap<PageID, Page>, label: &str) {
    println!("{}:", label);
    for pid in 1..=3 {
        let status = disk.get(&pid);
        match status {
            Some(p) => println!("  page{}: '{}' (page_lsn={})", pid, p.data_as_str(), p.page_lsn),
            None => println!("  page{}: <missing>", pid),
        }
    }
}

fn main() {
    println!("=== ARIES Recovery Simulator ===\n");
    println!("Scenario:");
    println!("  T1: BEGIN -> UPDATE(page1, 111) -> UPDATE(page2, 111) -> COMMIT");
    println!("  T2: BEGIN -> UPDATE(page1, 222) -> UPDATE(page3, 222) -> CRASH (no commit)");
    println!();
    println!("Before crash, STEAL wrote page1 (T2's uncommitted '222') to disk.");
    println!("NO-FORCE means page2 (T1's committed '111') was NOT on disk yet.\n");

    let mut log = LogManager::new();

    let t1 = 1;
    let l1 = log.append(LogRecord::begin(t1, 0));
    let l2 = log.append(LogRecord::update(t1, 1, "", "111", l1));
    let l3 = log.append(LogRecord::update(t1, 2, "", "111", l2));
    let _l4 = log.append(LogRecord::commit(t1, l3));

    let t2 = 2;
    let l5 = log.append(LogRecord::begin(t2, 0));
    let l6 = log.append(LogRecord::update(t2, 1, "111", "222", l5));
    let _l7 = log.append(LogRecord::update(t2, 3, "", "222", l6));

    let mut disk: HashMap<PageID, Page> = HashMap::new();
    disk.insert(1, {
        let mut p = Page::new(1, "222");
        p.page_lsn = l6;
        p
    });
    disk.insert(2, Page::new(2, ""));
    disk.insert(3, Page::new(3, ""));

    print_disk(&disk, "Disk state BEFORE recovery");

    recovery_log(Some(&log));

    let recovery = Recovery::new(log.into_records());
    recovery.recover(&mut disk);

    println!();
    print_disk(&disk, "Disk state AFTER recovery");

    assert_eq!(disk[&1].data_as_str(), "111");
    assert_eq!(disk[&2].data_as_str(), "111");
    assert_eq!(disk[&3].data_as_str(), "");

    println!("\nAll assertions passed. Recovery is correct.");
    println!("  page1 = '111' (T2's '222' undone, T1's '111' restored)");
    println!("  page2 = '111' (T1's committed change redone)");
    println!("  page3 = ''    (T2's uncommitted change undone)");
}

fn recovery_log(log: Option<&LogManager>) {
    if let Some(lm) = log {
        println!("\nWAL contents:");
        for r in &lm.records {
            let desc = match r.rtype {
                RecordType::Begin => format!("T{} BEGIN", r.trans_id),
                RecordType::Update => {
                    format!(
                        "T{} UPDATE page{} '{}' -> '{}'",
                        r.trans_id,
                        r.page_id,
                        String::from_utf8_lossy(&r.before_image),
                        String::from_utf8_lossy(&r.after_image)
                    )
                }
                RecordType::Commit => format!("T{} COMMIT", r.trans_id),
                RecordType::Abort => format!("T{} ABORT", r.trans_id),
                RecordType::CLR => format!(
                    "T{} CLR page{} undoNext={}",
                    r.trans_id, r.page_id, r.undo_next_lsn
                ),
                RecordType::Checkpoint => "CHECKPOINT".to_string(),
            };
            println!("  LSN={}: {} (prevLSN={})", r.lsn, desc, r.prev_lsn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_recovery(log: LogManager, disk: &mut HashMap<PageID, Page>) {
        let recovery = Recovery::new(log.into_records());
        recovery.recover(disk);
    }

    #[test]
    fn test_basic_recovery() {
        let mut log = LogManager::new();
        let t1 = 1;
        let l1 = log.append(LogRecord::begin(t1, 0));
        let l2 = log.append(LogRecord::update(t1, 1, "", "111", l1));
        let l3 = log.append(LogRecord::update(t1, 2, "", "111", l2));
        log.append(LogRecord::commit(t1, l3));

        let t2 = 2;
        let l5 = log.append(LogRecord::begin(t2, 0));
        let l6 = log.append(LogRecord::update(t2, 1, "111", "222", l5));
        log.append(LogRecord::update(t2, 3, "", "222", l6));

        let mut disk: HashMap<PageID, Page> = HashMap::new();
        disk.insert(1, {
            let mut p = Page::new(1, "222");
            p.page_lsn = l6;
            p
        });
        disk.insert(2, Page::new(2, ""));
        disk.insert(3, Page::new(3, ""));

        run_recovery(log, &mut disk);

        assert_eq!(disk[&1].data_as_str(), "111");
        assert_eq!(disk[&2].data_as_str(), "111");
        assert_eq!(disk[&3].data_as_str(), "");
    }

    #[test]
    fn test_all_committed() {
        let mut log = LogManager::new();
        let t1 = 1;
        let l1 = log.append(LogRecord::begin(t1, 0));
        let l2 = log.append(LogRecord::update(t1, 1, "", "111", l1));
        let l3 = log.append(LogRecord::update(t1, 2, "", "111", l2));
        log.append(LogRecord::commit(t1, l3));

        let mut disk: HashMap<PageID, Page> = HashMap::new();
        disk.insert(1, Page::new(1, ""));
        disk.insert(2, Page::new(2, ""));

        run_recovery(log, &mut disk);

        assert_eq!(disk[&1].data_as_str(), "111");
        assert_eq!(disk[&2].data_as_str(), "111");
    }

    #[test]
    fn test_undo_in_progress() {
        let mut log = LogManager::new();
        let t1 = 1;
        let l1 = log.append(LogRecord::begin(t1, 0));
        let l2 = log.append(LogRecord::update(t1, 1, "", "AAA", l1));
        let _l3 = log.append(LogRecord::update(t1, 2, "", "BBB", l2));

        let mut disk: HashMap<PageID, Page> = HashMap::new();
        disk.insert(1, {
            let mut p = Page::new(1, "AAA");
            p.page_lsn = l2;
            p
        });
        disk.insert(2, Page::new(2, ""));

        run_recovery(log, &mut disk);

        assert_eq!(disk[&1].data_as_str(), "");
        assert_eq!(disk[&2].data_as_str(), "");
    }

    #[test]
    fn test_page_not_in_dpt_skipped_during_redo() {
        let mut log = LogManager::new();
        let t1 = 1;
        let l1 = log.append(LogRecord::begin(t1, 0));
        log.append(LogRecord::update(t1, 1, "", "data", l1));
        log.append(LogRecord::commit(t1, l1));

        let mut disk: HashMap<PageID, Page> = HashMap::new();
        disk.insert(2, Page::new(2, "should-not-change"));

        run_recovery(log, &mut disk);

        assert_eq!(disk[&1].data_as_str(), "data");
        assert_eq!(disk[&2].data_as_str(), "should-not-change");
    }

    #[test]
    fn test_single_page_multiple_transactions() {
        let mut log = LogManager::new();
        let t1 = 1;
        let l1 = log.append(LogRecord::begin(t1, 0));
        let l2 = log.append(LogRecord::update(t1, 1, "", "v1", l1));
        log.append(LogRecord::commit(t1, l2));

        let t2 = 2;
        let l4 = log.append(LogRecord::begin(t2, 0));
        let l5 = log.append(LogRecord::update(t2, 1, "v1", "v2", l4));
        log.append(LogRecord::commit(t2, l5));

        let t3 = 3;
        let l7 = log.append(LogRecord::begin(t3, 0));
        log.append(LogRecord::update(t3, 1, "v2", "v3", l7));

        let mut disk: HashMap<PageID, Page> = HashMap::new();
        disk.insert(1, {
            let mut p = Page::new(1, "v2");
            p.page_lsn = l5;
            p
        });

        run_recovery(log, &mut disk);

        assert_eq!(disk[&1].data_as_str(), "v2");
    }
}

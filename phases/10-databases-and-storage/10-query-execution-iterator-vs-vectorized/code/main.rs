// Query Execution — Iterator vs Vectorized: Rust Volcano-style + batch executor.

use std::collections::HashMap;
use std::time::Instant;

type Value = i64;
type Row = Vec<(String, Value)>;

trait Executor {
    fn next(&mut self) -> Option<Row>;
}

trait BatchExecutor {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>>;
}

struct Scan {
    data: Vec<Row>,
    pos: usize,
}

impl Scan {
    fn new(data: Vec<Row>) -> Self {
        Scan { data, pos: 0 }
    }
}

impl Executor for Scan {
    fn next(&mut self) -> Option<Row> {
        if self.pos >= self.data.len() {
            return None;
        }
        let row = self.data[self.pos].clone();
        self.pos += 1;
        Some(row)
    }
}

impl BatchExecutor for Scan {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>> {
        if self.pos >= self.data.len() {
            return None;
        }
        let end = std::cmp::min(self.pos + batch_size, self.data.len());
        let batch = self.data[self.pos..end].to_vec();
        self.pos = end;
        Some(batch)
    }
}

struct Filter {
    child: Box<dyn Executor>,
    predicate: fn(&Row) -> bool,
}

impl Executor for Filter {
    fn next(&mut self) -> Option<Row> {
        while let Some(row) = self.child.next() {
            if (self.predicate)(&row) {
                return Some(row);
            }
        }
        None
    }
}

struct BatchFilter {
    child: Box<dyn BatchExecutor>,
    predicate: fn(&Row) -> bool,
}

impl BatchExecutor for BatchFilter {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>> {
        let batch = self.child.next_batch(batch_size)?;
        let result: Vec<Row> = batch.into_iter().filter(self.predicate).collect();
        if result.is_empty() {
            return self.next_batch(batch_size);
        }
        Some(result)
    }
}

struct Projection {
    child: Box<dyn Executor>,
    cols: Vec<String>,
}

impl Executor for Projection {
    fn next(&mut self) -> Option<Row> {
        let row = self.child.next()?;
        Some(
            row.into_iter()
                .filter(|(k, _)| self.cols.contains(k))
                .collect(),
        )
    }
}

struct BatchProjection {
    child: Box<dyn BatchExecutor>,
    cols: Vec<String>,
}

impl BatchExecutor for BatchProjection {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>> {
        let batch = self.child.next_batch(batch_size)?;
        let result: Vec<Row> = batch
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .filter(|(k, _)| self.cols.contains(k))
                    .collect()
            })
            .collect();
        Some(result)
    }
}

struct NLJoin {
    left: Box<dyn Executor>,
    right_data: Vec<Row>,
    right_pos: usize,
    outer_row: Option<Row>,
    condition: fn(&Row, &Row) -> bool,
}

impl NLJoin {
    fn new(
        left: Box<dyn Executor>,
        right: Box<dyn Executor>,
        condition: fn(&Row, &Row) -> bool,
    ) -> Self {
        let mut right = right;
        let mut right_data = Vec::new();
        while let Some(row) = right.next() {
            right_data.push(row);
        }
        NLJoin {
            left,
            right_data,
            right_pos: 0,
            outer_row: None,
            condition,
        }
    }
}

impl Executor for NLJoin {
    fn next(&mut self) -> Option<Row> {
        loop {
            if self.outer_row.is_none() {
                self.outer_row = self.left.next()?;
                self.right_pos = 0;
            }
            let outer = self.outer_row.as_ref().unwrap();
            while self.right_pos < self.right_data.len() {
                let inner = &self.right_data[self.right_pos];
                self.right_pos += 1;
                if (self.condition)(outer, inner) {
                    let mut merged = outer.clone();
                    merged.extend(inner.clone());
                    return Some(merged);
                }
            }
            self.outer_row = None;
        }
    }
}

fn make_dataset(count: usize) -> Vec<Row> {
    let mut data = Vec::with_capacity(count);
    for i in 0..count {
        data.push(vec![
            ("id".to_string(), i as Value),
            ("value".to_string(), (i * 7) % 1000),
            ("group".to_string(), (i % 5) as Value),
        ]);
    }
    data
}

fn main() {
    let data: Vec<Row> = (0..10)
        .map(|i| {
            vec![
                ("id".to_string(), i),
                ("value".to_string(), (i * 7) % 1000),
                ("group".to_string(), (i % 3) as Value),
            ]
        })
        .collect();

    let scan = Box::new(Scan::new(data.clone()));
    let filter = Box::new(Filter {
        child: scan,
        predicate: |r| r.iter().any(|(k, v)| k == "value" && *v > 30),
    });
    let proj = Box::new(Projection {
        child: filter,
        cols: vec!["id".to_string(), "value".to_string()],
    });

    println!("Iterator model results:");
    let mut exec: Box<dyn Executor> = proj;
    let mut count = 0;
    while let Some(row) = exec.next() {
        println!("  {:?}", row);
        count += 1;
    }
    println!("  ({} rows)", count);

    let left_data: Vec<Row> = (0..5)
        .map(|i| vec![("id".to_string(), i), ("name".to_string(), i + 100)])
        .collect();
    let right_data: Vec<Row> = (0..5)
        .map(|i| vec![("uid".to_string(), i), ("amount".to_string(), i * 50)])
        .collect();

    let left_scan = Box::new(Scan::new(left_data));
    let right_scan = Box::new(Scan::new(right_data));
    let mut join = NLJoin::new(left_scan, right_scan, |l, r| {
        l.iter().any(|(k, v)| k == "id")
            && r.iter().any(|(k2, v2)| k2 == "uid" && v2 == v)
    });

    println!("\nJoin results:");
    let mut count = 0;
    while let Some(row) = join.next() {
        println!("  {:?}", row);
        count += 1;
    }
    println!("  ({} rows)", count);

    let big_data = make_dataset(1_000_000);

    let scan_iter = Box::new(Scan::new(big_data.clone()));
    let filter_iter = Box::new(Filter {
        child: scan_iter,
        predicate: |r| r.iter().any(|(k, v)| k == "value" && *v > 500),
    });
    let proj_iter = Box::new(Projection {
        child: filter_iter,
        cols: vec!["id".to_string(), "value".to_string()],
    });

    let start = Instant::now();
    let mut exec_iter: Box<dyn Executor> = proj_iter;
    let mut iter_count = 0;
    while let Some(_row) = exec_iter.next() {
        iter_count += 1;
    }
    let iter_dur = start.elapsed();

    let scan_batch = Box::new(Scan::new(big_data));
    let filter_batch = Box::new(BatchFilter {
        child: scan_batch,
        predicate: |r| r.iter().any(|(k, v)| k == "value" && *v > 500),
    });
    let proj_batch = Box::new(BatchProjection {
        child: filter_batch,
        cols: vec!["id".to_string(), "value".to_string()],
    });

    let start = Instant::now();
    let mut exec_batch: Box<dyn BatchExecutor> = proj_batch;
    let mut batch_count = 0;
    while let Some(batch) = exec_batch.next_batch(1024) {
        batch_count += batch.len();
    }
    let batch_dur = start.elapsed();

    println!("\n--- Performance: 1M rows, Filter(value > 500) ---");
    println!("Iterator model: {:?} ({} rows)", iter_dur, iter_count);
    println!("Batch model:    {:?} ({} rows)", batch_dur, batch_count);
    println!(
        "Speedup: {:.2}x",
        iter_dur.as_secs_f64() / batch_dur.as_secs_f64()
    );
}

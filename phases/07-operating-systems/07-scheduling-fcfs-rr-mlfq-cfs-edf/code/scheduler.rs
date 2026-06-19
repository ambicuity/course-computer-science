use std::collections::VecDeque;

#[derive(Clone, Debug)]
struct Process {
    pid: u32,
    arrival: i32,
    burst: i32,
    remaining: i32,
    state: State,
    start_time: Option<i32>,
    finish_time: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
enum State {
    Ready,
    Running,
    Done,
}

#[derive(Debug)]
struct GanttEntry {
    pid: u32,
    start: i32,
    end: i32,
}

struct Gantt {
    entries: Vec<GanttEntry>,
}

impl Gantt {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn push(&mut self, pid: u32, start: i32, end: i32) {
        if let Some(last) = self.entries.last_mut() {
            if last.pid == pid && last.end == start {
                last.end = end;
                return;
            }
        }
        self.entries.push(GanttEntry { pid, start, end });
    }

    fn print(&self, label: &str) {
        print!("\n{label} Gantt Chart:\n|");
        for e in &self.entries {
            print!(" P{} ({}-{}) |", e.pid, e.start, e.end);
        }
        println!();
    }
}

fn print_metrics(procs: &[Process], n: usize) {
    let (mut tt, mut tw, mut tr) = (0.0, 0.0, 0.0);
    println!("\n{:<5} {:<10} {:<6} {:<10} {:<8}", "PID", "Turnaround", "Wait", "Response", "Finish");
    println!("{}", "-".repeat(45));
    for p in procs {
        let turn = p.finish_time.unwrap() - p.arrival;
        let wait = turn - p.burst;
        let resp = p.start_time.unwrap() - p.arrival;
        tt += turn as f64;
        tw += wait as f64;
        tr += resp as f64;
        println!("P{:<4} {:<10} {:<6} {:<10} {:<8}", p.pid, turn, wait, resp, p.finish_time.unwrap());
    }
    println!("{}", "-".repeat(45));
    println!("Avg   {:<10.2} {:<6.2} {:<10.2}", tt / n as f64, tw / n as f64, tr / n as f64);
}

fn make_procs(src: &[(u32, i32, i32)]) -> Vec<Process> {
    let mut v: Vec<Process> = src
        .iter()
        .map(|&(pid, arrival, burst)| Process {
            pid,
            arrival,
            burst,
            remaining: burst,
            state: State::Ready,
            start_time: None,
            finish_time: None,
        })
        .collect();
    v.sort_by_key(|p| (p.arrival, p.pid));
    v
}

/* ── FCFS ─────────────────────────────────────────────── */

fn fcfs(procs: &[Process]) -> Vec<Process> {
    let mut p: Vec<Process> = procs.to_vec();
    p.sort_by_key(|pr| (pr.arrival, pr.pid));
    let n = p.len();
    let mut gantt = Gantt::new();
    let mut time = 0;
    let mut done = 0;

    while done < n {
        if let Some(idx) = p.iter().position(|pr| pr.state == State::Ready && pr.arrival <= time) {
            if p[idx].start_time.is_none() {
                p[idx].start_time = Some(time);
            }
            let start = time;
            time += p[idx].burst;
            p[idx].finish_time = Some(time);
            p[idx].remaining = 0;
            p[idx].state = State::Done;
            done += 1;
            gantt.push(p[idx].pid, start, time);
        } else {
            time += 1;
        }
    }
    gantt.print("FCFS");
    print_metrics(&p, n);
    p
}

/* ── Round Robin ──────────────────────────────────────── */

fn round_robin(procs: &[Process], quantum: i32) -> Vec<Process> {
    let mut p: Vec<Process> = procs.to_vec();
    p.sort_by_key(|pr| (pr.arrival, pr.pid));
    let n = p.len();
    let mut gantt = Gantt::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut inq = vec![false; n];

    let mut time = 0;
    let mut done = 0;
    let mut next = 0;

    while done < n {
        while next < n && p[next].arrival <= time {
            if !inq[next] && p[next].state != State::Done {
                queue.push_back(next);
                inq[next] = true;
            }
            next += 1;
        }
        if queue.is_empty() {
            time += 1;
            continue;
        }

        let idx = queue.pop_front().unwrap();
        inq[idx] = false;
        if p[idx].start_time.is_none() {
            p[idx].start_time = Some(time);
        }

        let run = p[idx].remaining.min(quantum);
        let start = time;
        time += run;
        p[idx].remaining -= run;

        while next < n && p[next].arrival <= time {
            if !inq[next] && p[next].state != State::Done {
                queue.push_back(next);
                inq[next] = true;
            }
            next += 1;
        }

        if p[idx].remaining == 0 {
            p[idx].state = State::Done;
            p[idx].finish_time = Some(time);
            done += 1;
        } else {
            queue.push_back(idx);
            inq[idx] = true;
        }
        gantt.push(p[idx].pid, start, time);
    }
    gantt.print(&format!("RR (q={quantum})"));
    print_metrics(&p, n);
    p
}

/* ── CFS (simplified) ────────────────────────────────── */

fn cfs(procs: &[Process]) -> Vec<Process> {
    let mut p: Vec<Process> = procs.to_vec();
    p.sort_by_key(|pr| (pr.arrival, pr.pid));
    let n = p.len();
    let mut gantt = Gantt::new();

    #[derive(Clone)]
    struct CfsTask {
        idx: usize,
        vruntime: i64,
    }

    let mut tree: Vec<CfsTask> = Vec::new();
    let mut time: i32 = 0;
    let mut done: usize = 0;
    let mut next: usize = 0;
    let sched_latency: i32 = 48;

    while done < n {
        while next < n && p[next].arrival <= time {
            tree.push(CfsTask { idx: next, vruntime: 0 });
            tree.sort_by_key(|t| t.vruntime);
            next += 1;
        }
        if tree.is_empty() {
            time += 1;
            continue;
        }

        let task = tree.remove(0);
        let idx = task.idx;
        if p[idx].start_time.is_none() {
            p[idx].start_time = Some(time);
        }

        let weight = 1024u64;
        let total_weight: u64 = tree.iter().map(|_| 1024u64).sum::<u64>() + weight;
        let slice = ((sched_latency as u64) * weight / total_weight).max(1) as i32;
        let run = p[idx].remaining.min(slice);

        let start = time;
        time += run;
        p[idx].remaining -= run;
        let new_vr = task.vruntime + (run as i64 * 1024 / weight as i64);

        while next < n && p[next].arrival <= time {
            tree.push(CfsTask { idx: next, vruntime: new_vr });
            tree.sort_by_key(|t| t.vruntime);
            next += 1;
        }

        if p[idx].remaining == 0 {
            p[idx].state = State::Done;
            p[idx].finish_time = Some(time);
            done += 1;
        } else {
            tree.push(CfsTask { idx, vruntime: new_vr });
            tree.sort_by_key(|t| t.vruntime);
        }
        gantt.push(p[idx].pid, start, time);
    }
    gantt.print("CFS");
    print_metrics(&p, n);
    p
}

/* ── Main ─────────────────────────────────────────────── */

fn main() {
    let data = vec![
        (1, 0, 24),
        (2, 1, 3),
        (3, 2, 3),
        (4, 3, 12),
        (5, 5, 6),
    ];

    println!("=== Process Table ===");
    println!("{:<5} {:<8} {:<6}", "PID", "Arrival", "Burst");
    for &(pid, arr, bur) in &data {
        println!("P{:<4} {:<8} {:<6}", pid, arr, bur);
    }

    let procs = make_procs(&data);

    fcfs(&procs);
    round_robin(&procs, 4);
    cfs(&procs);

    println!("\n=== Algorithm Comparison ===");
    println!("FCFS:  Simple, convoy effect, poor response for short jobs");
    println!("RR(4): Fair, good response, more context switches");
    println!("CFS:   Fair by vruntime, red-black tree, Linux default");
}

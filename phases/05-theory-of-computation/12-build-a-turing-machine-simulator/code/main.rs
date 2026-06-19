#[derive(Clone, Copy)]
enum Move { L, R }

fn step(state: char, sym: char) -> (char, char, Move) {
    match (state, sym) {
        ('q', '1') => ('q', '1', Move::R),
        ('q', '_') => ('h', '1', Move::R),
        _ => ('h', sym, Move::R),
    }
}

fn main() {
    let mut tape = vec!['1', '1', '_', '_'];
    let mut head = 0usize;
    let mut state = 'q';

    while state != 'h' && head < tape.len() {
        let (ns, write, mv) = step(state, tape[head]);
        tape[head] = write;
        state = ns;
        match mv {
            Move::L => head = head.saturating_sub(1),
            Move::R => head += 1,
        }
    }

    let out: String = tape.into_iter().collect();
    println!("{out}");
}

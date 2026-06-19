# Build a Chess Engine with Search & Eval

> Strong play emerges from disciplined move generation plus efficient search pruning.

**Type:** Build
**Languages:** Rust, C++
**Prerequisites:** Phase 19 lessons 01-06
**Time:** ~720 minutes

## Learning Objectives

- Build a legal move generator baseline.
- Implement minimax/alpha-beta skeleton with depth limits.
- Design an evaluation heuristic with material/position factors.
- Define correctness and performance checkpoints.

## The Problem

Chess engines fail when search and board rules are developed without validation gates. Someone starts with a board representation, adds move generation, discovers that castling is broken, tries to fix it while also implementing alpha-beta, and can't tell whether bad play is due to a move generation bug, a search bug, or an evaluation bug.

The fix: staged development. First, build a move generator and verify it produces the correct number of legal moves for known positions (perft testing). Second, add alpha-beta search over a simple material-only evaluation. Third, gradually add positional evaluation terms (piece-square tables, pawn structure). Each stage is independently verifiable.

A chess engine is two things: a move generator (the rules of chess) and a search algorithm (finding the best move). The move generator must be 100% correct. The search algorithm can be approximate. If the move generator produces illegal moves or misses legal ones, the search is meaningless.

## The Concept

A chess engine has four components:

```
Position (board state)
    │
    ▼
┌──────────────────┐
│ Move Generator    │  All legal moves from current position
│ (rules of chess)  │  Pawns, knights, bishops, rooks, queens, kings
└──────────────────┘
    │
    ▼
┌──────────────────┐
│ Search            │  Minimax with alpha-beta pruning
│ (tree search)     │  Explore game tree to find best move
└──────────────────┘
    │
    ▼
┌──────────────────┐
│ Evaluation        │  Score a leaf position
│ (heuristic)       │  Material + positional factors
└──────────────────┘
```

The search uses the minimax algorithm: assume both players play optimally. At each level, the side to move picks the move that maximizes its score; the opponent picks the move that minimizes it. Alpha-beta pruning skips branches that provably can't improve the result.

```
Minimax tree (depth 2):

         MAX (White)
        /     |     \
      +3     -1     +2     ← White's move scores
      /       |       \
   MIN      MIN      MIN   ← Black's responses
  / | \    / | \    / | \
 3  5  3  -1 -2 -1  2  1  2

Alpha-beta: if we've already found +3 at the leftmost branch,
Black's second branch (-1, -2, -1) is pruned because Black can
force a score ≤ -1, which White already has +3 to beat.
```

## Build It

### Step 1: Board Representation

```rust
use std::fmt;

type Square = u8; // 0-63, a1=0, h8=63

#[derive(Debug, Clone, Copy, PartialEq)]
enum Piece {
    Pawn, Knight, Bishop, Rook, Queen, King,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Color { White, Black }

impl Color {
    fn other(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Move {
    from: Square,
    to: Square,
    promotion: Option<Piece>, // For pawn promotion
}

#[derive(Clone)]
struct Board {
    // Piece placement: piece[0..5] for white, piece[6..11] for black
    // Using bitboards: each u64 has bit i set if piece is on square i
    pieces: [u64; 12],
    side_to_move: Color,
    castling: u8,       // KQkq bits
    ep_square: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
}

impl Board {
    fn starting_position() -> Self {
        let mut board = Board {
            pieces: [0u64; 12],
            side_to_move: Color::White,
            castling: 0b1111, // KQkq all available
            ep_square: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        };
        // White pieces
        board.pieces[0] = 0x00FF000000000000; // Pawns on rank 2
        board.pieces[1] = 0x4200000000000000; // Knights
        board.pieces[2] = 0x2400000000000000; // Bishops
        board.pieces[3] = 0x8100000000000000; // Rooks
        board.pieces[4] = 0x0800000000000000; // Queen
        board.pieces[5] = 0x1000000000000000; // King
        // Black pieces
        board.pieces[6] = 0x000000000000FF00;
        board.pieces[7] = 0x0000000000000042;
        board.pieces[8] = 0x0000000000000024;
        board.pieces[9] = 0x0000000000000081;
        board.pieces[10] = 0x0000000000000008;
        board.pieces[11] = 0x0000000000000010;
        board
    }

    fn all_pieces(&self) -> u64 {
        self.pieces.iter().fold(0, |acc, &bb| acc | bb)
    }

    fn color_pieces(&self, color: Color) -> u64 {
        let start = match color { Color::White => 0, Color::Black => 6 };
        (start..start+6).fold(0, |acc, i| acc | self.pieces[i])
    }

    fn piece_at(&self, sq: Square) -> Option<(Piece, Color)> {
        let bit = 1u64 << sq;
        for i in 0..12 {
            if self.pieces[i] & bit != 0 {
                let piece = match i % 6 {
                    0 => Piece::Pawn, 1 => Piece::Knight, 2 => Piece::Bishop,
                    3 => Piece::Rook, 4 => Piece::Queen, 5 => Piece::King,
                    _ => unreachable!(),
                };
                let color = if i < 6 { Color::White } else { Color::Black };
                return Some((piece, color));
            }
        }
        None
    }
}
```

### Step 2: Move Generation (Simplified)

```rust
impl Board {
    fn generate_moves(&self) -> Vec<Move> {
        let mut moves = Vec::new();
        let us = self.side_to_move;
        let them = us.other();
        let own = self.color_pieces(us);
        let opp = self.color_pieces(them);
        let all = own | opp;

        let offset: usize = match us { Color::White => 0, Color::Black => 6 };

        // Pawn moves
        let pawns = self.pieces[offset];
        let (push_dir, start_rank, promo_rank): (i8, u64, u64) = match us {
            Color::White => (8, 0x00FF000000000000, 0x000000000000FF00),
            Color::Black => (-8, 0x000000000000FF00, 0x00FF000000000000),
        };

        let mut bits = pawns;
        while bits != 0 {
            let from = bits.trailing_zeros() as Square;
            bits &= bits - 1;
            let to = (from as i8 + push_dir) as Square;
            if to < 64 && all & (1u64 << to) == 0 {
                if (1u64 << to) & promo_rank != 0 {
                    for promo in &[Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
                        moves.push(Move { from, to, promotion: Some(*promo) });
                    }
                } else {
                    moves.push(Move { from, to, promotion: None });
                    // Double push from starting rank
                    let to2 = (from as i8 + 2 * push_dir) as Square;
                    if (1u64 << from) & start_rank != 0 && all & (1u64 << to2) == 0 {
                        moves.push(Move { from, to: to2, promotion: None });
                    }
                }
            }
            // Captures
            for &cap_dir in &[-1i8, 1i8] {
                let cap_to = (from as i8 + push_dir + cap_dir) as Square;
                if cap_to < 64 && (cap_to as i8 % 8).abs() != (from as i8 % 8).abs() + 1 {
                    // Diagonal capture
                    if opp & (1u64 << cap_to) != 0 || Some(cap_to) == self.ep_square {
                        moves.push(Move { from, to: cap_to, promotion: None });
                    }
                }
            }
        }

        // Knight moves
        let knight_offsets: [i8; 8] = [-17, -15, -10, -6, 6, 10, 15, 17];
        let mut knights = self.pieces[offset + 1];
        while knights != 0 {
            let from = knights.trailing_zeros() as Square;
            knights &= knights - 1;
            for &delta in &knight_offsets {
                let to = from as i8 + delta;
                if to >= 0 && to < 64 {
                    let to = to as Square;
                    // Check for wrapping
                    let file_diff = ((from % 8) as i8 - (to % 8) as i8).abs();
                    if file_diff <= 2 && all & (1u64 << to) & own == 0 {
                        moves.push(Move { from, to, promotion: None });
                    }
                }
            }
        }

        // King moves (simplified: no castling for now)
        let king = self.pieces[offset + 5];
        if king != 0 {
            let from = king.trailing_zeros() as Square;
            for &delta in &[-9i8, -8, -7, -1, 1, 7, 8, 9] {
                let to = from as i8 + delta;
                if to >= 0 && to < 64 {
                    let to = to as Square;
                    let file_diff = ((from % 8) as i8 - (to % 8) as i8).abs();
                    if file_diff <= 1 && all & (1u64 << to) & own == 0 {
                        moves.push(Move { from, to, promotion: None });
                    }
                }
            }
        }

        moves
    }
}
```

### Step 3: Alpha-Beta Search

```rust
// Material values (centipawns)
fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 20000,
    }
}

impl Board {
    // Evaluate position from White's perspective
    fn evaluate(&self) -> i32 {
        let mut score = 0;
        for i in 0..12 {
            let color_sign = if i < 6 { 1 } else { -1 };
            let piece = match i % 6 {
                0 => Piece::Pawn, 1 => Piece::Knight, 2 => Piece::Bishop,
                3 => Piece::Rook, 4 => Piece::Queen, 5 => Piece::King,
                _ => unreachable!(),
            };
            let count = self.pieces[i].count_ones() as i32;
            score += color_sign * piece_value(piece) * count;
        }
        score
    }

    // Make a move (simplified: doesn't handle all special cases)
    fn make_move(&self, mv: &Move) -> Board {
        let mut new_board = self.clone();
        let piece_info = self.piece_at(mv.from);
        if let Some((piece, color)) = piece_info {
            let idx = (color as usize * 6) + piece as usize;
            new_board.pieces[idx] &= !(1u64 << mv.from);
            new_board.pieces[idx] |= 1u64 << mv.to;

            // Capture: remove opponent piece at destination
            if let Some((cap_piece, cap_color)) = self.piece_at(mv.to) {
                let cap_idx = (cap_color as usize * 6) + cap_piece as usize;
                new_board.pieces[cap_idx] &= !(1u64 << mv.to);
            }
        }
        new_board.side_to_move = self.side_to_move.other();
        new_board
    }

    // Alpha-beta search
    fn alpha_beta(&self, depth: i32, mut alpha: i32, beta: i32) -> i32 {
        if depth == 0 {
            return self.evaluate();
        }

        let moves = self.generate_moves();
        if moves.is_empty() {
            return self.evaluate(); // Simplified: doesn't detect checkmate
        }

        let maximizing = self.side_to_move == Color::White;

        if maximizing {
            let mut max_eval = i32::MIN;
            for mv in &moves {
                let new_board = self.make_move(mv);
                let eval = new_board.alpha_beta(depth - 1, alpha, beta);
                max_eval = max_eval.max(eval);
                alpha = alpha.max(eval);
                if beta <= alpha { break; } // Beta cutoff
            }
            max_eval
        } else {
            let mut min_eval = i32::MAX;
            for mv in &moves {
                let new_board = self.make_move(mv);
                let eval = new_board.alpha_beta(depth - 1, alpha, beta);
                min_eval = min_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha { break; } // Alpha cutoff
            }
            min_eval
        }
    }

    // Find the best move
    fn best_move(&self, depth: i32) -> Option<Move> {
        let moves = self.generate_moves();
        if moves.is_empty() { return None; }

        let maximizing = self.side_to_move == Color::White;
        let mut best = None;
        let mut best_score = if maximizing { i32::MIN } else { i32::MAX };

        for mv in &moves {
            let new_board = self.make_move(mv);
            let score = new_board.alpha_beta(depth - 1, i32::MIN, i32::MAX);
            if (maximizing && score > best_score) || (!maximizing && score < best_score) {
                best_score = score;
                best = Some(mv.clone());
            }
        }
        best
    }
}

fn main() {
    let board = Board::starting_position();
    println!("Starting position evaluation: {}", board.evaluate());
    println!("Legal moves: {}", board.generate_moves().len());

    // Search to depth 4
    if let Some(mv) = board.best_move(4) {
        println!("Best move: from={} to={}", mv.from, mv.to);
    }
}
```

## Use It

The search/eval architecture generalizes to other adversarial planning domains:

- **Stockfish**: the strongest open-source chess engine. It uses the same alpha-beta framework but with many enhancements: null-move pruning, late move reductions, futility pruning, and a neural network evaluation (NNUE). The move generator uses bitboards like ours.
- **Leela Chess Zero (Lc0)**: uses Monte Carlo Tree Search (MCTS) instead of alpha-beta, with a neural network for both move selection and evaluation. Different search paradigm, same move generation layer.
- **AlphaZero**: demonstrated that a neural network trained by self-play can master chess, Go, and shogi without human knowledge. The search uses MCTS guided by the network's policy and value outputs.

The key production lesson: **move generation correctness is non-negotiable**. Every serious chess engine uses perft testing: count the number of legal moves at each depth for known positions. If your perft counts match the reference, your move generator is correct. If they don't, every search result is suspect.

## Read the Source

- [Chess Programming Wiki](https://www.chessprogramming.org/) — The definitive reference for chess engine programming. Covers bitboards, search algorithms, evaluation, and every known optimization.
- [Stockfish source](https://github.com/official-stockfish/Stockfish) — The strongest open-source engine. `src/movegen.cpp` shows the move generator; `src/search.cpp` shows the alpha-beta framework.
- [Crafty](http://www.craftychess.com/) — Robert Hyatt's educational chess engine. Well-commented code that's easier to read than Stockfish.

## Ship It

- `code/main.rs`: bitboard-based move generator, alpha-beta search, and material evaluation.
- `code/main.cpp`: equivalent C++ implementation with the same algorithms.
- `outputs/README.md`: chess engine milestone checklist covering move generation, search, evaluation, and perft validation.

## Exercises

1. **Easy** — Add a transposition table. Hash each position using Zobrist hashing (a 64-bit XOR of random values for each piece-square pair). Store positions and their scores in a hash table. On re-encounter, return the cached score instead of re-searching.
2. **Medium** — Implement iterative deepening. Instead of searching directly to depth N, search to depth 1, then depth 2, up to depth N. Use the results from shallower searches to improve move ordering for deeper searches (the best move from depth D is searched first at depth D+1).
3. **Hard** — Add move ordering heuristics. Search captures before quiet moves (MVV-LVA: Most Valuable Victim, Least Valuable Attacker). Search the previous best move (from the transposition table) first. Search killer moves (moves that caused beta cutoffs at the same depth) before other quiet moves. Measure the node count reduction.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Alpha-beta | "pruned minimax" | A branch-and-bound optimization for minimax search. Maintains two bounds (alpha, beta) and skips branches that provably can't improve the result. In the best case, doubles the searchable depth. |
| Evaluation function | "board score" | A heuristic that assigns a numeric score to a leaf position. Material counting is the simplest; production engines add positional factors (piece-square tables, pawn structure, king safety). |
| Move ordering | "search speed trick" | The order in which moves are searched. Good move ordering (captures first, then killer moves, then quiet moves) makes alpha-beta pruning more effective, dramatically reducing the search tree. |
| Horizon effect | "lookahead blind spot" | When the search depth is insufficient to see a tactical sequence to its conclusion. The engine may make a bad move because the consequence is just beyond its search depth. Quiescence search mitigates this. |
| Perft | "move count test" | A debugging function that counts the total number of legal moves at each depth from a given position. Comparing perft counts against known values verifies move generator correctness. |

## Further Reading

- [Chess Programming Wiki](https://www.chessprogramming.org/) — Comprehensive reference for all aspects of chess engine programming.
- [Stockfish](https://github.com/official-stockfish/Stockfish) — The strongest open-source chess engine.
- [How to Build a Chess Engine](https://github.com/maksimKorzh/chess_programming) — Step-by-step tutorials building a chess engine from scratch.

// Build a Chess Engine with Search & Eval
// Run: rustc main.rs && ./main
//
// Architecture:
//   Board (bitboards) → Move Generator → Alpha-Beta Search → Evaluation
//
// Implements bitboard-based board representation, move generation for
// pawns/knights/king, material evaluation, and alpha-beta search.

// =============================================================================
// Step 1: Board Representation (Bitboards)
// =============================================================================

type Square = u8; // 0-63, a1=0, h8=63

#[derive(Debug, Clone, Copy, PartialEq)]
enum Piece { Pawn, Knight, Bishop, Rook, Queen, King }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Color { White, Black }

impl Color {
    fn other(self) -> Color {
        match self { Color::White => Color::Black, Color::Black => Color::White }
    }
}

#[derive(Debug, Clone, Copy)]
struct Move {
    from: Square,
    to: Square,
    promotion: Option<Piece>,
}

#[derive(Clone)]
struct Board {
    pieces: [u64; 12], // [0..5] white, [6..11] black
    side_to_move: Color,
    castling: u8,
    ep_square: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
}

impl Board {
    fn starting_position() -> Self {
        let mut board = Board {
            pieces: [0u64; 12],
            side_to_move: Color::White,
            castling: 0b1111,
            ep_square: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        };
        board.pieces[0] = 0x00FF000000000000; // White pawns
        board.pieces[1] = 0x4200000000000000; // White knights
        board.pieces[2] = 0x2400000000000000; // White bishops
        board.pieces[3] = 0x8100000000000000; // White rooks
        board.pieces[4] = 0x0800000000000000; // White queen
        board.pieces[5] = 0x1000000000000000; // White king
        board.pieces[6] = 0x000000000000FF00; // Black pawns
        board.pieces[7] = 0x0000000000000042; // Black knights
        board.pieces[8] = 0x0000000000000024; // Black bishops
        board.pieces[9] = 0x0000000000000081; // Black rooks
        board.pieces[10] = 0x0000000000000008; // Black queen
        board.pieces[11] = 0x0000000000000010; // Black king
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

// =============================================================================
// Step 2: Move Generation
// =============================================================================

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
                    let to2 = (from as i8 + 2 * push_dir) as Square;
                    if (1u64 << from) & start_rank != 0 && all & (1u64 << to2) == 0 {
                        moves.push(Move { from, to: to2, promotion: None });
                    }
                }
            }
            for &cap_dir in &[-1i8, 1i8] {
                let cap_to = (from as i8 + push_dir + cap_dir) as Square;
                if cap_to < 64 && ((cap_to % 8) as i8 - (from % 8) as i8).abs() == 1 {
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
                    let file_diff = ((from % 8) as i8 - (to % 8) as i8).abs();
                    if file_diff <= 2 && all & (1u64 << to) & own == 0 {
                        moves.push(Move { from, to, promotion: None });
                    }
                }
            }
        }

        // King moves
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

// =============================================================================
// Step 3: Evaluation and Alpha-Beta Search
// =============================================================================

fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100, Piece::Knight => 320, Piece::Bishop => 330,
        Piece::Rook => 500, Piece::Queen => 900, Piece::King => 20000,
    }
}

impl Board {
    fn evaluate(&self) -> i32 {
        let mut score = 0;
        for i in 0..12 {
            let color_sign = if i < 6 { 1 } else { -1 };
            let piece = match i % 6 {
                0 => Piece::Pawn, 1 => Piece::Knight, 2 => Piece::Bishop,
                3 => Piece::Rook, 4 => Piece::Queen, 5 => Piece::King,
                _ => unreachable!(),
            };
            score += color_sign * piece_value(piece) * self.pieces[i].count_ones() as i32;
        }
        score
    }

    fn make_move(&self, mv: &Move) -> Board {
        let mut new_board = self.clone();
        if let Some((piece, color)) = self.piece_at(mv.from) {
            let idx = (color as usize * 6) + piece as usize;
            new_board.pieces[idx] &= !(1u64 << mv.from);
            new_board.pieces[idx] |= 1u64 << mv.to;
            if let Some((cap_piece, cap_color)) = self.piece_at(mv.to) {
                let cap_idx = (cap_color as usize * 6) + cap_piece as usize;
                new_board.pieces[cap_idx] &= !(1u64 << mv.to);
            }
        }
        new_board.side_to_move = self.side_to_move.other();
        new_board
    }

    fn alpha_beta(&self, depth: i32, mut alpha: i32, beta: i32) -> i32 {
        if depth == 0 { return self.evaluate(); }
        let moves = self.generate_moves();
        if moves.is_empty() { return self.evaluate(); }

        if self.side_to_move == Color::White {
            let mut max_eval = i32::MIN;
            for mv in &moves {
                let eval = self.make_move(mv).alpha_beta(depth - 1, alpha, beta);
                max_eval = max_eval.max(eval);
                alpha = alpha.max(eval);
                if beta <= alpha { break; }
            }
            max_eval
        } else {
            let mut min_eval = i32::MAX;
            for mv in &moves {
                let eval = self.make_move(mv).alpha_beta(depth - 1, alpha, beta);
                min_eval = min_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha { break; }
            }
            min_eval
        }
    }

    fn best_move(&self, depth: i32) -> Option<Move> {
        let moves = self.generate_moves();
        if moves.is_empty() { return None; }
        let maximizing = self.side_to_move == Color::White;
        let mut best = None;
        let mut best_score = if maximizing { i32::MIN } else { i32::MAX };
        for mv in &moves {
            let score = self.make_move(mv).alpha_beta(depth - 1, i32::MIN, i32::MAX);
            if (maximizing && score > best_score) || (!maximizing && score < best_score) {
                best_score = score;
                best = Some(*mv);
            }
        }
        best
    }
}

fn main() {
    let board = Board::starting_position();
    println!("Starting position evaluation: {}", board.evaluate());
    println!("Legal moves: {}", board.generate_moves().len());

    if let Some(mv) = board.best_move(4) {
        println!("Best move: from={} to={}", mv.from, mv.to);
    }
}

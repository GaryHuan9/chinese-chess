use crate::board::Board;
use crate::location::Move;
use crate::piece::{Piece, PieceKind};
use std::fmt::Formatter;

pub struct Game {
    board: Board,
    red_turn: bool,
    history: Vec<(Move, Option<Piece>)>,
    moves: Vec<Move>,
}

#[derive(Debug)]
pub enum Outcome {
    RedWon,
    BlackWon,
    Stalemate, // draw from no legal move and no check
    MoveRule,  // draw from the 50-move rule
}

impl Game {
    pub fn new(board: Board, red_turn: bool) -> Self {
        let moves = board.iter_legal_moves(red_turn).collect();
        Self {
            board,
            red_turn,
            history: Vec::new(),
            moves,
        }
    }

    pub fn opening() -> Self {
        Self::new(Board::opening(), true)
    }

    pub fn from_fen(fen: &str, red_turn: bool) -> Option<Self> {
        Some(Self::new(Board::from_fen(fen)?, red_turn))
    }

    pub fn fen(&self) -> (String, bool) {
        (self.board.fen(), self.red_turn)
    }

    pub fn play(&mut self, mv: Move) -> bool {
        if self.outcome().is_some() || !self.moves.contains(&mv) {
            return false;
        }

        let (piece, capture) = self.board.play(mv);
        assert_eq!(self.red_turn, piece.is_red());

        if let Some(capture) = capture {
            assert_ne!(self.red_turn, capture.is_red());
        }

        self.red_turn = !self.red_turn;
        self.history.push((mv, capture));

        self.moves = self.board.iter_legal_moves(self.red_turn).collect();
        true
    }

    pub fn undo(&mut self) -> Option<Move> {
        let (mv, capture) = self.history.pop()?;
        self.red_turn = !self.red_turn;
        self.board.undo(mv, capture);

        self.moves = self.board.iter_legal_moves(self.red_turn).collect();

        Some(mv)
    }

    pub fn moves(&self) -> &Vec<Move> {
        &self.moves
    }

    pub fn moves_ranked(&self) -> Vec<(Move, i32)> {
        let mut board = self.board.clone();

        // fn search(board: &mut Board, red: bool, depth: i32) -> i32 {
        //     board.iter_legal_moves(red)
        //         .map(|mv| {
        //             let (_, capture) = board.play(mv);
        //             let value = board.evaluate(self.red_turn);
        //             board.undo(mv, capture);
        //             (mv, value)
        //         }).fold(i32::MIN, )
        //
        // }

        self.moves
            .iter()
            .map(|&mv| {
                let (_, capture) = board.play(mv);
                let value = board.evaluate(self.red_turn);
                board.undo(mv, capture);
                (mv, value)
            })
            .collect()
    }

    pub fn outcome(&self) -> Option<Outcome> {
        if self.move_rule() {
            return Some(Outcome::MoveRule);
        }

        if !self.moves.is_empty() {
            return None;
        }

        let king = self.board.find_king(self.red_turn).unwrap();
        let check = self.board.iter_legal_moves(!self.red_turn).any(|mv| mv.to == king);

        match (check, self.red_turn) {
            (false, _) => Some(Outcome::Stalemate),
            (true, true) => Some(Outcome::BlackWon),
            (true, false) => Some(Outcome::RedWon),
        }
    }

    pub fn move_rule(&self) -> bool {
        const LENGTH: usize = 50;
        if self.history.len() < LENGTH {
            return false;
        }

        self.history.iter().rev().take(LENGTH).all(|(mv, capture)| {
            // no capture made or no pawn movement
            capture.is_none() && self.board[mv.to].map(|p| p.kind() != PieceKind::Pawn).unwrap_or(true)
        })
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.board.fen())?;
        write!(f, "{}", self.board)?;

        let captured = self.history.iter();
        let captured = captured
            .filter_map(|&(_, capture)| capture)
            .map(|piece| piece.to_string())
            .rev()
            .collect::<Vec<_>>();

        if !captured.is_empty() {
            writeln!(f, "captured - {}", captured.join(" "))?;
        }

        if let Some((mv, _)) = self.history.last() {
            let piece = self.board[mv.to].unwrap();
            write!(f, "{} {piece} - ", mv)?;
        }

        let red = if self.red_turn { "red" } else { "black" };
        let king = Piece::from_kind(PieceKind::King, self.red_turn);
        writeln!(f, "{} {} to play - {} moves available", red, king, self.moves.len())
    }
}

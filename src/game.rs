use crate::board::Board;
use crate::location::Move;
use crate::piece::{Piece, PieceKind};
use std::fmt::Formatter;

pub struct Game {
    board: Board,
    red_turn: bool,
    history: Vec<(Move, Option<Piece>)>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            red_turn: true,
            history: Vec::new(),
        }
    }

    pub fn opening() -> Self {
        Self {
            board: Board::opening(),
            red_turn: true,
            history: Vec::new(),
        }
    }

    pub fn from_fen(fen: &str, red_turn: bool) -> Option<Self> {
        Some(Self {
            board: Board::from_fen(fen)?,
            red_turn,
            history: Vec::new(),
        })
    }

    pub fn fen(&self) -> String {
        self.board.fen()
    }

    pub fn play(&mut self, mv: Move) {
        let (piece, capture) = self.board.play(mv);
        assert_eq!(self.red_turn, piece.is_red());

        if let Some(capture) = capture {
            assert_ne!(self.red_turn, capture.is_red());
        }

        self.red_turn = !self.red_turn;
        self.history.push((mv, capture));
    }

    pub fn undo(&mut self) -> Option<Move> {
        let (mv, capture) = self.history.pop()?;
        self.red_turn = !self.red_turn;
        self.board.undo(mv, capture);

        Some(mv)
    }

    pub fn moves(&self) -> Vec<Move> {
        self.board.iter_legal_moves(self.red_turn).collect()
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
        writeln!(f, "{} {} to play", red, king)
    }
}

use crate::board::Board;
use crate::location::Move;
use crate::piece::{Piece, PieceKind};
use std::fmt::Formatter;
use std::str::Chars;

pub struct Game {
    board: Board,
    red_turn: bool,
    moves: Vec<(Move, Option<Piece>)>,
}

impl Game {
    pub fn new() -> Self {
        Self { board: Board::new(), red_turn: true, moves: Vec::new() }
    }

    pub fn opening() -> Self {
        Self { board: Board::opening(), red_turn: true, moves: Vec::new() }
    }

    pub fn from_fen(fen: &mut Chars<'_>) -> Option<Self> {
        let board = Board::from_fen(fen)?;

        let red_turn = match fen.next()? {
            'w' => true,
            'b' => false,
            _ => return None,
        };

        Some(Self { board, red_turn, moves: Vec::new() })
    }

    pub fn play(&mut self, mv: Move) {
        let piece = self.board[mv.from].unwrap();
        let capture = self.board[mv.to];

        assert_eq!(matches!(piece, Piece::Red(_)), self.red_turn);
        assert_eq!(matches!(piece, Piece::Black(_)), !self.red_turn);

        if let Some(capture) = capture {
            assert_eq!(matches!(capture, Piece::Black(_)), self.red_turn);
            assert_eq!(matches!(capture, Piece::Red(_)), !self.red_turn);
        }

        self.board[mv.from] = None;
        self.board[mv.to] = Some(piece);
        self.red_turn = !self.red_turn;
        self.moves.push((mv, capture));
    }

    pub fn undo(&mut self) -> Option<Move> {
        let (mv, capture) = self.moves.pop()?;
        let piece = self.board[mv.to].unwrap();
        self.red_turn = !self.red_turn;

        assert!(self.board[mv.from].is_none());
        self.board[mv.from] = Some(piece);
        self.board[mv.to] = capture;

        Some(mv)
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.board)?;

        let mut captured = self.moves.iter().filter_map(|&(_, capture)| capture).peekable();
        if captured.peek().is_some() {
            write!(f, "captured: ")?;
            for piece in captured {
                write!(f, "{} ", piece)?;
            }
            writeln!(f)?;
        }

        if let Some((mv, _)) = self.moves.last() {
            let piece = self.board[mv.to].unwrap();
            writeln!(f, "moved: {piece} {}{}", mv.from, mv.to)?;
        }

        let red = if self.red_turn { "red" } else { "black" };
        let king = Piece::from_kind(PieceKind::King, self.red_turn);
        writeln!(f, "{} {} to move", red, king)
    }
}

use crate::board::Board;
use crate::display_format::DisplayFormat;
use crate::location::{Location, Move};
use crate::piece::{Piece, PieceKind};
use std::fmt::{Display, Formatter};

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

impl Outcome {
    pub fn display(&self, format: DisplayFormat) -> impl Display {
        let king = |red| Piece::from_kind(PieceKind::King, red);
        let format = format.with_concise(false);
        match self {
            Self::RedWon => format!("{} won by checkmating black", king(true).display(format)),
            Self::BlackWon => format!("{} won by checkmating red", king(false).display(format)),
            Self::Stalemate => "draw by stalemate".to_owned(),
            Self::MoveRule => "draw by 50-move rule".to_owned(),
        }
    }
}

impl Display for Outcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(DisplayFormat::string()))
    }
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

    pub fn king_in_check(&self, red: bool) -> bool {
        let king = self.board.find_king(red).unwrap();
        self.board.iter_legal_moves(!red).any(|mv| mv.to == king)
    }

    pub fn outcome(&self) -> Option<Outcome> {
        if self.move_rule() {
            return Some(Outcome::MoveRule);
        }

        if !self.moves.is_empty() {
            return None;
        }

        match (self.king_in_check(self.red_turn), self.red_turn) {
            (false, _) => Some(Outcome::Stalemate),
            (true, true) => Some(Outcome::BlackWon),
            (true, false) => Some(Outcome::RedWon),
        }
    }

    pub fn move_rule(&self) -> bool {
        const LENGTH: usize = 100;
        if self.history.len() < LENGTH {
            return false;
        }

        self.history.iter().rev().take(LENGTH).all(|(mv, capture)| {
            // no capture made or no pawn movement
            capture.is_none() && self.board[mv.to].map(|p| p.kind() != PieceKind::Pawn).unwrap_or(true)
        })
    }

    pub fn display(&self, format: DisplayFormat) -> impl Display {
        struct Impl<'a>(&'a Game, DisplayFormat);
        return Impl(self, format);

        impl Impl<'_> {
            fn format_row(&self, f: &mut Formatter<'_>, y: i8) -> std::fmt::Result {
                let &Self(game, format) = self;
                write!(f, "{y}")?;

                if let Some(mv) = game.history.last().map(|&(mv, _)| mv) {
                    for x in 0..Board::WIDTH {
                        let location = Location::from_xy(x, y).unwrap();
                        if let Some(piece) = game.board[location] {
                            let piece = piece.display(format.with_concise(true));
                            if format.effects && mv.to == location {
                                write!(f, " \x1B[3m{piece}\x1B[0m")?;
                            } else {
                                write!(f, " {piece}")?;
                            }
                        } else if mv.from == location {
                            write!(f, " ╶╴")?;
                        } else {
                            write!(f, "   ")?;
                        }
                    }
                } else {
                    for x in 0..Board::WIDTH {
                        let location = Location::from_xy(x, y).unwrap();
                        if let Some(piece) = game.board[location] {
                            write!(f, " {}", piece.display(format.with_concise(true)))?;
                        } else {
                            write!(f, "   ")?;
                        }
                    }
                }

                Ok(())
            }

            fn format_captured(&self, f: &mut Formatter<'_>, row: usize) -> std::fmt::Result {
                let &Self(game, format) = self;
                let captured = game.history.iter().filter_map(|&(_, capture)| capture);
                if captured.clone().next().is_none() {
                    return Ok(());
                }

                const HEIGHT: usize = Board::HEIGHT as usize + 1;
                write!(f, " │   ")?;

                let red = captured.clone().filter(|piece| piece.is_red());
                let red_count = red.clone().count();
                let pad = red_count.div_ceil(HEIGHT) * HEIGHT - red_count;
                let row = red
                    .map(Some)
                    .chain(std::iter::repeat_n(None, pad))
                    .chain(captured.filter(|piece| !piece.is_red()).map(Some))
                    .enumerate()
                    .filter_map(|(i, piece)| if i % HEIGHT == row { Some(piece) } else { None });

                for piece in row {
                    if let Some(piece) = piece {
                        write!(f, "{} ", piece.display(format.with_concise(true)))?;
                    } else {
                        write!(f, "   ")?;
                    }
                }

                Ok(())
            }
        }

        impl Display for Impl<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let &Self(game, format) = self;
                write!(f, "{}", game.board.fen())?;

                if format.concise {
                    return write!(f, " {}", if game.red_turn { "red" } else { "black" });
                }

                writeln!(f)?;

                for y in (0..Board::HEIGHT).rev() {
                    self.format_row(f, y)?;
                    self.format_captured(f, (Board::HEIGHT - y - 1) as usize)?;
                    writeln!(f)?;
                }

                for char in 'A'..='I' {
                    write!(f, "  {char}")?;
                }
                write!(f, " ")?;
                self.format_captured(f, Board::HEIGHT as usize)?;
                writeln!(f)?;

                if let Some(mv) = game.history.last().map(|&(mv, _)| mv) {
                    let piece = game.board[mv.to].unwrap().display(format.with_concise(true));
                    write!(f, "({}) {} {piece} - ", game.history.len(), mv)?;
                }

                if let Some(outcome) = game.outcome() {
                    write!(f, "{}", outcome.display(format))?;
                } else {
                    let check = game.king_in_check(game.red_turn);
                    let king = Piece::from_kind(PieceKind::King, game.red_turn).display(format);
                    write!(f, "{king} {} - ", if check { "in check" } else { "to play" })?;

                    write!(f, "{} legal moves", game.moves().len())?;
                }

                writeln!(f)
            }
        }
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(DisplayFormat::string()))
    }
}

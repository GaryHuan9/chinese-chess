use crate::location::{Location, Move};
use crate::piece::{Piece, PieceKind};
use std::fmt::Formatter;
use std::ops::{Index, IndexMut};
use std::str::Chars;

pub struct Board {
    pieces: Vec<Option<Piece>>,
}

impl Board {
    pub const WIDTH: i8 = 9;
    pub const HEIGHT: i8 = 10;

    pub fn new() -> Self {
        Self { pieces: vec![None; (Self::WIDTH * Self::HEIGHT) as usize] }
    }

    pub fn from_fen(fen: &mut Chars<'_>) -> Option<Self> {
        let mut board = Self::new();
        let mut y = Location::new().shift_y(Self::HEIGHT - 1).unwrap();
        let mut x = 0;

        for current in fen {
            match current {
                ' ' => break,
                '/' => {
                    if x != Self::WIDTH {
                        return None;
                    }
                    x = 0;
                    y = y.shift_y(-1)?;
                }
                '0'..='9' => x += current.to_digit(10).unwrap() as i8,
                _ => {
                    let piece = Piece::from_char(current)?;
                    board[y.shift_x(x).unwrap()] = Some(piece);
                    x += 1;
                }
            }
        }

        Option::from(board)
    }

    pub fn opening() -> Self {
        Self::from_fen(&mut "rheakaehr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RHEAKAEHR".chars()).unwrap()
    }

    pub fn is_occupied(&self, location: Location) -> bool {
        self[location].is_some()
    }

    pub fn fill_moves(&self, moves: &mut Vec<Move>, red: bool) {
        for y in 0..Self::HEIGHT {
            for x in 0..Self::WIDTH {
                let from = Location::from_xy(x, y).unwrap();
                let Some(piece) = self[from] else { continue };
                if piece.is_red() != red {
                    continue;
                }

                let mut add = |to: Option<Location>| {
                    let Some(to) = to else { return false };
                    if self[to].is_some_and(|piece| piece.is_red() == red) {
                        return false;
                    }
                    moves.push(Move { from, to });
                    true
                };

                match piece.kind() {
                    PieceKind::King => {
                        let mut add = |to: Option<Location>| {
                            let Some(to) = to else { return };
                            if (to.x() - Self::WIDTH / 2).abs() <= 1 && to.normalize(red).y() < 3 {
                                add(Some(to));
                            }
                        };
                        add(from.shift_x(1));
                        add(from.shift_x(-1));
                        add(from.shift_y(1));
                        add(from.shift_y(-1));
                    }
                    PieceKind::Advisor => {
                        let x = from.x() - Self::WIDTH / 2;
                        if x == 0 {
                            add(from.shift_xy(1, 1));
                            add(from.shift_xy(-1, 1));
                            add(from.shift_xy(1, -1));
                            add(from.shift_xy(-1, -1));
                        } else {
                            let from = from.normalize(red);
                            let y = from.y() - 1;
                            add(Some(from.shift_xy(-x, -y).unwrap().normalize(red)));
                        }
                    }
                    PieceKind::Elephant => {
                        let mut add = |x, y| {
                            let Some(block) = from.shift_xy(x, y) else { return };
                            if self.is_occupied(block) {
                                return;
                            };

                            let Some(to) = from.shift_xy(x * 2, y * 2) else { return };
                            if to.normalize(red).y() >= Self::HEIGHT / 2 {
                                return;
                            };

                            add(Some(to));
                        };
                        add(1, 1);
                        add(-1, 1);
                        add(1, -1);
                        add(-1, -1);
                    }
                    PieceKind::Horse => {
                        let mut add =
                            |shift_major: fn(Location) -> Option<Location>,
                             shift_minor: fn(Location, i8) -> Option<Location>| {
                                let Some(block) = shift_major(from) else { return };
                                if self.is_occupied(block) {
                                    return;
                                }

                                let Some(to) = shift_major(block) else { return };
                                add(shift_minor(to, 1));
                                add(shift_minor(to, -1));
                            };
                        add(|to| to.shift_x(1), |to, value| to.shift_y(value));
                        add(|to| to.shift_x(-1), |to, value| to.shift_y(value));
                        add(|to| to.shift_y(1), |to, value| to.shift_x(value));
                        add(|to| to.shift_y(-1), |to, value| to.shift_x(value));
                    }
                    PieceKind::Chariot => {
                        let mut add = |shift: fn(Location) -> Option<Location>| {
                            let mut current = shift(from);
                            while add(current) {
                                current = shift(current.unwrap());
                            }
                        };
                        add(|to| to.shift_x(1));
                        add(|to| to.shift_x(-1));
                        add(|to| to.shift_y(1));
                        add(|to| to.shift_y(-1));
                    }
                    PieceKind::Cannon => {
                        let mut add = |shift: fn(Location) -> Option<Location>| {
                            let mut current = shift(from);
                            loop {
                                let Some(to) = current else { return };
                                if self.is_occupied(to) {
                                    break;
                                }
                                moves.push(Move { from, to });
                                current = shift(to);
                            }

                            current = shift(current.unwrap());
                            loop {
                                let Some(to) = current else { return };

                                if let Some(piece) = self[to]
                                    && piece.is_red() != red
                                {
                                    moves.push(Move { from, to });
                                    break;
                                }

                                current = shift(to);
                            }
                        };

                        add(|to| to.shift_x(1));
                        add(|to| to.shift_x(-1));
                        add(|to| to.shift_y(1));
                        add(|to| to.shift_y(-1));
                    }
                    PieceKind::Pawn => {
                        let from = from.normalize(red);
                        let mut add = |to: Option<Location>| add(to.map(|to| to.normalize(red)));

                        add(from.shift_y(1));

                        if from.y() >= Self::HEIGHT / 2 {
                            add(from.shift_x(1));
                            add(from.shift_x(-1));
                        }
                    }
                }
            }
        }
    }
}

impl Index<Location> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Location) -> &Self::Output {
        &self.pieces[index.index()]
    }
}

impl IndexMut<Location> for Board {
    fn index_mut(&mut self, index: Location) -> &mut Self::Output {
        &mut self.pieces[index.index()]
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for y in (0..Self::HEIGHT).rev() {
            write!(f, "{y} ")?;
            for x in 0..Self::WIDTH {
                if let Some(piece) = self[Location::from_xy(x, y).unwrap()] {
                    write!(f, "{} ", piece)?;
                } else {
                    write!(f, "   ")?;
                }
            }
            writeln!(f)?;
        }
        for char in 'A'..='I' {
            write!(f, "  {char}")?;
        }
        writeln!(f)
    }
}

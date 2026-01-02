use crate::location::Location;
use crate::piece::Piece;
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

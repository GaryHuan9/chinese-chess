use crate::board::Board;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Location {
    x: i8,
    y: i8,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Move {
    pub from: Location,
    pub to: Location,
}

impl Location {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn from_xy(x: i8, y: i8) -> Option<Self> {
        Self::new().shift_xy(x, y)
    }

    pub fn from_index(index: usize) -> Option<Self> {
        if index > i8::MAX as usize {
            return None;
        }
        let x = index as i8 % Board::WIDTH;
        let y = index as i8 / Board::WIDTH;
        Self::from_xy(x, y)
    }

    pub fn shift_x(&self, y: i8) -> Option<Self> {
        let new_x = self.x + y;
        if 0 > new_x || new_x >= Board::WIDTH {
            return None;
        }
        Some(Self { x: new_x, y: self.y })
    }

    pub fn shift_y(&self, x: i8) -> Option<Self> {
        let new_y = self.y + x;
        if 0 > new_y || new_y >= Board::HEIGHT {
            return None;
        }
        Some(Self { x: self.x, y: new_y })
    }

    pub fn shift_xy(&self, x: i8, y: i8) -> Option<Self> {
        self.shift_x(x)?.shift_y(y)
    }

    pub fn index(&self) -> usize {
        (self.x + self.y * Board::WIDTH) as usize
    }

    pub fn x(&self) -> i8 {
        self.x
    }

    pub fn y(&self) -> i8 {
        self.y
    }

    pub fn normalize(&self, red: bool) -> Self {
        if red {
            *self
        } else {
            Self {
                x: self.x,
                y: Board::HEIGHT - self.y - 1,
            }
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", (b'a' + self.x as u8) as char, self.y)
    }
}

impl FromStr for Location {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let (x, y) = chars.next().zip(chars.next()).ok_or(ParseError)?;

        Location::from_xy(
            (x.to_ascii_lowercase() as u8).wrapping_sub(b'a') as i8,
            (y as u8).wrapping_sub(b'0') as i8,
        )
        .ok_or(ParseError)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from, self.to)
    }
}

impl FromStr for Move {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (from, to) = s.split_at_checked(2).ok_or(ParseError)?;
        Ok(Self {
            from: from.parse::<Location>()?,
            to: to.parse::<Location>()?,
        })
    }
}

#[derive(Debug)]
pub struct ParseError;

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "location parse error")
    }
}

impl std::error::Error for ParseError {}

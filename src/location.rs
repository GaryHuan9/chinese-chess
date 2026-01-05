use crate::board::Board;
use std::fmt::Formatter;
use std::str::Chars;

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

    pub fn from_chars(chars: &mut Chars<'_>) -> Option<Self> {
        let x = chars.next()?.to_ascii_uppercase() as u8;
        let y = chars.next()? as u8;
        Self::from_xy(x.wrapping_sub(b'A') as i8, y.wrapping_sub(b'0') as i8)
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
impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", (b'a' + self.x as u8) as char, self.y)
    }
}

use crate::piece::Piece;

pub struct Board {
    pieces: Vec<Piece>,
    captured: Vec<Piece>,
    white_turn: bool,
}

impl Board {
    pub const WIDTH: i8 = 9;
    pub const HEIGHT: i8 = 10;

    pub const STARTING_FEN: &str =
        "rheakaehr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RHEAKAEHR w - - 0 1";

    fn new() -> Self {
        Self {
            pieces: vec![Piece::None; (Self::WIDTH * Self::HEIGHT) as usize],
            captured: Vec::new(),
            white_turn: true,
        }
    }

    fn from_fen(fen: &str) -> Option<Self> {
        let mut location = Location::new().shift_y(Self::HEIGHT - 1)?;
        let mut board = Self::new();
        let mut chars = fen.chars();

        for current in &mut chars {
            match current {
                ' ' => break,
                '/' => location = location.shift_y(-1)?,
                _ => {
                    let shift = if let Some(piece) = Piece::from_char(current) {
                        board.pieces[location.index()] = piece;
                        1
                    } else if let Some(digit) = current.to_digit(10) {
                        digit
                    } else {
                        return None;
                    };
                }
            }
        }

        for current in &mut chars {}

        Option::from(board)
    }
}

#[derive(Copy, Clone)]
pub struct Location {
    x: i8,
    y: i8,
}

impl Location {
    pub fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        if index > i8::MAX as usize {
            return None;
        }
        let x = index as i8 % Board::WIDTH;
        let y = index as i8 / Board::WIDTH;
        Self::new().shift_x(x)?.shift_y(y)
    }

    pub fn shift_x(&self, value: i8) -> Option<Self> {
        let new_x = self.x + value;
        if 0 > new_x || new_x >= Board::WIDTH {
            return None;
        }
        Some(Self {
            x: new_x,
            y: self.y,
        })
    }

    pub fn shift_y(&self, value: i8) -> Option<Self> {
        let new_y = self.y + value;
        if 0 > new_y || new_y >= Board::HEIGHT {
            return None;
        }
        Some(Self {
            x: self.x,
            y: new_y,
        })
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
}

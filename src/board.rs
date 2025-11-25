use crate::piece::Piece;

pub struct Board {
    pieces: Vec<Piece>,
    captured: Vec<Piece>,
    white_turn: bool,
}

impl Board {
    pub const WIDTH: u8 = 9;
    pub const HEIGHT: u8 = 10;

    fn new() -> Self {
        Self::from_fen("rheakaehr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RHEAKAEHR w - - 0 1")
    }

    fn from_fen(fen: &str) -> Self {
        let mut board = Self {
            pieces: vec![Piece::None; (Self::WIDTH * Self::HEIGHT) as usize],
            captured: Vec::new(),
            white_turn: true,
        };

        let mut location = Location::new(0, Self::HEIGHT - 1);

        for current in fen.chars()
        {
            match current {
                ' '=> break,
                '/'=>location.shift_y();
                _=> {
                    
                }
                
            }

        }

        board
    }
}

#[derive(Copy, Clone)]
pub struct Location {
    x: u8,
    y: u8,
}

impl Location {
    pub fn new(x: u8, y: u8) -> Self {
        debug_assert!(x < Board::WIDTH);
        debug_assert!(y < Board::HEIGHT);
        Self { x, y }
    }

    pub fn from_index(index: usize) -> Self {
        Self::new(index as u8 % Board::WIDTH, index as u8 / Board::WIDTH)
    }

    pub fn index(self) -> usize {
        (self.x + self.y * Board::WIDTH) as usize
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }
}

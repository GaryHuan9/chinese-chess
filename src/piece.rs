use std::fmt::{Display, Formatter};
use std::num::NonZeroI8;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(i8)]
pub enum PieceKind {
    King,
    Advisor,
    Elephant,
    Horse,
    Chariot,
    Cannon,
    Pawn,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Piece {
    data: NonZeroI8,
}

impl Piece {
    pub fn from_fen_char(value: char) -> Option<Self> {
        let kind = match value.to_ascii_lowercase() {
            'k' => PieceKind::King,
            'a' => PieceKind::Advisor,
            'e' => PieceKind::Elephant,
            'h' => PieceKind::Horse,
            'r' => PieceKind::Chariot,
            'c' => PieceKind::Cannon,
            'p' => PieceKind::Pawn,
            _ => return None,
        };

        let red = value.is_ascii_uppercase();
        Some(Self::from_kind(kind, red))
    }

    pub fn from_kind(kind: PieceKind, red: bool) -> Self {
        let data = NonZeroI8::new(kind as i8 + 1).unwrap();
        let data = if red { data } else { -data };
        Self { data }
    }

    pub fn is_red(&self) -> bool {
        self.data.is_positive()
    }

    pub fn kind(&self) -> PieceKind {
        let data = self.data.abs().get() - 1;
        unsafe { std::mem::transmute(data) }
    }

    pub fn fen_char(&self) -> char {
        let result = match self.kind() {
            PieceKind::King => 'k',
            PieceKind::Advisor => 'a',
            PieceKind::Elephant => 'e',
            PieceKind::Horse => 'h',
            PieceKind::Chariot => 'r',
            PieceKind::Cannon => 'c',
            PieceKind::Pawn => 'p',
        };
        if self.is_red() {
            result.to_ascii_uppercase()
        } else {
            result
        }
    }

    pub fn chinese_char(&self) -> char {
        match (self.is_red(), self.kind()) {
            (true, PieceKind::King) => '帥',
            (true, PieceKind::Advisor) => '仕',
            (true, PieceKind::Elephant) => '相',
            (true, PieceKind::Horse) => '傌',
            (true, PieceKind::Chariot) => '俥',
            (true, PieceKind::Cannon) => '炮',
            (true, PieceKind::Pawn) => '兵',
            (false, PieceKind::King) => '將',
            (false, PieceKind::Advisor) => '士',
            (false, PieceKind::Elephant) => '象',
            (false, PieceKind::Horse) => '馬',
            (false, PieceKind::Chariot) => '車',
            (false, PieceKind::Cannon) => '砲',
            (false, PieceKind::Pawn) => '卒',
        }
    }

    pub fn base_value(&self, red: bool) -> i32 {
        let value = match self.kind() {
            PieceKind::King => 1000000,
            PieceKind::Advisor => 2000,
            PieceKind::Elephant => 2000,
            PieceKind::Horse => 4000,
            PieceKind::Chariot => 9000,
            PieceKind::Cannon => 4500,
            PieceKind::Pawn => 1500,
        };
        if self.is_red() == red { value } else { -value }
    }

    pub fn display(&self, chinese: bool) -> impl Display {
        let s = if chinese {
            self.chinese_char().to_string()
        } else {
            let c = self.fen_char();
            format!("{c}{c}")
        };
        if self.is_red() {
            format!("\x1B[31m{}\x1b[0m", s)
        } else {
            s
        }
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(true))
    }
}

use std::fmt::Formatter;
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
    pub fn from_char(value: char) -> Option<Self> {
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
        Self { data: if red { data } else { -data } }
    }

    pub fn is_red(&self) -> bool {
        self.data.is_positive()
    }

    pub fn kind(&self) -> PieceKind {
        let data = self.data.abs().get() - 1;
        unsafe { std::mem::transmute(data) }
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let char = match (self.is_red(), self.kind()) {
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
        };
        if self.is_red() { write!(f, "\x1B[31m{}\x1b[0m", char) } else { write!(f, "{}", char) }
    }
}

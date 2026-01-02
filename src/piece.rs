use std::fmt::Formatter;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PieceKind {
    King,
    Advisor,
    Elephant,
    Horse,
    Rook,
    Cannon,
    Pawn,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Piece {
    Red(PieceKind),
    Black(PieceKind),
}

impl Piece {
    pub fn from_char(value: char) -> Option<Self> {
        let piece_kind = match value.to_ascii_lowercase() {
            'k' => PieceKind::King,
            'a' => PieceKind::Advisor,
            'e' => PieceKind::Elephant,
            'h' => PieceKind::Horse,
            'r' => PieceKind::Rook,
            'c' => PieceKind::Cannon,
            'p' => PieceKind::Pawn,
            _ => return None,
        };

        if value.is_ascii_uppercase() { Some(Self::Red(piece_kind)) } else { Some(Self::Black(piece_kind)) }
    }

    pub fn from_kind(kind: PieceKind, red: bool) -> Self {
        if red { Self::Red(kind) } else { Self::Black(kind) }
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let char = match self {
            Self::Red(piece_kind) => match piece_kind {
                PieceKind::King => '帥',
                PieceKind::Advisor => '仕',
                PieceKind::Elephant => '相',
                PieceKind::Horse => '傌',
                PieceKind::Rook => '俥',
                PieceKind::Cannon => '炮',
                PieceKind::Pawn => '兵',
            },
            Self::Black(piece_kind) => match piece_kind {
                PieceKind::King => '將',
                PieceKind::Advisor => '士',
                PieceKind::Elephant => '象',
                PieceKind::Horse => '馬',
                PieceKind::Rook => '車',
                PieceKind::Cannon => '砲',
                PieceKind::Pawn => '卒',
            },
        };
        let red = matches!(self, Self::Red(_));
        if red { write!(f, "\x1B[31m{}\x1b[0m", char) } else { write!(f, "{}", char) }
    }
}

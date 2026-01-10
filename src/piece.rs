use crate::display_format::DisplayFormat;
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

    pub fn fen(&self) -> char {
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

    pub fn display(&self, format: DisplayFormat) -> impl Display + use<> {
        struct Impl(Piece, DisplayFormat);
        return Impl(*self, format);

        impl Display for Impl {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let &Self(piece, format) = self;

                if format.effects {
                    if piece.is_red() {
                        write!(f, "\x1B[31m")?;
                    } else {
                        write!(f, "\x1B[1m")?;
                    }
                }

                if !format.concise {
                    write!(f, "{} ", if piece.is_red() { "red" } else { "black" })?;
                }

                if format.chinese {
                    let c = match (piece.is_red(), piece.kind()) {
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
                    write!(f, "{c}")?;
                } else {
                    if !format.concise {
                        let s = match piece.kind() {
                            PieceKind::King => "king",
                            PieceKind::Advisor => "advisor",
                            PieceKind::Elephant => "elephant",
                            PieceKind::Horse => "horse",
                            PieceKind::Chariot => "chariot",
                            PieceKind::Cannon => "cannon",
                            PieceKind::Pawn => "pawn",
                        };

                        write!(f, "{s}")?;
                    } else {
                        write!(f, "{c}{c}", c = piece.fen())?;
                    }
                };

                if format.effects {
                    write!(f, "\x1B[0m")?;
                }

                Ok(())
            }
        }
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(DisplayFormat::string().with_concise(false)))
    }
}

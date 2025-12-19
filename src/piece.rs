#[derive(Clone)]
pub enum PieceType {
    King,
    Advisor,
    Elephant,
    Horse,
    Rook,
    Cannon,
    Pawn,
}

#[derive(Clone)]
pub enum Piece {
    None,
    White(PieceType), //AKA red
    Black(PieceType),
}

impl Piece {
    pub fn from_char(value: char) -> Option<Self> {
        let piece_type = match value.to_ascii_lowercase() {
            'k' => PieceType::King,
            'a' => PieceType::Advisor,
            'e' => PieceType::Elephant,
            'h' => PieceType::Horse,
            'r' => PieceType::Rook,
            'c' => PieceType::Cannon,
            'p' => PieceType::Pawn,
            _ => return None,
        };

        if value.is_ascii_uppercase() {
            Some(Piece::White(piece_type))
        } else {
            Some(Piece::Black(piece_type))
        }
    }
}

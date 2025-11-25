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
    pub fn from_char(value: char) -> Self {
        let piece_type = match value.to_ascii_lowercase() {
            'k' => PieceType::King,
            'a' => PieceType::Advisor,
            'e' => PieceType::Elephant,
            'h' => PieceType::Horse,
            'r' => PieceType::Rook,
            'c' => PieceType::Cannon,
            'p' => PieceType::Pawn,
            _ => panic!("Unknown character for Piece {value}"),
        };

        if value.is_ascii_uppercase() { Piece::White(piece_type) } else { Piece::Black(piece_type) }
    }
}
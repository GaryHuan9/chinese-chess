struct Location {
    x: u8,
    y: u8,
}

#[derive(Clone)]
enum PieceType {
    General,
    Advisor,
    Elephant,
    Horse,
    Chariot,
    Cannon,
    Soldier,
}

#[derive(Clone)]
enum Piece {
    None,
    Red(PieceType),
    Black(PieceType),
}

struct Board {
    pieces: Vec<Piece>,
    captured: Vec<Piece>,
}

impl Board {
    const Width: u32 = 9;
    const Height: u32 = 10;

    fn new() -> Self {
        Self {
            pieces: vec![Piece::None; (Self::Width * Self::Height) as usize],
            captured: Vec::new(),
        }
    }
}

fn main() {
    println!("Hello, world!");
}

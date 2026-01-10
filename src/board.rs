use crate::display_format::DisplayFormat;
use crate::location::{Location, Move};
use crate::piece::{Piece, PieceKind};
use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct Board {
    pieces: Vec<Option<Piece>>,
}

impl Board {
    pub const WIDTH: i8 = 9;
    pub const HEIGHT: i8 = 10;

    pub fn new() -> Self {
        Self {
            pieces: vec![None; (Self::WIDTH * Self::HEIGHT) as usize],
        }
    }

    pub fn from_fen(fen: &str) -> Option<Self> {
        let mut board = Self::new();
        let mut y = Location::new().shift_y(Self::HEIGHT - 1).unwrap();
        let mut x = 0;

        for current in fen.chars() {
            match current {
                ' ' => break,
                '/' => {
                    if x != Self::WIDTH {
                        return None;
                    }
                    x = 0;
                    y = y.shift_y(-1)?;
                }
                '0'..='9' => x += current.to_digit(10).unwrap() as i8,
                _ => {
                    let piece = Piece::from_fen_char(current)?;
                    board[y.shift_x(x).unwrap()] = Some(piece);
                    x += 1;
                }
            }
        }

        Option::from(board)
    }

    pub fn opening() -> Self {
        Self::from_fen("rheakaehr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RHEAKAEHR").unwrap()
    }

    pub fn fen(&self) -> String {
        let mut result = String::new();
        let spaces = |last_piece: i8, x| {
            if last_piece < x {
                (x - last_piece).to_string()
            } else {
                String::new()
            }
        };
        for y in (0..Self::HEIGHT).rev() {
            let mut last_piece = 0;
            for x in 0..Self::WIDTH {
                let location = Location::from_xy(x, y).unwrap();
                let Some(piece) = self[location] else { continue };

                result.push_str(&spaces(last_piece, x));
                result.push(piece.fen());
                last_piece = x + 1
            }

            result.push_str(&spaces(last_piece, Self::WIDTH));
            result.push('/');
        }

        result.pop();
        result
    }

    pub fn play(&mut self, mv: Move) -> (Piece, Option<Piece>) {
        assert_ne!(mv.from, mv.to);
        let piece = self[mv.from].unwrap();
        self[mv.from] = None;

        let capture = self[mv.to];
        self[mv.to] = Some(piece);
        (piece, capture)
    }

    pub fn undo(&mut self, mv: Move, capture: Option<Piece>) {
        assert!(self[mv.from].is_none());
        self[mv.from] = self[mv.to];
        self[mv.to] = capture;
    }

    pub fn find_king(&self, red: bool) -> Option<Location> {
        let king = Some(Piece::from_kind(PieceKind::King, red));
        let predicate = |piece: &Option<Piece>| *piece == king;
        Location::from_index(self.pieces.iter().position(predicate)?)
    }

    pub fn evaluate(&self, red: bool) -> i32 {
        self.pieces.iter().filter_map(|&p| p).map(|p| p.base_value(red)).sum()
    }

    pub fn iter_legal_moves(&self, red: bool) -> impl Iterator<Item = Move> {
        let mut copy = self.clone();

        self.iter_basic_moves(red).filter(move |mv| {
            let (_, capture) = copy.play(*mv);
            let Some(king) = copy.find_king(red) else {
                copy.undo(*mv, capture);
                return false;
            };
            let legal = copy.iter_basic_moves(!red).filter(|mv| mv.to == king).next();
            copy.undo(*mv, capture);
            legal.is_none()
        })
    }

    pub fn iter_basic_moves(&self, red: bool) -> impl Iterator<Item = Move> {
        let mut moves = vec![];
        for (index, &piece) in self.pieces.iter().enumerate() {
            let from = Location::from_index(index).unwrap();
            let Some(piece) = piece else { continue };
            if piece.is_red() != red {
                continue;
            }

            let mut add = |to: Option<Location>| {
                let Some(to) = to else { return };
                if let Some(piece) = self[to]
                    && piece.is_red() == red
                {
                    return;
                }
                moves.push(Move { from, to });
            };

            match piece.kind() {
                PieceKind::King => {
                    let mut add = |to: Option<Location>| {
                        let Some(to) = to else { return };
                        if (to.x() - Self::WIDTH / 2).abs() <= 1 && to.normalize(red).y() < 3 {
                            add(Some(to));
                        }
                    };
                    add(from.shift_x(1));
                    add(from.shift_x(-1));
                    add(from.shift_y(1));
                    add(from.shift_y(-1));

                    let mut current = from.normalize(red);
                    loop {
                        let Some(to) = current.shift_y(1) else { break };
                        current = to;

                        let to = to.normalize(red);
                        let Some(piece) = self[to] else { continue };
                        if piece.kind() == PieceKind::King && piece.is_red() != red {
                            moves.push(Move { from, to });
                        }

                        break;
                    }
                }
                PieceKind::Advisor => {
                    let x = from.x() - Self::WIDTH / 2;
                    if x == 0 {
                        add(from.shift_xy(1, 1));
                        add(from.shift_xy(-1, 1));
                        add(from.shift_xy(1, -1));
                        add(from.shift_xy(-1, -1));
                    } else {
                        let from = from.normalize(red);
                        let y = from.y() - 1;
                        add(Some(from.shift_xy(-x, -y).unwrap().normalize(red)));
                    }
                }
                PieceKind::Elephant => {
                    let mut add = |x, y| {
                        let Some(block) = from.shift_xy(x, y) else { return };
                        if self[block].is_some() {
                            return;
                        };

                        let Some(to) = from.shift_xy(x * 2, y * 2) else { return };
                        if to.normalize(red).y() >= Self::HEIGHT / 2 {
                            return;
                        };

                        add(Some(to));
                    };
                    add(1, 1);
                    add(-1, 1);
                    add(1, -1);
                    add(-1, -1);
                }
                PieceKind::Horse => {
                    let mut add =
                        |shift_major: fn(Location) -> Option<Location>,
                         shift_minor: fn(Location, i8) -> Option<Location>| {
                            let Some(block) = shift_major(from) else { return };
                            if self[block].is_some() {
                                return;
                            }

                            let Some(to) = shift_major(block) else { return };
                            add(shift_minor(to, 1));
                            add(shift_minor(to, -1));
                        };
                    add(|to| to.shift_x(1), |to, value| to.shift_y(value));
                    add(|to| to.shift_x(-1), |to, value| to.shift_y(value));
                    add(|to| to.shift_y(1), |to, value| to.shift_x(value));
                    add(|to| to.shift_y(-1), |to, value| to.shift_x(value));
                }
                PieceKind::Chariot => {
                    let mut add = |shift: fn(Location) -> Option<Location>| {
                        let mut current = shift(from);

                        while let Some(to) = current {
                            add(current);
                            if self[to].is_some() {
                                return;
                            }
                            current = shift(to);
                        }
                    };
                    add(|to| to.shift_x(1));
                    add(|to| to.shift_x(-1));
                    add(|to| to.shift_y(1));
                    add(|to| to.shift_y(-1));
                }
                PieceKind::Cannon => {
                    let mut add = |shift: fn(Location) -> Option<Location>| {
                        let mut current = shift(from);
                        loop {
                            let Some(to) = current else { return };
                            if self[to].is_some() {
                                break;
                            }
                            moves.push(Move { from, to });
                            current = shift(to);
                        }

                        current = shift(current.unwrap());
                        loop {
                            let Some(to) = current else { return };

                            if let Some(piece) = self[to] {
                                if piece.is_red() != red {
                                    moves.push(Move { from, to });
                                }
                                break;
                            }

                            current = shift(to);
                        }
                    };

                    add(|to| to.shift_x(1));
                    add(|to| to.shift_x(-1));
                    add(|to| to.shift_y(1));
                    add(|to| to.shift_y(-1));
                }
                PieceKind::Pawn => {
                    let from = from.normalize(red);
                    let mut add = |to: Option<Location>| add(to.map(|to| to.normalize(red)));

                    add(from.shift_y(1));

                    if from.y() >= Self::HEIGHT / 2 {
                        add(from.shift_x(1));
                        add(from.shift_x(-1));
                    }
                }
            }
        }
        moves.into_iter()
    }

    pub fn display(&self, format: DisplayFormat) -> impl Display {
        struct Impl<'a>(&'a Board, DisplayFormat);
        return Impl(self, format);

        impl Display for Impl<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let &Self(board, format) = self;
                write!(f, "{}", board.fen())?;

                if format.concise {
                    return Ok(());
                }

                writeln!(f)?;

                for y in (0..Board::HEIGHT).rev() {
                    write!(f, "{y}")?;
                    for x in 0..Board::WIDTH {
                        if let Some(piece) = board[Location::from_xy(x, y).unwrap()] {
                            write!(f, " {}", piece.display(format.with_concise(true)))?;
                        } else {
                            write!(f, "   ")?;
                        }
                    }
                    writeln!(f)?;
                }

                for char in 'A'..='I' {
                    write!(f, "  {char}")?;
                }
                Ok(())
            }
        }
    }
}

impl Index<Location> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Location) -> &Self::Output {
        &self.pieces[index.index()]
    }
}

impl IndexMut<Location> for Board {
    fn index_mut(&mut self, index: Location) -> &mut Self::Output {
        &mut self.pieces[index.index()]
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display(DisplayFormat::string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_count(fen: &str, depth: u32) -> usize {
        let mut board = Board::from_fen(fen).unwrap();
        let mut play_stack: Vec<Move> = board.iter_legal_moves(true).collect();
        let mut undo_stack: Vec<(usize, Move, Option<Piece>)> = vec![];

        if depth == 1 {
            return play_stack.len();
        }

        let mut result = 0;

        while let Some(mv) = play_stack.pop() {
            while let Some(&(index, mv, capture)) = undo_stack.last()
                && index == play_stack.len() + 1
            {
                undo_stack.pop();
                board.undo(mv, capture);
            }

            let (_, capture) = board.play(mv);
            undo_stack.push((play_stack.len(), mv, capture));

            let height = undo_stack.len() as u32;
            let red = height.is_multiple_of(2);
            let moves = board.iter_legal_moves(red);

            if depth == height + 1 {
                result += moves.count();
            } else {
                play_stack.extend(moves);
            }
        }

        result
    }

    fn assert_count(fen: &str, counts: &[usize]) {
        for (depth, &expected) in counts.iter().enumerate() {
            let depth = (depth + 1) as u32;
            assert_eq!(expected, node_count(fen, depth), "depth: {depth}");
        }
    }

    // perft numbers from https://www.chessprogramming.org/Chinese_Chess_Perft_Results

    #[test]
    fn perft_opening() {
        let results = &[44, 1920, 79666, 3290240, 133312995];
        assert_count("rheakaehr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RHEAKAEHR", results);
    }

    #[test]
    fn perft_position2() {
        let results = &[38, 1128, 43929, 1339047, 53112976];
        assert_count("r1ea1a3/4kh3/2h1e4/pHp1p1p1p/4c4/6P2/P1P2R2P/1CcC5/9/2EAKAE2", results);
    }

    #[test]
    fn perft_position3() {
        let results = &[7, 281, 8620, 326201, 10369923];
        assert_count("1ceak4/9/h2a5/2p1p3p/5cp2/2h2H3/6PCP/3AE4/2C6/3A1K1H1", results);
    }

    #[test]
    fn perft_position4() {
        let results = &[25, 424, 9850, 202884, 4739553, 100055401];
        assert_count("5a3/3k5/3aR4/9/5r3/5h3/9/3A1A3/5K3/2EC2E2", results);
    }

    #[test]
    fn perft_position5() {
        let results = &[28, 516, 14808, 395483, 11842230, 367168327];
        assert_count("CRH1k1e2/3ca4/4ea3/9/2hr5/9/9/4E4/4A4/4KA3", results);
    }

    #[test]
    fn perft_position6() {
        let results = &[21, 364, 7626, 162837, 3500505, 81195154];
        assert_count("R1H1k1e2/9/3aea3/9/2hr5/2E6/9/4E4/4A4/4KA3", results);
    }

    #[test]
    fn perft_position7() {
        let results = &[28, 222, 6241, 64971, 1914306, 23496493];
        assert_count("C1hHk4/9/9/9/9/9/h1pp5/E3C4/9/3A1K3", results);
    }

    #[test]
    fn perft_position8() {
        let results = &[23, 345, 8124, 149272, 3513104, 71287903];
        assert_count("4ka3/4a4/9/9/4H4/p8/9/4C3c/7h1/2EK5", results);
    }

    #[test]
    fn perft_position9() {
        let results = &[21, 195, 3883, 48060, 933096, 12250386];
        assert_count("2e1ka3/9/e3H4/4h4/9/9/9/4C4/2p6/2EK5", results);
    }

    #[test]
    fn perft_position10() {
        let results = &[30, 830, 22787, 649866, 17920736, 517687990];
        assert_count("1C2ka3/9/C1Hae1h2/p3p3p/6p2/9/P3P3P/3AE4/3p2c2/c1EAK4", results);
    }

    #[test]
    fn perft_position11() {
        let results = &[19, 583, 11714, 376467, 8148177, 270587571];
        assert_count("ChH1k1e2/c3a4/4ea3/9/2hr5/9/9/4C4/4A4/4KA3", results);
    }
}

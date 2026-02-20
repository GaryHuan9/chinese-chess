use crate::board::Board;
use crate::display_format::DisplayFormat;
use crate::location::Move;
use crate::piece::Piece;
use std::fmt::{Display, Formatter};

pub struct Ranker {
    board: Board,
    red_turn: bool,
    entries: Vec<Entry>,
}

#[derive(Clone)]
struct Entry {
    mv: Move,
    value: i32,
    checked: u32,
}

impl Ranker {
    pub fn new(board: Board, red_turn: bool) -> Ranker {
        let entries = board
            .iter_legal_moves(red_turn)
            .map(|mv| Entry {
                mv,
                value: 0,
                checked: 0,
            })
            .collect();
        Ranker {
            board,
            red_turn,
            entries,
        }
    }

    pub fn display(&self, format: DisplayFormat) -> impl Display {
        struct Impl<'a>(&'a Ranker, DisplayFormat);
        return Impl(self, format);

        impl Display for Impl<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let &Self(ranker, _) = self;
                let mut entries = ranker.entries.clone();
                entries.sort_by_key(|e| e.value);

                for entry in entries.iter().rev() {
                    writeln!(f, "{} {} {}", entry.mv, entry.value, entry.checked)?;
                }

                Ok(())
            }
        }
    }

    pub fn best(&self) -> Move {
        self.entries.iter().max_by_key(|e| e.value).unwrap().mv
    }

    fn rank_single(board: &mut Board, red_turn: bool, depth: u32, entry: &mut Entry) {
        if depth == 0 {
            unimplemented!();
        }
        let mut play_stack: Vec<Move> = board.iter_legal_moves(red_turn).collect();
        let mut undo_stack: Vec<(usize, Move, Option<Piece>, i32)> = vec![];

        // let mut lower = i32::MAX;
        // let mut upper = i32::MIN;

        let mut best = -i32::MAX;

        while let Some(mv) = play_stack.pop() {
            while let Some(&(index, mv, capture, value)) = undo_stack.last()
                && index == play_stack.len() + 1
            {
                undo_stack.pop();
                board.undo(mv, capture);

                if let Some((_, _, _, parent)) = undo_stack.last_mut() {
                    *parent = (*parent).max(-value);
                } else {
                    best = best.max(-value);
                }
            }

            let (_, capture) = board.play(mv);
            undo_stack.push((play_stack.len(), mv, capture, -i32::MAX));

            let height = undo_stack.len() as u32;
            let red_turn = !height.is_multiple_of(2) ^ red_turn;

            if depth == height {
                entry.checked += 1;
                undo_stack.last_mut().unwrap().3 = board.evaluate(red_turn);
            } else {
                let moves = board.iter_legal_moves(red_turn);
                play_stack.extend(moves);
            }
        }

        while let Some((_, mv, capture, value)) = undo_stack.pop() {
            board.undo(mv, capture);

            if let Some((_, _, _, parent)) = undo_stack.last_mut() {
                *parent = (*parent).max(-value);
            } else {
                best = best.max(-value);
            }
        }

        entry.value = best;
    }

    pub fn rank(&mut self, depth: u32) {
        for entry in &mut self.entries {
            let (_, capture) = self.board.play(entry.mv);

            entry.checked = 0;

            Self::rank_single(&mut self.board, !self.red_turn, depth, entry);
            self.board.undo(entry.mv, capture);
            entry.value = -entry.value;

            // println!("{}", self.board.display(DisplayFormat::pretty()));
        }
    }

    pub fn rank_recursive(&mut self, depth: u32) {
        fn search(board: &mut Board, red: bool, depth: u32) -> (i32, u32) {
            if depth == 0 {
                return (board.evaluate(red), 1);
            }

            let mut best = -i32::MAX;
            let mut total = 0u32;

            for mv in board.iter_legal_moves(red).collect::<Box<_>>() {
                let (_, capture) = board.play(mv);
                let (value, count) = search(board, !red, depth - 1);
                board.undo(mv, capture);

                best = best.max(-value);
                total += count;
            }

            (best, total)
        }

        for entry in &mut self.entries {
            let (_, capture) = self.board.play(entry.mv);
            let (value, checked) = search(&mut self.board, !self.red_turn, depth);
            self.board.undo(entry.mv, capture);

            entry.value = -value;
            entry.checked = checked;
        }
    }
}

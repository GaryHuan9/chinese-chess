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
    const MIN_VALUE: i32 = -i32::MAX;
    const MAX_VALUE: i32 = i32::MAX;

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

    fn rank_single(board: &mut Board, red_turn: bool, depth: u32) -> (i32, u32) {
        if depth == 0 {
            return (board.evaluate(red_turn), 1);
        }

        let mut play_stack: Vec<Move> = board.iter_legal_moves(red_turn).rev().collect();
        let mut undo_stack: Vec<(usize, Move, Option<Piece>, i32, i32)> = vec![];

        let mut best_value = Self::MIN_VALUE;
        let mut total_checked = 0;

        while let Some(mv) = play_stack.pop() {
            {
                let (lower_bound, upper_bound) = if let Some(&(_, _, _, parent_lower, parent_upper)) = undo_stack.last()
                {
                    (-parent_upper, -parent_lower)
                } else {
                    (Self::MIN_VALUE, -best_value)
                };

                undo_stack.push((play_stack.len(), mv, board.play(mv), lower_bound, upper_bound));
            }

            let height = undo_stack.len() as u32;
            let red_turn = !height.is_multiple_of(2) ^ red_turn;

            if depth == height {
                total_checked += 1;
                let var = &mut undo_stack.last_mut().unwrap().3;
                *var = (*var).max(board.evaluate(red_turn));
            } else {
                play_stack.extend(board.iter_legal_moves(red_turn).rev());
            }

            while let Some(&(index, mv, capture, lower_bound, upper_bound)) = undo_stack.last()
                && index == play_stack.len()
            {
                undo_stack.pop();
                board.undo(mv, capture);

                if let Some((height, _, _, parent_lower, parent_upper)) = undo_stack.last_mut() {
                    *parent_lower = (*parent_lower).max(-lower_bound);
                    if *parent_lower >= *parent_upper {
                        play_stack.truncate(*height);
                    }
                } else {
                    best_value = best_value.max(-lower_bound);
                }
            }
        }

        (best_value, total_checked)
    }

    pub fn rank(&mut self, depth: u32) {
        for entry in &mut self.entries {
            let capture = self.board.play(entry.mv);

            let (value, checked) = Self::rank_single(&mut self.board, !self.red_turn, depth);
            self.board.undo(entry.mv, capture);

            entry.value = -value;
            entry.checked += checked;
        }
    }

    pub fn rank_recursive(&mut self, depth: u32) {
        fn search(board: &mut Board, red: bool, depth: u32, mut lower_bound: i32, upper_bound: i32) -> (i32, u32) {
            if depth == 0 {
                return (board.evaluate(red), 1);
            }

            let mut total = 0u32;

            for mv in board.iter_legal_moves(red).collect::<Box<_>>() {
                let capture = board.play(mv);
                let (value, count) = search(board, !red, depth - 1, -upper_bound, -lower_bound);
                board.undo(mv, capture);

                lower_bound = lower_bound.max(-value);
                total += count;

                if lower_bound >= upper_bound {
                    break;
                }
            }

            (lower_bound, total)
        }

        for entry in &mut self.entries {
            let capture = self.board.play(entry.mv);
            let (value, checked) = search(&mut self.board, !self.red_turn, depth, Self::MIN_VALUE, Self::MAX_VALUE);
            self.board.undo(entry.mv, capture);

            entry.value = -value;
            entry.checked += checked;
        }
    }
}

use crate::display_format::DisplayFormat;
use crate::game::Game;
use crate::location::Move;
use std::fmt::{Display, Formatter};

pub struct Ranker {
    game: Game,
    entries: Vec<Entry>,
}

#[derive(Clone)]
struct Entry {
    mv: Move,
    value: i32,
    checked: u32,
}
const MIN_VALUE: i32 = -(i32::MAX - 1000);
const MAX_VALUE: i32 = -MIN_VALUE;

impl Ranker {
    pub fn new(game: Game) -> Ranker {
        let entries = game
            .iter_moves()
            .map(|mv| Entry {
                mv,
                value: 0,
                checked: 0,
            })
            .collect();
        Ranker { game, entries }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn make_move(&mut self, mv: Move) {
        self.game.make_move(mv);
        let entries = self
            .game
            .iter_moves()
            .map(|mv| Entry {
                mv,
                value: 0,
                checked: 0,
            })
            .collect();
        self.entries = entries;
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

    fn rank_single(game: &mut Game, depth: u32, lower: i32) -> (i32, u32) {
        if depth == 0 {
            return (game.evaluate(), 1);
        }

        struct Branch {
            best: i32,
            mark: u32,
            lower: i32,
            upper: i32,
        }

        impl Branch {
            fn new(lower: i32, upper: i32, mark: usize) -> Self {
                Self {
                    best: MIN_VALUE,
                    mark: mark as u32,
                    lower,
                    upper,
                }
            }

            fn record(&mut self, value: i32) -> Option<usize> {
                self.best = self.best.max(value);
                self.lower = self.lower.max(value);
                if self.lower >= self.upper {
                    return Some(self.mark as usize);
                }
                None
            }
        }

        let mut moves: Vec<Move> = game.iter_moves().collect();
        let mut stack: Vec<Branch> = vec![];
        let mut branch = Branch::new(MIN_VALUE, -lower, 0);
        let mut checked = 0;

        while let Some(mv) = moves.pop() {
            game.make_move(mv);

            let value = if (stack.len() as u32) < depth - 1 {
                // search deeper
                let length = moves.len();
                moves.extend(game.iter_moves());
                if length != moves.len() {
                    // has children
                    let new_branch = Branch::new(-branch.upper, -branch.lower, length);
                    stack.push(branch);
                    branch = new_branch;
                    continue;
                }

                // opponent has no move left
                MAX_VALUE
            } else {
                // reached leaf node
                checked += 1;
                -game.evaluate()
            };

            game.undo_move();

            if let Some(mark) = branch.record(value) {
                // prune branch
                moves.truncate(mark);
            }

            while branch.mark == moves.len() as u32 {
                // no more to search on this branch
                let Some(old_branch) = stack.pop() else {
                    // entire search complete
                    assert!(moves.is_empty());
                    break;
                };

                // continue back to parent branch
                let value = -branch.best;
                branch = old_branch;

                game.undo_move();

                if let Some(mark) = branch.record(value) {
                    // prune branch
                    moves.truncate(mark);
                }
            }
        }

        (branch.best, checked)
    }

    pub fn rank(&mut self, depth: u32) {
        let mut lower = MIN_VALUE;

        for entry in &mut self.entries {
            self.game.make_move(entry.mv);
            let (value, checked) = Self::rank_single(&mut self.game, depth, lower - 1);
            self.game.undo_move();

            entry.value = -value;
            entry.checked += checked;
            lower = lower.max(-value);
        }
    }

    pub fn rank_recursive(&mut self, depth: u32) {
        fn search(game: &mut Game, depth: u32, mut lower: i32, upper: i32) -> (i32, u32) {
            if depth == 0 {
                return (game.evaluate(), 1);
            }

            let mut total = 0u32;
            let mut best = MIN_VALUE;

            for mv in game.iter_moves().rev().collect::<Box<_>>() {
                game.make_move(mv);
                let (value, count) = search(game, depth - 1, -upper, -lower);
                game.undo_move();

                best = best.max(-value);
                lower = lower.max(-value);
                total += count;

                if lower >= upper {
                    break;
                }
            }

            (best, total)
        }

        let mut lower = MIN_VALUE;

        for entry in self.entries.iter_mut() {
            self.game.make_move(entry.mv);
            let (value, checked) = search(&mut self.game, depth, MIN_VALUE, -(lower - 1));
            self.game.undo_move();

            entry.value = -value;
            entry.checked += checked;
            lower = lower.max(-value);
        }
    }

    pub fn rank_simple(&mut self, depth: u32) {
        fn search(game: &mut Game, depth: u32) -> (i32, u32) {
            if depth == 0 {
                return (game.evaluate(), 1);
            }

            let mut total = 0u32;
            let mut best = MIN_VALUE;

            for mv in game.iter_moves().collect::<Box<_>>() {
                game.make_move(mv);
                let (value, count) = search(game, depth - 1);
                game.undo_move();

                total += count;
                best = best.max(-value);
            }

            (best, total)
        }

        for entry in self.entries.iter_mut() {
            self.game.make_move(entry.mv);
            let (value, checked) = search(&mut self.game, depth);
            self.game.undo_move();

            entry.value = -value;
            entry.checked += checked;
        }
    }
}

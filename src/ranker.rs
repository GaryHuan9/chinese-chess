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

impl Ranker {
    const MIN_VALUE: i32 = -(i32::MAX - 42);
    const MAX_VALUE: i32 = -Self::MIN_VALUE;

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

        let mut play_stack: Vec<Move> = game.iter_moves().collect();
        let mut undo_stack: Vec<(u32, i32, i32, i32)> = vec![(0, Self::MIN_VALUE, Self::MIN_VALUE, -lower)];
        let mut total_checked = 0;

        while let Some(mv) = play_stack.pop() {
            game.make_move(mv);

            if (undo_stack.len() as u32) < depth {
                let &(_, _, lower, upper) = undo_stack.last().unwrap();
                let (lower, upper) = (-upper, -lower);
                undo_stack.push((play_stack.len() as u32, Self::MIN_VALUE, lower, upper));
                play_stack.extend(game.iter_moves());
                continue;
            }

            total_checked += 1;
            let value = game.evaluate();
            game.undo_move();

            let (height, best, lower, upper) = undo_stack.last_mut().unwrap();
            *best = (*best).max(-value);
            *lower = (*lower).max(-value);
            if *lower >= *upper {
                play_stack.truncate(*height as usize);
            }

            while let &(height, value, _, _) = undo_stack.last().unwrap()
                && height == play_stack.len() as u32
            {
                if undo_stack.len() == 1 {
                    break;
                }

                undo_stack.pop();
                game.undo_move();

                let (height, best, lower, upper) = undo_stack.last_mut().unwrap();
                *best = (*best).max(-value);
                *lower = (*lower).max(-value);
                if *lower >= *upper {
                    play_stack.truncate(*height as usize);
                }
            }
        }

        (undo_stack.last().unwrap().1, total_checked)
    }

    pub fn rank(&mut self, depth: u32) {
        let mut lower = Self::MIN_VALUE;

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
            let mut best = Ranker::MIN_VALUE;

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

        let mut lower = Self::MIN_VALUE;

        for entry in self.entries.iter_mut() {
            self.game.make_move(entry.mv);
            let (value, checked) = search(&mut self.game, depth, Self::MIN_VALUE, -lower);
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
            let mut best = Ranker::MIN_VALUE;

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

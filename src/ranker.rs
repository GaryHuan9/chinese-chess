use crate::display_format::DisplayFormat;
use crate::game::Game;
use crate::location::Move;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};

pub struct Ranker {
    game: Game,
    entries: Box<[(Move, Option<Rank>)]>,
}

struct Rank {
    value: i32,
    chain: VecDeque<Move>,
    evaluated: u32,
}

const MIN_VALUE: i32 = -(i32::MAX - 1000);

impl Ranker {
    pub fn new(game: Game) -> Ranker {
        let entries = game.iter_moves().map(|mv| (mv, None)).collect();
        let result = Ranker { game, entries };
        assert!(result.is_sorted());
        result
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn best(&self) -> Option<Move> {
        assert!(self.is_sorted());
        self.entries.first().map(|&(mv, _)| mv)
    }

    pub fn make_move(&mut self, mv: Move) {
        assert!(self.entries.iter().find(|(m, _)| *m == mv).is_some());
        self.game.make_move(mv);
        self.entries = self.game.iter_moves().map(|mv| (mv, None)).collect();
        assert!(self.is_sorted());
    }

    pub fn rank(&mut self, depth: u32) {
        let mut lower = MIN_VALUE;

        for (mv, rank) in &mut self.entries {
            self.game.make_move(*mv);
            let value = Rank::replace(rank, search(&mut self.game, depth, MIN_VALUE, -(lower - 1), rank).rev());
            self.game.undo_move();

            lower = lower.max(value);
        }

        self.sort();

        fn search(game: &mut Game, depth: u32, lower: i32, upper: i32, rank: &Option<Rank>) -> Rank {
            if depth == 0 {
                return Rank::new(game.evaluate());
            }

            struct Branch {
                mark: u32,
                lower: i32,
                upper: i32,
                rank: Option<Rank>,
            }

            impl Branch {
                fn new(mark: usize, lower: i32, upper: i32) -> Self {
                    Self {
                        mark: mark as u32,
                        lower,
                        upper,
                        rank: None,
                    }
                }

                fn record(&mut self, rank: Rank, mv: Move) -> Option<usize> {
                    let value = Rank::combine(&mut self.rank, rank, mv);
                    self.lower = self.lower.max(value);

                    if self.lower >= self.upper {
                        Some(self.mark as usize)
                    } else {
                        None
                    }
                }
            }

            let mut moves: Vec<Move> = game.iter_moves().collect();
            let mut stack: Vec<Branch> = vec![];
            let mut branch = Branch::new(0, lower, upper);

            while let Some(mv) = moves.pop() {
                game.make_move(mv);

                let rank = if (stack.len() as u32) < depth - 1 {
                    // search deeper
                    let length = moves.len();
                    moves.extend(game.iter_moves());
                    if length != moves.len() {
                        // has children
                        let new_branch = Branch::new(length, -branch.upper, -branch.lower);
                        stack.push(branch);
                        branch = new_branch;
                        continue;
                    }

                    // opponent has no move left
                    Rank::minimum()
                } else {
                    // reached leaf node
                    Rank::new(game.evaluate())
                };

                game.undo_move();

                if let Some(mark) = branch.record(rank.rev(), mv) {
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
                    let rank = branch.rank.unwrap_or_else(Rank::minimum);
                    branch = old_branch;

                    let mv = game.undo_move();

                    if let Some(mark) = branch.record(rank.rev(), mv) {
                        // prune branch
                        moves.truncate(mark);
                    }
                }
            }

            branch.rank.unwrap_or_else(Rank::minimum)
        }
    }

    pub fn rank_recursive(&mut self, depth: u32) {
        let mut lower = MIN_VALUE;

        for (mv, rank) in &mut self.entries {
            self.game.make_move(*mv);
            let value = Rank::replace(rank, search(&mut self.game, depth, MIN_VALUE, -(lower - 1)).rev());
            self.game.undo_move();

            lower = lower.max(value);
        }

        self.sort();

        fn search(game: &mut Game, depth: u32, mut lower: i32, upper: i32) -> Rank {
            if depth == 0 {
                return Rank::new(game.evaluate());
            }

            let depth = depth - 1;
            let mut rank = None;

            for mv in game.iter_moves().rev().collect::<Box<_>>() {
                game.make_move(mv);
                let value = Rank::combine(&mut rank, search(game, depth, -upper, -lower).rev(), mv);
                game.undo_move();

                lower = lower.max(value);

                if lower >= upper {
                    break;
                }
            }

            rank.unwrap_or_else(Rank::minimum)
        }
    }

    pub fn rank_simple(&mut self, depth: u32) {
        for (mv, rank) in &mut self.entries {
            self.game.make_move(*mv);
            Rank::replace(rank, search(&mut self.game, depth).rev());
            self.game.undo_move();
        }

        self.sort();

        fn search(game: &mut Game, depth: u32) -> Rank {
            if depth == 0 {
                return Rank::new(game.evaluate());
            }

            let depth = depth - 1;
            let mut rank = None;

            for mv in game.iter_moves().rev().collect::<Box<_>>() {
                game.make_move(mv);
                Rank::combine(&mut rank, search(game, depth).rev(), mv);
                game.undo_move();
            }

            rank.unwrap_or_else(Rank::minimum)
        }
    }

    pub fn display(&self, format: DisplayFormat) -> impl Display {
        struct Impl<'a>(&'a Ranker, DisplayFormat);
        return Impl(self, format);

        impl Display for Impl<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let &Self(ranker, format) = self;
                assert!(ranker.is_sorted());

                let Some((best_mv, Some(best))) = ranker.entries.first() else {
                    write!(f, "{{}}")?;
                    return if format.concise { Ok(()) } else { writeln!(f) };
                };

                if format.concise {
                    return write!(
                        f,
                        "{} = {} / {} ({})",
                        best_mv,
                        best.value,
                        best.chain.len(),
                        best.evaluated
                    );
                }

                let Some((_, Some(worst))) = ranker.entries.last() else {
                    unreachable!()
                };

                let length = |n: i32| n.to_string().len();
                let width = length(best.value).max(length(worst.value));

                for (mv, rank) in &ranker.entries {
                    let piece = ranker.game[mv.from].unwrap();
                    let Some(rank) = rank else {
                        writeln!(f, "{} {} unranked", mv, piece.display(format.with_concise(true)))?;
                        continue;
                    };

                    let best = rank.value == best.value;

                    write!(
                        f,
                        "{} {} {} {:width$} / {} ({})",
                        mv,
                        piece.display(format.with_concise(true)),
                        if best { '=' } else { '≤' },
                        rank.value,
                        rank.chain.len(),
                        rank.evaluated,
                    )?;

                    let mut game = ranker.game.clone();
                    game.make_move(*mv);

                    for (i, &mv) in rank.chain.iter().enumerate() {
                        match i {
                            0 => write!(f, ": ")?,
                            c if c == if best { 5 } else { 3 } => {
                                write!(f, "…")?;
                                break;
                            }
                            _ => write!(f, ", ")?,
                        }

                        let piece = game[mv.from].unwrap();
                        write!(f, "{} {}", mv, piece.display(format.with_concise(true)))?;

                        game.make_move(mv);
                    }

                    writeln!(f)?
                }

                Ok(())
            }
        }
    }

    fn sort(&mut self) {
        self.entries
            .sort_by(|(_, a), (_, b)| Rank::compare_option(a, b).reverse());
    }

    fn is_sorted(&self) -> bool {
        self.entries
            .is_sorted_by(|(_, a), (_, b)| Rank::compare_option(a, b).is_ge())
    }
}

impl Rank {
    fn new(value: i32) -> Rank {
        Self {
            value,
            chain: VecDeque::new(),
            evaluated: 1,
        }
    }

    fn minimum() -> Rank {
        Self {
            value: MIN_VALUE,
            chain: VecDeque::new(),
            evaluated: 0,
        }
    }

    fn rev(self) -> Rank {
        Self {
            value: -self.value,
            chain: self.chain,
            evaluated: self.evaluated,
        }
    }

    fn combine(rank: &mut Option<Self>, mut other: Rank, mv: Move) -> i32 {
        match rank.as_mut() {
            None => {
                other.chain.push_front(mv);
                let value = other.value;
                *rank = Some(other);
                value
            }
            Some(rank) => {
                if Self::compare(&rank, &other).is_lt() {
                    rank.value = other.value;
                    rank.chain = other.chain;
                    rank.chain.push_front(mv);
                }

                rank.evaluated += other.evaluated;
                rank.value
            }
        }
    }

    fn replace(rank: &mut Option<Self>, mut other: Rank) -> i32 {
        if let Some(rank) = rank {
            other.evaluated += rank.evaluated;
        }
        let value = other.value;
        *rank = Some(other);
        value
    }

    fn compare(a: &Self, b: &Self) -> Ordering {
        let short = a.chain.len().cmp(&b.chain.len());
        let short = if a.value < 0 { short } else { short.reverse() };
        a.value.cmp(&b.value).then(short)
    }

    fn compare_option(a: &Option<Self>, b: &Option<Self>) -> Ordering {
        match (a, b) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(a), Some(b)) => Self::compare(a, b),
        }
    }
}

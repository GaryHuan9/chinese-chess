use crate::display_format::DisplayFormat;
use crate::game::Game;
use crate::location::Move;
use std::fmt::{Display, Formatter};
use std::ops::Neg;

pub struct Ranker {
    game: Game,
    depth: u32,
    entries: Box<[Entry]>,
}

#[derive(Debug)]
struct Entry {
    mv: Move,
    rank: Rank,
    chain: Vec<Move>,
    evaluated: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    data: i32,
}

struct Search<'a> {
    game: &'a mut Game,
    max_depth: u32,
    moves: Vec<Move>,
    evaluated: u32,
}

impl Ranker {
    pub fn new(game: Game) -> Self {
        let entries = game.iter_moves().map(Entry::new).collect();
        Self {
            game,
            depth: 0,
            entries,
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn best(&self) -> Option<Move> {
        self.entries.iter().max_by_key(|entry| entry.rank).map(|entry| entry.mv)
    }

    pub fn make_move(&mut self, mv: Move) {
        assert!(self.entries.iter().find(|entry| entry.mv == mv).is_some());
        self.game.make_move(mv);
        self.depth = 0;
        self.entries = self.game.iter_moves().map(Entry::new).collect();
    }

    pub fn deeper(&mut self) {
        self.depth += 1;

        let mut search = Search {
            game: &mut self.game,
            max_depth: self.depth,
            moves: Vec::new(),
            evaluated: 0,
        };

        let upper = Rank::mate(0);
        let mut lower = -upper;

        let mut entries = self.entries.iter_mut().collect::<Box<_>>();
        entries.sort_by_key(|entry| -entry.rank);

        for entry in entries {
            search.game.make_move(entry.mv);
            search.evaluated = entry.evaluated;
            let (rank, chain) = search.chain(0, -upper, -lower, &entry.chain);
            // let (rank, chain) = search.normal(0, -upper, -lower);

            assert!(search.moves.is_empty());
            entry.evaluated = search.evaluated;
            search.game.undo_move();

            let rank = -rank;
            entry.rank = rank;
            entry.chain = chain;
            entry.chain.reverse();

            lower = lower.max(rank.one_less());
        }
    }

    pub fn display(&self, format: DisplayFormat) -> impl Display {
        struct Impl<'a>(&'a Ranker, DisplayFormat);
        return Impl(self, format);

        impl Display for Impl<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let format = self.1;
                let mut entries = self.0.entries.iter().collect::<Box<_>>();
                entries.sort_by_key(|entry| -entry.rank);

                let Some((best, worst)) = entries.first().zip(entries.last()) else {
                    write!(f, "{{}}")?;
                    return if format.concise { Ok(()) } else { writeln!(f) };
                };

                if format.concise {
                    return write!(
                        f,
                        "{} = {} / {} ({})",
                        best.mv,
                        best.rank,
                        best.chain.len(),
                        best.evaluated
                    );
                }

                let length = |rank: Rank| rank.to_string().len();
                let width = length(best.rank).max(length(worst.rank));

                for entry in &entries {
                    let mut game = self.0.game.clone();
                    let piece = game[entry.mv.from].unwrap();
                    let best = entry.rank == best.rank;

                    write!(
                        f,
                        "{} {} {} {:width$} / {} ({})",
                        entry.mv,
                        piece.display(format.with_concise(true)),
                        if best { '=' } else { '≤' },
                        entry.rank,
                        entry.chain.len(),
                        entry.evaluated,
                    )?;

                    game.make_move(entry.mv);

                    for (i, &mv) in entry.chain.iter().enumerate() {
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

                let total = entries.iter().map(|entry| entry.evaluated).sum::<u32>();
                write!(f, "depth {} with {total} evaluation", self.0.depth)
            }
        }
    }
}

impl Rank {
    const CHECKMATE_VALUE: i32 = 2_000_000_000;
    const CHECKMATE_DEPTH_LIMIT: u32 = 100;

    fn new(value: i32) -> Rank {
        debug_assert!(value.abs() < Self::CHECKMATE_VALUE);
        Self { data: value }
    }

    fn mate(depth: u32) -> Rank {
        debug_assert!(depth < Self::CHECKMATE_DEPTH_LIMIT);
        Self {
            data: Self::CHECKMATE_VALUE + (Self::CHECKMATE_DEPTH_LIMIT - depth - 1) as i32,
        }
    }

    fn one_less(self) -> Rank {
        Self { data: self.data - 1 }
    }
}

impl Neg for Rank {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self { data: -self.data }
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

impl std::fmt::Debug for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Entry {
    fn new(mv: Move) -> Entry {
        Self {
            mv,
            rank: Rank::new(0),
            chain: Vec::new(),
            evaluated: 0,
        }
    }
}

impl<'a> Search<'a> {
    fn base(&mut self, depth: u32) -> Option<(Rank, Vec<Move>)> {
        if depth == self.max_depth {
            self.evaluated += 1;
            Some((Rank::new(self.game.evaluate()), Vec::new()))
        } else {
            None
        }
    }

    fn chain(&mut self, depth: u32, lower: Rank, upper: Rank, chain: &Vec<Move>) -> (Rank, Vec<Move>) {
        if let Some(base) = self.base(depth) {
            return base;
        }

        let Some(best) = chain.get(depth as usize) else {
            return self.normal(depth, lower, upper);
        };

        let old_length = self.moves.len();
        self.game.fill_moves(&mut self.moves);

        let best = self.moves[old_length..].iter().position(|mv| mv == best);
        let mv = self.moves.swap_remove(old_length + best.unwrap());

        self.game.make_move(mv);

        let (rank, chain) = self.chain(depth + 1, -upper, -lower, chain);

        self.game.undo_move();

        let mut lower = lower;
        let rank = -rank;
        let mut chain = chain;
        chain.push(mv);

        if lower < rank {
            lower = rank;
            if lower >= upper {
                self.moves.truncate(old_length);
                return (rank, chain);
            }
        }

        self.normal_recurse(depth, lower, upper, old_length, rank, chain)
    }

    fn normal(&mut self, depth: u32, lower: Rank, upper: Rank) -> (Rank, Vec<Move>) {
        if let Some(base) = self.base(depth) {
            return base;
        }

        let old_length = self.moves.len();
        self.game.fill_moves(&mut self.moves);

        self.normal_recurse(depth, lower, upper, old_length, -Rank::mate(depth), Vec::new())
    }

    fn normal_recurse(
        &mut self,
        depth: u32,
        lower: Rank,
        upper: Rank,
        old_length: usize,
        best_rank: Rank,
        best_chain: Vec<Move>,
    ) -> (Rank, Vec<Move>) {
        let new_length = self.moves.len();

        let mut lower = lower;
        let mut best_rank = best_rank;
        let mut best_chain = best_chain;

        for i in old_length..new_length {
            let mv = self.moves[i];
            self.game.make_move(mv);

            let (rank, chain) = self.normal(depth + 1, -upper, -lower);
            debug_assert!(self.moves.len() == new_length);

            self.game.undo_move();

            let rank = -rank;
            if best_rank < rank {
                best_rank = rank;
                best_chain = chain;
                best_chain.push(mv);

                if lower < rank {
                    lower = rank;
                    if lower >= upper {
                        break;
                    }
                }
            }
        }

        self.moves.truncate(old_length);
        (best_rank, best_chain)
    }
}

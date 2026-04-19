use crate::display_format::DisplayFormat;
use crate::game::Game;
use crate::location::Move;
use std::fmt::{Display, Formatter};
use std::ops::Neg;

pub struct Ranker {
    game: Game,
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

impl Ranker {
    pub fn new(game: Game) -> Self {
        let entries = game.iter_moves().map(Entry::new).collect();
        Self { game, entries }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn best(&self) -> Option<Move> {
        self.entries.iter().max_by_key(|entry| entry.rank).map(|entry| entry.mv)
    }

    pub fn make_move(&mut self, mv: Move) {
        assert!(self.entries.iter().find(|entry| entry.mv == mv).is_some());
        self.game.make_move(mv);
        self.entries = self.game.iter_moves().map(Entry::new).collect();
    }

    pub fn rank(&mut self, depth: u32) {
        let mut moves = Vec::new();
        let upper = Rank::mate(0);
        let mut lower = -upper;

        for entry in &mut self.entries {
            self.game.make_move(entry.mv);

            let (rank, chain) = search(
                depth,
                &mut self.game,
                &mut moves,
                &mut entry.evaluated,
                0,
                -upper,
                -lower,
            );

            let rank = -rank;
            self.game.undo_move();

            entry.rank = rank;
            entry.chain = chain;
            entry.chain.reverse();

            lower = lower.max(rank.one_less());
        }

        fn search(
            max_depth: u32,
            game: &mut Game,
            moves: &mut Vec<Move>,
            evaluated: &mut u32,
            depth: u32,
            lower: Rank,
            upper: Rank,
        ) -> (Rank, Vec<Move>) {
            if depth == max_depth {
                *evaluated += 1;
                return (Rank::new(game.evaluate()), Vec::new());
            }

            let mut best_rank = -Rank::mate(depth);
            let mut best_chain = Vec::new();
            let mut lower = lower;

            let old_length = moves.len();
            game.fill_moves(moves);
            let new_length = moves.len();

            for i in old_length..new_length {
                let mv = moves[i];
                game.make_move(mv);

                let (rank, chain) = search(max_depth, game, moves, evaluated, depth + 1, -upper, -lower);
                debug_assert!(moves.len() == new_length);

                let rank = -rank;
                game.undo_move();

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

            moves.truncate(old_length);
            (best_rank, best_chain)
        }
    }

    pub fn rank_simple(&mut self, depth: u32) {
        for entry in &mut self.entries {
            self.game.make_move(entry.mv);
            let (rank, chain) = search(&mut self.game, &mut entry.evaluated, depth, 0);
            let rank = -rank;
            self.game.undo_move();

            entry.rank = rank;
            entry.chain = chain;
            entry.chain.reverse();
        }

        fn search(game: &mut Game, evaluated: &mut u32, max_depth: u32, depth: u32) -> (Rank, Vec<Move>) {
            if depth == max_depth {
                *evaluated += 1;
                return (Rank::new(game.evaluate()), Vec::new());
            }

            let mut best_rank = -Rank::mate(depth);
            let mut best_chain = Vec::new();

            for mv in game.iter_moves().collect::<Box<_>>() {
                game.make_move(mv);
                let (rank, chain) = search(game, evaluated, max_depth, depth + 1);
                let rank = -rank;
                game.undo_move();

                if best_rank < rank {
                    best_rank = rank;
                    best_chain = chain;
                    best_chain.push(mv);
                }
            }

            (best_rank, best_chain)
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
                write!(f, "{total} evaluated",)
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

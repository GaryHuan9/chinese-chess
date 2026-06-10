use crate::display_format::DisplayFormat;
use crate::game::Game;
use crate::location::Move;
use std::fmt::{Display, Formatter};
use std::ops::Neg;

pub struct Ranker {
    game: Game,
    max_depth: u32,
    best_rank: Rank,
    best_chain: Vec<Move>,
    evaluated: u32,
    pruned: u32,
    moves_buffer: Vec<Move>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    data: i32,
}

impl Ranker {
    pub fn new(game: Game) -> Self {
        Self {
            game,
            max_depth: 0,
            best_rank: Rank::new(0),
            best_chain: Vec::new(),
            evaluated: 0,
            pruned: 0,
            moves_buffer: Vec::new(),
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn depth(&self) -> u32 {
        self.max_depth
    }

    pub fn best(&self) -> Option<Move> {
        self.best_chain.last().copied()
    }

    pub fn make_move(&mut self, mv: Move) {
        self.game.make_move(mv);
        self.max_depth = 0;
        self.best_rank = Rank::new(0);
        self.best_chain.clear();
        self.evaluated = 0;
        self.pruned = 0;
    }

    pub fn deeper(&mut self) {
        self.max_depth += 1;

        let upper = Rank::mate(0);
        let lower = -upper;

        let chain = std::mem::take(&mut self.best_chain);
        let (rank, chain) = self.search_hinted(0, lower, upper, chain);
        assert!(self.moves_buffer.is_empty());

        self.best_rank = rank;
        self.best_chain = chain;
    }

    fn search_hinted(&mut self, depth: u32, lower: Rank, upper: Rank, mut chain: Vec<Move>) -> (Rank, Vec<Move>) {
        let Some(best) = chain.pop() else {
            return self.search_normal(depth, lower, upper);
        };

        assert!(depth < self.max_depth);
        let old_length = self.moves_buffer.len();
        self.game.fill_moves(&mut self.moves_buffer);

        let best = self.moves_buffer[old_length..].iter().position(|&mv| mv == best);
        let mv = self.moves_buffer.swap_remove(old_length + best.unwrap());

        self.game.make_move(mv);

        let (rank, chain) = self.search_hinted(depth + 1, -upper, -lower, chain);

        self.game.undo_move();

        let mut lower = lower;
        let rank = -rank;
        let mut chain = chain;
        chain.push(mv);

        if lower < rank {
            lower = rank;
            if lower >= upper {
                self.pruned += 1;
                self.moves_buffer.truncate(old_length);
                return (rank, chain);
            }
        }

        self.search_recurse(depth, lower, upper, old_length, rank, chain)
    }

    fn search_normal(&mut self, depth: u32, lower: Rank, upper: Rank) -> (Rank, Vec<Move>) {
        if depth == self.max_depth {
            self.evaluated += 1;
            return (Rank::new(self.game.evaluate()), Vec::new());
        }

        let old_length = self.moves_buffer.len();
        self.game.fill_moves(&mut self.moves_buffer);

        self.search_recurse(depth, lower, upper, old_length, -Rank::mate(depth), Vec::new())
    }

    fn search_recurse(
        &mut self,
        depth: u32,
        lower: Rank,
        upper: Rank,
        old_length: usize,
        best_rank: Rank,
        best_chain: Vec<Move>,
    ) -> (Rank, Vec<Move>) {
        let new_length = self.moves_buffer.len();

        let mut lower = lower;
        let mut best_rank = best_rank;
        let mut best_chain = best_chain;

        for i in old_length..new_length {
            let mv = self.moves_buffer[i];
            self.game.make_move(mv);

            let (rank, chain) = self.search_normal(depth + 1, -upper, -lower);
            debug_assert!(self.moves_buffer.len() == new_length);

            self.game.undo_move();

            let rank = -rank;
            if best_rank < rank {
                best_rank = rank;
                best_chain = chain;
                best_chain.push(mv);

                if lower < rank {
                    lower = rank;
                    if lower >= upper {
                        self.pruned += 1;
                        break;
                    }
                }
            }
        }

        self.moves_buffer.truncate(old_length);
        (best_rank, best_chain)
    }

    pub fn display(&self, format: DisplayFormat) -> impl Display {
        struct Impl<'a>(&'a Ranker, DisplayFormat);
        return Impl(self, format);

        impl Display for Impl<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let ranker = self.0;
                let format = self.1;

                let Some(best) = ranker.best() else {
                    write!(f, "{{}}")?;
                    return if format.concise { Ok(()) } else { writeln!(f) };
                };

                let mut game = ranker.game.clone();
                let piece = game[best.from].unwrap();

                write!(
                    f,
                    "{} {} = {} / {}",
                    best,
                    piece.display(format.with_concise(true)),
                    ranker.best_rank,
                    ranker.best_chain.len(),
                )?;

                game.make_move(best);

                for (i, &mv) in ranker.best_chain.iter().rev().skip(1).enumerate() {
                    match i {
                        0 => write!(f, ": ")?,
                        5 => {
                            write!(f, "…")?;
                            break;
                        }
                        _ => write!(f, ", ")?,
                    }

                    let piece = game[mv.from].unwrap();
                    write!(f, "{} {}", mv, piece.display(format.with_concise(true)))?;

                    game.make_move(mv);
                }

                writeln!(f)?;
                write!(
                    f,
                    "depth {} with {} evaluated, {} pruned",
                    ranker.max_depth, ranker.evaluated, ranker.pruned
                )
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

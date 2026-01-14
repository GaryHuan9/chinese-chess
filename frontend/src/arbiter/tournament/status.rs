use chinese_chess::game::Outcome;
use std::fmt::Display;

#[derive(Copy, Clone)]
pub struct Status {
    pub score: Score,
    pub queued: u32,
    pub running: u32,
}

#[derive(Copy, Clone)]
pub struct Score {
    pub win: u32,
    pub loss: u32,
    pub draw: u32,
}

impl Status {
    pub fn new() -> Self {
        Self {
            score: Score::new(),
            queued: 0,
            running: 0,
        }
    }

    pub fn merge(&mut self, status: &Self) {
        self.score.merge(&status.score);
        self.queued += status.queued;
        self.running += status.running;
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} queued ({}) running ({})", self.score, self.queued, self.running)
    }
}

impl Score {
    pub fn new() -> Self {
        Self {
            win: 0,
            loss: 0,
            draw: 0,
        }
    }

    pub fn merge(&mut self, score: &Self) {
        self.win += score.win;
        self.loss += score.loss;
        self.draw += score.draw;
    }

    pub fn negate(&mut self) {
        (self.win, self.loss) = (self.loss, self.win);
    }
}

impl From<Outcome> for Score {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::RedWon => Self { win: 1, ..Self::new() },
            Outcome::BlackWon => Self { loss: 1, ..Self::new() },
            Outcome::Stalemate | Outcome::MoveRule => Self { draw: 1, ..Self::new() },
        }
    }
}

impl Default for Score {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "w({}) l({}) d({})", self.win, self.loss, self.draw)
    }
}

use crate::line_stream::AsyncLineStream;
use crate::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use chinese_chess::game::{Game, Outcome};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, Weak};

type PlayerId = usize;

pub struct Tournament {
    this: Weak<Mutex<Self>>,
    ids: HashMap<String, PlayerId>,
    players: Vec<Arc<Mutex<Player>>>,
    pending: Vec<(PlayerId, PlayerId, u32)>,
}

impl Tournament {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new_cyclic(|weak| {
            Mutex::new(Self {
                this: weak.clone(),
                ids: HashMap::new(),
                players: Vec::new(),
                pending: Vec::new(),
            })
        })
    }

    pub fn join(&mut self, name: String, stream: AsyncLineStream) {
        let id = self.ids.get(&name).copied().unwrap_or_else(|| {
            let id = self.players.len() as PlayerId;
            self.players.push(Arc::new(Mutex::new(Player::new(id))));
            self.ids.insert(name, id);
            id
        });

        {
            let mut player = self.players[id].lock().unwrap();
            player.create_instance(stream);
        }

        self.match_all();
    }

    fn match_all(&mut self) {
        while self.match_once() {}
    }

    fn match_once(&mut self) -> bool {
        self.pending.sort_by_key(|(_, _, count)| *count);

        for (home, away, count) in self.pending.iter_mut().rev() {
            if *count == 0 {
                break;
            }

            let [home, away] = self.players.get_disjoint_mut([*home, *away]).unwrap();
            let Some(future) = Player::play(home.clone(), away.clone()) else {
                continue;
            };

            *count -= 1;
            let this = self.this.clone();

            smol::spawn(async move {
                future.await;
                if let Some(this) = this.upgrade() {
                    this.lock().unwrap().match_all();
                }
            })
            .detach();

            return true;
        }

        false
    }
}

struct Player {
    id: PlayerId,
    instances: VecDeque<Instance>,
    scores: HashMap<PlayerId, Score>,
}

impl Player {
    pub fn new(id: PlayerId) -> Self {
        Self {
            id,
            instances: VecDeque::new(),
            scores: HashMap::new(),
        }
    }

    pub fn create_instance(&mut self, stream: AsyncLineStream) {
        let instance = Instance::new(self.id, stream);
        self.instances.push_back(instance);
    }

    pub fn play(home: Arc<Mutex<Self>>, away: Arc<Mutex<Self>>) -> Option<impl Future<Output = ()>> {
        assert!(!Arc::ptr_eq(&home, &away));

        let borrow_instance = |player: &Arc<Mutex<Self>>| {
            let mut player = player.lock().unwrap();
            player.instances.pop_front()
        };

        let return_instance = |player: &Arc<Mutex<Self>>, instance: Option<Instance>| {
            let Some(instance) = instance else { return };
            let mut player = player.lock().unwrap();
            player.instances.push_back(instance);
        };

        let home_instance = borrow_instance(&home)?;
        let Some(away_instance) = borrow_instance(&away) else {
            return_instance(&home, Some(home_instance));
            return None;
        };

        Some(async move {
            let game = Game::opening();
            let away_id = away_instance.id();
            let (score, home_instance, away_instance) = compete(game, home_instance, away_instance).await;

            return_instance(&home, home_instance);
            return_instance(&away, away_instance);

            let mut home = home.lock().unwrap();
            let record = home.scores.entry(away_id).or_insert_with(Score::new);
            *record = record.merge(&score);
        })
    }
}

struct Instance {
    id: PlayerId,
    stream: AsyncLineStream,
}

impl Instance {
    pub fn new(id: PlayerId, stream: AsyncLineStream) -> Self {
        Self { id, stream }
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }

    pub async fn recv(&self) -> Result<PlayerMessage, PlayerId> {
        self.stream
            .read_line()
            .await
            .and_then(|line| Protocol::decode_player(&line))
            .ok_or(self.id)
    }

    pub async fn send(&self, message: &ArbiterMessage) -> Result<(), PlayerId> {
        self.stream
            .write_line(Protocol::encode_arbiter(message))
            .await
            .map_err(|_| self.id)
    }
}

async fn compete(game: Game, home: Instance, away: Instance) -> (Score, Option<Instance>, Option<Instance>) {
    // home will always be playing red
    match compete_impl(game, &home, &away).await {
        Ok(score) => (score, Some(home), Some(away)),
        Err(id) => {
            if id == home.id {
                (Score::lost(), None, Some(away))
            } else {
                assert_eq!(id, away.id);
                (Score::won(), Some(home), None)
            }
        }
    }
}

async fn compete_impl(mut game: Game, home: &Instance, away: &Instance) -> Result<Score, PlayerId> {
    // setup game for both players
    {
        let message = ArbiterMessage::from_game(&game);

        smol::future::try_zip(home.send(&message), away.send(&message)).await?;

        let wait = async |player: &Instance| -> Result<(), PlayerId> {
            while !matches!(player.recv().await?, PlayerMessage::Ready) {}
            Ok(())
        };

        smol::future::try_zip(wait(&home), wait(&away)).await?;
    }

    loop {
        // one move
        let prompt = async |game: &mut Game, player: &Instance| -> Result<Option<Score>, PlayerId> {
            let score = Score::from_outcome(game.outcome());
            if score.is_some() {
                return Ok(score);
            }

            let mv = loop {
                home.send(&ArbiterMessage::Prompt { time: 1000 }).await?;
                let PlayerMessage::Play { mv } = player.recv().await? else {
                    return Err(player.id());
                };

                if game.play(mv) {
                    break mv;
                }
            };

            let message = ArbiterMessage::Update { mv };
            smol::future::try_zip(home.send(&message), away.send(&message)).await?;
            Ok(None)
        };

        if let Some(score) = prompt(&mut game, &home).await? {
            return Ok(score);
        }

        if let Some(score) = prompt(&mut game, &away).await? {
            return Ok(score);
        }
    }
}

pub struct Score {
    pub win: u32,
    pub loss: u32,
    pub draw: u32,
}

impl Score {
    pub fn new() -> Self {
        Self {
            win: 0,
            loss: 0,
            draw: 0,
        }
    }

    pub fn from_outcome(outcome: Option<Outcome>) -> Option<Self> {
        Some(match outcome? {
            Outcome::RedWon => Score::won(),
            Outcome::BlackWon => Score::lost(),
            Outcome::Stalemate | Outcome::MoveRule => Self { draw: 1, ..Self::new() },
        })
    }

    pub fn won() -> Self {
        Self { win: 1, ..Self::new() }
    }

    pub fn lost() -> Self {
        Self { loss: 1, ..Self::new() }
    }

    #[must_use]
    pub fn merge(&self, score: &Score) -> Self {
        Self {
            win: self.win + score.win,
            loss: self.loss + score.loss,
            draw: self.draw + score.draw,
        }
    }

    #[must_use]
    pub fn negate(&self) -> Self {
        Self {
            win: self.loss,
            loss: self.win,
            draw: self.draw,
        }
    }
}

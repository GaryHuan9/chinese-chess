use crate::line_stream::AsyncLineStream;
use crate::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use chinese_chess::game::{Game, Outcome};
use log::{debug, info, trace, warn};
use std::collections::{HashMap, VecDeque};
use std::ops::Div;
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
            let player = Player::new(id, name.clone());
            self.players.push(Arc::new(Mutex::new(player)));
            self.ids.insert(name.clone(), id);

            self.ids
                .clone()
                .keys()
                .filter(|n| n.as_str() != &name)
                .for_each(|n| self.enqueue(&name, &n, 10).unwrap());

            id
        });

        {
            let mut player = self.players[id].lock().unwrap();
            player.create_instance(stream);
        }

        self.match_all();
    }

    pub fn enqueue<'a>(&mut self, name0: &'a str, name1: &'a str, count: u32) -> Result<(), &'a str> {
        let id = |name| self.ids.get(name).copied().ok_or(name);

        let mut enqueue = |pair: (PlayerId, PlayerId), count: u32| {
            if count == 0 {
                return;
            }

            if let Some((_, _, total)) = self.pending.iter_mut().find(|(id0, id1, _)| (*id0, *id1) == pair) {
                *total += count;
            } else {
                self.pending.push((pair.0, pair.1, count))
            }
        };

        let id0 = id(name0)?;
        let id1 = id(name1)?;
        enqueue((id0, id1), count.div_ceil(2));
        enqueue((id1, id0), count.div(2));
        self.match_all();

        Ok(())
    }

    fn match_all(&mut self) {
        debug!("attempting to match all available players");
        while self.match_once() {}
    }

    fn match_once(&mut self) -> bool {
        self.pending.sort_by_key(|(_, _, count)| *count);

        for (home, away, count) in self.pending.iter_mut().rev() {
            if *count == 0 {
                break;
            }

            let [home, away] = self.players.get_disjoint_mut([*home, *away]).unwrap();
            let home_name = Player::name(home);
            let away_name = Player::name(away);

            trace!(
                "found {} pending matches between '{home_name}' and '{away_name}'",
                *count
            );

            let Some(future) = Player::play(home.clone(), away.clone()) else {
                continue;
            };

            trace!("spawning task for match between '{home_name}' and '{away_name}'",);

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
    name: String,
    instances: VecDeque<Instance>,
    scores: HashMap<PlayerId, Score>,
    total_instance_count: u32, // the total number of instances ever created for this player
}

impl Player {
    pub fn new(id: PlayerId, name: String) -> Self {
        info!("new player '{name}' registered with id {id}");
        Self {
            id,
            name,
            instances: VecDeque::new(),
            scores: HashMap::new(),
            total_instance_count: 0,
        }
    }

    pub fn create_instance(&mut self, stream: AsyncLineStream) {
        let name = format!("{}:{}", self.name, self.total_instance_count);
        let instance = Instance::new(self.id, name, stream);
        self.instances.push_back(instance);
        self.total_instance_count += 1;
    }

    pub fn name(this: &Arc<Mutex<Self>>) -> String {
        this.lock().unwrap().name.clone()
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

        let Some(home_instance) = borrow_instance(&home) else {
            trace!("'{}' has no available instance", Player::name(&home));
            return None;
        };

        let Some(away_instance) = borrow_instance(&away) else {
            trace!("'{}' has no available instance", Player::name(&away));
            return_instance(&home, Some(home_instance));
            return None;
        };

        Some(async move {
            let home_name = home_instance.name().to_owned();
            let away_name = away_instance.name().to_owned();
            let away_id = away_instance.id();

            let (score, home_instance, away_instance) = Instance::compete(home_instance, away_instance).await;

            return_instance(&home, home_instance);
            return_instance(&away, away_instance);

            let mut home = home.lock().unwrap();
            let record = home.scores.entry(away_id).or_insert_with(Score::new);
            *record = record.merge(&score);

            debug!(
                "match between '{}' and '{}' completed {:?} with total {:?}",
                home_name, away_name, score, record
            );
        })
    }
}

struct Instance {
    id: PlayerId,
    name: String,
    stream: AsyncLineStream,
}

impl Instance {
    pub fn new(id: PlayerId, name: String, stream: AsyncLineStream) -> Self {
        info!("new instance '{name}' registered with player id {id}");
        Self { id, name, stream }
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
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

    pub async fn compete(home: Instance, away: Instance) -> (Score, Option<Instance>, Option<Instance>) {
        let game = Game::opening();
        debug!(
            "creating game with standard openings for '{}' as red and '{}' as black",
            home.name(),
            away.name()
        );

        match Self::compete_impl(game, &home, &away).await {
            Ok(score) => (score, Some(home), Some(away)),
            Err(id) => {
                warn!("game terminated due to '{id}' resigning from disconnection");
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
        // setup game for both instances
        // home will always be playing red
        {
            let message = ArbiterMessage::from_game(&game);

            smol::future::try_zip(home.send(&message), away.send(&message)).await?;

            let wait = async |player: &Instance| -> Result<(), PlayerId> {
                while !matches!(player.recv().await?, PlayerMessage::Ready) {}
                Ok(())
            };

            smol::future::try_zip(wait(&home), wait(&away)).await?;
            trace!("both '{}' and '{}' are ready for game", home.name(), away.name());
        }

        loop {
            // one move
            let prompt = async |game: &mut Game, player: &Instance| -> Result<Option<Score>, PlayerId> {
                if let Some(outcome) = game.outcome() {
                    debug!("game concluded normally with {outcome:?}");
                    return Ok(Some(Score::from_outcome(outcome)));
                }

                let mv = loop {
                    let time = 1000;
                    trace!("prompting '{}' for next move with {time}ms remaining", player.name());

                    player.send(&ArbiterMessage::Prompt { time }).await?;
                    let PlayerMessage::Play { mv } = player.recv().await? else {
                        continue;
                    };

                    let legal = game.play(mv);
                    trace!(
                        "'{}' requested to play {} move {mv}",
                        player.name(),
                        if legal { "legal" } else { "illegal" }
                    );

                    if legal {
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
}

#[derive(Debug)]
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

    pub fn from_outcome(outcome: Outcome) -> Self {
        match outcome {
            Outcome::RedWon => Score::won(),
            Outcome::BlackWon => Score::lost(),
            Outcome::Stalemate | Outcome::MoveRule => Self { draw: 1, ..Self::new() },
        }
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

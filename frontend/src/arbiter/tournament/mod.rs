use crate::arbiter::tournament::status::{Score, Status};
use crate::line_stream::AsyncLineStream;
use log::debug;
use player::Player;
use std::collections::{HashMap, HashSet};
use std::ops::Div;
use std::sync::{Arc, RwLock, Weak};

mod instance;
mod player;
pub mod status;

type PlayerId = usize;

pub struct Tournament {
    this: Weak<RwLock<Self>>,
    ids: HashMap<String, PlayerId>,
    players: Vec<Arc<RwLock<Player>>>,
}

impl Tournament {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new_cyclic(|weak| {
            RwLock::new(Self {
                this: weak.clone(),
                ids: HashMap::new(),
                players: Vec::new(),
            })
        })
    }

    pub fn join(&mut self, name: String, stream: AsyncLineStream) {
        let id = self.ids.get(&name).copied().unwrap_or_else(|| {
            let id = self.players.len() as PlayerId;
            let player = Player::new(id, name.clone());
            self.players.push(Arc::new(RwLock::new(player)));
            self.ids.insert(name.clone(), id);

            id
        });

        {
            let mut player = self.players[id].write().unwrap();
            player.create_instance(stream);
        }

        self.match_all();
    }

    pub fn enqueue(&'_ self, name: &str) -> Option<Queue<'_>> {
        self.ids.get(name).copied().map(|id| Queue::new(self, id))
    }

    pub fn status(&self, name: &str) -> Option<impl Iterator<Item = (String, Status)>> {
        let id = self.ids.get(name).copied()?;

        // must collect first to avoid deadlock as with are holding a read lock
        let result = self.players[id].read().unwrap().iter_status().collect::<Box<_>>();
        let mut result = result
            .into_iter()
            .map(|(id, status)| (self.players[id].read().unwrap().name.clone(), status))
            .collect::<HashMap<_, _>>();

        for (other_id, player) in self.players.iter().enumerate() {
            let player = player.read().unwrap();

            if id == other_id {
                continue;
            }

            for (other_id, mut status) in player.iter_status() {
                if id != other_id {
                    continue;
                }

                status.score.negate();
                result
                    .entry(player.name.clone())
                    .and_modify(|current| current.merge(&status))
                    .or_insert(status);
            }
        }

        Some(result.into_iter())
    }

    fn match_all(&self) {
        debug!("attempting to match all available players");

        let mut candidates = self
            .players
            .iter()
            .enumerate()
            .map(|(id, player)| {
                player
                    .read()
                    .unwrap()
                    .iter_queued()
                    .map(|(other_id, queued)| (id, other_id, queued))
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        loop {
            candidates.sort_unstable_by_key(|(_, _, queued)| *queued);

            let mut candidates = candidates.iter_mut().rev().filter_map(|(home, away, queued)| {
                if *queued == 0 {
                    return None;
                }

                let home = self.players[*home].clone();
                let away = self.players[*away].clone();
                let Some(future) = Player::play(home, away) else {
                    return None;
                };

                *queued -= 1;
                let this = self.this.upgrade().unwrap();

                Some(async move {
                    future.await;
                    this.read().unwrap().match_all();
                })
            });

            if let Some(future) = candidates.next() {
                smol::spawn(future).detach();
            } else {
                break;
            }
        }
    }
}

pub struct Queue<'a> {
    tournament: &'a Tournament,
    player: PlayerId,
    pending: Vec<(String, bool, u32)>,
    except: Vec<String>,
}

impl<'a> Queue<'a> {
    fn new(tournament: &'a Tournament, player: PlayerId) -> Self {
        Self {
            tournament,
            player,
            pending: Vec::new(),
            except: Vec::new(),
        }
    }

    #[must_use]
    pub fn against(&mut self, name: String, count: u32) -> &Self {
        self.pending.push((name.clone(), true, count.div_ceil(2)));
        self.pending.push((name, false, count.div(2)));
        self
    }

    #[must_use]
    pub fn against_all(&mut self, count: u32) -> &Self {
        for (name, id) in &self.tournament.ids {
            if *id != self.player {
                let _ = self.against(name.clone(), count);
            }
        }
        self
    }

    #[must_use]
    pub fn against_as(&mut self, name: String, red: bool) -> &Self {
        self.pending.push((name, red, 1));
        self
    }

    #[must_use]
    pub fn except(&mut self, name: String) -> &Self {
        self.except.push(name);
        self
    }

    pub fn done(self) -> Result<(), impl IntoIterator> {
        let mut unknown = HashSet::new();

        for (name, home, count) in self.pending {
            let Some(id) = self.tournament.ids.get(&name).copied() else {
                unknown.insert(name);
                continue;
            };

            if self.except.contains(&name) || id == self.player {
                continue;
            }

            if count > 0 {
                let (home, away) = if home { (self.player, id) } else { (id, self.player) };
                self.tournament.players[home].write().unwrap().enqueue(away, count);
            }
        }

        self.tournament.match_all();

        unknown.extend(
            self.except
                .into_iter()
                .filter(|name| !self.tournament.ids.contains_key(name)),
        );
        if unknown.is_empty() { Ok(()) } else { Err(unknown) }
    }
}

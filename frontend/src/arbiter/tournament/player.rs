use crate::arbiter::tournament::instance::Instance;
use crate::arbiter::tournament::status::{Score, Status};
use crate::arbiter::tournament::PlayerId;
use crate::line_stream::AsyncLineStream;
use chinese_chess::game::Outcome;
use log::{debug, info, trace};
use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::sync::{Arc, RwLock};

pub struct Player {
    id: PlayerId,
    pub(crate) name: String,
    instances: VecDeque<Instance>,
    status: HashMap<PlayerId, Status>,
    total_instance_count: u32, // total number of instances ever created for this player
}

impl Player {
    pub fn new(id: PlayerId, name: String) -> Self {
        info!("new player '{name}' registered with id {id}");
        Self {
            id,
            name,
            instances: VecDeque::new(),
            status: HashMap::new(),
            total_instance_count: 0,
        }
    }

    pub fn create_instance(&mut self, stream: AsyncLineStream) {
        let name = format!("{}:{}", self.name, self.total_instance_count);
        let instance = Instance::new(self.id, name, stream);
        self.instances.push_back(instance);
        self.total_instance_count += 1;
    }

    pub fn enqueue(&mut self, away: PlayerId, count: u32) {
        assert_ne!(self.id, away);

        let status = &mut self.status.entry(away).or_insert_with(Status::new);
        status.queued += count;

        trace!(
            "'{}' enqueued {count} matches against player with id '{away}': {} ",
            self.name, *status
        );
    }

    pub fn iter_queued(&self) -> impl Iterator<Item = (PlayerId, u32)> {
        self.status
            .iter()
            .map(|(&away, Status { queued, .. })| (away, *queued))
            .filter(|(_, queued)| *queued > 0)
    }

    pub fn iter_status(&self) -> impl Iterator<Item = (PlayerId, Status)> {
        self.status.iter().map(|(id, status)| (*id, *status))
    }

    pub fn play(home: Arc<RwLock<Self>>, away: Arc<RwLock<Self>>) -> Option<impl Future<Output = ()>> {
        assert!(!Arc::ptr_eq(&home, &away));

        // borrow instance from away
        let (away_id, away_name, away_instance) = {
            let mut away = away.write().unwrap();
            let Some(instance) = away.instances.pop_front() else {
                trace!("'{}' has no available instance", away.name);
                return None;
            };

            (away.id, away.name.to_owned(), instance)
        };

        // check there is a queued match against away and atomically borrow instance from home
        let home_instance = {
            let mut lock = home.write().unwrap();
            let home = &mut *lock;
            let status = home.status.get_mut(&away_id);

            match status {
                None | Some(&mut Status { queued: 0, .. }) => {
                    trace!("'{}' has no match queued against '{}'", home.name, away_name);
                    None
                }
                Some(status) => match home.instances.pop_front() {
                    None => {
                        trace!("'{}' has no available instance", home.name);
                        None
                    }
                    Some(instance) => {
                        status.queued -= 1;
                        status.running += 1;
                        Some(instance)
                    }
                },
            }
        };

        // return instance to away if failed the atomic operation on home
        let Some(home_instance) = home_instance else {
            let mut away = away.write().unwrap();
            away.instances.push_back(away_instance);
            return None;
        };

        trace!(
            "spawning task for match between '{}' and '{away_name}'",
            home.read().unwrap().name
        );

        Some(async move {
            let home_name = home_instance.name().to_owned();
            let away_name = away_instance.name().to_owned();
            let (outcome, home_instance, away_instance) = Instance::compete(home_instance, away_instance).await;

            // return away instance
            if let Some(away_instance) = away_instance {
                let mut away = away.write().unwrap();
                away.instances.push_back(away_instance);
            }

            // update status and return home instance
            let mut home = home.write().unwrap();
            let status = home.status.get_mut(&away_id).unwrap();
            status.score.merge(&outcome.into());

            debug!("match between '{}' and '{}' done: {}", home_name, away_name, status);

            status.running -= 1;

            if let Some(home_instance) = home_instance {
                home.instances.push_back(home_instance)
            }
        })
    }
}

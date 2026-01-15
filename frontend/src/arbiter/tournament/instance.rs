use crate::arbiter::tournament::PlayerId;
use crate::line_stream::AsyncLineStream;
use crate::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use chinese_chess::game::{Game, Outcome};
use log::{debug, info, trace, warn};

pub struct Instance {
    id: PlayerId,
    pub(crate) name: String,
    stream: AsyncLineStream,
}

impl Instance {
    pub fn new(id: PlayerId, name: String, stream: AsyncLineStream) -> Self {
        info!("new instance '{name}' registered");
        Self { id, name, stream }
    }

    pub async fn compete(home: Instance, away: Instance) -> (Option<Outcome>, Option<Instance>, Option<Instance>) {
        let game = Game::opening();

        debug!(
            "creating game with standard openings for '{}' as red and '{}' as black",
            home.name, away.name
        );

        // try to initialize the game
        let home_init = Self::compete_init(&game, &home);
        let away_init = Self::compete_init(&game, &away);
        let (home_init, away_init) = smol::future::zip(home_init, away_init).await;

        if !home_init || !away_init {
            return (
                None,
                if home_init { Some(home) } else { None },
                if away_init { Some(away) } else { None },
            );
        }

        trace!("both '{}' and '{}' are ready for game", home.name, away.name);

        // home will always be playing red
        match Self::compete_main(game, &home, &away).await {
            Ok(outcome) => (Some(outcome), Some(home), Some(away)),
            Err(id) => {
                let (result, name) = if id == home.id {
                    ((Some(Outcome::BlackWon), None, Some(away)), home.name)
                } else {
                    assert_eq!(id, away.id);
                    ((Some(Outcome::RedWon), Some(home), None), away.name)
                };

                warn!("game terminated due to '{name}' resigning from disconnection");
                result
            }
        }
    }

    async fn recv(&self) -> Result<PlayerMessage, PlayerId> {
        self.stream
            .read_line()
            .await
            .and_then(|line| {
                let result = Protocol::decode_player(&line);
                if result.is_none() {
                    warn!("failed to decode '{}' message: {line}", self.name);
                }
                result
            })
            .ok_or(self.id)
    }

    async fn send(&self, message: &ArbiterMessage) -> Result<(), PlayerId> {
        self.stream
            .write_line(Protocol::encode_arbiter(message))
            .await
            .map_err(|_| self.id)
    }

    async fn compete_init(game: &Game, instance: &Instance) -> bool {
        let result = async {
            instance.send(&ArbiterMessage::from_game(game)).await?;
            while !matches!(instance.recv().await?, PlayerMessage::Ready) {}
            Ok::<(), PlayerId>(())
        };
        if result.await.is_err() {
            warn!("game failed to initialize for '{}'", instance.name);
            false
        } else {
            true
        }
    }

    async fn compete_main(mut game: Game, home: &Instance, away: &Instance) -> Result<Outcome, PlayerId> {
        loop {
            // one move
            let prompt = async |game: &mut Game, instance: &Instance| -> Result<Option<Outcome>, PlayerId> {
                if let Some(outcome) = game.outcome() {
                    debug!(
                        "game between '{}' and '{}' concluded normally with {outcome}",
                        home.name, away.name
                    );
                    return Ok(Some(outcome));
                }

                let mv = loop {
                    let time = 1000;
                    trace!("prompting '{}' for next move with {time}ms remaining", instance.name);
                    instance.send(&ArbiterMessage::Prompt { time }).await?;

                    let PlayerMessage::Play { mv } = instance.recv().await? else {
                        warn!(
                            "disconnecting '{}' due to unexpected message during game",
                            instance.name
                        );
                        return Err(instance.id);
                    };

                    let legal = game.play(mv);
                    trace!(
                        "'{}' requested to play {} move {mv}",
                        instance.name,
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

            // red move
            if let Some(outcome) = prompt(&mut game, home).await? {
                return Ok(outcome);
            }

            // black move
            if let Some(outcome) = prompt(&mut game, away).await? {
                return Ok(outcome);
            }
        }
    }
}

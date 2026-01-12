use crate::arbiter::tournament::status::Score;
use crate::arbiter::tournament::PlayerId;
use crate::line_stream::AsyncLineStream;
use crate::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use chinese_chess::game::{Game, Outcome};
use log::{debug, info, trace, warn};

pub struct Instance {
    id: PlayerId,
    name: String,
    stream: AsyncLineStream,
}

impl Instance {
    pub fn new(id: PlayerId, name: String, stream: AsyncLineStream) -> Self {
        info!("new instance '{name}' registered");
        Self { id, name, stream }
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

    pub async fn compete(home: Instance, away: Instance) -> (Outcome, Option<Instance>, Option<Instance>) {
        let game = Game::opening();
        debug!(
            "creating game with standard openings for '{}' as red and '{}' as black",
            home.name(),
            away.name()
        );

        // home will always be playing red
        match Self::compete_impl(game, &home, &away).await {
            Ok(score) => (score, Some(home), Some(away)),
            Err(id) => {
                let (result, name) = if id == home.id {
                    ((Outcome::BlackWon, None, Some(away)), home.name())
                } else {
                    assert_eq!(id, away.id);
                    ((Outcome::RedWon, Some(home), None), away.name())
                };
                warn!("game terminated due to '{name}' resigning from disconnection");
                result
            }
        }
    }

    async fn compete_impl(mut game: Game, home: &Instance, away: &Instance) -> Result<Outcome, PlayerId> {
        // setup game for both instances
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
            let prompt = async |game: &mut Game, player: &Instance| -> Result<Option<Outcome>, PlayerId> {
                if let Some(outcome) = game.outcome() {
                    debug!("game concluded normally with {outcome}");
                    return Ok(Some(outcome));
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

            if let Some(outcome) = prompt(&mut game, &home).await? {
                return Ok(outcome);
            }

            if let Some(outcome) = prompt(&mut game, &away).await? {
                return Ok(outcome);
            }
        }
    }
}

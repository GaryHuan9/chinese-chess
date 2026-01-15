use crate::arbiter::tournament::Tournament;
use crate::line_stream::LineStream;
use crate::protocol::{ArbiterMessage, PlayerMessage};
use chinese_chess::display_format::DisplayFormat;
use chinese_chess::game::Game;
use chinese_chess::location::{Location, Move};
use clap::{Parser, Subcommand};
use std::io::Error;
use std::net::TcpStream;
use std::sync::{Arc, RwLock};

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "show status of players")]
    Status { names: Vec<String> },
    #[command(about = "enqueue a player to compete")]
    Enqueue {
        name: String,
        #[arg(help = "players to compete against, all other players if not specified")]
        against: Vec<String>,
        #[arg(
            short,
            long,
            default_value_t = 2,
            help = "how many games to play against each player"
        )]
        count: u32,
    },
    #[command(alias = "c", about = "play against another player as a human player")]
    Contest {
        #[arg(default_value = "robot")]
        against: String,
        #[arg(short, long, default_value_t = false, help = "whether to play as red")]
        red: bool,
    },
}

pub fn begin(tournament: Arc<RwLock<Tournament>>, address: String) {
    begin_control(tournament, address);
}

fn begin_control(tournament: Arc<RwLock<Tournament>>, address: String) {
    const HUMAN_NAME: &str = "human";

    loop {
        match read_input() {
            Command::Status { names } => {
                if names.is_empty() {
                    println!("connected players:");
                    for name in tournament.read().unwrap().iter_players() {
                        println!("{}", name);
                    }
                } else {
                    for name in names {
                        let tournament = tournament.read().unwrap();
                        let Some(status) = tournament.status(&name) else {
                            println!("unknown player '{name}'");
                            continue;
                        };

                        for (other, status) in status {
                            println!("{name} vs. {other} - {status}");
                        }
                    }
                }
            }
            Command::Enqueue { name, against, count } => {
                let tournament = tournament.write().unwrap();
                let Some(queue) = tournament.enqueue(&name) else {
                    println!("unknown player '{name}'");
                    continue;
                };

                let queue = if against.is_empty() {
                    queue.against_all_except([HUMAN_NAME], count)
                } else {
                    against
                        .into_iter()
                        .fold(queue, |queue, name| queue.against(name, count))
                };

                if let Err(unknown) = queue.done() {
                    for name in unknown {
                        println!("unknown player '{name}'");
                    }
                }
            }
            Command::Contest { against, red } => {
                let init = || -> Result<LineStream, Error> {
                    let stream = LineStream::new(TcpStream::connect(&address)?);
                    stream.write(&PlayerMessage::Init { version: 1 })?;
                    stream.write(&PlayerMessage::Info {
                        name: HUMAN_NAME.to_owned(),
                    })?;
                    Ok(stream)
                };

                let Ok(stream) = init() else {
                    println!("failed to initialize");
                    continue;
                };

                // spin until connected and is able to enqueue, not gonna worry about proper blocking for now
                if loop {
                    let tournament = tournament.read().unwrap();
                    let Some(queue) = tournament.enqueue(HUMAN_NAME) else {
                        continue;
                    };
                    if let Err(unknown) = queue.against_as(against, red).done() {
                        for name in unknown {
                            println!("unknown player '{name}'");
                        }
                        break true;
                    }
                    break false;
                } {
                    continue;
                }

                println!("connected to tournament");
                if let Err(err) = begin_compete(stream) {
                    println!("disconnected with error - {err}");
                } else {
                    println!("disconnected");
                }
            }
        }
    }
}

fn begin_compete(stream: LineStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut game = if let ArbiterMessage::Game { fen, red_turn } = stream.read()?
        && let Some(game) = Game::from_fen(&fen, red_turn)
    {
        game
    } else {
        return Err("expected game message".into());
    };

    stream.write(&PlayerMessage::Ready)?;

    loop {
        loop {
            match stream.read()? {
                ArbiterMessage::Prompt { .. } => break,
                ArbiterMessage::Update { mv } => {
                    game.play(mv);

                    if game.outcome().is_some() {
                        println!("{}", game.display(DisplayFormat::pretty()));
                        return Ok(());
                    }
                }
                ArbiterMessage::Game { fen, red_turn } => {
                    game = Game::from_fen(&fen, red_turn).unwrap();
                    stream.write(&PlayerMessage::Ready)?;
                }
            };
        }

        println!("{}", game.display(DisplayFormat::pretty()));

        loop {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let line = line.trim().to_lowercase();

            let mv = {
                if let Ok(mv) = line.parse::<Move>() {
                    Some(mv)
                } else {
                    let mut moves = game.moves().clone();

                    for (i, char) in line.chars().enumerate() {
                        if moves.len() <= 1 {
                            break;
                        }

                        if !char.is_ascii_alphanumeric() {
                            continue;
                        }

                        let filter: Box<dyn Fn(Location) -> bool> = if char.is_ascii_digit() {
                            let y = (char as u8).wrapping_sub(b'0');
                            Box::new(move |location| location.y() == y as i8)
                        } else {
                            let x = (char as u8).wrapping_sub(b'a');
                            Box::new(move |location| location.x() == x as i8)
                        };

                        moves.retain(|mv| filter(if i <= 1 { mv.from } else { mv.to }));
                    }

                    if moves.len() == 1 { moves.first().cloned() } else { None }
                }
            };

            if let Some(mv) = mv {
                if game.moves().contains(&mv) {
                    stream.write(&PlayerMessage::Play { mv })?;
                    break;
                }
                println!("illegal move");
            } else if line == "end" {
                return Ok(());
            } else if line == "print" {
                println!("{}", game.display(DisplayFormat::pretty()));
            } else {
                println!("unknown input");
            }
        }
    }
}

fn read_input<T: clap::FromArgMatches + clap::Subcommand>() -> T {
    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();

        let parts = line.split_whitespace();

        #[derive(Parser)]
        #[command(
            name = "",
            no_binary_name = true,
            disable_help_flag = true,
            disable_version_flag = true,
            next_line_help = false,
            help_template = "{usage-heading} {usage}\n{all-args}"
        )]
        struct Input<T: clap::FromArgMatches + clap::Subcommand> {
            #[command(subcommand)]
            command: T,
        }

        match Input::<T>::try_parse_from(parts) {
            Ok(Input { command }) => return command,
            Err(err) => {
                print!("{}", err);
                continue;
            }
        };
    }
}

use crate::arbiter::tournament::Tournament;
use clap::{Parser, Subcommand};
use std::sync::{Arc, RwLock};

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "show status of players")]
    Status { names: Vec<String> },
    #[command(about = "enqueue a player to compete", alias = "e")]
    Enqueue {
        name: String,
        #[arg(help = "players to compete against")]
        against: Vec<String>,
        #[arg(
            short,
            long,
            default_value_t = 1,
            help = "how many games to play against each player"
        )]
        count: u32,
        #[arg(long, action = clap::ArgAction::Set, help = "whether to play as red, or play half the games as red if unspecified")]
        as_red: Option<bool>,
    },
}

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

pub fn begin(tournament: Arc<RwLock<Tournament>>, exec: Vec<String>) {
    for cmd_str in exec {
        let parts = cmd_str.split_whitespace();
        match Input::<Command>::try_parse_from(parts) {
            Ok(Input { command }) => execute_command(&tournament, command),
            Err(err) => {
                println!("Error parsing exec command '{}': {}", cmd_str, err);
            }
        }
    }
    begin_control(tournament);
}

fn begin_control(tournament: Arc<RwLock<Tournament>>) {
    loop {
        let command = read_input::<Command>();
        execute_command(&tournament, command);
    }
}

fn execute_command(tournament: &Arc<RwLock<Tournament>>, command: Command) {
    match command {
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
        Command::Enqueue {
            name,
            against,
            count,
            as_red,
        } => {
            let mut tournament = tournament.write().unwrap();
            let mut queue = tournament.enqueue(&name);

            for name in against {
                queue = queue.against(name, count, as_red);
            }
        }
    }
}

fn read_input<T: clap::FromArgMatches + clap::Subcommand>() -> T {
    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();

        let parts = line.split_whitespace();

        match Input::<T>::try_parse_from(parts) {
            Ok(Input { command }) => return command,
            Err(err) => {
                print!("{}", err);
                continue;
            }
        };
    }
}

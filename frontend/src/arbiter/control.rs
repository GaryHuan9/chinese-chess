use crate::arbiter::tournament::Tournament;
use clap::{CommandFactory, Parser, Subcommand};
use std::sync::{Arc, RwLock};

pub struct Control {
    tournament: Arc<RwLock<Tournament>>,
}

#[derive(Parser)]
#[command(name = "", no_binary_name = true)]
struct Commands {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Status,
    Enqueue,
    Play,
}

impl Control {
    pub fn new(tournament: Arc<RwLock<Tournament>>) -> Self {
        Self { tournament }
    }

    pub fn begin(&mut self) {
        loop {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();

            let line = line.trim().split_whitespace().collect::<Vec<_>>();
            if let Ok(Commands { command }) = Commands::try_parse_from(line) {
            } else {
                println!("{}", Commands::command().render_help());
            }
        }
    }
}

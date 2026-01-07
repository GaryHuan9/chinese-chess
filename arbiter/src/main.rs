use chinese_chess::game::Game;
use clap::Parser;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(short, long, default_value_t = 5000)]
    port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();

    let address = format!("127.0.0.1:{}", arguments.port);
    let listener = TcpListener::bind(address)?;

    for stream in listener.incoming() {
        if let Err(error) = handle_connection(stream) {
            println!("{}", error);
        }
    }

    Ok(())
}

struct Reader<'a> {
    reader: BufReader<&'a TcpStream>,
    buffer: String,
}

impl<'a> Reader<'a> {
    fn new(stream: &'a TcpStream) -> Self {
        Self {
            reader: BufReader::new(stream),
            buffer: String::new(),
        }
    }

    fn next(&mut self) -> Option<(&str, impl Iterator<Item = &str>)> {
        self.buffer.clear();
        if let Err(_) | Ok(0) = self.reader.read_line(&mut self.buffer) {
            return None;
        };

        let mut parts = self.buffer.trim().split_whitespace();
        let Some(command) = parts.next() else { return None };

        Some((command, parts))
    }
}

fn handle_connection(stream: std::io::Result<TcpStream>) -> Result<(), Box<dyn Error>> {
    let mut stream = &stream?;
    let mut reader = Reader::new(&stream);

    let name = {
        let Some(("login", mut parts)) = reader.next() else {
            return Err("expected login message".into());
        };

        match parts.next() {
            Some(version) if version.parse().is_ok_and(|version: i32| version == 1) => {}
            _ => return Err("expected version 1 in login message".into()),
        }

        let Some(name) = parts.next() else {
            return Err("expected name in login message".into());
        };

        name.to_string()
    };

    loop {
        let Some(("ready", _)) = reader.next() else {
            return Err("expected ready message".into());
        };

        let game = Game::opening();
        stream.write_fmt(format_args!("game {} {}\n", game.fen(), true))?;
        stream.write_fmt(format_args!("move {}\n", 1000))?;

        loop {
            let Some(("move", mut parts)) = reader.next() else {
                return Err("expected move message".into());
            };

            // let mv = match parts.next() {
            // Some(mv) if let Some(mv) = Move

            // }
        }
    }
}

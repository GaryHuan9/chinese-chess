use chinese_chess::game::Game;
use chinese_chess::location::Move;
use clap::Parser;
use frontend::protocol::{ProtocolReader, ProtocolWriter};
use std::error::Error;
use std::io;
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
            println!("error - client disconnected: {}", error);
        }
    }

    Ok(())
}

fn handle_connection(stream: io::Result<TcpStream>) -> Result<(), Box<dyn Error>> {
    let stream: &TcpStream = &stream?;
    let mut reader = ProtocolReader::new(stream);
    let mut writer = ProtocolWriter::new(stream);

    let name = {
        let Some(("init", mut parts)) = reader.next() else {
            return Err("expected init message".into());
        };

        if parts.next().is_none_or(|v| !v.parse().is_ok_and(|v: i32| v == 1)) {
            return Err("expected version 1 in init message".into());
        }

        let Some(name) = parts.next() else {
            return Err("expected name in init message".into());
        };

        name.to_string()
    };

    loop {
        while reader.next().ok_or("end of connection")?.0 != "ready" {}

        let mut game = Game::opening();
        writer.next("game", &format!("{} {}", game.fen(), true))?;

        loop {
            let moves = game.moves();

            if moves.is_empty() {
                writer.next("end", "")?;
                println!("black won");
                break;
            }

            let mv = loop {
                writer.next("prompt", "1000")?;

                let Some(("play", mut parts)) = reader.next() else {
                    return Err("expected play message".into());
                };

                let Some(mv) = parts.next().and_then(|mv| mv.parse().ok()) else {
                    return Err("invalid move format".into());
                };

                if moves.contains(&mv) {
                    break mv;
                }
            };

            game.play(mv);
            println!("{game}");
            writer.next("play", &mv.to_string())?;

            let moves = game.moves();

            if moves.is_empty() {
                writer.next("end", "")?;
                println!("red won");
                break;
            }

            let mv = loop {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_ascii_lowercase();

                if let Ok(mv) = input.parse::<Move>() {
                    if !moves.contains(&mv) {
                        println!("illegal move");
                        continue;
                    }

                    break mv;
                } else if input == "undo" {
                    // game.undo();
                    // game.undo();
                    //
                    // todo_fmt(format_args!("game {} {}\n", game.fen(), true))?;

                    todo!();
                } else {
                    println!("unknown input");
                }
            };

            game.play(mv);
            println!("{game}");
            writer.next("play", &mv.to_string())?;
        }
    }
}

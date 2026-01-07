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

    #[clap(long, default_value_t = true)]
    human: bool,
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

    {
        let Some(("init", mut parts)) = reader.next() else {
            return Err("expected init message".into());
        };

        if parts.next().is_none_or(|v| !v.parse().is_ok_and(|v: i32| v == 1)) {
            return Err("expected version 1 in init message".into());
        }
    }

    let _name = {
        let Some(("info", mut parts)) = reader.next() else {
            return Err("expected info message".into());
        };

        let Some(name) = parts.next() else {
            return Err("expected name in init message".into());
        };

        name.to_string()
    };

    loop {
        while reader.next().ok_or("end of connection")?.0 != "ready" {}

        let mut game = Game::opening();
        writer.next("game", &format!("{} {}", game.fen(), true))?;

        'game: loop {
            let moves = game.moves();

            if moves.is_empty() {
                writer.next("end", "")?;
                println!("black won");
                break 'game;
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
            writer.next("play", &mv.to_string())?;
            print!("{game}");

            let mut moves = game.moves();

            if moves.is_empty() {
                writer.next("end", "")?;
                println!("red won");
                break 'game;
            }

            let mv = loop {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim().to_ascii_lowercase();

                if let Ok(mv) = input.parse::<Move>() {
                    if moves.contains(&mv) {
                        break mv;
                    }
                    println!("illegal move");
                } else if input == "undo" {
                    game.undo();
                    game.undo();

                    writer.next("end", "")?;
                    while reader.next().ok_or("end of connection")?.0 != "ready" {}
                    writer.next("game", &format!("{} {}", game.fen(), true))?;
                    print!("{game}");

                    moves = game.moves();
                } else if input == "new" {
                    writer.next("end", "")?;
                    break 'game;
                } else {
                    println!("unknown input");
                }
            };

            game.play(mv);
            writer.next("play", &mv.to_string())?;
        }
    }
}

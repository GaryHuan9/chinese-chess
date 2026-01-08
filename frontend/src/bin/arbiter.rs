use chinese_chess::game::Game;
use chinese_chess::location::Move;
use clap::Parser;
use frontend::line_stream::AsyncLineStream;
use frontend::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use smol::LocalExecutor;
use std::error::Error;
use std::io;

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(short, long, default_value_t = 5000)]
    port: u16,

    #[clap(long, default_value_t = true)]
    human: bool,
}

fn main() {
    let executor = LocalExecutor::new();

    smol::block_on(executor.run(async {
        let arguments = Arguments::parse();
        let address = format!("127.0.0.1:{}", arguments.port);
        let listener = smol::net::TcpListener::bind(address).await.unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();
            executor.spawn(handle_connection(stream)).detach();
        }
    }));
}

async fn handle_connection(stream: smol::net::TcpStream) {
    let stream = AsyncLineStream::new(&stream);
    if let Err(err) = serve_connection(stream).await {
        println!("connection ended - {err}")
    }
}

async fn serve_connection(stream: AsyncLineStream) -> Result<(), Box<dyn Error>> {
    let read = async || Protocol::decode_player(&stream.read_line().await?);
    let write = |message| stream.write_line(Protocol::encode_arbiter(message));

    let Some(PlayerMessage::Init { version: 1 }) = read().await else {
        return Err("expected init message with version 1".into());
    };

    let Some(PlayerMessage::Info { name: _name }) = read().await else {
        return Err("expected info message".into());
    };

    loop {
        let mut game = Game::opening();

        write(ArbiterMessage::Game {
            fen: game.fen(),
            red_turn: game.red_turn(),
        })
        .await?;

        while !matches!(read().await.ok_or("end of connection")?, PlayerMessage::Ready) {}

        'game: loop {
            let moves = game.moves();

            if moves.is_empty() {
                println!("black won");
                break 'game;
            }

            let mv = loop {
                write(ArbiterMessage::Prompt { time: 1000 }).await?;

                let Some(PlayerMessage::Play { mv }) = read().await else {
                    return Err("expected play message".into());
                };

                if moves.contains(&mv) {
                    break mv;
                }
            };

            game.play(mv);
            write(ArbiterMessage::Update { mv }).await?;
            print!("{game}");

            let mut moves = game.moves();

            if moves.is_empty() {
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

                    write(ArbiterMessage::Game {
                        fen: game.fen(),
                        red_turn: game.red_turn(),
                    })
                    .await?;

                    while !matches!(read().await.ok_or("end of connection")?, PlayerMessage::Ready) {}
                    print!("{game}");

                    moves = game.moves();
                } else if input == "new" {
                    break 'game;
                } else {
                    println!("unknown input");
                }
            };

            game.play(mv);
            write(ArbiterMessage::Update { mv }).await?;
        }
    }
}

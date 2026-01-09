use chinese_chess::game::Game;
use clap::Parser;
use frontend::line_stream::LineStream;
use frontend::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use rand::Rng;
use std::error::Error;
use std::net::{IpAddr, SocketAddr, TcpStream};

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(short, long, default_value = "127.0.0.1")]
    ip: IpAddr,

    #[clap(short, long, default_value_t = 5000)]
    port: u16,

    #[clap(short, long, default_value = "robot")]
    name: String,

    #[clap(short, long, default_value_t = true)]
    random: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();

    let stream = TcpStream::connect(SocketAddr::new(arguments.ip, arguments.port))?;
    let stream = LineStream::new(&stream);

    let read = || Protocol::decode_arbiter(&stream.read_line()?);
    let write = |message| stream.write_line(Protocol::encode_player(&message));

    let mut random = rand::rng();

    write(PlayerMessage::Init { version: 1 })?;
    write(PlayerMessage::Info {
        name: arguments.name.clone(),
    })?;

    let mut game = None;

    loop {
        match read().ok_or("unexpected message")? {
            ArbiterMessage::Game { fen, red_turn } => {
                game = Some(Game::from_fen(&fen, red_turn).unwrap());
                write(PlayerMessage::Ready)?;
            }
            ArbiterMessage::Prompt { time: _time } => {
                let mut moves = {
                    let game = game.as_mut().unwrap();
                    print!("{game}");
                    game.moves_ranked()
                };

                if moves.is_empty() {
                    game = None;
                    continue;
                }

                let mv = if arguments.random {
                    moves[random.random_range(0..moves.len())].0
                } else {
                    moves.sort_by_key(|&(_, v)| -v);

                    for &(mv, value) in &moves {
                        println!("{} - {}", mv, value);
                    }

                    moves.first().unwrap().0
                };

                write(PlayerMessage::Play { mv })?;
            }
            ArbiterMessage::Update { mv } => {
                let played = game.as_mut().unwrap().play(mv);
                assert!(played);
            }
        }
    }
}

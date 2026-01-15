use chinese_chess::display_format::DisplayFormat;
use chinese_chess::game::Game;
use clap::Parser;
use frontend::line_stream::LineStream;
use frontend::protocol::{ArbiterMessage, PlayerMessage};
use std::error::Error;
use std::net::{IpAddr, SocketAddr, TcpStream};

#[derive(Parser, Debug)]
struct Arguments {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: IpAddr,

    #[arg(short, long, default_value_t = 5000)]
    port: u16,

    #[arg(short, long, default_value = "robot")]
    name: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();

    let address = SocketAddr::new(arguments.ip, arguments.port);
    let stream = LineStream::new(TcpStream::connect(address)?);

    stream.write(&PlayerMessage::Init { version: 1 })?;
    stream.write(&PlayerMessage::Info {
        name: arguments.name.clone(),
    })?;

    let mut game = Game::opening();

    loop {
        match stream.read()? {
            ArbiterMessage::Game { fen, red_turn } => {
                game = Game::from_fen(&fen, red_turn).unwrap();
                stream.write(&PlayerMessage::Ready)?;
            }
            ArbiterMessage::Prompt { time: _time } => {
                println!("{}", game.display(DisplayFormat::pretty()));

                let mut moves = game.moves_ranked();

                moves.sort_by_key(|&(_, v, _)| -v);

                for &(mv, value, count) in &moves {
                    println!("{} - ({count}) - {}", mv, value);
                }

                println!("{}", moves.iter().map(|&(_, _, c)| c).sum::<u32>());

                let mv = moves.first().unwrap().0;

                stream.write(&PlayerMessage::Play { mv })?;
            }
            ArbiterMessage::Update { mv } => {
                println!("arbiter update {mv}");
                let played = game.play(mv);
                assert!(played);

                if game.outcome().is_some() {
                    println!("{}", game.display(DisplayFormat::pretty()));
                }
            }
        }
    }
}

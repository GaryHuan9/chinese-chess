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

                let mut ranker = game.ranker();
                let depth = 3;

                ranker.rank_recursive(depth);

                let before_rank = ranker.display(DisplayFormat::pretty()).to_string();

                ranker.rank(depth);

                let after_rank = ranker.display(DisplayFormat::pretty()).to_string();

                let before_lines: Vec<&str> = before_rank.lines().collect();
                let after_lines: Vec<&str> = after_rank.lines().collect();
                let max_lines = before_lines.len().max(after_lines.len());
                let max_width = before_lines.iter().map(|s| s.len()).max().unwrap_or(0);

                for i in 0..max_lines {
                    let left = before_lines.get(i).unwrap_or(&"");
                    let right = after_lines.get(i).unwrap_or(&"");
                    println!("{:<width$}  |  {}", left, right, width = max_width);
                }

                stream.write(&PlayerMessage::Play { mv: ranker.best() })?;
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

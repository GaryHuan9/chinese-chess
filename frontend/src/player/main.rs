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

                let depth = 4;

                // let mut ranker = game.ranker();
                // ranker.rank_recursive(depth);
                // let recursive_rank = ranker.display(DisplayFormat::pretty()).to_string();

                let start = std::time::Instant::now();
                let mut ranker = game.ranker();
                ranker.rank(depth);
                let elapsed = start.elapsed().as_secs_f32();
                println!("{}", ranker.display(DisplayFormat::pretty()));
                println!("Time taken: {} ms", elapsed * 1000.0);

                // let iterative_rank = ranker.display(DisplayFormat::pretty()).to_string();
                //
                // let recursive_lines: Vec<&str> = recursive_rank.lines().collect();
                // let iterative_lines: Vec<&str> = iterative_rank.lines().collect();
                // let max_lines = recursive_lines.len().max(iterative_lines.len());
                // let recursive_width = recursive_lines.iter().map(|s| s.len()).max().unwrap_or(0);
                //
                // for i in 0..max_lines {
                //     let left = recursive_lines.get(i).unwrap_or(&"");
                //     let right = iterative_lines.get(i).unwrap_or(&"");
                //     let diff_marker = if left == right { "" } else { " DIFFERENT" };
                //     println!("{:<width$}  |  {}{}", left, right, diff_marker, width = recursive_width);
                // }
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

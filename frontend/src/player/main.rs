use chinese_chess::display_format::DisplayFormat;
use chinese_chess::game::Game;
use chinese_chess::ranker::Ranker;
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
    let fen = "1reakae2/7r1/9/p1p1p1p1p/1c6P/9/P1P1P1Pc1/2H3H2/9/R1EAKAE1R";
    let game = Game::from_fen(fen, false).unwrap();
    println!("{}", game.display(DisplayFormat::pretty()));

    let mut ranker = Ranker::new(game);
    ranker.rank(3);
    println!("{}", ranker.display(DisplayFormat::pretty()));

    return Ok(());

    let arguments = Arguments::parse();

    let address = SocketAddr::new(arguments.ip, arguments.port);
    let stream = LineStream::new(TcpStream::connect(address)?);

    stream.write(&PlayerMessage::Init { version: 1 })?;
    stream.write(&PlayerMessage::Info {
        name: arguments.name.clone(),
    })?;

    let mut ranker = Ranker::new(Game::opening());

    loop {
        match stream.read()? {
            ArbiterMessage::Game { fen, red_turn } => {
                let game = Game::from_fen(&fen, red_turn).unwrap();
                ranker = Ranker::new(game);
                stream.write(&PlayerMessage::Ready)?;
            }
            ArbiterMessage::Prompt { time: _time } => {
                println!("{}", ranker.game().display(DisplayFormat::pretty()));

                let depth = 3;

                let start = std::time::Instant::now();
                ranker.rank(depth);
                let elapsed = start.elapsed().as_secs_f32();
                println!("{}", ranker.display(DisplayFormat::pretty()));

                // const SHOW_SIMPLE: bool = false;
                // let iterative_rank = ranker.display(DisplayFormat::pretty()).to_string();
                //
                // let recursive_rank = {
                //     let mut ranker = Ranker::new(ranker.game().clone());
                //     ranker.rank_recursive(depth);
                //     ranker.display(DisplayFormat::pretty()).to_string()
                // };
                //
                // let simple_rank = if SHOW_SIMPLE {
                //     let mut ranker = Ranker::new(ranker.game().clone());
                //     ranker.rank_simple(depth);
                //     ranker.display(DisplayFormat::pretty()).to_string()
                // } else {
                //     String::new()
                // };
                //
                // let recursive_lines: Vec<&str> = recursive_rank.lines().collect();
                // let iterative_lines: Vec<&str> = iterative_rank.lines().collect();
                // let simple_lines: Vec<&str> = if SHOW_SIMPLE {
                //     simple_rank.lines().collect()
                // } else {
                //     vec![]
                // };
                // let max_lines = if SHOW_SIMPLE {
                //     recursive_lines.len().max(iterative_lines.len()).max(simple_lines.len())
                // } else {
                //     recursive_lines.len().max(iterative_lines.len())
                // };
                // let recursive_width = recursive_lines.iter().map(|s| s.len()).max().unwrap_or(0);
                // let iterative_width = iterative_lines.iter().map(|s| s.len()).max().unwrap_or(0);
                //
                // if SHOW_SIMPLE {
                //     println!(
                //         "{:<recursive_width$}  |  {:<iterative_width$}  |  {}",
                //         "RECURSIVE",
                //         "ITERATIVE",
                //         "SIMPLE",
                //         recursive_width = recursive_width,
                //         iterative_width = iterative_width
                //     );
                //     println!(
                //         "{:<recursive_width$}  |  {:<iterative_width$}  |  {}",
                //         "-".repeat(recursive_width),
                //         "-".repeat(iterative_width),
                //         "-".repeat(simple_lines.iter().map(|s| s.len()).max().unwrap_or(0)),
                //         recursive_width = recursive_width,
                //         iterative_width = iterative_width
                //     );
                // } else {
                //     println!(
                //         "{:<recursive_width$}  |  {}",
                //         "RECURSIVE",
                //         "ITERATIVE",
                //         recursive_width = recursive_width
                //     );
                //     println!(
                //         "{:<recursive_width$}  |  {}",
                //         "-".repeat(recursive_width),
                //         "-".repeat(iterative_width),
                //         recursive_width = recursive_width
                //     );
                // }
                //
                // for i in 0..max_lines {
                //     let left = recursive_lines.get(i).unwrap_or(&"");
                //     let right = iterative_lines.get(i).unwrap_or(&"");
                //     if SHOW_SIMPLE {
                //         let simple = simple_lines.get(i).unwrap_or(&"");
                //         let diff_marker = if left == right && left == simple {
                //             ""
                //         } else {
                //             " DIFFERENT"
                //         };
                //         println!(
                //             "{:<recursive_width$}  |  {:<iterative_width$}  |  {}{}",
                //             left, right, simple, diff_marker
                //         );
                //     } else {
                //         let diff_marker = if left == right { "" } else { " DIFFERENT" };
                //         println!("{:<recursive_width$}  |  {}{}", left, right, diff_marker);
                //     }
                // }
                println!("Time taken: {} ms", elapsed * 1000.0);
                stream.write(&PlayerMessage::Play { mv: ranker.best() })?;
            }
            ArbiterMessage::Update { mv } => {
                println!("arbiter update {mv}");
                ranker.make_move(mv);

                if ranker.game().outcome().is_some() {
                    println!("{}", ranker.game().display(DisplayFormat::pretty()));
                }
            }
        }
    }
}

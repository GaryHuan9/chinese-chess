use chinese_chess::display_format::DisplayFormat;
use chinese_chess::game::Game;
use chinese_chess::ranker::Ranker;
use clap::Parser;
use frontend::line_stream::LineStream;
use frontend::protocol::PlayerMessage;
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

    #[arg(short, long, default_value_t = 4)]
    depth: u32,
}

fn display_difference(ranker: &Ranker, reference: &Ranker) {
    let lines0 = ranker.display(DisplayFormat::pretty()).to_string();
    let lines1 = reference.display(DisplayFormat::pretty()).to_string();
    let lines = lines0
        .lines()
        .zip(lines1.lines())
        .filter_map(|(l0, l1)| if l0 == l1 { None } else { Some(format!("{l0}\n{l1}\n")) });
    let lines = lines.collect::<Box<[_]>>();
    if lines.is_empty() {
        println!("no difference");
    } else {
        println!("{}", lines.join("\n"));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut game = Game::from_fen("CRH1k1e2/3ca4/4ea3/9/2hr5/9/9/4E4/4A4/4KA3", true).unwrap();
    game.make_move("c9b7".parse()?);
    game.make_move("e8d9".parse()?);
    game.make_move("b9d9".parse()?);
    // game.make_move("e9e8".parse()?);
    // game.make_move("a9a8".parse()?);
    // game.make_move("d8d7".parse()?);
    // game.make_move("b7d8".parse()?);
    // let mut game = Game::from_fen("R1H1k1e2/9/3aea3/9/2hr5/2E6/9/4E4/4A4/4KA3", false).unwrap();
    // game.make_move("c9b7".parse()?);
    // game.make_move("e9e8".parse()?);
    // game.make_move("a9a8".parse()?);
    println!("{}", game.display(DisplayFormat::pretty()));

    let mut ranker = Ranker::new(game.clone());
    let depth = 4;

    ranker.rank_simple(depth);
    println!("{}", ranker.display(DisplayFormat::pretty()));

    {
        let mut reference = Ranker::new(game.clone());
        reference.rank_recursive(depth);
        display_difference(&ranker, &reference);
    }

    return Ok(());
    let arguments = Arguments::parse();

    let address = SocketAddr::new(arguments.ip, arguments.port);
    let stream = LineStream::new(TcpStream::connect(address)?);

    stream.write(&PlayerMessage::Init { version: 1 })?;
    stream.write(&PlayerMessage::Info {
        name: arguments.name.clone(),
    })?;

    // let mut ranker = Ranker::new(Game::opening());
    //
    // loop {
    //     match stream.read()? {
    //         ArbiterMessage::Game { fen, red_turn } => {
    //             let game = Game::from_fen(&fen, red_turn).unwrap();
    //             ranker = Ranker::new(game);
    //             stream.write(&PlayerMessage::Ready)?;
    //         }
    //         ArbiterMessage::Prompt { time: _time } => {
    //             // println!("{}", ranker.game().display(DisplayFormat::pretty()));
    //
    //             let depth = arguments.depth;
    //
    //             let start = std::time::Instant::now();
    //             ranker.rank(depth);
    //             let elapsed = start.elapsed().as_secs_f32();
    //             println!("{}", ranker.display(DisplayFormat::pretty()));
    //
    //             // const SHOW_SIMPLE: bool = false;
    //             // let iterative_rank = ranker.display(DisplayFormat::pretty()).to_string();
    //             //
    //             // let recursive_rank = {
    //             //     let mut ranker = Ranker::new(ranker.game().clone());
    //             //     ranker.rank_recursive(depth);
    //             //     ranker.display(DisplayFormat::pretty()).to_string()
    //             // };
    //             //
    //             // let simple_rank = if SHOW_SIMPLE {
    //             //     let mut ranker = Ranker::new(ranker.game().clone());
    //             //     ranker.rank_simple(depth);
    //             //     ranker.display(DisplayFormat::pretty()).to_string()
    //             // } else {
    //             //     String::new()
    //             // };
    //             //
    //             // let recursive_lines: Vec<&str> = recursive_rank.lines().collect();
    //             // let iterative_lines: Vec<&str> = iterative_rank.lines().collect();
    //             // let simple_lines: Vec<&str> = if SHOW_SIMPLE {
    //             //     simple_rank.lines().collect()
    //             // } else {
    //             //     vec![]
    //             // };
    //             // let max_lines = if SHOW_SIMPLE {
    //             //     recursive_lines.len().max(iterative_lines.len()).max(simple_lines.len())
    //             // } else {
    //             //     recursive_lines.len().max(iterative_lines.len())
    //             // };
    //             // let recursive_width = recursive_lines.iter().map(|s| s.len()).max().unwrap_or(0);
    //             // let iterative_width = iterative_lines.iter().map(|s| s.len()).max().unwrap_or(0);
    //             //
    //             // if SHOW_SIMPLE {
    //             //     println!(
    //             //         "{:<recursive_width$}  |  {:<iterative_width$}  |  {}",
    //             //         "RECURSIVE",
    //             //         "ITERATIVE",
    //             //         "SIMPLE",
    //             //         recursive_width = recursive_width,
    //             //         iterative_width = iterative_width
    //             //     );
    //             //     println!(
    //             //         "{:<recursive_width$}  |  {:<iterative_width$}  |  {}",
    //             //         "-".repeat(recursive_width),
    //             //         "-".repeat(iterative_width),
    //             //         "-".repeat(simple_lines.iter().map(|s| s.len()).max().unwrap_or(0)),
    //             //         recursive_width = recursive_width,
    //             //         iterative_width = iterative_width
    //             //     );
    //             // } else {
    //             //     println!(
    //             //         "{:<recursive_width$}  |  {}",
    //             //         "RECURSIVE",
    //             //         "ITERATIVE",
    //             //         recursive_width = recursive_width
    //             //     );
    //             //     println!(
    //             //         "{:<recursive_width$}  |  {}",
    //             //         "-".repeat(recursive_width),
    //             //         "-".repeat(iterative_width),
    //             //         recursive_width = recursive_width
    //             //     );
    //             // }
    //             //
    //             // for i in 0..max_lines {
    //             //     let left = recursive_lines.get(i).unwrap_or(&"");
    //             //     let right = iterative_lines.get(i).unwrap_or(&"");
    //             //     if SHOW_SIMPLE {
    //             //         let simple = simple_lines.get(i).unwrap_or(&"");
    //             //         let diff_marker = if left == right && left == simple {
    //             //             ""
    //             //         } else {
    //             //             " DIFFERENT"
    //             //         };
    //             //         println!(
    //             //             "{:<recursive_width$}  |  {:<iterative_width$}  |  {}{}",
    //             //             left, right, simple, diff_marker
    //             //         );
    //             //     } else {
    //             //         let diff_marker = if left == right { "" } else { " DIFFERENT" };
    //             //         println!("{:<recursive_width$}  |  {}{}", left, right, diff_marker);
    //             //     }
    //             // }
    //             println!("Time taken: {} ms", elapsed * 1000.0);
    //             stream.write(&PlayerMessage::Play { mv: ranker.best() })?;
    //         }
    //         ArbiterMessage::Update { mv } => {
    //             println!("arbiter update {mv}");
    //             ranker.make_move(mv);
    //
    //             if ranker.game().outcome().is_some() {
    //                 println!("{}", ranker.game().display(DisplayFormat::pretty()));
    //             }
    //         }
    //     }
    // }
}

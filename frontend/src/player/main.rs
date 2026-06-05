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
        println!("differences:");
        println!("{}", lines.join("\n"));
    }
}

fn test() -> Result<(), Box<dyn Error>> {
    let game = Game::opening();
    let mut ranker = Ranker::new(game);

    for _ in 0..4 {
        ranker.deeper();
        println!("{}", ranker.display(DisplayFormat::pretty()));
    }

    // let mut game = Game::from_fen("CRH1k1e2/3ca4/4ea3/9/2hr5/9/9/4E4/4A4/4KA3", true).unwrap();
    // game.make_move("c9b7".parse()?);
    // game.make_move("e8d9".parse()?);
    // game.make_move("b9d9".parse()?);
    // game.make_move("e9e8".parse()?);
    // // game.make_move("a9a8".parse()?);
    // // game.make_move("d8d7".parse()?);
    // // game.make_move("b7d8".parse()?);
    // // let mut game = Game::from_fen("R1H1k1e2/9/3aea3/9/2hr5/2E6/9/4E4/4A4/4KA3", true).unwrap();
    // // game.make_move("c9b7".parse()?);
    // // game.make_move("e9e8".parse()?);
    // // game.make_move("a9a8".parse()?);
    // println!("{}", game.display(DisplayFormat::pretty()));
    //
    // let depth = 5;
    //
    // let mut ranker = Ranker::new(game.clone());
    // ranker.rank(depth);
    // println!("{}", ranker.display(DisplayFormat::pretty()));
    //
    // {
    //     let mut reference = Ranker::new(game.clone());
    //     reference.simple(depth);
    //     println!("{}", reference.display(DisplayFormat::pretty()));
    //     display_difference(&ranker, &reference);
    // }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    return test();

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
            ArbiterMessage::Prompt { time } => {
                println!("{}", ranker.game().display(DisplayFormat::pretty()));
                println!("{time}ms thinking time");

                let start = std::time::Instant::now();
                let time = std::time::Duration::from_millis(time as u64);

                loop {
                    ranker.deeper();

                    if let Some(remain) = time.checked_sub(start.elapsed()) {
                        println!(
                            "({}) {}ms thinking time: {}",
                            ranker.depth(),
                            remain.as_millis(),
                            ranker.display(DisplayFormat::string())
                        );
                    } else {
                        break;
                    }
                }

                let duration = start.elapsed();
                println!("{}", ranker.display(DisplayFormat::pretty()));
                println!("total {}ms thinking time", duration.as_millis());

                if let Some(best) = ranker.best() {
                    stream.write(&PlayerMessage::Play { mv: best })?;
                } else {
                    println!("no viable move")
                }
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

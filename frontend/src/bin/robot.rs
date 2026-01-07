use chinese_chess::game::Game;
use clap::Parser;
use frontend::protocol::{ProtocolReader, ProtocolWriter};
use rand::Rng;
use std::error::Error;
use std::net::{IpAddr, SocketAddr, TcpStream};

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(short, long, default_value = "127.0.0.1")]
    ip: IpAddr,

    #[clap(short, long, default_value_t = 5000)]
    port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = Arguments::parse();

    let stream = TcpStream::connect(SocketAddr::new(arguments.ip, arguments.port))?;
    let mut reader = ProtocolReader::new(&stream);
    let mut writer = ProtocolWriter::new(&stream);

    let mut random = rand::rng();

    writer.next("init", "1 robot")?;

    loop {
        writer.next("ready", "")?;

        let Some(mut game) = ({
            let Some(("game", mut parts)) = reader.next() else {
                return Err("expect game message".into());
            };

            let fen = parts.next();
            let red_turn = parts.next().and_then(|red_turn| red_turn.parse().ok());
            fen.zip(red_turn).and_then(|(fen, turn)| Game::from_fen(fen, turn))
        }) else {
            return Err("invalid game format".into());
        };

        loop {
            let Some((message, mut parts)) = reader.next() else {
                return Err("end of connection".into());
            };

            match message {
                "prompt" => {
                    let moves = game.moves();
                    if moves.is_empty() {
                        break;
                    }

                    let index = random.random_range(0..moves.len());
                    let mv = moves[index];
                    writer.next("play", &mv.to_string())?;
                }
                "play" => {
                    let Some(mv) = parts.next().and_then(|mv| mv.parse().ok()) else {
                        return Err("invalid move format".into());
                    };

                    game.play(mv);
                }
                "end" => break,
                _ => return Err("unexpected message".into()),
            }
        }
    }
}

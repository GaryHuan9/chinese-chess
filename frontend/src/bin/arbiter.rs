use clap::Parser;
use frontend::line_stream::AsyncLineStream;
use frontend::protocol::{PlayerMessage, Protocol};
use frontend::tournament::Tournament;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
struct Arguments {
    #[clap(short, long, default_value_t = 5000)]
    port: u16,

    #[clap(long, default_value_t = true)]
    human: bool,
}

fn main() {
    let arguments = Arguments::parse();
    let address = format!("127.0.0.1:{}", arguments.port);
    let tournament: Arc<Mutex<Tournament>> = Tournament::new();

    smol::block_on(async {
        let listener = smol::net::TcpListener::bind(address).await.unwrap();

        loop {
            let (stream, _) = listener.accept().await.unwrap();
            smol::spawn(connect(tournament.clone(), stream)).detach();
        }
    });
}

async fn connect(tournament: Arc<Mutex<Tournament>>, stream: smol::net::TcpStream) {
    let stream = AsyncLineStream::new(stream);
    let _ = connect_impl(tournament, stream).await;
}

async fn connect_impl(
    tournament: Arc<Mutex<Tournament>>,
    stream: AsyncLineStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let read = async || Protocol::decode_player(&stream.read_line().await?);

    let Some(PlayerMessage::Init { version: 1 }) = read().await else {
        return Err("expected init message with version 1".into());
    };

    let Some(PlayerMessage::Info { name }) = read().await else {
        return Err("expected info message".into());
    };

    let mut tournament = tournament.lock().map_err(|_| "poisoned")?;
    tournament.join(name, stream);
    Ok(())
}

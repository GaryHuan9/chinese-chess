use clap::Parser;
use env_logger::Target;
use frontend::line_stream::AsyncLineStream;
use frontend::protocol::{PlayerMessage, Protocol};
use frontend::tournament::Tournament;
use log::{info, warn, LevelFilter};
use smol::net::TcpStream as AsyncTcpStream;
use std::net::SocketAddr;
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

    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(LevelFilter::max()).target(Target::Stdout).init();

    let address = format!("127.0.0.1:{}", arguments.port);
    let tournament: Arc<Mutex<Tournament>> = Tournament::new();

    smol::block_on(async {
        let listener = smol::net::TcpListener::bind(&address).await.unwrap();
        info!("server listening at {address}");

        loop {
            let (stream, address) = listener.accept().await.unwrap();
            info!("received incoming connection from {address}");
            smol::spawn(connect(tournament.clone(), stream, address)).detach();
        }
    });
}

async fn connect(tournament: Arc<Mutex<Tournament>>, stream: AsyncTcpStream, address: SocketAddr) {
    let stream = AsyncLineStream::new(stream);
    if let Err(err) = initialize_connection(tournament, stream).await {
        warn!("connection from {address} closed with error {err}");
    }
}

async fn initialize_connection(
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

    info!("connection initialized as instance for player '{name}'");
    let mut tournament = tournament.lock().map_err(|_| "tournament poisoned")?;
    tournament.join(name, stream);
    Ok(())
}

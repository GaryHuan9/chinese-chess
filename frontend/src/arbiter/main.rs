use chrono::Local;
use clap::Parser;
use env_logger::Target;
use frontend::arbiter::control::Control;
use frontend::arbiter::tournament::Tournament;
use frontend::line_stream::AsyncLineStream;
use frontend::protocol::{PlayerMessage, Protocol};
use log::{info, warn, LevelFilter};
use smol::net::TcpStream as AsyncTcpStream;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Parser)]
struct Arguments {
    #[clap(short, long, default_value_t = 5000)]
    port: u16,
}

fn main() {
    let arguments = Arguments::parse();

    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("log.txt")
        .unwrap();

    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        // .filter_level(LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "{style}[{}] [{:5}]{style:#} {}",
                Local::now().format("%T%.3f"),
                record.level(),
                record.args(),
                style = buf.default_level_style(record.level()),
            )
        })
        .target(Target::Pipe(Box::new(std::io::BufWriter::new(file))))
        .target(Target::Stderr)
        .init();

    let address = format!("127.0.0.1:{}", arguments.port);
    let tournament: Arc<RwLock<Tournament>> = Tournament::new();

    let mut control = Control::new(tournament.clone());
    thread::spawn(move || control.begin());

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

async fn connect(tournament: Arc<RwLock<Tournament>>, stream: AsyncTcpStream, address: SocketAddr) {
    let stream = AsyncLineStream::new(stream);
    if let Err(err) = initialize_connection(tournament, stream).await {
        warn!("connection from {address} closed with error {err}");
    }
}

async fn initialize_connection(
    tournament: Arc<RwLock<Tournament>>,
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
    let mut tournament = tournament.write().map_err(|_| "tournament poisoned")?;
    tournament.join(name, stream);
    Ok(())
}

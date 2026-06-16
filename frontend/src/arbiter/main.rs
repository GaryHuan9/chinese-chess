use chrono::Local;
use clap::error::ErrorKind;
use clap::Parser;
use env_logger::Target;
use frontend::arbiter::control;
use frontend::arbiter::tournament::Tournament;
use frontend::line_stream::AsyncLineStream;
use frontend::protocol::{PlayerMessage, Protocol};
use log::{info, warn, LevelFilter};
use rustyline::ExternalPrinter;
use smol::net::TcpStream as AsyncTcpStream;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Parser)]
struct Arguments {
    #[clap(short, long, default_value_t = 6000)]
    port: u16,

    #[clap(short, long, help = "Commands to execute on startup")]
    exec: Vec<String>,

    #[clap(long, default_value_t = LevelFilter::Trace)]
    log: LevelFilter,

    #[clap(long)]
    log_file: Option<String>,
}

fn main() {
    let arguments = Arguments::parse();
    let console = setup_console(&arguments);
    let tournament: Arc<RwLock<Tournament>> = Tournament::new();

    {
        let tournament = tournament.clone();
        thread::spawn(move || control::begin(tournament, &arguments.exec, console));
    }

    smol::block_on(async {
        let address = format!("127.0.0.1:{}", arguments.port);
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
    tournament.join(&name, stream);
    Ok(())
}

fn setup_console(arguments: &Arguments) -> rustyline::DefaultEditor {
    let mut console = rustyline::DefaultEditor::new().unwrap();

    let target = if let Some(path) = &arguments.log_file {
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .expect("Failed to open log file");
        Target::Pipe(Box::new(file))
    } else {
        struct Writer<T: ExternalPrinter> {
            printer: T,
        }

        impl<T: ExternalPrinter> Write for Writer<T> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let line = String::from_utf8_lossy(buf).into_owned();
                self.printer
                    .print(line)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let printer = console.create_external_printer().unwrap();
        Target::Pipe(Box::new(Writer { printer }))
    };

    env_logger::Builder::from_default_env()
        .filter_level(arguments.log)
        .filter_module("rustyline", LevelFilter::Warn)
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
        .target(target)
        .init();

    console
}

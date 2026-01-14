use crate::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use smol::io::AsyncReadExt;
use smol::io::AsyncWriteExt;
use smol::net::TcpStream as AsyncTcpStream;
use std::cell::RefCell;
use std::io::{BufRead, Read};
use std::io::{BufReader, Write};
use std::net::TcpStream;
use std::rc::Rc;

pub struct LineStream {
    stream: RcStream,
    reader: RefCell<BufReader<RcStream>>,
}

struct RcStream {
    stream: Rc<TcpStream>,
}

impl LineStream {
    pub fn new(stream: TcpStream) -> Self {
        let stream = Rc::new(stream);
        Self {
            stream: RcStream { stream: stream.clone() },
            reader: RefCell::new(BufReader::new(RcStream { stream })),
        }
    }

    pub fn read_line(&self) -> Option<String> {
        let mut line = String::new();
        loop {
            if let Err(_) | Ok(0) = self.reader.borrow_mut().read_line(&mut line) {
                return None;
            }

            let result = line.trim();
            if !result.is_empty() {
                return Some(result.to_string());
            }
            line.clear();
        }
    }

    pub fn write_line(&self, mut line: String) -> Result<(), std::io::Error> {
        let mut stream: &TcpStream = &self.stream.stream;
        line.push('\n');
        stream.write(line.as_bytes()).map(|_| ())
    }

    pub fn read(&self) -> Result<ArbiterMessage, std::io::Error> {
        self.read_line()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed",
            ))
            .and_then(|line| {
                Protocol::decode_arbiter(&line)
                    .ok_or(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid message"))
            })
    }

    pub fn write(&self, message: &PlayerMessage) -> Result<(), std::io::Error> {
        self.write_line(Protocol::encode_player(message))
    }
}

impl Read for RcStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.as_ref().read(buf)
    }
}

pub struct AsyncLineStream {
    stream: AsyncTcpStream,
}

impl AsyncLineStream {
    pub const MAX_LENGTH: u32 = 100;

    pub fn new(inner: AsyncTcpStream) -> Self {
        Self { stream: inner }
    }

    pub async fn read_line(&self) -> Option<String> {
        let mut inner = self.stream.clone();
        let mut line = String::new();

        loop {
            let mut buffer = [0u8; 1];
            if let Err(_) | Ok(0) = inner.read(&mut buffer).await {
                return None;
            }

            let char = buffer[0] as char;

            if char.is_ascii_whitespace() {
                if line.is_empty() {
                    continue;
                }
                if char == '\n' {
                    while line.ends_with(' ') {
                        line.pop();
                    }
                    return Some(line);
                }
                line.push(' ');
            } else if char.is_ascii_graphic() {
                line.push(char);
            }
        }
    }

    pub async fn write_line(&self, mut line: String) -> Result<(), std::io::Error> {
        line.push('\n');
        let mut inner = self.stream.clone();
        inner.write(line.as_bytes()).await.map(|_| ())
    }
}

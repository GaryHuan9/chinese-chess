use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::TcpStream as AsyncTcpStream;
use std::cell::RefCell;
use std::io::BufRead;
use std::io::{BufReader, Write};
use std::net::TcpStream;

pub struct LineStream<'a> {
    inner: &'a TcpStream,
    reader: RefCell<BufReader<&'a TcpStream>>,
}

impl<'a> LineStream<'a> {
    pub fn new(inner: &'a TcpStream) -> Self {
        Self {
            inner,
            reader: RefCell::new(BufReader::new(inner)),
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
        let mut inner = self.inner;
        line.push('\n');
        inner.write(line.as_bytes()).map(|_| ())
    }
}

pub struct AsyncLineStream {
    inner: AsyncTcpStream,
}

impl AsyncLineStream {
    pub const MAX_LENGTH: u32 = 100;

    pub fn new(inner: AsyncTcpStream) -> Self {
        Self { inner }
    }

    pub async fn read_line(&self) -> Option<String> {
        let mut inner = self.inner.clone();
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
        let mut inner = self.inner.clone();
        inner.write(line.as_bytes()).await.map(|_| ())
    }
}

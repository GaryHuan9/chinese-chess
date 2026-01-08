use smol::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
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
    inner: RefCell<AsyncTcpStream>,
    reader: RefCell<AsyncBufReader<AsyncTcpStream>>,
}

impl AsyncLineStream {
    pub fn new(inner: &AsyncTcpStream) -> Self {
        Self {
            inner: RefCell::new(inner.clone()),
            reader: RefCell::new(AsyncBufReader::new(inner.clone())),
        }
    }

    pub async fn read_line(&self) -> Option<String> {
        let mut line = String::new();
        loop {
            if let Err(_) | Ok(0) = self.reader.borrow_mut().read_line(&mut line).await {
                return None;
            }

            let result = line.trim();
            if !result.is_empty() {
                return Some(result.to_string());
            }
            line.clear();
        }
    }

    pub async fn write_line(&self, mut line: String) -> Result<(), std::io::Error> {
        line.push('\n');
        self.inner.borrow_mut().write(line.as_bytes()).await.map(|_| ())
    }
}

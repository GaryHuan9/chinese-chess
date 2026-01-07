use std::io::{BufRead, BufReader, Read, Write};

pub struct ProtocolReader<T: Read> {
    reader: BufReader<T>,
    line: String,
}

pub struct ProtocolWriter<T: Write> {
    writer: T,
}

impl<T: Read> ProtocolReader<T> {
    pub fn new(read: T) -> Self {
        Self {
            reader: BufReader::new(read),
            line: String::new(),
        }
    }

    pub fn next(&mut self) -> Option<(&str, impl Iterator<Item = &str>)> {
        self.line.clear();
        if let Err(_) | Ok(0) = self.reader.read_line(&mut self.line) {
            return None;
        };

        let mut parts = self.line.trim().split_whitespace();
        let Some(command) = parts.next() else { return None };

        Some((command, parts.fuse()))
    }
}

impl<T: Write> ProtocolWriter<T> {
    pub fn new(write: T) -> Self {
        Self { writer: write }
    }

    pub fn next(&mut self, kind: &str, arguments: &str) -> std::io::Result<()> {
        let data = format!("{kind} {arguments}\n");
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }
}

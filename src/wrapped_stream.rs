use std::io::{BufReader, Read, Write};
use std::net::TcpStream;

use bson::{Document};


pub struct WrappedStream {
    reader: BufReader<TcpStream>,
    stream: TcpStream,
    length_buffer: [u8; 4],
}

impl WrappedStream {
    pub fn new(stream: TcpStream) -> Self {
        WrappedStream {
            reader: BufReader::new(stream.try_clone().unwrap()),
            stream,
            length_buffer: [0 as u8; 4],
        }
    }

    pub fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.stream.write(data)
    }

    pub fn send_with_32bit_integer_length(&mut self, buffer: Vec<u8>) -> std::io::Result<usize> {
        self.stream.write(u32::to_le_bytes(buffer.len() as u32).as_ref())?;
        self.stream.write(buffer.as_slice())
    }

    pub fn send_bson(&mut self, doc: &Document) -> std::io::Result<usize> {
        let mut buffer = Vec::new();
        doc.to_writer(&mut buffer).unwrap();
        self.send_with_32bit_integer_length(buffer)
    }

    pub fn recv_32bit_integer(&mut self) -> u32 {
        self.reader.read_exact(&mut self.length_buffer).unwrap();
        u32::from_le_bytes(self.length_buffer)
    }

    pub fn recv_based_on_32bit_integer(&mut self) -> Vec<u8> {
        let data_size = self.recv_32bit_integer();
        let mut buf = vec![0u8; data_size as usize];
        self.reader.read_exact(&mut buf).unwrap();
        buf
    }

    pub fn recv_bson(&mut self) -> Document {
        let buf = self.recv_based_on_32bit_integer();
        Document::from_reader_utf8_lossy(buf.as_slice()).expect("invalid bson received from data client")
    }
}

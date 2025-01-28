use nih_plug::prelude::AtomicF32;
use std::{
    io::{Read, Write},
    sync::{atomic::AtomicU32, Arc},
};

pub struct Client {
    pub name: String,
    pub udp_addr: std::net::SocketAddr,
    pub is_running: bool,
    pub is_connected: bool,
    stream: std::net::TcpStream,
    pub delay_in_samples: Arc<AtomicU32>,
    pub delay_in_ms: Arc<AtomicF32>,
    buf: [u8; 1],
}
impl Client {
    pub fn new(name: String, udp_addr: std::net::SocketAddr, stream: std::net::TcpStream) -> Self {
        stream.set_nonblocking(true).unwrap();
        Self {
            name,
            udp_addr,
            is_running: false,
            is_connected: true,
            stream,
            delay_in_samples: Arc::new(AtomicU32::new(0)),
            delay_in_ms: Arc::new(AtomicF32::new(0.0)),
            buf: [0u8; 1],
        }
    }
    pub fn start(&mut self) -> bool {
        self.stream.write_all("START".as_bytes()).is_ok()
    }
    pub fn stop(&mut self) -> bool {
        self.stream.write_all("STOP".as_bytes()).is_ok()
    }
    pub fn still_alive(&mut self) -> bool {
        // self.stream.write(&[0]).is_ok()
        match self.stream.read(&mut self.buf) {
            Ok(0) => false,
            Err(_) => true,
            _ => true,
        }
    }
}

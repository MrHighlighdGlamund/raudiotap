use nih_plug::prelude::AtomicF32;
use std::{
    io::{Read, Write},
    sync::{atomic::AtomicU32, Arc},
};

use crate::utilities::enmus::GuiMessage;

pub struct Client {
    pub name: String,
    pub udp_addr: std::net::SocketAddr,
    pub is_running: bool,
    pub is_connected: bool,
    stream: std::net::TcpStream,
    pub delay_in_samples: Arc<AtomicU32>,
    pub delay_in_ms: Arc<AtomicF32>,
    buf: [u8; 1],
    send_message: crossbeam_channel::Sender<GuiMessage>,
}
impl Client {
    pub fn new(
        name: String,
        udp_addr: std::net::SocketAddr,
        stream: std::net::TcpStream,
        send_message: crossbeam_channel::Sender<GuiMessage>,
    ) -> Self {
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
            send_message,
        }
    }
    pub fn start(&mut self) -> bool {
        self.stream.write_all("START".as_bytes()).is_ok()
    }
    pub fn stop(&mut self) -> bool {
        self.stream.write_all("STOP".as_bytes()).is_ok()
    }
    pub fn new_delay(&mut self) {
        let update_message = format!(
            "DELAY:{}",
            self.delay_in_samples
                .load(std::sync::atomic::Ordering::Relaxed)
        );
        self.stream.write_all(update_message.as_bytes()).unwrap();
    }
    pub fn still_alive(&mut self) -> bool {
        // self.stream.write(&[0]).is_ok()
        match self.stream.read(&mut self.buf) {
            Ok(0) => {
                self.send_message
                    .send(GuiMessage::ServerError(
                        "Client Disconnected: ".to_string() + &self.name,
                    ))
                    .unwrap();
                self.is_connected = false;

                false
            }
            Err(_) => true,
            _ => true,
        }
    }
}

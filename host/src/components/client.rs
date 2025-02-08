use nih_plug::prelude::AtomicF32;
use std::{
    io::{Read, Write},
    sync::{atomic::AtomicU32, Arc},
};

use crate::utilities::enmus::ClientMessage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Client {
    pub name: String,
    pub udp_addr: std::net::SocketAddr,
    pub is_running: bool,
    pub is_connected: bool,
    pub delay: Arc<AtomicF32>,
    pub ping: f32,
    pub sender: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    buf: [u8; 1],
}
impl Client {
    pub fn new(
        name: String,
        udp_addr: std::net::SocketAddr,
        sender: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
        receiver: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    ) -> Self {
        Self {
            name,
            udp_addr,
            is_running: false,
            is_connected: true,
            delay: Arc::new(AtomicF32::new(0.0)),
            ping: 0.0,
            sender,
            receiver,
            buf: [0u8; 1],
        }
    }
    // pub async fn start(&mut self)  {
    //     self.stream.write_all("START".as_bytes()).await.is_ok();
    //     // self.stream.write_all("START".as_bytes()).is_ok()
    // }
    // pub async fn stop(&mut self)  {
    //     self.stream.write_all("STOP".as_bytes()).await.is_ok();
    // }
    // pub async fn new_delay(&mut self) {
    //     let update_message = format!(
    //         "DELAY:{}",
    //         self.delay_in_samples
    //             .load(std::sync::atomic::Ordering::Relaxed)
    //     );
    //     self.stream.write_all(update_message.as_bytes()).await.is_ok();
    // }
    // pub fn still_alive(&mut self) -> bool {
    //     // self.stream.write(&[0]).is_ok()
    //     match self.stream.read(&mut self.buf) {
    //         Ok(0) => {
    //             self.send_message
    //                 .send(GuiMessage::ServerError(
    //                     "Client Disconnected: ".to_string() + &self.name,
    //                 ))
    //                 .unwrap();
    //             self.is_connected = false;

    //             false
    //         }
    //         Err(_) => true,
    //         _ => true,
    //     }
    // }
}

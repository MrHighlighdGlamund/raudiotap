use crate::utilities::enmus::GuiMessage;
use std::{
    io::Read,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

pub struct UdpSender {
    udp_socket: Option<std::net::UdpSocket>,
    targets_addr: Option<Vec<std::net::SocketAddr>>,
    send_message: crossbeam_channel::Sender<GuiMessage>,
    audio_queue: Option<rtrb::Consumer<u8>>,
    pub targets_update: Arc<AtomicBool>,
    pub targets_addr_shared: std::sync::Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>>,
}
impl UdpSender {
    pub fn new(
        send_message: crossbeam_channel::Sender<GuiMessage>,
        audio_queue: Option<rtrb::Consumer<u8>>,
    ) -> Self {
        let udp_socket = if let Ok(udp_socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            Some(udp_socket)
        } else {
            send_message
                .send(GuiMessage::Log("Failed to bind UDP socket".to_string()))
                .unwrap();
            None
        };

        Self {
            udp_socket,
            targets_addr: Some(Vec::new()),
            send_message,
            audio_queue,
            targets_update: Arc::new(AtomicBool::new(false)),
            targets_addr_shared: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    pub fn run(&mut self) {
        let mut audio_queue = match self.audio_queue.take() {
            Some(queue) => queue,
            None => {
                self.send_message
                    .send(GuiMessage::Log("Audio queue not found".to_string()))
                    .unwrap();
                return;
            }
        };
        let mut targets = match self.targets_addr.take() {
            Some(targets) => targets,
            None => Vec::new(),
        };
        let mut udp_socket = match self.udp_socket.take() {
            Some(udp_socket) => udp_socket,
            None => {
                self.send_message
                    .send(GuiMessage::Log("UDP socket not found".to_string()))
                    .unwrap();
                return;
            }
        };
        let targets_update = self.targets_update.clone();
        let targets_addr_shared = self.targets_addr_shared.clone();
        let mut buffer: Vec<u8> = vec![0; 512];
        let mut targets_live = false;
        let send_message = self.send_message.clone();
        thread::spawn(move || loop {
            if targets_update.load(std::sync::atomic::Ordering::Relaxed) {
                targets.clear();
                for target in targets_addr_shared.lock().unwrap().iter() {
                    targets.push(*target);
                }
                targets_live = !targets.is_empty();
                while audio_queue.pop().is_ok(){};

                targets_update.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            if targets_live {
                match audio_queue.read_chunk(512) {
                    Ok(chunk) => {
                        buffer = chunk.into_iter().collect();
                        for target in targets.iter() {
                            udp_socket.send_to(&buffer, target).unwrap();
                        }
                    }
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                buffer.clear();
            }
            else {
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        });
    }
}
impl Clone for UdpSender {
    fn clone(&self) -> Self {
        Self {
            udp_socket: None,
            targets_addr: self.targets_addr.clone(),
            send_message: self.send_message.clone(),
            audio_queue: None,
            targets_update: self.targets_update.clone(),
            targets_addr_shared: self.targets_addr_shared.clone(),
        }
    }
}

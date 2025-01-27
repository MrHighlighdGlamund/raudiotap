use std::sync::{atomic::AtomicBool, Arc};
use crate::utilities::enmus::GuiMessage;

pub struct UdpSender {
    udp_socket: Option<std::net::UdpSocket>,
    targets_addr: Vec<std::net::SocketAddr>,
    send_message: crossbeam_channel::Sender<GuiMessage>,
    audio_queue: Option<rtrb::Consumer<u8>>,
    pub targets_update: Arc<AtomicBool>,
    pub targets_addr_shared: std::sync::Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>>,
}
impl UdpSender {
    pub fn new(send_message: crossbeam_channel::Sender<GuiMessage>, audio_queue: Option<rtrb::Consumer<u8>>) -> Self {
        
        let udp_socket = if let Ok(udp_socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            Some(udp_socket)
        } else {
            None
        };
            
        Self {
            udp_socket,
            targets_addr: Vec::new(),
            send_message,
            audio_queue,
            targets_update: Arc::new(AtomicBool::new(false)),
            targets_addr_shared: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
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

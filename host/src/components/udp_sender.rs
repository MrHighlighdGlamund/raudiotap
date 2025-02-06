use tokio::select;
use tokio::sync::Notify;
use tokio::sync::{RwLock, Semaphore};

use crate::utilities::enmus::GuiMessage;
use std::sync::atomic::{AtomicU16, AtomicU64};
use std::{
    io::Read,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

pub struct UdpSender {
    targets_addr: Option<Vec<std::net::SocketAddr>>,
    send_message: crossbeam_channel::Sender<GuiMessage>,
    audio_queue: Option<rtrb::Consumer<u8>>,
    pub targets_update: Arc<AtomicBool>,
    pub targets_addr_shared: std::sync::Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>>,
    pub udp_chunk_size: Arc<AtomicU64>,
}
impl UdpSender {
    pub fn new(
        send_message: crossbeam_channel::Sender<GuiMessage>,
        audio_queue: Option<rtrb::Consumer<u8>>,
        targets_addr_shared: Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>>,
        targets_update: Arc<AtomicBool>,
        udp_chunk_size: Arc<AtomicU64>,
    ) -> Self {
        Self {
            targets_addr: Some(Vec::new()),
            send_message,
            audio_queue,
            targets_update,
            targets_addr_shared,
            udp_chunk_size,
        }
    }
    pub async fn run(&mut self) {
        let mut audio_queue = match self.audio_queue.take() {
            Some(queue) => queue,
            None => {
                self.send_message
                    .send(GuiMessage::Log("Audio queue not found".to_string()))
                    .unwrap();
                return;
            }
        };
        // let mut targets = match self.targets_addr.take() {
        //     Some(targets) => targets,
        //     None => Vec::new(),
        // };
        let mut targets = Vec::new();

        let udp_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let targets_update = self.targets_update.clone();
        let targets_addr_shared = self.targets_addr_shared.clone();
        let mut targets_live = false;
        let send_message = self.send_message.clone();

        loop {
            let udp_chunk_size:u64 = self.udp_chunk_size.load(std::sync::atomic::Ordering::Acquire).into();
            let mut buffer: Vec<u8> = vec![0; udp_chunk_size as usize];
            targets.clear();
            for target in targets_addr_shared.lock().unwrap().iter() {
                targets.push(*target);
            }
            targets_update.store(false, std::sync::atomic::Ordering::Release);
            if targets.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }
            while audio_queue.pop().is_ok() {}
            loop {
                if targets_update.load(std::sync::atomic::Ordering::Acquire) {
                    break;
                }
                if let Ok(chunk) = audio_queue.read_chunk(udp_chunk_size as usize) {
                    buffer = chunk.into_iter().collect();
                    for target in targets.iter() {
                        udp_socket.send_to(&buffer, target).await.unwrap();
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_micros(10));
                }
                buffer.clear();
            }
        }

        // if targets_update.load(std::sync::atomic::Ordering::Relaxed) {
        //     targets.clear();
        //     for target in targets_addr_shared.lock().unwrap().iter() {
        //         targets.push(*target);
        //     }
        //     targets_live = !targets.is_empty();

        //     targets_update.store(false, std::sync::atomic::Ordering::Relaxed);
        // }
        // if targets_live {
        //     match audio_queue.read_chunk(512) {
        //         Ok(chunk) => {
        //             buffer = chunk.into_iter().collect();
        //             for target in targets.iter() {
        //                 udp_socket.send_to(&buffer, target).await.unwrap();
        //             }
        //         }
        //         Err(_) => {
        //             std::thread::sleep(std::time::Duration::from_millis(1));
        //         }
        //     }
        //     buffer.clear();
        // } else {
        //     std::thread::sleep(std::time::Duration::from_millis(16));
        // }
    }
}
// impl Clone for UdpSender {
//     fn clone(&self) -> Self {
//         Self {
//             targets_addr: self.targets_addr.clone(),
//             send_message: self.send_message.clone(),
//             audio_queue: None,
//             targets_update: self.targets_update.clone(),
//             targets_addr_shared: self.targets_addr_shared.clone(),
//         }
//     }
// }

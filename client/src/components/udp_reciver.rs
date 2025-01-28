use std::{convert::TryInto, net::UdpSocket, sync::{
    atomic::{AtomicBool, AtomicU32},
    Arc,
}};

use crate::utilities::{enmus::GuiMessage, helper_functions::get_udp_socket_address};
pub struct UdpReciver {
    thread_handle: Option<std::thread::JoinHandle<()>>,
    stop_bool: Arc<AtomicBool>,
    socket_addr: std::net::SocketAddr,
    send_message: Arc<crossbeam_channel::Sender<GuiMessage>>,
    pub add_delay_count: Arc<AtomicU32>,
    pub update_delay: Arc<AtomicBool>,
}
impl UdpReciver {
    pub fn run(&mut self, mut audio_queue: rtrb::Producer<i16>) {
        let stop_bool = self.stop_bool.clone();
        let mut buf = [0u8; 512];
        let update_delay = self.update_delay.clone();
        let add_delay_count = self.add_delay_count.clone();
        let socket = UdpSocket::bind(self.socket_addr).expect("Could not bind to socket");
        // socket.set_nonblocking(true).unwrap();
        socket.set_read_timeout(Some(std::time::Duration::from_millis(100))).unwrap();
        self.thread_handle = Some(std::thread::spawn(move || loop {
            if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            if update_delay.load(std::sync::atomic::Ordering::Relaxed) {
                for _ in 0..add_delay_count.load(std::sync::atomic::Ordering::Relaxed) {
                    match audio_queue.push(0) {
                        Ok(_) => {}
                        Err(_) => {
                            while let Err(_) = audio_queue.push(0) {
                                if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                                    break;
                                }
                            }
                        }
                    }
                }
                update_delay.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            match socket.recv_from(&mut buf) {
                Ok(_) => {}
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(3));
                    
                }
            }
            buf.chunks(2).for_each(|chunk| {
                match audio_queue.push(i16::from_le_bytes(chunk.try_into().unwrap())) {
                    Ok(_) => {}
                    Err(_) => loop {
                        if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        match audio_queue.push(i16::from_le_bytes(chunk.try_into().unwrap())) {
                            Ok(_) => {
                                break;
                            }
                            Err(_) => {}
                        }
                    },
                }
            });
        }));
    }
    pub fn stop(&mut self) {
        self.stop_bool
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            match handle.join() {
                Ok(_) => {}
                Err(e) => {
                    if let Some(err) = e.downcast_ref::<std::io::Error>() {
                        self.send_message
                            .send(GuiMessage::Log(
                                "UDP_T_JOIN_ERROR".to_string() + &err.to_string(),
                            ))
                            .unwrap();
                    }
                }
            }
        }
        self.send_message
            .send(GuiMessage::Log(
                "UDP Thread Stopped Successfully".to_string(),
            ))
            .unwrap();
        self.stop_bool
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn new(send_message: Arc<crossbeam_channel::Sender<GuiMessage>>) -> Self {
        let mut socket_addr: std::net::SocketAddr = "127.0.0.1:8000".parse().unwrap();

        match get_udp_socket_address() {
            Some(ip) => {
                socket_addr = ip;
            }
            None => {
                send_message
                    .send(GuiMessage::ServerError(
                        "Unable to get UDP Socket address".to_string(),
                    ))
                    .unwrap();
            }
        }
        Self {
            thread_handle: None,
            stop_bool: Arc::new(AtomicBool::new(false)),
            socket_addr,
            send_message,
            add_delay_count: Arc::new(AtomicU32::new(0)),
            update_delay: Arc::new(AtomicBool::new(false)),
        }
    }
}

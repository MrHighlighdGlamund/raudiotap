use crossbeam_channel::Sender;
use local_ip_addr::get_local_ip_address;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex,
    },
};

use crate::utilities::enmus::GuiMessage;
use crate::{components::client::Client, utilities::helper_functions::get_local_socket_address};

pub struct Server {
    thread_hanle: Option<std::thread::JoinHandle<()>>,
    stop_thread: Arc<AtomicBool>,
    socket_addr: SocketAddr,
    send_message: crossbeam_channel::Sender<GuiMessage>,
    pub update_udp_thread: Arc<AtomicBool>,
    pub targets_addr_shared: Arc<Mutex<Vec<SocketAddr>>>,

    pub clients: Arc<Mutex<Vec<Client>>>,
    pub sample_rate: Arc<AtomicU32>,
}
impl Server {
    pub fn new(
        send_message: Sender<GuiMessage>,
        update_udp_thread: Arc<AtomicBool>,
        targets_addr_shared: Arc<Mutex<Vec<SocketAddr>>>,
    ) -> Self {
        let mut clients: Vec<Client> = Vec::new();
        Self {
            socket_addr: "127.0.0.1:8000".parse().unwrap(),
            clients: Arc::new(Mutex::new(clients)),
            send_message,
            update_udp_thread,
            targets_addr_shared,
            sample_rate: Arc::new(AtomicU32::new(0)),
            thread_hanle: None,
            stop_thread: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn run(&mut self) {
        match get_local_socket_address() {
            Some(socket_addr) => {
                self.socket_addr = socket_addr;
            }
            None => {
                self.send_message
                    .send(GuiMessage::ServerError(
                        "Unable to get TCP Socket address".to_string(),
                    ))
                    .unwrap();
                return;
            }
        }
        let mut tcp_listener: Option<TcpListener> = TcpListener::bind(self.socket_addr).ok();
        if tcp_listener.is_none() {
            self.send_message
                .send(GuiMessage::ServerError(
                    "Unable to start TCP Server, maybe one is allready connected".to_string(),
                ))
                .unwrap();
            return;
        }
        let tcp_listener = tcp_listener.take().expect("tcp_listener is none");
        tcp_listener
            .set_nonblocking(true)
            .expect("Failed to set nonblocking");

        let stop_thread = self.stop_thread.clone();
        let clients = self.clients.clone();
        let sample_rate = self.sample_rate.clone();
        self.thread_hanle = Some(std::thread::spawn(move || loop {
            let upate_cycle = std::time::Duration::from_secs(2);
            let mut update_time = std::time::Instant::now();
            for stream in tcp_listener.incoming() {
                if stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                if update_time.elapsed() > upate_cycle {
                    clients
                        .lock()
                        .unwrap()
                        .retain_mut(|client| client.still_alive());
                    update_time = std::time::Instant::now();
                }
                std::thread::sleep(std::time::Duration::from_millis(70));

                match stream {
                    Ok(mut stream) => {
                        let mut message = [0; 512];
                        match stream.read(&mut message) {
                            Ok(bytes) => match String::from_utf8(message[..bytes].to_vec()) {
                                Ok(message) => {
                                    let command = message.split(":").collect::<Vec<&str>>()[0];
                                    match command {
                                        "CONNECT" => {
                                            let data = message.split(":").collect::<Vec<&str>>()[1];
                                            println!("CONNECT: {}", data);
                                            let mut client_ip = stream.peer_addr().unwrap();
                                            client_ip.set_port(8000);

                                            clients.lock().unwrap().retain_mut(|client| {
                                                client.udp_addr != client_ip
                                            });
                                            let message_SR = format!("SAMPLERATE:{}", sample_rate.load(std::sync::atomic::Ordering::Relaxed));
                                            stream.write_all(message_SR.as_bytes()).unwrap();



                                            let client = Client::new(
                                                data.to_string(),
                                                client_ip,
                                                stream.try_clone().unwrap(),
                                            );
                                            clients.lock().unwrap().push(client);
                                        }
                                        _ => {
                                            println!("Unknown command: {}", command);
                                        }
                                    }
                                }
                                Err(_) => {
                                    println!("Failed to convert message to utf8");
                                }
                            },
                            Err(_) => {
                                println!("Failed to read tcp message");
                            }
                        }
                    }
                    Err(_) => {
                        // println!("Failed to get stream")
                    }
                }
            }
        }));
    }
}
impl Clone for Server {
    fn clone(&self) -> Self {
        Self {
            socket_addr: self.socket_addr.clone(),
            clients: self.clients.clone(),
            send_message: self.send_message.clone(),
            update_udp_thread: self.update_udp_thread.clone(),
            targets_addr_shared: self.targets_addr_shared.clone(),
            sample_rate: self.sample_rate.clone(),
            thread_hanle: None,
            stop_thread: self.stop_thread.clone(),
        }
    }
}

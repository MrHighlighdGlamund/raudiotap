use crate::utilities::enmus::{ClientMessage, GuiMessage};
use crate::{components::client::Client, utilities::helper_functions::get_local_socket_address};
use crossbeam_channel::Sender;
use local_ip_addr::get_local_ip_address;
use regex::Regex;
use std::sync::atomic::{AtomicU16, AtomicU64};
use std::thread;
use std::{
    io::{Read, Write},
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::{select, task, time};

pub struct Server {
    thread_hanle: Option<std::thread::JoinHandle<()>>,
    stop_thread: Arc<AtomicBool>,
    socket_addr: SocketAddr,
    send_message: crossbeam_channel::Sender<GuiMessage>,
    pub update_udp_thread: Arc<AtomicBool>,
    pub targets_addr_shared: Arc<Mutex<Vec<SocketAddr>>>,
    pub send_client: crossbeam_channel::Sender<Client>,
    pub sample_rate: Arc<AtomicU32>,
    pub udp_chunk_size: Arc<AtomicU64>,
}
impl Server {
    pub fn new(
        send_message: Sender<GuiMessage>,
        update_udp_thread: Arc<AtomicBool>,
        targets_addr_shared: Arc<Mutex<Vec<SocketAddr>>>,
        sample_rate: Arc<AtomicU32>,
        send_client: crossbeam_channel::Sender<Client>,
        udp_chunk_size: Arc<AtomicU64>,
    ) -> Self {
        Self {
            socket_addr: "127.0.0.1:8000".parse().unwrap(),
            send_message,
            update_udp_thread,
            targets_addr_shared,
            sample_rate,
            thread_hanle: None,
            stop_thread: Arc::new(AtomicBool::new(false)),
            send_client,
            udp_chunk_size,
        }
    }
    pub async fn run(&mut self) {
        match get_local_socket_address() {
            Some(socket_addr) => {
                self.send_message.send(GuiMessage::ServerError(format!(
                    "Successfully got TCP Socket address: {}",
                    socket_addr
                )));
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

        let mut tcp_listener = match TcpListener::bind(self.socket_addr).await {
            Ok(tcp_listener) => tcp_listener,
            Err(e) => {
                self.send_message
                    .send(GuiMessage::ServerError(
                        "Unable to start TCP Server, maybe one is allready connected".to_string(),
                    ))
                    .unwrap();
                self.send_message
                    .send(GuiMessage::ServerError(
                        "Unable to start TCP Server, maybe one is allready connected".to_string(),
                    ))
                    .unwrap();
                return;
            }
        };

        let send_client = self.send_client.clone();
        let sample_rate = self.sample_rate.clone();
        let send_message = self.send_message.clone();
        let udp_chunk_size = self.udp_chunk_size.clone();

        let update_udp_thread = self.update_udp_thread.clone();
        let targets_addr_shared = self.targets_addr_shared.clone();

        loop {
            match tcp_listener.accept().await {
                Ok((mut stream, _)) => {
                    let send_client = send_client.clone();
                    let update_udp_thread = update_udp_thread.clone();
                    let targets_addr_shared = targets_addr_shared.clone();
                    let sample_rate = sample_rate.clone();

                    let udp_chunk_size = udp_chunk_size.clone();
                    let chunk_msg =
                        format!("UDP_CHUNK_SIZE:{}", udp_chunk_size.load(Ordering::Acquire));

                    tokio::spawn(async move {
                        handle_client(
                            stream,
                            send_client,
                            update_udp_thread,
                            targets_addr_shared,
                            sample_rate.clone(),
                            udp_chunk_size.clone(),
                        )
                        .await;
                    });
                }
                Err(_e) => {}
            }
        }
    }
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Self {
            socket_addr: self.socket_addr.clone(),
            send_message: self.send_message.clone(),
            update_udp_thread: self.update_udp_thread.clone(),
            targets_addr_shared: self.targets_addr_shared.clone(),
            sample_rate: self.sample_rate.clone(),
            thread_hanle: None,
            stop_thread: self.stop_thread.clone(),
            send_client: self.send_client.clone(),
            udp_chunk_size: self.udp_chunk_size.clone(),
        }
    }
}

async fn handle_client(
    mut stream: tokio::net::TcpStream,
    send_client: crossbeam_channel::Sender<Client>,
    update_udp_thread: Arc<AtomicBool>,
    targets_addr_shared: Arc<Mutex<Vec<SocketAddr>>>,
    sample_rate: Arc<AtomicU32>,
    udp_chunk_size: Arc<AtomicU64>,
) {
    let (send_to_gui, recv_from_client) = tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
    let (mut send_to_client, mut recv_from_gui) =
        tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
    let mut msg = [0; 512];
    let mut client_ip = stream.peer_addr().unwrap();
    client_ip.set_port(8000);
    let mut recv_from_client: Option<tokio::sync::mpsc::UnboundedReceiver<ClientMessage>> =
        Some(recv_from_client);
    let mut interval = time::interval(tokio::time::Duration::from_secs(1)); // Interval for the print task
    let mut ping_fail_count = 0;

    loop {
        select! {
        Some(msg) = recv_from_gui.recv() => {  // No `.await` here, just check `Some(msg)`
            match msg {
                ClientMessage::UdpChunkSize(chunk_size) => {
                    udp_chunk_size.store(chunk_size, Ordering::Release);
                    let msg = format!("UDP_CHUNK_SIZE:{}", chunk_size);
                    let _ = stream.write_all(msg.as_bytes()).await;
                    update_udp_thread.store(true, Ordering::Release);
                }
                ClientMessage::Start => {

                    let mut client_ip = stream.peer_addr().unwrap();
                    client_ip.set_port(8000);
                    targets_addr_shared.lock().unwrap().retain(|addr| *addr != client_ip);
                    targets_addr_shared.lock().unwrap().push(client_ip);
                    update_udp_thread.store(true, Ordering::Release);
                    let msg = format!("START:{}", udp_chunk_size.load(Ordering::Acquire));

                    let _ = stream.write_all(msg.as_bytes()).await;
                }
                ClientMessage::Stop => {
                    let mut client_ip = stream.peer_addr().unwrap();
                    client_ip.set_port(8000);
                    targets_addr_shared.lock().unwrap().retain(|addr| *addr != client_ip);
                    update_udp_thread.store(true, Ordering::Release);
                    let _ = stream.write_all("STOP".as_bytes()).await;
                }
                ClientMessage::Delay(new_sample_delay) => {
                    let _ = stream.write_all(format!("DELAY:{}", new_sample_delay).as_bytes()).await;
                }
                _ => {
                    println!("Unknown message");
                }
            }

        },

            result = stream.read(&mut msg) => {
                match result {
                    Ok(0) => {
                        break;
                    }
                    Ok(bytes) => {
                        if let Ok(message) = String::from_utf8(msg[..bytes].to_vec()) {
                            let message_parts: Vec<&str> = message.split(':').collect();
                                let command = message_parts[0];

                                match command {
                                    "CONNECT" => {
                                        let data = message_parts[1].to_string();
                                        let mut client_ip = stream.peer_addr().unwrap();
                                        client_ip.set_port(8000);
                                        let message_SR = format!(
                                            "SAMPLERATE:{}",
                                            sample_rate.load(Ordering::Relaxed)
                                        );
                                        let _ = stream.write_all(message_SR.as_bytes()).await;
                                        let new_client = Client::new(
                                            data,
                                            client_ip,
                                            send_to_client.clone(),
                                            recv_from_client.take().unwrap(),
                                        );
                                        let _ = send_client.send(new_client);
                                    }
                                    "CLIENTSTART" => {
                                        send_to_gui.send(ClientMessage::Start).unwrap();
                                    }
                                    "CLIENTSTOP" => {
                                        send_to_gui.send(ClientMessage::Stop).unwrap();
                                    }
                                    "CLIENTDELAY" => {
                                        let delay:f32 = message_parts[1].parse().unwrap();
                                        send_to_gui.send(ClientMessage::Delay(delay));   
                                    }

                                    _ => {
                                    }
                                }
                        }
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            },
             _ = interval.tick() => {

                 let ping = ping_ip(&client_ip.to_string()).await;
                 if let Some(ping) = ping {
                     send_to_gui.send(ClientMessage::Ping(ping)).unwrap();
                 }
                 else if ping_fail_count < 3 {
                     ping_fail_count += 1;
                 }
                 else {
                     break;
                 }
             }
        }
    }

    let _ = send_to_gui.send(ClientMessage::Disconnect);
}
async fn ping_ip(ip: &str) -> Option<f32> {
    // Run the ping command
    //
    let ip = ip.split(":").collect::<Vec<&str>>()[0];
    let output = if cfg!(target_os = "windows") {
        Command::new("ping").args(["-n", "1", ip]).output().await
    } else {
        Command::new("ping").args(["-c", "1", ip]).output().await
    };

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Regex to extract ping time (works for Windows & Linux/macOS)
            let re = if cfg!(target_os = "windows") {
                Regex::new(r"Minimum = (\d+)ms").unwrap() // Windows format
            } else {
                Regex::new(r"time=([\d.]+) ms").unwrap() // Linux/macOS format
            };

            if let Some(caps) = re.captures(&stdout) {
                if let Some(time) = caps.get(1) {
                    let ping_time: f32 = time.as_str().parse().unwrap_or(0.0);
                    return Some(ping_time);
                }
            }

            None
        }
        Err(e) => None,
    }
}

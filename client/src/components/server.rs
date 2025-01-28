use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;

use local_ip_addr::get_local_ip_address;

use super::{audio::Audio, udp_reciver::UdpReciver};

use crate::utilities::enmus::GuiMessage;
use crate::utilities::helper_functions::get_local_socket_address;
pub struct Server {
    ip_address: String,
    pub ip_socket_local: std::net::SocketAddr,
    test_ips: Arc<Vec<std::net::SocketAddr>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    send_message: Arc<crossbeam_channel::Sender<GuiMessage>>,
    pub recv_message: Arc<crossbeam_channel::Receiver<GuiMessage>>,
}
impl Server {
    pub fn run(&mut self) {
        let local_ip = self.ip_address.clone();
        let test_ips = self.test_ips.clone();
        let mut tcp_stream: Option<std::net::TcpStream> = None;
        let send_message = self.send_message.clone();

        self.thread_handle = Some(std::thread::spawn(move || {
            let mut audio = Audio::new(send_message.clone());
            let mut udp_reciver = UdpReciver::new(send_message.clone());
            // find host
            while tcp_stream.is_none() {
                for ip in test_ips.iter() {
                    if std::net::TcpStream::connect_timeout(
                        ip,
                        std::time::Duration::from_millis(40),
                    )
                    .is_ok()
                    {
                        tcp_stream = Some(std::net::TcpStream::connect(ip).unwrap());
                        send_message
                            .send(GuiMessage::Log(
                                "Connected to ".to_string() + &ip.to_string(),
                            ))
                            .unwrap();
                        let command = "CONNECT:AndrePhone".to_string();
                        tcp_stream
                            .as_mut()
                            .unwrap()
                            .write_all(command.as_bytes())
                            .unwrap();
                        break;
                    }
                }
            }
            // Connected to Host

            let mut tcp_stream = tcp_stream.take().expect("tcp_stream is none");
            loop {
                let mut buffer = [0; 512];
                match tcp_stream.read(&mut buffer) {
                    Ok(0) => {
                        send_message
                            .send(GuiMessage::Log("Connection closed by server".to_string()))
                            .unwrap();
                        break;
                    }
                    Ok(n) => {
                        let msg = String::from_utf8_lossy(&buffer[..n]);
                        match msg {
                            msg if msg.contains("START") => {
                                send_message
                                    .send(GuiMessage::Log(
                                        "Received START message from server".to_string(),
                                    ))
                                    .unwrap();
                                audio.stop();
                                udp_reciver.stop();

                                let (producer, consumer) = rtrb::RingBuffer::<i16>::new(96000 * 64);
                                audio.run(consumer);
                                std::thread::sleep(std::time::Duration::from_millis(300));
                                udp_reciver.run(producer);
                            }
                            msg if msg.contains("STOP") => {
                                send_message
                                    .send(GuiMessage::Log(
                                        "Received STOP message from server".to_string(),
                                    ))
                                    .unwrap();
                                audio.stop();
                                udp_reciver.stop();
                            }
                            message if message.contains("SAMPLERATE") => {
                                let new_sample_rate: u32 = match message
                                    .split(":")
                                    .collect::<Vec<&str>>()[1]
                                    .trim()
                                    .parse()
                                {
                                    Ok(sample_rate) => sample_rate,
                                    Err(e) => {
                                        send_message
                                                .send(GuiMessage::Log(
                                                    "Received invalid SAMPLERATE message from server. Error: ".to_string() + &e.to_string(),
                                            ))
                                            .unwrap();
                                        continue;
                                    }
                                };
                                audio.sample_rate
                                    .store(new_sample_rate, std::sync::atomic::Ordering::Relaxed);
                                send_message
                                    .send(GuiMessage::Log(
                                        "Received SAMPLERATE message from server. New sample rate: "
                                            .to_string()
                                            + &new_sample_rate.to_string(),
                                    ))
                                    .unwrap();
                            }
                            message if message.contains("DELAY") => {
                                let new_sample_delay = message.split(":").collect::<Vec<&str>>()[1]
                                    .trim()
                                    .parse()
                                    .unwrap();
                                let current_sample_delay = audio
                                    .current_delay
                                    .load(std::sync::atomic::Ordering::Relaxed);
                                if current_sample_delay != new_sample_delay {
                                    if new_sample_delay > current_sample_delay {
                                        let add_samples = (new_sample_delay - current_sample_delay);
                                        udp_reciver.add_delay_count.store(
                                            add_samples,
                                            std::sync::atomic::Ordering::Relaxed,
                                        );
                                        udp_reciver
                                            .update_delay
                                            .store(true, std::sync::atomic::Ordering::Relaxed);
                                    } else {
                                        let delete_samples =
                                            (current_sample_delay - new_sample_delay);
                                        audio.delete_delay_count.store(
                                            delete_samples,
                                            std::sync::atomic::Ordering::Relaxed,
                                        );
                                        audio
                                            .update_delay
                                            .store(true, std::sync::atomic::Ordering::Relaxed);
                                    }
                                    audio.current_delay.store(
                                        new_sample_delay,
                                        std::sync::atomic::Ordering::Relaxed,
                                    );
                                }
                                send_message
                                    .send(GuiMessage::Log(
                                        "Received DELAY message from server. New delay: "
                                            .to_string()
                                            + &new_sample_delay.to_string(),
                                    ))
                                    .unwrap();
                            }
                            _ => {
                                send_message
                                    .send(GuiMessage::Log(
                                        "Received unknown message from server".to_string()
                                            + msg.as_ref(),
                                    ))
                                    .unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from server: {}", e);
                        break;
                    }
                }
            }
        }));
    }
    pub fn stop(&mut self) {}

    pub fn new() -> Self {
        let (send_message, recv_message) = crossbeam_channel::bounded::<GuiMessage>(1000);
        let send_message: Arc<crossbeam_channel::Sender<GuiMessage>> = Arc::new(send_message);
        let recv_message: Arc<crossbeam_channel::Receiver<GuiMessage>> = Arc::new(recv_message);

        let audio = Audio::new(send_message.clone());
        let udp_reciver = UdpReciver::new(send_message.clone());
        let mut ip_socket_local: SocketAddr = "127.0.0.1:7878".parse().unwrap();
        match get_local_socket_address() {
            Some(ip) => {
                ip_socket_local = ip;
            }
            None => {
                send_message
                    .send(GuiMessage::ServerError(
                        "Unable to get local IP address".to_string(),
                    ))
                    .unwrap();
            }
        }

        let local_ip = get_local_ip_address().unwrap();

        let ipchunk = local_ip.split(".").collect::<Vec<&str>>()[0..3].join(".");
        let ipchunk = ipchunk + ".";
        let mut test_ips: Vec<std::net::SocketAddr> = Vec::new();
        for i in 1..255 {
            let ip = ipchunk.to_string() + &i.to_string() + ":7878";
            test_ips.push(ip.parse().unwrap());
        }

        Self {
            ip_address: local_ip.to_string(),
            ip_socket_local,
            test_ips: Arc::new(test_ips),
            thread_handle: None,
            send_message,
            recv_message,
        }
    }
}
impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

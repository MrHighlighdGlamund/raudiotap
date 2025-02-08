use std::cmp::Ordering;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use local_ip_addr::get_local_ip_address;

use super::{audio::Audio, udp_reciver::UdpReciver};

use crate::utilities::enmus::*;
use crate::utilities::helper_functions::get_local_socket_address;
pub struct Server {
    ip_address: String,
    pub ip_socket_local: std::net::SocketAddr,
    test_ips: Arc<Vec<std::net::SocketAddr>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    send_message: crossbeam_channel::Sender<ServerMessage>,
    pub recv_message: crossbeam_channel::Receiver<GuiMessage>,
    stop_bool: Arc<AtomicBool>,
}
impl Server {
    pub fn run(&mut self) {
        let local_ip = self.ip_address.clone();
        let test_ips = self.test_ips.clone();
        let mut tcp_stream: Option<std::net::TcpStream> = None;
        let send_message = self.send_message.clone();
        let recv_message = self.recv_message.clone();
        let stop_bool = self.stop_bool.clone();

        self.thread_handle = Some(std::thread::spawn(move || {
            let mut audio = Audio::new(send_message.clone());
            let mut udp_reciver = UdpReciver::new(send_message.clone());
            // find host
            loop {
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
                                .send(ServerMessage::Log(
                                    "Connected to ".to_string() + &ip.to_string(),
                                ))
                                .unwrap();
                            send_message.send(ServerMessage::Connected).unwrap();
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
                tcp_stream
                    .set_read_timeout(Some(std::time::Duration::from_millis(100)))
                    .unwrap();
                send_message.send(ServerMessage::Connected).unwrap();

                let mut msg = [0; 512];
                loop {
                    if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }

                    if let Ok(msg) = recv_message.try_recv() {
                        match msg {
                            GuiMessage::Start => {
                                let command = "CLIENTSTART".to_string();
                                tcp_stream.write_all(command.as_bytes()).unwrap();
                            }
                            GuiMessage::Stop => {
                                let command = "CLIENTSTOP".to_string();
                                tcp_stream.write_all(command.as_bytes()).unwrap();
                            }
                            GuiMessage::Delay(delay) => {
                                tcp_stream
                                    .write_all(format!("CLIENTDELAY:{}", delay).as_bytes())
                                    .unwrap();
                            }
                        }
                    };

                    match tcp_stream.read(&mut msg) {
                        Ok(0) => {
                            send_message
                                .send(ServerMessage::Log(
                                    "Connection closed by server".to_string(),
                                ))
                                .unwrap();
                            send_message.send(ServerMessage::Disconnected).unwrap();
                            break;
                        }
                        Ok(bytes) => {
                            if let Ok(message) = String::from_utf8(msg[..bytes].to_vec()) {
                                let message_parts: Vec<&str> = message.split(':').collect();
                                let command = message_parts[0];
                                match command {
                                    "START" => {
                                        udp_reciver.udp_chunk_size.store(
                                            message_parts[1].trim().parse().unwrap(),
                                            std::sync::atomic::Ordering::Release,
                                        );

                                        send_message
                                            .send(ServerMessage::Log(
                                                "Received START message from server".to_string(),
                                            ))
                                            .unwrap();
                                        audio.stop();
                                        udp_reciver.stop();
                                        let (producer, consumer) =
                                            rtrb::RingBuffer::<i16>::new(96000 * 64);
                                        audio.run(consumer);
                                        // std::thread::sleep(std::time::Duration::from_millis(300));
                                        udp_reciver.run(producer);
                                        send_message.send(ServerMessage::Started).unwrap();
                                    }

                                    "STOP" => {
                                        send_message
                                            .send(ServerMessage::Log(
                                                "Received STOP message from server".to_string(),
                                            ))
                                            .unwrap();
                                        audio.stop();
                                        udp_reciver.stop();
                                        send_message.send(ServerMessage::Stopped).unwrap();
                                    }
                                    "SAMPLERATE" => {
                                        let new_sample_rate: u32 = match message_parts[1]
                                            .trim()
                                            .parse()
                                        {
                                            Ok(sample_rate) => sample_rate,
                                            Err(e) => {
                                                send_message
                                                .send(ServerMessage::Log(
                                                    "Received invalid SAMPLERATE message from server. Error: ".to_string() + &e.to_string(),
                                            ))
                                            .unwrap();
                                                continue;
                                            }
                                        };
                                        audio.sample_rate.store(
                                            new_sample_rate,
                                            std::sync::atomic::Ordering::Relaxed,
                                        );
                                        send_message
                                    .send(ServerMessage::Log(
                                        "Received SAMPLERATE message from server. New sample rate: "
                                            .to_string()
                                            + &new_sample_rate.to_string(),
                                    ))
                                    .unwrap();
                                    }
                                    "UDP_CHUNK_SIZE" => {
                                        send_message
                                    .send(ServerMessage::Log(
                                        "Received UDP_CHUNK_SIZE message from server. New UDP chunk size: "
                                            .to_string()
                                            + &message_parts[1].trim(),
                                    ))

                                    .unwrap();
                                        let new_ = message_parts[1].trim().parse().unwrap();
                                        udp_reciver
                                            .udp_chunk_size
                                            .store(new_, std::sync::atomic::Ordering::Release);
                                        udp_reciver
                                            .update
                                            .store(true, std::sync::atomic::Ordering::Release);
                                        tcp_stream.write_all("CLIENTSTART".as_bytes()).unwrap();
                                    }

                                    "DELAY" => {
                                        let sample_rate = audio
                                            .sample_rate
                                            .load(std::sync::atomic::Ordering::Acquire);

                                        let new_delay: f32 =
                                            message_parts[1].trim().parse().unwrap();
                                        send_message
                                    .send(ServerMessage::Delay(
                                        new_delay
                                    ))
                                    .unwrap();
                                        
                                        let delay_in_samples =
                                            (new_delay as f32 / 1000.0 * sample_rate as f32 * 2.0)
                                                as u32;
                                        let current_sample_delay = audio
                                            .current_delay
                                            .load(std::sync::atomic::Ordering::Acquire);
                                        if current_sample_delay != delay_in_samples {
                                            if delay_in_samples > current_sample_delay {
                                                let add_samples =
                                                    (delay_in_samples - current_sample_delay);
                                                udp_reciver.add_delay_count.store(
                                                    add_samples,
                                                    std::sync::atomic::Ordering::Release,
                                                );
                                                udp_reciver.update_delay.store(
                                                    true,
                                                    std::sync::atomic::Ordering::Release,
                                                );
                                                udp_reciver.update.store(
                                                    true,
                                                    std::sync::atomic::Ordering::Release,
                                                );
                                            } else {
                                                let delete_samples =
                                                    (current_sample_delay - delay_in_samples);
                                                audio.delete_delay_count.store(
                                                    delete_samples,
                                                    std::sync::atomic::Ordering::Release,
                                                );
                                                audio.update_delay.store(
                                                    true,
                                                    std::sync::atomic::Ordering::Release,
                                                );
                                                udp_reciver.update.store(
                                                    true,
                                                    std::sync::atomic::Ordering::Release,
                                                );
                                            }
                                            audio.current_delay.store(
                                                delay_in_samples,
                                                std::sync::atomic::Ordering::Relaxed,
                                            );
                                        }
                                    }

                                    _ => {
                                        send_message
                                            .send(ServerMessage::Log(
                                                "Received unknown message from server".to_string()
                                                    + message.as_ref(),
                                            ))
                                            .unwrap();
                                    }
                                }
                            }
                        }
                        Err(e) => {}
                    }
                }
            }
        }));
    }

    pub fn stop(&mut self) {}

    pub fn new(
        send_message: crossbeam_channel::Sender<ServerMessage>,
        recv_message: crossbeam_channel::Receiver<GuiMessage>,
        stop_bool: Arc<AtomicBool>,
    ) -> Self {
        // let (send_message, recv_message) = crossbeam_channel::bounded::<GuiMessage>(1000);
        // let send_message: Arc<crossbeam_channel::Sender<GuiMessage>> = Arc::new(send_message);
        // let recv_message: Arc<crossbeam_channel::Receiver<GuiMessage>> = Arc::new(recv_message);

        let audio = Audio::new(send_message.clone());
        let udp_reciver = UdpReciver::new(send_message.clone());
        let mut ip_socket_local: SocketAddr = "127.0.0.1:7878".parse().unwrap();
        match get_local_socket_address() {
            Some(ip) => {
                ip_socket_local = ip;
            }
            None => {
                send_message
                    .send(ServerMessage::Log(
                        "Unable to get local IP address".to_string(),
                    ))
                    .unwrap();
            }
        }

        let local_ip = match get_local_ip_address() {
            Ok(ip) => ip,
            Err(e) => {
                send_message
                    .send(ServerMessage::Log(
                        "Unable to get local IP address".to_string(),
                    ))
                    .unwrap();
                "127.0.0.1".to_string()
            }
        };

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
            stop_bool,
        }
    }
}

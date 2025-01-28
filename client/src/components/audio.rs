use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::utilities::enmus::GuiMessage;

pub struct Audio {
    audio_stream: Option<cpal::Stream>,
    stop_bool: Arc<AtomicBool>,
    send_message: Arc<crossbeam_channel::Sender<GuiMessage>>,
    pub current_delay: Arc<AtomicU32>,
    pub update_delay: Arc<AtomicBool>,
    pub delete_delay_count: Arc<AtomicU32>,
    pub sample_rate: Arc<AtomicU32>,
}
impl Audio {
    pub fn run(&mut self, mut audio_queue: rtrb::Consumer<i16>) {
        let stop_bool = self.stop_bool.clone();
        let update_delay = self.update_delay.clone();
        let delete_delay_count = self.delete_delay_count.clone();
        let send_message = self.send_message.clone();
        
        let host = cpal::default_host();
        let (mut prod, mut consum) = rtrb::RingBuffer::<i16>::new(96000 * 64);
        let device = host
            .default_output_device()
            .expect("Failed to get default output device");
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(self.sample_rate.load(std::sync::atomic::Ordering::Relaxed)),
            buffer_size: cpal::BufferSize::Default,
        };

        self.audio_stream = Some(
            device
                .build_output_stream(
                    &config,
                    move |data: &mut [i16], driver_err_callback: &cpal::OutputCallbackInfo| {
                        if update_delay.load(std::sync::atomic::Ordering::Relaxed) {
                            for _ in
                                0..delete_delay_count.load(std::sync::atomic::Ordering::Relaxed)
                            {
                                while audio_queue.pop().is_err() {
                                    if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                                        break;
                                    }
                                }
                            }
                            update_delay.store(false, std::sync::atomic::Ordering::Relaxed);
                        }

                        for sample in data {
                            match audio_queue.pop() {
                                Ok(value) => {
                                    *sample = value;
                                }
                                Err(_) => loop {
                                    // send_message.send(GuiMessage::BufferUnderrun).unwrap();

                                    if stop_bool.load(std::sync::atomic::Ordering::Relaxed) {
                                        break;
                                    }
                                    match audio_queue.pop() {
                                        Ok(value) => {
                                            *sample = value;
                                            break;
                                        }
                                        Err(_) => {}
                                    }
                                },
                            }
                        }
                    },
                    move |err| {},
                    None,
                )
                .expect("Failed to build output stream"),
        );

        match self.audio_stream.as_mut().unwrap().play() {
            Ok(_) => {}
            Err(e) => {}
        }
    }
    pub fn stop(&mut self) {
        self.stop_bool
            .store(true, std::sync::atomic::Ordering::Relaxed);
        drop(self.audio_stream.take());
        self.stop_bool
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn new(send_message: Arc<crossbeam_channel::Sender<GuiMessage>>) -> Self {
        Self {
            audio_stream: None,
            stop_bool: Arc::new(AtomicBool::new(false)),
            send_message,
            current_delay: Arc::new(AtomicU32::new(0)),
            update_delay: Arc::new(AtomicBool::new(false)),
            delete_delay_count: Arc::new(AtomicU32::new(0)),
            sample_rate: Arc::new(AtomicU32::new(0)),
        }
    }
}

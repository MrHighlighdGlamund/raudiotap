use nih_plug::{log::warn, prelude::*};
use std::sync::{
    atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64},
    Arc,
};
pub mod gui;
pub mod utilities {
    pub mod enmus;
    pub mod helper_functions;
    pub mod plugin_parameters;
}
pub mod components {
    pub mod client;
    pub mod server;
    pub mod udp_sender;
}
use byteorder::{LittleEndian, WriteBytesExt};
use utilities::plugin_parameters::RaudiotapParams;
use wavers::ConvertTo;

pub struct Raudiotap {
    params: Arc<RaudiotapParams>,
    send_message: crossbeam_channel::Sender<utilities::enmus::GuiMessage>,
    recv_message: Option<crossbeam_channel::Receiver<utilities::enmus::GuiMessage>>,
    audio_queue: rtrb::Producer<u8>,
    sample_rate: Arc<AtomicU32>,
    // udp_sender: components::udp_sender::UdpSender,
    recv_client: crossbeam_channel::Receiver<components::client::Client>,
    // server: components::server::Server,
}

impl Default for Raudiotap {
    fn default() -> Self {
        let (send_message, recv_message) = crossbeam_channel::unbounded();
        let recv_message = Some(recv_message);
        let (audio_queue_p, audio_queue_c) = rtrb::RingBuffer::<u8>::new(96000 * 64);
        let audio_queue_c = Some(audio_queue_c);
        let sample_rate: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
        let (send_client, recv_client) =
            crossbeam_channel::unbounded::<components::client::Client>();
        let targets_update: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let targets_addr_shared: std::sync::Arc<std::sync::Mutex<Vec<std::net::SocketAddr>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let udp_chunk_size: Arc<AtomicU64> = Arc::new(AtomicU64::new(128));
        let mut server = components::server::Server::new(
            send_message.clone(),
            targets_update.clone(),
            targets_addr_shared.clone(),
            sample_rate.clone(),
            send_client,
            udp_chunk_size.clone(),
        );
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                server.run().await;
            });
        });
        let send_message_for_udp = send_message.clone();
        std::thread::spawn(move || {
            let mut udp_sender =
                components::udp_sender::UdpSender::new(send_message_for_udp.clone(), audio_queue_c, targets_addr_shared, targets_update, udp_chunk_size);
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {


                udp_sender.run().await;


            });
        });

        // server.run();
        // udp_sender.run();
        Self {
            params: Arc::new(RaudiotapParams::default()),
            send_message,
            recv_message,
            audio_queue: audio_queue_p,
            // udp_sender,
            sample_rate,
            recv_client,
            // server,
        }
    }
}

impl Plugin for Raudiotap {
    // AUDIOCALLBACK
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                match self.audio_queue.write_chunk_uninit(2) {
                    Ok(chunk) => {
                        chunk.fill_from_iter(f32_to_i16(*sample).to_le_bytes());
                    }
                    Err(_) => {}
                }
            }
        }
        ProcessStatus::Normal
    }
    // AUDIOCALLBACK
    const NAME: &'static str = "Raudiotap";
    const VENDOR: &'static str = "MrHighlighdGlamund";
    const URL: &'static str = "www.golem.de";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let recv_message = self.recv_message.take();

        gui::gui(
            self,
            _async_executor,
            recv_message,
            self.recv_client.clone(),
            self.sample_rate.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        println!("sample rate");
        let sample_rate = buffer_config.sample_rate as u32;
        self.sample_rate
            .store(sample_rate, std::sync::atomic::Ordering::SeqCst);
        // self.server
        //     .sample_rate
        //     .store(sample_rate, std::sync::atomic::Ordering::SeqCst);
        true
    }
}
fn f32_to_i16(s: f32) -> i16 {
    s.convert_to()
}

impl ClapPlugin for Raudiotap {
    const CLAP_ID: &'static str = "com.mrhighlightglamund.raudiotap";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("raudiotap");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Raudiotap {
    const VST3_CLASS_ID: [u8; 16] = *b"1234567raudiotap";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
nih_export_clap!(Raudiotap);
nih_export_vst3!(Raudiotap);

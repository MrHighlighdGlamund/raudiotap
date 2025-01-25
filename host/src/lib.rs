use std::sync::Arc;

use byteorder::{LittleEndian, WriteBytesExt};
use nih_plug::{log::warn, prelude::*};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Vec2},
    resizable_window::ResizableWindow,
    widgets, EguiState,
};



pub struct Raudiotap {
    params: Arc<GainParams>,
}

mod gui;
#[derive(Params)]
pub struct GainParams {
    editor_state: Arc<EguiState>,
}
impl Default for GainParams {
    fn default() -> Self {
        let (width, height) = (500, 300);

        Self {
            editor_state: EguiState::from_size(width, height),
        }
    }
}

impl Default for Raudiotap {
    fn default() -> Self {


        Self {
            params: Arc::new(GainParams::default()),
        }
    }
}

impl Plugin for Raudiotap {
    const NAME: &'static str = "VSTtoNETWORK";
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
        gui::gui(self, _async_executor)
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
            }
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Raudiotap {
    const CLAP_ID: &'static str = "com.mrhighlightglamund.vstToNetwork";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("vstToNetwork");
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
    const VST3_CLASS_ID: [u8; 16] = *b"1234vstToNetwork";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}



nih_export_clap!(Raudiotap);
nih_export_vst3!(Raudiotap);

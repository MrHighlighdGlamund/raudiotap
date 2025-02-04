use components::server::Server;
use egui_wgpu::wgpu;
use egui_winit::winit;
use utilities::enmus::GuiMessage;
use std::sync::{Arc, OnceLock};
use std::sync::atomic::AtomicBool;

use jni::{objects::JObject, sys::JNIEnv};
// use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget};
pub mod gui;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

// use winit::event_loop::ControlFlow;

// use egui_wgpu::winit::Painter;
// use egui_winit::State;
//use egui_winit_platform::{Platform, PlatformDescriptor};
use winit::event::Event::*;
const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;
pub mod utilities {
    pub mod enmus;
    pub mod helper_functions;
}

pub mod components {
    pub mod audio;
    pub mod server;
    pub mod udp_reciver;
}
static STOP_BACKGROUND_SERVICE: OnceLock<Arc<AtomicBool>> = OnceLock::new();

static CHANNEL_TO_GUI: OnceLock<(
    crossbeam_channel::Sender<GuiMessage>,
    crossbeam_channel::Receiver<GuiMessage>,
)> = OnceLock::new();
static CHANNEL_TO_SERVER: OnceLock<(
    crossbeam_channel::Sender<GuiMessage>,
    crossbeam_channel::Receiver<GuiMessage>,
)> = OnceLock::new();
fn initialize_channels() {
    // CHANNEL_TO_SERVER.set(crossbeam_channel::unbounded()).unwrap();
    // CHANNEL_TO_GUI.set(crossbeam_channel::unbounded()).unwrap();
    if CHANNEL_TO_SERVER
        .set(crossbeam_channel::unbounded())
        .is_ok()
    {}
    if CHANNEL_TO_GUI.set(crossbeam_channel::unbounded()).is_ok() {}
    if STOP_BACKGROUND_SERVICE
        .set(Arc::new(AtomicBool::new(false)))
        .is_ok()
    {}
}


#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    initialize_channels();
    use egui_winit::winit::event_loop::EventLoopBuilder;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace) // Default comes from `log::max_level`, i.e. Off
            .with_filter(
                android_logger::FilterBuilder::new()
                    .filter_level(log::LevelFilter::Debug)
                    //.filter_module("android_activity", log::LevelFilter::Trace)
                    //.filter_module("winit", log::LevelFilter::Trace)
                    .build(),
            ),
    );

    let event_loop = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    // _main(event_loop);
    gui::gui_thread(
            event_loop,
            CHANNEL_TO_SERVER.get().unwrap().0.clone(),
            CHANNEL_TO_GUI.get().unwrap().1.clone(),
            STOP_BACKGROUND_SERVICE.get().unwrap().clone(),
        );

}

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn) // Default Log Level
        .parse_default_env()
        .init();

    let event_loop = EventLoopBuilder::with_user_event().build();
    _main(event_loop);
}
#[no_mangle]
pub extern "C" fn Java_com_glamund_raudiotap_RustCall_start_1audio_1service(env: JNIEnv, _: JObject) {
    initialize_channels();
    let mut server = Server::new(
        CHANNEL_TO_GUI.get().unwrap().0.clone(),
        CHANNEL_TO_SERVER.get().unwrap().1.clone(),
        STOP_BACKGROUND_SERVICE.get().unwrap().clone(),
    );
    server.run();
}

#[no_mangle]
pub extern "C" fn Java_com_glamund_raudiotap_RustCall_stop_1audio_1service(env: JNIEnv, _: JObject) {
    STOP_BACKGROUND_SERVICE
        .get()
        .unwrap()
        .store(true, std::sync::atomic::Ordering::SeqCst);
    std::process::exit(0);

    // Implementation of the start_audio_service function
}

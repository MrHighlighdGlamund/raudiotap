use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use components::server::Server;
use egui_wgpu::winit::Painter;
use egui_winit::State;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::JNIEnv;
use ndk_context::android_context;
use utilities::enmus::GuiMessage;
use winit::event::Event::*;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

pub mod gui;
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
#[cfg(any(target_os = "ios", target_os = "android"))]
fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
            std::process::abort()
        }
    }
}

#[cfg(target_os = "ios")]
fn _start_app() {
    stop_unwind(|| main());
}

#[no_mangle]
#[inline(never)]
#[cfg(target_os = "ios")]
pub extern "C" fn start_app() {
    _start_app();
}

#[cfg(not(target_os = "android"))]
pub fn main() {
    initialize_channels();
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .init();

    let event_loop = EventLoopBuilder::with_user_event().build();

    gui::gui_thread(
        event_loop,
        CHANNEL_TO_SERVER.get().unwrap().0.clone(),
        CHANNEL_TO_GUI.get().unwrap().1.clone(),
    );
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    initialize_channels();
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Warn),
    );
    let android_context = android_context();
    let vm = unsafe { jni::JavaVM::from_raw(android_context.vm().cast()) }.unwrap();
    let mut env = vm.attach_current_thread().unwrap();

    // Get the Activity
    let context = unsafe { JObject::from_raw(android_context.context().cast()) };
    start_foreground_service(&mut env, context);

    let event_loop = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    stop_unwind(|| {
        gui::gui_thread(
            event_loop,
            CHANNEL_TO_SERVER.get().unwrap().0.clone(),
            CHANNEL_TO_GUI.get().unwrap().1.clone(),
        )
    });
}

#[no_mangle]
pub extern "C" fn Java_com_example_raud_1service_RustCall_start_1audio_1service(
    env: JNIEnv,
    _: JObject,
) {
    initialize_channels();
    let mut server = Server::new(
        CHANNEL_TO_GUI.get().unwrap().0.clone(),
        CHANNEL_TO_SERVER.get().unwrap().1.clone(),
        STOP_BACKGROUND_SERVICE.get().unwrap().clone(),
    );
    server.run();
    // std::thread::spawn(move || loop {

    //     std::thread::sleep(std::time::Duration::from_secs(1));
    // });
}

#[no_mangle]
pub extern "C" fn Java_com_example_raud_1service_RustCall_stop_1audio_1service(
    env: JNIEnv,
    _: JObject,
) {
    // Implementation of the start_audio_service function
    for i in 0..100 {
        println!("Stopping audio service");
    }
    STOP_BACKGROUND_SERVICE
        .get()
        .unwrap()
        .store(true, std::sync::atomic::Ordering::SeqCst);
}

#[no_mangle]
fn start_foreground_service(env: &mut JNIEnv, context: JObject) {
    let class_loader = env
        .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .expect("Failed to get ClassLoader")
        .l()
        .unwrap();

    let service_class = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(
                &env.new_string("com/example/raud_service/RaudServ").unwrap(),
            )],
        )
        .expect("Failed to load RaudServ class")
        .l()
        .unwrap();

    let service_class_obj = JObject::from(service_class);

    let intent = env
        .new_object("android/content/Intent", "()V", &[])
        .expect("Failed to create new Intent object");

    env.call_method(
        &intent,
        "setClass",
        "(Landroid/content/Context;Ljava/lang/Class;)Landroid/content/Intent;",
        &[JValue::Object(&context), JValue::Object(&service_class_obj)],
    )
    .expect("Failed to set class for Intent");

    env.call_method(
        &context,
        "startService",
        "(Landroid/content/Intent;)Landroid/content/ComponentName;",
        &[JValue::Object(&intent)],
    )
    .expect("Failed to start service");
}

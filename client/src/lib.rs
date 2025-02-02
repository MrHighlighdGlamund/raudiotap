#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
use ndk_context::android_context;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::JNIEnv;


use winit::event::Event::*;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget};

use egui_wgpu::winit::Painter;
use egui_winit::State;

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;
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
// pub use components::server::Server;
// use components::audio::Audio;
// use components::udp_reciver::UdpReciver;
/// A custom event type for the winit app.
enum Event {
    RequestRedraw,
}

/// Enable egui to request redraws via a custom Winit event...
#[derive(Clone)]
struct RepaintSignal(std::sync::Arc<std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>>);

fn create_window<T>(
    event_loop: &EventLoopWindowTarget<T>,
    state: &mut State,
    painter: &mut Painter,
) -> winit::window::Window {
    let window = winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("RaudioTap")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .build(event_loop)
        .unwrap();

    pollster::block_on(painter.set_window(Some(&window))).unwrap();

    // NB: calling set_window will lazily initialize render state which
    // means we will be able to query the maximum supported texture
    // dimensions
    if let Some(max_size) = painter.max_texture_side() {
        state.set_max_texture_side(max_size);
    }

    let pixels_per_point = window.scale_factor() as f32;
    state.set_pixels_per_point(pixels_per_point);

    window.request_redraw();

    window
}

fn _main(event_loop: EventLoop<Event>) {
    let ctx = egui::Context::default();
    let repaint_signal = RepaintSignal(std::sync::Arc::new(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));
    ctx.set_request_repaint_callback(move |_| {
        repaint_signal
            .0
            .lock()
            .unwrap()
            .send_event(Event::RequestRedraw)
            .ok();
    });

    let mut state = State::new(&event_loop);
    let mut painter = Painter::new(
        egui_wgpu::WgpuConfiguration::default(),
        1, // msaa samples
        None,
        false,
    );
    let mut window: Option<winit::window::Window> = None;
    let mut egui_demo_windows = egui_demo_lib::DemoWindows::default();

    event_loop.run(move |event, event_loop, control_flow| match event {
        Resumed => match window {
            None => {
                window = Some(create_window(event_loop, &mut state, &mut painter));
            }
            Some(ref window) => {
                pollster::block_on(painter.set_window(Some(window))).unwrap();
                window.request_redraw();
            }
        },
        Suspended => {
            window = None;
        }
        RedrawRequested(..) => {
            if let Some(window) = window.as_ref() {
                let raw_input = state.take_egui_input(window);

                let full_output = ctx.run(raw_input, |ctx| {
                    egui_demo_windows.ui(ctx);
                });
                state.handle_platform_output(window, &ctx, full_output.platform_output);

                painter.paint_and_update_textures(
                    state.pixels_per_point(),
                    egui::Rgba::default().to_array(),
                    &ctx.tessellate(full_output.shapes),
                    &full_output.textures_delta,
                    false,
                );

                if full_output.repaint_after.is_zero() {
                    window.request_redraw();
                }
            }
        }
        MainEventsCleared | UserEvent(Event::RequestRedraw) => {
            if let Some(window) = window.as_ref() {
                window.request_redraw();
            }
        }
        WindowEvent { event, .. } => {
            match event {
                winit::event::WindowEvent::Resized(size) => {
                    painter.on_window_resized(size.width, size.height);
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }

            let response = state.on_event(&ctx, &event);
            if response.repaint {
                if let Some(window) = window.as_ref() {
                    window.request_redraw();
                }
            }
        }
        _ => (),
    });
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
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .init();

    let event_loop = EventLoopBuilder::with_user_event().build();
    gui::gui_thread(event_loop)
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
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
    stop_unwind(|| gui::gui_thread(event_loop));
}

#[no_mangle]
pub extern "C" fn Java_com_example_raud_1service_RustCall_start_1audio_1service(env: JNIEnv, _: JObject) {
    std::thread::spawn(|| loop {
        // Your Rust audio playback logic here
        println!("Audio service started");
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
}

#[no_mangle]
pub extern "C" fn Java_com_example_raud_1service_RustCall_stop_1audio_1service(env: JNIEnv, _: JObject) {
    // Implementation of the start_audio_service function
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
    println!("Audio service stopped");
}
pub extern "C" fn stop_audio_service() {
}
fn start_foreground_service(env: &mut JNIEnv, context: JObject) {
let class_loader = env
        .call_method(
            &context,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .expect("Failed to get ClassLoader")
        .l()
        .unwrap();

let service_class = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&env.new_string("com/example/raud_service/RaudServ").unwrap())],
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


use crate::components::server::Server;
use crate::utilities::enmus::*;
// use android_activity::AndroidApp;
use egui::{Context, Image, ScrollArea, TextBuffer, TextureHandle};
use egui_wgpu::winit::Painter;
use egui_winit::{winit, State};
use winit::event::Event::*;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget};

use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jboolean;
use jni::JNIEnv;

use std::cmp::Ordering;
use std::io::Cursor;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::task::Poll;
use std::thread;
use std::time::Duration;

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;
struct gui_params {
    message: Vec<String>,
    client_state: ClientSate,
    button_color: egui::Color32,
    button_label: String,
    client_delay: f32,
}
impl gui_params {
    fn new() -> Self {
        Self {
            message: Vec::new(),
            client_state: ClientSate::Disconnected,
            button_color: egui::Color32::from_rgb(43, 42, 51),
            button_label: String::new(),
            client_delay: 0.0,
        }
    }
}

fn gui_build(
    ctx: &egui::Context,
    sender: &mut crossbeam_channel::Sender<GuiMessage>,
    receiver: &mut crossbeam_channel::Receiver<ServerMessage>,
    gui_params: &mut gui_params,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let duration = std::time::Instant::now();
        if let Ok(msg) = receiver.try_recv() {
            match msg {
                ServerMessage::Log(msg) => {
                    gui_params.message.push(msg);
                }
                ServerMessage::Connected => {
                    gui_params.client_state = ClientSate::Connected;
                }
                ServerMessage::Disconnected => {
                    gui_params.client_state = ClientSate::Disconnected;
                }
                ServerMessage::Started => {
                    gui_params.client_state = ClientSate::Running;
                }
                ServerMessage::Stopped => {
                    gui_params.client_state = ClientSate::Connected;
                }
                ServerMessage::Delay(delay) => {
                    gui_params.client_delay = delay;
                }
                _ => return,
            }
        }
        match gui_params.client_state {
            ClientSate::Disconnected => {
                gui_params.button_label = "DISCONNECTED".to_string();
                gui_params.button_label.push_str(":Searching for host...");
                gui_params.button_color = egui::Color32::from_rgb(0, 0, 0);
            }
            ClientSate::Connected => {
                gui_params.button_label = "CONNECTED".to_string();
                gui_params.button_label.push_str(":Press to Start");
                gui_params.button_color = egui::Color32::from_rgb(43, 0, 0);
            }
            ClientSate::Running => {
                gui_params.button_label = "RUNNING".to_string();
                gui_params.button_label.push_str(":Press to Stop");
                gui_params.button_color = egui::Color32::from_rgb(0, 39, 0);
            }
        }

        ui.group(|ui| {
            ui.add_sized(
                [ui.available_width(), ui.available_height() / 2.0],
                egui::Button::new(&gui_params.button_label).fill(gui_params.button_color),
            )
            .clicked()
            .then(|| match gui_params.client_state {
                ClientSate::Connected => {
                    sender.send(GuiMessage::Start).unwrap();
                }
                ClientSate::Running => {
                    sender.send(GuiMessage::Stop).unwrap();
                }
                _ => {}
            })
        });
        ui.group(|ui| {
            let ava_size = ui.available_size();
            let buttonsize = egui::vec2(
                ava_size.x / 8.0 - (ui.spacing().item_spacing.x),
                ava_size.y / 8.0 - ui.spacing().item_spacing.y,
            );
            let dragv_size = egui::vec2(
                ava_size.x - 2.0 * (ava_size.x / 4.0) ,
                ava_size.y / 8.0 - ui.spacing().item_spacing.y,
            );
            ui.horizontal(|ui| {
                ui.add_sized(buttonsize, egui::Button::new("<<<<<"))
                    .clicked()
                    .then(|| {
                        if gui_params.client_delay > 0.0 {
                            gui_params.client_delay -= 0.10;
                            sender
                                .send(GuiMessage::Delay(gui_params.client_delay))
                                .unwrap();
                        }
                    });
                ui.add_sized(buttonsize, egui::Button::new("<<<"))
                    .clicked()
                    .then(|| {
                        if gui_params.client_delay > 0.0 {
                            gui_params.client_delay -= 0.01;
                            sender
                                .send(GuiMessage::Delay(gui_params.client_delay))
                                .unwrap();
                        }
                    });
                ui.add_sized(
                    dragv_size,
                    egui::DragValue::new(&mut gui_params.client_delay)
                        .clamp_range(0.0..=2000.0)
                        .suffix("  Delay ms")
                        .speed(0.80),
                )
                .drag_released()
                .then(|| {
                    sender
                        .send(GuiMessage::Delay(gui_params.client_delay))
                        .unwrap();
                });

                ui.add_sized(buttonsize, egui::Button::new(">>>"))
                    .clicked()
                    .then(|| {
                        if gui_params.client_delay < 2000.0 {
                            gui_params.client_delay += 0.01;
                            sender
                                .send(GuiMessage::Delay(gui_params.client_delay))
                                .unwrap();
                        }
                    });
                ui.add_sized(buttonsize, egui::Button::new(">>>>>"))
                    .clicked()
                    .then(|| {
                        if gui_params.client_delay < 2000.0 {
                            gui_params.client_delay += 0.10;
                            sender
                                .send(GuiMessage::Delay(gui_params.client_delay))
                                .unwrap();
                        }
                    });
            });
        });

        ui.group(|ui| {
            ui.label(format!("Logs:"));
            // `ui.group` creates a group in the UI
            ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false; 2])
                .max_height(ui.available_height())
                .max_width(ui.available_width())
                .show(ui, |ui| {
                    // Create a vertical scroll area
                    for msg in gui_params.message.iter() {
                        // Iterate over messages
                        ui.label(msg); // Display each message as a label
                    }
                });
        });

        thread::sleep(Duration::from_millis(16) - duration.elapsed());
    });
}

pub fn gui_thread(
    event_loop: EventLoop<Event>,
    sender: crossbeam_channel::Sender<GuiMessage>,
    receiver: crossbeam_channel::Receiver<ServerMessage>,
    stop: Arc<AtomicBool>,
) {
    // let mut server = Server::new();
    // server.run();
    let mut gui_params = gui_params::new();
    let mut sender = sender.clone();
    let mut receiver = receiver.clone();
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
    let stop = stop.clone();
    // let thread = std::thread::spawn(move || {});
    event_loop.run(move |event, event_loop, control_flow| {
        match event {
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
                        gui_build(ctx, &mut sender, &mut receiver, &mut gui_params);
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
                // app.poll_events(None, |event| {
                //                 match event {
                //                     _ => {println!("Event: {:#?}", event);}
                //                 }
                //             })
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
                    winit::event::WindowEvent::Destroyed => {}
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

            LoopDestroyed => {}
            _ => (),
        }
    });
}
pub enum Event {
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
        .with_title("egui winit + wgpu example")
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

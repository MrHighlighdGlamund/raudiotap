use crate::components::server::Server;
use crate::utilities::enmus::GuiMessage;
use egui::{ScrollArea, TextBuffer};
use egui_wgpu::winit::Painter;
use egui_winit::State;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::thread;

use winit::event::Event::*;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopWindowTarget};

const INITIAL_WIDTH: u32 = 1920;
const INITIAL_HEIGHT: u32 = 1080;
struct gui_params {
    underrun_count: u32,
    message: Vec<String>,
}
impl gui_params {
    fn new() -> Self {
        Self {
            underrun_count: 0,
            message: Vec::new(),
        }
    }
}

fn gui_build(ctx: &egui::Context, server: &mut Server, gui_params: &mut gui_params) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let duration = std::time::Instant::now();
        if let Ok(msg) = server.recv_message.try_recv() {
            match msg {
                GuiMessage::BufferUnderrun => {
                    gui_params.underrun_count += 1;
                }
                GuiMessage::ServerError(msg) => {
                    gui_params.message.push(msg);
                }
                GuiMessage::Log(msg) => {
                    gui_params.message.push(msg);
                }
                _ => println!("Unknown message"),
            }
        }
        let button = ui.add(egui::Button::new("Start"));

        if button.clicked() {}
        ui.group(|ui| {
            ui.label(format!("Underrun count: {}", gui_params.underrun_count));
        });


        if !gui_params.message.is_empty() {
            ui.group(|ui| {
                // `ui.group` creates a group in the UI
                ScrollArea::vertical().show(ui, |ui| {
                    // Create a vertical scroll area
                    for msg in gui_params.message.iter() {
                        // Iterate over messages
                        ui.label(msg); // Display each message as a label
                    }
                });
            });
        }

        thread::sleep(Duration::from_millis(16) - duration.elapsed());

    });

}

pub fn gui_thread(event_loop: EventLoop<Event>) {
    let mut server = Server::new();
    server.run();
    let mut gui_params = gui_params::new();
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
                    gui_build(ctx, &mut server, &mut gui_params);
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

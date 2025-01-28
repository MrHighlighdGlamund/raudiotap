use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
    u32,
};

use eframe::egui::Label;
use nih_plug::{log::warn, prelude::*};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Align, Direction, FontId, Frame, Layout, TextStyle, Vec2},
    resizable_window::ResizableWindow,
    widgets, EguiState,
};

use crate::{utilities::enmus::GuiMessage, Raudiotap};

pub fn gui(
    plugin: &mut Raudiotap,
    _async_executor: AsyncExecutor<Raudiotap>,
    mut recv_message: Option<crossbeam_channel::Receiver<GuiMessage>>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let egui_state = params.editor_state.clone();
    let server = plugin.server.clone();
    let recv_message: crossbeam_channel::Receiver<GuiMessage> = recv_message.take().unwrap();
    let update_targets = Arc::new(AtomicBool::new(false));

    create_egui_editor(
        plugin.params.editor_state.clone(),
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            ResizableWindow::new("res-wind")
                .min_size(Vec2::new(500.0, 300.0))
                .show(egui_ctx, egui_state.as_ref(), |ui| {
                    let duration = std::time::Instant::now();
                    ui.label("Raudiotap");
                    // for msg in recv_message.try_iter() {
                    //     match msg {
                    //         _ => println!("Unknown message"),
                    //     }
                    // }
                    if update_targets.load(Ordering::Relaxed) {
                        server.targets_addr_shared.lock().unwrap().clear();
                        for client in server.clients.lock().unwrap().iter_mut() {
                            if client.is_running {
                                server
                                .targets_addr_shared
                                .lock()
                                .unwrap()
                                .push(client.udp_addr);
                                client.start();
                            }
                            else {
                                client.stop();
                            }
                                                    }

                        update_targets.store(false, Ordering::Relaxed);
                        server.update_udp_thread.store(true, Ordering::Relaxed);
                    }

                    let clients_len = server.clients.lock().unwrap().len();
                    if clients_len != 0 {
                        ui.group(|ui| {
                            let mut start_client = false;
                            let clients_panel_size = ui.available_size();
                            let clients_panel_size_x = clients_panel_size.x / 2 as f32
                                - ui.spacing().item_spacing.x
                                - 10.0;
                            let clients_panel_size_y = clients_panel_size.y / clients_len as f32
                                - ui.spacing().item_spacing.y
                                - 10.0;

                            for client in server.clients.lock().unwrap().iter_mut() {
                                ui.horizontal(|ui| {
                                    ui.group(|ui| {
                                        let mut button_label = client.name.to_string();

                                        let button_color = if client.is_running {
                                            button_label.push_str(" (running)");
                                            egui::Color32::from_rgb(255, 62, 4)
                                        } else {
                                            button_label.push_str(" (stopped)");
                                            egui::Color32::from_rgb(43, 42, 51)
                                        };

                                        let client_widgets = ui.add_sized(
                                            Vec2::new(clients_panel_size_x, clients_panel_size_y),
                                            egui::Button::new(
                                                // client.is_running.load(Ordering::Relaxed),
                                                button_label,
                                            )
                                            .fill(button_color),
                                        );

                                        if client_widgets.clicked() {
                                            update_targets.store(true, Ordering::Relaxed);
                                            client.is_running = !client.is_running;
                                        }
                                        ui.group(|ui| {
                                            let mut delay_in_ms =
                                                client.delay_in_ms.load(Ordering::Relaxed);
                                            let result = ui.add_sized(
                                                [
                                                    ui.available_width()
                                                        - ui.spacing().item_spacing.x,
                                                    ui.available_height()
                                                        - ui.spacing().item_spacing.y,
                                                ],
                                                egui::DragValue::new(&mut delay_in_ms)
                                                    .clamp_range(0.0..=2000.0)
                                                    .suffix(" ms")
                                                    .speed(0.80),
                                            );
                                            if result.drag_stopped() {}

                                            if result.changed() {
                                                let sample_rate =
                                                    server.sample_rate.load(Ordering::Relaxed);

                                                let delay_in_samples = (delay_in_ms as f32 / 1000.0
                                                    * sample_rate as f32
                                                    * 2.0)
                                                    as u32;
                                                client
                                                    .delay_in_ms
                                                    .store(delay_in_ms, Ordering::Relaxed);
                                                client
                                                    .delay_in_samples
                                                    .store(delay_in_samples, Ordering::Relaxed);
                                            }
                                        });
                                    });
                                });
                            }

                            // if start_client {
                            //     for client in server.clients.lock().unwrap().iter_mut() {
                            //         client.stop_client();
                            //     }
                            //     for client in server.clients.lock().unwrap().iter_mut() {
                            //         if client.starting.load(Ordering::Relaxed) {
                            //             client.start_client();
                            //             client.starting.store(false, Ordering::Relaxed);
                            //         }
                            //     }
                            // }
                        });
                    }

                    thread::sleep(Duration::from_millis(16) - duration.elapsed());
                });
        },
    )
}

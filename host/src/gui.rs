use chrono::{Local, Timelike};
use crossbeam_channel::{Receiver, Sender};
use std::{
    fmt::Debug,
    io::Write,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
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
    egui::{self, Align, Direction, FontId, Frame, Layout, ScrollArea, TextStyle, Vec2},
    resizable_window::ResizableWindow,
    widgets, EguiState,
};

use crate::{components::client::Client, utilities::enmus::GuiMessage, Raudiotap};

pub fn gui(
    plugin: &mut Raudiotap,
    _async_executor: AsyncExecutor<Raudiotap>,
    mut recv_message: Option<crossbeam_channel::Receiver<GuiMessage>>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let egui_state = params.editor_state.clone();
    let server = plugin.server.clone();
    let recv_message: crossbeam_channel::Receiver<GuiMessage> = recv_message.take().unwrap();
    let mut messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new())); //Vec<String> = Vec::new();
    let update_targets = Arc::new(AtomicBool::new(false));
    let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    let recv_clients = server.recv_client.clone();
    let refresh_counter = Arc::new(AtomicI32::new(0));
    create_egui_editor(
        plugin.params.editor_state.clone(),
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            ResizableWindow::new("res-wind")
                .min_size(Vec2::new(500.0, 300.0))
                .show(egui_ctx, egui_state.as_ref(), |ui| {
                    let mut clients = clients.lock().unwrap();
                    let mut messages = messages.lock().unwrap();
                    let duration = std::time::Instant::now();
                    refresh_counter.fetch_add(1, Ordering::Relaxed);
                    recv_message.try_iter().for_each(|msg| match msg {
                        GuiMessage::Log(msg) => {
                            let timestamp = chrono::Local::now();
                            let timestamp = format!(
                                "{:02}:{:02}:{:02}",
                                timestamp.hour(),
                                timestamp.minute(),
                                timestamp.second()
                            );
                            messages.push(format!("{} - {}", timestamp, msg));
                        }
                        GuiMessage::ServerError(msg) => {
                            let timestamp = chrono::Local::now();
                            let timestamp = format!(
                                "{:02}:{:02}:{:02}",
                                timestamp.hour(),
                                timestamp.minute(),
                                timestamp.second()
                            );
                            messages.push(format!("{} - {}", timestamp, msg));
                        }
                        GuiMessage::TextError(msg) => {
                            let timestamp = chrono::Local::now();
                            let timestamp = format!(
                                "{:02}:{:02}:{:02}",
                                timestamp.hour(),
                                timestamp.minute(),
                                timestamp.second()
                            );
                            messages.push(format!("{} - {}", timestamp, msg));
                        }
                        _ => {}
                    });

                    if refresh_counter.load(Ordering::Relaxed) == 200 {
                        clients.retain_mut(|client| client.still_alive());
                        refresh_counter.store(0, Ordering::Relaxed);
                    }
                    for new_client in recv_clients.try_iter() {
                        clients.retain_mut(|client| client.udp_addr != new_client.udp_addr);
                        clients.push(new_client);
                    }
                    if update_targets.load(Ordering::Relaxed) {
                        server.targets_addr_shared.lock().unwrap().clear();
                        clients.retain_mut(|client| {
                            if client.is_running {
                                let sucess = client.start();
                                if !sucess {
                                    return false;
                                }
                                server
                                    .targets_addr_shared
                                    .lock()
                                    .unwrap()
                                    .push(client.udp_addr);
                            } else {
                                let sucess = client.stop();
                                if !sucess {
                                    return false;
                                }
                            }
                            true
                        });
                        update_targets.store(false, Ordering::Relaxed);
                        server.update_udp_thread.store(true, Ordering::Relaxed);
                    }

                    ui.group(|ui| {
                        let mut start_client = false;
                        let clients_panel_size = ui.available_size();
                        let clients_panel_size_x =
                            clients_panel_size.x / 2 as f32 - ui.spacing().item_spacing.x - 10.0;
                        let clients_panel_size_y = clients_panel_size.y / clients.len() as f32
                            - ui.spacing().item_spacing.y
                            - 10.0;

                        for client in clients.iter_mut() {
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
                                        Vec2::new(clients_panel_size_x, clients_panel_size_y / 2.0),
                                        egui::Button::new(button_label).fill(button_color),
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
                                                ui.available_width() - ui.spacing().item_spacing.x,
                                                ui.available_height() - ui.spacing().item_spacing.y,
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
                    });
                    ui.group(|ui| {
                        ui.label("Logs:");
                        egui::ScrollArea::vertical()
                            .max_width(ui.available_width())
                            .max_height(ui.available_height())
                            .auto_shrink([false; 2])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for msg in messages.iter() {
                                    ui.label(msg);
                                }
                            });
                    });
                });
        },
    )
}

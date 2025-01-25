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
use rtrb::Consumer;

use crate::Raudiotap;



pub fn gui(plugin: &mut Raudiotap, _async_executor: AsyncExecutor<Raudiotap>) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let egui_state = params.editor_state.clone();
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

                    
                    thread::sleep(Duration::from_millis(16) -duration.elapsed());
                });
        },
    )
}

use std::thread;
use winit::platform::windows::EventLoopBuilderExtWindows;

use crate::{cell::*, map::*, ui::*};

mod cell;
mod map;
mod ui;
mod evolution;

fn run() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]),
        event_loop_builder: Some(Box::new(|b| {
            b.with_any_thread(true);
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|_| Ok(Box::new(PlantEvolutionApp::new()))),
    )
    .unwrap();
}

fn main() {
    // Main thread stack is not big enough even for 1 MapData
    let h = thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run);
    let _ = h.unwrap().join();
}

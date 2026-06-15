#![feature(vec_from_fn)]

use std::{
    sync::{Arc, mpsc},
    thread,
};

use plant_evolution_lib::{
    engine::run_engine,
    map::MapData,
    populate_consts,
    slow_mutex::SlowMutex,
    ui::PlantEvolutionApp,
};

#[hotpath::main]
fn main() {
    populate_consts();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]),
        ..Default::default()
    };

    let maps = Arc::new(SlowMutex::new(Vec::from_fn(200, |_| MapData::default())));

    let (tx, rx) = mpsc::channel();

    {
        let maps = maps.clone();
        thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                run_engine(rx, maps);
            })
            .unwrap();
    }

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|_| Ok(Box::new(PlantEvolutionApp::new(tx, maps)))),
    )
    .unwrap();
}

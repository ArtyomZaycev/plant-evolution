#![feature(vec_from_fn)]
#![feature(iter_intersperse)]
#![feature(integer_atomics)]

mod ui;

use plant_evolution_lib::{engine::*, map::MapData, precalc::populate_consts};

use crate::ui::ui::PlantEvolutionApp;

#[hotpath::main]
fn main() {
    populate_consts();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]),
        ..Default::default()
    };

    let maps = Vec::from_fn(200, |_| MapData::default());

    let engine = Engine::new(maps, EngineParameters::default());

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|_| Ok(Box::new(PlantEvolutionApp::new(engine)))),
    )
    .unwrap();
}

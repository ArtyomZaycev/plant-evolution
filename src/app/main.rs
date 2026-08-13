#![feature(vec_from_fn)]
#![feature(iter_intersperse)]

mod ui;

use plant_evolution_lib::{
    engine::*, evolution::consts::*, map::MapData, precalc::populate_consts, utils::rng,
};

use crate::ui::{consts::*, ui::PlantEvolutionApp};

#[hotpath::main]
fn main() {
    populate_consts();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(DEFAULT_WINDOW_SIZE),
        ..Default::default()
    };

    let mut maps_rng = rng::get_rng();
    let maps = Vec::from_fn(DEFAULT_NUMBER_OF_PLANTS, |_| MapData::generate(&mut maps_rng));

    let engine = Engine::new(maps, EngineParameters::default());

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|_| {
            Ok(Box::new(PlantEvolutionApp::new(
                engine,
                EngineParameters::default(),
            )))
        }),
    )
    .unwrap();
}

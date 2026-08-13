#![feature(vec_from_fn)]
#![feature(iter_intersperse)]
#![feature(path_trailing_sep)]

mod ui;
mod config;

use plant_evolution_lib::{
    engine::*, evolution::consts::*, map::MapData, precalc::populate_consts, utils::rng,
};

use crate::{config::Config, ui::{consts::*, ui::PlantEvolutionApp}};

#[hotpath::main]
fn main() {
    populate_consts();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(DEFAULT_WINDOW_SIZE),
        ..Default::default()
    };

    let config = Config::load();
    let rng_seed = config.get_locked_seed().unwrap_or(rng::get_seed());
    let mut maps_rng = rng::get_rng_seeded(rng_seed);
    let maps = Vec::from_fn(DEFAULT_NUMBER_OF_PLANTS, |_| MapData::generate(&mut maps_rng));

    let engine = Engine::new(rng_seed, maps, EngineParameters::default());

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|_| {
            Ok(Box::new(PlantEvolutionApp::new(
                engine,
                config,
                EngineParameters::default(),
            )))
        }),
    )
    .unwrap();
}

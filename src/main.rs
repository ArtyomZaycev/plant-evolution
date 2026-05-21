use std::thread;
use winit::platform::windows::EventLoopBuilderExtWindows;

use crate::{cell::*, map::*, ui::*};

mod cell;
mod map;
mod ui;

fn run() {
    let plant_nutrition = PlantNutrition {
        sunlight: 100.,
        air: 100.,
        minerals: 100.,
        water: 100.,
        power: 10.,
    };

    let basic_cell = PlantCellAbilities {
        sunlight_consumption: 0.1,
        air_consumption: 0.1,
        minerals_consumption: 0.1,
        water_consumption: 0.1,
        power_production_speed: 0.1,
        cost: 0.,
    }
    .populate_cost();
    let cells = [
        PlantCellAbilities {
            sunlight_consumption: 1.,
            air_consumption: 1.,
            minerals_consumption: 1.,
            water_consumption: 1.,
            power_production_speed: 1.,
            cost: 0.,
        }
        .populate_cost(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
    ];

    let evolution_data = PlantEvolutionData::generate();

    let map = MapData::generate(cells, evolution_data, plant_nutrition);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        event_loop_builder: Some(Box::new(|b| {
            b.with_any_thread(true);
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|cc| Ok(Box::new(PlantEvolutionApp::new(map)))),
    )
    .unwrap();
}

fn main() {
    let h = thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run);
    let _ = h.unwrap().join();
}

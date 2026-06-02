use plant_evolution_lib::{populate_consts, ui::PlantEvolutionApp};

fn main() {
    populate_consts();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Plant Evolution",
        options,
        Box::new(|_| Ok(Box::new(PlantEvolutionApp::new()))),
    )
    .unwrap();
}

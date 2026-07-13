use egui::{Align, Layout, Slider, Vec2};

use super::utils::*;
use plant_evolution_lib::engine::*;

pub struct PerformanceParametersEditor {
    // Can't be changed as of now
    pub multithreading_enabled: bool,
    pub number_of_threads: u32,

    pub use_local_growth: bool,
    pub slow_updates: bool,
}

impl EditorUi<PerformanceParameters> for PerformanceParametersEditor {
    fn new(settings: PerformanceParameters) -> Self {
        Self {
            multithreading_enabled: settings.multithreading_enabled,
            number_of_threads: settings.number_of_threads,
            use_local_growth: settings.use_local_growth,
            slow_updates: settings.slow_updates,
        }
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn parse(&self) -> Option<PerformanceParameters> {
        if self.is_valid() {
            Some(PerformanceParameters {
                multithreading_enabled: self.multithreading_enabled,
                number_of_threads: self.number_of_threads,
                use_local_growth: self.use_local_growth,
                slow_updates: self.slow_updates,
            })
        } else {
            None
        }
    }
}

impl egui::Widget for &mut PerformanceParametersEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = Vec2::new(120., 10.);
        let layout = Layout::left_to_right(Align::Center)
            .with_main_justify(true)
            .with_main_align(Align::LEFT);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Multithreading")
                });
                ui.add_enabled_ui(false, |ui| {
                    ui.radio_value(&mut self.multithreading_enabled, true, "Enabled");
                    ui.radio_value(&mut self.multithreading_enabled, false, "Disabled");
                });
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Number of threads"));
                ui.add_enabled(false, Slider::new(&mut self.number_of_threads, 2..=64));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Local growth recalculation")
                });
                ui.radio_value(&mut self.use_local_growth, true, "Enabled");
                ui.radio_value(&mut self.use_local_growth, false, "Disabled");
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Slow updates")
                });
                ui.radio_value(&mut self.slow_updates, true, "Enabled");
                ui.radio_value(&mut self.slow_updates, false, "Disabled");
            });
        })
        .response
    }
}

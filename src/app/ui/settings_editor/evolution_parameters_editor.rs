use egui::{Align, Layout, Slider, Vec2};

use super::utils::*;
use plant_evolution_lib::engine::*;

pub struct EvolutionParametersEditor {
    plants: usize,
    samples: usize,
    parent_evolution: bool,
    change_chance: f32,
    change_entropy: f32,
    ticks_per_evolution: u32,
}

impl EditorUi<EvolutionParameters> for EvolutionParametersEditor {
    fn new(settings: EvolutionParameters) -> Self {
        Self {
            plants: settings.plants,
            samples: settings.samples,
            parent_evolution: settings.parent_evolution,
            change_chance: settings.change_chance,
            change_entropy: settings.change_entropy,
            ticks_per_evolution: settings.run_evolution_parameters.ticks_per_evolution,
        }
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn parse(&self) -> Option<EvolutionParameters> {
        if self.is_valid() {
            Some(EvolutionParameters {
                plants: self.plants,
                samples: self.samples,
                parent_evolution: self.parent_evolution,
                change_chance: self.change_chance,
                change_entropy: self.change_entropy,
                run_evolution_parameters: RunEvolutionParameters {
                    ticks_per_evolution: self.ticks_per_evolution,
                    ticks_per_slow_write: RunEvolutionParameters::default().ticks_per_slow_write,
                },
            })
        } else {
            None
        }
    }
}

impl egui::Widget for &mut EvolutionParametersEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = Vec2::new(120., 10.);
        let layout = Layout::left_to_right(Align::Center)
            .with_main_justify(true)
            .with_main_align(Align::LEFT);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Number of plants"));
                ui.add(Slider::new(&mut self.plants, 1..=1000));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Number of samples")
                });
                ui.add(Slider::new(&mut self.samples, 1..=self.plants));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Parents Evolution")
                });
                ui.radio_value(&mut self.parent_evolution, true, "Enabled");
                ui.radio_value(&mut self.parent_evolution, false, "Disabled");
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Evolution chance"));
                ui.add(Slider::new(&mut self.change_chance, 0.05..=1.0));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Evolution entropy")
                });
                ui.add(Slider::new(&mut self.change_entropy, 0.05..=1.0));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Ticks per evolution")
                });
                ui.add(Slider::new(&mut self.ticks_per_evolution, 50..=10000));
            });
        })
        .response
    }
}

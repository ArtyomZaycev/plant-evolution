use crate::{engine::*, ui_settings::basics::{NumericInput, RawSetting}};

pub struct EvolutionParametersRaw {
    plants: NumericInput<usize>,
    samples: NumericInput<usize>,
    parent_evolution: bool,
    change_chance: NumericInput<f32>,
    change_entropy: NumericInput<f32>,
    ticks_per_evolution: NumericInput<u32>,
}

impl RawSetting<EvolutionParameters> for EvolutionParametersRaw {
    fn new(settings: EvolutionParameters) -> Self {
        Self {
            plants: NumericInput::new(settings.plants, 10..=1000).with_label("Number of plants"),
            samples: NumericInput::new(settings.samples, 10..=200).with_label("Amount of samples"),
            parent_evolution: settings.parent_evolution,
            change_chance: NumericInput::new(settings.change_chance, 0.05..=1.).with_label("Change chance"),
            change_entropy: NumericInput::new(settings.change_entropy, 0.05..=1.).with_label("Change entropy"),
            ticks_per_evolution: NumericInput::new(settings.run_evolution_parameters.ticks_per_evolution, 100..=10000).with_label("Ticks per evolution"),
        }
    }

    fn parse(&self) -> Option<EvolutionParameters> {
        if self.plants.is_valid() && self.samples.is_valid() && self.change_chance.is_valid() && self.change_entropy.is_valid() && self.change_entropy.is_valid() && self.ticks_per_evolution.is_valid() {
            Some(EvolutionParameters {
                plants: self.plants.get_value(),
                samples: self.samples.get_value(),
                parent_evolution: self.parent_evolution,
                change_chance: self.change_chance.get_value(),
                change_entropy: self.change_entropy.get_value(),
                run_evolution_parameters: RunEvolutionParameters {
                    ticks_per_evolution: self.ticks_per_evolution.get_value(),
                    ticks_per_slow_write: RunEvolutionParameters::default().ticks_per_slow_write,
                },
            })
        } else {
            None
        }
    }
}

impl egui::Widget for &mut EvolutionParametersRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.add(&mut self.plants);
            ui.add(&mut self.samples);
            ui.toggle_value(&mut self.parent_evolution, "Parents evolution");
            ui.add(&mut self.change_chance);
            ui.add(&mut self.change_entropy);
        }).response
    }
}

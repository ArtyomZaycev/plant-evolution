use crate::{engine::*, ui_settings::settings_trait::RawSetting};


pub struct EvolutionParametersRaw {
    pub plants: String,
    pub samples: String,
    pub parent_evolution: bool,
    pub change_chance: String,
    pub change_entropy: String,
    pub ticks_per_evolution: String,
}

impl RawSetting<(EvolutionParameters, RunEvolutionParameters)> for EvolutionParametersRaw {
    fn new(settings: (EvolutionParameters, RunEvolutionParameters)) -> Self {
        Self {
            plants: 200.to_string(),
            samples: settings.0.samples.to_string(),
            parent_evolution: settings.0.parent_evolution,
            change_chance: settings.0.change_chance.to_string(),
            change_entropy: settings.0.change_entropy.to_string(),
            ticks_per_evolution: settings.1.ticks_per_evolution.to_string(),
        }
    }

    fn parse(&self) -> Option<(EvolutionParameters, RunEvolutionParameters)> {
        let plants = self.plants.parse::<usize>().ok()?;
        let samples = self.samples.parse::<usize>().ok()?;
        let parent_evolution = self.parent_evolution;
        let change_chance = self.change_chance.parse::<f32>().ok()?;
        let change_entropy = self.change_entropy.parse::<f32>().ok()?;
        let ticks_per_evolution = self.ticks_per_evolution.parse::<u32>().ok()?;

        Some((
            EvolutionParameters {
                samples,
                parent_evolution,
                change_chance,
                change_entropy,
                ..EvolutionParameters::default()
            },
            RunEvolutionParameters {
                ticks_per_evolution,
                ..RunEvolutionParameters::default()
            }
        ))
    }
}

impl egui::Widget for &mut EvolutionParametersRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        todo!()
    }
}
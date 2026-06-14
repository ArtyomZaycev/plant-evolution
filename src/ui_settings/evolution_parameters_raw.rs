use crate::{engine::*, ui_settings::settings_trait::RawSetting};

pub struct EvolutionParametersRaw {
    pub plants: String,
    pub samples: String,
    pub parent_evolution: bool,
    pub change_chance: String,
    pub change_entropy: String,
    pub ticks_per_evolution: String,
}

impl RawSetting<EvolutionParameters> for EvolutionParametersRaw {
    fn new(settings: EvolutionParameters) -> Self {
        Self {
            plants: 200.to_string(),
            samples: settings.samples.to_string(),
            parent_evolution: settings.parent_evolution,
            change_chance: settings.change_chance.to_string(),
            change_entropy: settings.change_entropy.to_string(),
            ticks_per_evolution: settings
                .run_evolution_parameters
                .ticks_per_evolution
                .to_string(),
        }
    }

    fn parse(&self) -> Option<EvolutionParameters> {
        let plants = self.plants.parse::<usize>().ok()?;
        let samples = self.samples.parse::<usize>().ok()?;
        let parent_evolution = self.parent_evolution;
        let change_chance = self.change_chance.parse::<f32>().ok()?;
        let change_entropy = self.change_entropy.parse::<f32>().ok()?;
        let ticks_per_evolution = self.ticks_per_evolution.parse::<u32>().ok()?;

        Some(EvolutionParameters {
            plants,
            samples,
            parent_evolution,
            change_chance,
            change_entropy,
            run_evolution_parameters: RunEvolutionParameters {
                ticks_per_evolution,
                ticks_per_slow_write: RunEvolutionParameters::default().ticks_per_slow_write,
            },
        })
    }
}

impl egui::Widget for &mut EvolutionParametersRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        todo!()
    }
}

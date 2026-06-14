use crate::{engine::EngineParameters, ui::VisualSettings, ui_settings::basics::RawSetting};

use super::{evolution_parameters_raw::*, saving_parameters_raw::*, visual_settings_raw::*};

pub enum SettingsRawState {
    InProgress,
    Cancelled,
    Applied(VisualSettings, EngineParameters),
}

pub struct SettingsRaw {
    tab: usize,
    state: SettingsRawState,

    visual_settings: VisualSettingsRaw,
    saving_parameters: SavingParametersRaw,
    evolution_parameters: EvolutionParametersRaw,
}

impl SettingsRaw {
    pub fn get_state(&self) -> &SettingsRawState {
        &self.state
    }
}

impl RawSetting<(VisualSettings, EngineParameters)> for SettingsRaw {
    fn new(settings: (VisualSettings, EngineParameters)) -> Self {
        Self {
            tab: 0,
            state: SettingsRawState::InProgress,
            visual_settings: VisualSettingsRaw::new(settings.0),
            saving_parameters: SavingParametersRaw::new(settings.1.saving_parameters),
            evolution_parameters: EvolutionParametersRaw::new(settings.1.evolution_parameters),
        }
    }

    fn is_valid(&self) -> bool {
        self.visual_settings.is_valid()
            && self.saving_parameters.is_valid()
            && self.evolution_parameters.is_valid()
    }

    fn parse(&self) -> Option<(VisualSettings, EngineParameters)> {
        Some((
            self.visual_settings.parse()?,
            EngineParameters {
                saving_parameters: self.saving_parameters.parse()?,
                evolution_parameters: self.evolution_parameters.parse()?,
            },
        ))
    }
}

impl egui::Widget for &mut SettingsRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add_enabled_ui(matches!(self.state, SettingsRawState::InProgress), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, 0, "Visual");
                ui.selectable_value(&mut self.tab, 1, "Saving");
                ui.selectable_value(&mut self.tab, 2, "Evolution");
            });
            ui.separator();
            match self.tab {
                0 => ui.add(&mut self.visual_settings),
                1 => ui.add(&mut self.saving_parameters),
                2 => ui.add(&mut self.evolution_parameters),
                _ => panic!("Unknown tab"),
            };
            ui.separator();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                if ui.button("Reset all to default").clicked() {
                    self.state = SettingsRawState::Applied(
                        VisualSettings::default(),
                        EngineParameters::default(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .add_enabled(self.is_valid(), egui::Button::new("Apply"))
                        .clicked()
                    {
                        let (ui_settings, engine_parameters) = self.parse().unwrap();
                        self.state = SettingsRawState::Applied(ui_settings, engine_parameters);
                    }
                    if ui.button("Cancel").clicked() {
                        self.state = SettingsRawState::Cancelled;
                    }
                });
            });
        })
        .response
    }
}

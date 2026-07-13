use egui::{Align, Layout};

use plant_evolution_lib::engine::*;

use crate::ui::settings::VisualSettings;

use super::utils::*;

use super::{
    evolution_parameters_editor::*, performance_parameters_editor::*, saving_parameters_editor::*,
    visual_settings_editor::*,
};

pub enum SettingsRawState {
    InProgress,
    Cancelled,
    Applied(VisualSettings, EngineParameters),
}

pub struct AppSettingsEditor {
    tab: usize,
    state: SettingsRawState,

    visual_settings: VisualSettingsEditor,
    saving_parameters: SavingParametersEditor,
    evolution_parameters: EvolutionParametersEditor,
    performance_parameters: PerformanceParametersEditor,
}

impl AppSettingsEditor {
    pub fn get_state(&self) -> &SettingsRawState {
        &self.state
    }
}

impl EditorUi<(VisualSettings, EngineParameters)> for AppSettingsEditor {
    fn new(settings: (VisualSettings, EngineParameters)) -> Self {
        Self {
            tab: 0,
            state: SettingsRawState::InProgress,
            visual_settings: VisualSettingsEditor::new(settings.0),
            saving_parameters: SavingParametersEditor::new(settings.1.saving_parameters),
            evolution_parameters: EvolutionParametersEditor::new(settings.1.evolution_parameters),
            performance_parameters: PerformanceParametersEditor::new(
                settings.1.performance_parameters,
            ),
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
                performance_parameters: self.performance_parameters.parse()?,
            },
        ))
    }
}

impl egui::Widget for &mut AppSettingsEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.with_layout(
            Layout::left_to_right(Align::TOP).with_main_justify(true),
            |ui| ui.heading("Settings"),
        );
        ui.separator();
        ui.add_enabled_ui(matches!(self.state, SettingsRawState::InProgress), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, 0, "Visual");
                ui.selectable_value(&mut self.tab, 1, "Autosaving");
                ui.selectable_value(&mut self.tab, 2, "Evolution");
                ui.selectable_value(&mut self.tab, 3, "Performance");
            });
            ui.separator();
            match self.tab {
                0 => ui.add(&mut self.visual_settings),
                1 => ui.add(&mut self.saving_parameters),
                2 => ui.add(&mut self.evolution_parameters),
                3 => ui.add(&mut self.performance_parameters),
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

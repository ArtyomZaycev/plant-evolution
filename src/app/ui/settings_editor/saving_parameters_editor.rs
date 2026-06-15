use std::time::Duration;

use super::utils::*;
use plant_evolution_lib::engine::*;

pub struct SavingParametersEditor {
    enabled: bool,
    period: SavingPeriod,
    selection: SaveSelection,
}

impl EditorUi<SavingParameters> for SavingParametersEditor {
    fn new(settings: SavingParameters) -> Self {
        Self {
            enabled: settings.enabled,
            period: settings.period,
            selection: settings.selection,
        }
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn parse(&self) -> Option<SavingParameters> {
        if self.is_valid() {
            Some(SavingParameters {
                enabled: self.enabled,
                period: self.period,
                selection: self.selection,
            })
        } else {
            None
        }
    }
}

impl egui::Widget for &mut SavingParametersEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.enabled, true, "Enabled");
                ui.radio_value(&mut self.enabled, false, "Disabled");
            });
            ui.add_enabled_ui(self.enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Period");
                    if ui
                        .radio(
                            matches!(self.period, SavingPeriod::EveryDuration(_)),
                            "Seconds",
                        )
                        .clicked()
                    {
                        self.period = SavingPeriod::EveryDuration(Default::default());
                    }
                    if ui
                        .radio(matches!(self.period, SavingPeriod::EveryTick(_)), "Ticks")
                        .clicked()
                    {
                        self.period = SavingPeriod::EveryTick(Default::default());
                    }
                    if ui
                        .radio(
                            matches!(self.period, SavingPeriod::EveryEvolution(_)),
                            "Evolutions",
                        )
                        .clicked()
                    {
                        self.period = SavingPeriod::EveryEvolution(Default::default());
                    }
                });
                match &mut self.period {
                    SavingPeriod::EveryDuration(duration) => {
                        let mut value = duration.as_secs_f32();
                        ui.add(egui::Slider::new(&mut value, 5.0..=900.));
                        *duration = Duration::from_secs_f32(value);
                    }
                    SavingPeriod::EveryTick(value) => {
                        ui.add(egui::Slider::new(value, 100..=10000));
                    }
                    SavingPeriod::EveryEvolution(value) => {
                        ui.add(egui::Slider::new(value, 1..=10000));
                    }
                };

                ui.horizontal(|ui| {
                    ui.label("Selection");
                    if ui
                        .radio(matches!(self.selection, SaveSelection::All), "All")
                        .clicked()
                    {
                        self.selection = SaveSelection::All;
                    }
                    if ui
                        .radio(matches!(self.selection, SaveSelection::Best(_)), "Best")
                        .clicked()
                    {
                        self.selection = SaveSelection::Best(Default::default());
                    }
                });
                match &mut self.selection {
                    SaveSelection::All => {}
                    SaveSelection::Best(value) => {
                        ui.add(egui::Slider::new(value, 1..=10000));
                    }
                };
            });
        })
        .response
    }
}

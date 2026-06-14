use std::time::Duration;

use crate::{engine::{SaveSelection, SavingParameters, SavingPeriod}, ui_settings::basics::{NumericInput, RawSetting}};

pub struct SavingParametersRaw {
    enabled: bool,

    period: SavingPeriod,
    period_duration: NumericInput<f32>,
    period_value: NumericInput<u32>,

    selection: SaveSelection,
    selection_value: NumericInput<usize>,
}

impl RawSetting<SavingParameters> for SavingParametersRaw {
    fn new(settings: SavingParameters) -> Self {
        Self {
            enabled: settings.enabled,

            period: settings.period,
            period_duration: NumericInput::new(if let SavingPeriod::EveryDuration(duration) = settings.period {
                duration.as_secs_f32()
            } else {
                1.
            }, 1.0..=100.),
            period_value: match settings.period {
                SavingPeriod::EveryDuration(_) => NumericInput::new(0, 1..=100),
                SavingPeriod::EveryTick(value) => NumericInput::new(value, 50..=1000),
                SavingPeriod::EveryEvolution(value) => NumericInput::new(value, 1..=1000),
            },

            selection: settings.selection,
            selection_value: match settings.selection {
                SaveSelection::All => NumericInput::new(1, 1..=100),
                SaveSelection::Best(value) => NumericInput::new(value, 1..=100),
            },
        }
    }

    fn parse(&self) -> Option<SavingParameters> {
        if self.period_duration.is_valid() && self.period_value.is_valid() && self.selection_value.is_valid() {
            Some(SavingParameters { enabled: self.enabled, period: match self.period {
                SavingPeriod::EveryDuration(_) => SavingPeriod::EveryDuration(Duration::from_secs_f32(self.period_duration.get_value())),
                SavingPeriod::EveryTick(_) => SavingPeriod::EveryTick(self.period_value.get_value()),
                SavingPeriod::EveryEvolution(_) => SavingPeriod::EveryEvolution(self.period_value.get_value()),
            }, selection: match self.selection {
                SaveSelection::All => SaveSelection::All,
                SaveSelection::Best(_) => SaveSelection::Best(self.selection_value.get_value()),
            } })
        } else {
            None
        }
    }
}

impl egui::Widget for &mut SavingParametersRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.toggle_value(&mut self.enabled, "Enabled");
            ui.add_enabled_ui(self.enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Period");
                    if ui.radio(matches!(self.period, SavingPeriod::EveryDuration(_)), "Seconds").clicked() {
                        self.period = SavingPeriod::EveryDuration(Default::default());
                    }
                    if ui.radio(matches!(self.period, SavingPeriod::EveryTick(_)), "Ticks").clicked() {
                        self.period = SavingPeriod::EveryTick(Default::default());
                    }
                    if ui.radio(matches!(self.period, SavingPeriod::EveryEvolution(_)), "Evolutions").clicked() {
                        self.period = SavingPeriod::EveryEvolution(Default::default());
                    }
                });
                match self.period {
                    SavingPeriod::EveryDuration(_) => ui.add(&mut self.period_duration),
                    SavingPeriod::EveryTick(_) => ui.add(&mut self.period_value),
                    SavingPeriod::EveryEvolution(_) => ui.add(&mut self.period_value),
                };

                ui.horizontal(|ui| {
                    ui.label("Selection");
                    if ui.radio(matches!(self.selection, SaveSelection::All), "All").clicked() {
                        self.selection = SaveSelection::All;
                    }
                    if ui.radio(matches!(self.selection, SaveSelection::Best(_)), "Best").clicked() {
                        self.selection = SaveSelection::Best(Default::default());
                    }
                });
                match self.selection {
                    SaveSelection::All => {},
                    SaveSelection::Best(_) => {ui.add(&mut self.selection_value);},
                };
            });
        }).response
    }
}

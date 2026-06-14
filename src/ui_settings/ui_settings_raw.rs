use crate::{ui::UiSettings, ui_settings::basics::RawSetting};

pub struct UiSettingsRaw {}

impl RawSetting<UiSettings> for UiSettingsRaw {
    fn new(settings: UiSettings) -> Self {
        Self {

        }
    }

    fn parse(&self) -> Option<UiSettings> {
        Some(UiSettings {  })
    }
}

impl egui::Widget for &mut UiSettingsRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {

        }).response
    }
}

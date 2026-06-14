use crate::{ui::UiSettings, ui_settings::settings_trait::RawSetting};


pub struct UiSettingsRaw {

}

impl RawSetting<UiSettings> for UiSettingsRaw {
    fn new(settings: UiSettings) -> Self {
        todo!()
    }

    fn parse(&self) -> Option<UiSettings> {
        todo!()
    }
}

impl egui::Widget for &mut UiSettingsRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        todo!()
    }
}
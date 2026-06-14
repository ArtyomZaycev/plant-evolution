use crate::{engine::SavingParameters, ui_settings::settings_trait::RawSetting};

pub struct SavingParametersRaw {}

impl RawSetting<SavingParameters> for SavingParametersRaw {
    fn new(settings: SavingParameters) -> Self {
        todo!()
    }

    fn parse(&self) -> Option<SavingParameters> {
        todo!()
    }
}

impl egui::Widget for &mut SavingParametersRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        todo!()
    }
}

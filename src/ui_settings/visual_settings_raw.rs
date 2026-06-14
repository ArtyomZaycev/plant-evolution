use egui::{Align, Color32, Layout, Slider, Vec2};

use crate::{ui::VisualSettings, ui_settings::basics::RawSetting};

pub struct VisualSettingsRaw {
    min_cell_size: f32,
    plant_color: Color32,
    seed_color: Color32,
    air_color: Color32,
    soil_color: Color32,
    highlight_hovered_cell: bool,
    highlight_pointer: bool,
}

impl RawSetting<VisualSettings> for VisualSettingsRaw {
    fn new(settings: VisualSettings) -> Self {
        Self {
            min_cell_size: settings.min_cell_size,
            plant_color: settings.plant_color,
            seed_color: settings.seed_color,
            air_color: settings.air_color,
            soil_color: settings.soil_color,
            highlight_hovered_cell: settings.highlight_hovered_cell,
            highlight_pointer: settings.highlight_pointer,
        }
    }

    fn parse(&self) -> Option<VisualSettings> {
        Some(VisualSettings {
            min_cell_size: self.min_cell_size,
            plant_color: self.plant_color,
            seed_color: self.seed_color,
            air_color: self.air_color,
            soil_color: self.soil_color,
            highlight_hovered_cell: self.highlight_hovered_cell,
            highlight_pointer: self.highlight_pointer,
        })
    }
}

impl egui::Widget for &mut VisualSettingsRaw {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = Vec2::new(150., 10.);
        let layout = Layout::left_to_right(Align::Center)
            .with_main_justify(true)
            .with_main_align(Align::LEFT);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Minimum cell size (pixels)"));
                ui.add(Slider::new(&mut self.min_cell_size, 1.0..=32.0));
            });
            /*ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Number of samples")
                });
                ui.add(Slider::new(&mut self.samples, 1..=self.plants));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Parents Evolution")
                });
                ui.radio_value(&mut self.parent_evolution, true, "Enabled");
                ui.radio_value(&mut self.parent_evolution, false, "Disabled");
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Evolution chance"));
                ui.add(Slider::new(&mut self.change_chance, 0.05..=1.0));
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Evolution entropy")
                });
                ui.add(Slider::new(&mut self.change_entropy, 0.05..=1.0));
            });*/
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Highlight hovered")
                });
                ui.radio_value(&mut self.highlight_hovered_cell, true, "Enabled");
                ui.radio_value(&mut self.highlight_hovered_cell, false, "Disabled");
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Highlight pointer")
                });
                ui.radio_value(&mut self.highlight_pointer, true, "Enabled");
                ui.radio_value(&mut self.highlight_pointer, false, "Disabled");
            });
        })
        .response
    }
}

use egui::{
    Align, Button, Color32, Layout, PopupCloseBehavior, Slider, Vec2, color_picker,
    containers::menu::{self, MenuConfig},
};

use crate::ui::settings::VisualSettings;

use super::utils::*;

pub struct VisualSettingsEditor {
    min_cell_size: f32,
    plant_color: Color32,
    seed_color: Color32,
    air_color: Color32,
    soil_color: Color32,
    hovered_map_border_color: Color32,
    highlighted_map_border_color: Color32,
    highlight_hovered_cell: bool,
    highlight_pointer: bool,
}

impl EditorUi<VisualSettings> for VisualSettingsEditor {
    fn new(settings: VisualSettings) -> Self {
        Self {
            min_cell_size: settings.min_cell_size,
            plant_color: settings.plant_color,
            seed_color: settings.seed_color,
            air_color: settings.air_color,
            soil_color: settings.soil_color,
            hovered_map_border_color: settings.hovered_map_border_color,
            highlighted_map_border_color: settings.highlighted_map_border_color,
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
            hovered_map_border_color: self.hovered_map_border_color,
            highlighted_map_border_color: self.highlighted_map_border_color,
            highlight_hovered_cell: self.highlight_hovered_cell,
            highlight_pointer: self.highlight_pointer,
        })
    }
}

impl egui::Widget for &mut VisualSettingsEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = Vec2::new(150., 10.);
        let layout = Layout::left_to_right(Align::Center)
            .with_main_justify(true)
            .with_main_align(Align::LEFT);
        let show_color_picker = |ui: &mut egui::Ui, color: &mut Color32| {
            let mut menu_button = menu::MenuButton::new("Change")
                .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside));
            menu_button.button = menu_button.button.fill(*color);
            menu_button.ui(ui, |ui| {
                color_picker::color_picker_color32(ui, color, color_picker::Alpha::Opaque);
                if ui.button("Apply").clicked() {
                    egui::Popup::close_all(ui.ctx());
                }
            });
        };
        let color_selection =
            |ui: &mut egui::Ui, label: &str, color: &mut Color32, default_color: Color32| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label(label));
                    ui.horizontal(|ui| {
                        show_color_picker(ui, color);
                        if ui.add(Button::new("Reset").fill(default_color)).clicked() {
                            *color = default_color;
                        }
                    });
                });
            };
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Minimum cell size (pixels)")
                });
                ui.add(Slider::new(&mut self.min_cell_size, 1.0..=32.0));
            });

            color_selection(
                ui,
                "Plant cell color",
                &mut self.plant_color,
                VisualSettings::default().plant_color,
            );
            color_selection(
                ui,
                "Plant seed color",
                &mut self.seed_color,
                VisualSettings::default().seed_color,
            );
            color_selection(
                ui,
                "Air color",
                &mut self.air_color,
                VisualSettings::default().air_color,
            );
            color_selection(
                ui,
                "Soil color",
                &mut self.soil_color,
                VisualSettings::default().soil_color,
            );

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

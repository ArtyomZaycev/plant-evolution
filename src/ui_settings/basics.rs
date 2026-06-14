use std::{ops::RangeInclusive, str::FromStr};

use egui::{Widget, emath::Numeric};

pub trait RawSetting<T> {
    fn new(settings: T) -> Self;

    fn is_valid(&self) -> bool {
        self.parse().is_some()
    }

    fn parse(&self) -> Option<T>;
}

pub struct NumericInput<T: ToString + FromStr + Numeric> {
    show_label: bool,
    show_slider: bool,
    show_text_input: bool,

    label: String,
    range: RangeInclusive<T>,

    string_value: String,
    value: T,
}

impl<T: ToString + FromStr + Numeric> NumericInput<T> {
    pub fn new(value: T, range: RangeInclusive<T>) -> Self {
        Self {
            show_label: false,
            show_slider: true,
            show_text_input: true,
            label: String::default(),
            range,
            string_value: value.to_string(),
            value,
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.update_label(label);
        self
    }

    pub fn update_label(&mut self, label: &str) {
        self.label = label.to_owned();
        self.show_label = true;
    }

    pub fn update_range(&mut self, range: RangeInclusive<T>) {
        // TODO: clamp value
        self.range = range;
    }

    pub fn is_valid(&self) -> bool {
        self.string_value.parse::<T>().is_ok_and(|new_value| self.range.contains(&new_value))
    }

    pub fn get_value(&self) -> T {
        self.value
    }
}

impl<T: ToString + FromStr + Numeric> Widget for &mut NumericInput<T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            if self.show_label {
                ui.label(&self.label);
            }
            if self.show_slider {
                if ui.add(egui::Slider::new(&mut self.value, self.range.clone())).changed() {
                    self.string_value = self.value.to_string();
                }
            }
            if self.show_text_input {
                if self.show_slider {
                    ui.separator();
                }

                let response = ui.add(egui::TextEdit::singleline(&mut self.string_value));
                if ui
                    .add_enabled(
                        self.is_valid(),
                        egui::Button::new("Select"),
                    )
                    .clicked()
                    || (response.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter)))
                {
                    self.value = self.string_value.parse().ok().unwrap();
                    self.string_value = self.value.to_string();
                }
                if response.lost_focus() {
                    self.string_value = self.value.to_string();
                };
            }
        }).response
    }
}

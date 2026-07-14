use std::time::Duration;

use egui::{Align, Layout, Slider, Vec2};

use super::utils::*;
use plant_evolution_lib::{engine::*, utils::DEFAULT_THREAD_COUNT};

pub struct PerformanceParametersEditor {
    // Can be changed only if thread_evolution in enabled
    pub multithreading_enabled: bool,
    pub number_of_threads: u32,

    pub use_local_growth: bool,

    pub enable_updates: bool,
    pub slow_updates: bool,
    pub slow_update_interval: u32,
}

impl EditorUi<PerformanceParameters> for PerformanceParametersEditor {
    fn new(settings: PerformanceParameters) -> Self {
        Self {
            multithreading_enabled: settings.multithreading_enabled,
            number_of_threads: settings.number_of_threads,
            use_local_growth: settings.use_local_growth,
            enable_updates: settings.enable_updates,
            slow_updates: settings.slow_updates,
            slow_update_interval: settings.slow_update_interval.as_millis() as u32,
        }
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn parse(&self) -> Option<PerformanceParameters> {
        if self.is_valid() {
            Some(PerformanceParameters {
                multithreading_enabled: self.multithreading_enabled,
                number_of_threads: self.number_of_threads,
                use_local_growth: self.use_local_growth,
                enable_updates: self.enable_updates,
                slow_updates: self.slow_updates,
                slow_update_interval: Duration::from_millis(self.slow_update_interval as u64),
            })
        } else {
            None
        }
    }
}

impl egui::Widget for &mut PerformanceParametersEditor {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = Vec2::new(120., 10.);
        let layout = Layout::left_to_right(Align::Center)
            .with_main_justify(true)
            .with_main_align(Align::LEFT);
        ui.vertical(|ui| {
            // Multithreading
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Multithreading"));
                ui.add_enabled_ui(cfg!(feature = "thread_evolution"), |ui| {
                    ui.radio_value(&mut self.multithreading_enabled, true, "Enabled");
                    ui.radio_value(&mut self.multithreading_enabled, false, "Disabled");
                });
            });
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Number of threads")
                });
                ui.add_enabled(
                    cfg!(feature = "thread_evolution"),
                    Slider::new(&mut self.number_of_threads, 2..=DEFAULT_THREAD_COUNT),
                );
            });
            ui.separator();

            // Algorithmic optimizations
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                    ui.label("Local growth recalculation")
                });
                ui.radio_value(&mut self.use_local_growth, true, "Enabled");
                ui.radio_value(&mut self.use_local_growth, false, "Disabled");
            });
            ui.separator();

            // Slow updates settings
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Updates"));
                ui.radio_value(&mut self.enable_updates, true, "Enabled");
                ui.radio_value(&mut self.enable_updates, false, "Disabled");
            });
            ui.add_enabled_ui(self.enable_updates, |ui| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(desired_size, layout, |ui| ui.label("Slow updates"));
                    ui.radio_value(&mut self.slow_updates, true, "Enabled");
                    ui.radio_value(&mut self.slow_updates, false, "Disabled");
                });
            });
            ui.add_enabled_ui(self.enable_updates && self.slow_updates, |ui| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(desired_size, layout, |ui| {
                        ui.label("Slow updates interval")
                    });
                    ui.horizontal(|ui| {
                        ui.add(Slider::new(&mut self.slow_update_interval, 20..=5000));
                        ui.label("milliseconds");
                    });
                });
            });
        })
        .response
    }
}

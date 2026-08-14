use std::{cell::RefCell, time::SystemTime};

use egui::{Align2, Color32, Pos2, Rect, Sense, Vec2};
use egui_plot::{Line, Plot, PlotPoints, Points};
use plant_evolution_lib::{map::*, precalc::MAP_SIZE};

use crate::ui::{map::*, settings::VisualSettings};


struct TrailData {
    time: SystemTime,
    total_evolutions: u32,
    map: MapData,
    score: f32,
}

impl TrailData {
    fn new(map: MapData, total_evolutions: u32) -> Self {
        Self {
            time: SystemTime::now(),
            total_evolutions,
            score: map.calculate_score(),
            map,
        }
    }
}

pub struct MapsTrail {
    pub enabled: bool,
    trail: Vec<TrailData>,
    show_map: Option<(usize, Pos2)>,
}

impl MapsTrail {
    pub fn new() -> Self {
        Self {
            enabled: true,
            trail: vec![],
            show_map: None,
        }
    }

    pub fn clear(&mut self) {
        self.trail.clear();
    }

    pub fn last_score(&self) -> Option<f32> {
        self.trail.last().map(|v| v.score)
    }

    pub fn push(&mut self, map: &MapData, total_evolutions: u32) {
        if self.enabled {
            self.trail.push(TrailData::new(map.clone(), total_evolutions));
        }
    }
    
    pub fn show(&mut self, ui: &mut egui::Ui, visual_settings: &VisualSettings) -> egui::Response {
        if self.trail.is_empty() {
            ui.label("No data")
        } else {
            let points = self.trail.iter().map(|data| {
                [data.total_evolutions as f64, data.score as f64]
            }).collect::<Vec<_>>();
            let line = Line::new("trail", points.clone()).allow_hover(false).color(Color32::RED);
            let points = Points::new("points", points).color(Color32::RED).radius(2.);

            let hover_data = RefCell::new(None);
            let plot_responce = Plot::new("evolution_trail")
                .x_axis_label("Total evolutions")
                .y_axis_label("Score")
                .label_formatter(|hover| {
                    if let egui_plot::HoverPosition::NearDataPoint { plot_name: _, position, index } = hover {
                        hover_data.replace(Some((*index, position.clone())));
                    }
                    None
                })
                .view_aspect(2.0)
                .show(ui, |plot_ui| {
                    plot_ui.points(points);
                    plot_ui.line(line);
                });

            let mut weak_open = true;
            if let Some((index, position)) = hover_data.take() {
                self.show_map = Some((index, plot_responce.transform.position_from_point(&position)));
                weak_open = false;
            }
            
            if let Some((index, position)) = self.show_map.clone() {
                let window_id: egui::Id = "plant_history_preview".into();
                let old_window_size = ui.ctx().memory(|mem| mem.area_rect(window_id));

                let window_padding = 8.;
                let response = egui::Window::new(format!("Plant 1 №{}", index + 1))
                    .id(window_id)
                    .fixed_pos(position + Vec2::splat(window_padding))
                    .resizable(true)
                    .collapsible(false)
                    .constrain(false)
                    .show(ui.ctx(), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Score: {}", self.trail[index].score));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                ui.label(format!("{} Total evolutions", self.trail[index].total_evolutions));
                            });
                        });

                        let border_size = 1.;
                        let cell_size = ((ui.available_width() - border_size * 2.) / MAP_SIZE.0 as f32).min((ui.available_height() - border_size * 2.) / MAP_SIZE.1 as f32);
                        let canvas_start = ui.next_widget_position() + Vec2::splat(border_size);
                        let map_size = get_ui_map_size(cell_size);
                        ui.allocate_rect(
                            Rect::from_min_max(
                                canvas_start - Vec2::splat(border_size),
                                canvas_start + map_size + Vec2::splat(border_size),
                            ),
                            Sense::HOVER,
                        );
                        let painter = ui.painter_at(Rect::from_min_size(canvas_start, map_size));
                        draw_map(visual_settings, &painter, &self.trail[index].map, cell_size, None, canvas_start);
                    }).unwrap().response;
                    
                let path_rect = Rect::from_min_max(position - Vec2::splat(window_padding), response.rect.right_bottom() + Vec2::splat(window_padding));
                
                let new_window_size = ui.ctx().memory(|mem| mem.area_rect(window_id));
                match (old_window_size, new_window_size) {
                    (Some(old_window_size), Some(new_window_size)) => if old_window_size != new_window_size {
                        weak_open = false;
                    }
                    _ => {}
                }
                if ui.input(|inp| inp.pointer.is_decidedly_dragging()) {
                    weak_open = false;
                }

                if let Some(hover_pos) = ui.input(|inp| inp.pointer.hover_pos()) {
                    if path_rect.contains(hover_pos) {
                        weak_open = false;
                    }
                }
                /*if response.hovered() {
                    weak_open = false;
                }*/
            }

            if weak_open {
                self.show_map = None;
            }

            ui.label(format!("Memory usage: {} KB", (size_of::<Self>() + self.trail.len() * size_of::<TrailData>()) / 1000));

            plot_responce.response
        }
    }
}
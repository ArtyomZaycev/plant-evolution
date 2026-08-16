use std::{cell::RefCell, time::SystemTime};

use egui::{Color32, Pos2, Rect, Sense, Vec2};
use egui_plot::{Line, Plot, Points};
use plant_evolution_lib::{map::*, precalc::MAP_SIZE};

use crate::ui::{map::*, settings::VisualSettings};

#[derive(Debug, Clone)]
struct TrailData {
    _time: SystemTime,
    total_evolutions: u32,
    map: Option<Box<MapData>>,
    score: f32,
}

impl TrailData {
    fn new(map: Option<MapData>, score: f32, total_evolutions: u32) -> Self {
        Self {
            _time: SystemTime::now(),
            total_evolutions,
            score,
            map: map.map(Box::new),
        }
    }

    fn get_size(&self) -> usize {
        size_of::<Self>()
            + if self.map.is_some() {
                size_of::<MapData>()
            } else {
                0
            }
    }
}

pub struct MapsTrail {
    pub rng_seed: u64,
    pub record: bool,
    pub compress: bool,
    trail: Vec<TrailData>,
    show_map: Option<(usize, Pos2)>,
}

impl MapsTrail {
    pub fn new(rng_seed: u64) -> Self {
        Self {
            rng_seed,
            record: true,
            compress: true,
            trail: vec![],
            show_map: None,
        }
    }

    pub fn clear(&mut self) {
        self.trail.clear();
    }

    fn last_score(&self) -> Option<f32> {
        self.trail.last().map(|v| v.score)
    }

    fn last_evolutions(&self) -> Option<u32> {
        self.get_last_map().map(|map| map.evolution_data.evolutions)
    }

    pub fn last_total_evolutions(&self) -> Option<u32> {
        self.trail.last().map(|v| v.total_evolutions)
    }

    pub fn push(&mut self, map: &MapData, total_evolutions: u32) {
        if self.record {
            let score = map.calculate_score();
            if score > self.last_score().unwrap_or(f32::NEG_INFINITY)
                && total_evolutions > self.last_evolutions().unwrap_or(0)
            {
                match self.get_last_map() {
                    Some(last_map) => {
                        let same_cells = last_map.cells_pos.len() == map.cells_pos.len()
                            && last_map.cells_pos.iter().zip(map.cells_pos.iter()).all(
                                |(&(x1, y1), &(x2, y2))| {
                                    last_map.cells[y1][x1].t == map.cells[y2][x2].t
                                },
                            );
                        if same_cells {
                            self.trail
                                .push(TrailData::new(None, score, total_evolutions));
                        } else {
                            self.trail.push(TrailData::new(
                                Some(map.clone()),
                                score,
                                total_evolutions,
                            ));
                        }
                    }
                    None => {
                        self.trail
                            .push(TrailData::new(Some(map.clone()), score, total_evolutions));
                    }
                }
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, visual_settings: &VisualSettings) -> egui::Response {
        self.record = visual_settings.record_history;
        self.compress = visual_settings.compress_history;

        let points = self
            .trail
            .iter()
            .map(|data| [data.total_evolutions as f64, data.score as f64])
            .collect::<Vec<_>>();
        let line = Line::new("trail", points.clone())
            .allow_hover(false)
            .color(Color32::RED);
        let points = Points::new("points", points).color(Color32::RED).radius(2.);

        let hover_data = RefCell::new(None);
        let plot_responce = Plot::new("evolution_trail")
            .x_axis_label("Total evolutions")
            .y_axis_label("Score")
            .label_formatter(|hover| {
                if let egui_plot::HoverPosition::NearDataPoint {
                    plot_name: _,
                    position,
                    index,
                } = hover
                {
                    hover_data.replace(Some((*index, *position)));
                }
                None
            })
            .view_aspect(2.0)
            .clamp_grid(true)
            .show(ui, |plot_ui| {
                plot_ui.points(points);
                plot_ui.line(line);
            });

        let mut weak_open = true;
        if let Some((index, position)) = hover_data.take() {
            self.show_map = Some((
                index,
                plot_responce.transform.position_from_point(&position),
            ));
            weak_open = false;
        }

        if let Some((index, position)) = self.show_map {
            let window_id: egui::Id = "plant_history_preview".into();
            let old_window_size = ui.ctx().memory(|mem| mem.area_rect(window_id));

            let window_padding = 8.;
            let response = egui::Window::new(format!("Evolution {}", index + 1))
                .id(window_id)
                .fixed_pos(position + Vec2::splat(window_padding))
                .resizable(true)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Score: {}", self.trail[index].score));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!(
                                "{} Total evolutions",
                                self.trail[index].total_evolutions
                            ));
                        });
                    });

                    let border_size = 1.;
                    let cell_size = ((ui.available_width() - border_size * 2.) / MAP_SIZE.0 as f32)
                        .min((ui.available_height() - border_size * 2.) / MAP_SIZE.1 as f32);
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
                    match self.get_last_map_from(index) {
                        Some(map) => {
                            draw_map(
                                visual_settings,
                                &painter,
                                map,
                                cell_size,
                                None,
                                canvas_start,
                            );
                        }
                        None => {
                            ui.label("History is broken");
                        }
                    }
                })
                .unwrap()
                .response;

            let path_rect = Rect::from_min_max(
                position - Vec2::splat(window_padding),
                response.rect.left_top() + Vec2::splat(window_padding),
            )
            .union(response.rect.expand(window_padding));

            let new_window_size = ui.ctx().memory(|mem| mem.area_rect(window_id));
            if let (Some(old_window_size), Some(new_window_size)) =
                (old_window_size, new_window_size)
                && old_window_size != new_window_size
            {
                weak_open = false;
            }
            if ui.input(|inp| inp.pointer.is_decidedly_dragging()) {
                weak_open = false;
            }

            if let Some(hover_pos) = ui.input(|inp| inp.pointer.hover_pos())
                && path_rect.contains(hover_pos)
            {
                weak_open = false;
            }
        }

        if weak_open {
            self.show_map = None;
        }

        ui.horizontal(|ui| {
            ui.label(format!(
                "Memory usage: {:.2} MB",
                (size_of::<Self>() + self.trail.iter().map(|data| data.get_size()).sum::<usize>())
                    as f32
                    / 1_000_000.
            ));
            if ui.button("Clear").clicked() {
                self.cleanup_data();
            }
        });

        plot_responce.response
    }
}

impl MapsTrail {
    fn get_last_map(&self) -> Option<&Box<MapData>> {
        if self.trail.is_empty() {
            None
        } else {
            self.get_last_map_from(self.trail.len() - 1)
        }
    }

    fn get_last_map_from(&self, idx: usize) -> Option<&Box<MapData>> {
        self.trail[..=idx]
            .iter()
            .rev()
            .find_map(|data| data.map.as_ref())
    }

    fn cleanup_data(&mut self) {
        if let Some(last_map) = self.get_last_map() {
            let last_data = self.trail.last().unwrap();
            let last_full_data = TrailData {
                map: Some(last_map.clone()),
                ..last_data.clone()
            };
            self.trail.clear();
            self.trail.push(last_full_data);
        } else {
            self.trail.clear();
        }
    }
}

use std::sync::{Arc, mpsc};

use egui::{Align2, Button, Color32, FontId, Pos2, Rect, Sense, TextEdit, Vec2};

use crate::{
    const_precalc::*,
    engine::{EngineCommand, EngineParameters},
    evolution::*,
    map::*,
    slow_mutex::SlowMutex,
    ui_settings::{
        basics::RawSetting,
        settings_raw::{SettingsRaw, SettingsRawState},
    },
};

#[derive(Debug, Clone)]
pub struct VisualSettings {
    pub min_cell_size: f32,

    pub plant_color: Color32,
    pub seed_color: Color32,

    pub air_color: Color32,
    pub soil_color: Color32,

    pub hovered_map_border_color: Color32,
    pub highlighted_map_border_color: Color32,

    pub highlight_hovered_cell: bool,
    pub highlight_pointer: bool,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            min_cell_size: 4.,
            plant_color: Color32::GREEN,
            seed_color: Color32::RED,
            air_color: Color32::LIGHT_BLUE,
            soil_color: Color32::YELLOW,
            hovered_map_border_color: Color32::PURPLE,
            highlighted_map_border_color: Color32::BLUE,
            highlight_hovered_cell: true,
            highlight_pointer: true,
        }
    }
}

pub struct PlantEvolutionApp {
    visual_settings: VisualSettings,
    settings: Option<SettingsRaw>,

    cell_size: f32,

    selected_map_index_str: String,
    selected_maps_index: Vec<usize>,
    maps: Vec<MapData>,

    maps_version: u128,
    command_sender: mpsc::Sender<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,

    engine_parameters: EngineParameters,
    run: bool,
    run_evolution: bool,

    highlighted_map: Option<usize>,
    hovered_cell: Option<(usize, usize, usize)>,
    highlighted_cell: Option<(usize, usize, usize)>,
    selected_decision_tree: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new(
        sender: mpsc::Sender<EngineCommand>,
        slow_maps: Arc<SlowMutex<Vec<MapData>>>,
    ) -> Self {
        Self {
            visual_settings: VisualSettings::default(),
            settings: None,
            cell_size: 6.,
            selected_map_index_str: "1".to_owned(),
            selected_maps_index: vec![0],
            maps_version: 0,
            command_sender: sender,
            maps: slow_maps.force_read(),
            slow_maps,
            engine_parameters: EngineParameters::default(),
            run: false,
            run_evolution: false,
            highlighted_map: None,
            hovered_cell: None,
            highlighted_cell: None,
            selected_decision_tree: None,
        }
    }

    fn get_selected_map_index_to_str(selected_maps_index: &[usize]) -> String {
        let mut str = vec![];
        let mut is_in_range = vec![false; selected_maps_index.len()];
        for i in 1..selected_maps_index.len() {
            if selected_maps_index[i - 1] + 1 == selected_maps_index[i] {
                is_in_range[i] = true;
            }
        }
        let mut i = 0;
        while i < selected_maps_index.len() {
            let mut j = i;
            while j + 1 < selected_maps_index.len() && is_in_range[j + 1] {
                j += 1;
            }
            if i == j {
                str.push((selected_maps_index[i] + 1).to_string());
            } else if selected_maps_index[i] + 1 == selected_maps_index[j] {
                str.push((selected_maps_index[i] + 1).to_string());
                str.push((selected_maps_index[j] + 1).to_string());
            } else {
                str.push(format!(
                    "{}-{}",
                    selected_maps_index[i] + 1,
                    selected_maps_index[j] + 1
                ));
            }
            i = j + 1;
        }
        str.into_iter()
            .intersperse(", ".to_string())
            .collect::<Vec<_>>()
            .concat()
    }

    fn get_selected_map_index_from_str(
        selected_maps_index_str: &str,
        max_idx: usize,
    ) -> Option<Vec<usize>> {
        let mut str = selected_maps_index_str
            .split(",")
            .map(|s| s.trim())
            .filter(|s| s.len() > 0);
        let res = str.try_fold(vec![], |mut acc, str| {
            if str.contains("-") {
                let sp = str.split("-").map(|s| s.trim()).collect::<Vec<_>>();
                if sp.len() != 2 {
                    None
                } else {
                    sp[0].parse().ok().and_then(|lower_bound: usize| {
                        sp[1].parse().ok().and_then(|upper_bound: usize| {
                            if lower_bound <= upper_bound && upper_bound <= max_idx {
                                acc.extend(lower_bound - 1..=upper_bound - 1);
                                Some(acc)
                            } else {
                                None
                            }
                        })
                    })
                }
            } else {
                str.parse().ok().and_then(|v: usize| {
                    if v <= max_idx {
                        acc.push(v - 1);
                        Some(acc)
                    } else {
                        None
                    }
                })
            }
        });
        res.map(|mut v| {
            v.sort();
            v.dedup();
            v
        })
    }

    fn get_ui_map_size(&self) -> Vec2 {
        Vec2::new(
            MAP_SIZE.0 as f32 * self.cell_size,
            MAP_SIZE.1 as f32 * self.cell_size,
        )
    }

    fn draw_map(&mut self, ui: &mut egui::Ui, map_idx: usize, canvas_start: Pos2) {
        let painter = ui.painter_at(Rect::from_min_size(canvas_start, self.get_ui_map_size()));

        for i in 0..MAP_SIZE.1 {
            for j in 0..MAP_SIZE.0 {
                let rect = Rect::from_min_size(
                    canvas_start
                        + Vec2 {
                            x: j as f32 * self.cell_size,
                            y: i as f32 * self.cell_size,
                        },
                    Vec2 {
                        x: self.cell_size,
                        y: self.cell_size,
                    },
                );

                let color = if self.maps[map_idx].plants[i][j].is_some() {
                    self.visual_settings.plant_color
                } else {
                    match self.maps[map_idx].map[i][j] {
                        MapCell::Air(_) => self.visual_settings.air_color,
                        MapCell::Soil(_) => self.visual_settings.soil_color,
                    }
                };

                let color = if self.visual_settings.highlight_hovered_cell && self.hovered_cell.or(self.highlighted_cell) == Some((map_idx, j, i))
                {
                    Color32::BROWN
                } else {
                    color
                };
                painter.rect_filled(rect, 0., color);

                if self.maps[map_idx].plants[i][j].is_some()
                    && self.maps[map_idx].evolution_data.cells_abilities
                        [self.maps[map_idx].plants[i][j].t]
                        .seed
                {
                    painter.circle_filled(
                        canvas_start
                            + Vec2 {
                                x: j as f32 * self.cell_size + 0.5 * self.cell_size,
                                y: i as f32 * self.cell_size + 0.5 * self.cell_size,
                            },
                        self.cell_size * 0.4,
                        self.visual_settings.seed_color,
                    );
                }
            }
        }

        if self.visual_settings.highlight_pointer {
            ui.ctx().input(|i| i.pointer.interact_pos()).inspect(|pos| {
                painter.circle_filled(*pos, 2., Color32::RED);
            });
        }

        painter.text(
            canvas_start + Vec2::splat(self.cell_size * 2.),
            Align2::LEFT_TOP,
            format!("Plant {}", map_idx + 1),
            FontId::default(),
            Color32::BLACK,
        );
    }

    fn draw_map_border(&self, ui: &mut egui::Ui, canvas_start: Pos2, highlighted: bool) {
        let (border_width, color) = if highlighted {
            (self.cell_size / 2., self.visual_settings.highlighted_map_border_color)
        } else {
            (self.cell_size / 2., self.visual_settings.hovered_map_border_color)
        };
        let min = (canvas_start - Pos2::new(border_width, border_width)).to_pos2();
        let max = canvas_start + self.get_ui_map_size() + Vec2::new(border_width, border_width);
        let rect = Rect::from_min_max(min, max);
        let painter = ui.painter_at(rect);
        painter.rect_stroke(rect, 0., (border_width, color), egui::StrokeKind::Inside);
    }

    fn update_selected_maps(&mut self) {
        if self.selected_maps_index.len() == 0 {
            self.selected_maps_index = vec![0];
        }
        self.selected_maps_index.sort();
        self.selected_map_index_str =
            Self::get_selected_map_index_to_str(&self.selected_maps_index);
        if !self
            .highlighted_map
            .is_some_and(|map_idx| self.selected_maps_index.contains(&map_idx))
        {
            self.highlighted_map = None;
        }
    }
}

impl eframe::App for PlantEvolutionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        if let Some((maps, version)) = self.slow_maps.slow_read_versioned(self.maps_version) {
            self.maps_version = version;
            self.maps = maps;
        }

        let mut close_settings = false;
        if let Some(settings) = &mut self.settings {
            match settings.get_state() {
                SettingsRawState::InProgress => {}
                SettingsRawState::Cancelled => {
                    close_settings = true;
                }
                SettingsRawState::Applied(ui_settings, engine_parameters) => {
                    self.visual_settings = ui_settings.clone();
                    self.engine_parameters = engine_parameters.clone();
                    self.command_sender
                        .send(EngineCommand::UpdateParameters(
                            self.engine_parameters.clone(),
                        ))
                        .unwrap();
                    close_settings = true;
                }
            }
            egui::Modal::new("settings".into()).show(ui.ctx(), |ui| {
                ui.add(settings);
            });
        }
        if close_settings {
            self.settings = None;
        }

        if let Some((map_idx, cell_idx)) = self.selected_decision_tree {
            let evolution_data = &self.maps[map_idx].evolution_data.cells_evolution_data[cell_idx];
            egui::Window::new(format!(
                "Plant {}, cell {} decision tree",
                map_idx + 1,
                cell_idx + 1
            ))
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Cell volatility: {:.2}", evolution_data.volatility));
                ui.label(format!(
                    "Suicide (v={:.2}): {}",
                    evolution_data.suicide_weights.volatility,
                    evolution_data.suicide_weights.get_formula()
                ));
                egui::CollapsingHeader::new("Up")
                    .default_open(true)
                    .show(ui, |ui| {
                        evolution_data.weights[0]
                            .iter()
                            .enumerate()
                            .for_each(|(i, w)| {
                                ui.label(format!(
                                    "{} (v={:.2}): {}",
                                    i + 1,
                                    w.volatility,
                                    w.get_formula()
                                ));
                            });
                    });
                egui::CollapsingHeader::new("Sideways")
                    .default_open(true)
                    .show(ui, |ui| {
                        evolution_data.weights[1]
                            .iter()
                            .enumerate()
                            .for_each(|(i, w)| {
                                ui.label(format!(
                                    "{} (v={:.2}): {}",
                                    i + 1,
                                    w.volatility,
                                    w.get_formula()
                                ));
                            });
                    });
                egui::CollapsingHeader::new("Down")
                    .default_open(true)
                    .show(ui, |ui| {
                        evolution_data.weights[2]
                            .iter()
                            .enumerate()
                            .for_each(|(i, w)| {
                                ui.label(format!(
                                    "{} (v={:.2}): {}",
                                    i + 1,
                                    w.volatility,
                                    w.get_formula()
                                ));
                            });
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("Close").clicked() {
                        self.selected_decision_tree = None;
                    }
                });
            });
        }

        egui::Panel::top("settings").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {});
                if ui.button("Settings").clicked() {
                    self.settings = Some(SettingsRaw::new((
                        self.visual_settings.clone(),
                        self.engine_parameters.clone(),
                    )));
                }
            })
        });

        egui::Panel::left("plants_list").show_inside(ui, |ui| {
            let response =
                ui.add(TextEdit::singleline(&mut self.selected_map_index_str).desired_width(150.));
            let new_selected_map_index_str = Self::get_selected_map_index_from_str(
                &self.selected_map_index_str,
                self.maps.len(),
            );

            if ui
                .add_enabled(
                    new_selected_map_index_str.is_some(),
                    egui::Button::new("Select"),
                )
                .clicked()
                || (response.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter)))
            {
                self.selected_maps_index = new_selected_map_index_str.unwrap();
                self.update_selected_maps();
            }
            if response.lost_focus() {
                self.selected_map_index_str =
                    Self::get_selected_map_index_to_str(&self.selected_maps_index);
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(80.);
                (0..self.maps.len()).for_each(|i| {
                    let already_has = self.selected_maps_index.iter().position(|&idx| idx == i);
                    if ui
                        .selectable_label(already_has.is_some(), format!("Plant {}", i + 1))
                        .clicked()
                    {
                        if ui.input(|inp| inp.modifiers.ctrl) {
                            match already_has {
                                Some(idx) => {
                                    if self.selected_maps_index.len() > 1 {
                                        self.selected_maps_index.remove(idx);
                                    }
                                }
                                None => {
                                    self.selected_maps_index.push(i);
                                }
                            }
                        } else if ui.input(|inp| inp.modifiers.shift) {
                            let range = *self.selected_maps_index.last().unwrap()..=i;
                            let mut new_idx = vec![];
                            for j in range.clone() {
                                if !self.selected_maps_index.contains(&j) {
                                    new_idx.push(j);
                                }
                            }
                            if new_idx.len() == 0 {
                                self.selected_maps_index = self
                                    .selected_maps_index
                                    .iter()
                                    .filter(|&&j| !range.contains(&j))
                                    .cloned()
                                    .collect();
                            } else {
                                self.selected_maps_index.extend(new_idx);
                            }
                        } else {
                            self.selected_maps_index = vec![i];
                        }
                        self.update_selected_maps();
                    }
                });
            });
        });

        egui::Panel::right("control_menu").show_inside(ui, |ui| {
            ui.set_min_width(200.);

            ui.horizontal(|ui| {
                if ui.add_enabled(true, Button::new("Evolve!")).clicked() {
                    self.command_sender.send(EngineCommand::Evolve).unwrap();
                }

                if ui
                    .toggle_value(&mut self.run_evolution, "Run Evolution")
                    .changed()
                {
                    if self.run_evolution {
                        self.command_sender
                            .send(EngineCommand::RunEvolution)
                            .unwrap();
                    } else {
                        self.command_sender
                            .send(EngineCommand::StopRunEvolution)
                            .unwrap();
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Tick!").clicked() {
                    self.command_sender.send(EngineCommand::Tick).unwrap();
                }
                if ui.toggle_value(&mut self.run, "Grow").changed() {
                    if self.run {
                        self.command_sender.send(EngineCommand::RunTick).unwrap();
                    } else {
                        self.command_sender
                            .send(EngineCommand::StopRunTick)
                            .unwrap();
                    }
                };
            });

            if ui.button("Restart").clicked() {
                self.command_sender.send(EngineCommand::Restart).unwrap();
            }

            ui.separator();

            let map_idx = self.hovered_cell.map_or(
                self.highlighted_map.unwrap_or(self.selected_maps_index[0]),
                |(map_idx, _, _)| map_idx,
            );

            ui.heading(format!("Plant {}", map_idx + 1));

            ui.horizontal(|ui| {
                ui.label(format!("Evolutions: {}", self.maps[map_idx].evolutions));
                ui.label(format!("Step: {}", self.maps[map_idx].ticks));
            });
            ui.label(format!(
                "Score: {:.2}",
                calculate_score(&self.maps[map_idx])
            ));

            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label("Nutritions:");
                ui.label(format!(
                    "Sunlight: {:.2}",
                    self.maps[map_idx].plant_nutrition.sunlight
                ));
                ui.label(format!(
                    "Air: {:.2}",
                    self.maps[map_idx].plant_nutrition.air
                ));
                ui.label(format!(
                    "Minerals: {}",
                    self.maps[map_idx].plant_nutrition.minerals
                ));
                ui.label(format!(
                    "Water: {:.2}",
                    self.maps[map_idx].plant_nutrition.water
                ));
                ui.label(format!(
                    "Power: {:.2}",
                    self.maps[map_idx].plant_nutrition.energy
                ));

                let mut new_desision_tree = None;

                self.maps[map_idx]
                    .evolution_data
                    .cells_abilities
                    .iter()
                    .enumerate()
                    .for_each(|(i, cell)| {
                        ui.horizontal_top(|ui| {
                            ui.collapsing(format!("Cell {}", i + 1), |ui| {
                                ui.label(format!(
                                    "Volatility: {:.2}",
                                    self.maps[map_idx].evolution_data.cells_evolution_data[i]
                                        .volatility
                                ));
                                if ui
                                    .add_enabled(
                                        self.selected_decision_tree != Some((map_idx, i)),
                                        Button::new("Decision tree"),
                                    )
                                    .clicked()
                                {
                                    new_desision_tree = Some((map_idx, i));
                                }
                                ui.label(format!("Sunlight: {:.2}", cell.sunlight_consumption));
                                ui.label(format!("Air: {:.2}", cell.air_consumption));
                                ui.label(format!("Minerals: {:.2}", cell.minerals_consumption));
                                ui.label(format!("Water: {:.2}", cell.water_consumption));
                                ui.label(format!("Power: {:.2}", cell.energy_production_speed));
                                ui.label(format!("Seed: {}", cell.seed));
                                ui.label(format!("Grow cost: {:.2}", cell.grow_cost));
                                ui.label(format!("Passive cost: {:.2}", cell.passive_cost));
                            });
                        });
                    });

                if new_desision_tree.is_some() {
                    self.selected_decision_tree = new_desision_tree;
                }

                ui.separator();

                self.highlighted_cell = None;
                if ui
                    .label(format!(
                        "Next growth {:.2} cell {} at {:?}",
                        self.maps[map_idx].next_cell_growth.0,
                        self.maps[map_idx].next_cell_growth.3,
                        (
                            self.maps[map_idx].next_cell_growth.1,
                            self.maps[map_idx].next_cell_growth.2
                        )
                    ))
                    .hovered()
                {
                    self.highlighted_cell = Some((
                        map_idx,
                        self.maps[map_idx].next_cell_growth.1,
                        self.maps[map_idx].next_cell_growth.2,
                    ));
                }
                if ui
                    .label(format!(
                        "Next suicide {:.2} at {:?}",
                        self.maps[map_idx].next_cell_suicide.0,
                        (
                            self.maps[map_idx].next_cell_suicide.1,
                            self.maps[map_idx].next_cell_suicide.2
                        )
                    ))
                    .hovered()
                {
                    if self.maps[map_idx].plants[self.maps[map_idx].next_cell_suicide.2]
                        [self.maps[map_idx].next_cell_suicide.1]
                        .is_some()
                    {
                        self.highlighted_cell = Some((
                            map_idx,
                            self.maps[map_idx].next_cell_suicide.1,
                            self.maps[map_idx].next_cell_suicide.2,
                        ));
                    }
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::Panel::bottom("cell_info")
                .min_size(100.)
                .show_inside(ui, |ui| match self.hovered_cell.or(self.highlighted_cell) {
                    Some((map_idx, x, y)) => {
                        let cell_info = format!("cell_info {:?};", &self.maps[map_idx].map[y][x]);
                        ui.label(format!("({}, {}) => {}", x, y, cell_info));

                        let plant_info = if self.maps[map_idx].plants[y][x].is_some() {
                            format!(
                                "plant {}, sunlight: {:.2}, air: {:.2}, minerals: {:.2}, water: {:.2}",
                                self.maps[map_idx].plants[y][x].t + 1,
                                self.maps[map_idx].plants[y][x].input.sunlight,
                                self.maps[map_idx].plants[y][x].input.air,
                                self.maps[map_idx].plants[y][x].input.minerals,
                                self.maps[map_idx].plants[y][x].input.water
                            )
                        } else {
                            "".to_owned()
                        };
                        ui.label(format!("{}", plant_info));
                        ui.label(format!("{:?}", self.maps[map_idx].plants[y][x]));
                    }
                    None => {
                        ui.label("Nothing selected");
                    }
                });

            self.hovered_cell = None;
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let available = ui.available_size();

                    let min_border_size = self.visual_settings.min_cell_size * 2.;
                    let min_map_width: f32 = self.visual_settings.min_cell_size * MAP_SIZE.0 as f32;
                    let columns = (((available.x - min_border_size) / (min_map_width + min_border_size)).floor() as usize).min(self.selected_maps_index.len()).max(1);
                    let rows = self.selected_maps_index.len().div_ceil(columns);
                    let map_width = (available.x - (columns + 1) as f32 * min_border_size) / columns as f32;
                    if self.selected_maps_index.len() == 1 {
                        self.cell_size = self.visual_settings.min_cell_size.max({
                            ((available.x - 2. * min_border_size) / MAP_SIZE.0 as f32)
                                .min((available.y - 2. * min_border_size) / MAP_SIZE.1 as f32)
                        });
                    } else {
                        self.cell_size = self.visual_settings.min_cell_size.max({
                            map_width / MAP_SIZE.0 as f32
                        });
                    }
                    let map_width = self.cell_size * MAP_SIZE.0 as f32;
                    let map_height = self.cell_size * MAP_SIZE.1 as f32;
                    let border_width = (available.x - columns as f32 * map_width) / (columns + 1) as f32;
                    let border_height = min_border_size;

                    let start_pos = ui.next_widget_position() + Vec2::new(0., border_height);
                    let canvas_reponse = ui.allocate_rect(Rect::from_min_size(start_pos, Vec2::new(
                        border_width * (columns + 1) as f32 + map_width * columns as f32,
                        border_height * rows as f32 + map_height * rows as f32,
                    )), Sense::click());
                    for i in 0..self.selected_maps_index.len() {
                        let map_idx = self.selected_maps_index[i];
                        let row = i / columns;
                        let column = i % columns;

                        let canvas_start = Pos2::new(
                            start_pos.x + border_width + column as f32 * map_width + border_width * column as f32,
                            start_pos.y + map_height * row as f32 + border_height * row as f32
                        );

                        if self.hovered_cell.is_none() && canvas_reponse.hovered() {
                            self.hovered_cell = ui.ctx().input(|i| i.pointer.interact_pos()).and_then(|pos| {
                                let pos = pos - canvas_start;
                                if pos.x < 0. || pos.y < 0. {
                                    None
                                } else {
                                    let x = (pos.x / self.cell_size) as usize;
                                    let y = (pos.y / self.cell_size) as usize;

                                    if x >= MAP_SIZE.0 || y >= MAP_SIZE.1 {
                                        None
                                    } else {
                                        Some((map_idx, x, y))
                                    }
                                }
                            });
                        }

                        let map_rect = Rect::from_min_size(canvas_start, self.get_ui_map_size());
                        if canvas_reponse.hovered() && ui.input(|inp| inp.pointer.hover_pos()).is_some_and(|p| map_rect.contains(p)) {
                            self.draw_map_border(ui, canvas_start, false);
                            if canvas_reponse.clicked() && ui.input(|inp| inp.pointer.primary_clicked()) {
                                match self.highlighted_map {
                                    Some(idx) if idx == map_idx => self.highlighted_map = None,
                                    _ => self.highlighted_map = Some(map_idx),
                                }
                            }
                        }

                        self.draw_map(ui, map_idx, canvas_start);
                        if self.highlighted_map == Some(map_idx) {
                            self.draw_map_border(ui, canvas_start, true);
                        }
                    }
                });
            });
        });
    }
}

use std::{
    sync::{Arc, mpsc},
    time::Duration,
};

use egui::{Button, Color32, Frame, Pos2, Rect, Sense, TextEdit, UiBuilder, Vec2};

use crate::{
    const_precalc::*,
    engine::{EngineCommand, EngineParameters},
    evolution::*,
    map::*,
    slow_mutex::SlowMutex,
};

pub struct SavingEngineParametersInput {
    pub enabled: bool,
    pub period_type: usize,
    pub period_duration: Duration,
    pub period_value: String,
    pub selection_type: usize,
    pub selection_value: String,
}

pub struct PlantEvolutionApp {
    min_cell_size: f32,
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

    hovered_cell: Option<(usize, usize)>,
    highlighted_cell: Option<(usize, usize)>,
    selected_decision_tree: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new(
        sender: mpsc::Sender<EngineCommand>,
        slow_maps: Arc<SlowMutex<Vec<MapData>>>,
    ) -> Self {
        Self {
            min_cell_size: 2.,
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
            hovered_cell: None,
            highlighted_cell: None,
            selected_decision_tree: None,
        }
    }

    fn get_map(&self) -> &MapData {
        &self.maps[self.selected_maps_index[0]]
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

    fn draw_map(&mut self, ui: &mut egui::Ui, map_idx: usize, canvas_start: Pos2) {
        let ui_map_size = Vec2::new(
            MAP_SIZE.0 as f32 * self.cell_size,
            MAP_SIZE.1 as f32 * self.cell_size,
        );
        let response = ui.allocate_response(ui.available_size(), Sense::empty());
        let painter = ui.painter_at(Rect::from_min_size(canvas_start, ui_map_size));

        let pointer_pos: Option<Pos2> = ui.ctx().input(|i| i.pointer.latest_pos());
        self.hovered_cell = pointer_pos.and_then(|pos| {
            let pos = pos - canvas_start;
            if pos.x < 0. || pos.y < 0. {
                None
            } else {
                let x = (pos.x / self.cell_size) as usize;
                let y = (pos.y / self.cell_size) as usize;

                if x >= MAP_SIZE.0 || y >= MAP_SIZE.1 {
                    None
                } else {
                    Some((x, y))
                }
            }
        });

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

                let color = if self.get_map().plants[i][j].is_some() {
                    Color32::GREEN
                } else {
                    match self.get_map().map[i][j] {
                        MapCell::Air(_) => Color32::LIGHT_BLUE,
                        MapCell::Soil(_) => Color32::YELLOW,
                    }
                };

                let color = if self.hovered_cell.or(self.highlighted_cell) == Some((j, i)) {
                    Color32::BROWN
                } else {
                    color
                };
                painter.rect_filled(rect, 0., color);

                if self.get_map().plants[i][j].is_some()
                    && self.get_map().evolution_data.cells_abilities
                        [self.get_map().plants[i][j].t]
                        .seed
                {
                    painter.circle_filled(
                        canvas_start
                            + Vec2 {
                                x: j as f32 * self.cell_size + 0.5 * self.cell_size,
                                y: i as f32 * self.cell_size + 0.5 * self.cell_size,
                            },
                        self.cell_size * 0.4,
                        Color32::RED,
                    );
                }
            }
        }

        pointer_pos.inspect(|pos| {
            painter.circle_filled(*pos, 2., Color32::RED);
        });
    }
}

impl eframe::App for PlantEvolutionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        if let Some((maps, version)) = self.slow_maps.slow_read_versioned(self.maps_version) {
            self.maps_version = version;
            self.maps = maps;
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
                ui.label(format!(
                    "Suicide: {:.2}",
                    evolution_data.suicide_weights.get_formula()
                ));
                ui.collapsing("Up", |ui| {
                    evolution_data.weights[0]
                        .iter()
                        .enumerate()
                        .for_each(|(i, w)| {
                            ui.label(format!("{}: {}", i + 1, w.get_formula()));
                        });
                });
                ui.collapsing("Sideways", |ui| {
                    evolution_data.weights[1]
                        .iter()
                        .enumerate()
                        .for_each(|(i, w)| {
                            ui.label(format!("{}: {}", i + 1, w.get_formula()));
                        });
                });
                ui.collapsing("Down", |ui| {
                    evolution_data.weights[2]
                        .iter()
                        .enumerate()
                        .for_each(|(i, w)| {
                            ui.label(format!("{}: {}", i + 1, w.get_formula()));
                        });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("Close").clicked() {
                        self.selected_decision_tree = None;
                    }
                });
            });
        }

        egui::Panel::top("settings").show_inside(ui, |ui| {});

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
                self.selected_map_index_str =
                    Self::get_selected_map_index_to_str(&self.selected_maps_index);
            }
            if response.lost_focus() {
                self.selected_map_index_str =
                    Self::get_selected_map_index_to_str(&self.selected_maps_index);
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(80.);
                self.maps.iter().enumerate().for_each(|(i, _)| {
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
                        if self.selected_maps_index.len() == 0 {
                            self.selected_maps_index = vec![0];
                        }
                        self.selected_maps_index.sort();
                        self.selected_map_index_str =
                            Self::get_selected_map_index_to_str(&self.selected_maps_index);
                    }
                });
            });
        });

        egui::Panel::right("control_menu").show_inside(ui, |ui| {
            ui.set_min_width(200.);
            ui.horizontal(|ui| {
                ui.label(format!("Evolutions: {}", self.get_map().evolutions));
                ui.label(format!("Step: {}", self.get_map().ticks));
            });
            ui.label(format!("Score: {:.2}", calculate_score(&self.get_map())));

            ui.separator();

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

            ui.label("Nutritions:");
            ui.label(format!(
                "Sunlight: {:.2}",
                self.get_map().plant_nutrition.sunlight
            ));
            ui.label(format!("Air: {:.2}", self.get_map().plant_nutrition.air));
            ui.label(format!(
                "Minerals: {}",
                self.get_map().plant_nutrition.minerals
            ));
            ui.label(format!(
                "Water: {:.2}",
                self.get_map().plant_nutrition.water
            ));
            ui.label(format!(
                "Power: {:.2}",
                self.get_map().plant_nutrition.energy
            ));

            let mut new_desision_tree = None;
            self.get_map()
                .evolution_data
                .cells_abilities
                .iter()
                .enumerate()
                .for_each(|(i, cell)| {
                    ui.horizontal_top(|ui| {
                        ui.collapsing(format!("Cell {}", i + 1), |ui| {
                            if ui
                                .add_enabled(
                                    self.selected_decision_tree
                                        != Some((self.selected_maps_index[0], i)),
                                    Button::new("Decision tree"),
                                )
                                .clicked()
                            {
                                new_desision_tree = Some((self.selected_maps_index[0], i));
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
                    "Next growth {:.2}: cell {} at {:?}",
                    self.get_map().next_cell_growth.0,
                    self.get_map().next_cell_growth.3,
                    (
                        self.get_map().next_cell_growth.1,
                        self.get_map().next_cell_growth.2
                    )
                ))
                .hovered()
            {
                self.highlighted_cell = Some((
                    self.get_map().next_cell_growth.1,
                    self.get_map().next_cell_growth.2,
                ));
            }
            if ui
                .label(format!(
                    "Next suicide {:.2} at {:?}",
                    self.get_map().next_cell_suicide.0,
                    (
                        self.get_map().next_cell_suicide.1,
                        self.get_map().next_cell_suicide.2
                    )
                ))
                .hovered()
            {
                if self.get_map().plants[self.get_map().next_cell_suicide.2]
                    [self.get_map().next_cell_suicide.1]
                    .is_some()
                {
                    self.highlighted_cell = Some((
                        self.get_map().next_cell_suicide.1,
                        self.get_map().next_cell_suicide.2,
                    ));
                }
            }
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::Panel::bottom("cell_info")
                .min_size(100.)
                .show_inside(ui, |ui| match self.hovered_cell.or(self.highlighted_cell) {
                    Some((x, y)) => {
                        let cell_info = format!("cell_info {:?};", &self.get_map().map[y][x]);
                        ui.label(format!("({}, {}) => {}", x, y, cell_info));

                        let plant_info = if self.get_map().plants[y][x].is_some() {
                            format!(
                                "plant {}, sunlight: {:.2}, air: {:.2}, minerals: {:.2}, water: {:.2}",
                                self.get_map().plants[y][x].t + 1,
                                self.get_map().plants[y][x].input.sunlight,
                                self.get_map().plants[y][x].input.air,
                                self.get_map().plants[y][x].input.minerals,
                                self.get_map().plants[y][x].input.water
                            )
                        } else {
                            "".to_owned()
                        };
                        ui.label(format!("{}", plant_info));
                        ui.label(format!("{:?}", self.get_map().plants[y][x]));
                    }
                    None => {
                        ui.label("Nothing selected");
                    }
                });

            Frame::canvas(ui.style()).show(ui, |ui| {
                let available = ui.available_size();
                self.cell_size = self.min_cell_size.max({
                    (available.x / MAP_SIZE.0 as f32)
                        .min(available.y / MAP_SIZE.1 as f32)
                });

                let canvas_start = (ui.next_widget_position() + available / 2.
                    - Pos2 {
                        x: self.cell_size * MAP_SIZE.0 as f32 / 2.,
                        y: self.cell_size * MAP_SIZE.1 as f32 / 2.,
                    })
                .to_pos2();

                self.draw_map(ui, self.selected_maps_index[0], canvas_start);
            });
        });
    }
}

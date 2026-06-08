use std::{
    sync::{Arc, mpsc},
    time::Duration,
};

use egui::{Button, Color32, Frame, Pos2, Rect, Sense, TextEdit, Vec2};

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
    selected_map_index: usize,
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
            selected_map_index: 0,
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
        &self.maps[self.selected_map_index]
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
            ui.horizontal(|ui| {
                let response = ui
                    .add(TextEdit::singleline(&mut self.selected_map_index_str).desired_width(64.));
                let idx = self.selected_map_index_str.parse::<usize>();
                if ui
                    .add_enabled(
                        idx.as_ref().is_ok_and(|&idx| idx <= self.maps.len()),
                        egui::Button::new("Select"),
                    )
                    .clicked()
                {
                    self.selected_map_index = idx.unwrap() - 1;
                    self.selected_map_index_str = (self.selected_map_index + 1).to_string();
                }
                if response.lost_focus() {
                    self.selected_map_index_str = (self.selected_map_index + 1).to_string();
                };
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(80.);
                self.maps.iter().enumerate().for_each(|(i, _)| {
                    if ui
                        .selectable_value(
                            &mut self.selected_map_index,
                            i,
                            format!("Plant {}", i + 1),
                        )
                        .clicked()
                    {
                        if self.selected_map_index == i {
                            self.selected_map_index = i;
                            self.selected_map_index_str = (self.selected_map_index + 1).to_string();
                        }
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
                                        != Some((self.selected_map_index, i)),
                                    Button::new("Decision tree"),
                                )
                                .clicked()
                            {
                                new_desision_tree = Some((self.selected_map_index, i));
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
                let (response, painter) =
                    ui.allocate_painter(ui.available_size_before_wrap(), Sense::empty());
                self.cell_size = self.min_cell_size.max({
                    (response.rect.width() / MAP_SIZE.0 as f32)
                        .min(response.rect.height() / MAP_SIZE.1 as f32)
                });

                let canvas_start = (response.rect.center()
                    - Pos2 {
                        x: self.cell_size * MAP_SIZE.0 as f32 / 2.,
                        y: self.cell_size * MAP_SIZE.1 as f32 / 2.,
                    })
                .to_pos2();

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
            });
        });
    }
}

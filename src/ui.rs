use std::sync::{Arc, mpsc};

use egui::{Button, Color32, Frame, Pos2, Rect, Sense, Vec2};

use crate::{const_precalc::*, engine::EngineCommand, evolution::*, map::*, slow_mutex::SlowMutex};

pub struct PlantEvolutionApp {
    min_cell_size: f32,
    cell_size: f32,

    selected_map_index: usize,
    maps: Vec<MapData>,

    maps_version: u128,
    command_sender: mpsc::Sender<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,

    run: bool,
    run_evolution: bool,

    highlited_cell: Option<(usize, usize)>,
    highlited_proximity_cell: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new(
        sender: mpsc::Sender<EngineCommand>,
        slow_maps: Arc<SlowMutex<Vec<MapData>>>,
    ) -> Self {
        Self {
            min_cell_size: 1.,
            cell_size: 6.,
            selected_map_index: 0,
            maps_version: 0,
            command_sender: sender,
            maps: slow_maps.force_read(),
            slow_maps,
            run: false,
            run_evolution: false,
            highlited_cell: None,
            highlited_proximity_cell: None,
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

        egui::Panel::left("evolution_menu").show_inside(ui, |ui| {
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(80.);
                self.maps.iter().enumerate().for_each(|(i, _)| {
                    let mut selected = i == self.selected_map_index;
                    ui.toggle_value(&mut selected, format!("Plant {}", i + 1));
                    if selected {
                        self.selected_map_index = i;
                    }
                });
            });
        });

        if true {
            egui::Panel::right("control_menu").show_inside(ui, |ui| {
                ui.set_min_width(200.);
                ui.horizontal(|ui| {
                    ui.label(format!("Evolutions: {}", self.get_map().evolutions));
                    ui.label(format!("Step: {}", self.get_map().ticks));
                    ui.label(format!("Score: {}", calculate_score(&self.get_map())));
                });

                ui.horizontal(|ui| {
                    if ui.toggle_value(&mut self.run, "Run").changed() {
                        if self.run {
                            self.command_sender.send(EngineCommand::RunTick).unwrap();
                        } else {
                            self.command_sender
                                .send(EngineCommand::StopRunTick)
                                .unwrap();
                        }
                    };
                    if ui.button("Tick!").clicked() {
                        self.command_sender.send(EngineCommand::Tick).unwrap();
                    }
                    if ui.button("Restart").clicked() {
                        self.command_sender.send(EngineCommand::Restart).unwrap();
                    }
                });

                ui.label("Nutritions:");
                ui.label(format!(
                    "Sunlight: {}",
                    self.get_map().plant_nutrition.sunlight
                ));
                ui.label(format!("Air: {}", self.get_map().plant_nutrition.air));
                ui.label(format!(
                    "Minerals: {}",
                    self.get_map().plant_nutrition.minerals
                ));
                ui.label(format!("Water: {}", self.get_map().plant_nutrition.water));
                ui.label(format!("Power: {}", self.get_map().plant_nutrition.power));

                self.get_map()
                    .evolution_data
                    .cells_abilities
                    .iter()
                    .enumerate()
                    .for_each(|(i, cell)| {
                        ui.collapsing(format!("Cell {}", i), |ui| {
                            ui.label(format!("Sunlight: {}", cell.sunlight_consumption));
                            ui.label(format!("Air: {}", cell.air_consumption));
                            ui.label(format!("Minerals: {}", cell.minerals_consumption));
                            ui.label(format!("Water: {}", cell.water_consumption));
                            ui.label(format!("Power: {}", cell.power_production_speed));
                            ui.label(format!("Cost: {}", cell.cost));
                        });
                    });
            });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::Panel::bottom("cell_info").show_inside(ui, |ui| match self.highlited_cell {
                Some((x, y)) => {
                    let cell_info = format!("cell_info {:?};", &self.get_map().map[y][x]);
                    ui.label(format!("({}, {}) => {}", x, y, cell_info));

                    let plant_info = if self.get_map().plants[y][x].t != usize::MAX {
                        format!(
                            "plant {}, sunlight: {}, air: {}, minerals: {}, water: {}",
                            self.get_map().plants[y][x].t,
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

                    if let Some((proximity_x, proximity_y)) = self.highlited_proximity_cell {
                        let proximity = &PROXIMITY_DXDY.get().unwrap()[proximity_y][proximity_x];
                        if let Some(&(_, _, distance, angle)) = proximity.iter().find(|&&(px, py, _, _)| px == x && py == y) {
                            ui.label(format!("distance = {}, angle = {}", distance, angle));
                        } else {
                            ui.label("");
                        }
                    } else {
                        ui.label("");
                    }
                }
                None => {
                    ui.label("Nothing selected");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Growth:");
            });

            Frame::canvas(ui.style()).show(ui, |ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.available_size_before_wrap(), Sense::empty());
                self.cell_size = self.min_cell_size.max({
                    (response.rect.width() / MAP_SIZE.0 as f32).min(
                        response.rect.height() / MAP_SIZE.1 as f32
                    )
                });

                let pointer_pos: Option<Pos2> = ui.ctx().input(|i| i.pointer.latest_pos());
                self.highlited_cell = pointer_pos.and_then(|pos| {
                    let pos = pos - response.rect.min;
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
                if let Some(highlited_cell) = self.highlited_cell {
                    ui.ctx().input(|input| {
                        if input.key_pressed(egui::Key::X) {
                            if self.highlited_proximity_cell.is_some_and(
                                |highlited_proximity_cell| {
                                    highlited_proximity_cell == highlited_cell
                                },
                            ) {
                                self.highlited_proximity_cell = None;
                            } else {
                                self.highlited_proximity_cell = Some(highlited_cell);
                            }
                        }
                    });
                }

                for i in 0..MAP_SIZE.1 {
                    for j in 0..MAP_SIZE.0 {
                        let rect = Rect::from_min_size(
                            response.rect.min
                                + Vec2 {
                                    x: j as f32 * self.cell_size,
                                    y: i as f32 * self.cell_size,
                                },
                            Vec2 {
                                x: self.cell_size,
                                y: self.cell_size,
                            },
                        );

                        let color = if self.get_map().plants[i][j].t != usize::MAX {
                            Color32::GREEN
                        } else {
                            match self.get_map().map[i][j] {
                                MapCell::Air(_) => Color32::LIGHT_BLUE,
                                MapCell::Soil(_) => Color32::YELLOW,
                            }
                        };

                        let color = if self.highlited_cell == Some((j, i)) {
                            Color32::BROWN
                        } else {
                            color
                        };
                        painter.rect_filled(rect, 0., color);
                    }
                }

                if let Some((proximity_x, proximity_y)) = self.highlited_proximity_cell {
                    let proximity = &PROXIMITY_DXDY.get().unwrap()[proximity_y][proximity_x];
                    for &(x, y, _, _) in proximity {
                        let center = response.rect.min
                            + Vec2 {
                                x: x as f32 * self.cell_size + self.cell_size / 2.,
                                y: y as f32 * self.cell_size + self.cell_size / 2.,
                            };
                        painter.circle_filled(center, self.cell_size / 3., Color32::PURPLE);
                    }
                    let center = response.rect.min
                        + Vec2 {
                            x: proximity_x as f32 * self.cell_size + self.cell_size / 2.,
                            y: proximity_y as f32 * self.cell_size + self.cell_size / 2.,
                        };
                    painter.circle_filled(center, self.cell_size / 3., Color32::MAGENTA);
                }

                pointer_pos.inspect(|pos| {
                    painter.circle_filled(*pos, 2., Color32::RED);
                });
            });
        });
    }
}

use std::sync::{Arc, mpsc};

use egui::{Button, Color32, Frame, Pos2, Rect, Sense, Vec2};

use crate::{const_precalc::*, engine::EngineCommand, evolution::*, map::*, slow_mutex::SlowMutex};

pub struct PlantEvolutionApp {
    cell_size: f32,

    selected_map_index: usize,
    maps: Vec<MapData>,

    maps_version: u128,
    command_sender: mpsc::Sender<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,

    run: bool,
    run_evolution: bool,

    highlited_cell: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new(
        sender: mpsc::Sender<EngineCommand>,
        slow_maps: Arc<SlowMutex<Vec<MapData>>>,
    ) -> Self {
        Self {
            cell_size: 6.,
            selected_map_index: 0,
            maps_version: 0,
            command_sender: sender,
            maps: slow_maps.force_read(),
            slow_maps,
            run: false,
            run_evolution: false,
            highlited_cell: None,
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
                    ui.label(format!("Step: {}", self.get_map().time));
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
            if false {
                /*ui.label("Long term evolution is running");
                if let Some(data) = self.evolution_running_channel.1.iter().last() {
                    ui.label("Progress:");
                    ui.label(format!(
                        "Evolutions: {}/{}",
                        data.evolution, data.evolution_total
                    ));
                    ui.label(format!("Ticks: {}/{}", data.tick, data.tick_total));
                }*/
            } else {
                egui::Panel::bottom("cell_info").show_inside(ui, |ui| match self.highlited_cell {
                    Some((x, y)) => {
                        let cell_info = format!("cell_info {:?};", &self.get_map().map[y][x]);
                        ui.label(format!("({}, {}) => {}", x, y, cell_info));

                        let plant_info = if self.get_map().plants[y][x].t != usize::MAX {
                            format!("plant {}, sunlight: {}, air: {}, minerals: {}, water: {}", self.get_map().plants[y][x].t, self.get_map().plants[y][x].input.sunlight, self.get_map().plants[y][x].input.air, self.get_map().plants[y][x].input.minerals, self.get_map().plants[y][x].input.water)
                        } else {
                            "".to_owned()
                        };
                        ui.label(format!("{}", plant_info));
                    }
                    None => {
                        ui.label("Nothing selected");
                    }
                });
                Frame::canvas(ui.style()).show(ui, |ui| {
                    let (response, painter) =
                        ui.allocate_painter(ui.available_size_before_wrap(), Sense::empty());

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

                    pointer_pos.inspect(|pos| {
                        painter.circle_filled(*pos, 2., Color32::RED);
                    });
                });
            }
        });
    }
}

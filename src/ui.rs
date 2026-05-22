use egui::{Color32, Frame, Pos2, Rect, Sense, TextEdit, Vec2, emath};

use crate::{evolution::*, map::*};

pub struct PlantEvolutionApp {
    cell_size: f32,

    selected_map_index: usize,
    number_of_plants: usize,
    maps: Vec<MapData>,

    run: bool,
    highlited_cell: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new() -> Self {
        let number_of_plants: usize = 100;
        let maps = (0..number_of_plants)
            .map(|_| {
                let (a, b, c) = get_basic_map_data();
                MapData::generate(a, b, c)
            })
            .collect();
        Self {
            cell_size: 6.,
            selected_map_index: 0,
            number_of_plants,
            maps,
            run: false,
            highlited_cell: None,
        }
    }

    fn get_map(&self) -> &MapData {
        &self.maps[self.selected_map_index]
    }

    fn get_map_mut(&mut self) -> &mut MapData {
        &mut self.maps[self.selected_map_index]
    }
}

impl eframe::App for PlantEvolutionApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::left("evolution_menu").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let mut text = self.number_of_plants.to_string();
                TextEdit::singleline(&mut text).desired_width(64.).show(ui);
                if let Ok(number) = text.parse() {
                    self.number_of_plants = number;
                }

                if ui.button("Evolve!").clicked() {}
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

        egui::Panel::right("control_menu").show_inside(ui, |ui| {
            ui.set_min_width(200.);
            ui.horizontal(|ui| {
                ui.label(format!("Step: {}", self.get_map().time));
                ui.label(format!("Score: {}", calculate_score(&self.get_map())));
            });

            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.run, "Run");
                if ui.button("Tick!").clicked() || self.run {
                    self.maps.iter_mut().for_each(|map| {
                        map.tick();
                    });
                    ui.ctx().request_repaint();
                }
                if ui.button("Restart").clicked() {
                    self.maps.iter_mut().for_each(|map| {
                        let (a, b, c) = get_basic_map_data();
                        map.restart(a, b, c);
                    });
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
                .cells
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

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::Panel::bottom("cell_info").show_inside(ui, |ui| match self.highlited_cell {
                Some((x, y)) => {
                    let cell_name = match &self.get_map().map[y][x] {
                        MapCell::Air => "air".to_owned(),
                        MapCell::Soil(_) => "soil".to_owned(),
                        MapCell::Plant(plant_cell) => format!("plant {}", plant_cell.t),
                    };
                    ui.label(format!("({}, {}) => {}", x, y, cell_name));
                }
                None => {
                    ui.label("Nothing selected");
                }
            });
            Frame::canvas(ui.style()).show(ui, |ui| {
                let (mut response, painter) =
                    ui.allocate_painter(ui.available_size_before_wrap(), Sense::empty());

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
                    response.rect,
                );
                let from_screen = to_screen.inverse();

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

                self.get_map().map.iter().enumerate().for_each(|(i, row)| {
                    row.iter().enumerate().for_each(|(j, cell)| {
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
                        let color = match cell {
                            MapCell::Air => Color32::LIGHT_BLUE,
                            MapCell::Soil(_) => Color32::YELLOW,
                            MapCell::Plant(_) => Color32::GREEN,
                        };

                        let color = if self.highlited_cell == Some((j, i)) {
                            Color32::BROWN
                        } else {
                            color
                        };
                        painter.rect_filled(rect, 0., color);
                    });
                });

                pointer_pos.inspect(|pos| {
                    painter.circle_filled(*pos, 2., Color32::RED);
                });
            });
        });
    }
}

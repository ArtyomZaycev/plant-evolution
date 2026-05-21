use egui::{Color32, Frame, Pos2, Rect, Sense, Vec2, emath};

use crate::map::{MAP_SIZE, MapCell, MapData};

pub struct PlantEvolutionApp {
    cell_size: f32,

    map: MapData,
    highlited_cell: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new(map: MapData) -> Self {
        Self {
            cell_size: 6.,
            map,
            highlited_cell: None,
        }
    }
}

impl eframe::App for PlantEvolutionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::right("control_menu").show_inside(ui, |ui| {
            if ui.button("Tick!").clicked() {
                self.map.tick();
            }

            ui.label("Nutritions:");
            ui.label(format!("Sunlight: {}", self.map.plant_nutrition.sunlight));
            ui.label(format!("Air: {}", self.map.plant_nutrition.air));
            ui.label(format!("Minerals: {}", self.map.plant_nutrition.minerals));
            ui.label(format!("Water: {}", self.map.plant_nutrition.water));
            ui.label(format!("Power: {}", self.map.plant_nutrition.power));
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::Panel::bottom("cell_info").show_inside(ui, |ui| {
                match self.highlited_cell {
                    Some((x, y)) => {
                        let cell_name = match &self.map.map[y][x] {
                            MapCell::Air => "air".to_owned(),
                            MapCell::Soil(_) => "soil".to_owned(),
                            MapCell::Plant(plant_cell) => format!("plant {}", plant_cell.t),
                        };
                        ui.label(format!("({}, {}) => {}", x, y, cell_name));
                    },
                    None => {
                        ui.label("Nothing selected");
                    },
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

                self.map.map.iter().enumerate().for_each(|(i, row)| {
                    row.iter().enumerate().for_each(|(j, cell)| {
                        let rect = Rect::from_min_size(response.rect.min + Vec2 { x: j as f32 * self.cell_size, y: i as f32 * self.cell_size }, Vec2 { x: self.cell_size, y: self.cell_size });
                        let color = match cell {
                            MapCell::Air => Color32::LIGHT_BLUE,
                            MapCell::Soil(_) => Color32::YELLOW,
                            MapCell::Plant(_) => Color32::GREEN,
                        };

                        let color = if self.highlited_cell == Some((j, i)) {Color32::BROWN} else {color};
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
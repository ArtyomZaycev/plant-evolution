use egui::{Align2, Button, Color32, FontId, Pos2, Rect, Sense, TextEdit, Vec2};

use plant_evolution_lib::{engine::*, map::*, precalc::*, utils::*};

use crate::ui::{
    settings::VisualSettings,
    settings_editor::{editor::*, utils::EditorUi},
    toast::*,
};

pub struct PlantEvolutionApp {
    toast_manager: ToastManager,
    engine: Engine,

    visual_settings: VisualSettings,
    settings: Option<AppSettingsEditor>,

    cell_size: f32,

    engine_inner_state: VersionedMutexData<InnerEngineState>,

    autoevolve_enabled: bool,
    autoevolve_at_str: String,
    autoevolve_at: u32,

    selected_map_index_str: String,
    selected_maps_index: Vec<usize>,
    maps: SlowMutexReadResult<Vec<MapData>>,

    highlighted_map: Option<usize>,
    hovered_cell: Option<(usize, usize, usize)>,
    highlighted_cell: Option<(usize, usize, usize)>,
    selected_decision_tree: Option<(usize, usize)>,
}

impl PlantEvolutionApp {
    pub fn new(engine: Engine) -> Self {
        Self {
            toast_manager: ToastManager::new(),
            maps: engine.state.maps.read(),
            visual_settings: VisualSettings::default(),
            settings: None,
            cell_size: 6.,
            engine_inner_state: engine.state.inner_state.read(),
            autoevolve_enabled: true,
            autoevolve_at_str: 500.to_string(),
            autoevolve_at: 500,
            selected_map_index_str: "1".to_owned(),
            selected_maps_index: vec![0],
            highlighted_map: None,
            hovered_cell: None,
            highlighted_cell: None,
            selected_decision_tree: None,
            engine,
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

                let color = if self.maps[map_idx].cells[i][j].is_some() {
                    self.visual_settings.plant_color
                } else {
                    match self.maps[map_idx].map[i][j] {
                        MapCell::Air(_) => self.visual_settings.air_color,
                        MapCell::Soil(_) => self.visual_settings.soil_color,
                    }
                };

                let color = if self.visual_settings.highlight_hovered_cell
                    && self.hovered_cell.or(self.highlighted_cell) == Some((map_idx, j, i))
                {
                    Color32::BROWN
                } else {
                    color
                };
                painter.rect_filled(rect, 0., color);

                if self.maps[map_idx].cells[i][j].is_some()
                    && self.maps[map_idx].evolution_data.cells_abilities
                        [self.maps[map_idx].cells[i][j].t]
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
            (
                self.cell_size / 2.,
                self.visual_settings.highlighted_map_border_color,
            )
        } else {
            (
                self.cell_size / 2.,
                self.visual_settings.hovered_map_border_color,
            )
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

    fn push_save_log(&mut self, save_log: SaveLog) {
        self.toast_manager.add(Toast::new(match save_log.error {
            Some(err) => format!("Error saving: {err}"),
            None => format!("Saved to {:?}", save_log.path),
        }));
    }

    fn save_maps(&mut self, selection: &SaveSelection) {
        let folder = self.engine.state.parameters.read().saving_parameters.path.clone();
        let simulation_id = self.engine.state.simulation_id.read().unwrap().clone();
        self.push_save_log(save_maps(
            simulation_save_folder_path(
                folder,
                simulation_id,
            ),
            &selection,
            &self.maps,
        ));
    }

    fn get_autoevolve(&self) -> Option<u32> {
        if self.autoevolve_enabled {
            Some(self.autoevolve_at)
        } else {
            None
        }
    }

    fn update_autoevolve_state(&self) {
        if self.engine
            .state
            .inner_state.cloned() != InnerEngineState::Stale {
                self.engine
                    .state
                    .inner_state.write(InnerEngineState::RunSimulation { autoevolve: self.get_autoevolve() });
            }
    }
}

impl eframe::App for PlantEvolutionApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        self.engine.state.maps.slow_update(&mut self.maps);
        self.engine
            .state
            .inner_state
            .update(&mut self.engine_inner_state);

        self.toast_manager.show(ui);

        let mut close_settings = false;
        if let Some(settings) = &mut self.settings {
            match settings.get_state() {
                SettingsRawState::InProgress => {}
                SettingsRawState::Cancelled => {
                    close_settings = true;
                }
                SettingsRawState::Applied(ui_settings, engine_parameters) => {
                    self.visual_settings = ui_settings.clone();
                    self.engine
                        .state
                        .parameters
                        .unchecked_write(engine_parameters.clone());
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
                ui.menu_button("File", |ui| {
                    ui.set_min_width(100.);
                    ui.menu_button("Save", |ui| {
                        ui.set_min_width(80.);
                        if ui.button("Best").clicked() {
                            self.save_maps(&SaveSelection::Best(1));
                        }
                        if ui.button("Selected").clicked() {
                            self.save_maps(&SaveSelection::Selected(
                                self.selected_maps_index.clone(),
                            ));
                        }
                        if ui.button("All").clicked() {
                            self.save_maps(&SaveSelection::All);
                        }
                    });
                    if ui.button("Save As").clicked() {}
                    if ui.button("Load").clicked() {}
                    ui.separator();
                    if ui.button("Restart").clicked() {
                        self.engine.send_command(EngineCommand::Restart).unwrap();
                    }
                    ui.separator();
                    if ui.button("Settings").clicked() {
                        self.settings = Some(AppSettingsEditor::new((
                            self.visual_settings.clone(),
                            self.engine.state.parameters.cloned(),
                        )));
                    }
                });
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
                            let range_start = *self.selected_maps_index.last().unwrap();
                            let range = if range_start <= i {range_start..=i} else {i..=range_start};
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

            let engine_state = VersionedMutexData::get_cloned(&self.engine_inner_state);
            ui.horizontal(|ui| {
                ui.label("Simulation");
                if ui
                    .radio(engine_state == InnerEngineState::Stale, "Disabled")
                    .clicked()
                {
                    self.engine.state.inner_state.write(InnerEngineState::Stale);
                }
                if ui
                    .radio(matches!(engine_state, InnerEngineState::RunSimulation{autoevolve: None}), "Grow")
                    .clicked()
                {
                    self.autoevolve_enabled = false;
                    self.engine.state.inner_state.write(InnerEngineState::RunSimulation { autoevolve: self.get_autoevolve() });
                }
                if ui
                    .radio(matches!(engine_state, InnerEngineState::RunSimulation{autoevolve: Some(_)}), "Evolve")
                    .clicked()
                {
                    self.autoevolve_enabled = true;
                    self.engine.state.inner_state.write(InnerEngineState::RunSimulation { autoevolve: self.get_autoevolve() });
                }
            });
            
            ui.horizontal(|ui| {
                if ui.add_enabled(engine_state == InnerEngineState::Stale, egui::Button::new("Grow")).clicked() {
                    self.engine.send_command(EngineCommand::Tick).unwrap();
                }
                if ui.add_enabled(engine_state == InnerEngineState::Stale || self.autoevolve_enabled == false, egui::Button::new("Evolve")).clicked() {
                    self.engine.send_command(EngineCommand::Evolve).unwrap();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Ticks per evolution");
                let response = ui.text_edit_singleline(&mut self.autoevolve_at_str);
                if response.lost_focus() {
                    if let Ok(autoevolve_at) = self.autoevolve_at_str.parse() {
                        self.autoevolve_at = autoevolve_at;
                        self.autoevolve_at_str = autoevolve_at.to_string();
                        self.update_autoevolve_state();
                    }
                }
            });

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
                self.maps[map_idx].calculate_score(),
            ));

            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label("Nutrition:");
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
                    "Energy: {:.2}",
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
                    if self.maps[map_idx].cells[self.maps[map_idx].next_cell_suicide.2]
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
                .min_size(40.)
                .show_inside(ui, |ui| match self.hovered_cell.or(self.highlighted_cell) {
                    Some((map_idx, x, y)) => {
                        ui.label(format!("({}, {}) => {}", x, y, &self.maps[map_idx].map[y][x]));
                        let plant_info = if self.maps[map_idx].cells[y][x].is_some() {
                            format!(
                                "Plant cell {}, sunlight: {:.2}, air: {:.2}, minerals: {:.2}, water: {:.2}",
                                self.maps[map_idx].cells[y][x].t + 1,
                                self.maps[map_idx].cells[y][x].input.sunlight,
                                self.maps[map_idx].cells[y][x].input.air,
                                self.maps[map_idx].cells[y][x].input.minerals,
                                self.maps[map_idx].cells[y][x].input.water
                            )
                        } else {
                            "".to_owned()
                        };
                        ui.label(plant_info);
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

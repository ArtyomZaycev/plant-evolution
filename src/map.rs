use std::f32;

use crate::cell::*;

// (X, Y)
pub const MAP_SIZE: (usize, usize) = (128, 128);
pub const PLANT_CENTER: (usize, usize) = (MAP_SIZE.0 / 2, MAP_SIZE.1 / 2 + 2);

#[derive(Debug, Clone)]
pub struct PlantCell {
    pub t: usize,
    pub input: PlantCellInput,
}

#[derive(Debug, Clone)]
pub struct SoilParameters {
    pub minerals: f32,
    pub water: f32,
}

impl Default for SoilParameters {
    fn default() -> Self {
        Self {
            minerals: 1.,
            water: 1.,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MapCell {
    Air,
    Soil(SoilParameters),
    Plant(PlantCell),
}

#[derive(Debug, Clone)]
pub struct PlantNutrition {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,

    pub power: f32,
}

pub struct MapData {
    pub cells: [PlantCellAbilities; NUMBER_OF_CELLS],
    pub evolution_data: PlantEvolutionData,

    pub time: i32,
    pub plant_nutrition: PlantNutrition,
    pub map: [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
}

impl MapData {
    fn calc_sunlight(&self, x: usize, y: usize) -> f32 {
        let basic_sunlight = (MAP_SIZE.1 - y) as f32 / MAP_SIZE.1 as f32;
        (0..y).fold(basic_sunlight, |sunlight, i: usize| match &self.map[i][x] {
            MapCell::Air => sunlight,
            MapCell::Soil(_) => 0.,
            MapCell::Plant(_) => sunlight / 2.,
        })
    }

    fn calc_air_get_cell(&self, x: usize, y: usize, dx: i32, dy: i32) -> MapCell {
        let new_x = x as i32 + dx;
        let new_x = if new_x < 0 {
            0
        } else if new_x >= MAP_SIZE.0 as i32 {
            MAP_SIZE.0 - 1
        } else {
            new_x as usize
        };

        let new_y = y as i32 + dy;
        let new_y = if new_y < 0 {
            0
        } else if new_y >= MAP_SIZE.0 as i32 {
            MAP_SIZE.1 - 1
        } else {
            new_y as usize
        };

        self.map[new_y][new_x].clone()
    }
    fn calc_air(&self, x: usize, y: usize) -> f32 {
        let air_sum = (-2..=2).fold(0, |air, dx: i32| {
            (-2..=2).fold(air, |air, dy: i32| {
                let distance = dx.abs() + dy.abs();
                if distance > 0 && distance < 4 {
                    let cell = self.calc_air_get_cell(x, y, dx, dy);
                    match cell {
                        MapCell::Air => air + (4 - distance),
                        MapCell::Soil(_) => air,
                        MapCell::Plant(_) => air + (4 - distance) / 3,
                    }
                } else {
                    air
                }
            })
        });
        air_sum as f32 / 36.
    }
    fn calc_minerals(&self, x: usize, y: usize) -> f32 {
        let minerals_sum = (-2..=2).fold(0., |minerals, dx: i32| {
            (-2..=2).fold(minerals, |minerals, dy: i32| {
                let distance = dx.abs() + dy.abs();
                if distance > 0 && distance < 4 {
                    let cell = self.calc_air_get_cell(x, y, dx, dy);
                    match cell {
                        MapCell::Air => minerals,
                        MapCell::Soil(soil_parameters) => minerals + soil_parameters.minerals,
                        MapCell::Plant(_) => minerals,
                    }
                } else {
                    minerals
                }
            })
        });
        minerals_sum as f32 / 36.
    }
    fn calc_water(&self, x: usize, y: usize) -> f32 {
        let water_sum = (-2..=2).fold(0., |water, dx: i32| {
            (-2..=2).fold(water, |water, dy: i32| {
                let distance = dx.abs() + dy.abs();
                if distance > 0 && distance < 4 {
                    let cell = self.calc_air_get_cell(x, y, dx, dy);
                    match cell {
                        MapCell::Air => water,
                        MapCell::Soil(soil_parameters) => water + soil_parameters.water,
                        MapCell::Plant(_) => water,
                    }
                } else {
                    water
                }
            })
        });
        water_sum as f32 / 36.
    }

    fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
    }
    fn calc_cells_proximity_data(
        &self,
        x: usize,
        y: usize,
    ) -> [PlantCellProximityData; NUMBER_OF_CELLS] {
        let cx = PLANT_CENTER.0 as f32;
        let cy = PLANT_CENTER.1 as f32;
        let px = x as f32;
        let py = y as f32;
        let mut new_proximity_data = [PlantCellProximityData::default(); NUMBER_OF_CELLS];
        self.map.iter().enumerate().for_each(|(i, row)| {
            row.iter().enumerate().for_each(|(j, cell)| {
                if i != y && j != x {
                    match cell {
                        MapCell::Plant(plant_cell) => {
                            let x = j as f32;
                            let y = i as f32;

                            let ab = Self::distance(px, py, x, y);
                            let ac = Self::distance(px, py, cx, cy);
                            let bc = Self::distance(x, y, cx, cy);

                            let angle =
                                ((ab.powi(2) + ac.powi(2) + bc.powi(2)) / (2. * ab * ac)).acos();
                            if angle < f32::consts::FRAC_PI_4 {
                                let line_angle = {
                                    let x1 = px;
                                    let y1 = py;
                                    let x2 = 2. * x1 - cx;
                                    let y2 = cy;
                                    let d = (y2 - y1) * (x - x1) - (x2 - x1) * (y - y1);
                                    if d < 0. {
                                        angle + f32::consts::FRAC_PI_4
                                    } else {
                                        f32::consts::FRAC_PI_4 - angle
                                    }
                                };
                                let distance = ab;
                                // TODO: Prefer smaller angle
                                if distance < new_proximity_data[plant_cell.t].distance {
                                    new_proximity_data[plant_cell.t] = PlantCellProximityData {
                                        distance: distance
                                            / Self::distance(
                                                0.,
                                                0.,
                                                MAP_SIZE.0 as f32,
                                                MAP_SIZE.1 as f32,
                                            ),
                                        direction: line_angle,
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        });

        new_proximity_data
    }
}

impl MapData {
    fn calculate_plant_inputs(&self) -> [[MapCell; MAP_SIZE.0]; MAP_SIZE.1] {
        self.map
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .enumerate()
                    .map(|(j, cell)| match cell {
                        MapCell::Air => cell.clone(),
                        MapCell::Soil(_) => cell.clone(),
                        MapCell::Plant(plant_cell) => MapCell::Plant(PlantCell {
                            t: plant_cell.t,
                            input: PlantCellInput {
                                sunlight: self.calc_sunlight(j, i),
                                air: self.calc_air(j, i),
                                minerals: self.calc_minerals(j, i),
                                water: self.calc_water(j, i),
                                cells_proximity_data: self.calc_cells_proximity_data(j, i),
                            },
                        }),
                    })
                    .collect::<Vec<MapCell>>()
                    .try_into()
                    .unwrap()
            })
            .collect::<Vec<[MapCell; MAP_SIZE.0]>>()
            .try_into()
            .unwrap()
    }

    fn calculate_plant_nutritions(&self) -> PlantNutrition {
        let nutrition = self
            .map
            .iter()
            .fold(self.plant_nutrition.clone(), |nutrition, row| {
                row.iter().fold(nutrition, |nutrition, cell| match cell {
                    MapCell::Plant(plant_cell) => PlantNutrition {
                        sunlight: nutrition.sunlight
                            + plant_cell.input.sunlight
                                * self.cells[plant_cell.t].sunlight_consumption,
                        air: nutrition.air
                            + plant_cell.input.air * self.cells[plant_cell.t].air_consumption,
                        minerals: nutrition.minerals
                            + plant_cell.input.minerals
                                * self.cells[plant_cell.t].minerals_consumption,
                        water: nutrition.water
                            + plant_cell.input.water * self.cells[plant_cell.t].water_consumption,
                        power: nutrition.power,
                    },
                    _ => nutrition,
                })
            });
        self.map.iter().fold(nutrition, |nutrition, row| {
            row.iter().fold(nutrition, |nutrition, cell| match cell {
                MapCell::Plant(plant_cell) => {
                    let min_resource = [
                        nutrition.sunlight,
                        nutrition.air,
                        nutrition.minerals,
                        nutrition.water,
                    ]
                    .into_iter()
                    .reduce(f32::min)
                    .unwrap();
                    let produced =
                        min_resource.min(self.cells[plant_cell.t].power_production_speed);
                    PlantNutrition {
                        sunlight: nutrition.sunlight - produced,
                        air: nutrition.air - produced,
                        minerals: nutrition.minerals - produced,
                        water: nutrition.water - produced,
                        power: nutrition.power + produced,
                    }
                }
                _ => nutrition,
            })
        })
    }

    fn grow_plant(&mut self) {
        let mut cells: [[[f32; NUMBER_OF_CELLS]; MAP_SIZE.0]; MAP_SIZE.1] =
            [[[0 as f32; NUMBER_OF_CELLS]; MAP_SIZE.0]; MAP_SIZE.1];

        self.map.iter().enumerate().for_each(|(i, row)| {
            row.into_iter()
                .enumerate()
                .for_each(|(j, cell)| match cell {
                    MapCell::Plant(plant_cell) => {
                        let weights = &self.evolution_data.cells_evolution_data[plant_cell.t];
                        if i > 0 {
                            let weights = &weights.weights[0];
                            (0..NUMBER_OF_CELLS).into_iter().for_each(|c| {
                                cells[i - 1][j][c] += weights[c].calc_cell(&plant_cell.input);
                            });
                        }
                        if i + 1 < MAP_SIZE.1 {
                            let weights = &weights.weights[2];
                            (0..NUMBER_OF_CELLS).into_iter().for_each(|c| {
                                cells[i + 1][j][c] += weights[c].calc_cell(&plant_cell.input);
                            });
                        }
                        let nj = if j < PLANT_CENTER.0 && j > 0 {
                            j - 1
                        } else if j >= PLANT_CENTER.0 && j + 1 < MAP_SIZE.0 {
                            j + 1
                        } else {
                            j
                        };
                        if nj != j {
                            let weights = &weights.weights[1];
                            (0..NUMBER_OF_CELLS).into_iter().for_each(|c| {
                                cells[i][nj][c] += weights[c].calc_cell(&plant_cell.input);
                            })
                        }
                        if j == PLANT_CENTER.0 {
                            let weights = &weights.weights[1];
                            (0..NUMBER_OF_CELLS).into_iter().for_each(|c| {
                                cells[i][j - 1][c] += weights[c].calc_cell(&plant_cell.input);
                            })
                        }
                    }
                    _ => {}
                });
        });

        let max_data = cells
            .iter()
            .enumerate()
            .fold((-1. as f32, 0, 0, 0), |acc, (i, row)| {
                row.iter().enumerate().fold(acc, |acc, (j, a)| {
                    a.iter().enumerate().fold(acc, |acc, (c, &weight)| {
                        let new_acc = if weight > acc.0 {
                            (weight, i, j, c)
                        } else {
                            acc
                        };

                        match &self.map[i][j] {
                            MapCell::Air => new_acc,
                            MapCell::Soil(_) => new_acc,
                            MapCell::Plant(_) => acc,
                        }
                    })
                })
            });

        if max_data.0 >= 0. {
            let (_, y, x, cell_type) = max_data;
            if self.plant_nutrition.power >= self.cells[cell_type].cost {
                self.plant_nutrition.power -= self.cells[cell_type].cost;
                self.map.iter_mut().enumerate().for_each(|(i, row)| {
                    row.iter_mut().enumerate().for_each(|(j, c)| {
                        if i == y && j == x {
                            *c = MapCell::Plant(PlantCell {
                                t: cell_type,
                                input: PlantCellInput::default(),
                            });
                        }
                    });
                });
            }
        }
    }
}

impl MapData {
    pub fn generate(
        cells: [PlantCellAbilities; NUMBER_OF_CELLS],
        evolution_data: PlantEvolutionData,
        plant_nutrition: PlantNutrition,
    ) -> Self {
        Self {
            cells,
            evolution_data,
            time: 0,
            plant_nutrition,
            map: (0..MAP_SIZE.1)
                .map(|i| {
                    (0..MAP_SIZE.0)
                        .map(|j| {
                            if i == PLANT_CENTER.1 && j == PLANT_CENTER.0 {
                                MapCell::Plant(PlantCell {
                                    t: 0,
                                    input: PlantCellInput::default(),
                                })
                            } else if i <= MAP_SIZE.1 / 2 {
                                MapCell::Air
                            } else {
                                MapCell::Soil(SoilParameters::default())
                            }
                        })
                        .collect::<Vec<MapCell>>()
                        .try_into()
                        .unwrap()
                })
                .collect::<Vec<[MapCell; MAP_SIZE.0]>>()
                .try_into()
                .unwrap(),
        }
    }

    pub fn restart(
        &mut self,
        cells: [PlantCellAbilities; NUMBER_OF_CELLS],
        evolution_data: PlantEvolutionData,
        plant_nutrition: PlantNutrition,
    ) {
        let new_map = Self::generate(cells, evolution_data, plant_nutrition);

        self.cells = new_map.cells;
        self.evolution_data = new_map.evolution_data;
        self.time = 0;
        self.plant_nutrition = new_map.plant_nutrition;
        self.map = new_map.map;
    }

    pub fn tick(&mut self) {
        self.map = self.calculate_plant_inputs();
        self.plant_nutrition = self.calculate_plant_nutritions();
        self.grow_plant();
        self.time += 1;
    }
}

pub fn get_basic_map_data() -> (
    [PlantCellAbilities; NUMBER_OF_CELLS],
    PlantEvolutionData,
    PlantNutrition,
) {
    let basic_cell = PlantCellAbilities {
        sunlight_consumption: 0.1,
        air_consumption: 0.1,
        minerals_consumption: 0.1,
        water_consumption: 0.1,
        power_production_speed: 0.1,
        cost: 0.,
    }
    .populate_cost();

    let cells = [
        PlantCellAbilities {
            sunlight_consumption: 1.,
            air_consumption: 1.,
            minerals_consumption: 1.,
            water_consumption: 1.,
            power_production_speed: 1.,
            cost: 0.,
        }
        .populate_cost(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
        basic_cell.clone(),
    ];

    let evolution_data = PlantEvolutionData::generate();

    let plant_nutrition = PlantNutrition {
        sunlight: 100.,
        air: 100.,
        minerals: 100.,
        water: 100.,
        power: 10.,
    };

    (cells, evolution_data, plant_nutrition)
}

use std::f32;

use crate::{cell::*, const_precalc::*, evolution::PlantEvolutionData, random_evolution::*};

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
            minerals: 0.1,
            water: 0.1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MapCell {
    Air,
    Soil(SoilParameters),
    Plant(PlantCell),
}

#[derive(Debug, Default, Clone)]
pub struct PlantNutrition {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,

    pub power: f32,
}

#[derive(Debug, Clone)]
pub struct MapData {
    pub evolution_data: PlantEvolutionData,
    pub starting_plant_nutrition: PlantNutrition,

    pub time: i32,
    pub plant_nutrition: PlantNutrition,

    pub plants_pos: Vec<(usize, usize)>,
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

    fn calc_nutrition(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let dxdy = &DXDY_2D.get().unwrap()[y][x];
        let sum = dxdy.iter().fold(
            (0., 0., 0.),
            |(air, minerals, water), &(nx, ny, distance)| match &self.map[ny][nx] {
                MapCell::Air => (air + (4. - distance), minerals, water),
                MapCell::Soil(soil_parameters) => (
                    air,
                    minerals + soil_parameters.minerals,
                    water + soil_parameters.water,
                ),
                MapCell::Plant(_) => (air + (4. - distance) / 3., minerals, water),
            },
        );
        (
            sum.0 / dxdy.len() as f32,
            sum.1 / dxdy.len() as f32,
            sum.2 / dxdy.len() as f32,
        )
    }

    fn calc_cells_proximity_data(
        &self,
        x: usize,
        y: usize,
    ) -> [PlantCellProximityData; NUMBER_OF_CELLS] {
        let mut proximity_data = [PlantCellProximityData::default(); NUMBER_OF_CELLS];
        PROXIMITY_DXDY.get().unwrap()[y][x]
            .iter()
            .for_each(|&(j, i, distance, angle)| match &self.map[i][j] {
                MapCell::Plant(cell) => {
                    if proximity_data[cell.t].distance == 1. {
                        proximity_data[cell.t] = PlantCellProximityData {
                            distance,
                            direction: angle,
                        }
                    }
                }
                _ => {}
            });
        proximity_data
    }
}

impl MapData {
    fn populate_plant_inputs(&mut self) {
        for k in 0..self.plants_pos.len() {
            let (j, i) = self.plants_pos[k];

            let (air, minerals, water) = self.calc_nutrition(j, i);
            let input = PlantCellInput {
                sunlight: self.calc_sunlight(j, i),
                air,
                minerals,
                water,
                cells_proximity_data: self.calc_cells_proximity_data(j, i),
            };
            if let MapCell::Plant(cell) = &self.map[i][j] {
                self.map[i][j] = MapCell::Plant(PlantCell {
                    t: cell.t,
                    input: input,
                });
            }
        }
    }

    fn calculate_plant_nutritions(&self) -> PlantNutrition {
        let nutrition =
            self.plants_pos
                .iter()
                .fold(PlantNutrition::default(), |nutrition, &(j, i)| {
                    if let MapCell::Plant(cell) = &self.map[i][j] {
                        PlantNutrition {
                            sunlight: nutrition.sunlight
                                + cell.input.sunlight
                                    * self.evolution_data.cells_abilities[cell.t]
                                        .sunlight_consumption,
                            air: nutrition.air
                                + cell.input.air
                                    * self.evolution_data.cells_abilities[cell.t].air_consumption,
                            minerals: nutrition.minerals
                                + cell.input.minerals
                                    * self.evolution_data.cells_abilities[cell.t]
                                        .minerals_consumption,
                            water: nutrition.water
                                + cell.input.water
                                    * self.evolution_data.cells_abilities[cell.t].water_consumption,
                            power: nutrition.power
                                + self.evolution_data.cells_abilities[cell.t]
                                    .power_production_speed,
                        }
                    } else {
                        nutrition
                    }
                });

        let produced = [
            self.plant_nutrition.sunlight + nutrition.sunlight,
            self.plant_nutrition.air + nutrition.air,
            self.plant_nutrition.minerals + nutrition.minerals,
            self.plant_nutrition.water + nutrition.water,
            nutrition.power,
        ]
        .into_iter()
        .reduce(f32::min)
        .unwrap();
        PlantNutrition {
            sunlight: self.plant_nutrition.sunlight + nutrition.sunlight - produced,
            air: self.plant_nutrition.air + nutrition.air - produced,
            minerals: self.plant_nutrition.minerals + nutrition.minerals - produced,
            water: self.plant_nutrition.water + nutrition.water - produced,
            power: self.plant_nutrition.power + produced,
        }
    }

    fn grow_plant(&mut self) {
        let mut cells: [[[f32; NUMBER_OF_CELLS]; MAP_SIZE.0]; MAP_SIZE.1] =
            [[[0 as f32; NUMBER_OF_CELLS]; MAP_SIZE.0]; MAP_SIZE.1];

        self.plants_pos.iter().for_each(|&(j, i)| {
            if let MapCell::Plant(plant_cell) = &self.map[i][j] {
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
            if self.plant_nutrition.power >= self.evolution_data.cells_abilities[cell_type].cost {
                self.plant_nutrition.power -= self.evolution_data.cells_abilities[cell_type].cost;
                self.map[y][x] = MapCell::Plant(PlantCell {
                        t: cell_type,
                        input: PlantCellInput::default(),
                    });
                self.plants_pos.push((x, y));
            }
        }
    }
}

impl MapData {
    fn get_basic_map() -> [[MapCell; MAP_SIZE.0]; MAP_SIZE.1] {
        core::array::from_fn(|i| {
            core::array::from_fn(|j| {
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
        })
    }

    pub fn generate(evolution_data: PlantEvolutionData, plant_nutrition: PlantNutrition) -> Self {
        Self {
            evolution_data,
            starting_plant_nutrition: plant_nutrition.clone(),
            time: 0,
            plant_nutrition,
            plants_pos: vec![PLANT_CENTER],
            map: Self::get_basic_map(),
        }
    }

    pub fn restart(&mut self) {
        self.time = 0;
        self.plant_nutrition = self.starting_plant_nutrition.clone();
        self.map = Self::get_basic_map();
    }

    pub fn tick(&mut self) {
        self.populate_plant_inputs();
        self.plant_nutrition = self.calculate_plant_nutritions();
        self.grow_plant();
        self.time += 1;
    }
}

pub fn get_basic_map_data() -> (PlantEvolutionData, PlantNutrition) {
    let evolution_data = PlantEvolutionData::generate();

    let plant_nutrition = PlantNutrition {
        sunlight: 5.,
        air: 2.,
        minerals: 1.,
        water: 1.,
        power: 10.,
    };

    (evolution_data, plant_nutrition)
}

impl RandomEvolution for MapData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.evolution_data
            .evolve_random(rng, change_chance, change_entropy);
        self.restart();
    }
}

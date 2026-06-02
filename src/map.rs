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
    fn populate_sunlight(&mut self) {
        let mut sorted_plants = self.plants_pos.clone();
        sorted_plants.sort();
        let mut count = 0usize;
        let mut last_x = usize::MAX;
        sorted_plants.iter().for_each(|&(x, y)| {
            if x != last_x {
                count = 0;
                last_x = x;
            }
            let light = (0.5f32).powi(count as i32);
            count += 1;
            if let MapCell::Plant(cell) = &mut self.map[y][x] {
                cell.input.sunlight = light;
            }
        });
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
                    if proximity_data[cell.t].distance == 2. {
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
                sunlight: 0.,
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
        self.populate_sunlight();
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
        let mut max_data = (-1., 0, 0, 0);
        self.plants_pos.iter().for_each(|&(j, i)| {
            if let MapCell::Plant(plant_cell) = &self.map[i][j] {
                let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
                GROWTH_DIRECTION.get().unwrap()[i][j]
                    .iter()
                    .for_each(|&(nj, ni, d)| {
                        if !matches!(self.map[ni][nj], MapCell::Plant(_)) {
                            let weights = &evolution.weights[d];
                            for c in 0..NUMBER_OF_CELLS {
                                let score = weights[c].calc_cell(&plant_cell.input);
                                if score > max_data.0 {
                                    max_data = (score, nj, ni, c);
                                }
                            }
                        }
                    });
            }
        });

        if max_data.0 >= 0. {
            let (_, x, y, cell_type) = max_data;
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
        self.plants_pos = vec![PLANT_CENTER];
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

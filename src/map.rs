use std::{cell::LazyCell, f32};

use crate::{cell::*, const_precalc::*, evolution::PlantEvolutionData, random_evolution::*};

#[derive(Debug, Clone)]
pub struct PlantCell {
    pub t: usize,
    pub input: PlantCellInput,
}

impl Default for PlantCell {
    fn default() -> Self {
        Self {
            t: usize::MAX,
            input: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AirParameters {
    pub sunlight: f32,
}

impl Default for AirParameters {
    fn default() -> Self {
        Self { sunlight: 0. }
    }
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
    Air(AirParameters),
    Soil(SoilParameters),
}

#[derive(Debug, Default, Clone)]
pub struct PlantNutrition {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,

    pub power: f32,
}

// TODO: Separate for into MapData (for ui) and FullMapData (for engine)
#[derive(Debug, Clone)]
pub struct MapData {
    pub evolution_data: PlantEvolutionData,

    // TODO: Move outside
    pub starting_plant_nutrition: PlantNutrition,

    pub next_cell_growth: (f32, usize, usize, usize),

    pub evolutions: u32,
    pub ticks: u32,
    pub plant_nutrition: PlantNutrition,

    pub plants_pos: Vec<(usize, usize)>,
    pub map: [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
    pub plants: [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1],
}

impl MapData {
    fn update_sunlight(&mut self, x: usize, y: usize) {
        let mut sunlight = if y == 0 {
            1.
        } else {
            match &self.map[y - 1][x] {
                MapCell::Air(air_parameters) => air_parameters.sunlight * 0.5,
                MapCell::Soil(_) => 0.,
            }
        };

        for i in y + 1..MAP_SIZE.1 {
            match &mut self.map[i][x] {
                MapCell::Air(air_parameters) => {
                    air_parameters.sunlight = sunlight;
                    sunlight *= 0.99;
                }
                MapCell::Soil(_) => break,
            }
        }
    }

    fn calc_nutrition(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let dxdy = &DXDY_2D.get().unwrap()[y][x];

        let mut air = 0.;
        let mut minerals = 0.;
        let mut water = 0.;
        for &(nx, ny, distance) in dxdy {
            match &self.map[ny][nx] {
                MapCell::Air(_) => {
                    if self.plants[ny][nx].t == usize::MAX {
                        air += (4. - distance).sqrt()
                    } else {
                        air += (4. - distance).sqrt() / 3.;
                    }
                }
                MapCell::Soil(soil_parameters) => {
                    if self.plants[ny][nx].t == usize::MAX {
                        minerals += soil_parameters.minerals;
                        water += soil_parameters.water;
                    } else {
                        minerals += soil_parameters.minerals / 6.;
                        water += soil_parameters.water / 6.;
                    }
                }
            }
        }
        (
            air / dxdy.len() as f32,
            minerals / dxdy.len() as f32,
            water / dxdy.len() as f32,
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
            .for_each(|&(j, i, distance, angle)| {
                let cell = &self.plants[i][j];
                if cell.t != usize::MAX && proximity_data[cell.t].distance == 0. {
                    proximity_data[cell.t] = PlantCellProximityData {
                        distance,
                        direction: angle,
                    }
                }
            });
        proximity_data
    }
    
    fn update_cells_proximity_data_cell(
        &mut self,
        x: usize,
        y: usize,
    ) {
        let ct = self.plants[y][x].t;
        PROXIMITY_DXDY_REV.get().unwrap()[y][x]
            .iter()
            .for_each(|&(j, i, distance, angle)| {
                let cell = &mut self.plants[i][j];
                if cell.t != usize::MAX && cell.input.cells_proximity_data[ct].distance < distance {
                    cell.input.cells_proximity_data[ct] = PlantCellProximityData {
                        distance,
                        direction: angle,
                    }
                }
            });
    }
}

impl MapData {
    fn populate_plant_inputs(&mut self) {
        for &(j, i) in &self.plants_pos {
            let (air, minerals, water) = self.calc_nutrition(j, i);
            self.plants[i][j].input = PlantCellInput {
                sunlight: match &self.map[i][j] {
                    MapCell::Air(air_parameters) => air_parameters.sunlight,
                    MapCell::Soil(_) => 0.,
                },
                air,
                minerals,
                water,
                cells_proximity_data: self.calc_cells_proximity_data(j, i),
            };
        }
    }

    fn populate_plant_inputs_cell(&mut self, x: usize, y: usize) {
        for &(j, i) in &self.plants_pos {
            let (air, minerals, water) = self.calc_nutrition(j, i);
            self.plants[i][j].input = PlantCellInput {
                sunlight: match &self.map[i][j] {
                    MapCell::Air(air_parameters) => air_parameters.sunlight,
                    MapCell::Soil(_) => 0.,
                },
                air,
                minerals,
                water,
                cells_proximity_data: self.plants[i][j].input.cells_proximity_data,
            };
        }
        self.update_cells_proximity_data_cell(x, y);
    }

    fn calculate_plant_nutritions(&self) -> PlantNutrition {
        let nutrition =
            self.plants_pos
                .iter()
                .fold(PlantNutrition::default(), |nutrition, &(j, i)| {
                    let cell = &self.plants[i][j];
                    PlantNutrition {
                        sunlight: nutrition.sunlight
                            + cell.input.sunlight
                                * self.evolution_data.cells_abilities[cell.t].sunlight_consumption,
                        air: nutrition.air
                            + cell.input.air
                                * self.evolution_data.cells_abilities[cell.t].air_consumption,
                        minerals: nutrition.minerals
                            + cell.input.minerals
                                * self.evolution_data.cells_abilities[cell.t].minerals_consumption,
                        water: nutrition.water
                            + cell.input.water
                                * self.evolution_data.cells_abilities[cell.t].water_consumption,
                        power: nutrition.power
                            + self.evolution_data.cells_abilities[cell.t].power_production_speed,
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

    fn recalc_next_cell_growth(&mut self) {
        self.next_cell_growth = (-1., 0, 0, 0);
        self.plants_pos.iter().for_each(|&(j, i)| {
            let plant_cell = &self.plants[i][j];
            let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
            GROWTH_DIRECTION.get().unwrap()[i][j]
                .iter()
                .for_each(|&(nj, ni, d)| {
                    if self.plants[ni][nj].t == usize::MAX {
                        let weights = &evolution.weights[d];
                        for c in 0..NUMBER_OF_CELLS {
                            let score = weights[c].calc_cell(&plant_cell.input);
                            if score > self.next_cell_growth.0 {
                                self.next_cell_growth = (score, nj, ni, c);
                            }
                        }
                    }
                });
        });
    }

    fn grow_plant(&mut self) {
        if self.next_cell_growth.0 >= 0. {
            let (_, x, y, cell_type) = self.next_cell_growth;
            if self.plant_nutrition.power >= self.evolution_data.cells_abilities[cell_type].cost {
                self.plant_nutrition.power -= self.evolution_data.cells_abilities[cell_type].cost;
                self.plants[y][x] = PlantCell {
                    t: cell_type,
                    input: PlantCellInput::default(),
                };
                self.plants_pos.push((x, y));
                self.update_sunlight(x, y);
                self.populate_plant_inputs_cell(x, y);
                self.recalc_next_cell_growth();
            }
        }
    }
}

impl MapData {
    const BASIC_MAP_DATA: LazyCell<(
        [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
        [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1],
    )> = LazyCell::new(MapData::generate_basic_map);

    fn generate_basic_map() -> (
        [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
        [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1],
    ) {
        let mut sunlight = 1.;
        (
            core::array::from_fn(|i| {
                sunlight *= 0.99;
                core::array::from_fn(|j| {
                    if i <= MAP_SIZE.1 / 2 {
                        MapCell::Air(AirParameters { sunlight })
                    } else {
                        const LOW_DEPTH_MINERALS: f32 = 0.05;
                        const LOW_DEPTH_WATER: f32 = 0.2;
                        const HIGH_DEPTH_MINERALS: f32 = 0.3;
                        const HIGH_DEPTH_WATER: f32 = 0.01;
                        let depth = i - MAP_SIZE.1 / 2;
                        let depth = depth as f32 / (MAP_SIZE.1 / 2) as f32;
                        MapCell::Soil(SoilParameters {
                            minerals: LOW_DEPTH_MINERALS + (HIGH_DEPTH_MINERALS - LOW_DEPTH_MINERALS).abs() * depth,
                            water: HIGH_DEPTH_WATER + (HIGH_DEPTH_WATER - LOW_DEPTH_WATER).abs() * (1. - depth),
                        })
                    }
                })
            }),
            core::array::from_fn(|i| {
                core::array::from_fn(|j| {
                    if i == PLANT_CENTER.1 && j == PLANT_CENTER.0 {
                        PlantCell {
                            t: 0,
                            input: PlantCellInput::default(),
                        }
                    } else {
                        PlantCell::default()
                    }
                })
            }),
        )
    }

    pub fn generate(evolution_data: PlantEvolutionData, plant_nutrition: PlantNutrition) -> Self {
        let (map, plants) = Self::BASIC_MAP_DATA.clone();
        let mut s = Self {
            evolution_data,
            starting_plant_nutrition: plant_nutrition.clone(),
            next_cell_growth: (-1., 0, 0, 0),
            evolutions: 0,
            ticks: 0,
            plant_nutrition,
            plants_pos: vec![PLANT_CENTER],
            map,
            plants,
        };
        s.populate_plant_inputs();
        s.recalc_next_cell_growth();
        s
    }

    pub fn restart(&mut self) {
        self.ticks = 0;
        self.plant_nutrition = self.starting_plant_nutrition.clone();
        self.plants_pos = vec![PLANT_CENTER];
        (self.map, self.plants) = Self::BASIC_MAP_DATA.clone();
        self.populate_plant_inputs();
        self.recalc_next_cell_growth();
    }

    pub fn tick(&mut self) {
        self.plant_nutrition = self.calculate_plant_nutritions();
        self.grow_plant();
        self.ticks += 1;
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
        self.evolutions += 1;
    }
}

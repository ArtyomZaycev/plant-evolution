use std::f32;

use crate::{cell::*, const_precalc::*};

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
        for i in 0..MAP_SIZE.1 {
            for j in 0..MAP_SIZE.0 {
                match self.map[i][j].clone() {
                    MapCell::Plant(cell) => {
                        let (air, minerals, water) = self.calc_nutrition(j, i);
                        let input = PlantCellInput {
                            sunlight: self.calc_sunlight(j, i),
                            air,
                            minerals,
                            water,
                            cells_proximity_data: self.calc_cells_proximity_data(j, i),
                        };
                        self.map[i][j] = MapCell::Plant(PlantCell {
                            input: input,
                            ..cell
                        });
                    }
                    _ => {}
                }
            }
        }
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
        self.populate_plant_inputs();
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

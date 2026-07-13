use std::{cell::LazyCell, collections::HashMap, f32};

use super::{map_cell::*, plant_cell::*};
use crate::{evolution::*, precalc::*, utils::*};

#[derive(Debug, Default, Clone)]
pub struct PlantNutrition {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,

    pub energy: f32,
}

// TODO: Separate for into MapData (for ui) and FullMapData (for engine)
#[derive(Debug, Clone)]
pub struct MapData {
    pub evolution_data: PlantEvolutionData,

    // TODO: Move outside
    pub starting_plant_nutrition: PlantNutrition,

    pub next_cell_growth: (f32, usize, usize, usize),
    pub next_cell_suicide: (f32, usize, usize),

    pub ticks: u32,
    pub plant_nutrition: PlantNutrition,

    pub total_passive_cost: f32,
    pub nutrition_per_tick: PlantNutrition,

    pub cells_pos: Vec<(usize, usize)>,
    pub map: [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
    pub cells: [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1],
}

impl Default for MapData {
    fn default() -> Self {
        let (a, b) = get_basic_map_data();
        Self::generate(a, b)
    }
}

impl MapData {
    #[hotpath::measure]
    fn update_sunlight(&mut self, x: usize, y: usize) {
        let mut sunlight = if y == 0 {
            1.
        } else {
            match &self.map[y - 1][x] {
                MapCell::Air(air_parameters) => air_parameters.sunlight * 0.3,
                MapCell::Soil(_) => 0.,
            }
        };

        for i in y + 1..MAP_SIZE.1 {
            if sunlight < 0.001 {
                break;
            }
            match &mut self.map[i][x] {
                MapCell::Air(air_parameters) => {
                    air_parameters.sunlight = sunlight;
                    if self.cells[i][x].is_some() {
                        sunlight *= 0.3;
                    } else {
                        sunlight *= 0.93;
                    }
                }
                MapCell::Soil(_) => break,
            }
        }
    }

    #[hotpath::measure]
    fn update_sunlight_all(&mut self) {
        for x in 0..MAP_SIZE.0 {
            self.update_sunlight(x, 0);
        }
    }

    #[hotpath::measure]
    fn calc_nutrition(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let dxdy = &DXDY_2D[y][x];

        let mut air = 0.;
        let mut minerals = 0.;
        let mut water = 0.;
        match &self.map[y][x] {
            MapCell::Air(_) => {
                for &(nx, ny, distance) in dxdy {
                    match &self.map[ny][nx] {
                        MapCell::Air(_) => {
                            if self.cells[ny][nx].is_none() {
                                air += distance
                            } else {
                                air += distance / 8.;
                            }
                        }
                        MapCell::Soil(_) => {}
                    }
                }
            }
            MapCell::Soil(_) => {
                for &(nx, ny, distance) in dxdy {
                    match &self.map[ny][nx] {
                        MapCell::Air(_) => {}
                        MapCell::Soil(soil_parameters) => {
                            if self.cells[ny][nx].is_none() {
                                minerals += soil_parameters.minerals * distance;
                                water += soil_parameters.water * distance;
                            } else {
                            }
                        }
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

    #[hotpath::measure]
    fn calc_cells_proximity_data(&self, x: usize, y: usize) -> [[bool; NUMBER_OF_CELLS]; 4] {
        let mut proximity_data = [[false; NUMBER_OF_CELLS]; 4];
        let xidx = if x >= PLANT_CENTER.0 { 0 } else { 1 };
        if x > 0 && self.cells[y][x - 1].is_some() {
            proximity_data[xidx][self.cells[y][x - 1].t] = true;
        }
        if x + 1 < MAP_SIZE.0 && self.cells[y][x + 1].is_some() {
            proximity_data[1 - xidx][self.cells[y][x + 1].t] = true;
        }
        if y > 0 && self.cells[y - 1][x].is_some() {
            proximity_data[2][self.cells[y - 1][x].t] = true;
        }
        if y + 1 < MAP_SIZE.1 && self.cells[y + 1][x].is_some() {
            proximity_data[3][self.cells[y + 1][x].t] = true;
        }
        proximity_data
    }
}

impl MapData {
    #[hotpath::measure]
    fn populate_plant_inputs(&mut self) {
        for &(j, i) in &self.cells_pos {
            let (air, minerals, water) = self.calc_nutrition(j, i);
            self.cells[i][j].input = PlantCellInput {
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

    #[hotpath::measure]
    fn recalc_plant_nutrition(&mut self) {
        self.total_passive_cost = 0.;
        self.nutrition_per_tick =
            self.cells_pos
                .iter()
                .fold(PlantNutrition::default(), |nutrition, &(j, i)| {
                    let cell = &self.cells[i][j];
                    let abilities = &self.evolution_data.cells_abilities[cell.t];
                    self.total_passive_cost += abilities.passive_cost;
                    PlantNutrition {
                        sunlight: nutrition.sunlight
                            + cell.input.sunlight * *abilities.sunlight_consumption,
                        air: nutrition.air + cell.input.air * *abilities.air_consumption,
                        minerals: nutrition.minerals
                            + cell.input.minerals * *abilities.minerals_consumption,
                        water: nutrition.water + cell.input.water * *abilities.water_consumption,
                        energy: nutrition.energy + *abilities.energy_production_speed,
                    }
                });
    }

    #[hotpath::measure]
    fn update_plant_nutritions(&mut self) {
        let produced = [
            self.plant_nutrition.sunlight + self.nutrition_per_tick.sunlight,
            self.plant_nutrition.air + self.nutrition_per_tick.air,
            self.plant_nutrition.minerals + self.nutrition_per_tick.minerals,
            self.plant_nutrition.water + self.nutrition_per_tick.water,
            self.nutrition_per_tick.energy,
        ]
        .into_iter()
        .reduce(f32::min)
        .unwrap();
        self.plant_nutrition.sunlight += self.nutrition_per_tick.sunlight - produced;
        self.plant_nutrition.air += self.nutrition_per_tick.air - produced;
        self.plant_nutrition.minerals += self.nutrition_per_tick.minerals - produced;
        self.plant_nutrition.water += self.nutrition_per_tick.water - produced;
        self.plant_nutrition.energy += produced - self.total_passive_cost;
    }

    // TODO: Optimize
    #[hotpath::measure]
    fn recalc_next_cell_growth(&mut self) {
        let mut growth_w = HashMap::new();
        self.next_cell_growth = (f32::NEG_INFINITY, 0, 0, 0);
        self.cells_pos.iter().for_each(|&(j, i)| {
            let plant_cell = &self.cells[i][j];
            let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
            GROWTH_DIRECTION[i][j].iter().for_each(|&(nj, ni, d)| {
                if self.cells[ni][nj].t == usize::MAX {
                    let weights = &evolution.weights[d];
                    for c in 0..NUMBER_OF_CELLS {
                        let score = weights[c].calculate(
                            &plant_cell.input,
                            (1. - i as f32 / MAP_SIZE.1 as f32) * 2. - 1.,
                            (j as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.),
                        );
                        let cw = growth_w.entry((ni, nj, c)).or_default();
                        *cw += score;
                        if *cw >= self.next_cell_growth.0 {
                            self.next_cell_growth = (*cw, nj, ni, c);
                        }
                    }
                }
            });
        });
    }

    #[hotpath::measure]
    fn recalc_next_cell_suicide(&mut self) {
        self.next_cell_suicide = (f32::NEG_INFINITY, 0, 0);
        self.cells_pos.iter().for_each(|&(j, i)| {
            if j != PLANT_CENTER.0 || i != PLANT_CENTER.1 {
                let plant_cell = &self.cells[i][j];
                let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
                let score = evolution.calc_suicide(
                    &plant_cell.input,
                    (1. - i as f32 / MAP_SIZE.1 as f32) * 2. - 1.,
                    (j as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.),
                );
                if score > self.next_cell_suicide.0 {
                    self.next_cell_suicide = (score, j, i);
                }
            }
        });
    }

    #[hotpath::measure]
    fn search_cells(&self, x: usize, y: usize, ex_plants: &mut [[bool; MAP_SIZE.1]; MAP_SIZE.0]) {
        ex_plants[y][x] = true;
        if x > 0 && !ex_plants[y][x - 1] && self.cells[y][x - 1].is_some() {
            self.search_cells(x - 1, y, ex_plants);
        }
        if x + 1 < MAP_SIZE.0 && !ex_plants[y][x + 1] && self.cells[y][x + 1].is_some() {
            self.search_cells(x + 1, y, ex_plants);
        }
        if y > 0 && !ex_plants[y - 1][x] && self.cells[y - 1][x].is_some() {
            self.search_cells(x, y - 1, ex_plants);
        }
        if y + 1 < MAP_SIZE.1 && !ex_plants[y + 1][x] && self.cells[y + 1][x].is_some() {
            self.search_cells(x, y + 1, ex_plants);
        }
    }

    #[hotpath::measure]
    pub fn remove_cell(&mut self, x: usize, y: usize) {
        self.cells[y][x] = PlantCell::default();
        self.cells_pos = vec![PLANT_CENTER];
        let mut ex_plants = [[false; MAP_SIZE.0]; MAP_SIZE.1];
        self.search_cells(PLANT_CENTER.0, PLANT_CENTER.1, &mut ex_plants);

        for i in 0..MAP_SIZE.1 {
            for j in 0..MAP_SIZE.0 {
                if self.cells[i][j].is_some() {
                    if !ex_plants[i][j] {
                        self.cells[i][j] = PlantCell::default();
                    } else {
                        self.cells_pos.push((j, i));
                    }
                }
            }
        }
    }

    #[hotpath::measure]
    fn grow_plant(&mut self) {
        if self.next_cell_growth.0 >= 0. || self.next_cell_suicide.0 >= 0. {
            if self.next_cell_suicide.0 > self.next_cell_growth.0 {
                let (_, x, y) = self.next_cell_suicide;
                if x != PLANT_CENTER.0 || y != PLANT_CENTER.1 {
                    self.remove_cell(x, y);
                    self.update_sunlight_all();
                    self.populate_plant_inputs();
                    self.recalc_plant_nutrition();
                    self.recalc_next_cell_growth();
                    self.recalc_next_cell_suicide();
                }
            } else {
                let (_, x, y, cell_type) = self.next_cell_growth;
                if self.plant_nutrition.energy
                    >= self.evolution_data.cells_abilities[cell_type].grow_cost
                {
                    self.plant_nutrition.energy -=
                        self.evolution_data.cells_abilities[cell_type].grow_cost;
                    self.cells[y][x] = PlantCell {
                        t: cell_type,
                        input: PlantCellInput::default(),
                    };
                    self.cells_pos.push((x, y));
                    self.update_sunlight(x, y);
                    self.populate_plant_inputs();
                    self.recalc_plant_nutrition();
                    self.recalc_next_cell_growth();
                    self.recalc_next_cell_suicide();
                }
            }
        }
    }
}

impl MapData {
    const BASIC_MAP: LazyCell<[[MapCell; MAP_SIZE.0]; MAP_SIZE.1]> =
        LazyCell::new(MapData::generate_basic_map);
    const BASIC_PLANTS: LazyCell<[[PlantCell; MAP_SIZE.0]; MAP_SIZE.1]> =
        LazyCell::new(MapData::generate_basic_plants);

    fn generate_basic_map() -> [[MapCell; MAP_SIZE.0]; MAP_SIZE.1] {
        let mut sunlight = 1.;
        core::array::from_fn(|i| {
            sunlight *= 0.99;
            core::array::from_fn(|_| {
                if i <= MAP_SIZE.1 / 2 {
                    MapCell::Air(AirParameters { sunlight })
                } else {
                    const LOW_DEPTH_MINERALS: f32 = 0.1;
                    const LOW_DEPTH_WATER: f32 = 0.2;
                    const HIGH_DEPTH_MINERALS: f32 = 0.3;
                    const HIGH_DEPTH_WATER: f32 = 0.01;
                    let depth = i - MAP_SIZE.1 / 2;
                    let depth = depth as f32 / (MAP_SIZE.1 / 2) as f32;
                    MapCell::Soil(SoilParameters {
                        minerals: LOW_DEPTH_MINERALS
                            + (HIGH_DEPTH_MINERALS - LOW_DEPTH_MINERALS).abs() * depth,
                        water: HIGH_DEPTH_WATER
                            + (HIGH_DEPTH_WATER - LOW_DEPTH_WATER).abs() * (1. - depth),
                    })
                }
            })
        })
    }

    fn generate_basic_plants() -> [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1] {
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
        })
    }

    pub fn generate(evolution_data: PlantEvolutionData, plant_nutrition: PlantNutrition) -> Self {
        let mut s = Self {
            evolution_data,
            starting_plant_nutrition: plant_nutrition.clone(),
            next_cell_growth: (f32::NEG_INFINITY, 0, 0, 0),
            next_cell_suicide: (f32::NEG_INFINITY, 0, 0),
            ticks: 0,
            plant_nutrition,
            total_passive_cost: 0.,
            nutrition_per_tick: PlantNutrition::default(),
            cells_pos: vec![PLANT_CENTER],
            map: Self::BASIC_MAP.clone(),
            cells: Self::BASIC_PLANTS.clone(),
        };
        s.populate_plant_inputs();
        s.recalc_plant_nutrition();
        s.recalc_next_cell_growth();
        s.recalc_next_cell_suicide();
        s
    }

    #[hotpath::measure]
    pub fn restart(&mut self) {
        self.ticks = 0;
        self.plant_nutrition = self.starting_plant_nutrition.clone();
        hotpath::measure_block!("restart: map&plants clone", {
            // Time is literally the same as clone
            /*unsafe {
                // Do not inline, it breaks somehow
                let rf = &Self::BASIC_MAP;
                let basic_map = LazyCell::force(rf);
                let src_ptr = basic_map.as_ptr();
                let dst_ptr = self.map.as_mut_ptr();
                // Size of the outer array, as inner is accounted for in T
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, MAP_SIZE.1);
            }*/
            //self.map.copy_from_slice(&*Self::BASIC_MAP);
            
            self.cells_pos.iter().for_each(|&(x, y)| {
                self.cells[y][x].t = usize::MAX;
            });
            self.cells[PLANT_CENTER.1][PLANT_CENTER.0].t = 0;
            self.map = Self::BASIC_MAP.clone();
        });
        self.cells_pos = vec![PLANT_CENTER];
        self.populate_plant_inputs();
        self.recalc_next_cell_growth();
    }

    #[hotpath::measure]
    pub fn tick(&mut self) {
        self.update_plant_nutritions();
        self.grow_plant();
        self.ticks += 1;
    }

    #[hotpath::measure]
    pub fn calculate_score(&self) -> f32 {
        let mut seeds = vec![];

        self.cells_pos.iter().for_each(|&(j, i)| {
            let cell = &self.cells[i][j];
            let abilities = &self.evolution_data.cells_abilities[cell.t];
            if abilities.seed && matches!(self.map[i][j], MapCell::Air(_)) {
                seeds.push((j, i));
            }
        });

        let mut seeds_score: f32 = 0.;
        for &(x, y) in &seeds {
            let mut cnt = 0;
            for &(x2, y2) in &seeds {
                if x != x2 || y != y2 {
                    if (x as f32 - x2 as f32).powi(2) + (y as f32 - y2 as f32).powi(2) < 25. {
                        cnt += 1;
                    }
                }
            }
            seeds_score += 2. / (cnt + 1) as f32;
        }

        (seeds_score * 10.)
            + ([
                self.nutrition_per_tick.sunlight,
                self.nutrition_per_tick.air,
                self.nutrition_per_tick.minerals,
                self.nutrition_per_tick.water,
                self.nutrition_per_tick.energy,
            ]
            .into_iter()
            .reduce(f32::min)
            .unwrap()
                * 100.)
                .sqrt()
    }
}

pub fn get_basic_map_data() -> (PlantEvolutionData, PlantNutrition) {
    let mut rng = get_rng();
    let evolution_data = PlantEvolutionData::generate(&mut rng);

    let plant_nutrition = PlantNutrition {
        sunlight: 20.,
        air: 20.,
        minerals: 1.,
        water: 1.,
        energy: 20.,
    };

    (evolution_data, plant_nutrition)
}

impl RandomEvolution for MapData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        self.evolution_data
            .evolve_random(rng, change_chance, change_entropy)
    }
}

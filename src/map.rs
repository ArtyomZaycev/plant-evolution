use std::{cell::LazyCell, collections::HashMap, f32};

use crate::{cell::*, const_precalc::*, evolution::PlantEvolutionData, random_evolution::*};

#[derive(Debug, Clone)]
pub struct PlantCell {
    pub t: usize,
    pub input: PlantCellInput,
}

impl PlantCell {
    pub fn is_none(&self) -> bool {
        self.t == usize::MAX
    }
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
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
    pub next_cell_suicide: (f32, usize, usize),

    pub evolutions: u32,
    pub ticks: u32,
    pub plant_nutrition: PlantNutrition,

    pub plants_pos: Vec<(usize, usize)>,
    pub map: [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
    pub plants: [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1],
}

impl MapData {
    #[hotpath::measure]
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
            if sunlight < 0.001 {
                break;
            }
            match &mut self.map[i][x] {
                MapCell::Air(air_parameters) => {
                    air_parameters.sunlight = sunlight;
                    if self.plants[i][x].is_some() {
                        sunlight *= 0.4;
                    } else {
                        sunlight *= 0.99;
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
        let dxdy = &DXDY_2D.get().unwrap()[y][x];

        let mut air = 0.;
        let mut minerals = 0.;
        let mut water = 0.;
        match &self.map[y][x] {
            MapCell::Air(_) => {
                for &(nx, ny, distance) in dxdy {
                    match &self.map[ny][nx] {
                        MapCell::Air(_) => {
                            if self.plants[ny][nx].is_none() {
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
                            if self.plants[ny][nx].is_none() {
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
        if x > 0 && self.plants[y][x - 1].is_some() {
            proximity_data[xidx][self.plants[y][x - 1].t] = true;
        }
        if x + 1 < MAP_SIZE.0 && self.plants[y][x + 1].is_some() {
            proximity_data[1 - xidx][self.plants[y][x + 1].t] = true;
        }
        if y > 0 && self.plants[y - 1][x].is_some() {
            proximity_data[2][self.plants[y - 1][x].t] = true;
        }
        if y + 1 < MAP_SIZE.1 && self.plants[y + 1][x].is_some() {
            proximity_data[3][self.plants[y + 1][x].t] = true;
        }
        proximity_data
    }
}

impl MapData {
    #[hotpath::measure]
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

    #[hotpath::measure]
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
            power: self.plant_nutrition.power + produced - self.plants_pos.len() as f32 * 0.002,
        }
    }

    #[hotpath::measure]
    fn recalc_next_cell_growth(&mut self) {
        let mut growth_w = HashMap::new();
        self.next_cell_growth = (f32::NEG_INFINITY, 0, 0, 0);
        self.plants_pos.iter().for_each(|&(j, i)| {
            let plant_cell = &self.plants[i][j];
            let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
            GROWTH_DIRECTION.get().unwrap()[i][j]
                .iter()
                .for_each(|&(nj, ni, d)| {
                    if self.plants[ni][nj].t == usize::MAX {
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
        self.plants_pos.iter().for_each(|&(j, i)| {
            if j != PLANT_CENTER.0 || i != PLANT_CENTER.1 {
                let plant_cell = &self.plants[i][j];
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
        if x > 0 && !ex_plants[y][x - 1] && self.plants[y][x - 1].is_some() {
            self.search_cells(x - 1, y, ex_plants);
        }
        if x + 1 < MAP_SIZE.0 && !ex_plants[y][x + 1] && self.plants[y][x + 1].is_some() {
            self.search_cells(x + 1, y, ex_plants);
        }
        if y > 0 && !ex_plants[y - 1][x] && self.plants[y - 1][x].is_some() {
            self.search_cells(x, y - 1, ex_plants);
        }
        if y + 1 < MAP_SIZE.1 && !ex_plants[y + 1][x] && self.plants[y + 1][x].is_some() {
            self.search_cells(x, y + 1, ex_plants);
        }
    }

    #[hotpath::measure]
    pub fn remove_cell(&mut self, x: usize, y: usize) {
        self.plants[y][x] = PlantCell::default();
        self.plants_pos = vec![PLANT_CENTER];
        let mut ex_plants = [[false; MAP_SIZE.0]; MAP_SIZE.1];
        self.search_cells(PLANT_CENTER.0, PLANT_CENTER.1, &mut ex_plants);

        for i in 0..MAP_SIZE.1 {
            for j in 0..MAP_SIZE.0 {
                if self.plants[i][j].is_some() {
                    if !ex_plants[i][j] {
                        self.plants[i][j] = PlantCell::default();
                    } else {
                        self.plants_pos.push((j, i));
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
                    self.recalc_next_cell_growth();
                    self.recalc_next_cell_suicide();
                }
            } else {
                let (_, x, y, cell_type) = self.next_cell_growth;
                if self.plant_nutrition.power >= self.evolution_data.cells_abilities[cell_type].cost
                {
                    self.plant_nutrition.power -=
                        self.evolution_data.cells_abilities[cell_type].cost;
                    self.plants[y][x] = PlantCell {
                        t: cell_type,
                        input: PlantCellInput::default(),
                    };
                    self.plants_pos.push((x, y));
                    self.update_sunlight(x, y);
                    self.populate_plant_inputs();
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
                    const LOW_DEPTH_MINERALS: f32 = 0.05;
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
            evolutions: 0,
            ticks: 0,
            plant_nutrition,
            plants_pos: vec![PLANT_CENTER],
            map: Self::BASIC_MAP.clone(),
            plants: Self::BASIC_PLANTS.clone(),
        };
        s.populate_plant_inputs();
        s.recalc_next_cell_growth();
        s.recalc_next_cell_suicide();
        s
    }

    #[hotpath::measure]
    pub fn restart(&mut self) {
        self.ticks = 0;
        self.plant_nutrition = self.starting_plant_nutrition.clone();
        self.plants_pos = vec![PLANT_CENTER];
        /*
        unsafe {
            let src_ptr = Self::BASIC_MAP.as_ptr();
            let dst_ptr = self.map.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, MAP_SIZE.0 * MAP_SIZE.1);
        }
        unsafe {
            let src_ptr = Self::BASIC_PLANTS.as_ptr();
            let dst_ptr = self.plants.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, MAP_SIZE.0 * MAP_SIZE.1);
        } */
        self.map = Self::BASIC_MAP.clone();
        self.plants = Self::BASIC_PLANTS.clone();
        self.populate_plant_inputs();
        self.recalc_next_cell_growth();
    }

    #[hotpath::measure]
    pub fn tick(&mut self) {
        self.plant_nutrition = self.calculate_plant_nutritions();
        self.grow_plant();
        self.ticks += 1;
    }
}

pub fn get_basic_map_data() -> (PlantEvolutionData, PlantNutrition) {
    let evolution_data = PlantEvolutionData::generate();

    let plant_nutrition = PlantNutrition {
        sunlight: 20.,
        air: 20.,
        minerals: 1.,
        water: 1.,
        power: 20.,
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

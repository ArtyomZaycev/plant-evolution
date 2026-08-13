use std::{cell::LazyCell, collections::HashMap, f32};

use super::{map_cell::*, plant_cell::*};
use crate::{
    evolution::{consts::*, *},
    precalc::*,
    utils::*,
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PlantNutrition {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,

    pub energy: f32,
}

impl PlantNutrition {
    pub const STARTING: Self = Self {
        sunlight: 20.,
        air: 20.,
        minerals: 1.,
        water: 1.,
        energy: 20.,
    };
}

// TODO: Separate for into MapData (for ui) and FullMapData (for engine)
#[derive(Debug, Clone)]
pub struct MapData {
    pub evolution_data: PlantEvolutionData,

    pub next_cell_growth: (f32, usize, usize, usize),
    pub next_cell_suicide: (f32, usize, usize),

    pub ticks: u32,
    pub plant_nutrition: PlantNutrition,

    pub total_passive_cost: f32,
    pub nutrition_per_tick: PlantNutrition,

    pub all_next_cell_growth: HashMap<(usize, usize), [f32; NUMBER_OF_CELLS]>,
    pub cells_pos: Vec<(usize, usize)>,
    pub map: [[MapCell; MAP_SIZE.0]; MAP_SIZE.1],
    pub cells: [[PlantCell; MAP_SIZE.0]; MAP_SIZE.1],
}

impl Default for MapData {
    fn default() -> Self {
        let mut rng = get_rng();
        Self::generate(
            PlantEvolutionData::generate(&mut rng),
            PlantNutrition::STARTING,
        )
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
                        sunlight *= SUNLIGHT_CELL_MULTIPLIER;
                    } else {
                        sunlight *= SUNLIGHT_AIR_MULTIPLIER;
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
        let dxdy = &DXDY2_2D[y][x];

        let mut air = 0.;
        let mut minerals = 0.;
        let mut water = 0.;
        match &self.map[y][x] {
            MapCell::Air(_) => {
                for &(nx, ny, distance) in dxdy {
                    match &self.map[ny][nx] {
                        MapCell::Air(_) => {
                            if self.cells[ny][nx].is_none() {
                                air += distance * AIR_AIR_MULTIPLIER;
                            } else {
                                air += distance * AIR_CELL_MULTIPLIER;
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

    fn update_plant_nutritions(&mut self, ticks: u32) {
        let ticks = ticks as f32;
        let produced = [
            self.plant_nutrition.sunlight + self.nutrition_per_tick.sunlight * ticks,
            self.plant_nutrition.air + self.nutrition_per_tick.air * ticks,
            self.plant_nutrition.minerals + self.nutrition_per_tick.minerals * ticks,
            self.plant_nutrition.water + self.nutrition_per_tick.water * ticks,
            self.nutrition_per_tick.energy * ticks,
        ]
        .into_iter()
        .reduce(f32::min)
        .unwrap();
        self.plant_nutrition.sunlight += self.nutrition_per_tick.sunlight * ticks - produced;
        self.plant_nutrition.air += self.nutrition_per_tick.air * ticks - produced;
        self.plant_nutrition.minerals += self.nutrition_per_tick.minerals * ticks - produced;
        self.plant_nutrition.water += self.nutrition_per_tick.water * ticks - produced;
        self.plant_nutrition.energy += produced - self.total_passive_cost * ticks;
    }

    // TODO: Measure performance agains simpler comparisons
    /// Determenistic way of choosing next cell growth
    fn compare_next_cell_growth(
        x1: usize,
        y1: usize,
        c1: usize,
        w1: f32,
        x2: usize,
        y2: usize,
        c2: usize,
        w2: f32,
    ) -> std::cmp::Ordering {
        (
            // Weight (how much the plant wants to grow here)
            w1,
            // Horizontal distance from the center (less is better)
            -((x1.abs_diff(PLANT_CENTER.0) + y1.abs_diff(PLANT_CENTER.1)) as i32),
            // Cell type (less is better)
            -(c1 as i32),
            // Horizontal position
            x1,
            // Vertical position
            y1,
        )
            .partial_cmp(&(
                w2,
                -((x2.abs_diff(PLANT_CENTER.0) + y2.abs_diff(PLANT_CENTER.1)) as i32),
                -(c2 as i32),
                x2,
                y2,
            ))
            .unwrap()
    }

    fn update_next_cell_growth_array(
        from: &PlantCell,
        height: f32,
        xdist: f32,
        weights: &[WithVolatility<WeightsTree>; NUMBER_OF_CELLS],
        next_cell_growth: &mut [f32; NUMBER_OF_CELLS],
    ) {
        for c in 0..NUMBER_OF_CELLS {
            let score = weights[c].calculate(&from.input, height, xdist);
            // Can't update self.next_cell_growth here
            // That would not account for negative results later
            next_cell_growth[c] += score;
        }
    }

    fn update_next_cell_growth_from_calc(&mut self) {
        self.next_cell_growth = (f32::NEG_INFINITY, 0, 0, 0);
        for (&(x, y), carr) in &self.all_next_cell_growth {
            for (c, &w) in carr.iter().enumerate() {
                if Self::compare_next_cell_growth(
                    x,
                    y,
                    c,
                    w,
                    self.next_cell_growth.1,
                    self.next_cell_growth.2,
                    self.next_cell_growth.3,
                    self.next_cell_growth.0,
                )
                .is_gt()
                {
                    self.next_cell_growth = (w, x, y, c);
                }
            }
        }
    }

    #[hotpath::measure]
    fn recalc_all_next_cell_growth(&mut self) {
        self.all_next_cell_growth.clear();
        self.cells_pos.iter().for_each(|&(j, i)| {
            let plant_cell = &self.cells[i][j];
            let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
            GROWTH_DIRECTION[i][j].iter().for_each(|&(nj, ni, d)| {
                if self.cells[ni][nj].is_none() {
                    let weights = &evolution.weights[d];
                    let next_cell_growth = self.all_next_cell_growth.entry((nj, ni)).or_default();
                    Self::update_next_cell_growth_array(
                        plant_cell,
                        (1. - i as f32 / MAP_SIZE.1 as f32) * 2. - 1.,
                        (j as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.),
                        weights,
                        next_cell_growth,
                    );
                }
            });
        });
        self.update_next_cell_growth_from_calc();
    }

    /// Assumes new cells has grown at (x, y)
    fn recalc_next_cell_growth(&mut self, x: usize, y: usize) {
        let recalc_needed = &GROWTH_RECALC_NEEDED_FOR[y][x];

        self.all_next_cell_growth.remove(&(x, y));
        recalc_needed.iter().for_each(|&(nx, ny)| {
            if let Some(value) = self.all_next_cell_growth.get_mut(&(nx, ny)) {
                *value = Default::default();
            }
        });

        // TODO: only check adjacent to recalc_needed cells
        self.cells_pos.iter().for_each(|&(j, i)| {
            let plant_cell = &self.cells[i][j];
            let evolution = &self.evolution_data.cells_evolution_data[plant_cell.t];
            GROWTH_DIRECTION[i][j].iter().for_each(|&(nj, ni, d)| {
                if self.cells[ni][nj].is_none() && recalc_needed.contains(&(nj, ni)) {
                    let weights = &evolution.weights[d];
                    let next_cell_growth = self.all_next_cell_growth.entry((nj, ni)).or_default();
                    Self::update_next_cell_growth_array(
                        plant_cell,
                        (1. - i as f32 / MAP_SIZE.1 as f32) * 2. - 1.,
                        (j as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.),
                        weights,
                        next_cell_growth,
                    );
                }
            });
        });
        self.update_next_cell_growth_from_calc();
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
    fn search_cells(&self, x: usize, y: usize, ex_plants: &mut [[bool; MAP_SIZE.0]; MAP_SIZE.1]) {
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

    fn do_grow_plant_cell(
        &mut self,
        use_local_growth_recalculation: bool,
        x: usize,
        y: usize,
        cell_type: usize,
    ) {
        self.plant_nutrition.energy -= self.evolution_data.cells_abilities[cell_type].grow_cost;
        self.cells[y][x] = PlantCell {
            t: cell_type,
            input: PlantCellInput::default(),
        };
        self.cells_pos.push((x, y));
        self.update_sunlight(x, y);
        self.populate_plant_inputs();
        self.recalc_plant_nutrition();
        if use_local_growth_recalculation {
            self.recalc_next_cell_growth(x, y);
        } else {
            self.recalc_all_next_cell_growth();
        }
        self.recalc_next_cell_suicide();
    }

    fn grow_plant(&mut self, use_local_growth_recalculation: bool) {
        if self.next_cell_growth.0 >= 0. || self.next_cell_suicide.0 >= 0. {
            if self.next_cell_suicide.0 >= self.next_cell_growth.0 {
                let (_, x, y) = self.next_cell_suicide;
                if x != PLANT_CENTER.0 || y != PLANT_CENTER.1 {
                    self.remove_cell(x, y);
                    self.update_sunlight_all();
                    self.populate_plant_inputs();
                    self.recalc_plant_nutrition();
                    self.recalc_all_next_cell_growth();
                    self.recalc_next_cell_suicide();
                }
            } else {
                let (_, x, y, cell_type) = self.next_cell_growth;
                if self.plant_nutrition.energy
                    >= self.evolution_data.cells_abilities[cell_type].grow_cost
                {
                    self.do_grow_plant_cell(use_local_growth_recalculation, x, y, cell_type);
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
            sunlight *= SUNLIGHT_AIR_MULTIPLIER;
            core::array::from_fn(|_| {
                if i < GROUND_LEVEL {
                    MapCell::Air(AirParameters { sunlight })
                } else {
                    // TODO: Use GROUND_LEVEL
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

    fn fill_as_basic_map(map: &mut [[MapCell; MAP_SIZE.0]; MAP_SIZE.1]) {
        let mut sunlight = 1.;
        // Soil is always the same, no need to update it
        map[..GROUND_LEVEL].iter_mut().for_each(|row| {
            sunlight *= SUNLIGHT_AIR_MULTIPLIER;
            row.fill(MapCell::Air(AirParameters { sunlight }));
        });
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
            next_cell_growth: (f32::NEG_INFINITY, 0, 0, 0),
            next_cell_suicide: (f32::NEG_INFINITY, 0, 0),
            ticks: 0,
            plant_nutrition,
            total_passive_cost: 0.,
            nutrition_per_tick: PlantNutrition::default(),
            all_next_cell_growth: HashMap::new(),
            cells_pos: vec![PLANT_CENTER],
            map: Self::BASIC_MAP.clone(),
            cells: Self::BASIC_PLANTS.clone(),
        };
        s.populate_plant_inputs();
        s.recalc_plant_nutrition();
        s.recalc_all_next_cell_growth();
        s.recalc_next_cell_suicide();
        s
    }

    #[hotpath::measure]
    pub fn restart(&mut self) {
        self.ticks = 0;
        self.plant_nutrition = PlantNutrition::STARTING;
        hotpath::measure_block!("map_restart_cloning", {
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
            Self::fill_as_basic_map(&mut self.map);
        });
        self.cells_pos.resize(1, PLANT_CENTER);
        self.cells_pos[0] = PLANT_CENTER;

        self.populate_plant_inputs();
        self.recalc_all_next_cell_growth();
    }

    pub fn tick(&mut self, use_local_growth_recalculation: bool) {
        self.update_plant_nutritions(1);
        self.grow_plant(use_local_growth_recalculation);
        self.ticks += 1;
    }

    pub fn tick_many(&mut self, ticks: u32, use_local_growth_recalculation: bool) {
        if ticks == 0 {
            return;
        } else if ticks == 1 {
            self.tick(use_local_growth_recalculation);
            return;
        }

        assert!(ticks < 10000);

        if self.next_cell_growth.0 >= 0. || self.next_cell_suicide.0 >= 0. {
            if self.next_cell_suicide.0 >= self.next_cell_growth.0 {
                // Next action is cell suicide
                self.tick(use_local_growth_recalculation);
                self.tick_many(ticks - 1, use_local_growth_recalculation);
            } else if self.plant_nutrition.energy
                >= self.evolution_data.cells_abilities[self.next_cell_growth.3].grow_cost
            {
                // Plant has enough energy for growth
                self.tick(use_local_growth_recalculation);
                self.tick_many(ticks - 1, use_local_growth_recalculation);
            } else if self.total_passive_cost >= self.nutrition_per_tick.energy {
                // There will never be enough energy for next growth
                self.update_plant_nutritions(ticks);
                self.ticks += ticks;
            } else {
                let required_energy = self.evolution_data.cells_abilities[self.next_cell_growth.3]
                    .grow_cost
                    - self.plant_nutrition.energy;
                let energy_per_tick = self.nutrition_per_tick.energy - self.total_passive_cost;
                // In best case scenario
                let till_enough_energy = (required_energy / energy_per_tick).ceil() as u32;

                if till_enough_energy > ticks {
                    // Will not have enough ticks
                    self.update_plant_nutritions(ticks);
                    self.ticks += ticks;
                } else {
                    let till_depleted_fn = |r: f32, rpt: f32| {
                        if rpt >= self.nutrition_per_tick.energy {
                            u32::MAX
                        } else {
                            (r / (self.nutrition_per_tick.energy - rpt)).floor() as u32
                        }
                    };

                    let till_sunlight_depleted = till_depleted_fn(
                        self.plant_nutrition.sunlight,
                        self.nutrition_per_tick.sunlight,
                    );
                    let till_air_depleted =
                        till_depleted_fn(self.plant_nutrition.air, self.nutrition_per_tick.air);
                    let till_minerals_depleted = till_depleted_fn(
                        self.plant_nutrition.minerals,
                        self.nutrition_per_tick.minerals,
                    );
                    let till_water_depleted =
                        till_depleted_fn(self.plant_nutrition.water, self.nutrition_per_tick.water);

                    let till_depleted = [
                        till_sunlight_depleted,
                        till_air_depleted,
                        till_minerals_depleted,
                        till_water_depleted,
                    ]
                    .into_iter()
                    .min()
                    .unwrap();

                    if till_depleted == 0 {
                        // We are already depleted (or one tick until depleted)
                        let energy_per_tick = [
                            self.nutrition_per_tick.sunlight,
                            self.nutrition_per_tick.air,
                            self.nutrition_per_tick.minerals,
                            self.nutrition_per_tick.water,
                            self.nutrition_per_tick.energy,
                        ]
                        .into_iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap()
                            - self.total_passive_cost;

                        let energy_next_tick = [
                            self.plant_nutrition.sunlight,
                            self.plant_nutrition.air,
                            self.plant_nutrition.minerals,
                            self.plant_nutrition.water,
                            self.nutrition_per_tick.energy,
                        ]
                        .into_iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();

                        if energy_per_tick <= 0. {
                            // There will never be enough energy for next growth
                            self.update_plant_nutritions(ticks);
                            self.ticks += ticks;
                        } else {
                            let till_enough_energy = ((required_energy - energy_next_tick)
                                / energy_per_tick)
                                .ceil() as u32;
                            if till_enough_energy > ticks {
                                // Will not have enough ticks
                                self.update_plant_nutritions(ticks);
                                self.ticks += ticks;
                            } else {
                                // There will be enough energy eventually
                                assert!(till_enough_energy > 0);
                                self.update_plant_nutritions(till_enough_energy - 1);
                                self.ticks += till_enough_energy - 1;
                                let cell_count = self.cells_pos.len();
                                self.tick(use_local_growth_recalculation);
                                assert!(self.cells_pos.len() > cell_count);
                                self.tick_many(
                                    ticks - till_enough_energy,
                                    use_local_growth_recalculation,
                                );
                            }
                        }
                    } else if till_enough_energy > till_depleted {
                        // Not enough energy will be produced for the next growth
                        if till_depleted >= ticks {
                            // Don't have enough time to collect energy
                            self.update_plant_nutritions(ticks);
                            self.ticks += ticks;
                        } else {
                            // We deplete before can grow anything, but there's still ticks left
                            self.update_plant_nutritions(till_depleted);
                            self.ticks += till_depleted;
                            self.tick_many(ticks - till_depleted, use_local_growth_recalculation);
                        }
                    } else {
                        // There will be enough energy eventually
                        assert!(till_enough_energy > 0);

                        self.update_plant_nutritions(till_enough_energy - 1);
                        self.ticks += till_enough_energy - 1;

                        // We should need 1 more tick
                        assert!(
                            self.plant_nutrition.energy
                                < self.evolution_data.cells_abilities[self.next_cell_growth.3]
                                    .grow_cost
                        );
                        assert!(
                            self.plant_nutrition.energy + energy_per_tick
                                >= self.evolution_data.cells_abilities[self.next_cell_growth.3]
                                    .grow_cost
                        );

                        let old_nutrition = self.plant_nutrition.clone();
                        let old_nutrition_per_tick = self.nutrition_per_tick.clone();

                        let cell_count = self.cells_pos.len();
                        self.tick(use_local_growth_recalculation);
                        if self.cells_pos.len() <= cell_count {
                            println!("old_nutrition = {:?}", old_nutrition);
                            println!("old_nutrition_per_tick = {:?}", old_nutrition_per_tick);

                            println!("nutrition = {:?}", self.plant_nutrition);
                            println!("nutrition per tick = {:?}", self.nutrition_per_tick);
                            println!(
                                "need energy = {:?}",
                                self.evolution_data.cells_abilities[self.next_cell_growth.3]
                                    .grow_cost
                            );

                            println!("required_energy = {required_energy}");
                            println!("energy_per_tick = {energy_per_tick}");
                            println!("till_enough_energy = {till_enough_energy}");
                            println!("till_depleted = {till_depleted}");
                            println!("till_sunlight_depleted = {till_sunlight_depleted}");
                            println!("till_air_depleted = {till_air_depleted}");
                            println!("till_minerals_depleted = {till_minerals_depleted}");
                            println!("till_water_depleted = {till_water_depleted}");
                        }
                        assert!(self.cells_pos.len() > cell_count);
                        self.tick_many(ticks - till_enough_energy, use_local_growth_recalculation);
                    }
                }
            }
        }
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
                    if x.abs_diff(x2) + y.abs_diff(y2) < SEEDS_MIN_DISTANCE {
                        cnt += 1;
                    }
                }
            }
            seeds_score += 1. / (cnt + 1) as f32;
        }

        (seeds_score * SEED_SCORE)
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
                * SCORE_NUTRITION_MULTIPLIER)
                .sqrt()
    }
}

impl RandomEvolution for MapData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        self.evolution_data
            .evolve_random(rng, change_chance, change_entropy)
    }
}

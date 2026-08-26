use std::{collections::{HashMap, HashSet}, sync::LazyLock};

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

/// One growable target cell with its summed growth scores per cell type.
///
/// Sparse: only empty cells reachable from plant cells are stored. `max_score`
/// is the largest score in `scores`, precomputed to speed up the max scan.
#[derive(Debug, Clone)]
pub struct NextCellGrowthEntry {
    pub x: usize,
    pub y: usize,
    pub scores: [f32; NUMBER_OF_CELLS],
    pub max_score: f32,
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

    /// Sparse, contiguous list of growable target cells (empty cells reachable
    /// from plant cells) with their summed growth scores. Order is unspecified;
    /// the max scan iterates it linearly.
    pub all_next_cell_growth: Vec<NextCellGrowthEntry>,
    /// Active cells with their environment input.
    pub cells_pos: Vec<PlantCellPos>,
    /// Packed position (`y * MAP_SIZE.0 + x`) -> slot in `cells_pos`, for O(1)
    /// input lookup when iterating cells by position.
    pub cell_slots: HashMap<usize, usize>,
    /// Sunlight per air cell (`y * MAP_SIZE.0 + x`, only rows below `GROUND_LEVEL` are stored).
    pub sunlight: Vec<f32>,
    /// Minerals per soil row (indexed by `y - GROUND_LEVEL`).
    pub soil_minerals: Vec<f32>,
    /// Water per soil row (indexed by `y - GROUND_LEVEL`).
    pub soil_water: Vec<f32>,
    /// Dense cell-type grid (`y * MAP_SIZE.0 + x`), `u8::MAX` = empty cell.
    pub cells: Vec<u8>,
}

impl Default for MapData {
    fn default() -> Self {
        let basic_terrain = LazyLock::force(&BASIC_TERRAIN);
        let mut s = Self {
            evolution_data: PlantEvolutionData::default(),
            next_cell_growth: (f32::NEG_INFINITY, 0, 0, 0),
            next_cell_suicide: (f32::NEG_INFINITY, 0, 0),
            ticks: 0,
            plant_nutrition: PlantNutrition::STARTING,
            total_passive_cost: 0.,
            nutrition_per_tick: PlantNutrition::default(),
            all_next_cell_growth: Vec::new(),
            cells_pos: vec![PlantCellPos::new(PLANT_CENTER.0, PLANT_CENTER.1)],
            cell_slots: HashMap::from([(PLANT_CENTER.1 * MAP_SIZE.0 + PLANT_CENTER.0, 0)]),
            sunlight: basic_terrain.sunlight.clone(),
            soil_minerals: basic_terrain.soil_minerals.clone(),
            soil_water: basic_terrain.soil_water.clone(),
            cells: Self::basic_cells(),
        };
        s.populate_plant_inputs();
        s.recalc_plant_nutrition();
        s.recalc_all_next_cell_growth();
        s.recalc_next_cell_suicide();
        s
    }
}

impl MapData {
    #[inline]
    pub fn cell_t(&self, x: usize, y: usize) -> u8 {
        self.cells[y * MAP_SIZE.0 + x]
    }

    #[inline]
    pub fn cell_is_some(&self, x: usize, y: usize) -> bool {
        self.cell_t(x, y) != u8::MAX
    }

    #[inline]
    pub fn cell_is_none(&self, x: usize, y: usize) -> bool {
        !self.cell_is_some(x, y)
    }

    #[inline]
    fn set_cell_t(&mut self, x: usize, y: usize, t: u8) {
        self.cells[y * MAP_SIZE.0 + x] = t;
    }

    /// Input of an active cell at `(x, y)`. Linear scan of `cells_pos`;
    /// intended for UI-only lookups, not hot paths.
    pub fn cell_input(&self, x: usize, y: usize) -> Option<&PlantCellInput> {
        self.cells_pos
            .iter()
            .find(|pos| pos.x == x && pos.y == y)
            .map(|pos| &pos.input)
    }

    /// Terrain view at `(x, y)` for UI. The terrain itself is static
    /// (air above `GROUND_LEVEL`, soil below); only sunlight changes.
    pub fn map_cell(&self, x: usize, y: usize) -> MapCell {
        if y < GROUND_LEVEL {
            MapCell::Air(AirParameters {
                sunlight: self.sunlight[y * MAP_SIZE.0 + x],
            })
        } else {
            MapCell::Soil(SoilParameters {
                minerals: self.soil_minerals[y - GROUND_LEVEL],
                water: self.soil_water[y - GROUND_LEVEL],
            })
        }
    }

    #[hotpath::measure]
    fn update_sunlight(&mut self, x: usize, y: usize) {
        let mut sunlight = if y == 0 {
            1.
        } else if y - 1 < GROUND_LEVEL {
            self.sunlight[(y - 1) * MAP_SIZE.0 + x] * 0.3
        } else {
            0.
        };

        for i in y + 1..GROUND_LEVEL {
            if sunlight < 0.001 {
                break;
            }
            self.sunlight[i * MAP_SIZE.0 + x] = sunlight;
            if self.cell_is_some(x, i) {
                sunlight *= SUNLIGHT_CELL_MULTIPLIER;
            } else {
                sunlight *= SUNLIGHT_AIR_MULTIPLIER;
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
        if y < GROUND_LEVEL {
            for &(nx, ny, distance) in dxdy {
                if ny < GROUND_LEVEL {
                    if self.cell_is_none(nx, ny) {
                        air += distance * AIR_AIR_MULTIPLIER;
                    } else {
                        air += distance * AIR_CELL_MULTIPLIER;
                    }
                }
            }
        } else {
            for &(nx, ny, distance) in dxdy {
                if ny >= GROUND_LEVEL && self.cell_is_none(nx, ny) {
                    minerals += self.soil_minerals[ny - GROUND_LEVEL] * distance;
                    water += self.soil_water[ny - GROUND_LEVEL] * distance;
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
        if x > 0 && self.cell_is_some(x - 1, y) {
            proximity_data[xidx][self.cell_t(x - 1, y) as usize] = true;
        }
        if x + 1 < MAP_SIZE.0 && self.cell_is_some(x + 1, y) {
            proximity_data[1 - xidx][self.cell_t(x + 1, y) as usize] = true;
        }
        if y > 0 && self.cell_is_some(x, y - 1) {
            proximity_data[2][self.cell_t(x, y - 1) as usize] = true;
        }
        if y + 1 < MAP_SIZE.1 && self.cell_is_some(x, y + 1) {
            proximity_data[3][self.cell_t(x, y + 1) as usize] = true;
        }
        proximity_data
    }
}

impl MapData {
    #[hotpath::measure]
    fn populate_plant_inputs(&mut self) {
        for idx in 0..self.cells_pos.len() {
            let (j, i) = {
                let pos = &self.cells_pos[idx];
                (pos.x, pos.y)
            };
            let (air, minerals, water) = self.calc_nutrition(j, i);
            let cells_proximity_data = self.calc_cells_proximity_data(j, i);
            let sunlight = if i < GROUND_LEVEL {
                self.sunlight[i * MAP_SIZE.0 + j]
            } else {
                0.
            };
            self.cells_pos[idx].input = PlantCellInput {
                sunlight,
                air,
                minerals,
                water,
                cells_proximity_data,
            };
        }
    }

    #[hotpath::measure]
    fn recalc_plant_nutrition(&mut self) {
        self.total_passive_cost = 0.;
        let mut nutrition = PlantNutrition::default();
        for pos in &self.cells_pos {
            let t = self.cells[pos.y * MAP_SIZE.0 + pos.x];
            let abilities = &self.evolution_data.cells_abilities[t as usize];
            self.total_passive_cost += abilities.passive_cost;
            nutrition = PlantNutrition {
                sunlight: nutrition.sunlight
                    + pos.input.sunlight * *abilities.sunlight_consumption,
                air: nutrition.air + pos.input.air * *abilities.air_consumption,
                minerals: nutrition.minerals
                    + pos.input.minerals * *abilities.minerals_consumption,
                water: nutrition.water + pos.input.water * *abilities.water_consumption,
                energy: nutrition.energy + *abilities.energy_production_speed,
            };
        }
        self.nutrition_per_tick = nutrition;
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

    // TODO: Measure performance against simpler comparisons
    /// Deterministic ordering for choosing next cell growth.
    /// True if `(w, x, y, c)` is strictly better than `(bw, bx, by, bc)` - it
    /// mirrors the original tuple comparison `(w, -(dist as i32), -(c as i32),
    /// x, y)` exactly: higher weight, then closer to center, then lower cell
    /// type, then higher x, then higher y. Short-circuits on the first key.
    #[inline(always)]
    fn is_better_growth(
        w: f32,
        x: usize,
        y: usize,
        c: usize,
        bw: f32,
        bx: usize,
        by: usize,
        bc: usize,
    ) -> bool {
        if w > bw {
            return true;
        }
        if w < bw {
            return false;
        }
        let d = x.abs_diff(PLANT_CENTER.0) + y.abs_diff(PLANT_CENTER.1);
        let bd = bx.abs_diff(PLANT_CENTER.0) + by.abs_diff(PLANT_CENTER.1);
        if d < bd {
            return true;
        }
        if d > bd {
            return false;
        }
        if c < bc {
            return true;
        }
        if c > bc {
            return false;
        }
        if x > bx {
            return true;
        }
        if x < bx {
            return false;
        }
        y > by
    }

    fn update_next_cell_growth_array(
        from: &PlantCellInput,
        height: f32,
        xdist: f32,
        weights: &[WithVolatility<WeightsTree>; NUMBER_OF_CELLS],
        next_cell_growth: &mut [f32; NUMBER_OF_CELLS],
    ) {
        for c in 0..NUMBER_OF_CELLS {
            let score = weights[c].calculate_safe(from, height, xdist);
            // Can't update self.next_cell_growth here
            // That would not account for negative results later
            next_cell_growth[c] += score;
        }
    }

    fn update_next_cell_growth_from_calc(&mut self) {
        self.next_cell_growth = (f32::NEG_INFINITY, 0, 0, 0);
        for entry in &self.all_next_cell_growth {
            // Skip the whole entry if no cell type's score can beat the current
            // best weight. A score equal to it could still win via the
            // distance/type/x/y tie-breaks, so we require strictly less.
            if entry.max_score < self.next_cell_growth.0 {
                continue;
            }
            for (c, &w) in entry.scores.iter().enumerate() {
                if Self::is_better_growth(
                    w,
                    entry.x,
                    entry.y,
                    c,
                    self.next_cell_growth.0,
                    self.next_cell_growth.1,
                    self.next_cell_growth.2,
                    self.next_cell_growth.3,
                ) {
                    self.next_cell_growth = (w, entry.x, entry.y, c);
                }
            }
        }
    }

    /// Collects growth-score contributions from every plant cell towards its
    /// empty neighbor cells. `recalc_needed` optionally limits targets (local
    /// recalculation): when set, the loop is reversed - for each target in the
    /// (small) set it checks the 4 cardinal neighbor cells for sources, using
    /// the persistent `cell_slots` index for O(1) source input lookup instead
    /// of scanning every plant cell. Contributions are returned unsorted, one
    /// per (cell, direction) pair, packed as `(pos, scores)`.
    fn collect_next_cell_growth_contributions(
        &self,
        recalc_needed: Option<&HashSet<(usize, usize)>>,
    ) -> Vec<(usize, [f32; NUMBER_OF_CELLS])> {
        let Some(needed) = recalc_needed else {
            // Full rebuild: every empty neighbor cell of every plant cell.
            let mut contributions = Vec::with_capacity(self.cells_pos.len() * 4);
            for idx in 0..self.cells_pos.len() {
                let (j, i, t) = {
                    let pos = &self.cells_pos[idx];
                    (pos.x, pos.y, self.cells[pos.y * MAP_SIZE.0 + pos.x])
                };
                let evolution = &self.evolution_data.cells_evolution_data[t as usize];
                // Height/xdist depend only on the source cell, not the direction.
                let height = (1. - i as f32 / MAP_SIZE.1 as f32) * 2. - 1.;
                let xdist = (j as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.);
                for &(nj, ni, d) in &GROWTH_DIRECTION[i][j] {
                    if self.cells[ni * MAP_SIZE.0 + nj] == u8::MAX {
                        let mut scores = [0.; NUMBER_OF_CELLS];
                        Self::update_next_cell_growth_array(
                            &self.cells_pos[idx].input,
                            height,
                            xdist,
                            &evolution.weights[d as usize],
                            &mut scores,
                        );
                        contributions.push((ni * MAP_SIZE.0 + nj, scores));
                    }
                }
            }
            return contributions;
        };

        // Local rebuild: true reversal - iterate the (small) recalc-needed
        // target set and enumerate its sources via the precomputed inverse
        // `GROWTH_SOURCES` (no bounds checks needed).
        let mut contributions = Vec::with_capacity(needed.len() * 4);
        for &(nj, ni) in needed {
            if self.cells[ni * MAP_SIZE.0 + nj] != u8::MAX {
                continue; // occupied cells are not growth targets
            }
            for &(sx, sy, d) in &GROWTH_SOURCES[ni][nj] {
                self.push_source_contribution(sx, sy, d, nj, ni, &mut contributions);
            }
        }
        contributions
    }

    /// Pushes the growth contribution from source cell `(sx, sy)` into target
    /// `(tx, ty)` using the precomputed direction `d`. No-op if the source is
    /// not occupied. The source `input` is resolved in O(1) via `cell_slots`.
    fn push_source_contribution(
        &self,
        sx: usize,
        sy: usize,
        d: GrowthDirection,
        tx: usize,
        ty: usize,
        contributions: &mut Vec<(usize, [f32; NUMBER_OF_CELLS])>,
    ) {
        let source_packed = sy * MAP_SIZE.0 + sx;
        let t = self.cells[source_packed];
        if t == u8::MAX {
            return;
        }
        let slot = self.cell_slots[&source_packed];
        let pos = &self.cells_pos[slot];
        let evolution = &self.evolution_data.cells_evolution_data[t as usize];
        let mut scores = [0.; NUMBER_OF_CELLS];
        Self::update_next_cell_growth_array(
            &pos.input,
            (1. - sy as f32 / MAP_SIZE.1 as f32) * 2. - 1.,
            (sx as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.),
            &evolution.weights[d as usize],
            &mut scores,
        );
        contributions.push((ty * MAP_SIZE.0 + tx, scores));
    }

    /// Sorts contributions by position and merges duplicates in place
    /// (summing scores).
    fn merge_next_cell_growth_contributions(
        contributions: Vec<(usize, [f32; NUMBER_OF_CELLS])>,
    ) -> Vec<NextCellGrowthEntry> {
        let mut contributions = contributions;
        contributions.sort_unstable_by_key(|&(pos, _)| pos);

        // Compact duplicates in place: adjacent equal positions are summed
        // into the first occurrence.
        let mut write = 0usize;
        for read in 0..contributions.len() {
            let (pos, scores) = contributions[read];
            if write == 0 || contributions[write - 1].0 != pos {
                contributions[write] = (pos, scores);
                write += 1;
            } else {
                let target = &mut contributions[write - 1].1;
                for c in 0..NUMBER_OF_CELLS {
                    target[c] += scores[c];
                }
            }
        }
        contributions.truncate(write);

        contributions
            .into_iter()
            .map(|(pos, scores)| NextCellGrowthEntry {
                x: pos % MAP_SIZE.0,
                y: pos / MAP_SIZE.0,
                scores,
                max_score: scores.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            })
            .collect()
    }

    #[hotpath::measure]
    fn recalc_all_next_cell_growth(&mut self) {
        let contributions = self.collect_next_cell_growth_contributions(None);
        self.all_next_cell_growth = Self::merge_next_cell_growth_contributions(contributions);
        self.update_next_cell_growth_from_calc();
    }

    /// Assumes new cells has grown at (x, y)
    fn recalc_next_cell_growth(&mut self, x: usize, y: usize) {
        let recalc_needed = &GROWTH_RECALC_NEEDED_FOR[y][x];

        // Drop the grown cell and every target that needs recalculation; their
        // scores are rebuilt from scratch below.
        self.all_next_cell_growth.retain(|entry| {
            !((entry.x == x && entry.y == y) || recalc_needed.contains(&(entry.x, entry.y)))
        });

        let contributions = self.collect_next_cell_growth_contributions(Some(recalc_needed));
        self.all_next_cell_growth
            .extend(Self::merge_next_cell_growth_contributions(contributions));
        self.update_next_cell_growth_from_calc();
    }

    #[hotpath::measure]
    fn recalc_next_cell_suicide(&mut self) {
        self.next_cell_suicide = (f32::NEG_INFINITY, 0, 0);
        for pos in &self.cells_pos {
            let (j, i) = (pos.x, pos.y);
            if j != PLANT_CENTER.0 || i != PLANT_CENTER.1 {
                let t = self.cells[pos.y * MAP_SIZE.0 + pos.x];
                let evolution = &self.evolution_data.cells_evolution_data[t as usize];
                let score = evolution.calc_suicide(
                    &pos.input,
                    (1. - i as f32 / MAP_SIZE.1 as f32) * 2. - 1.,
                    (j as f32 - PLANT_CENTER.0 as f32).abs() / (MAP_SIZE.0 as f32 / 2.),
                );
                if score > self.next_cell_suicide.0 {
                    self.next_cell_suicide = (score, j, i);
                }
            }
        }
    }

    #[hotpath::measure]
    fn search_cells(&self, x: usize, y: usize, ex_plants: &mut [[bool; MAP_SIZE.0]; MAP_SIZE.1]) {
        ex_plants[y][x] = true;
        if x > 0 && !ex_plants[y][x - 1] && self.cell_is_some(x - 1, y) {
            self.search_cells(x - 1, y, ex_plants);
        }
        if x + 1 < MAP_SIZE.0 && !ex_plants[y][x + 1] && self.cell_is_some(x + 1, y) {
            self.search_cells(x + 1, y, ex_plants);
        }
        if y > 0 && !ex_plants[y - 1][x] && self.cell_is_some(x, y - 1) {
            self.search_cells(x, y - 1, ex_plants);
        }
        if y + 1 < MAP_SIZE.1 && !ex_plants[y + 1][x] && self.cell_is_some(x, y + 1) {
            self.search_cells(x, y + 1, ex_plants);
        }
    }

    #[hotpath::measure]
    pub fn remove_cell(&mut self, x: usize, y: usize) {
        self.set_cell_t(x, y, u8::MAX);
        let mut ex_plants = [[false; MAP_SIZE.0]; MAP_SIZE.1];
        self.search_cells(PLANT_CENTER.0, PLANT_CENTER.1, &mut ex_plants);

        let old_cells_pos = std::mem::take(&mut self.cells_pos);
        let (kept, removed): (Vec<_>, Vec<_>) =
            old_cells_pos.into_iter().partition(|pos| ex_plants[pos.y][pos.x]);
        for pos in &removed {
            self.set_cell_t(pos.x, pos.y, u8::MAX);
        }
        self.cells_pos = kept;
        self.cell_slots.clear();
        for (slot, pos) in self.cells_pos.iter().enumerate() {
            self.cell_slots.insert(pos.y * MAP_SIZE.0 + pos.x, slot);
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
        self.set_cell_t(x, y, cell_type as u8);
        self.cells_pos.push(PlantCellPos::new(x, y));
        self.cell_slots
            .insert(y * MAP_SIZE.0 + x, self.cells_pos.len() - 1);
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

/// Number of air cells in the terrain (`GROUND_LEVEL` rows).
const AIR_CELLS: usize = GROUND_LEVEL * MAP_SIZE.0;
/// Number of soil rows in the terrain.
const SOIL_ROWS: usize = MAP_SIZE.1 - GROUND_LEVEL;

/// Static terrain in SoA form. Terrain never changes during a simulation:
/// air rows (0..`GROUND_LEVEL`) carry sunlight, soil rows carry minerals/water
/// that are constant per row (a function of depth only).
#[derive(Clone)]
struct BasicTerrain {
    sunlight: Vec<f32>,
    soil_minerals: Vec<f32>,
    soil_water: Vec<f32>,
}

impl BasicTerrain {
    fn generate() -> Self {
        let mut sunlight = vec![0.; AIR_CELLS];
        let mut soil_minerals = vec![0.; SOIL_ROWS];
        let mut soil_water = vec![0.; SOIL_ROWS];
        let mut row_sunlight = 1.;
        for i in 0..GROUND_LEVEL {
            row_sunlight *= SUNLIGHT_AIR_MULTIPLIER;
            sunlight[i * MAP_SIZE.0..(i + 1) * MAP_SIZE.0].fill(row_sunlight);
        }
        for i in GROUND_LEVEL..MAP_SIZE.1 {
            let depth = (i - GROUND_LEVEL) as f32 / SOIL_ROWS as f32;
            soil_minerals[i - GROUND_LEVEL] = LOW_DEPTH_MINERALS
                + (HIGH_DEPTH_MINERALS - LOW_DEPTH_MINERALS).abs() * depth;
            soil_water[i - GROUND_LEVEL] =
                HIGH_DEPTH_WATER + (HIGH_DEPTH_WATER - LOW_DEPTH_WATER).abs() * (1. - depth);
        }
        Self {
            sunlight,
            soil_minerals,
            soil_water,
        }
    }
}

static BASIC_TERRAIN: LazyLock<BasicTerrain> = LazyLock::new(BasicTerrain::generate);

impl MapData {
    fn fill_as_basic_map(&mut self) {
        let basic = LazyLock::force(&BASIC_TERRAIN);
        self.sunlight.copy_from_slice(&basic.sunlight);
        self.soil_minerals.copy_from_slice(&basic.soil_minerals);
        self.soil_water.copy_from_slice(&basic.soil_water);
    }

    fn basic_cells() -> Vec<u8> {
        let mut cells = vec![u8::MAX; MAP_SIZE.0 * MAP_SIZE.1];
        cells[PLANT_CENTER.1 * MAP_SIZE.0 + PLANT_CENTER.0] = 0;
        cells
    }

    pub fn generate(rng: &mut Rng) -> Self {
        let mut s = Self {
            evolution_data: PlantEvolutionData::generate(rng),
            ..Default::default()
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

            self.cells.fill(u8::MAX);
            self.cells[PLANT_CENTER.1 * MAP_SIZE.0 + PLANT_CENTER.0] = 0;
            self.fill_as_basic_map();
        });
        self.cells_pos.clear();
        self.cells_pos
            .push(PlantCellPos::new(PLANT_CENTER.0, PLANT_CENTER.1));
        self.cell_slots.clear();
        self.cell_slots
            .insert(PLANT_CENTER.1 * MAP_SIZE.0 + PLANT_CENTER.0, 0);

        self.populate_plant_inputs();
        self.recalc_plant_nutrition();
        self.recalc_all_next_cell_growth();
        self.recalc_next_cell_suicide();
    }

    pub fn tick(&mut self, use_local_growth_recalculation: bool) {
        self.update_plant_nutritions(1);
        self.grow_plant(use_local_growth_recalculation);
        self.ticks += 1;
    }

    #[hotpath::measure]
    pub fn calculate_score(&self) -> f32 {
        let mut seeds = vec![];

        self.cells_pos.iter().for_each(|pos| {
            let (j, i) = (pos.x, pos.y);
            let abilities = &self.evolution_data.cells_abilities[self.cell_t(j, i) as usize];
            if abilities.seed && i < GROUND_LEVEL {
                seeds.push((j, i));
            }
        });

        let mut seeds_score: f32 = 0.;
        for &(x, y) in &seeds {
            let mut cnt = 0;
            for &(x2, y2) in &seeds {
                if (x != x2 || y != y2) && x.abs_diff(x2) + y.abs_diff(y2) < SEEDS_MIN_DISTANCE {
                    cnt += 1;
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

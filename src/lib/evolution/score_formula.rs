use formula::Formula;

use crate::{evolution::consts::{SCORE_NUTRITION_MULTIPLIER, SEED_SCORE, SEEDS_MIN_DISTANCE}, map::{MapData, PlantNutrition}, precalc::GROUND_LEVEL};

/*

    Current:
        seed_result = DEFAULT (seed_distance = 5)
        score = seed_result + sqrt(lowest_nutrition_per_tick)

*/

pub enum NutritionId {
    Sunlight,
    Air,
    Minerals,
    Water,
    Energy,
}

pub enum MapInputId {
    CellsAmount,
    Nutrition(NutritionId),
    NutritionPerTick(NutritionId),
    PassiveCost,
    LowestNutrition,
    LowestNutritionPerTick,
    SeedResult,
}

pub struct MapInput<'a> {
    cells_amount: usize,
    nutrition: &'a PlantNutrition,
    nutrition_per_tick: &'a PlantNutrition,
    passive_cost: f32,
    lowest_nutrition: f32,
    lowest_nutrition_per_tick: f32,
    seed_score: f32,
}

pub enum SeedInputId {
    Amount,
}

pub struct SeedInput {
    amount: usize,
}

pub enum SeedFormula {
    Default { seed_distance: usize },
    Custom(Box<dyn Formula<SeedInput>>),
}

impl SeedFormula {
    fn collect_input(map: &MapData) -> SeedInput {
        SeedInput { amount: map.cells_pos.iter().fold(0, |amount, pos| {
            let (j, i) = (pos.x, pos.y);
            let abilities = &map.evolution_data.cells_abilities[map.cell_t(j, i) as usize];
            if abilities.seed && i < GROUND_LEVEL {
                amount + 1
            } else {
                amount
            }
        }) }
    }

    fn calculate_native(map: &MapData, distance: usize) -> f32 {
        let mut seeds = vec![];

        map.cells_pos.iter().for_each(|pos| {
            let (j, i) = (pos.x, pos.y);
            let abilities = &map.evolution_data.cells_abilities[map.cell_t(j, i) as usize];
            if abilities.seed && i < GROUND_LEVEL {
                seeds.push((j, i));
            }
        });

        let mut seeds_score: f32 = 0.;
        for &(x, y) in &seeds {
            let mut cnt = 0;
            for &(x2, y2) in &seeds {
                if (x != x2 || y != y2) && x.abs_diff(x2) + y.abs_diff(y2) < distance {
                    cnt += 1;
                }
            }
            seeds_score += 1. / (cnt + 1) as f32;
        }

        seeds_score * SEED_SCORE
    }

    fn calculate(&self, map: &MapData) -> f32 {
        match self {
            SeedFormula::Default { seed_distance } => Self::calculate_native(map, *seed_distance),
            SeedFormula::Custom(formula) => formula.calculate(&Self::collect_input(map)),
        }
    }
}

pub enum ScoreFormula {
    Native,
    Custom(Box<dyn for<'a> Formula<MapInput<'a>>>),
}

impl ScoreFormula {
    fn calculate_native(input: &MapInput<'_>) -> f32 {
        input.seed_score
            + (input.lowest_nutrition_per_tick
                * SCORE_NUTRITION_MULTIPLIER)
                .sqrt()
    }

    #[inline]
    pub fn calculate(&self, input: &MapInput<'_>) -> f32 {
        match self {
            ScoreFormula::Native => Self::calculate_native(input),
            ScoreFormula::Custom(formula) => formula.calculate(&input),
        }
    }
}

pub struct MapScoreFormula {
    seed_formula: SeedFormula,
    map_formula: ScoreFormula,
}

impl MapScoreFormula {
    fn collect_input<'a>(map: &'a MapData, seed_score: f32) -> MapInput<'a> {
        MapInput {
            cells_amount: map.cells_pos.len(),
            nutrition: &map.plant_nutrition,
            nutrition_per_tick: &map.nutrition_per_tick,
            passive_cost: map.total_passive_cost,
            lowest_nutrition: [
                    map.plant_nutrition.sunlight,
                    map.plant_nutrition.air,
                    map.plant_nutrition.minerals,
                    map.plant_nutrition.water,
                    map.plant_nutrition.energy,
                ]
                .into_iter()
                .reduce(f32::min)
                .unwrap(),
            lowest_nutrition_per_tick: [
                    map.nutrition_per_tick.sunlight,
                    map.nutrition_per_tick.air,
                    map.nutrition_per_tick.minerals,
                    map.nutrition_per_tick.water,
                    map.nutrition_per_tick.energy,
                ]
                .into_iter()
                .reduce(f32::min)
                .unwrap(),
            seed_score,
        }
    }

    pub fn calculate(&self, map: &MapData) -> f32 {
        let seed_score = self.seed_formula.calculate(map);
        let input = Self::collect_input(map, seed_score);
        self.map_formula.calculate(&input)
    }
}


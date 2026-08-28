use std::{cell::OnceCell, sync::Arc};

use formula::{Formula, ParameterId, Parameters};

use crate::{
    evolution::consts::{SCORE_NUTRITION_MULTIPLIER, SEED_SCORE, SEEDS_MIN_DISTANCE}, map::{MapData, PlantNutrition}, precalc::GROUND_LEVEL,
};

/*
    Default:
        seed_result = DEFAULT (seed_distance = 5) * 20
        score = seed_result + sqrt(lowest_nutrition_per_tick * 100)
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NutritionId {
    Sunlight,
    Air,
    Minerals,
    Water,
    Energy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapInputId {
    CellsAmount,
    Nutrition(NutritionId),
    NutritionPerTick(NutritionId),
    PassiveCost,
    LowestNutrition,
    LowestNutritionPerTick,
    SeedScore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeedInputId {
    Amount,
}

impl ParameterId for NutritionId {
    fn get_name(&self) -> String {
        match self {
            NutritionId::Sunlight => "sunlight".to_owned(),
            NutritionId::Air => "air".to_owned(),
            NutritionId::Minerals => "minerals".to_owned(),
            NutritionId::Water => "water".to_owned(),
            NutritionId::Energy => "energy".to_owned(),
        }
    }
}

impl ParameterId for MapInputId {
    fn get_name(&self) -> String {
        match self {
            MapInputId::CellsAmount => "cells_amount".to_owned(),
            MapInputId::Nutrition(nutrition_id) => format!("total_{}", nutrition_id.get_name()),
            MapInputId::NutritionPerTick(nutrition_id) => format!("{}_pt", nutrition_id.get_name()),
            MapInputId::PassiveCost => "passive_cost".to_owned(),
            MapInputId::LowestNutrition => "lowest_nutrition".to_owned(),
            MapInputId::LowestNutritionPerTick => "lowest_nutrition_pt".to_owned(),
            MapInputId::SeedScore => "seeds_score".to_owned(),
        }
    }
}

impl ParameterId for SeedInputId {
    fn get_name(&self) -> String {
        match self {
            SeedInputId::Amount => "amount".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapInput<'a> {
    map: &'a MapData,
    lowest_nutrition: OnceCell<f32>,
    lowest_nutrition_per_tick: f32,
    seed_score: f32,
}

#[derive(Debug, Clone)]
pub struct SeedInput {
    amount: usize,
}

impl Parameters<NutritionId> for PlantNutrition {
    fn get_value(&self, id: &NutritionId) -> f32 {
        match id {
            NutritionId::Sunlight => self.sunlight,
            NutritionId::Air => self.air,
            NutritionId::Minerals => self.minerals,
            NutritionId::Water => self.water,
            NutritionId::Energy => self.energy,
        }
    }
}

fn get_lowest_nutrition(nutrition: &PlantNutrition) -> f32 {
    [
        nutrition.sunlight,
        nutrition.air,
        nutrition.minerals,
        nutrition.water,
        nutrition.energy,
    ]
    .into_iter()
    .reduce(f32::min)
    .unwrap()
}

impl Parameters<MapInputId> for MapInput<'_> {
    fn get_value(&self, id: &MapInputId) -> f32 {
        match id {
            MapInputId::CellsAmount => self.map.cells_pos.len() as f32,
            MapInputId::Nutrition(nutrition_id) => self.map.plant_nutrition.get_value(nutrition_id),
            MapInputId::NutritionPerTick(nutrition_id) => self.map.nutrition_per_tick.get_value(nutrition_id),
            MapInputId::PassiveCost => self.map.total_passive_cost,
            MapInputId::LowestNutrition => *self.lowest_nutrition.get_or_init(|| get_lowest_nutrition(&self.map.plant_nutrition)),
            MapInputId::LowestNutritionPerTick => self.lowest_nutrition_per_tick,
            MapInputId::SeedScore => self.seed_score,
        }
    }
}

impl Parameters<SeedInputId> for SeedInput {
    fn get_value(&self, id: &SeedInputId) -> f32 {
        match id {
            SeedInputId::Amount => self.amount as f32,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SeedFormula {
    Default { seed_distance: usize, multiplier: f32 },
    Custom(Arc<dyn Formula<SeedInput> + Send + Sync>),
}

impl Default for SeedFormula {
    fn default() -> Self {
        Self::Default { seed_distance: SEEDS_MIN_DISTANCE, multiplier: SEED_SCORE }
    }
}

impl SeedFormula {
    fn collect_input(map: &MapData) -> SeedInput {
        SeedInput {
            amount: map.cells_pos.iter().fold(0, |amount, pos| {
                let (j, i) = (pos.x, pos.y);
                let abilities = &map.evolution_data.cells_abilities[map.cell_t(j, i) as usize];
                if abilities.seed && i < GROUND_LEVEL {
                    amount + 1
                } else {
                    amount
                }
            }),
        }
    }

    fn calculate_native(map: &MapData, distance: usize, multiplier: f32) -> f32 {
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
                if x.abs_diff(x2) + y.abs_diff(y2) < distance {
                    cnt += 1;
                }
            }
            seeds_score += 1. / cnt as f32;
        }

        seeds_score * multiplier
    }

    #[inline]
    fn calculate(&self, map: &MapData) -> f32 {
        match self {
            SeedFormula::Default { seed_distance, multiplier } => Self::calculate_native(map, *seed_distance, *multiplier),
            SeedFormula::Custom(formula) => formula.calculate(&Self::collect_input(map)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScoreFormula {
    /// seed_score + sqrt(lowest_nutrition_per_tick * multiplier)
    Default { multiplier: f32 },
    Custom(Arc<dyn for<'a> Formula<MapInput<'a>> + Send + Sync>),
}

impl Default for ScoreFormula {
    fn default() -> Self {
        Self::Default { multiplier: SCORE_NUTRITION_MULTIPLIER }
    }
}

impl ScoreFormula {
    #[inline]
    fn calculate_native(input: &MapInput<'_>, multiplier: f32) -> f32 {
        input.seed_score + (input.lowest_nutrition_per_tick * multiplier).sqrt()
    }

    #[inline]
    fn calculate(&self, input: &MapInput<'_>) -> f32 {
        match self {
            ScoreFormula::Default { multiplier: lowest_nutrition_multiplier } => Self::calculate_native(input, *lowest_nutrition_multiplier),
            ScoreFormula::Custom(formula) => formula.calculate(input),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MapScoreFormula {
    seed_formula: SeedFormula,
    map_formula: ScoreFormula,
}

impl MapScoreFormula {
    fn collect_input<'a>(map: &'a MapData, seed_score: f32) -> MapInput<'a> {
        MapInput {
            map,
            lowest_nutrition: OnceCell::new(),
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

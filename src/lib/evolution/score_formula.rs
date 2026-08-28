use std::{cell::OnceCell, hash::Hash};

use formula::{
    Formula, Nodes, ParameterId, ParameterIdAll, Parameters, TabulonFormula, TreeArrayFormula, TreeFormula, format::{FormulaFormatter, FullOpFormatter},
};

use crate::{
    evolution::consts::{SCORE_NUTRITION_MULTIPLIER, SEED_SCORE, SEEDS_MIN_DISTANCE},
    map::{MapData, PlantNutrition},
    precalc::GROUND_LEVEL,
};

/*
    Default:
        seed_result = DEFAULT (seed_distance = 5) * 20
        score = seed_result + sqrt(lowest_nutrition_per_tick * 100)
*/

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NutritionId {
    Sunlight,
    Air,
    Minerals,
    Water,
    Energy,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapInputId {
    SeedsScore(usize), // 1..=10
    CellsAmount,
    Nutrition(NutritionId),
    NutritionPerTick(NutritionId),
    PassiveCost,
    LowestNutrition,
    LowestNutritionPerTick,
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

impl ParameterIdAll for NutritionId {
    fn get_all() -> impl Iterator<Item = (String, Self)> {
        [
            NutritionId::Sunlight, NutritionId::Air, NutritionId::Minerals, NutritionId::Water, NutritionId::Energy
        ].into_iter().map(|id| (id.get_name(), id))
    }
}

impl ParameterId for MapInputId {
    fn get_name(&self) -> String {
        match self {
            MapInputId::SeedsScore(distance) => format!("seeds_score_{}", distance),
            MapInputId::CellsAmount => "cells_amount".to_owned(),
            MapInputId::Nutrition(nutrition_id) => format!("total_{}", nutrition_id.get_name()),
            MapInputId::NutritionPerTick(nutrition_id) => format!("{}_pt", nutrition_id.get_name()),
            MapInputId::PassiveCost => "passive_cost".to_owned(),
            MapInputId::LowestNutrition => "lowest_nutrition".to_owned(),
            MapInputId::LowestNutritionPerTick => "lowest_nutrition_pt".to_owned(),
        }
    }
}

impl ParameterIdAll for MapInputId {
    fn get_all() -> impl Iterator<Item = (String, Self)> {
        [
            MapInputId::PassiveCost,
            MapInputId::LowestNutrition,
            MapInputId::LowestNutritionPerTick,
            MapInputId::CellsAmount,
        ]
        .into_iter()
        .chain((1..=10).map(|distance| MapInputId::SeedsScore(distance)))
        .chain(NutritionId::get_all().map(|(_, id)| MapInputId::Nutrition(id)))
        .chain(NutritionId::get_all().map(|(_, id)| MapInputId::NutritionPerTick(id)))
        .map(|id| (id.get_name(), id))
    }
}

#[derive(Debug)]
pub struct MapInput<'a> {
    map: &'a MapData,
    seeds_score: [OnceCell<f32>; 10], // 1..=10
    lowest_nutrition: OnceCell<f32>,
    lowest_nutrition_per_tick: f32,
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

fn calculate_seeds_score(map: &MapData, distance: usize) -> f32 {
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

    seeds_score
}

impl Parameters<MapInputId> for MapInput<'_> {
    fn get_value(&self, id: &MapInputId) -> f32 {
        match id {
            MapInputId::SeedsScore(distance) => *self.seeds_score[*distance].get_or_init(|| calculate_seeds_score(self.map, distance + 1)),
            MapInputId::CellsAmount => self.map.cells_pos.len() as f32,
            MapInputId::Nutrition(nutrition_id) => self.map.plant_nutrition.get_value(nutrition_id),
            MapInputId::NutritionPerTick(nutrition_id) => {
                self.map.nutrition_per_tick.get_value(nutrition_id)
            }
            MapInputId::PassiveCost => self.map.total_passive_cost,
            MapInputId::LowestNutrition => *self
                .lowest_nutrition.get_or_init(|| get_lowest_nutrition(&self.map.plant_nutrition)),
            MapInputId::LowestNutritionPerTick => self.lowest_nutrition_per_tick,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DefaultScoreParameters {
    seed_distance: usize,
    seed_multiplier: f32,
    nutrition_multiplier: f32,
}

impl Default for DefaultScoreParameters {
    fn default() -> Self {
        Self {
            seed_distance: SEEDS_MIN_DISTANCE,
            seed_multiplier: SEED_SCORE,
            nutrition_multiplier: SCORE_NUTRITION_MULTIPLIER,
        }
    }
}

#[derive(Debug)]
pub enum MapScoreFormula {
    /// seed_score + sqrt(lowest_nutrition_per_tick * multiplier)
    Native(DefaultScoreParameters),
    Custom(Box<dyn for<'a> Formula<MapInput<'a>> + Send>),
}

impl Default for MapScoreFormula {
    fn default() -> Self {
        Self::Native(DefaultScoreParameters::default())
    }
}

impl MapScoreFormula {
    fn collect_input<'a>(map: &'a MapData) -> MapInput<'a> {
        MapInput {
            map,
            seeds_score: std::array::repeat(OnceCell::new()),
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
        }
    }

    pub fn calculate(&self, map: &MapData) -> f32 {
        match self {
            MapScoreFormula::Native(parameters) => {
                calculate_seeds_score(map, parameters.seed_distance) * parameters.seed_multiplier + (get_lowest_nutrition(&map.nutrition_per_tick) * parameters.nutrition_multiplier).sqrt()
            },
            MapScoreFormula::Custom(formula) => formula.calculate(&Self::collect_input(map)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EngineType {
    Tree,
    TreeArray,
    Tabulon,
}

#[derive(Debug, Clone)]
pub enum MapScoreFormulaPrototype {
    Native(DefaultScoreParameters),
    Custom {
        nodes: Nodes<MapInputId>,
        engine: EngineType,
    },
}

impl MapScoreFormulaPrototype {
    pub fn build(&self) -> MapScoreFormula {
        match self {
            MapScoreFormulaPrototype::Native(parameters) => MapScoreFormula::Native(parameters.clone()),
            MapScoreFormulaPrototype::Custom { nodes, engine } => MapScoreFormula::Custom(match engine {
                EngineType::Tree => Box::new(TreeFormula::new(nodes.clone())),
                EngineType::TreeArray => {
                    Box::new(TreeArrayFormula::try_from_nodes_vec(nodes.clone()).unwrap())
                }
                EngineType::Tabulon => Box::new(
                    TabulonFormula::new(FullOpFormatter::format_nodes(&nodes, 0).unwrap()).unwrap(),
                ),
            }),
        }
    }
}

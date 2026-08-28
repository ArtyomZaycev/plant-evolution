/// WIP

use math_formula_egui::nodes_builder::{NodesBuilder, UiParametersList};
use plant_evolution_lib::evolution::{
    MapScoreFormulaPrototype, MapInputId, NutritionId,
    consts::{SCORE_NUTRITION_MULTIPLIER, SEED_SCORE, SEEDS_MIN_DISTANCE},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SeedFormulaType {
    Default,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MapFormulaType {
    Default,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EngineType {
    Best,
    Tree,
    TreeArray,
    Tabulon,
}

pub struct ScoringFormulaBuilder {
    seed_formula_type: SeedFormulaType,
    seed_distance: usize,
    seed_multiplier: f32,
    seed_formula_nodes: NodesBuilder<SeedInputId>,

    map_formula_type: MapFormulaType,
    total_multiplier: f32,
    map_formula_nodes: NodesBuilder<MapInputId>,
}

impl egui::Widget for &mut ScoringFormulaBuilder {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("score_map_type").show_ui(ui, |ui| {
                ui.selectable_value(&mut self.map_formula_type, MapFormulaType::Default, "Default");
                ui.selectable_value(&mut self.map_formula_type, MapFormulaType::Custom, "Custom");
            });

            match self.map_formula_type {
                MapFormulaType::Default => {
                    ui.add(egui::Slider::new(&mut self.total_multiplier, 0_f32..=100_f32));
                },
                MapFormulaType::Custom => {
                    self.map_formula_nodes.show_string_expression(ui);
                    // TODO: Reset button
                },
            }
        });

        if self.map_formula_type == MapFormulaType::Custom {
            self.map_formula_nodes.show_nodes_tree(ui);
        }

        todo!()
    }
}

pub enum ScoringFormulaState {
    Active(ScoringFormulaBuilder),
    Building(ScoringFormulaBuilder),
    Built(MapScoreFormulaPrototype),
}

impl ScoringFormulaState {
    pub fn new(old_builder: &MapScoreFormulaPrototype) -> Self {
        let mut seed_nodes_builder = NodesBuilder::new(
            "seed_formula_nodes",
            UiParametersList::builder(|b| b.value("Number of seeds", SeedInputId::Amount)),
        );
        let (seed_formula_type, seed_distance, seed_multiplier, seed_formula_nodes) =
            match &old_builder.seed_formula_prototype {
                SeedFormulaPrototype::Default {
                    distance,
                    multiplier,
                } => (
                    SeedFormulaType::Default,
                    *distance,
                    *multiplier,
                    seed_nodes_builder,
                ),
                SeedFormulaPrototype::Custom { nodes, engine: _ } => {
                    seed_nodes_builder.set_nodes(nodes);
                    (
                        SeedFormulaType::Custom,
                        SEEDS_MIN_DISTANCE,
                        SEED_SCORE,
                        seed_nodes_builder,
                    )
                }
            };

        let mut score_nodes_builder = NodesBuilder::new(
            "score_formula_nodes",
            UiParametersList::builder(|b| {
                b.value("Number of cells", MapInputId::CellsAmount);
                b.value("Passive energy cost", MapInputId::PassiveCost);
                b.value("Lowest nutrition", MapInputId::LowestNutrition);
                b.value(
                    "Lowest nutrition per tick",
                    MapInputId::LowestNutritionPerTick,
                );
                b.value("Seed formula score", MapInputId::SeedScore);
                b.list("Nutrition".to_owned(), Some("Nutrition".to_owned()), |b| {
                    b.value("Sunlight", MapInputId::Nutrition(NutritionId::Sunlight));
                    b.value("Sunlight", MapInputId::Nutrition(NutritionId::Air));
                    b.value("Sunlight", MapInputId::Nutrition(NutritionId::Minerals));
                    b.value("Sunlight", MapInputId::Nutrition(NutritionId::Water));
                    b.value("Sunlight", MapInputId::Nutrition(NutritionId::Energy));
                });
                b.list(
                    "Nutrition per tick".to_owned(),
                    Some("Nutrition per tick".to_owned()),
                    |b| {
                        b.value(
                            "Sunlight",
                            MapInputId::NutritionPerTick(NutritionId::Sunlight),
                        );
                        b.value("Sunlight", MapInputId::NutritionPerTick(NutritionId::Air));
                        b.value(
                            "Sunlight",
                            MapInputId::NutritionPerTick(NutritionId::Minerals),
                        );
                        b.value("Sunlight", MapInputId::NutritionPerTick(NutritionId::Water));
                        b.value(
                            "Sunlight",
                            MapInputId::NutritionPerTick(NutritionId::Energy),
                        );
                    },
                );
            }),
        );

        let (map_formula_type, total_multiplier, map_formula_nodes) =
            match &old_builder.map_formula_prototype {
                MapScoreFormulaPrototype::Default { multiplier } => {
                    (MapFormulaType::Default, *multiplier, score_nodes_builder)
                }
                MapScoreFormulaPrototype::Custom { nodes, engine: _ } => {
                    score_nodes_builder.set_nodes(&nodes);
                    (
                        MapFormulaType::Custom,
                        SCORE_NUTRITION_MULTIPLIER,
                        score_nodes_builder,
                    )
                }
            };

        Self::Active(ScoringFormulaBuilder {
            seed_formula_type,
            seed_distance,
            seed_multiplier,
            seed_formula_nodes,
            map_formula_type,
            total_multiplier,
            map_formula_nodes,
        })
    }
}

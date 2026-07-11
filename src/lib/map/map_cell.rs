use std::fmt::Display;

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

impl Display for MapCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapCell::Air(air_parameters) => write!(f, "Air; sunlight = {}", air_parameters.sunlight),
            MapCell::Soil(soil_parameters) => write!(f, "Soil; water = {}, minerals = {}", soil_parameters.water, soil_parameters.minerals),
        }
    }
}
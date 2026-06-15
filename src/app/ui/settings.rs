use egui::Color32;


#[derive(Debug, Clone)]
pub struct VisualSettings {
    pub min_cell_size: f32,

    pub plant_color: Color32,
    pub seed_color: Color32,

    pub air_color: Color32,
    pub soil_color: Color32,

    pub hovered_map_border_color: Color32,
    pub highlighted_map_border_color: Color32,

    pub highlight_hovered_cell: bool,
    pub highlight_pointer: bool,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            min_cell_size: 4.,
            plant_color: Color32::GREEN,
            seed_color: Color32::RED,
            air_color: Color32::LIGHT_BLUE,
            soil_color: Color32::YELLOW,
            hovered_map_border_color: Color32::PURPLE,
            highlighted_map_border_color: Color32::BLUE,
            highlight_hovered_cell: true,
            highlight_pointer: true,
        }
    }
}
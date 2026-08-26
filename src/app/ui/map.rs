use egui::{Color32, Pos2, Rect, Vec2};
use plant_evolution_lib::{
    map::{MapCell, MapData},
    precalc::MAP_SIZE,
};

use crate::ui::{consts::*, settings::VisualSettings};

pub fn get_ui_map_size(cell_size: f32) -> Vec2 {
    Vec2::new(MAP_SIZE.0 as f32 * cell_size, MAP_SIZE.1 as f32 * cell_size)
}

pub fn draw_map(
    visual_settings: &VisualSettings,
    painter: &egui::Painter,
    map: &MapData,
    cell_size: f32,
    highlighted_cell: Option<(usize, usize)>,
    canvas_start: Pos2,
) {
    for i in 0..MAP_SIZE.1 {
        for j in 0..MAP_SIZE.0 {
            let rect = Rect::from_min_size(
                canvas_start
                    + Vec2 {
                        x: j as f32 * cell_size,
                        y: i as f32 * cell_size,
                    },
                Vec2 {
                    x: cell_size,
                    y: cell_size,
                },
            );

            let color = if map.cell_is_some(j, i) {
                visual_settings.plant_color
            } else {
                match map.map[i][j] {
                    MapCell::Air(_) => visual_settings.air_color,
                    MapCell::Soil(_) => visual_settings.soil_color,
                }
            };

            let color =
                if visual_settings.highlight_hovered_cell && highlighted_cell == Some((j, i)) {
                    Color32::BROWN
                } else {
                    color
                };
            painter.rect_filled(rect, 0., color);

            if map.cell_is_some(j, i)
                && map.evolution_data.cells_abilities[map.cell_t(j, i) as usize].seed
            {
                painter.circle_filled(
                    canvas_start
                        + Vec2 {
                            x: j as f32 * cell_size + 0.5 * cell_size,
                            y: i as f32 * cell_size + 0.5 * cell_size,
                        },
                    cell_size * SEED_RADIUS_MULTIPLIER,
                    visual_settings.seed_color,
                );
            }
        }
    }
}

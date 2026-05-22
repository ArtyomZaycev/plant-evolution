use crate::map::*;

pub fn calculate_score(map: &MapData) -> f32 {
    map.map.iter().fold(0., |acc, row| {
        row.iter().fold(acc, |acc, cell| match cell {
            MapCell::Air => acc,
            MapCell::Soil(_) => acc,
            MapCell::Plant(plant_cell) => acc + map.cells[plant_cell.t].cost,
        })
    })
}

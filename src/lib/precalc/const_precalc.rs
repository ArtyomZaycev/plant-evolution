use std::{collections::HashSet, sync::LazyLock};

pub const NUMBER_OF_CELLS: usize = 8;

// (X, Y)
pub const MAP_SIZE: (usize, usize) = (81, 81);
// at map[GROUND_LEVEL..][..] will be SoilCell
pub const GROUND_LEVEL: usize = MAP_SIZE.1 / 2 + 1;
pub const PLANT_CENTER: (usize, usize) = (MAP_SIZE.0 / 2, GROUND_LEVEL + 1);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GrowthDirection {
    Up = 0,
    Down = 1,
    Inwards = 2,
    Outwards = 3,
}

pub type DxDy2d = (usize, usize, f32);
pub type GrowthDir = (usize, usize, GrowthDirection);
// Every adjacent cell with distance 1
pub static DXDY1_2D: LazyLock<[[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(|| generate_dxdy(1));
// Every adjacent cell with distance 2
pub static DXDY2_2D: LazyLock<[[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(|| generate_dxdy(2));
// Every adjacent cell with distance 3
pub static DXDY3_2D: LazyLock<[[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(|| generate_dxdy(3));

// Where this cell can grow
// For now it's [up, down, inwards, outwards]
pub static GROWTH_DIRECTION: LazyLock<[[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(generate_growth_direction);

// Inverse of GROWTH_DIRECTION: for each target cell, the source cells (and
// their direction) that can grow into it.
pub static GROWTH_SOURCES: LazyLock<[[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(generate_growth_sources);

pub static GROWTH_RECALC_NEEDED_FOR: LazyLock<[[HashSet<(usize, usize)>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(generate_recalc_needed_for);

pub fn populate_consts() {
    LazyLock::force(&DXDY1_2D);
    LazyLock::force(&DXDY2_2D);
    LazyLock::force(&DXDY3_2D);
    LazyLock::force(&GROWTH_DIRECTION);
    LazyLock::force(&GROWTH_SOURCES);
    LazyLock::force(&GROWTH_RECALC_NEEDED_FOR);
}

fn generate_dxdy(max_distance: i32) -> [[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1] {
    let max_distance_square = max_distance.pow(2);
    let max_distance_square_f32 = max_distance_square as f32;
    core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            let mut dxdy = Vec::new();
            (-max_distance..=max_distance).for_each(|dx: i32| {
                (-max_distance..=max_distance).for_each(|dy: i32| {
                    let distance = dx * dx + dy * dy;
                    if distance > 0 && distance <= max_distance_square {
                        let new_x = (j as i32 + dx).clamp(0, MAP_SIZE.0 as i32 - 1) as usize;
                        let new_y = (i as i32 + dy).clamp(0, MAP_SIZE.1 as i32 - 1) as usize;

                        dxdy.push((
                            new_x,
                            new_y,
                            (max_distance_square_f32 - distance as f32).sqrt(),
                        ));
                    }
                })
            });
            dxdy
        })
    })
}

fn generate_growth_direction() -> [[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1] {
    core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            let mut dirs = Vec::new();
            if i > 0 {
                dirs.push((j, i - 1, GrowthDirection::Down));
            }
            if i + 1 < MAP_SIZE.1 {
                dirs.push((j, i + 1, GrowthDirection::Up));
            }
            if j == PLANT_CENTER.0 {
                dirs.push((j - 1, i, GrowthDirection::Outwards));
                dirs.push((j + 1, i, GrowthDirection::Outwards));
            } else {
                if j < PLANT_CENTER.0 {
                    if j > 0 {
                        dirs.push((j - 1, i, GrowthDirection::Outwards));
                    }
                    dirs.push((j + 1, i, GrowthDirection::Inwards));
                } else {
                    if j + 1 < MAP_SIZE.0 {
                        dirs.push((j + 1, i, GrowthDirection::Outwards));
                    }
                    dirs.push((j - 1, i, GrowthDirection::Inwards));
                }
            }
            dirs
        })
    })
}

fn generate_growth_sources() -> [[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1] {
    // Invert GROWTH_DIRECTION: every (source -> target, dir) becomes a
    // (source, dir) entry under the target.
    let mut sources: [[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1] =
        core::array::from_fn(|_| core::array::from_fn(|_| Vec::new()));
    for i in 0..MAP_SIZE.1 {
        for j in 0..MAP_SIZE.0 {
            for &(tx, ty, d) in &GROWTH_DIRECTION[i][j] {
                sources[ty][tx].push((j, i, d));
            }
        }
    }
    sources
}

fn generate_recalc_needed_for() -> [[HashSet<(usize, usize)>; MAP_SIZE.0]; MAP_SIZE.1] {
    core::array::from_fn(|y| {
        core::array::from_fn(|x| {
            // Where air/water/minerals was updated
            DXDY3_2D[y][x]
                .iter()
                .map(|&(x, y, _)| (x, y))
                // Where sunlight was updated
                .chain((y + 3..GROUND_LEVEL).flat_map(|y| {
                    let mut res = vec![(x, y)];
                    if x > 0 {
                        res.push((x - 1, y));
                    }
                    if x + 1 < MAP_SIZE.0 {
                        res.push((x + 1, y));
                    }
                    res
                }))
                .chain({
                    let mut res = vec![];
                    if y < GROUND_LEVEL {
                        res.push((x, GROUND_LEVEL));
                    }
                    res
                })
                .collect::<HashSet<_>>()
        })
    })
}

use std::sync::LazyLock;

pub const NUMBER_OF_CELLS: usize = 8;

// (X, Y)
pub const MAP_SIZE: (usize, usize) = (81, 81);
// at map[GROUND_LEVEL..][..] will be SoilCell
pub const GROUND_LEVEL: usize = MAP_SIZE.1 / 2 + 1;
pub const PLANT_CENTER: (usize, usize) = (MAP_SIZE.0 / 2, GROUND_LEVEL + 1);

pub type DxDy2d = (usize, usize, f32);
pub type GrowthDir = (usize, usize, usize);
// Every adjacent cell with distance 2
pub static DXDY_2D: LazyLock<[[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(|| generate_dxdy());
// Where this cell can grow
// For now it's [up, down, outwards..]
pub static GROWTH_DIRECTION: LazyLock<[[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1]> =
    LazyLock::new(|| generate_growth_direction());

pub fn populate_consts() {
    LazyLock::force(&DXDY_2D);
    LazyLock::force(&GROWTH_DIRECTION);
}

fn generate_dxdy() -> [[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1] {
    core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            let mut dxdy = Vec::new();
            (-2..=2).for_each(|dx: i32| {
                (-2..=2).for_each(|dy: i32| {
                    let distance = dx * dx + dy * dy;
                    if distance > 0 && distance <= 4 {
                        let new_x = (j as i32 + dx).clamp(0, MAP_SIZE.0 as i32 - 1) as usize;
                        let new_y = (i as i32 + dy).clamp(0, MAP_SIZE.1 as i32 - 1) as usize;

                        dxdy.push((new_x, new_y, (4. - distance as f32).sqrt()));
                    }
                })
            });
            dxdy
        })
    })
}

// TODO: plants should be able to grow towards the center
fn generate_growth_direction() -> [[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1] {
    core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            let mut dirs = Vec::new();
            if i > 0 {
                dirs.push((j, i - 1, 0));
            }
            if i + 1 < MAP_SIZE.1 {
                dirs.push((j, i + 1, 2));
            }
            if j == PLANT_CENTER.0 {
                dirs.push((j - 1, i, 1));
                dirs.push((j + 1, i, 1));
            } else {
                if j < PLANT_CENTER.0 && j > 0 {
                    dirs.push((j - 1, i, 1));
                }
                if j > PLANT_CENTER.0 && j + 1 < MAP_SIZE.0 {
                    dirs.push((j + 1, i, 1));
                }
            }
            dirs
        })
    })
}

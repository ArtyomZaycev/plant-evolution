use std::sync::OnceLock;

pub const NUMBER_OF_CELLS: usize = 8;

// (X, Y)
pub const MAP_SIZE: (usize, usize) = (128, 128);
pub const PLANT_CENTER: (usize, usize) = (MAP_SIZE.0 / 2, MAP_SIZE.1 / 2 + 2);

pub type DxDy2d = (usize, usize, f32);
pub type GrowthDir = (usize, usize, usize);
pub static DXDY_2D: OnceLock<[[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1]> = OnceLock::new();
pub static GROWTH_DIRECTION: OnceLock<[[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1]> = OnceLock::new();

pub fn populate_consts() {
    DXDY_2D.set(generate_dxdy()).unwrap();
    GROWTH_DIRECTION.set(generate_growth_direction()).unwrap();
}

fn generate_dxdy() -> [[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1] {
    core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            let mut dxdy = Vec::new();
            (-2..=2).for_each(|dx: i32| {
                (-2..=2).for_each(|dy: i32| {
                    let distance = dx.abs() + dy.abs();
                    if distance > 0 && distance < 4 {
                        let new_x = (j as i32 + dx).clamp(0, MAP_SIZE.0 as i32 - 1) as usize;
                        let new_y = (i as i32 + dy).clamp(0, MAP_SIZE.1 as i32 - 1) as usize;

                        dxdy.push((new_x, new_y, distance as f32));
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

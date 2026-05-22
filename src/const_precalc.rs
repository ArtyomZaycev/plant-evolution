use std::sync::OnceLock;

pub const NUMBER_OF_CELLS: usize = 8;

// (X, Y)
pub const MAP_SIZE: (usize, usize) = (128, 128);
pub const PLANT_CENTER: (usize, usize) = (MAP_SIZE.0 / 2, MAP_SIZE.1 / 2 + 2);

pub type DxDy_2d = (usize, usize, f32);
pub type DxDyProximity = (usize, usize, f32, f32);
pub static DXDY_2D: OnceLock<[[Vec<DxDy_2d>; MAP_SIZE.0]; MAP_SIZE.1]> = OnceLock::new();
pub static PROXIMITY_DXDY: OnceLock<[[Vec<DxDyProximity>; MAP_SIZE.0]; MAP_SIZE.1]> =
    OnceLock::new();

pub fn populate_consts() {
    use crate::const_precalc::DXDY_2D;

    DXDY_2D.set(generate_dxdy()).unwrap();
    PROXIMITY_DXDY.set(generate_proximity_dxdy()).unwrap();
}

fn generate_dxdy() -> [[Vec<DxDy_2d>; MAP_SIZE.0]; MAP_SIZE.1] {
    core::array::from_fn(|i| {
        core::array::from_fn(|j| {
            let mut dxdy = Vec::new();
            (-2..=2).for_each(|dx: i32| {
                (-2..=2).for_each(|dy: i32| {
                    let distance = dx.abs() + dy.abs();
                    if distance > 0 && distance < 4 {
                        let new_x = j as i32 + dx;
                        let new_x = if new_x < 0 {
                            0
                        } else if new_x >= MAP_SIZE.0 as i32 {
                            MAP_SIZE.0 - 1
                        } else {
                            new_x as usize
                        };

                        let new_y = i as i32 + dy;
                        let new_y = if new_y < 0 {
                            0
                        } else if new_y >= MAP_SIZE.0 as i32 {
                            MAP_SIZE.1 - 1
                        } else {
                            new_y as usize
                        };

                        dxdy.push((new_x, new_y, distance as f32));
                    }
                })
            });
            dxdy
        })
    })
}

fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
}
fn generate_proximity_dxdy() -> [[Vec<DxDyProximity>; MAP_SIZE.0]; MAP_SIZE.1] {
    let cx = PLANT_CENTER.0 as f32;
    let cy = PLANT_CENTER.1 as f32;

    core::array::from_fn(|y| {
        core::array::from_fn(|x| {
            let mut dxdy = Vec::new();

            let px = x as f32;
            let py = y as f32;

            for i in 0..MAP_SIZE.1 {
                for j in 0..MAP_SIZE.0 {
                    if i != y && j != x {
                        let x = j as f32;
                        let y = i as f32;

                        let ab = distance(px, py, x, y);
                        let ac = distance(px, py, cx, cy);
                        let bc = distance(x, y, cx, cy);

                        let angle =
                            ((ab.powi(2) + ac.powi(2) + bc.powi(2)) / (2. * ab * ac)).acos();

                        if angle < std::f32::consts::FRAC_PI_4 {
                            let distance = ab;
                            let line_angle = {
                                let x1 = px;
                                let y1 = py;
                                let x2 = 2. * x1 - cx;
                                let y2 = cy;
                                let d = (y2 - y1) * (x - x1) - (x2 - x1) * (y - y1);
                                if d < 0. {
                                    angle + std::f32::consts::FRAC_PI_4
                                } else {
                                    std::f32::consts::FRAC_PI_4 - angle
                                }
                            };

                            // TODO: Normalize distance & line_angle
                            dxdy.push(((distance, angle), (j, i, distance, line_angle)));
                        }
                    }
                }
            }

            dxdy.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            dxdy.into_iter().map(|v| v.1).collect()
        })
    })
}

use std::sync::OnceLock;

pub const NUMBER_OF_CELLS: usize = 8;

// (X, Y)
pub const MAP_SIZE: (usize, usize) = (128, 128);
pub const PLANT_CENTER: (usize, usize) = (MAP_SIZE.0 / 2, MAP_SIZE.1 / 2 + 2);

pub type DxDy2d = (usize, usize, f32);
pub type DxDyProximity = (usize, usize, f32, f32);
pub type GrowthDir = (usize, usize, usize);
pub static DXDY_2D: OnceLock<[[Vec<DxDy2d>; MAP_SIZE.0]; MAP_SIZE.1]> = OnceLock::new();
pub static PROXIMITY_DXDY: OnceLock<[[Vec<DxDyProximity>; MAP_SIZE.0]; MAP_SIZE.1]> =
    OnceLock::new();
pub static PROXIMITY_DXDY_REV: OnceLock<[[Vec<DxDyProximity>; MAP_SIZE.0]; MAP_SIZE.1]> =
    OnceLock::new();
pub static GROWTH_DIRECTION: OnceLock<[[Vec<GrowthDir>; MAP_SIZE.0]; MAP_SIZE.1]> = OnceLock::new();

pub fn populate_consts() {
    DXDY_2D.set(generate_dxdy()).unwrap();
    PROXIMITY_DXDY.set(generate_proximity_dxdy()).unwrap();
    PROXIMITY_DXDY_REV.set(generate_proximity_dxdy_rev()).unwrap();
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

fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
}

// ln(22)
const MAX_DIFFERENTIATED_DISTANCE: f32 = 3.091042;
fn normalize_distance(d: f32) -> f32 {
    if d > 30. {
        0.
    } else {
        (1. - d.ln() / MAX_DIFFERENTIATED_DISTANCE).max(0.)
    }
}

fn normalize_angle(a: f32) -> f32 {
    a / std::f32::consts::FRAC_PI_2
}

fn generate_proximity_dxdy() -> [[Vec<DxDyProximity>; MAP_SIZE.0]; MAP_SIZE.1] {
    let cx = PLANT_CENTER.0 as f32 + 0.5;
    let cy = PLANT_CENTER.1 as f32 + 0.5;

    core::array::from_fn(|y| {
        core::array::from_fn(|x| {
            let mut dxdy = Vec::new();

            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let ac: f32 = distance(px, py, cx, cy);
            // Only cells within `ac` distance can satisfy `ab <= ac`,
            // so restrict the search to a bounding box of radius `ac` around (x, y).
            let radius = ac.ceil() as usize;
            let y_start = y.saturating_sub(radius);
            let y_end = (y + radius).min(MAP_SIZE.1 - 1);
            let x_start = x.saturating_sub(radius);
            let x_end = (x + radius).min(MAP_SIZE.0 - 1);

            for i in y_start..=y_end {
                for j in x_start..=x_end {
                    if i != y || j != x {
                        let xf = j as f32 + 0.5;
                        let yf = i as f32 + 0.5;

                        let ab = distance(px, py, xf, yf);
                        let bc = distance(xf, yf, cx, cy);

                        let angle =
                            ((ab.powi(2) + ac.powi(2) - bc.powi(2)) / (2. * ab * ac)).acos();

                        if ab <= ac && angle < std::f32::consts::FRAC_PI_4 {
                            let dist = ab;

                            // determine on which side of the line between p and center is point we're checking
                            let line_angle = {
                                let x1 = px;
                                let y1 = py;
                                let x2 = cx;
                                let y2 = cy;
                                let d = (y2 - y1) * (xf - x1) - (x2 - x1) * (yf - y1);

                                if d < 0. {
                                    angle + std::f32::consts::FRAC_PI_4
                                } else {
                                    std::f32::consts::FRAC_PI_4 - angle
                                }
                            };

                            let distance = normalize_distance(dist);
                            if distance > 0. {
                                dxdy.push((
                                    (dist, angle),
                                    (j, i, distance, normalize_angle(line_angle)),
                                ));
                            }
                        }
                    }
                }
            }

            dxdy.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            dxdy.into_iter().map(|v| v.1).collect()
        })
    })
}

fn generate_proximity_dxdy_rev() -> [[Vec<DxDyProximity>; MAP_SIZE.0]; MAP_SIZE.1] {
    let mut arr = core::array::from_fn(|_| {
        core::array::from_fn(|_| {
            vec![]
        })
    });

    let proximity = PROXIMITY_DXDY.get().unwrap();
    proximity.iter().enumerate().for_each(|(i, row)| {
        row.iter().enumerate().for_each(|(j, proximity)| {
            proximity.iter().for_each(|&(x, y, distance, angle)| {
                arr[y][x].push((j, i, distance, angle));
            });
        });
    });

    arr
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
                if j >= PLANT_CENTER.0 && j + 1 < MAP_SIZE.0 {
                    dirs.push((j + 1, i, 1));
                }
            }
            dirs
        })
    })
}

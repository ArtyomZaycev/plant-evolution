use std::sync::LazyLock;

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

/// A 2D table stored as a single contiguous buffer plus per-cell offset
/// ranges. Cell `(x, y)`'s elements are `data[offsets[y*W+x]..offsets[y*W+x+1]]`.
///
/// Replaces `[[Vec<T>; W]; H]` (one heap allocation per cell) with two flat
/// allocations, giving better cache locality and a constant allocation count
/// regardless of `MAP_SIZE`.
pub struct Flattened2D<T> {
    width: usize,
    offsets: Vec<usize>,
    data: Vec<T>,
}

impl<T> Flattened2D<T> {
    /// Flattens `rows` (`rows[y][x]` = the per-cell element list) into
    /// row-major storage.
    fn from_rows(rows: Vec<Vec<Vec<T>>>) -> Self {
        let width = rows.first().map_or(0, |row| row.len());
        let mut offsets = Vec::with_capacity(rows.len() * width + 1);
        offsets.push(0);
        let mut data = Vec::new();
        for row in rows {
            for cell in row {
                data.extend(cell);
                offsets.push(data.len());
            }
        }
        Self {
            width,
            offsets,
            data,
        }
    }

    /// The elements of cell `(x, y)`.
    #[inline]
    pub fn slice(&self, x: usize, y: usize) -> &[T] {
        let idx = y * self.width + x;
        &self.data[self.offsets[idx]..self.offsets[idx + 1]]
    }
}

// Every adjacent cell with distance 1
pub static DXDY1_2D: LazyLock<Flattened2D<DxDy2d>> = LazyLock::new(|| generate_dxdy(1));
// Every adjacent cell with distance 2
pub static DXDY2_2D: LazyLock<Flattened2D<DxDy2d>> = LazyLock::new(|| generate_dxdy(2));
// Every adjacent cell with distance 3
pub static DXDY3_2D: LazyLock<Flattened2D<DxDy2d>> = LazyLock::new(|| generate_dxdy(3));

// Where this cell can grow
// For now it's [up, down, inwards, outwards]
pub static GROWTH_DIRECTION: LazyLock<Flattened2D<GrowthDir>> =
    LazyLock::new(generate_growth_direction);

// Inverse of GROWTH_DIRECTION: for each target cell, the source cells (and
// their direction) that can grow into it.
pub static GROWTH_SOURCES: LazyLock<Flattened2D<GrowthDir>> =
    LazyLock::new(generate_growth_sources);

pub static GROWTH_RECALC_NEEDED_FOR: LazyLock<Flattened2D<(usize, usize)>> =
    LazyLock::new(generate_recalc_needed_for);

pub fn populate_consts() {
    LazyLock::force(&DXDY1_2D);
    LazyLock::force(&DXDY2_2D);
    LazyLock::force(&DXDY3_2D);
    LazyLock::force(&GROWTH_DIRECTION);
    LazyLock::force(&GROWTH_SOURCES);
    LazyLock::force(&GROWTH_RECALC_NEEDED_FOR);
}

fn generate_dxdy(max_distance: i32) -> Flattened2D<DxDy2d> {
    let max_distance_square = max_distance.pow(2);
    let max_distance_square_f32 = max_distance_square as f32;
    let rows: Vec<Vec<Vec<DxDy2d>>> = (0..MAP_SIZE.1)
        .map(|i| {
            (0..MAP_SIZE.0)
                .map(|j| {
                    let mut dxdy = Vec::new();
                    (-max_distance..=max_distance).for_each(|dx: i32| {
                        (-max_distance..=max_distance).for_each(|dy: i32| {
                            let distance = dx * dx + dy * dy;
                            if distance > 0 && distance <= max_distance_square {
                                let new_x =
                                    (j as i32 + dx).clamp(0, MAP_SIZE.0 as i32 - 1) as usize;
                                let new_y =
                                    (i as i32 + dy).clamp(0, MAP_SIZE.1 as i32 - 1) as usize;

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
                .collect()
        })
        .collect();
    Flattened2D::from_rows(rows)
}

fn generate_growth_direction() -> Flattened2D<GrowthDir> {
    let rows: Vec<Vec<Vec<GrowthDir>>> = (0..MAP_SIZE.1)
        .map(|i| {
            (0..MAP_SIZE.0)
                .map(|j| {
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
                .collect()
        })
        .collect();
    Flattened2D::from_rows(rows)
}

fn generate_growth_sources() -> Flattened2D<GrowthDir> {
    // Invert GROWTH_DIRECTION: every (source -> target, dir) becomes a
    // (source, dir) entry under the target.
    let mut rows: Vec<Vec<Vec<GrowthDir>>> = (0..MAP_SIZE.1)
        .map(|_| (0..MAP_SIZE.0).map(|_| Vec::new()).collect())
        .collect();
    for i in 0..MAP_SIZE.1 {
        for j in 0..MAP_SIZE.0 {
            for &(tx, ty, d) in GROWTH_DIRECTION.slice(j, i) {
                rows[ty][tx].push((j, i, d));
            }
        }
    }
    Flattened2D::from_rows(rows)
}

fn generate_recalc_needed_for() -> Flattened2D<(usize, usize)> {
    let rows: Vec<Vec<Vec<(usize, usize)>>> = (0..MAP_SIZE.1)
        .map(|y| {
            (0..MAP_SIZE.0)
                .map(|x| {
                    // Where air/water/minerals was updated
                    let mut set: Vec<(usize, usize)> = DXDY3_2D
                        .slice(x, y)
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
                        .chain((y < GROUND_LEVEL).then(|| (x, GROUND_LEVEL)))
                        .collect();
                    // Sorted + deduped so the slice supports binary search.
                    set.sort_unstable();
                    set.dedup();
                    set
                })
                .collect()
        })
        .collect();
    Flattened2D::from_rows(rows)
}

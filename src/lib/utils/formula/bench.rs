use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use crate::utils::formula::{Formula, ParameterId, Parameters};

pub fn bench_formula<PId: ParameterId, P: Parameters<PId>, F: Formula<P>>(
    formula: F,
    parameters: &P,
    time_limit: Duration,
) -> u128 {
    let start = Instant::now();

    (0..100).for_each(|_| {
        black_box(formula.calculate(parameters));
    });

    let elapsed = start.elapsed();
    let approx_iter = time_limit.div_duration_floor(elapsed) * 100;

    let start = Instant::now();
    (0..approx_iter).for_each(|_| {
        black_box(formula.calculate(parameters));
    });
    let elapsed = start.elapsed();

    (approx_iter as f64 * time_limit.div_duration_f64(elapsed)) as u128
}

#![feature(vec_from_fn)]

use std::{hint::black_box, thread, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use plant_evolution_lib::{evolution::run_evolution_random, map::*, precalc::*, utils::*};

extern crate plant_evolution_lib;

// cargo flamegraph --bench evolution

fn evolution_benchmark() {
    let number_of_plants: usize = 200;
    let mut maps = Vec::from_fn(number_of_plants, |_| MapData::default());
    let mut rng = SmallRng::seed_from_u64(DEFAULT_SEED);
    run_evolution_random(None, &mut maps, &mut rng, 1000, 1000, 0.9, 0.1);
    black_box(maps);
}

fn run_big_stack_thread<F: FnOnce() + Send + 'static>(f: F) {
    let h = thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(f);
    let _ = h.unwrap().join();
}

fn criterion_benchmark(c: &mut Criterion) {
    run_big_stack_thread(|| {
        populate_consts();
    });

    let mut group = c.benchmark_group("evolution-group");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(60));
    group.bench_function("random-evolution", |b| {
        b.iter(|| run_big_stack_thread(evolution_benchmark))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

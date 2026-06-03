use std::{hint::black_box, thread, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use plant_evolution_lib::{map::*, populate_consts, random_evolution::*};

extern crate plant_evolution_lib;

// cargo flamegraph --bench evolution

fn evolution_benchmark() {
    let number_of_plants: usize = 200;
    let mut maps = (0..number_of_plants)
        .map(|_| {
            let (a, b) = get_basic_map_data();
            MapData::generate(a, b)
        })
        .collect::<Vec<_>>();
    run_evolution_random(None, &mut maps, 1000, 1000, 0.9, 0.1);
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

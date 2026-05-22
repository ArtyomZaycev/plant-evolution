use std::{thread, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use plant_evolution_lib::{map::*, populate_consts};

extern crate plant_evolution_lib;

// cargo flamegraph --bench growth

fn growth_benchmark() {
    populate_consts();

    let number_of_plants: usize = 10;
    let mut maps = (0..number_of_plants)
        .map(|_| {
            let (a, b, c) = get_basic_map_data();
            MapData::generate(a, b, c)
        })
        .collect::<Vec<_>>();
    (0..100).for_each(|_| {
        maps.iter_mut().for_each(|map| {
            map.tick();
        });
    });
}

fn run_big_stack_thread<F: FnOnce() + Send + 'static>(f: F) {
    let h = thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(f);
    let _ = h.unwrap().join();
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample-size-example");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(60));
    group.bench_function("growth", |b| {
        b.iter(|| run_big_stack_thread(growth_benchmark))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

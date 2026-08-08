#![feature(vec_from_fn)]

use std::{sync::atomic::Ordering, thread::sleep, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use plant_evolution_lib::{engine::*, map::*, precalc::*};

fn engine_bencmark(engine: &mut Engine, autoevolve_at: u32, total_evolution: u32) {
    engine.send_command(EngineCommand::RunSimulationa(autoevolve_at)).unwrap();
    loop {
        sleep(Duration::from_millis(10));
        if engine.state.total_evolutions.load(Ordering::Relaxed) >= total_evolution {
            //println!("Total evolutions: {}/{}", engine.state.total_evolutions.load(Ordering::Relaxed), total_evolution);
            engine.send_command(EngineCommand::Stop).unwrap();
            engine.send_command(EngineCommand::Restart).unwrap();
            break;
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    populate_consts();

    let maps = Vec::from_fn(200, |_| MapData::default());
    let parameters = EngineParameters {
        saving_parameters: SavingParameters::DISABLED,
        evolution_parameters: EvolutionParameters {
            plants: 200,
            samples: 10,
            parent_evolution: true,
            change_chance: 0.05,
            change_entropy: 0.8,
            run_evolution_parameters: RunEvolutionParameters {
                ticks_per_slow_write: 500,
            },
        },
        performance_parameters: PerformanceParameters::default(),
    };

    let mut engine = Engine::new(maps, parameters);

    let mut group = c.benchmark_group("engine-benchmarks");
    group
        .sampling_mode(criterion::SamplingMode::Flat)
        .sample_size(10)
        .measurement_time(Duration::from_secs(180));

    group.bench_function("accuracy", |b| {
        engine.state.parameters.update_data(|parameters| {
            parameters.performance_parameters = PerformanceParameters::ACCURACY
        });
        b.iter(|| {
            engine_bencmark(&mut engine, 500, 100);
        });
    });

    group.bench_function("performace", |b| {
        engine.state.parameters.update_data(|parameters| {
            parameters.performance_parameters = PerformanceParameters::PERFORMANCE
        });
        b.iter(|| {
            engine_bencmark(&mut engine, 500, 100);
        });
    });

    group.bench_function("ui_performace", |b| {
        engine.state.parameters.update_data(|parameters| {
            parameters.performance_parameters = PerformanceParameters::UI_PERFORMANCE
        });
        b.iter(|| {
            engine_bencmark(&mut engine, 500, 100);
        });
    });

    group.bench_function("accuracy_long", |b| {
        engine.state.parameters.update_data(|parameters| {
            parameters.performance_parameters = PerformanceParameters::ACCURACY
        });
        b.iter(|| {
            engine_bencmark(&mut engine, 500, 2000);
        });
    });

    group.bench_function("performace_long", |b| {
        engine.state.parameters.update_data(|parameters| {
            parameters.performance_parameters = PerformanceParameters::PERFORMANCE
        });
        b.iter(|| {
            engine_bencmark(&mut engine, 500, 2000);
        });
    });

    group.bench_function("ui_performace_long", |b| {
        engine.state.parameters.update_data(|parameters| {
            parameters.performance_parameters = PerformanceParameters::UI_PERFORMANCE
        });
        b.iter(|| {
            engine_bencmark(&mut engine, 500, 2000);
        });
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

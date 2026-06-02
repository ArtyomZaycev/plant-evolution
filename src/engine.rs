use std::sync::{Arc, mpsc};

use crate::{
    evolution::{PlantEvolutionData, sample_maps},
    map::MapData,
    random_evolution::RandomEvolution,
    slow_mutex::SlowMutex,
};

pub enum EngineCommand {
    Restart,
    Tick,
    RunTick,
    StopRunTick,
    Evolve {
        change_change: f32,
        change_entropy: f32,
    },
}

pub fn run_engine(
    receiver: mpsc::Receiver<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,
) {
    let mut maps = slow_maps.force_read();
    let mut run_tick = false;

    loop {
        if let Ok(command) = receiver.try_recv() {
            match command {
                EngineCommand::Restart => {
                    maps.iter_mut().for_each(|map| {
                        map.evolution_data = PlantEvolutionData::generate();
                        map.restart();
                    });
                    slow_maps.force_write(maps.clone());
                }
                EngineCommand::Tick => {
                    maps.iter_mut().for_each(|map| {
                        map.tick();
                    });
                    slow_maps.force_write(maps.clone());
                }
                EngineCommand::RunTick => {
                    run_tick = true;
                }
                EngineCommand::StopRunTick => {
                    run_tick = false;
                    slow_maps.force_write(maps.clone());
                }
                EngineCommand::Evolve {
                    change_change,
                    change_entropy,
                } => {
                    sample_maps(&mut maps);
                    maps.evolve_random(&mut rand::rng(), change_change, change_entropy);
                    slow_maps.force_write(maps.clone());
                }
            }
        }

        if run_tick {
            maps.iter_mut().for_each(|map| {
                map.tick();
            });
            slow_maps.slow_write(&maps);
        }
    }
}

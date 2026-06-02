use std::sync::{Arc, mpsc};

use crate::{
    evolution::{PlantEvolutionData, sample_maps},
    map::MapData,
    random_evolution::RandomEvolution,
    slow_mutex::SlowMutex,
};

#[derive(Debug, Clone, Copy)]
pub struct EvolutionParameters {
    change_chance: f32,
    change_entropy: f32,
}

impl Default for EvolutionParameters {
    fn default() -> Self {
        Self {
            change_chance: 0.6,
            change_entropy: 0.05,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RunEvolutionParameters {
    ticks_per_evolution: u32,
}

impl Default for RunEvolutionParameters {
    fn default() -> Self {
        Self {
            ticks_per_evolution: 500,
        }
    }
}

pub enum EngineCommand {
    Restart,

    Tick,
    RunTick,
    StopRunTick,

    UpdateEvolutionParameters(EvolutionParameters),
    Evolve,

    UpdateRunEvolutionParameters(RunEvolutionParameters),
    RunEvolution,
    StopRunEvolution,
}

enum EngineState {
    Stale,
    RunTick,
    RunEvolution,
}

const ENGINE_RUN_EVOLUTION_TICKS_PER_SLOW_WRITE: u32 = 10;

pub fn run_engine(
    receiver: mpsc::Receiver<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,
) {
    let mut rng = rand::rng();

    let mut maps = slow_maps.force_read();

    let mut evolution_parameters = EvolutionParameters::default();
    let mut run_evolution_parameters = RunEvolutionParameters::default();

    let mut state = EngineState::Stale;

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
                    state = EngineState::RunTick;
                }
                EngineCommand::StopRunTick => {
                    state = EngineState::Stale;
                    slow_maps.force_write(maps.clone());
                }

                EngineCommand::UpdateEvolutionParameters(new_evolution_parameters) => {
                    evolution_parameters = new_evolution_parameters;
                }
                EngineCommand::Evolve => {
                    sample_maps(&mut maps);
                    maps.evolve_random(
                        &mut rand::rng(),
                        evolution_parameters.change_chance,
                        evolution_parameters.change_entropy,
                    );
                    slow_maps.force_write(maps.clone());
                }
                EngineCommand::UpdateRunEvolutionParameters(new_run_evolution_parameters) => {
                    run_evolution_parameters = new_run_evolution_parameters;
                }
                EngineCommand::RunEvolution => {
                    state = EngineState::RunEvolution;
                }
                EngineCommand::StopRunEvolution => {
                    state = EngineState::Stale;
                    slow_maps.force_write(maps.clone());
                }
            }
        }

        match state {
            EngineState::Stale => {}
            EngineState::RunTick => {
                maps.iter_mut().for_each(|map| {
                    map.tick();
                });
                slow_maps.slow_write(&maps);
            }
            EngineState::RunEvolution => {
                (0..(ENGINE_RUN_EVOLUTION_TICKS_PER_SLOW_WRITE.min(
                    run_evolution_parameters
                        .ticks_per_evolution
                        .saturating_sub(maps[0].time),
                )))
                    .for_each(|_| {
                        maps.iter_mut().for_each(|map| map.tick());
                        slow_maps.slow_write(&maps);
                    });
                slow_maps.slow_write(&maps);
                if maps[0].time >= run_evolution_parameters.ticks_per_evolution {
                    sample_maps(&mut maps);
                    maps.evolve_random(
                        &mut rng,
                        evolution_parameters.change_chance,
                        evolution_parameters.change_entropy,
                    );
                }
            }
        }
    }
}

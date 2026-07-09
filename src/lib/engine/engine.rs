use std::{
    sync::{Arc, mpsc},
    thread,
    time::{Duration, SystemTime},
};

use super::{parameters::*, saving::*};
use crate::{evolution::*, map::MapData, utils::SlowMutex};

pub enum EngineCommand {
    Restart,
    UpdateParameters(EngineParameters),

    Save,
    Load(String),

    Tick,
    RunTick,
    StopRunTick,

    Evolve,
    RunEvolution,
    StopRunEvolution,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EngineState {
    Stale,
    RunTick,
    RunEvolution,
}

#[derive(Debug, Clone, Default)]
pub struct EngineParameters {
    pub saving_parameters: SavingParameters,
    pub evolution_parameters: EvolutionParameters,
}

#[derive(Debug, Clone, Copy)]
struct SaveMark {
    time: SystemTime,
    evolution: u32,
}

impl Default for SaveMark {
    fn default() -> Self {
        Self {
            time: SystemTime::UNIX_EPOCH,
            evolution: Default::default(),
        }
    }
}

pub fn run_engine(
    receiver: mpsc::Receiver<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,
) {
    let mut rng = rand::rng();

    let mut maps = slow_maps.force_read();

    let mut parameters = EngineParameters::default();

    let mut save = false;
    let mut last_save = SaveMark::default();

    let mut state = EngineState::Stale;

    loop {
        if let Ok(command) = receiver.try_recv() {
            match command {
                EngineCommand::Restart => {
                    maps.iter_mut().for_each(|map| {
                        map.evolution_data = PlantEvolutionData::generate();
                        map.evolutions = 0;
                        map.restart();
                    });
                    last_save = SaveMark::default();
                    slow_maps.force_write(maps.clone());
                }
                EngineCommand::UpdateParameters(new_parameters) => {
                    parameters = new_parameters;
                }

                EngineCommand::Save => {
                    save = true;
                }
                EngineCommand::Load(_) => {
                    state = EngineState::Stale;
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

                EngineCommand::Evolve => {
                    if parameters.evolution_parameters.parent_evolution {
                        parents_random_evolve(
                            &mut rng,
                            &mut maps,
                            parameters.evolution_parameters.plants,
                            parameters.evolution_parameters.samples,
                            parameters.evolution_parameters.change_chance,
                            parameters.evolution_parameters.change_entropy,
                        );
                    } else {
                        random_evolve(
                            &mut rng,
                            &mut maps,
                            parameters.evolution_parameters.plants,
                            parameters.evolution_parameters.samples,
                            parameters.evolution_parameters.change_chance,
                            parameters.evolution_parameters.change_entropy,
                        );
                    }
                    slow_maps.force_write(maps.clone());
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

        if !save && parameters.saving_parameters.enabled {
            match parameters.saving_parameters.period {
                SavingPeriod::EveryDuration(duration) => {
                    save = SystemTime::now().duration_since(last_save.time).unwrap() > duration;
                }
                SavingPeriod::EveryEvolution(period) => {
                    save = maps[0].evolutions.saturating_sub(last_save.evolution) > period;
                }
            }
        }

        if save {
            save_maps("".into(), &parameters.saving_parameters.selection, &maps);
            last_save = SaveMark {
                time: SystemTime::now(),
                evolution: maps[0].evolutions,
            };
            save = false;
        }

        match state {
            EngineState::Stale => {
                thread::sleep(Duration::from_millis(20));
            }
            EngineState::RunTick => {
                maps.iter_mut().for_each(|map| {
                    map.tick();
                });
                slow_maps.slow_write(&maps);
            }
            EngineState::RunEvolution => {
                (0..(parameters
                    .evolution_parameters
                    .run_evolution_parameters
                    .ticks_per_slow_write
                    .min(
                        parameters
                            .evolution_parameters
                            .run_evolution_parameters
                            .ticks_per_evolution
                            .saturating_sub(maps[0].ticks),
                    )))
                    .for_each(|_| {
                        maps.iter_mut().for_each(|map| map.tick());
                    });
                if maps[0].ticks
                    >= parameters
                        .evolution_parameters
                        .run_evolution_parameters
                        .ticks_per_evolution
                {
                    slow_maps.force_write(maps.clone());
                    if parameters.evolution_parameters.parent_evolution {
                        parents_random_evolve(
                            &mut rng,
                            &mut maps,
                            parameters.evolution_parameters.plants,
                            parameters.evolution_parameters.samples,
                            parameters.evolution_parameters.change_chance,
                            parameters.evolution_parameters.change_entropy,
                        );
                    } else {
                        random_evolve(
                            &mut rng,
                            &mut maps,
                            parameters.evolution_parameters.plants,
                            parameters.evolution_parameters.samples,
                            parameters.evolution_parameters.change_chance,
                            parameters.evolution_parameters.change_entropy,
                        );
                    }
                }
            }
        }
    }
}

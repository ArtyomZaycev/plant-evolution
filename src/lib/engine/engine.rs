use std::{
    sync::{Arc, RwLock, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

use super::{parameters::*, saving::*};
use crate::{evolution::*, map::MapData, utils::*};

pub enum EngineCommand {
    Restart,

    Load(String),

    Tick,
    Evolve,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InnerEngineState {
    #[default]
    Stale,
    RunSimulation {
        autoevolve: Option<u32>,
    },
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

pub struct Engine {
    command_sender: mpsc::Sender<EngineCommand>,
    #[allow(dead_code)]
    handler: JoinHandle<()>,
    pub state: EngineSharedState,
}

// Accessible by both threads
#[derive(Debug, Clone)]
pub struct EngineSharedState {
    pub simulation_id: Arc<RwLock<String>>,
    pub inner_state: Arc<VersionedMutex<InnerEngineState>>,
    pub maps: Arc<SlowMutex<Vec<MapData>>>,
    pub parameters: Arc<VersionedMutex<EngineParameters>>,
}

impl EngineSharedState {
    fn new(maps: SlowMutex<Vec<MapData>>) -> Self {
        Self {
            simulation_id: Arc::new(RwLock::new(format!(
                "Simulation {}",
                chrono::Local::now().format("%Y-%m-%d %H-%M-%S")
            ))),
            inner_state: Default::default(),
            maps: Arc::new(maps),
            parameters: Default::default(),
        }
    }
}

impl Engine {
    pub fn new(maps: Vec<MapData>) -> Self {
        let maps = SlowMutex::new(maps);
        let state = EngineSharedState::new(maps);
        let (tx, rx) = mpsc::channel();
        Self {
            command_sender: tx,
            handler: Self::create_run_thread(state.clone(), rx),
            state,
        }
    }

    pub fn send_command(
        &mut self,
        command: EngineCommand,
    ) -> Result<(), mpsc::SendError<EngineCommand>> {
        self.command_sender.send(command)
    }

    fn create_run_thread(
        state: EngineSharedState,
        rx: mpsc::Receiver<EngineCommand>,
    ) -> JoinHandle<()> {
        thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                Self::run(state, rx);
            })
            .unwrap()
    }

    fn run(shared_state: EngineSharedState, receiver: mpsc::Receiver<EngineCommand>) {
        let mut rng = rand::rng();

        let mut parameters = shared_state.parameters.read();
        let mut maps = SlowMutexReadResult::get(shared_state.maps.read());

        let mut last_save = SaveMark::default();

        loop {
            let state = VersionedMutexData::take(shared_state.inner_state.read());
            shared_state.parameters.update(&mut parameters);

            if let Ok(command) = receiver.try_recv() {
                match command {
                    EngineCommand::Restart => {
                        maps.iter_mut().for_each(|map| {
                            map.evolution_data = PlantEvolutionData::generate();
                            map.evolutions = 0;
                            map.restart();
                        });
                        last_save = SaveMark::default();
                        shared_state.maps.force_write(maps.clone());
                    }

                    EngineCommand::Load(_) => {}

                    EngineCommand::Tick => {
                        maps.iter_mut().for_each(|map| {
                            map.tick();
                        });
                        shared_state.maps.force_write(maps.clone());
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
                        shared_state.maps.force_write(maps.clone());
                    }
                }
            }

            let save = if parameters.saving_parameters.enabled {
                match parameters.saving_parameters.period {
                    SavingPeriod::EveryDuration(duration) => {
                        SystemTime::now().duration_since(last_save.time).unwrap() > duration
                    }
                    SavingPeriod::EveryEvolution(period) => {
                        maps[0].evolutions.saturating_sub(last_save.evolution) > period
                    }
                }
            } else {
                false
            };

            if save {
                save_maps(
                    simulation_save_folder_path(
                        parameters.saving_parameters.path.clone(),
                        shared_state.simulation_id.read().unwrap().clone(),
                    ),
                    &parameters.saving_parameters.selection,
                    &maps,
                );
                last_save = SaveMark {
                    time: SystemTime::now(),
                    evolution: maps[0].evolutions,
                };
            }

            match state {
                InnerEngineState::Stale => {
                    thread::sleep(Duration::from_millis(20));
                }
                InnerEngineState::RunSimulation { autoevolve: None } => {
                    maps.iter_mut().for_each(|map| {
                        map.tick();
                    });
                    shared_state.maps.slow_write(&maps);
                }
                InnerEngineState::RunSimulation {
                    autoevolve: Some(ticks_per_evolution),
                } => {
                    (0..(parameters
                        .evolution_parameters
                        .run_evolution_parameters
                        .ticks_per_slow_write
                        .min(ticks_per_evolution.saturating_sub(maps[0].ticks))))
                        .for_each(|_| {
                            maps.iter_mut().for_each(|map| map.tick());
                        });
                    if maps[0].ticks >= ticks_per_evolution {
                        shared_state.maps.force_write(maps.clone());
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
}

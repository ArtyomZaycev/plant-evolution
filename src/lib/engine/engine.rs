use std::{
    sync::{Arc, RwLock, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

use super::{parameters::*, saving::*};
use crate::{evolution::*, map::MapData, utils::*};

pub enum EngineCommand {
    Restart,

    Save,
    Load(String),

    Tick,
    Evolve,

    GoStale,
    RunTick,
    RunEvolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InnerEngineState {
    #[default]
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

// TODO: Rename
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

        let mut save = false;
        let mut last_save = SaveMark::default();

        loop {
            let mut state = VersionedMutexData::take(shared_state.inner_state.read());
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

                    EngineCommand::Save => {
                        save = true;
                    }
                    EngineCommand::Load(_) => {
                        state = InnerEngineState::Stale;
                    }

                    EngineCommand::Tick => {
                        state = InnerEngineState::Stale;
                        maps.iter_mut().for_each(|map| {
                            map.tick();
                        });
                        shared_state.maps.force_write(maps.clone());
                    }
                    EngineCommand::Evolve => {
                        state = InnerEngineState::Stale;
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

                    EngineCommand::GoStale => {
                        state = InnerEngineState::Stale;
                        shared_state.maps.force_write(maps.clone());
                    }
                    EngineCommand::RunTick => {
                        state = InnerEngineState::RunTick;
                    }
                    EngineCommand::RunEvolution => {
                        state = InnerEngineState::RunEvolution;
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
                save = false;
            }

            match state {
                InnerEngineState::Stale => {
                    thread::sleep(Duration::from_millis(20));
                }
                InnerEngineState::RunTick => {
                    maps.iter_mut().for_each(|map| {
                        map.tick();
                    });
                    shared_state.maps.slow_write(&maps);
                }
                InnerEngineState::RunEvolution => {
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

            shared_state.inner_state.write(state);
        }
    }
}

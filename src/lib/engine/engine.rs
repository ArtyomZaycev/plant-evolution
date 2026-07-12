use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
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

pub enum EngineLog {
    SaveLog(SaveLog),
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
    pub logs_receiver: mpsc::Receiver<EngineLog>,
    #[allow(dead_code)]
    handler: JoinHandle<()>,
    pub state: EngineSharedState,
}

// Accessible by both threads
// Needs to be reworked, doesn't give information about what should/can be updated from where
#[derive(Clone)]
pub struct EngineSharedState {
    pub total_evolutions: Arc<AtomicU32>,
    pub simulation_id: Arc<RwLock<String>>,
    pub inner_state: Arc<VersionedMutex<InnerEngineState>>,
    pub maps: Arc<SlowMutex<Vec<MapData>>>,
    pub parameters: Arc<VersionedMutex<EngineParameters>>,
}

impl EngineSharedState {
    fn new(maps: SlowMutex<Vec<MapData>>) -> Self {
        Self {
            total_evolutions: Default::default(),
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
    #[cfg(feature = "thread_evolution")]
    const DEFAULT_THREAD_COUNT: u32 = 8;

    pub fn new(maps: Vec<MapData>) -> Self {
        let maps = SlowMutex::new(maps);
        let state = EngineSharedState::new(maps);
        let (commands_tx, commands_rx) = mpsc::channel();
        let (logs_tx, logs_rx) = mpsc::channel();
        Self {
            command_sender: commands_tx,
            logs_receiver: logs_rx,
            handler: Self::create_run_thread(state.clone(), commands_rx, logs_tx),
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
        tx: mpsc::Sender<EngineLog>,
    ) -> JoinHandle<()> {
        thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                Self::run(state, rx, tx);
            })
            .unwrap()
    }

    fn run(
        shared_state: EngineSharedState,
        receiver: mpsc::Receiver<EngineCommand>,
        logs_sender: mpsc::Sender<EngineLog>,
    ) {
        let mut rng = get_rng();

        #[cfg(feature = "thread_evolution")]
        let mut threadpool = {
            let thread_count = std::env::var("threadpool_size")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(Self::DEFAULT_THREAD_COUNT);
            scoped_threadpool::Pool::new(thread_count)
        };

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
                            map.evolution_data = PlantEvolutionData::generate(&mut rng);
                            map.restart();
                        });
                        last_save = SaveMark::default();
                        shared_state.total_evolutions.store(0, Ordering::Relaxed);
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
                        shared_state.total_evolutions.update(
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                            |v| v + 1,
                        );
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
                        shared_state
                            .total_evolutions
                            .load(Ordering::Relaxed)
                            .saturating_sub(last_save.evolution)
                            > period
                    }
                }
            } else {
                false
            };

            if save {
                let save_log = save_maps(
                    simulation_save_folder_path(
                        parameters.saving_parameters.path.clone(),
                        shared_state.simulation_id.read().unwrap().clone(),
                    ),
                    &parameters.saving_parameters.selection,
                    &maps,
                );
                last_save = SaveMark {
                    time: save_log.time,
                    evolution: shared_state.total_evolutions.load(Ordering::Relaxed),
                };
                logs_sender.send(EngineLog::SaveLog(save_log)).unwrap();
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
                    let number_of_ticks = parameters
                        .evolution_parameters
                        .run_evolution_parameters
                        .ticks_per_slow_write
                        .min(ticks_per_evolution.saturating_sub(maps[0].ticks));

                    #[cfg(feature = "thread_evolution")]
                    hotpath::measure_block!("thread_evolution", {
                        let chunk_size = maps.len().div_ceil(threadpool.thread_count() as usize);
                        threadpool.scoped(|scope| {
                            for chunk in maps.chunks_mut(chunk_size) {
                                scope.execute(|| {
                                    chunk.iter_mut().for_each(|map| {
                                        (0..number_of_ticks).for_each(|_| map.tick())
                                    });
                                });
                            }
                        });
                    });

                    #[cfg(not(feature = "thread_evolution"))]
                    hotpath::measure_block!("not thread_evolution", {
                        maps.iter_mut()
                            .for_each(|map| (0..number_of_ticks).for_each(|_| map.tick()));
                    });

                    if maps[0].ticks < ticks_per_evolution {
                        shared_state.maps.slow_write(&maps);
                    } else {
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
                        shared_state.total_evolutions.update(
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                            |v| v + 1,
                        );
                    }
                }
            }
        }
    }
}

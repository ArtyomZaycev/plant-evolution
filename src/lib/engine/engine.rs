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
    UpdateParameters(EngineParameters),

    Load(String),

    Restart,

    Tick,
    Evolve,

    Stop,
    RunTicks,
    RunSimulationa(u32),

    Die,
}

pub enum EngineData {
    SaveLog(SaveLog),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum InnerEngineState {
    #[default]
    Stale,
    // TODO: Separate by autoevolve
    RunSimulation {
        autoevolve: Option<u32>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct EngineParameters {
    pub saving_parameters: SavingParameters,
    pub evolution_parameters: EvolutionParameters,
    pub performance_parameters: PerformanceParameters,
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
    pub logs_receiver: mpsc::Receiver<EngineData>,
    #[allow(dead_code)]
    handler: JoinHandle<()>,
    pub state: EngineSharedState,
    pub maps: ReadAccessor<Versioned<Vec<MapData>>>,
}

/// Accessible by both threads<br>
/// Engine can't be expected to do something because shared state changed,
/// use EngineCommand for that
#[derive(Clone)]
pub struct EngineSharedState {
    pub total_evolutions: Arc<AtomicU32>,
    pub simulation_id: Arc<RwLock<String>>,
}

impl EngineSharedState {
    fn new() -> Self {
        Self {
            total_evolutions: Default::default(),
            simulation_id: Arc::new(RwLock::new(format!(
                "Simulation {}",
                chrono::Local::now().format("%Y-%m-%d %H-%M-%S")
            ))),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.send_command(EngineCommand::Die);
    }
}

impl Engine {
    pub fn new(maps: Vec<MapData>, parameters: EngineParameters) -> Self {
        let maps_buffer = SharedBuffer::new_cloned(Versioned::new(maps.clone()));
        let (reader, writer) = maps_buffer.init();
        let state = EngineSharedState::new();
        let (commands_tx, commands_rx) = mpsc::channel();
        let (logs_tx, logs_rx) = mpsc::channel();
        Self {
            command_sender: commands_tx,
            logs_receiver: logs_rx,
            handler: Self::create_run_thread(
                state.clone(),
                parameters,
                writer,
                commands_rx,
                logs_tx,
            ),
            state,
            maps: reader,
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
        parameters: EngineParameters,
        maps_accessor: WriteAccessor<Versioned<Vec<MapData>>>,
        rx: mpsc::Receiver<EngineCommand>,
        tx: mpsc::Sender<EngineData>,
    ) -> JoinHandle<()> {
        thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                Self::run(state, parameters, maps_accessor, rx, tx);
            })
            .unwrap()
    }

    fn do_tick_many(
        map: &mut MapData,
        number_of_ticks: u32,
        use_local_growth: bool,
        use_tick_many: bool,
    ) {
        if use_tick_many {
            let mut old_map = map.clone();
            map.tick_many(number_of_ticks, use_local_growth);
            (0..number_of_ticks).for_each(|_| old_map.tick(use_local_growth));

            assert_eq!(map.ticks, old_map.ticks);
            if map.plant_nutrition != old_map.plant_nutrition {
                println!(
                    "not equal!\nleft = {:?}\nright = {:?}",
                    map.plant_nutrition, old_map.plant_nutrition
                );
            }
            assert_eq!(map.cells_pos, old_map.cells_pos);
        } else {
            (0..number_of_ticks).for_each(|_| map.tick(use_local_growth));
        }
    }

    #[cfg(feature = "thread_evolution")]
    fn run_ticks_threaded(
        threadpool: &mut scoped_threadpool::Pool,
        maps: &mut Vec<MapData>,
        thread_count: u32,
        number_of_ticks: u32,
        use_local_growth: bool,
        use_tick_many: bool,
    ) {
        hotpath::measure_block!("thread_evolution", {
            let chunk_size = maps
                .len()
                .div_ceil(threadpool.thread_count().min(thread_count) as usize);
            threadpool.scoped(|scope| {
                for chunk in maps.chunks_mut(chunk_size) {
                    scope.execute(|| {
                        chunk.iter_mut().for_each(|map| {
                            Self::do_tick_many(
                                map,
                                number_of_ticks,
                                use_local_growth,
                                use_tick_many,
                            );
                        });
                    });
                }
            });
        });
    }

    fn run_ticks(
        maps: &mut Vec<MapData>,
        number_of_ticks: u32,
        use_local_growth: bool,
        use_tick_many: bool,
    ) {
        hotpath::measure_block!("not thread_evolution", {
            maps.iter_mut().for_each(|map| {
                Self::do_tick_many(map, number_of_ticks, use_local_growth, use_tick_many)
            });
        });
    }

    fn do_evolution(rng: &mut Rng, parameters: &EvolutionParameters, maps: &mut Vec<MapData>) {
        if parameters.parent_evolution {
            parents_random_evolve(
                rng,
                maps,
                parameters.plants,
                parameters.samples,
                parameters.change_chance,
                parameters.change_entropy,
            );
        } else {
            random_evolve(
                rng,
                maps,
                parameters.plants,
                parameters.samples,
                parameters.change_chance,
                parameters.change_entropy,
            );
        }
    }

    fn run(
        shared_state: EngineSharedState,
        mut parameters: EngineParameters,
        maps_accessor: WriteAccessor<Versioned<Vec<MapData>>>,
        receiver: mpsc::Receiver<EngineCommand>,
        logs_sender: mpsc::Sender<EngineData>,
    ) {
        let mut rng = get_rng();

        #[cfg(feature = "thread_evolution")]
        let mut threadpool = {
            let thread_count = DEFAULT_THREAD_COUNT;
            scoped_threadpool::Pool::new(thread_count)
        };

        let mut maps_update_stopwatch = Stopwatch::new(Duration::from_millis(100));
        let mut maps = maps_accessor.as_inner().read().unwrap().get_data();

        let mut last_save = SaveMark::default();

        let mut state = InnerEngineState::Stale;

        loop {
            if let Ok(command) = receiver.try_recv() {
                match command {
                    EngineCommand::UpdateParameters(new_parameters) => {
                        maps_update_stopwatch.interval =
                            new_parameters.performance_parameters.slow_update_interval;
                        parameters = new_parameters;
                    }

                    EngineCommand::Load(_) => {
                        todo!()
                    }

                    EngineCommand::Restart => {
                        maps.iter_mut().for_each(|map| {
                            map.evolution_data = PlantEvolutionData::generate(&mut rng);
                            map.restart();
                        });
                        last_save = SaveMark::default();
                        shared_state.total_evolutions.store(0, Ordering::Relaxed);

                        maps_accessor.write().unwrap().force_write(maps.clone());
                        maps_update_stopwatch.reset();
                    }

                    EngineCommand::Tick => {
                        let use_local_growth = parameters.performance_parameters.use_local_growth;
                        maps.iter_mut().for_each(|map| {
                            map.tick(use_local_growth);
                        });

                        maps_accessor.write().unwrap().force_write(maps.clone());
                        maps_update_stopwatch.reset();
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

                        maps_accessor.write().unwrap().force_write(maps.clone());
                        maps_update_stopwatch.reset();
                    }

                    EngineCommand::Stop => {
                        state = InnerEngineState::Stale;
                    }
                    EngineCommand::RunTicks => {
                        state = InnerEngineState::RunSimulation { autoevolve: None };
                    }
                    EngineCommand::RunSimulationa(autoevolve_at) => {
                        state = InnerEngineState::RunSimulation {
                            autoevolve: Some(autoevolve_at),
                        };
                    }

                    EngineCommand::Die => {
                        maps_accessor.write().unwrap().write(&maps);
                        maps_update_stopwatch.reset();
                        break;
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
                logs_sender.send(EngineData::SaveLog(save_log)).unwrap();
            }

            match state {
                InnerEngineState::Stale => {
                    thread::sleep(Duration::from_millis(20));
                }
                InnerEngineState::RunSimulation { autoevolve: None } => {
                    let use_local_growth = parameters.performance_parameters.use_local_growth;
                    maps.iter_mut().for_each(|map| {
                        map.tick(use_local_growth);
                    });
                    if parameters.performance_parameters.enable_updates {
                        if maps_update_stopwatch.is_elapsed_reset() {
                            maps_accessor.write().unwrap().force_write(maps.clone());
                        }
                    }
                }
                InnerEngineState::RunSimulation {
                    autoevolve: Some(ticks_per_evolution),
                } => {
                    let number_of_ticks = parameters
                        .evolution_parameters
                        .run_evolution_parameters
                        .ticks_per_slow_write
                        .min(ticks_per_evolution.saturating_sub(maps[0].ticks));

                    let use_local_growth = parameters.performance_parameters.use_local_growth;
                    let use_tick_many = parameters.performance_parameters.use_tick_many;

                    #[cfg(feature = "thread_evolution")]
                    if parameters.performance_parameters.multithreading_enabled {
                        Self::run_ticks_threaded(
                            &mut threadpool,
                            &mut maps,
                            parameters.performance_parameters.number_of_threads,
                            number_of_ticks,
                            use_local_growth,
                            use_tick_many,
                        );
                    } else {
                        Self::run_ticks(
                            &mut maps,
                            number_of_ticks,
                            use_local_growth,
                            use_tick_many,
                        );
                    }

                    #[cfg(not(feature = "thread_evolution"))]
                    Self::run_ticks(&mut maps, number_of_ticks, use_local_growth, use_tick_many);

                    if maps[0].ticks < ticks_per_evolution {
                        if parameters.performance_parameters.enable_updates {
                            if maps_update_stopwatch.is_elapsed_reset() {
                                maps_accessor.write().unwrap().force_write(maps.clone());
                            }
                        }
                    } else {
                        if parameters.performance_parameters.enable_updates {
                            if parameters.performance_parameters.slow_updates {
                                if maps_update_stopwatch.is_elapsed() {
                                    maps_accessor.write().unwrap().force_write(maps.clone());
                                    maps_update_stopwatch.reset();
                                }
                            } else {
                                maps_accessor.write().unwrap().force_write(maps.clone());
                                maps_update_stopwatch.reset();
                            }
                        }
                        Self::do_evolution(&mut rng, &parameters.evolution_parameters, &mut maps);
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

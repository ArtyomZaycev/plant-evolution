use std::{
    fs::create_dir_all,
    mem::discriminant,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{
    evolution::{PlantEvolutionData, calculate_score, sample_evolve_maps},
    map::*,
    random_evolution::RandomEvolution,
    slow_mutex::SlowMutex,
};

#[derive(Debug, Clone, Copy)]
pub enum SavingPeriod {
    // Always works
    EveryDuration(Duration),
    // For Tick and RunTick
    EveryTick(u32),
    // For RunEvolution
    EveryEvolution(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum SaveSelection {
    All,
    Best(usize),
}

#[derive(Debug, Clone)]
pub struct SavingParameters {
    enabled: bool,
    period: SavingPeriod,
    selection: SaveSelection,
}

impl Default for SavingParameters {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            period: SavingPeriod::EveryDuration(Duration::from_mins(5)),
            selection: SaveSelection::Best(1),
        }
    }
}

fn main_save_folder_path(simulation_id: &str) -> String {
    format!("./saves/{simulation_id}",)
}

fn next_save_folder_path(simulation_id: &str) -> String {
    format!(
        "{}/{}",
        main_save_folder_path(simulation_id),
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SaveFileInfo {
    pub version: usize,
}

fn save_maps(parameters: &SavingParameters, simulation_id: &str, maps: &Vec<MapData>) {
    if let Err(err) = create_dir_all(main_save_folder_path(simulation_id)) {
        println!("Saving failed: Can't create main save folder: {:?}", err)
    } else {
        let save_file_info = SaveFileInfo { version: 1 };

        let folder = next_save_folder_path(simulation_id);
        if let Err(err) = create_dir_all(next_save_folder_path(simulation_id)) {
            println!("Saving failed: Can't create next save folder {:?}", err)
        } else {
            let _ = std::fs::write(
                format!("{folder}/info.json"),
                serde_json::to_string(&save_file_info).unwrap(),
            );

            let save_map = |idx: usize| {
                let file_path = format!("{folder}/map{idx}");
                let _ = std::fs::write(
                    file_path,
                    serde_json::to_string(&maps[idx].evolution_data).unwrap(),
                );
            };

            match parameters.selection {
                SaveSelection::All => {
                    (0..maps.len()).for_each(save_map);
                }
                SaveSelection::Best(samples) => {
                    let mut maps_score = maps
                        .iter()
                        .enumerate()
                        .map(|(i, map)| (calculate_score(map), i))
                        .collect::<Vec<_>>();
                    maps_score.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap().reverse());

                    for i in 0..(maps.len().min(samples)) {
                        save_map(maps_score[i].1);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EvolutionParameters {
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
}

impl Default for EvolutionParameters {
    fn default() -> Self {
        Self {
            samples: 10,
            change_chance: 0.1,
            change_entropy: 0.8,
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
    Load(String),
    Save,
    Restart,

    Tick,
    RunTick,
    StopRunTick,

    UpdateEvolutionParameters(EvolutionParameters),
    Evolve,

    UpdateSavingParameters(SavingParameters),
    UpdateRunEvolutionParameters(RunEvolutionParameters),
    RunEvolution,
    StopRunEvolution,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EngineState {
    Stale,
    RunTick,
    RunEvolution,
}

const ENGINE_RUN_EVOLUTION_TICKS_PER_SLOW_WRITE: u32 = 100;

pub fn run_engine(
    receiver: mpsc::Receiver<EngineCommand>,
    slow_maps: Arc<SlowMutex<Vec<MapData>>>,
) {
    let mut rng = rand::rng();

    let simulation_id = rng.random::<u64>().to_string();

    let mut maps = slow_maps.force_read();

    let mut saving_parameters = SavingParameters::default();
    let mut evolution_parameters = EvolutionParameters::default();
    let mut run_evolution_parameters = RunEvolutionParameters::default();

    let mut save = false;
    let mut last_save: u128 = 0;

    let mut state = EngineState::Stale;

    loop {
        if let Ok(command) = receiver.try_recv() {
            match command {
                EngineCommand::Load(path) => {
                    state = EngineState::Stale;
                }
                EngineCommand::Save => {
                    save = true;
                }
                EngineCommand::Restart => {
                    maps.iter_mut().for_each(|map| {
                        map.evolution_data = PlantEvolutionData::generate();
                        map.evolutions = 0;
                        map.restart();
                    });
                    last_save = 0;
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

                EngineCommand::UpdateSavingParameters(new_saving_parameters) => {
                    if !new_saving_parameters.enabled
                        || discriminant(&saving_parameters.period)
                            != discriminant(&new_saving_parameters.period)
                    {
                        last_save = 0;
                    }
                    saving_parameters = new_saving_parameters;
                }
                EngineCommand::UpdateEvolutionParameters(new_evolution_parameters) => {
                    evolution_parameters = new_evolution_parameters;
                }
                EngineCommand::Evolve => {
                    sample_evolve_maps(&mut maps, evolution_parameters.samples, |map| {
                        map.evolve_random(
                            &mut rng,
                            evolution_parameters.change_chance,
                            evolution_parameters.change_entropy,
                        )
                    });
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

        if save || saving_parameters.enabled {
            match saving_parameters.period {
                SavingPeriod::EveryDuration(duration) => {
                    let time = std::time::SystemTime::now();
                    if save
                        || time
                            .duration_since(
                                SystemTime::UNIX_EPOCH + Duration::from_millis(last_save as u64),
                            )
                            .unwrap()
                            > duration
                    {
                        save_maps(&saving_parameters, &simulation_id, &maps);
                        last_save = time
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                    }
                }
                SavingPeriod::EveryTick(period) => {
                    if save || state != EngineState::RunEvolution {
                        if save || maps[0].ticks.saturating_sub(last_save as u32) > period {
                            save_maps(&saving_parameters, &simulation_id, &maps);
                            last_save = maps[0].ticks as u128;
                        }
                    }
                }
                SavingPeriod::EveryEvolution(period) => {
                    if save || maps[0].evolutions.saturating_sub(last_save as u32) > period {
                        save_maps(&saving_parameters, &simulation_id, &maps);
                        last_save = maps[0].evolutions as u128;
                    }
                }
            }

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
                (0..(ENGINE_RUN_EVOLUTION_TICKS_PER_SLOW_WRITE.min(
                    run_evolution_parameters
                        .ticks_per_evolution
                        .saturating_sub(maps[0].ticks),
                )))
                    .for_each(|_| {
                        maps.iter_mut().for_each(|map| map.tick());
                    });
                if maps[0].ticks >= run_evolution_parameters.ticks_per_evolution {
                    slow_maps.force_write(maps.clone());
                    sample_evolve_maps(&mut maps, evolution_parameters.samples, |map| {
                        map.evolve_random(
                            &mut rng,
                            evolution_parameters.change_chance,
                            evolution_parameters.change_entropy,
                        )
                    });
                }
            }
        }
    }
}

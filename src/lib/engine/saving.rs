use std::path::PathBuf;
use std::{fs::create_dir_all, time::SystemTime};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::evolution::MapScoreFormula;
use crate::map::MapData;

use super::parameters::*;

#[derive(Debug, Clone)]
pub struct SaveLog {
    pub time: SystemTime,
    pub path: PathBuf,
    pub error: Option<String>,
}

pub fn simulation_save_folder_path(mut saves_folder: PathBuf, simulation_id: String) -> PathBuf {
    saves_folder.push(format!("{}/", simulation_id));
    saves_folder
}

fn next_save_folder_path(simulation_folder: &PathBuf) -> PathBuf {
    let mut path = simulation_folder.clone();
    path.push(format!("{}/", Local::now().format("%Y-%m-%d %H-%M-%S %6f")));
    path
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SaveFileInfo {
    pub version: usize,
}

pub fn save_maps(
    folder: PathBuf,
    selection: &SaveSelection,
    maps: &Vec<MapData>,
    score_formula: &MapScoreFormula,
) -> SaveLog {
    let mut save_log = SaveLog {
        time: SystemTime::now(),
        path: Default::default(),
        error: None,
    };

    if let Err(err) = create_dir_all(&folder) {
        save_log.error = Some("Can't create main save folder".to_owned());
        eprintln!("Saving failed: Can't create main save folder: {:?}", err);
        return save_log;
    }

    let save_file_info = SaveFileInfo { version: 1 };

    let folder = next_save_folder_path(&folder);
    if let Err(err) = create_dir_all(&folder) {
        save_log.error = Some("Can't create next save folder".to_owned());
        eprintln!("Saving failed: Can't create next save folder {:?}", err);
        return save_log;
    }

    save_log.path = folder.clone();
    let mut path = folder.clone();
    path.push("info.json");

    if let Err(err) = std::fs::write(&path, serde_json::to_string(&save_file_info).unwrap()) {
        save_log.error = Some("Can't create info.json".to_owned());
        eprintln!("Saving failed: Can't create metadata file {:?}", err);
        return save_log;
    }

    let mut save_map = |idx: usize| {
        if let Err(err) = std::fs::write(
            path.with_file_name(format!("map_{idx}")),
            serde_json::to_string(&maps[idx].evolution_data).unwrap(),
        ) {
            save_log.error = Some(format!("Failed saving map {}", idx + 1));
            eprintln!(
                "Saving partially failed: Can't create map file {} {:?}",
                idx, err
            );
        }
    };

    match selection {
        SaveSelection::All => {
            (0..maps.len()).for_each(save_map);
        }
        SaveSelection::Best(samples) => {
            let mut maps_score = maps
                .iter()
                .enumerate()
                .map(|(i, map)| (score_formula.calculate(map), i))
                .collect::<Vec<_>>();
            maps_score.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap().reverse());

            for i in 0..(maps.len().min(*samples)) {
                save_map(maps_score[i].1);
            }
        }
        SaveSelection::Selected(selected) => {
            selected.iter().copied().for_each(save_map);
        }
    }

    save_log
}

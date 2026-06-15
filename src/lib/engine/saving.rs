use std::fs::create_dir_all;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::map::MapData;

use super::parameters::*;

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

pub fn save_maps(parameters: &SavingParameters, simulation_id: &str, maps: &Vec<MapData>) {
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
                        .map(|(i, map)| (map.calculate_score(), i))
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

use std::{fs, path::PathBuf};

use directories_next::ProjectDirs;
use plant_evolution_lib::utils::rng;

pub struct Config {
    locked_seed: Option<u64>,
}

#[allow(dead_code)]
pub enum SeedLockError {
    StableRngEnabled,
    IoError(std::io::Error),
}

impl Config {
    const LOCKED_SEED_FILE_NAME: &str = "locked_seed";

    fn config_dir() -> PathBuf {
        ProjectDirs::from("xyz", "Aspid", "Plant Evolution")
            .map(|project_dirs| project_dirs.config_dir().to_path_buf())
            .unwrap_or("./config/".into())
            .with_trailing_sep()
            .to_path_buf()
    }

    pub fn load() -> Self {
        let dir = Self::config_dir();
        let _ = fs::create_dir_all(&dir);

        let locked_seed = if cfg!(feature = "stable_rng") {
            Some(rng::get_seed())
        } else {
            fs::read_to_string(dir.with_file_name(Self::LOCKED_SEED_FILE_NAME))
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
        };

        Self { locked_seed }
    }

    pub fn lock_seed(&mut self, seed: u64) -> Result<(), SeedLockError> {
        if cfg!(not(feature = "stable_rng")) {
            let path = Self::config_dir().with_file_name(Self::LOCKED_SEED_FILE_NAME);
            let result = fs::write(path, seed.to_string());
            if result.is_ok() {
                self.locked_seed = Some(seed);
            }
            result.map_err(SeedLockError::IoError)
        } else {
            Err(SeedLockError::StableRngEnabled)
        }
    }

    pub fn unlock_seed(&mut self) -> Result<(), SeedLockError> {
        if cfg!(not(feature = "stable_rng")) {
            let path = Self::config_dir().with_file_name(Self::LOCKED_SEED_FILE_NAME);
            let result = fs::remove_file(path);
            if result.is_ok() {
                self.locked_seed = None;
            }
            result.map_err(SeedLockError::IoError)
        } else {
            Err(SeedLockError::StableRngEnabled)
        }
    }

    pub fn get_locked_seed(&self) -> Option<u64> {
        self.locked_seed
    }
}

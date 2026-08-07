use std::{path::PathBuf, time::Duration};

use crate::utils::DEFAULT_THREAD_COUNT;

#[derive(Debug, Clone, Copy)]
pub enum SavingPeriod {
    EveryDuration(Duration),
    EveryEvolution(u32),
}

#[derive(Debug, Clone)]
pub enum SaveSelection {
    All,
    Best(usize),
    Selected(Vec<usize>),
}

#[derive(Debug, Clone)]
pub struct SavingParameters {
    pub path: PathBuf,
    pub enabled: bool,
    pub period: SavingPeriod,
    pub selection: SaveSelection,
}

impl SavingParameters {
    pub const DISABLED: Self = Self {
        path: PathBuf::new(),
        enabled: false,
        period: SavingPeriod::EveryDuration(Duration::from_mins(5)),
        selection: SaveSelection::Best(1),
    };
}

impl Default for SavingParameters {
    fn default() -> Self {
        Self {
            path: "./saves/".into(),
            enabled: false,
            period: SavingPeriod::EveryDuration(Duration::from_mins(5)),
            selection: SaveSelection::Best(1),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EvolutionParameters {
    pub plants: usize,
    pub samples: usize,
    pub parent_evolution: bool,
    pub change_chance: f32,
    pub change_entropy: f32,

    pub run_evolution_parameters: RunEvolutionParameters,
}

impl Default for EvolutionParameters {
    fn default() -> Self {
        Self {
            plants: 200,
            samples: 10,
            parent_evolution: true,
            change_chance: 0.05,
            change_entropy: 0.8,
            run_evolution_parameters: RunEvolutionParameters::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RunEvolutionParameters {
    pub ticks_per_slow_write: u32,
}

impl Default for RunEvolutionParameters {
    fn default() -> Self {
        Self {
            ticks_per_slow_write: 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceParameters {
    pub multithreading_enabled: bool,
    pub number_of_threads: u32,

    pub use_local_growth: bool,
    pub use_tick_many: bool,

    pub enable_updates: bool,
    pub slow_updates: bool,
    pub slow_update_interval: Duration,
}

impl PerformanceParameters {
    pub const ACCURACY: Self = Self {
        multithreading_enabled: false,
        number_of_threads: DEFAULT_THREAD_COUNT,
        use_local_growth: false,
        use_tick_many: false,
        enable_updates: true,
        slow_updates: false,
        slow_update_interval: Duration::from_millis(100),
    };

    pub const PERFORMANCE: Self = Self {
        multithreading_enabled: cfg!(feature = "thread_evolution"),
        number_of_threads: DEFAULT_THREAD_COUNT,
        use_local_growth: true,
        use_tick_many: false,
        enable_updates: false,
        slow_updates: true,
        slow_update_interval: Duration::from_millis(100),
    };

    pub const UI_PERFORMANCE: Self = Self {
        enable_updates: true,
        ..Self::PERFORMANCE
    };

    pub const BALANCE: Self = Self::UI_PERFORMANCE;
}

impl Default for PerformanceParameters {
    fn default() -> Self {
        Self {
            multithreading_enabled: cfg!(feature = "thread_evolution"),
            number_of_threads: DEFAULT_THREAD_COUNT,

            use_local_growth: false,
            use_tick_many: false,

            enable_updates: true,
            slow_updates: true,
            slow_update_interval: Duration::from_millis(100),
        }
    }
}

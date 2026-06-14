#![feature(integer_atomics)]
#![feature(lock_value_accessors)]
#![feature(iter_intersperse)]

pub mod cell;
mod const_precalc;
pub mod engine;
pub mod evolution;
pub mod evolution_volatility;
pub mod map;
pub mod parents_evolution;
pub mod random_evolution;
pub mod slow_mutex;
pub mod ui;
pub mod weights_tree;
pub mod weights_tree_ui;
pub mod ui_settings;

pub use const_precalc::populate_consts;

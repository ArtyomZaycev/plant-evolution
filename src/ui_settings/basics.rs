use std::{ops::RangeInclusive, str::FromStr};

use egui::{Widget, emath::Numeric};

pub trait RawSetting<T> {
    fn new(settings: T) -> Self;

    fn is_valid(&self) -> bool {
        self.parse().is_some()
    }

    fn parse(&self) -> Option<T>;
}

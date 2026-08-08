use std::{
    ops::{Deref, DerefMut},
    time::SystemTime,
};

pub trait Version: Default + PartialOrd {
    fn update(&mut self);
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimestampVersion(u128);

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequentialVersion(u32);

impl Version for TimestampVersion {
    fn update(&mut self) {
        self.0 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros();
    }
}

impl Version for SequentialVersion {
    fn update(&mut self) {
        self.0 += 1;
    }
}

pub struct Versioned<T, V: Version = TimestampVersion> {
    version: V,
    data: T,
}

impl<T, V: Version> Versioned<T, V> {
    pub fn new(data: T) -> Self {
        Self {
            version: V::default(),
            data,
        }
    }

    pub fn clone_from(&mut self, other: &Self)
    where
        T: Clone,
    {
        if other.version > self.version {
            self.data = other.data.clone();
        }
    }

    pub fn take_from(&mut self, other: Self) {
        if other.version > self.version {
            self.data = other.data;
        }
    }

    pub fn update_from<F: FnOnce(&T) -> T>(&mut self, other: Self, copy: F) {
        if other.version > self.version {
            self.data = copy(&other.data);
        }
    }
}

impl<T, V: Version> Deref for Versioned<T, V> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, V: Version> DerefMut for Versioned<T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.version.update();
        &mut self.data
    }
}

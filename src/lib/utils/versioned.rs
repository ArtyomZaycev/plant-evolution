use std::{
    ops::{Deref, DerefMut},
    time::SystemTime,
};

pub trait Version: Default + Clone + PartialOrd {
    fn update(&mut self);
}

pub trait AtomicVersion: Default + Clone + PartialOrd {
    fn update(&self);
}

impl<T: AtomicVersion> Version for T {
    fn update(&mut self) {
        AtomicVersion::update(self);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimestampVersion(pub u128);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequentialVersion(pub u32);

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

impl<T: Clone, V: Version> Clone for Versioned<T, V> {
    fn clone(&self) -> Self {
        Self {
            version: self.version.clone(),
            data: self.data.clone(),
        }
    }
}

// TODO: Rework, this is a mess
impl<T, V: Version> Versioned<T, V> {
    pub fn new(data: T) -> Self {
        Self {
            version: V::default(),
            data,
        }
    }

    pub fn version(&self) -> &V {
        &self.version
    }

    pub fn get_data(&self) -> VersionedData<T, V>
    where
        T: Clone,
    {
        VersionedData(self.clone())
    }

    pub fn update(&self, other: &mut VersionedData<T, V>) -> bool
    where
        T: Clone,
    {
        if self.version > other.0.version {
            other.0.force_write(self.get_data());
            true
        } else {
            false
        }
    }

    pub fn force_update(&self, other: &mut VersionedData<T, V>)
    where
        T: Clone,
    {
        other.0.force_write(self.get_data());
    }

    pub fn write(&mut self, other: &VersionedData<T, V>) -> bool
    where
        T: Clone,
    {
        if other.0.version > self.version {
            self.force_write(other.clone());
            true
        } else {
            false
        }
    }

    pub fn force_write(&mut self, other: VersionedData<T, V>) {
        self.version = other.0.version;
        self.data = other.0.data;
    }

    pub fn update_data<F: FnOnce(&mut T)>(&mut self, f: F) {
        self.version.update();
        f(&mut self.data);
    }
}

pub struct VersionedData<T, V: Version = TimestampVersion>(Versioned<T, V>);

impl<T, V: Version> VersionedData<T, V> {
    pub fn version(&self) -> &V {
        self.0.version()
    }
}

impl<T: Clone, V: Version> Clone for VersionedData<T, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, V: Version> Deref for VersionedData<T, V> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<T, V: Version> DerefMut for VersionedData<T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.version.update();
        &mut self.0.data
    }
}

#[cfg(test)]
mod test {
    use crate::utils::{TimestampVersion, Versioned};

    #[test]
    fn test() {
        let mut versioned = Versioned::<i32, TimestampVersion>::new(0);
        let mut read_data = versioned.get_data();
        let mut write_data = versioned.get_data();

        assert_eq!(*read_data, 0);
        assert_eq!(*write_data, 0);

        *write_data = 123;
        assert_eq!(*read_data, 0);
        assert_eq!(*write_data, 123);

        versioned.update(&mut read_data);
        assert_eq!(*read_data, 0);

        versioned.write(&write_data);
        assert_eq!(*write_data, 123);
        versioned.update(&mut read_data);
        assert_eq!(*read_data, 123);
    }
}

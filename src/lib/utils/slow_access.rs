use std::{ops::{Deref, DerefMut}, time::{Duration, SystemTime}};


pub struct SlowAccess<T> {
    last_access: SystemTime,
    access_interval: Duration,
    data: T,
}

impl<T> SlowAccess<T> {
    pub fn new(data: T, access_interval: Duration) -> Self {
        Self {
            last_access: SystemTime::now(),
            access_interval,
            data,
        }
    }
    
    pub fn new_default(access_interval: Duration) -> Self where T: Default {
        Self {
            last_access: SystemTime::now(),
            access_interval,
            data: Default::default(),
        }
    }

    pub fn slow_access(&mut self) -> Option<&mut T> {
        let now = SystemTime::now();
        if now.duration_since(self.last_access).is_ok_and(|duration| duration >= self.access_interval) {
            self.last_access = now;
            Some(&mut self.data)
        } else {
            None
        }
    }

    pub fn force_access(&mut self) -> &mut T {
        self.last_access = SystemTime::now();
        &mut self.data
    }
    
    pub fn slow_update<U, F: FnOnce(&mut T) -> U>(&mut self, f: F, default: U) -> U {
        match self.slow_access() {
            Some(data) => f(data),
            None => default,
        }
    }
    
    pub fn force_update<U, F: FnOnce(&mut T) -> U>(&mut self, f: F) -> U {
        f(self.force_access())
    }
}

impl<T> Deref for SlowAccess<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for SlowAccess<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
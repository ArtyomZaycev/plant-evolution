use std::{
    ops::Deref, sync::{
        Mutex,
        atomic::{AtomicU128, Ordering},
    }, time::SystemTime,
};

// TODO: Use duration
#[derive(Debug)]
pub struct SlowMutex<T> {
    read_update_interval: u128,
    write_update_interval: u128,
    last_write: AtomicU128,
    data: Mutex<T>,
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

impl<T: Default + Clone> Default for SlowMutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> SlowMutex<T>
where
    T: Clone,
{
    pub fn new(data: T) -> Self {
        Self {
            read_update_interval: 10,
            write_update_interval: 20,
            last_write: get_timestamp().into(),
            data: Mutex::new(data),
        }
    }

    #[hotpath::measure]
    pub fn read(&self) -> SlowMutexReadResult<T> {
        let data = self.data.lock().unwrap();
        SlowMutexReadResult {
            write_timestamp: self.last_write.load(Ordering::Relaxed),
            read_timestamp: get_timestamp(),
            data: data.clone()
        }
    }

    #[hotpath::measure]
    pub fn slow_update(&self, data: &mut SlowMutexReadResult<T>) -> bool {
        if get_timestamp() - data.read_timestamp >= self.read_update_interval {
            self.update(data)
        } else {
            false
        }
    }

    #[hotpath::measure]
    pub fn update(&self, data: &mut SlowMutexReadResult<T>) -> bool {
        if data.write_timestamp != self.last_write.load(Ordering::Relaxed) {
            *data = self.read();
            true
        } else {
            false
        }
    }

    #[hotpath::measure]
    pub fn slow_write(&self, data: &T) -> bool {
        if get_timestamp() - self.last_write.load(Ordering::Relaxed) >= self.write_update_interval {
            self.force_write(data.clone());
            true
        } else {
            false
        }
    }

    #[hotpath::measure]
    pub fn force_write(&self, new_data: T) {
        let mut data = self.data.lock().unwrap();
        *data = new_data;
        self.last_write.store(get_timestamp(), Ordering::Relaxed);
    }
}

pub struct SlowMutexReadResult<T> {
    // Basically version in the data
    write_timestamp: u128,
    // To make sure we don't read too frequently
    read_timestamp: u128,
    data: T
}

impl<T> SlowMutexReadResult<T> {
    pub fn get_data(result: Self) -> T {
        result.data
    }
}

impl<T> Deref for SlowMutexReadResult<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

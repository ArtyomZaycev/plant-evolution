use std::{
    sync::{
        Mutex,
        atomic::{AtomicU128, Ordering},
    },
    time::SystemTime,
};

pub struct SlowMutex<T> {
    read_update_interval: u128,
    write_update_interval: u128,
    last_read: AtomicU128,
    last_write: AtomicU128,
    data: Mutex<T>,
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

impl<T> SlowMutex<T>
where
    T: Clone,
{
    pub fn new(data: T) -> Self {
        Self {
            read_update_interval: 10,
            write_update_interval: 20,
            last_read: 0.into(),
            last_write: get_timestamp().into(),
            data: Mutex::new(data),
        }
    }

    #[hotpath::measure]
    pub fn slow_read(&self) -> Option<T> {
        if get_timestamp() - self.last_read.load(Ordering::Relaxed) >= self.read_update_interval {
            Some(self.force_read())
        } else {
            None
        }
    }

    #[hotpath::measure]
    pub fn slow_read_versioned(&self, old_version: u128) -> Option<(T, u128)> {
        let version = self.last_write.load(Ordering::Relaxed);
        if old_version != version
            && get_timestamp() - self.last_read.load(Ordering::Relaxed) >= self.read_update_interval
        {
            Some((self.force_read(), version))
        } else {
            None
        }
    }

    #[hotpath::measure]
    pub fn force_read(&self) -> T {
        let data = self.data.lock().unwrap();
        self.last_read.store(get_timestamp(), Ordering::Relaxed);
        data.clone()
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
    pub fn force_write(&self, data: T) {
        self.data.set(data).unwrap();
        self.last_write.store(get_timestamp(), Ordering::Relaxed);
    }
}

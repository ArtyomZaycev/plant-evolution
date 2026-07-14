use std::{
    borrow::Borrow, ops::Deref, sync::{
        Mutex,
        atomic::{AtomicU128, Ordering},
    }, time::SystemTime,
};

pub struct SlowMutex<T> {
    pub read_update_interval: AtomicU128,
    pub write_update_interval: AtomicU128,
    last_write: AtomicU128,
    data: hotpath::wrap::std::sync::Mutex<T>,
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
            read_update_interval: 20.into(),
            write_update_interval: 20.into(),
            last_write: get_timestamp().into(),
            data: hotpath::mutex!(Mutex::new(data), label = "SlowMutex"),
        }
    }

    pub fn read(&self) -> SlowMutexReadResult<T> {
        let data = self.data.lock().unwrap();
        SlowMutexReadResult {
            write_timestamp: self.last_write.load(Ordering::Relaxed),
            read_timestamp: get_timestamp(),
            data: data.clone(),
        }
    }

    pub fn slow_update(&self, data: &mut SlowMutexReadResult<T>) -> bool {
        if get_timestamp() - data.read_timestamp
            >= self.read_update_interval.load(Ordering::Relaxed)
        {
            self.update(data)
        } else {
            false
        }
    }

    pub fn update(&self, data: &mut SlowMutexReadResult<T>) -> bool {
        if data.write_timestamp != self.last_write.load(Ordering::Relaxed) {
            *data = self.read();
            true
        } else {
            false
        }
    }

    pub fn slow_write(&self, data: &T) -> bool {
        if get_timestamp() - self.last_write.load(Ordering::Relaxed)
            >= self.write_update_interval.load(Ordering::Relaxed)
        {
            self.force_write(data.clone());
            true
        } else {
            false
        }
    }

    pub fn force_write(&self, new_data: T) {
        let mut data = self.data.lock().unwrap();
        *data = new_data;
        self.last_write.store(get_timestamp(), Ordering::Relaxed);
    }
}

pub struct SlowMutexReadResult<T> {
    // Basically version of the data
    write_timestamp: u128,
    // To make sure we don't read too frequently
    read_timestamp: u128,
    data: T,
}

impl<T> SlowMutexReadResult<T> {
    pub fn get(value: Self) -> T {
        value.data
    }

    pub fn get_cloned(value: &Self) -> T
    where
        T: Clone,
    {
        value.data.clone()
    }
}

impl<T> Borrow<T> for SlowMutexReadResult<T> {
    fn borrow(&self) -> &T {
        &self.data
    }
}

impl<T> Deref for SlowMutexReadResult<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

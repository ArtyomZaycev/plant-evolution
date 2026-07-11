use std::{
    ops::Deref, sync::{
        Mutex, atomic::{AtomicU128, Ordering},
    }, time::SystemTime,
};

#[derive(Debug)]
pub struct VersionedMutex<T: ?Sized> {
    last_write: AtomicU128,
    data: Mutex<T>,
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

impl<T: Default + Clone> Default for VersionedMutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: Clone> VersionedMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            last_write: get_timestamp().into(),
            data: Mutex::new(data),
        }
    }

    pub fn cloned(&self) -> T {
        self.data.lock().unwrap().clone()
    }

    #[hotpath::measure]
    pub fn read(&self) -> VersionedMutexData<T> {
        let data = self.data.lock().unwrap();
        VersionedMutexData {
            write_timestamp: self.last_write.load(Ordering::Relaxed),
            data: data.clone(),
        }
    }

    #[hotpath::measure]
    pub fn update(&self, data: &mut VersionedMutexData<T>) -> bool {
        if data.write_timestamp != self.last_write.load(Ordering::Relaxed) {
            *data = self.read();
            true
        } else {
            false
        }
    }

    #[hotpath::measure]
    pub fn unchecked_write(&self, new_data: T) {
        let mut data = self.data.lock().unwrap();
        *data = new_data;
        self.last_write.store(get_timestamp(), Ordering::Relaxed);
    }

    #[hotpath::measure]
    pub fn write(&self, new_data: T)
    where
        T: PartialEq,
    {
        let mut data = self.data.lock().unwrap();
        if *data != new_data {
            *data = new_data;
            self.last_write.store(get_timestamp(), Ordering::Relaxed);
        }
    }

    pub fn update_data<F: FnOnce(&mut T)>(&self, f: F) {
        let mut data = self.data.lock().unwrap();
        f(&mut data);
        self.last_write.store(get_timestamp(), Ordering::Relaxed);
    }
}

pub struct VersionedMutexData<T> {
    // Basically version of the data
    write_timestamp: u128,
    data: T,
}

impl<T> VersionedMutexData<T> {
    pub fn take(value: Self) -> T {
        value.data
    }
    pub fn get_cloned(value: &Self) -> T
    where
        T: Clone,
    {
        value.data.clone()
    }
    pub fn get_ref(value: &Self) -> &T {
        &value.data
    }
}

impl<T> Deref for VersionedMutexData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

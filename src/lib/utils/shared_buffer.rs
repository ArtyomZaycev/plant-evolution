use std::{
    any::type_name,
    cell::SyncUnsafeCell,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct SharedBuffer<T> {
    buffer: Arc<Buffer<T>>,
    state: Arc<State>,
}

pub struct Buffer<T> {
    data1: SyncUnsafeCell<T>, // to read (if not swapped)
    data2: SyncUnsafeCell<T>, // to write (if not swapped)
}

impl<T> SharedBuffer<T> {
    pub fn new(data1: T, data2: T) -> Self {
        Self {
            buffer: Arc::new(Buffer {
                data1: SyncUnsafeCell::new(data1),
                data2: SyncUnsafeCell::new(data2),
            }),
            state: Default::default(),
        }
    }

    pub fn init(&self) -> (Accessor<T>, Accessor<T>) {
        (
            Accessor {
                buffer: self.buffer.clone(),
                state: self.state.clone(),
            },
            Accessor {
                buffer: self.buffer.clone(),
                state: self.state.clone(),
            },
        )
    }
}

#[derive(Debug, Default)]
struct State {
    reading: AtomicBool,
    writing: AtomicBool,
    need_swap: AtomicBool,
    swapped: RwLock<bool>,
}

impl State {
    fn start_read(&self) -> Option<bool> {
        let swapped = self.swapped.read().unwrap();
        if self.reading.swap(true, Ordering::Relaxed) {
            None
        } else {
            Some(*swapped)
        }
    }

    fn start_write(&self) -> Option<bool> {
        let swapped = self.swapped.read().unwrap();
        if self.writing.swap(true, Ordering::Relaxed) {
            None
        } else {
            self.need_swap.store(true, Ordering::Relaxed);
            Some(*swapped)
        }
    }

    fn stop_read(&self) {
        let swapped = self.swapped.read().unwrap();
        self.reading.store(false, Ordering::Relaxed);
        drop(swapped);
        self.try_swap();
    }

    fn stop_write(&self) {
        let swapped = self.swapped.read().unwrap();
        self.writing.store(false, Ordering::Relaxed);
        drop(swapped);
        self.try_swap();
    }

    fn try_swap(&self) {
        let mut swapped_lock = self.swapped.write().unwrap();
        if !self.reading.load(Ordering::Relaxed)
            && !self.writing.load(Ordering::Relaxed)
            && self.need_swap.load(Ordering::Relaxed)
        {
            *swapped_lock = !*swapped_lock;
            self.need_swap.store(false, Ordering::Relaxed);
        }
        drop(swapped_lock);
    }
}

pub struct Accessor<T> {
    buffer: Arc<Buffer<T>>,
    state: Arc<State>,
}

#[derive(Debug)]
pub enum AccessorError {
    LockAlreadyExists,
}

impl Display for AccessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lock aready exists")
    }
}

impl std::error::Error for AccessorError {}

impl<T> Accessor<T> {
    pub fn read<'a>(&'a self) -> Result<ReadLock<'a, T>, AccessorError> {
        if let Some(swapped) = self.state.start_read() {
            Ok(ReadLock {
                accessor: &self,
                data: if swapped {
                    self.buffer.data2.get()
                } else {
                    self.buffer.data1.get()
                },
            })
        } else {
            Err(AccessorError::LockAlreadyExists)
        }
    }

    pub fn write<'a>(&'a self) -> Result<WriteLock<'a, T>, AccessorError> {
        if let Some(swapped) = self.state.start_write() {
            Ok(WriteLock {
                accessor: &self,
                data: if swapped {
                    self.buffer.data1.get()
                } else {
                    self.buffer.data2.get()
                },
            })
        } else {
            Err(AccessorError::LockAlreadyExists)
        }
    }
}

pub struct ReadLock<'a, T> {
    accessor: &'a Accessor<T>,
    data: *const T,
}

impl<'a, T> Debug for ReadLock<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadLock")
            .field("T", &type_name::<T>())
            .field("adress", &self.data)
            .finish()
    }
}

impl<'a, T> Deref for ReadLock<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> Drop for ReadLock<'a, T> {
    fn drop(&mut self) {
        self.accessor.state.stop_read();
    }
}

pub struct WriteLock<'a, T> {
    accessor: &'a Accessor<T>,
    data: *mut T,
}

impl<'a, T> Debug for WriteLock<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteLock")
            .field("T", &type_name::<T>())
            .field("adress", &self.data)
            .finish()
    }
}

impl<'a, T> Deref for WriteLock<'a, T> {
    type Target = T;

    fn deref(&self) -> &'a Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> DerefMut for WriteLock<'a, T> {
    fn deref_mut(&mut self) -> &'a mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<'a, T> Drop for WriteLock<'a, T> {
    fn drop(&mut self) {
        self.accessor.state.stop_write();
    }
}

#[allow(unused_imports)]
mod test {
    use std::{sync::Barrier, thread};

    use crate::utils::SharedBuffer;

    #[test]
    fn test1() {
        let state = SharedBuffer::new(0, 0);
        let (a1, a2) = state.init();

        let barrier = Barrier::new(2);

        thread::scope(|s| {
            s.spawn(|| {
                let lock = a1.read().unwrap();
                barrier.wait();
                assert_eq!(*lock, 0);
                drop(lock);

                barrier.wait();
                let lock = a1.read().unwrap();
                assert_eq!(*lock, 12);
                drop(lock);

                let lock = a1.read().unwrap();
                barrier.wait();
                //println!("r1: {:?}", lock);
                assert_eq!(*lock, 12);
                barrier.wait();
                assert_eq!(*lock, 12);
                drop(lock);
                let lock = a1.read().unwrap();
                //println!("r2: {:?}", lock);
                assert_eq!(*lock, -1);
            });

            s.spawn(|| {
                barrier.wait();
                let lock = a2.write().unwrap();
                assert_eq!(*lock, 0);
                drop(lock);

                let mut lock = a2.write().unwrap();
                *lock = 12;
                drop(lock);
                barrier.wait();

                barrier.wait();
                let mut lock = a2.write().unwrap();
                //println!("w1: {:?}", lock);
                *lock = -1;
                drop(lock);
                barrier.wait();
            });
        });
    }
}

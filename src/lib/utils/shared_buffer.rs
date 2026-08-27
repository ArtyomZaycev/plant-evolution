use std::{
    any::type_name,
    cell::UnsafeCell,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

pub struct SharedBuffer<T> {
    buffer: Arc<Buffer<T>>,
    state: Arc<State>,
}

pub struct Buffer<T> {
    data1: UnsafeCell<T>, // to read (if not swapped)
    data2: UnsafeCell<T>, // to write (if not swapped)
}

// SAFETY: single-reader/single-writer protocol. The reader and writer always
// touch opposite cells, and the swap only occurs when both flags are idle
// (single atomic CAS in `State::release`), so each `T` is accessed by only
// one thread at a time. `Send` is sufficient for handing each cell between
// threads.
unsafe impl<T: Send> Sync for Buffer<T> {}

impl<T> SharedBuffer<T> {
    pub fn new(data1: T, data2: T) -> Self {
        Self {
            buffer: Arc::new(Buffer {
                data1: UnsafeCell::new(data1),
                data2: UnsafeCell::new(data2),
            }),
            state: Default::default(),
        }
    }

    pub fn new_cloned(data: T) -> Self
    where
        T: Clone,
    {
        Self {
            buffer: Arc::new(Buffer {
                data1: UnsafeCell::new(data.clone()),
                data2: UnsafeCell::new(data),
            }),
            state: Default::default(),
        }
    }

    pub fn init(&self) -> (ReadAccessor<T>, WriteAccessor<T>) {
        (
            ReadAccessor(Accessor {
                buffer: self.buffer.clone(),
                state: self.state.clone(),
            }),
            WriteAccessor(Accessor {
                buffer: self.buffer.clone(),
                state: self.state.clone(),
            }),
        )
    }
}

/// Bit 0: `swapped` — which buffer the reader currently reads.
/// Bit 1: `reading` — a read lock is currently held.
/// Bit 2: `writing` — a write lock is currently held.
/// Bit 3: `need_swap` — a write completed since the last swap.
const SWAPPED: u8 = 0b0001;
const READING: u8 = 0b0010;
const WRITING: u8 = 0b0100;
const NEED_SWAP: u8 = 0b1000;

/// The whole access/swap state packed into a single atomic, so no `RwLock`
/// (or any other OS-backed lock) is acquired on the hot read/write paths.
#[derive(Debug)]
struct State {
    flags: AtomicU8,
}

impl Default for State {
    fn default() -> Self {
        Self {
            flags: AtomicU8::new(0),
        }
    }
}

impl State {
    #[inline]
    fn start_read(&self) -> Option<bool> {
        let mut flags = self.flags.load(Ordering::Acquire);
        loop {
            if flags & READING != 0 {
                return None;
            }
            match self.flags.compare_exchange_weak(
                flags,
                flags | READING,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(flags & SWAPPED != 0),
                Err(actual) => flags = actual,
            }
        }
    }

    #[inline]
    fn start_write(&self) -> Option<bool> {
        let mut flags = self.flags.load(Ordering::Acquire);
        loop {
            if flags & WRITING != 0 {
                return None;
            }
            match self.flags.compare_exchange_weak(
                flags,
                flags | WRITING | NEED_SWAP,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(flags & SWAPPED != 0),
                Err(actual) => flags = actual,
            }
        }
    }

    #[inline]
    fn stop_read(&self) {
        self.release(READING);
    }

    #[inline]
    fn stop_write(&self) {
        self.release(WRITING);
    }

    /// Clear `flag` and, if both accesses are now idle and a swap is pending,
    /// atomically flip `swapped` and consume `need_swap`. Doing the whole
    /// transition in one CAS guarantees a single, non-duplicated swap.
    #[inline]
    fn release(&self, flag: u8) {
        let mut flags = self.flags.load(Ordering::Acquire);
        loop {
            let mut next = flags & !flag;
            if next & (READING | WRITING) == 0 && next & NEED_SWAP != 0 {
                next ^= SWAPPED;
                next &= !NEED_SWAP;
            }
            match self
                .flags
                .compare_exchange_weak(flags, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => break,
                Err(actual) => flags = actual,
            }
        }
    }
}

/// `Accessor` is split into `ReadAccessor` and `WriteAccessor` for the usual use case
/// where one thread only reads the data and another only writes it.<br>
/// They can be easily downcast with `.as_inner()`.
pub struct Accessor<T> {
    buffer: Arc<Buffer<T>>,
    state: Arc<State>,
}

pub struct ReadAccessor<T>(Accessor<T>);
pub struct WriteAccessor<T>(Accessor<T>);

impl<T> ReadAccessor<T> {
    #[inline]
    pub fn read<'a>(&'a self) -> Result<ReadLock<'a, T>, AccessorError> {
        self.0.read()
    }

    pub fn as_inner(&self) -> &Accessor<T> {
        &self.0
    }
}

impl<T> WriteAccessor<T> {
    #[inline]
    pub fn write<'a>(&'a self) -> Result<WriteLock<'a, T>, AccessorError> {
        self.0.write()
    }

    pub fn as_inner(&self) -> &Accessor<T> {
        &self.0
    }
}

#[derive(Debug)]
pub enum AccessorError {
    LockAlreadyExists,
}

impl Display for AccessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lock already exists")
    }
}

impl std::error::Error for AccessorError {}

impl<T> Accessor<T> {
    #[inline]
    pub fn read<'a>(&'a self) -> Result<ReadLock<'a, T>, AccessorError> {
        if let Some(swapped) = self.state.start_read() {
            Ok(ReadLock {
                accessor: self,
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

    #[inline]
    pub fn write<'a>(&'a self) -> Result<WriteLock<'a, T>, AccessorError> {
        if let Some(swapped) = self.state.start_write() {
            Ok(WriteLock {
                accessor: self,
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
            .field("address", &self.data)
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
    #[inline]
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
            .field("address", &self.data)
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
    #[inline]
    fn drop(&mut self) {
        self.accessor.state.stop_write();
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Barrier, thread};

    use crate::utils::SharedBuffer;

    #[test]
    fn test() {
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

    #[test]
    fn stress_single_reader_writer() {
        let buffer = SharedBuffer::new(0usize, 0usize);
        let (reader, writer) = buffer.init();

        let barrier = Barrier::new(2);

        thread::scope(|s| {
            s.spawn(|| {
                barrier.wait();
                let mut last = 0usize;
                for _ in 0..100_000 {
                    let lock = reader.read().unwrap();
                    let value = *lock;
                    assert!(
                        value >= last,
                        "reader observed out-of-order value {value} after {last}"
                    );
                    last = value;
                    drop(lock);
                }
            });

            s.spawn(|| {
                barrier.wait();
                for i in 0..100_000usize {
                    let mut lock = writer.write().unwrap();
                    *lock = i;
                    drop(lock);
                }
            });
        });
    }
}

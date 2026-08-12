use std::time::{Duration, SystemTime};

pub struct Stopwatch {
    last: SystemTime,
    pub interval: Duration,
}

impl Stopwatch {
    pub fn new(interval: Duration) -> Self {
        Self {
            last: SystemTime::now(),
            interval,
        }
    }

    pub fn is_elapsed(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last)
            .is_ok_and(|duration| duration >= self.interval)
    }

    pub fn reset(&mut self) {
        self.last = SystemTime::now();
    }

    pub fn is_elapsed_reset(&mut self) -> bool {
        if self.is_elapsed() {
            self.reset();
            true
        } else {
            false
        }
    }
}

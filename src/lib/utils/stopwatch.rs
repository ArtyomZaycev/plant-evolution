use std::time::{Duration, SystemTime};

pub struct Stopwatch {
    last_access: SystemTime,
    pub access_interval: Duration,
}

impl Stopwatch {
    pub fn new(interval: Duration) -> Self {
        Self {
            last_access: SystemTime::now(),
            access_interval: interval,
        }
    }

    pub fn slow_run<F: FnOnce()>(&mut self, f: F) {
        let now = SystemTime::now();
        if now
            .duration_since(self.last_access)
            .is_ok_and(|duration| duration >= self.access_interval)
        {
            self.last_access = now;
            f();
        }
    }

    pub fn force_run<F: FnOnce()>(&mut self, f: F) {
        self.last_access = SystemTime::now();
        f();
    }
}

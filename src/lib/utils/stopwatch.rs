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

    pub fn force(&mut self) {
        self.last_access = SystemTime::UNIX_EPOCH;
    }

    pub fn slow_run_checked<F: FnOnce() -> bool>(&mut self, f: F) -> bool {
        let now = SystemTime::now();
        if now
            .duration_since(self.last_access)
            .is_ok_and(|duration| duration >= self.access_interval)
        {
            if f() {
                self.last_access = now;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn slow_run<F: FnOnce()>(&mut self, f: F) -> bool {
        self.slow_run_checked(|| {
            f();
            true
        })
    }

    pub fn force_run_checked<F: FnOnce() -> bool>(&mut self, f: F) {
        if f() {
            self.last_access = SystemTime::now();
        }
    }

    pub fn force_run<F: FnOnce()>(&mut self, f: F) {
        f();
        self.last_access = SystemTime::now();
    }
}

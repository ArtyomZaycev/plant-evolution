use std::{ops::{Deref, DerefMut}, time::{Duration, SystemTime}};


pub struct Stopwatch {
    last_access: SystemTime,
    access_interval: Duration,
}

impl Stopwatch {

}
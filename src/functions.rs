use std::borrow::Borrow;
use std::time::Duration;
use chrono::{DateTime, Local};

pub fn time_remaining<T: Borrow<DateTime<Local>>>(time: T) -> Duration {
    (*time.borrow() - Local::now()).to_std().unwrap_or_default()
}
use std::time::{Instant, Duration};

pub struct Statistics {
    pub vehicles_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_time: Duration,
    pub min_time: Duration,
    pub close_calls: u32,
}
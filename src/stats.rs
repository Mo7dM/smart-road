//! What the simulation records while it runs, shown when the user quits.

use crate::vehicle::Vehicle;

pub struct Statistics {
    pub vehicles_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_time: f32,
    pub min_time: f32,
    pub close_calls: u32,
}

impl Statistics {
    pub fn new() -> Statistics {
        Statistics {
            vehicles_passed: 0,
            max_velocity: 0.0,
            min_velocity: f32::INFINITY,
            max_time: 0.0,
            min_time: f32::INFINITY,
            close_calls: 0,
        }
    }

    /// Called every frame for every vehicle still on the road.
    pub fn observe(&mut self, vehicle: &Vehicle) {
        if vehicle.velocity > self.max_velocity {
            self.max_velocity = vehicle.velocity;
        }
        if vehicle.velocity < self.min_velocity {
            self.min_velocity = vehicle.velocity;
        }
    }

    /// Called once, when a vehicle leaves the canvas.
    pub fn record_exit(&mut self, vehicle: &Vehicle, now: f32) {
        self.vehicles_passed += 1;
        let taken = now - vehicle.entered_intersection_at.unwrap_or(vehicle.spawn_time);
        if taken > self.max_time {
            self.max_time = taken;
        }
        if taken < self.min_time {
            self.min_time = taken;
        }
    }
}

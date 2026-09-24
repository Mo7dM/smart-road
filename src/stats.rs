//! What the simulation records while it runs, shown when the user quits.

use crate::vehicle::Vehicle;

pub struct Statistics {
    pub vehicles_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    /// Longest and shortest time a vehicle took to pass the intersection:
    /// from the moment the controller saw it until it left the canvas.
    pub max_time: f32,
    pub min_time: f32,
    pub close_calls: u32,

    // Extra statistics, beyond what the subject asks for.
    pub vehicles_spawned: u32,
    total_time: f32,
    /// Vehicles that had to stop at least once before getting through.
    pub vehicles_stopped: u32,
    pub longest_wait: f32,
    pub most_on_road: usize,
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
            vehicles_spawned: 0,
            total_time: 0.0,
            vehicles_stopped: 0,
            longest_wait: 0.0,
            most_on_road: 0,
        }
    }

    /// Called every frame for every vehicle still on the road. A vehicle
    /// waiting at the line is driving at 0, so that is the slowest speed
    /// reached as soon as anybody has had to stop.
    pub fn observe(&mut self, vehicle: &Vehicle) {
        self.max_velocity = self.max_velocity.max(vehicle.velocity);
        self.min_velocity = self.min_velocity.min(vehicle.velocity);
    }

    /// Called once per frame with how many vehicles are on the road.
    pub fn observe_traffic(&mut self, on_road: usize) {
        self.most_on_road = self.most_on_road.max(on_road);
    }

    /// Called once, when a vehicle leaves the canvas.
    pub fn record_exit(&mut self, vehicle: &Vehicle) {
        self.vehicles_passed += 1;
        self.max_time = self.max_time.max(vehicle.time);
        self.min_time = self.min_time.min(vehicle.time);
        self.total_time += vehicle.time;
        if vehicle.waited > 0.0 {
            self.vehicles_stopped += 1;
        }
        self.longest_wait = self.longest_wait.max(vehicle.waited);
    }

    /// The subject's statistics as label / value pairs, ready to display.
    pub fn required_rows(&self) -> Vec<(&'static str, String)> {
        vec![
            ("VEHICLES PASSED", self.vehicles_passed.to_string()),
            ("MAX VELOCITY", velocity(self.max_velocity, self.min_velocity.is_finite())),
            ("MIN VELOCITY", velocity(self.min_velocity, self.min_velocity.is_finite())),
            ("MAX TIME", seconds(self.max_time, self.vehicles_passed > 0)),
            ("MIN TIME", seconds(self.min_time, self.vehicles_passed > 0)),
            ("CLOSE CALLS", self.close_calls.to_string()),
        ]
    }

    /// The extra statistics, as label / value pairs.
    pub fn extra_rows(&self, elapsed: f32) -> Vec<(&'static str, String)> {
        let passed = self.vehicles_passed > 0;
        let average = self.total_time / self.vehicles_passed.max(1) as f32;
        vec![
            ("VEHICLES SPAWNED", self.vehicles_spawned.to_string()),
            ("AVERAGE TIME", seconds(average, passed)),
            ("VEHICLES THAT STOPPED", self.vehicles_stopped.to_string()),
            ("LONGEST WAIT", seconds(self.longest_wait, passed)),
            ("MOST ON ROAD AT ONCE", self.most_on_road.to_string()),
            ("SIMULATION LENGTH", seconds(elapsed, true)),
        ]
    }
}

/// Nothing to report yet shows as a dash rather than 0 or infinity.
fn velocity(value: f32, known: bool) -> String {
    if known { format!("{:.0} PX/S", value) } else { "-".to_string() }
}

fn seconds(value: f32, known: bool) -> String {
    if known { format!("{:.2} S", value) } else { "-".to_string() }
}

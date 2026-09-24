//! The world: every vehicle on the road, and the clock they all share.

use crate::config::RANDOM_SPAWN_INTERVAL;
use crate::controller;
use crate::rng::Rng;
use crate::spawner::Spawner;
use crate::stats::Statistics;
use crate::vehicle::{Direction, Turn, Vehicle};

pub struct Simulation {
    pub vehicles: Vec<Vehicle>,
    pub spawner: Spawner,
    pub stats: Statistics,
    pub random_mode: bool,
    /// Seconds since the simulation started. Every time in the project is
    /// measured on this clock, which keeps the whole thing reproducible.
    pub time: f32,
    rng: Rng,
    since_random_spawn: f32,
}

impl Simulation {
    pub fn new() -> Simulation {
        Simulation {
            vehicles: Vec::new(),
            spawner: Spawner::new(),
            stats: Statistics::new(),
            random_mode: false,
            time: 0.0,
            rng: Rng::new(),
            since_random_spawn: 0.0,
        }
    }

    /// Put a vehicle on a named route. Silently does nothing if the lane is
    /// not ready for one.
    pub fn spawn(&mut self, from: Direction, turn: Turn) {
        let Simulation {
            vehicles,
            spawner,
            time,
            ..
        } = self;
        if let Some(vehicle) = spawner.try_spawn(from, turn, *time, vehicles) {
            vehicles.push(vehicle);
        }
    }

    /// What an arrow key does: a vehicle from that side, on a random route.
    pub fn spawn_from(&mut self, from: Direction) {
        let turn = self.rng.pick(&Turn::ALL);
        self.spawn(from, turn);
    }

    /// What the `R` key does: a vehicle from a random side, on a random route.
    pub fn spawn_random(&mut self) {
        let from = self.rng.pick(&Direction::ALL);
        let turn = self.rng.pick(&Turn::ALL);
        self.spawn(from, turn);
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        if self.random_mode {
            self.since_random_spawn += dt;
            while self.since_random_spawn >= RANDOM_SPAWN_INTERVAL {
                self.since_random_spawn -= RANDOM_SPAWN_INTERVAL;
                self.spawn_random();
            }
        }

        controller::update(&mut self.vehicles);

        let now = self.time;
        for vehicle in &mut self.vehicles {
            vehicle.update(dt, now);
            self.stats.observe(vehicle);
        }

        // Retire everything that has driven off the canvas.
        let stats = &mut self.stats;
        self.vehicles.retain(|vehicle| {
            if vehicle.exited {
                stats.record_exit(vehicle, now);
                false
            } else {
                true
            }
        });
    }
}

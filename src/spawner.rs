use std::time::{Duration, Instant};
use std::collections::HashMap;

use crate::vehicle::{Vehicle, Route, Direction, Turn};
use crate::config::SPAWN_COOLDOWN_MS;

pub struct Spawner {
    last_spawn: HashMap<Direction, Instant>,
    next_id: u32,
}

impl Spawner {
    pub fn new() -> Self {
        Spawner {
            last_spawn: HashMap::new(),
            next_id: 0,
        }
    }

    /// Try to spawn a vehicle from `from`. Returns None if the cooldown
    /// for that direction hasn't elapsed yet (prevents stacking).
    pub fn try_spawn(&mut self, from: Direction, turn: Turn) -> Option<Vehicle> {
        let now = Instant::now();

        if let Some(&last) = self.last_spawn.get(&from) {
            if now.duration_since(last) < Duration::from_millis(SPAWN_COOLDOWN_MS) {
                return None;
            }
        }

        self.last_spawn.insert(from, now);

        let vehicle = Vehicle {
            id: self.next_id,
            route: Route { from, turn },
            position: (0.0, 0.0), // set properly once path.rs gives you the start point
            velocity: 0.0,
            distance_traveled: 0.0,
            spawn_time: now,
            entered_intersection_at: None,
            exited: false,
        };

        self.next_id += 1;
        Some(vehicle)
    }
}
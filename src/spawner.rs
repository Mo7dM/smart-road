//! Creating vehicles, and refusing to create them when that would put one on
//! top of another.

use std::collections::HashMap;

use crate::config::{SAFE_DISTANCE, SPAWN_COOLDOWN, VEHICLE_LENGTH};
use crate::vehicle::{Direction, Route, Turn, Vehicle};

pub struct Spawner {
    /// Simulation time of the last spawn on each approach.
    last_spawn: HashMap<Direction, f32>,
    next_id: u32,
}

impl Spawner {
    pub fn new() -> Spawner {
        Spawner {
            last_spawn: HashMap::new(),
            next_id: 0,
        }
    }

    /// Try to put a vehicle on the road. Returns `None` when the approach is
    /// still on cooldown, or when the lane's last vehicle has not yet moved
    /// far enough for a new one to fit behind it safely.
    pub fn try_spawn(
        &mut self,
        from: Direction,
        turn: Turn,
        now: f32,
        vehicles: &[Vehicle],
    ) -> Option<Vehicle> {
        if let Some(&last) = self.last_spawn.get(&from) {
            if now - last < SPAWN_COOLDOWN {
                return None;
            }
        }

        if !self.lane_is_clear(from, turn, vehicles) {
            return None;
        }

        self.last_spawn.insert(from, now);
        let id = self.next_id;
        self.next_id += 1;
        Some(Vehicle::new(id, Route { from, turn }, now))
    }

    /// A lane is clear when the vehicle furthest back on it has travelled far
    /// enough that a new one can spawn behind it without breaking the safety
    /// distance.
    fn lane_is_clear(&self, from: Direction, turn: Turn, vehicles: &[Vehicle]) -> bool {
        let needed = VEHICLE_LENGTH + SAFE_DISTANCE;
        vehicles
            .iter()
            .filter(|v| v.route.from == from && v.route.turn == turn)
            .all(|v| v.distance_traveled >= needed)
    }
}

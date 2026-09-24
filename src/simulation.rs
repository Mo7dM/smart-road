//! The world: every vehicle on the road, and the clock they all share.

use crate::config::RANDOM_SPAWN_INTERVAL;
use crate::conflict::Conflicts;
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
    conflicts: Conflicts,
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
            conflicts: Conflicts::new(),
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

        controller::update(&mut self.vehicles, &self.conflicts, dt);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SAFE_DISTANCE, VEHICLE_LENGTH, VEHICLE_WIDTH};

    type Point = (f32, f32);

    /// The car's actual rectangle: its two axes and four corners. This is
    /// deliberately not the disc model the controller plans with, so the
    /// test checks the plan against the real bodies.
    fn body(v: &Vehicle) -> ([Point; 2], [Point; 4]) {
        let rad = (v.angle as f32).to_radians();
        let ahead = (rad.cos(), rad.sin());
        let side = (-ahead.1, ahead.0);
        let (hl, hw) = (VEHICLE_LENGTH / 2.0, VEHICLE_WIDTH / 2.0);
        let (x, y) = v.position;
        let corner = |i: f32, j: f32| {
            (
                x + ahead.0 * hl * i + side.0 * hw * j,
                y + ahead.1 * hl * i + side.1 * hw * j,
            )
        };
        (
            [ahead, side],
            [corner(1.0, 1.0), corner(1.0, -1.0), corner(-1.0, 1.0), corner(-1.0, -1.0)],
        )
    }

    /// Separating axis test for two rotated rectangles.
    fn bodies_overlap(a: &Vehicle, b: &Vehicle) -> bool {
        let (axes_a, corners_a) = body(a);
        let (axes_b, corners_b) = body(b);
        let project = |corners: &[Point; 4], (ax, ay): Point| {
            corners
                .iter()
                .map(|&(x, y)| x * ax + y * ay)
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| (lo.min(p), hi.max(p)))
        };
        axes_a.iter().chain(&axes_b).all(|&axis| {
            let (a0, a1) = project(&corners_a, axis);
            let (b0, b1) = project(&corners_b, axis);
            a0 < b1 && b0 < a1
        })
    }

    /// Run `sim` for `seconds`, calling `before_frame` ahead of every step,
    /// and fail if two vehicles ever touch or one tailgates another. Returns
    /// the longest any vehicle spent on screen.
    fn run_checked(sim: &mut Simulation, seconds: f32, mut before_frame: impl FnMut(&mut Simulation, usize)) -> f32 {
        let dt = 1.0 / 60.0;
        let mut longest_trip: f32 = 0.0;
        for frame in 0..(seconds / dt) as usize {
            before_frame(sim, frame);
            sim.update(dt);

            for (i, a) in sim.vehicles.iter().enumerate() {
                longest_trip = longest_trip.max(sim.time - a.spawn_time);
                for b in &sim.vehicles[i + 1..] {
                    assert!(
                        !bodies_overlap(a, b),
                        "{:?} #{} and {:?} #{} collided at t={}",
                        a.route,
                        a.id,
                        b.route,
                        b.id,
                        sim.time
                    );
                    if a.route == b.route {
                        let gap = (a.distance_traveled - b.distance_traveled).abs() - VEHICLE_LENGTH;
                        assert!(gap >= SAFE_DISTANCE - 0.01, "#{} and #{} tailgating: {}", a.id, b.id, gap);
                    }
                }
            }
        }
        longest_trip
    }

    /// Every lane kept as full as the spawner allows, which is more traffic
    /// than the intersection can serve. Queues grow, but nobody may touch and
    /// the intersection may never lock up.
    #[test]
    fn saturated_traffic_never_collides_or_locks_up() {
        let mut sim = Simulation::new();
        let mut passed_at_last_check = 0;
        run_checked(&mut sim, 120.0, |sim, frame| {
            // Rotate which turn gets first go, so the per-approach cooldown
            // does not favour one lane.
            for from in Direction::ALL {
                for k in 0..3 {
                    sim.spawn(from, Turn::ALL[(frame + k) % 3]);
                }
            }
            if frame > 0 && frame % 600 == 0 {
                let passed = sim.stats.vehicles_passed;
                assert!(passed > passed_at_last_check + 20, "only {} passed in 10s at t={}", passed - passed_at_last_check, sim.time);
                passed_at_last_check = passed;
            }
        });
        println!("saturated: {} passed", sim.stats.vehicles_passed);
    }

    /// The `R` key's rate of traffic flows without anybody queueing for long.
    #[test]
    fn random_traffic_flows() {
        let mut sim = Simulation::new();
        sim.rng = Rng::seeded(7);
        sim.random_mode = true;
        let longest = run_checked(&mut sim, 300.0, |_, _| {});
        println!("random: {} passed, longest trip {:.1}s", sim.stats.vehicles_passed, longest);
        assert!(longest < 10.0, "a vehicle took {:.1}s", longest);
    }
}

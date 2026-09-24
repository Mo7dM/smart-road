//! The world: every vehicle on the road, and the clock they all share.

use std::collections::HashSet;

use crate::config::{RANDOM_SPAWN_INTERVAL, SAFE_DISTANCE, VEHICLE_LENGTH};
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
    /// Pairs of vehicle ids already counted as a close call, so a pair that
    /// stays too close for several frames counts once.
    close_pairs: HashSet<(u32, u32)>,
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
            close_pairs: HashSet::new(),
        }
    }

    /// Put a vehicle on a named route. Silently does nothing if the lane is
    /// not ready for one.
    pub fn spawn(&mut self, from: Direction, turn: Turn) {
        let Simulation {
            vehicles,
            spawner,
            stats,
            time,
            ..
        } = self;
        if let Some(vehicle) = spawner.try_spawn(from, turn, *time, vehicles) {
            vehicles.push(vehicle);
            stats.vehicles_spawned += 1;
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

        for vehicle in &mut self.vehicles {
            vehicle.update(dt);
            self.stats.observe(vehicle);
        }
        self.stats.observe_traffic(self.vehicles.len());

        for pair in too_close(&self.vehicles, &self.conflicts) {
            if self.close_pairs.insert(pair) {
                self.stats.close_calls += 1;
            }
        }

        // Retire everything that has driven off the canvas.
        let stats = &mut self.stats;
        self.vehicles.retain(|vehicle| {
            if vehicle.exited {
                stats.record_exit(vehicle);
                false
            } else {
                true
            }
        });
        let vehicles = &self.vehicles;
        let alive = |id: u32| vehicles.iter().any(|v| v.id == id);
        self.close_pairs.retain(|&(a, b)| alive(a) && alive(b));
    }
}

/// Every pair of vehicles, by id, that is breaking the safe distance right
/// now: in the same lane, a gap under `SAFE_DISTANCE`; on routes that cross,
/// both inside the stretch where they meet at once. The controller keeps
/// this empty, so anything here is a close call.
fn too_close(vehicles: &[Vehicle], conflicts: &Conflicts) -> Vec<(u32, u32)> {
    // Rounding slack, so vehicles exactly the safe distance apart do not count.
    const SLACK: f32 = 0.01;
    let mut pairs = Vec::new();
    for (i, a) in vehicles.iter().enumerate() {
        for b in &vehicles[i + 1..] {
            let close = if a.route == b.route {
                let gap = (a.distance_traveled - b.distance_traveled).abs() - VEHICLE_LENGTH;
                gap < SAFE_DISTANCE - SLACK
            } else {
                conflicts.both_inside(a.route, a.distance_traveled, b.route, b.distance_traveled)
            };
            if close {
                pairs.push((a.id.min(b.id), a.id.max(b.id)));
            }
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SPEED_FAST, VEHICLE_WIDTH};
    use crate::vehicle::Route;

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
                longest_trip = longest_trip.max(a.time);
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
        assert_eq!(sim.stats.close_calls, 0);
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
        assert_eq!(sim.stats.close_calls, 0);
    }

    /// Run until the road is empty again.
    fn drain(sim: &mut Simulation) {
        run_checked(sim, 30.0, |_, _| {});
        assert!(sim.vehicles.is_empty(), "still on the road after 30s");
    }

    /// Audit: two vehicles from the south and two from the west, then quit.
    /// The statistics must say exactly 4 passed.
    #[test]
    fn four_vehicles_are_counted() {
        let mut sim = Simulation::new();
        sim.rng = Rng::seeded(3);
        let mut presses = [Direction::South, Direction::West, Direction::South, Direction::West].into_iter();
        run_checked(&mut sim, 3.0, |sim, frame| {
            // A key press every 0.7 seconds: past the spawn cooldown.
            if frame % 42 == 0 {
                if let Some(from) = presses.next() {
                    sim.spawn_from(from);
                }
            }
        });
        drain(&mut sim);
        assert_eq!(sim.stats.vehicles_spawned, 4);
        assert_eq!(sim.stats.vehicles_passed, 4);
        assert_eq!(sim.stats.close_calls, 0);
    }

    /// Audit: a single vehicle's time is both the max and the min, and it is
    /// the time it spends on screen: its path length at Fast.
    #[test]
    fn one_vehicle_times_itself() {
        let mut sim = Simulation::new();
        let route = Route { from: Direction::North, turn: Turn::Straight };
        sim.spawn(route.from, route.turn);
        let expected = sim.vehicles[0].path.total_len / SPEED_FAST;
        drain(&mut sim);

        assert_eq!(sim.stats.vehicles_passed, 1);
        assert_eq!(sim.stats.max_time, sim.stats.min_time);
        assert!((sim.stats.max_time - expected).abs() < 1.0 / 30.0, "{} vs {}", sim.stats.max_time, expected);
        assert_eq!(sim.stats.max_velocity, SPEED_FAST);
    }

    #[test]
    fn close_calls_are_counted_once_per_pair() {
        let conflicts = Conflicts::new();
        let south = Route { from: Direction::North, turn: Turn::Straight };
        let east = Route { from: Direction::West, turn: Turn::Straight };
        let middle = |on: Route, of: Route| {
            let (lo, hi) = conflicts.zone(on, of).unwrap();
            (lo + hi) / 2.0
        };

        let mut a = Vehicle::new(1, south);
        a.distance_traveled = middle(south, east);
        let mut b = Vehicle::new(2, east);
        b.distance_traveled = middle(east, south);
        let mut c = Vehicle::new(3, south);
        c.distance_traveled = a.distance_traveled - VEHICLE_LENGTH - SAFE_DISTANCE + 5.0;
        assert_eq!(too_close(&[a, b, c], &conflicts), [(1, 2), (1, 3)]);

        let far = Vehicle::new(4, south);
        let other = Vehicle::new(5, east);
        assert!(too_close(&[far, other], &conflicts).is_empty());
    }
}

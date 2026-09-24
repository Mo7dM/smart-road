//! The smart intersection algorithm: every frame, decide how fast each
//! vehicle should drive.
//!
//! Two rules, both in terms of distance along a path:
//!
//! - **Following.** Never close to within `SAFE_DISTANCE` of the vehicle
//!   ahead in the same lane.
//! - **Crossing.** Routes that cross share a conflict zone (see
//!   `conflict.rs`). A vehicle only passes its stop line at Fast, and only
//!   when that keeps it out of every zone while another vehicle is in the
//!   matching one. Past the line it is committed and holds Fast until it is
//!   out of its last zone, so the predictions everyone else made about it
//!   hold.
//!
//! On the approach, a vehicle picks the fastest speed that brings it to the
//! line in time to cross into a gap: Fast when the way is clear, Medium or
//! Slow to arrive just after someone else, and a stop at the line when no
//! gap is in sight. Crawling through the box instead would hold every zone
//! it touches for longer, and the vehicles behind would have to crawl too.
//!
//! First come, first served: an older vehicle's plan is fixed before a
//! younger one looks for a gap around it.

use crate::config::{CROSSING_MARGIN, FOLLOW_HORIZON, SAFE_DISTANCE, SPEED_FAST, VEHICLE_LENGTH};
use crate::conflict::{Conflicts, Phase};
use crate::vehicle::{Route, Speed, Vehicle};

/// Fastest first, so the first one that works is the one picked.
const MOVING: [Speed; 3] = [Speed::Fast, Speed::Medium, Speed::Slow];

/// Set every vehicle's `target_speed` for a frame of `dt` seconds.
pub fn update(vehicles: &mut [Vehicle], conflicts: &Conflicts, dt: f32) {
    let leaders = lane_leaders(vehicles);
    let phases: Vec<Phase> = vehicles
        .iter()
        .map(|v| conflicts.phase(v.route, v.distance_traveled))
        .collect();

    // Committed vehicles first: their speed is already decided. Then clear
    // ones front to back, so each sees the speed its leader was just given.
    // Then approaching ones, oldest first — which also puts every leader
    // before its follower, since nobody overtakes.
    let rank = |phase: Phase| match phase {
        Phase::Committed => 0,
        Phase::Clear => 1,
        Phase::Approaching => 2,
    };
    let mut order: Vec<usize> = (0..vehicles.len()).collect();
    order.sort_by(|&a, &b| {
        let (va, vb) = (&vehicles[a], &vehicles[b]);
        rank(phases[a]).cmp(&rank(phases[b])).then_with(|| match phases[a] {
            Phase::Clear => vb.distance_traveled.total_cmp(&va.distance_traveled),
            _ => va.id.cmp(&vb.id),
        })
    });

    let mut claims: Vec<Claim> = Vec::new();
    for i in order {
        let vehicle = &vehicles[i];
        let route = vehicle.route;
        let distance = vehicle.distance_traveled;

        let follow = match leaders[i] {
            None => Speed::Fast,
            Some(j) => {
                let ahead = &vehicles[j];
                // Two vehicles holding the same speed exactly the safe
                // distance apart drift by float rounding. That must not read
                // as closing in, or a vehicle could drop below Fast in the
                // box behind one that is leaving it.
                const ROUNDING: f32 = 1e-3;
                let gap = ahead.distance_traveled - distance - VEHICLE_LENGTH;
                speed_before(gap - SAFE_DISTANCE + ROUNDING, ahead.target_speed.value(), dt)
            }
        };

        let speed = match phases[i] {
            Phase::Committed => {
                // Everything ahead of it is past the line too, so at Fast as
                // well: holding Fast can never close on the vehicle in front.
                debug_assert_eq!(vehicle.target_speed, Speed::Fast);
                claims.push(Claim { route, distance, approach: Some(SPEED_FAST) });
                continue;
            }
            Phase::Clear => follow,
            Phase::Approaching => {
                let stop_line = conflicts.stop_line(route);
                let plan = MOVING
                    .into_iter()
                    .filter(|&s| s <= follow)
                    .filter(|&s| s == Speed::Fast || distance + s.value() * dt <= stop_line)
                    .find(|&s| {
                        let me = Claim { route, distance, approach: Some(s.value()) };
                        claims.iter().all(|c| me.keeps_clear_of(c, conflicts))
                    });

                match plan {
                    Some(s) => {
                        claims.push(Claim { route, distance, approach: Some(s.value()) });
                        s
                    }
                    None => {
                        claims.push(Claim { route, distance, approach: None });
                        follow.min(speed_before(stop_line - distance, 0.0, dt))
                    }
                }
            }
        };
        vehicles[i].target_speed = speed;
    }
}

/// A vehicle's hold on the conflict zones ahead of it, as seen by every
/// vehicle that plans after it.
struct Claim {
    route: Route,
    distance: f32,
    /// The speed it drives at up to its stop line, before crossing at Fast.
    /// `None` while it waits for a gap and has no plan.
    approach: Option<f32>,
}

impl Claim {
    /// When the vehicle will be inside `zone`, in seconds from now. `None`
    /// once it is past it. A waiting vehicle could get there as soon as
    /// driving Fast allows, and nobody knows when it will leave.
    fn window(&self, zone: (f32, f32), conflicts: &Conflicts) -> Option<(f32, f32)> {
        if self.distance > zone.1 {
            return None;
        }
        let approach = self.approach.unwrap_or(SPEED_FAST);
        // Where it switches to Fast: the stop line, or here if already past.
        let line = conflicts.stop_line(self.route).max(self.distance);
        let time_to = |p: f32| {
            if p <= line {
                (p - self.distance) / approach
            } else {
                (line - self.distance) / approach + (p - line) / SPEED_FAST
            }
        };
        let enter = time_to(zone.0).max(0.0);
        let leave = if self.approach.is_some() { time_to(zone.1) } else { f32::INFINITY };
        Some((enter, leave))
    }

    /// True when the two vehicles are never in their matching conflict zones
    /// within `CROSSING_MARGIN` of each other.
    fn keeps_clear_of(&self, other: &Claim, conflicts: &Conflicts) -> bool {
        let (Some(mine), Some(theirs)) = (
            conflicts.zone(self.route, other.route),
            conflicts.zone(other.route, self.route),
        ) else {
            return true;
        };
        let (Some(a), Some(b)) = (self.window(mine, conflicts), other.window(theirs, conflicts)) else {
            return true;
        };
        a.1 + CROSSING_MARGIN <= b.0 || b.1 + CROSSING_MARGIN <= a.0
    }
}

/// For each vehicle, the index of the one directly ahead of it in its lane.
/// A lane carries exactly one route, so that is the same route.
fn lane_leaders(vehicles: &[Vehicle]) -> Vec<Option<usize>> {
    let mut by_lane: Vec<usize> = (0..vehicles.len()).collect();
    by_lane.sort_by(|&a, &b| {
        let (va, vb) = (&vehicles[a], &vehicles[b]);
        va.route
            .index()
            .cmp(&vb.route.index())
            .then(vb.distance_traveled.total_cmp(&va.distance_traveled))
    });

    let mut leaders = vec![None; vehicles.len()];
    for pair in by_lane.windows(2) {
        if vehicles[pair[0]].route == vehicles[pair[1]].route {
            leaders[pair[1]] = Some(pair[0]);
        }
    }
    leaders
}

/// The fastest speed that keeps a vehicle behind a limit `room` ahead of it,
/// which itself moves at `limit_velocity`: the safe distance behind the
/// vehicle in front, or a stop line.
///
/// It prefers a speed that holds for `FOLLOW_HORIZON`, so a vehicle closing
/// in steps down through the speeds early. Holding a speed safely over the
/// horizon is also safe over any shorter frame. Past that it creeps at Slow
/// while one more frame fits, and only then stops.
fn speed_before(room: f32, limit_velocity: f32, dt: f32) -> Speed {
    let fits = |speed: Speed, horizon: f32| room + (limit_velocity - speed.value()) * horizon >= 0.0;
    MOVING
        .into_iter()
        .find(|&s| fits(s, FOLLOW_HORIZON))
        .or_else(|| fits(Speed::Slow, dt).then_some(Speed::Slow))
        .unwrap_or(Speed::Stopped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SPEED_SLOW;
    use crate::vehicle::{Direction, Turn};

    const DT: f32 = 1.0 / 60.0;

    fn vehicle(id: u32, from: Direction, turn: Turn, distance: f32) -> Vehicle {
        let mut v = Vehicle::new(id, Route { from, turn }, 0.0);
        v.distance_traveled = distance;
        v
    }

    /// A fast vehicle catching up with a slow one steps down through every
    /// speed, and never gets closer than the safe distance.
    #[test]
    fn follower_slows_down_without_breaking_the_gap() {
        let mut gap = 400.0;
        let mut seen = Vec::new();

        for _ in 0..(20.0 / DT) as u32 {
            let speed = speed_before(gap - SAFE_DISTANCE, SPEED_SLOW, DT);
            if seen.last() != Some(&speed) {
                seen.push(speed);
            }
            gap += (SPEED_SLOW - speed.value()) * DT;
            assert!(gap >= SAFE_DISTANCE, "gap fell to {}", gap);
        }

        assert_eq!(seen, [Speed::Fast, Speed::Medium, Speed::Slow]);
    }

    /// Driving at a line steps down the same way, and stops just short of it.
    #[test]
    fn stops_at_the_line_without_crossing_it() {
        let line = 500.0;
        let mut at = 0.0;
        for _ in 0..(20.0 / DT) as u32 {
            at += speed_before(line - at, 0.0, DT).value() * DT;
            assert!(at <= line);
        }
        assert!(line - at < SPEED_SLOW * DT, "stopped {} short", line - at);
    }

    #[test]
    fn only_vehicles_in_the_same_lane_are_followed() {
        let conflicts = Conflicts::new();
        // Deliberately out of order in the list: the follower comes first.
        let mut vehicles = vec![
            vehicle(1, Direction::North, Turn::Straight, 100.0),
            vehicle(0, Direction::North, Turn::Left, 140.0),
        ];
        update(&mut vehicles, &conflicts, DT);
        assert_eq!(vehicles[0].target_speed, Speed::Fast, "a vehicle in the next lane is not a leader");

        vehicles.push(vehicle(0, Direction::North, Turn::Straight, 140.0));
        update(&mut vehicles, &conflicts, DT);
        assert_eq!(vehicles[2].target_speed, Speed::Fast);
        assert_eq!(vehicles[0].target_speed, Speed::Medium);
    }

    /// Two vehicles that would meet in the box: the older one keeps going,
    /// the younger one slows to arrive after it.
    #[test]
    fn crossing_vehicles_take_turns() {
        let conflicts = Conflicts::new();
        let south = Route { from: Direction::North, turn: Turn::Straight };
        let east = Route { from: Direction::West, turn: Turn::Straight };
        let before = |r: Route| conflicts.stop_line(r) - 100.0;

        let mut vehicles = vec![
            vehicle(1, east.from, east.turn, before(east)),
            vehicle(0, south.from, south.turn, before(south)),
        ];
        update(&mut vehicles, &conflicts, DT);
        assert_eq!(vehicles[1].target_speed, Speed::Fast);
        assert!(vehicles[0].target_speed < Speed::Fast);
        assert!(vehicles[0].target_speed > Speed::Stopped, "it can time its arrival instead of stopping");
    }

    /// Right at the line, crossing at anything but Fast is not an option:
    /// the vehicle waits instead.
    #[test]
    fn a_vehicle_at_the_line_crosses_fast_or_waits() {
        let conflicts = Conflicts::new();
        let south = Route { from: Direction::North, turn: Turn::Straight };
        let east = Route { from: Direction::West, turn: Turn::Straight };

        let mut vehicles = vec![
            vehicle(1, east.from, east.turn, conflicts.stop_line(east) - 0.5),
            vehicle(0, south.from, south.turn, conflicts.stop_line(south) + 1.0),
        ];
        vehicles[1].target_speed = Speed::Fast;
        update(&mut vehicles, &conflicts, DT);
        assert_eq!(vehicles[0].target_speed, Speed::Stopped);
    }
}

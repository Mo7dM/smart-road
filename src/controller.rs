//! The smart intersection algorithm: every frame, decide how fast each
//! vehicle should drive.

use std::collections::HashMap;

use crate::config::{FOLLOW_HORIZON, SAFE_DISTANCE, VEHICLE_LENGTH};
use crate::vehicle::{Route, Speed, Vehicle};

/// Fastest first, so the first safe one is the one picked.
const SPEEDS: [Speed; 3] = [Speed::Fast, Speed::Medium, Speed::Slow];

/// Set every vehicle's `target_speed` for this frame.
pub fn update(vehicles: &mut [Vehicle]) {
    // Front to back, so that a follower sees the speed its leader has just
    // been given for this frame, not the one it drove at last frame.
    let mut order: Vec<usize> = (0..vehicles.len()).collect();
    order.sort_by(|&a, &b| {
        vehicles[b]
            .distance_traveled
            .total_cmp(&vehicles[a].distance_traveled)
    });

    // A lane carries exactly one route, so the route names the lane. For
    // each lane, the distance and new velocity of the rearmost vehicle seen
    // so far — the leader of the next one back.
    let mut last_in_lane: HashMap<Route, (f32, f32)> = HashMap::new();

    for i in order {
        let vehicle = &mut vehicles[i];
        let speed = match last_in_lane.get(&vehicle.route) {
            None => Speed::Fast,
            Some(&(leader_at, leader_velocity)) => {
                let gap = leader_at - vehicle.distance_traveled - VEHICLE_LENGTH;
                following_speed(gap, leader_velocity)
            }
        };
        vehicle.target_speed = speed;
        last_in_lane.insert(vehicle.route, (vehicle.distance_traveled, speed.value()));
    }
}

/// The fastest speed that still leaves `SAFE_DISTANCE` to the vehicle ahead
/// after `FOLLOW_HORIZON` seconds, if it keeps `leader_velocity`. `gap` is
/// bumper to bumper.
///
/// Holding a speed that is safe over the horizon is also safe over any
/// shorter frame, so the gap never drops below `SAFE_DISTANCE` in between.
fn following_speed(gap: f32, leader_velocity: f32) -> Speed {
    SPEEDS
        .into_iter()
        .find(|speed| gap + (leader_velocity - speed.value()) * FOLLOW_HORIZON >= SAFE_DISTANCE)
        // Nothing fits only when the leader is slower than Slow. Slow is the
        // floor until vehicles can stop.
        .unwrap_or(Speed::Slow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SPEED_SLOW;
    use crate::vehicle::{Direction, Turn};

    /// A fast vehicle catching up with a slow one steps down through every
    /// speed, and never gets closer than the safe distance.
    #[test]
    fn follower_slows_down_without_breaking_the_gap() {
        let dt = 1.0 / 60.0;
        let mut gap = 400.0;
        let mut seen = Vec::new();

        for _ in 0..(20.0 / dt) as u32 {
            let speed = following_speed(gap, SPEED_SLOW);
            if seen.last() != Some(&speed) {
                seen.push(speed);
            }
            gap += (SPEED_SLOW - speed.value()) * dt;
            assert!(gap >= SAFE_DISTANCE, "gap fell to {}", gap);
        }

        assert_eq!(seen, [Speed::Fast, Speed::Medium, Speed::Slow]);
    }

    #[test]
    fn only_vehicles_in_the_same_lane_are_followed() {
        let lane = Route { from: Direction::North, turn: Turn::Straight };
        let other = Route { from: Direction::North, turn: Turn::Left };

        // Deliberately out of order in the list: the follower comes first.
        let mut follower = Vehicle::new(0, lane, 0.0);
        follower.distance_traveled = 100.0;
        let mut beside = Vehicle::new(1, other, 0.0);
        beside.distance_traveled = 140.0;

        let mut vehicles = vec![follower, beside];
        update(&mut vehicles);
        assert_eq!(vehicles[0].target_speed, Speed::Fast, "a vehicle in the next lane is not a leader");

        let mut leader = Vehicle::new(2, lane, 0.0);
        leader.distance_traveled = 140.0;
        vehicles.push(leader);
        update(&mut vehicles);
        assert_eq!(vehicles[2].target_speed, Speed::Fast);
        assert_eq!(vehicles[0].target_speed, Speed::Medium);
    }
}

//! Where routes cross, worked out once from the paths.
//!
//! Two vehicles on different routes can only touch while each is inside a
//! particular stretch of its own path. Those stretches are distance ranges,
//! so at run time "could these two collide?" becomes "are both inside their
//! range at the same time?" — the controller never looks at (x, y).

use std::f32::consts::SQRT_2;

use crate::config::{CROSSING_CLEARANCE, VEHICLE_LENGTH, VEHICLE_WIDTH};
use crate::path::Path;
use crate::vehicle::Route;

/// Distance between samples along a path, in pixels. Every range is widened
/// by one step on both sides, so sampling can only make it larger.
const STEP: f32 = 2.0;

/// A vehicle's body is modelled as two discs on its centre line, one towards
/// each end. With this offset and radius they cover the whole rectangle,
/// corners included, plus the clearance.
const DISC_OFFSET: f32 = (VEHICLE_LENGTH - VEHICLE_WIDTH) / 2.0;
const DISC_RADIUS: f32 = VEHICLE_WIDTH / 2.0 * SQRT_2 + CROSSING_CLEARANCE;

/// Where a vehicle is relative to the conflict zones on its route.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Before the stop line, or right on it: free to wait for a gap.
    Approaching,
    /// Past the stop line and not yet out of its last zone.
    Committed,
    /// Past every zone, or on a route that crosses nothing.
    Clear,
}

pub struct Conflicts {
    /// `zones[a][b]`: the range of distances along route `a` within which a
    /// vehicle on it can touch one on route `b`.
    zones: [[Option<(f32, f32)>; Route::COUNT]; Route::COUNT],
    /// Distance at which a vehicle on each route must stop if it may not go.
    stop_line: [f32; Route::COUNT],
    /// Distance past which a vehicle on each route is out of every zone.
    clear_at: [f32; Route::COUNT],
}

impl Conflicts {
    pub fn new() -> Conflicts {
        let paths: Vec<Path> = Route::all().map(Path::new).collect();
        let bodies: Vec<Vec<(f32, [(f32, f32); 2])>> = paths
            .iter()
            .map(|path| {
                sample_range(path.box_enter - VEHICLE_LENGTH, path.box_exit + VEHICLE_LENGTH)
                    .map(|s| (s, discs(path, s)))
                    .collect()
            })
            .collect();

        let mut zones = [[None; Route::COUNT]; Route::COUNT];
        for a in 0..Route::COUNT {
            for b in a + 1..Route::COUNT {
                let mut on_a: Option<(f32, f32)> = None;
                let mut on_b: Option<(f32, f32)> = None;
                for &(sa, da) in &bodies[a] {
                    for &(sb, db) in &bodies[b] {
                        if touching(da, db) {
                            on_a = Some(extend(on_a, sa));
                            on_b = Some(extend(on_b, sb));
                        }
                    }
                }
                zones[a][b] = on_a.map(widen);
                zones[b][a] = on_b.map(widen);
            }
        }

        // A route that crosses nothing never has to stop, and is clear from
        // the start.
        let mut stop_line = [f32::NEG_INFINITY; Route::COUNT];
        let mut clear_at = [f32::NEG_INFINITY; Route::COUNT];
        for a in 0..Route::COUNT {
            let mine = zones[a].iter().flatten();
            if let Some(first) = mine.clone().map(|z| z.0).reduce(f32::min) {
                // Stop with the front bumper at the edge of the box, or
                // earlier if a zone reaches out past it.
                stop_line[a] = first.min(paths[a].box_enter - VEHICLE_LENGTH / 2.0);
                clear_at[a] = mine.map(|z| z.1).reduce(f32::max).unwrap();
            }
        }

        Conflicts { zones, stop_line, clear_at }
    }

    /// The stretch of `on`'s path where it can touch a vehicle on `against`.
    pub fn zone(&self, on: Route, against: Route) -> Option<(f32, f32)> {
        self.zones[on.index()][against.index()]
    }

    /// True when vehicles at these distances on these routes are both inside
    /// the stretch where their routes meet: close enough to touch.
    pub fn both_inside(&self, a: Route, at_a: f32, b: Route, at_b: f32) -> bool {
        let inside = |zone: Option<(f32, f32)>, at: f32| zone.is_some_and(|(lo, hi)| at >= lo && at <= hi);
        inside(self.zone(a, b), at_a) && inside(self.zone(b, a), at_b)
    }

    pub fn stop_line(&self, route: Route) -> f32 {
        self.stop_line[route.index()]
    }

    pub fn phase(&self, route: Route, distance: f32) -> Phase {
        if distance > self.clear_at[route.index()] {
            Phase::Clear
        } else if distance > self.stop_line[route.index()] {
            Phase::Committed
        } else {
            Phase::Approaching
        }
    }
}

fn sample_range(from: f32, to: f32) -> impl Iterator<Item = f32> {
    let steps = ((to - from) / STEP).ceil() as u32;
    (0..=steps).map(move |i| from + i as f32 * STEP)
}

/// Centres of the two discs that cover a vehicle at distance `s`.
fn discs(path: &Path, s: f32) -> [(f32, f32); 2] {
    let ((x, y), angle) = path.sample(s);
    let rad = (angle as f32).to_radians();
    let (dx, dy) = (rad.cos() * DISC_OFFSET, rad.sin() * DISC_OFFSET);
    [(x + dx, y + dy), (x - dx, y - dy)]
}

fn touching(a: [(f32, f32); 2], b: [(f32, f32); 2]) -> bool {
    let reach = 2.0 * DISC_RADIUS;
    a.iter().any(|p| {
        b.iter()
            .any(|q| (p.0 - q.0).powi(2) + (p.1 - q.1).powi(2) < reach * reach)
    })
}

fn extend(range: Option<(f32, f32)>, s: f32) -> (f32, f32) {
    match range {
        None => (s, s),
        Some((lo, hi)) => (lo.min(s), hi.max(s)),
    }
}

fn widen((lo, hi): (f32, f32)) -> (f32, f32) {
    (lo - STEP, hi + STEP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::{Direction, Turn};

    fn route(from: Direction, turn: Turn) -> Route {
        Route { from, turn }
    }

    #[test]
    fn conflicts_are_symmetric() {
        let conflicts = Conflicts::new();
        for a in Route::all() {
            for b in Route::all() {
                assert_eq!(
                    conflicts.zone(a, b).is_some(),
                    conflicts.zone(b, a).is_some(),
                    "{:?} / {:?}",
                    a,
                    b
                );
            }
        }
    }

    /// Lanes that run side by side, on the same approach, never cross.
    #[test]
    fn routes_from_the_same_side_never_conflict() {
        let conflicts = Conflicts::new();
        for from in Direction::ALL {
            for a in Turn::ALL {
                for b in Turn::ALL {
                    assert!(conflicts.zone(route(from, a), route(from, b)).is_none());
                }
            }
        }
    }

    /// A right turn stays in its own corner of the box, so it never waits.
    #[test]
    fn right_turns_cross_nothing() {
        let conflicts = Conflicts::new();
        for from in Direction::ALL {
            let right = route(from, Turn::Right);
            for other in Route::all() {
                assert!(conflicts.zone(right, other).is_none(), "{:?} crosses {:?}", right, other);
            }
            assert_eq!(conflicts.phase(right, 0.0), Phase::Clear);
        }
    }

    #[test]
    fn perpendicular_straight_routes_conflict() {
        let conflicts = Conflicts::new();
        let south = route(Direction::North, Turn::Straight);
        assert!(conflicts.zone(south, route(Direction::West, Turn::Straight)).is_some());
        assert!(conflicts.zone(south, route(Direction::East, Turn::Straight)).is_some());
        assert!(conflicts.zone(south, route(Direction::South, Turn::Straight)).is_none());
    }

    /// Conflicts are only looked for near the box. That is enough because
    /// everywhere else vehicles drive straight in lanes far enough apart
    /// never to touch — which this checks against the full paths.
    #[test]
    fn routes_only_touch_near_the_box() {
        let paths: Vec<Path> = Route::all().map(Path::new).collect();
        let near = |p: &Path, s: f32| s >= p.box_enter - VEHICLE_LENGTH && s <= p.box_exit + VEHICLE_LENGTH;
        for a in 0..Route::COUNT {
            for b in a + 1..Route::COUNT {
                for sa in sample_range(0.0, paths[a].total_len).step_by(2) {
                    for sb in sample_range(0.0, paths[b].total_len).step_by(2) {
                        if touching(discs(&paths[a], sa), discs(&paths[b], sb)) {
                            assert!(
                                near(&paths[a], sa) && near(&paths[b], sb),
                                "routes {} and {} touch away from the box at {} / {}",
                                a,
                                b,
                                sa,
                                sb
                            );
                        }
                    }
                }
            }
        }
    }
}

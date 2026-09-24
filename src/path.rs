//! Lane geometry and the path a vehicle follows through the intersection.
//!
//! A path is measured by arc length: one number, the distance travelled from
//! the spawn point, is enough to place a vehicle and point it the right way.
//! It has three phases — a straight approach, a quarter-circle turn, and a
//! straight exit — and a straight route is just the same thing with a turn of
//! radius zero.

use std::f32::consts::PI;

use crate::config::*;
use crate::vehicle::{Direction, Route, Turn};

/// Centre line coordinate of the lane a route approaches on: an x for the
/// vertical road, a y for the horizontal one.
pub fn entry_lane_coord(route: Route) -> f32 {
    let heading = route.entry_heading();
    let center = if heading.is_vertical() { CENTER_X } else { CENTER_Y };
    center + heading.side_sign() * route.turn.lane_offset()
}

/// Centre line coordinate of the lane a route leaves on, after its turn.
pub fn exit_lane_coord(route: Route) -> f32 {
    let heading = route.exit_heading();
    let center = if heading.is_vertical() { CENTER_X } else { CENTER_Y };
    center + heading.side_sign() * route.turn.lane_offset()
}

/// Full (x, y) of a point on the approach lane, `along` measured in screen
/// coordinates on the axis the vehicle travels.
pub fn entry_point(route: Route, along: f32) -> (f32, f32) {
    let lane = entry_lane_coord(route);
    if route.entry_heading().is_vertical() {
        (lane, along)
    } else {
        (along, lane)
    }
}

/// Where a vehicle appears: just outside the window, on its approach lane.
pub fn spawn_point(route: Route) -> (f32, f32) {
    let margin = VEHICLE_LENGTH;
    let along = match route.from {
        Direction::North | Direction::West => -margin,
        Direction::South => WINDOW_HEIGHT as f32 + margin,
        Direction::East => WINDOW_WIDTH as f32 + margin,
    };
    entry_point(route, along)
}

/// The point inside the intersection where the approach lane and the exit
/// lane cross — the corner a turning vehicle pivots around. For a straight
/// route it is simply the middle of the intersection on that lane.
pub fn turn_point(route: Route) -> (f32, f32) {
    let entry = entry_lane_coord(route);
    let exit = exit_lane_coord(route);
    if route.entry_heading().is_vertical() {
        match route.turn {
            Turn::Straight => (entry, CENTER_Y),
            _ => (entry, exit),
        }
    } else {
        match route.turn {
            Turn::Straight => (CENTER_X, entry),
            _ => (exit, entry),
        }
    }
}

/// Coordinate of the edge of the intersection box that a heading reaches
/// first, on the axis that heading travels along.
pub fn near_edge(heading: Direction) -> f32 {
    match heading {
        Direction::South => BOX_TOP,
        Direction::North => BOX_BOTTOM,
        Direction::East => BOX_LEFT,
        Direction::West => BOX_RIGHT,
    }
}

/// The component of a heading's unit vector on the axis it travels: +1 or -1.
fn along_unit(heading: Direction) -> f32 {
    let (ux, uy) = heading.unit();
    if heading.is_vertical() { uy } else { ux }
}

/// The coordinate of a point on the axis `heading` travels along.
fn along_of(heading: Direction, p: (f32, f32)) -> f32 {
    if heading.is_vertical() { p.1 } else { p.0 }
}

/// Unit vector 90 degrees to the right of a heading, in screen coordinates.
fn right_of(heading: Direction) -> (f32, f32) {
    let (x, y) = heading.unit();
    (-y, x)
}

struct Arc {
    center: (f32, f32),
    /// Vector from the centre to the point where the arc begins.
    spoke: (f32, f32),
    /// Total rotation in degrees: +90 turning right, -90 turning left.
    sweep: f32,
    len: f32,
}

pub struct Path {
    start: (f32, f32),
    heading: Direction,
    exit_heading: Direction,
    /// Distance from the spawn point to where the turn begins.
    approach_len: f32,
    arc: Arc,
    /// Point where the turn ends and the straight exit begins.
    arc_end: (f32, f32),
    /// Total length of the path; past this the vehicle is off screen.
    pub total_len: f32,
    /// Distances at which the vehicle enters and leaves the intersection box.
    pub box_enter: f32,
    pub box_exit: f32,
}

impl Path {
    pub fn new(route: Route) -> Path {
        let heading = route.entry_heading();
        let exit_heading = route.exit_heading();
        let start = spawn_point(route);
        let tp = turn_point(route);

        // The turn radius is not a free parameter: it is the distance from
        // the pivot to the edge of the box. That makes the arc start exactly
        // where the lane meets the intersection and end exactly where the
        // exit lane leaves it, so the curve is tangent to both lanes.
        let radius = match route.turn {
            Turn::Straight => 0.0,
            _ => (along_of(heading, tp) - near_edge(heading)).abs(),
        };

        let (hx, hy) = heading.unit();
        let (ex, ey) = exit_heading.unit();
        let arc_start = (tp.0 - hx * radius, tp.1 - hy * radius);
        let arc_end = (tp.0 + ex * radius, tp.1 + ey * radius);

        let sweep = match route.turn {
            Turn::Right => 90.0,
            Turn::Left => -90.0,
            Turn::Straight => 0.0,
        };
        // The centre of the turn sits one radius to the side the vehicle is
        // turning towards.
        let (nx, ny) = right_of(heading);
        let side = if sweep >= 0.0 { 1.0 } else { -1.0 };
        let center = (
            arc_start.0 + nx * radius * side,
            arc_start.1 + ny * radius * side,
        );
        let arc = Arc {
            center,
            spoke: (arc_start.0 - center.0, arc_start.1 - center.1),
            sweep,
            len: radius * PI / 2.0,
        };

        let approach_len = (along_of(heading, arc_start) - along_of(heading, start)).abs();

        // Run the exit straight until the vehicle is clear of the window.
        let far_edge = if exit_heading.is_vertical() {
            WINDOW_HEIGHT as f32
        } else {
            WINDOW_WIDTH as f32
        };
        let exit_target = if along_unit(exit_heading) > 0.0 {
            far_edge + VEHICLE_LENGTH
        } else {
            -VEHICLE_LENGTH
        };
        let exit_len = (exit_target - along_of(exit_heading, arc_end)).abs();

        // Where the vehicle occupies the intersection box. For a turn this is
        // exactly the arc; for a straight route it is the box's full width.
        let box_enter = (near_edge(heading) - along_of(heading, start)).abs();
        let arc_len = arc.len;
        let box_exit = match route.turn {
            Turn::Straight => box_enter + ROAD_WIDTH,
            _ => approach_len + arc_len,
        };

        Path {
            start,
            heading,
            exit_heading,
            approach_len,
            arc,
            arc_end,
            total_len: approach_len + arc_len + exit_len,
            box_enter,
            box_exit,
        }
    }

    /// Position and heading angle (degrees, clockwise, 0 = east) at distance
    /// `s` along the path.
    pub fn sample(&self, s: f32) -> ((f32, f32), f64) {
        if s <= self.approach_len {
            let (ux, uy) = self.heading.unit();
            return (
                (self.start.0 + ux * s, self.start.1 + uy * s),
                self.heading.angle(),
            );
        }

        let into_arc = s - self.approach_len;
        if into_arc < self.arc.len {
            let t = into_arc / self.arc.len;
            let turned = self.arc.sweep * t;
            let rad = turned.to_radians();
            let (sx, sy) = self.arc.spoke;
            // Rotating in screen coordinates, where y grows downwards.
            let x = sx * rad.cos() - sy * rad.sin();
            let y = sx * rad.sin() + sy * rad.cos();
            return (
                (self.arc.center.0 + x, self.arc.center.1 + y),
                self.heading.angle() + turned as f64,
            );
        }

        let past = into_arc - self.arc.len;
        let (ux, uy) = self.exit_heading.unit();
        (
            (self.arc_end.0 + ux * past, self.arc_end.1 + uy * past),
            self.exit_heading.angle(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_routes() -> Vec<Route> {
        let mut routes = Vec::new();
        for from in Direction::ALL {
            for turn in Turn::ALL {
                routes.push(Route { from, turn });
            }
        }
        routes
    }

    /// The coordinate across the direction of travel — the one that says
    /// which lane a vehicle is in.
    fn across(heading: Direction, p: (f32, f32)) -> f32 {
        if heading.is_vertical() { p.0 } else { p.1 }
    }

    /// Shortest distance between two headings, in degrees: 359 and 1 are two
    /// degrees apart, not 358.
    fn angle_gap(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    }

    #[test]
    fn approach_stays_in_its_lane() {
        for route in all_routes() {
            let path = Path::new(route);
            let heading = route.entry_heading();
            let mut s = 0.0;
            while s < path.box_enter {
                let (p, angle) = path.sample(s);
                assert!(
                    (across(heading, p) - entry_lane_coord(route)).abs() < 0.01,
                    "{:?} drifted out of its approach lane at {}",
                    route,
                    s
                );
                assert!(angle_gap(angle, heading.angle()) < 0.01);
                s += 5.0;
            }
        }
    }

    #[test]
    fn exit_lands_in_the_right_lane() {
        for route in all_routes() {
            let path = Path::new(route);
            let exit = route.exit_heading();
            let mut s = path.box_exit;
            while s <= path.total_len {
                let (p, angle) = path.sample(s);
                assert!(
                    (across(exit, p) - exit_lane_coord(route)).abs() < 0.01,
                    "{:?} left on the wrong lane at {}",
                    route,
                    s
                );
                assert!(
                    angle_gap(angle, exit.angle()) < 0.01,
                    "{:?} left pointing the wrong way: {}",
                    route,
                    angle
                );
                s += 5.0;
            }
        }
    }

    /// The turn must be a smooth curve, not a jump: stepping a fixed distance
    /// along the path must move the vehicle that same distance on screen.
    #[test]
    fn path_is_continuous() {
        let step = 1.0;
        for route in all_routes() {
            let path = Path::new(route);
            let mut s = 0.0;
            while s + step <= path.total_len {
                let (a, _) = path.sample(s);
                let (b, _) = path.sample(s + step);
                let moved = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                assert!(
                    (moved - step).abs() < 0.02,
                    "{:?} jumped {} px for a {} px step at {}",
                    route,
                    moved,
                    step,
                    s
                );
                s += step;
            }
        }
    }

    /// Turning must actually rotate the vehicle, and only during the turn.
    #[test]
    fn turns_rotate_a_quarter_circle() {
        for route in all_routes() {
            let path = Path::new(route);
            let (_, before) = path.sample(path.box_enter);
            let (_, after) = path.sample(path.box_exit);
            let turned = angle_gap(after, before);
            let expected = match route.turn {
                Turn::Straight => 0.0,
                _ => 90.0,
            };
            assert!(
                (turned - expected).abs() < 0.05,
                "{:?} rotated {} degrees, expected {}",
                route,
                turned,
                expected
            );
        }
    }

    #[test]
    fn vehicles_start_and_finish_off_screen() {
        for route in all_routes() {
            let path = Path::new(route);
            for s in [0.0, path.total_len] {
                let (p, _) = path.sample(s);
                let outside = p.0 < 0.0
                    || p.1 < 0.0
                    || p.0 > WINDOW_WIDTH as f32
                    || p.1 > WINDOW_HEIGHT as f32;
                assert!(outside, "{:?} is on screen at s = {}", route, s);
            }
        }
    }
}


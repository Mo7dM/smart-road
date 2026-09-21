//! Lane geometry: where each route enters, where it leaves, and the point at
//! which the two meet. Every coordinate in the simulation is derived here so
//! the rendering and the driving logic can never disagree.

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
        Direction::North => -margin,
        Direction::South => WINDOW_HEIGHT as f32 + margin,
        Direction::West => -margin,
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
        // Travelling on x = entry, leaving on y = exit.
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

// ─── Window ────────────────────────────────────────────────────────────────
pub const WINDOW_WIDTH: u32 = 900;
pub const WINDOW_HEIGHT: u32 = 850;

/// The statistics window shown when the simulation ends.
pub const STATS_WIDTH: u32 = 720;
pub const STATS_HEIGHT: u32 = 600;

// ─── Road geometry ─────────────────────────────────────────────────────────
// The intersection is a cross. Each road carries 3 incoming + 3 outgoing
// lanes, so a road is 6 lanes wide and both roads meet in a square box.

pub const LANE_WIDTH: f32 = 40.0;
pub const LANES_PER_SIDE: f32 = 3.0;

/// Half the width of a road: distance from its centre line to its outer edge.
pub const ROAD_HALF: f32 = LANE_WIDTH * LANES_PER_SIDE; // 120
pub const ROAD_WIDTH: f32 = ROAD_HALF * 2.0; // 240

/// Centre of the intersection (both roads cross here).
pub const CENTER_X: f32 = WINDOW_WIDTH as f32 / 2.0;
pub const CENTER_Y: f32 = WINDOW_HEIGHT as f32 / 2.0;

/// The intersection box, where the two roads overlap. Each road is centred
/// on its own axis, so the box is derived per axis and the window need not
/// be square.
pub const BOX_LEFT: f32 = CENTER_X - ROAD_HALF;
pub const BOX_RIGHT: f32 = CENTER_X + ROAD_HALF;
pub const BOX_TOP: f32 = CENTER_Y - ROAD_HALF;
pub const BOX_BOTTOM: f32 = CENTER_Y + ROAD_HALF;

/// Distance from a road's centre line to the centre of each lane, indexed by
/// turn: a right turn uses the outermost lane, a left turn the innermost.
pub const LANE_OFFSET_RIGHT: f32 = 100.0;
pub const LANE_OFFSET_STRAIGHT: f32 = 60.0;
pub const LANE_OFFSET_LEFT: f32 = 20.0;

// ─── Vehicles ──────────────────────────────────────────────────────────────
pub const VEHICLE_LENGTH: f32 = 32.0;
pub const VEHICLE_WIDTH: f32 = 20.0;

/// Gap a vehicle must keep from the one ahead, bumper to bumper.
pub const SAFE_DISTANCE: f32 = 50.0;

/// How far ahead, in seconds, a vehicle looks when picking its speed behind
/// another one: it only takes a speed that still leaves `SAFE_DISTANCE` after
/// this long. A horizon well past one frame makes a closing vehicle step down
/// Fast -> Medium -> Slow instead of braking at the last moment. It must be
/// at least the longest frame, or the gap could be broken within one.
pub const FOLLOW_HORIZON: f32 = 1.0;

/// Extra room, in pixels, kept around a vehicle's body when working out where
/// two routes can touch. Neighbouring lanes must still never count as
/// touching, which caps it at a few pixels; a test in `conflict.rs` checks it.
pub const CROSSING_CLEARANCE: f32 = 4.0;

/// Minimum time, in seconds, between one vehicle leaving a stretch of road
/// that two routes share and another one entering it.
pub const CROSSING_MARGIN: f32 = 0.25;

/// Minimum delay, in seconds, between two spawns on the same approach, so
/// that spamming a key never stacks vehicles on top of each other.
pub const SPAWN_COOLDOWN: f32 = 0.6;

/// How often the `R` key's continuous mode releases another vehicle.
pub const RANDOM_SPAWN_INTERVAL: f32 = 0.5;

// The three velocities a vehicle can be told to drive at, in pixels/second.
pub const SPEED_SLOW: f32 = 60.0;
pub const SPEED_MEDIUM: f32 = 120.0;
pub const SPEED_FAST: f32 = 200.0;

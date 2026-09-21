// ─── Window ────────────────────────────────────────────────────────────────
pub const WINDOW_WIDTH: u32 = 900;
pub const WINDOW_HEIGHT: u32 = 850;

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

/// The intersection box: x and y both inside [INTER_MIN, INTER_MAX].
pub const INTER_MIN: f32 = CENTER_X - ROAD_HALF; // 330
pub const INTER_MAX: f32 = CENTER_X + ROAD_HALF; // 570

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

/// Minimum delay between two spawns on the same approach, so that spamming a
/// key never stacks vehicles on top of each other.
pub const SPAWN_COOLDOWN_MS: u64 = 400;

// The three velocities a vehicle can be told to drive at, in pixels/second.
pub const SPEED_SLOW: f32 = 60.0;
pub const SPEED_MEDIUM: f32 = 120.0;
pub const SPEED_FAST: f32 = 200.0;

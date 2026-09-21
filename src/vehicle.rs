use std::time::Instant;

/// A cardinal direction. Used both as "the side a vehicle comes from" and as
/// "the way a vehicle is currently heading" — `Route::from` is the former,
/// everything else is the latter.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ];

    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    /// The heading a vehicle ends up with after taking `turn` while heading
    /// in `self`.
    pub fn apply_turn(self, turn: Turn) -> Direction {
        match turn {
            Turn::Straight => self,
            Turn::Left => match self {
                Direction::North => Direction::West,
                Direction::South => Direction::East,
                Direction::East => Direction::North,
                Direction::West => Direction::South,
            },
            Turn::Right => match self {
                Direction::North => Direction::East,
                Direction::South => Direction::West,
                Direction::East => Direction::South,
                Direction::West => Direction::North,
            },
        }
    }

    /// Unit vector of this heading in screen coordinates (y grows downwards).
    pub fn unit(self) -> (f32, f32) {
        match self {
            Direction::North => (0.0, -1.0),
            Direction::South => (0.0, 1.0),
            Direction::East => (1.0, 0.0),
            Direction::West => (-1.0, 0.0),
        }
    }

    /// True when traffic with this heading travels along the vertical road.
    pub fn is_vertical(self) -> bool {
        matches!(self, Direction::North | Direction::South)
    }

    /// Which half of the road traffic with this heading drives on, expressed
    /// as the sign of the offset from the road's centre line. Right-hand
    /// traffic: southbound keeps to the west half, northbound to the east
    /// half, westbound to the north half, eastbound to the south half.
    pub fn side_sign(self) -> f32 {
        match self {
            Direction::North | Direction::East => 1.0,
            Direction::South | Direction::West => -1.0,
        }
    }

    /// Angle of this heading in degrees, clockwise, 0 = pointing east.
    pub fn angle(self) -> f64 {
        match self {
            Direction::East => 0.0,
            Direction::South => 90.0,
            Direction::West => 180.0,
            Direction::North => 270.0,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Turn {
    Left,
    Straight,
    Right,
}

impl Turn {
    pub const ALL: [Turn; 3] = [Turn::Left, Turn::Straight, Turn::Right];

    /// Distance from the road's centre line to the centre of the lane that
    /// carries this turn.
    pub fn lane_offset(self) -> f32 {
        match self {
            Turn::Right => crate::config::LANE_OFFSET_RIGHT,
            Turn::Straight => crate::config::LANE_OFFSET_STRAIGHT,
            Turn::Left => crate::config::LANE_OFFSET_LEFT,
        }
    }
}

/// Where a vehicle comes from and what it does at the intersection. A lane
/// has exactly one route, and a vehicle can never change either.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    pub from: Direction,
    pub turn: Turn,
}

impl Route {
    /// Heading while approaching the intersection.
    pub fn entry_heading(self) -> Direction {
        self.from.opposite()
    }

    /// Heading after the turn is done.
    pub fn exit_heading(self) -> Direction {
        self.entry_heading().apply_turn(self.turn)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Speed {
    Slow,
    Medium,
    Fast,
}

impl Speed {
    pub fn value(self) -> f32 {
        match self {
            Speed::Slow => crate::config::SPEED_SLOW,
            Speed::Medium => crate::config::SPEED_MEDIUM,
            Speed::Fast => crate::config::SPEED_FAST,
        }
    }
}

pub struct Vehicle {
    pub id: u32,
    pub route: Route,
    pub position: (f32, f32),
    pub velocity: f32,
    pub distance_traveled: f32,
    pub spawn_time: Instant,
    pub entered_intersection_at: Option<Instant>,
    pub exited: bool,
}

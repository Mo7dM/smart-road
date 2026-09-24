use crate::path::Path;

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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Route {
    pub from: Direction,
    pub turn: Turn,
}

impl Route {
    pub const COUNT: usize = 12;

    /// Every route, in `index` order.
    pub fn all() -> impl Iterator<Item = Route> {
        Direction::ALL
            .into_iter()
            .flat_map(|from| Turn::ALL.into_iter().map(move |turn| Route { from, turn }))
    }

    /// A number in `0..COUNT`, for tables indexed by route.
    pub fn index(self) -> usize {
        self.from as usize * Turn::ALL.len() + self.turn as usize
    }

    /// Heading while approaching the intersection.
    pub fn entry_heading(self) -> Direction {
        self.from.opposite()
    }

    /// Heading after the turn is done.
    pub fn exit_heading(self) -> Direction {
        self.entry_heading().apply_turn(self.turn)
    }
}

/// Ordered slowest first, so speeds compare the way their values do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Speed {
    /// Waiting: behind a stopped vehicle, or at the stop line for a gap.
    Stopped,
    Slow,
    Medium,
    Fast,
}

impl Speed {
    pub fn value(self) -> f32 {
        match self {
            Speed::Stopped => 0.0,
            Speed::Slow => crate::config::SPEED_SLOW,
            Speed::Medium => crate::config::SPEED_MEDIUM,
            Speed::Fast => crate::config::SPEED_FAST,
        }
    }
}

/// One autonomous vehicle. Its whole state on the road is a single distance
/// along its path; position and heading are derived from it every frame.
///
/// `distance_traveled`, `time` and `velocity` are the physics the subject
/// asks for: the controller sets the velocity, and distance grows by
/// `velocity * dt` while time grows by `dt`.
pub struct Vehicle {
    pub id: u32,
    pub route: Route,
    pub path: Path,
    /// Distance covered since spawning, in pixels.
    pub distance_traveled: f32,
    /// Seconds since the intersection controller first saw this vehicle,
    /// which is the moment it spawned. When it leaves the canvas, this is the
    /// time it took to pass the intersection.
    pub time: f32,
    /// Current speed in pixels per second.
    pub velocity: f32,
    /// The speed the intersection controller wants this vehicle to drive at.
    pub target_speed: Speed,
    /// Seconds spent stopped, waiting behind another vehicle or at the line.
    pub waited: f32,
    pub position: (f32, f32),
    /// Heading in degrees, clockwise, 0 = east.
    pub angle: f64,
    pub exited: bool,
}

impl Vehicle {
    pub fn new(id: u32, route: Route) -> Vehicle {
        let path = Path::new(route);
        let (position, angle) = path.sample(0.0);
        Vehicle {
            id,
            route,
            path,
            distance_traveled: 0.0,
            time: 0.0,
            velocity: 0.0,
            target_speed: Speed::Fast,
            waited: 0.0,
            position,
            angle,
            exited: false,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.velocity = self.target_speed.value();
        self.distance_traveled += self.velocity * dt;
        self.time += dt;
        if self.target_speed == Speed::Stopped {
            self.waited += dt;
        }

        let (position, angle) = self.path.sample(self.distance_traveled);
        self.position = position;
        self.angle = angle;

        if self.distance_traveled >= self.path.total_len {
            self.exited = true;
        }
    }
}

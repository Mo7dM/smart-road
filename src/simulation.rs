use crate::vehicle::Vehicle;
use crate::spawner::Spawner;
use crate::stats::Statistics;

pub struct Simulation {
    pub vehicles: Vec<Vehicle>,
    pub spawner: Spawner,
    pub stats: Statistics,
    pub random_mode: bool,
}

use std::time::{Instant, Duration};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Direction { North, South, East, West }
#[derive(Clone, Copy)]
pub enum Turn { Left, Straight, Right }
pub struct Route {
    pub from: Direction,
    pub turn: Turn,
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

pub enum Speed { Slow, Medium, Fast }
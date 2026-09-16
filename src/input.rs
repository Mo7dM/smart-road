use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use crate::vehicle::Direction;

pub enum Command {
    Spawn(Direction),
    ToggleRandom,
    Quit,
}

pub fn handle_event(event: &Event) -> Option<Command> {
    match event {
        Event::Quit { .. } => Some(Command::Quit),

        Event::KeyDown { keycode: Some(Keycode::Escape), repeat: false, .. } => {
            Some(Command::Quit)
        }

        Event::KeyDown { keycode: Some(Keycode::Up), repeat: false, .. } => {
            Some(Command::Spawn(Direction::South)) // south -> north
        }
        Event::KeyDown { keycode: Some(Keycode::Down), repeat: false, .. } => {
            Some(Command::Spawn(Direction::North)) // north -> south
        }
        Event::KeyDown { keycode: Some(Keycode::Right), repeat: false, .. } => {
            Some(Command::Spawn(Direction::West)) // west -> east
        }
        Event::KeyDown { keycode: Some(Keycode::Left), repeat: false, .. } => {
            Some(Command::Spawn(Direction::East)) // east -> west
        }

        Event::KeyDown { keycode: Some(Keycode::R), repeat: false, .. } => {
            Some(Command::ToggleRandom)
        }

        _ => None,
    }
}
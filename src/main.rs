mod config;
mod input;
mod vehicle;
mod path;
mod spawner;
mod controller;
mod simulation;
mod stats;
mod render;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use crate::input::{handle_event, Command};

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Smart Road", config::WINDOW_WIDTH, config::WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            if let Some(cmd) = handle_event(&event) {
                match cmd {
                    Command::Quit => break 'running,
                    Command::Spawn(dir) => {
                        println!("{:?}", dir);// should be simulation.spawn_from(dir);
                    }
                    Command::ToggleRandom => {
                        println!("R");// should be simulation.random_mode = !simulation.random_mode;
                    }
                }
            }
        }

        canvas.clear();
        canvas.present();
    }
}
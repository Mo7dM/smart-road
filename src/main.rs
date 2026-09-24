mod config;
mod conflict;
mod controller;
mod input;
mod path;
mod render;
mod rng;
mod simulation;
mod spawner;
mod stats;
mod vehicle;

use std::time::{Duration, Instant};

use crate::input::{handle_event, Command};
use crate::simulation::Simulation;

const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// A long stall (dragging the window, a breakpoint) must not teleport every
/// vehicle across the intersection, so cap how much time one frame may cover.
const MAX_DT: f32 = 0.05;

fn main() {
    // Dev helper: `smart-road --snapshot out.bmp [seconds]` renders one frame
    // to a file instead of opening a window.
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--snapshot" {
        let seconds = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3.0);
        snapshot(&args[2], seconds);
        return;
    }

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Smart Road", config::WINDOW_WIDTH, config::WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let sprites = render::Sprites::new(&texture_creator);
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut simulation = Simulation::new();
    let mut last_frame = Instant::now();

    'running: loop {
        let frame_start = Instant::now();
        let dt = (frame_start - last_frame).as_secs_f32().min(MAX_DT);
        last_frame = frame_start;

        for event in event_pump.poll_iter() {
            if let Some(command) = handle_event(&event) {
                match command {
                    Command::Quit => break 'running,
                    Command::Spawn(from) => simulation.spawn_from(from),
                    Command::ToggleRandom => {
                        simulation.random_mode = !simulation.random_mode;
                    }
                }
            }
        }

        simulation.update(dt);

        render::draw_scene(&mut canvas);
        sprites.draw_vehicles(&mut canvas, &simulation);
        canvas.present();

        if let Some(rest) = FRAME.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}

fn snapshot(path: &str, seconds: f32) {
    use sdl2::pixels::PixelFormatEnum;
    use sdl2::render::Canvas;
    use sdl2::surface::Surface;
    use crate::vehicle::{Direction, Turn};

    let step = 1.0 / 60.0;
    let mut simulation = Simulation::new();

    // One vehicle on each of the twelve routes, spread out far enough in time
    // that the per-approach cooldown lets them all through.
    for turn in Turn::ALL {
        for from in Direction::ALL {
            simulation.spawn(from, turn);
        }
        for _ in 0..40 {
            simulation.update(step);
        }
    }
    for _ in 0..((seconds / step) as u32) {
        simulation.update(step);
    }

    let surface = Surface::new(
        config::WINDOW_WIDTH,
        config::WINDOW_HEIGHT,
        PixelFormatEnum::RGB24,
    )
    .unwrap();
    let mut canvas = Canvas::from_surface(surface).unwrap();
    let texture_creator = canvas.texture_creator();
    let sprites = render::Sprites::new(&texture_creator);

    render::draw_scene(&mut canvas);
    sprites.draw_vehicles(&mut canvas, &simulation);
    canvas.surface().save_bmp(path).unwrap();
}

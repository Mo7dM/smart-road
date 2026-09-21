mod config;
mod controller;
mod input;
mod path;
mod render;
mod simulation;
mod spawner;
mod stats;
mod vehicle;

use std::time::{Duration, Instant};

use crate::input::{handle_event, Command};

const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

fn main() {
    // Dev helper: `smart-road --snapshot out.bmp` renders one frame to a file
    // instead of opening a window.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--snapshot" {
        snapshot(&args[2]);
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
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        let frame_start = Instant::now();

        for event in event_pump.poll_iter() {
            if let Some(cmd) = handle_event(&event) {
                match cmd {
                    Command::Quit => break 'running,
                    Command::Spawn(dir) => {
                        println!("{:?}", dir); // should be simulation.spawn_from(dir);
                    }
                    Command::ToggleRandom => {
                        println!("R"); // should be simulation.random_mode = !simulation.random_mode;
                    }
                }
            }
        }

        render::draw_scene(&mut canvas);
        canvas.present();

        if let Some(rest) = FRAME.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}

fn snapshot(path: &str) {
    use sdl2::pixels::PixelFormatEnum;
    use sdl2::render::Canvas;
    use sdl2::surface::Surface;

    let surface = Surface::new(
        config::WINDOW_WIDTH,
        config::WINDOW_HEIGHT,
        PixelFormatEnum::RGB24,
    )
    .unwrap();
    let mut canvas = Canvas::from_surface(surface).unwrap();
    render::draw_scene(&mut canvas);
    canvas.surface().save_bmp(path).unwrap();
}

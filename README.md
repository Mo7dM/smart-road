# smart-road

A traffic simulation of a cross intersection with **no traffic lights**.
Every vehicle is autonomous, and a central algorithm controls their speeds so
they cross without colliding and without congestion. Built in Rust with SDL2.

## Run

```bash
cargo run --release
```

SDL2 is built from source by the `sdl2` crate (`bundled` feature), so you
need `cmake` installed. No other setup is needed.

```bash
cargo test    # run the tests
```

## Controls

| Key | Action |
|---|---|
| `↑` | Spawn a vehicle from the south, heading north |
| `↓` | Spawn a vehicle from the north, heading south |
| `→` | Spawn a vehicle from the west, heading east |
| `←` | Spawn a vehicle from the east, heading west |
| `R` | Toggle continuous random traffic |
| `Esc` | End the simulation and show the statistics (press again to quit) |

Each spawned vehicle gets a random route: left, straight or right. Spamming a
key does not stack vehicles: a side only spawns again after 0.6 s, and only
once its lane has room.

## How it works

- **Lanes.** Each road has three lanes in and three out. Each lane has one
  route (right, straight, left), and vehicles never change lanes.
- **Vehicles.** A vehicle's position is one number, the distance it has
  driven along its path. Each frame, `distance += velocity × dt` and
  `time += dt`. The position and angle come from the path, so vehicles
  rotate smoothly through turns.
- **Three speeds.** Slow (60 px/s), Medium (120 px/s) and Fast (200 px/s),
  plus stopped.
- **Safe distance.** A vehicle keeps at least 50 px behind the one ahead in
  its lane, slowing down step by step as the gap closes.
- **Crossing.** At startup, the program works out where each pair of routes
  can touch inside the intersection. A vehicle crosses its stop line only at
  Fast, and only if it will be clear of every other vehicle's crossing time
  by 0.25 s. Otherwise it slows down to arrive later, or waits at the line.
  Vehicles are served first come, first served.

## Statistics

When the simulation ends, a window shows:

- vehicles that passed the intersection
- max and min velocity
- max and min time to pass the intersection (from when a vehicle appears
  until it leaves the screen)
- close calls (pairs of vehicles that broke the safe distance)

It also shows extra statistics: vehicles spawned, average time, how many
vehicles had to stop, the longest wait, the most vehicles on the road at
once, and the simulation length.

## Assets

Everything is drawn in code: the road, the car sprites and the statistics
font. The only dependency is `sdl2`.

## Project layout

| File | Purpose |
|---|---|
| `main.rs` | Window, game loop, statistics window |
| `config.rs` | All constants: sizes, speeds, distances |
| `vehicle.rs` | Routes, speeds, and vehicle physics |
| `path.rs` | Lane geometry and the path each route follows |
| `conflict.rs` | Where routes cross, and stop lines |
| `controller.rs` | The smart intersection algorithm |
| `spawner.rs` | Creating vehicles without stacking them |
| `simulation.rs` | The world and the per-frame update |
| `stats.rs` | What is recorded for the statistics |
| `render.rs` | Drawing the road, vehicles and statistics |
| `font.rs` | Hand-drawn bitmap font |
| `input.rs` | Keyboard handling |
| `rng.rs` | Small random number generator |

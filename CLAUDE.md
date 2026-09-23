# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

The 01-edu `smart-road` project: a traffic simulation of a cross intersection
with **no traffic lights**. Every vehicle is autonomous, and a central
algorithm controls their velocities so they cross without colliding and
without congestion. Rust + SDL2, no game engine.

Subject and audit questions: https://public.01-edu.org/subjects/road_intersection/

## Commands

```bash
cargo run                      # open the simulation window
cargo test                     # all tests (path geometry invariants)
cargo test path_is_continuous  # a single test by name
cargo test -- --nocapture      # see println! output from tests
```

Dev helper, not part of the subject: `--snapshot` runs the simulation headless
and writes one frame to a BMP, for verifying rendering without a window.

```bash
cargo run -- --snapshot /tmp/frame.bmp 2.5   # 2.5 simulated seconds
sips -s format png /tmp/frame.bmp --out /tmp/frame.png
```

It spawns one vehicle on each of the twelve routes, so it exercises the whole
geometry in one image. Remove it before the final hand-in if it is a
distraction at audit.

## Build environment (macOS)

SDL2 is built from source via the crate's `bundled` + `static-link` features.
This is not optional here: Homebrew's `sdl2` is now `sdl2-compat`, a shim over
SDL3, and linking against it aborts the `sdl2` crate on the first frame with
`trying to construct an enum from an invalid value 0x207`.

That build needs `cmake` installed, and `.cargo/config.toml` sets
`CMAKE_POLICY_VERSION_MINIMUM = "3.5"` because CMake 4 otherwise refuses SDL's
CMakeLists. Do not remove that env block.

## Architecture

### The central idea

A vehicle's position on the road is **one number**: `distance_traveled`, the
arc length covered since spawning. `Path::sample(s)` turns that number into a
position *and* a heading angle. Nothing nudges `(x, y)` per frame.

Everything downstream falls out of this:

- **Physics** — `velocity = distance / time` is the stored state, not a
  derived quantity.
- **Safe distance** — two vehicles in the same lane compare `distance_traveled`.
  A subtraction, not geometry.
- **The intersection algorithm** — "when does this vehicle reach the conflict
  point?" is `(s_conflict - distance_traveled) / velocity`.

Keep new logic in these terms. Reaching for rectangle intersection tests or
per-frame `(x, y)` deltas is a sign of fighting the design.

### Geometry is derived, never hardcoded

Two rules generate the entire layout, in `config.rs` + `path.rs`:

1. **Lane position** — a lane's centre is
   `road_centre + side_sign(heading) * lane_offset(turn)`.
   Right-hand traffic, so `side_sign` puts southbound traffic on the west half,
   northbound on the east half, and so on. Right turns use the outermost lane
   (offset 100), straight the middle (60), left the innermost (20). Applying
   the same formula with the *exit* heading gives the exit lane, which is why
   no two paths cross inside the box.

2. **Turn radius** — not a tuned constant. It is the distance from the pivot
   (where entry and exit lane centre lines cross) to the edge of the
   intersection box, which makes the quarter-circle tangent to both lanes.
   Right turns come out at r=20, left turns at r=140.

The intersection box is derived **per axis** (`BOX_LEFT/RIGHT/TOP/BOTTOM`), so
the window does not have to be square. Do not reintroduce a single
min/max pair shared by both axes.

### One clock

All timing uses `Simulation::time`, an `f32` of seconds advanced by `dt`, not
`std::time::Instant`. Spawn cooldowns, per-vehicle times and statistics are on
this clock. That is what lets the simulation be fast-forwarded headlessly and
keeps the timing statistics reproducible.

### No crates beyond sdl2

Randomness is a hand-written xorshift (`rng.rs`), car sprites are drawn from
rectangles at startup (`render.rs`), and the statistics window will use a
hand-drawn bitmap font. This is deliberate — it also earns the subject's "own
assets" bonus. Do not add `rand`, `sdl2_image` or `sdl2_ttf`.

### Module map

| Module | Responsibility |
|---|---|
| `config.rs` | Every tunable constant and derived geometry value |
| `vehicle.rs` | `Direction`/`Turn`/`Route`/`Speed` and their logic; `Vehicle::update` |
| `path.rs` | Lane coordinates and the arc-length path; the geometry tests live here |
| `spawner.rs` | Creating vehicles, and the two anti-stacking rules |
| `simulation.rs` | The world: vehicle list, shared clock, per-frame update |
| `controller.rs` | The smart intersection algorithm — **currently empty** |
| `stats.rs` | What gets recorded and shown on exit |
| `render.rs` | Road, markings, route arrows, car sprites |
| `input.rs` | Keyboard events to `Command` |
| `main.rs` | Window, game loop, and the `--snapshot` dev path |

`render.rs` is generic over `Canvas<T>` so it draws to a window or an
offscreen surface with the same code.

## Status

**Done — road and geometry.** Cross intersection, lane markings, per-lane
route arrows, all derived from the two rules above.

**Done — driving.** Vehicles spawn off-screen, follow their lane, turn along a
real arc (rotated sprite, not a snap), exit, and are retired. Arrow keys and
`R` are wired up. Anti-stacking is enforced twice: a 0.6s cooldown per
approach *and* a lane-clearance check that refuses to spawn until the lane's
rearmost vehicle has moved `VEHICLE_LENGTH + SAFE_DISTANCE`.

Five tests in `path.rs` cover all twelve routes: each stays in its approach
lane, exits on the correct lane pointing the right way, rotates exactly 90°
(0° for straight), never jumps position, and starts and ends off-screen.

**Not done — the smart intersection algorithm** (`controller.rs` is empty).
Vehicles currently all run at `Speed::Medium` and drive straight through each
other. This needs:
- Safe-distance following, so a vehicle never closes on the one ahead.
- Use of all three velocities, and a reason to pick each.
- Conflict resolution inside the box, so crossing routes do not collide.
- Close-call detection (safe distance violated) for the statistics.

**Not done — statistics window.** `Statistics` collects the numbers but
nothing displays them. `Esc` currently just quits; it must show a window with
vehicles passed, max/min velocity, max/min time through the intersection, and
close calls. Needs the hand-drawn bitmap font.

**Not done — bonus.** Acceleration and deceleration (vehicles currently snap
to their target speed in `Vehicle::update`), and extra statistics.

## Working agreement

The user drives this project step by step and **makes all commits themselves**
— do not run `git commit`. Suggest a commit message when a step is finished.

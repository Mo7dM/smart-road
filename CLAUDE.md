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
cargo test                     # all tests: geometry, controller, traffic, stats
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
   no two routes share a lane, before or after the box. Routes from different
   approaches do cross *inside* it; `conflict.rs` works out where.

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
rectangles at startup (`render.rs`), and the statistics window uses a
hand-drawn 5x7 bitmap font (`font.rs`). This is deliberate — it also earns the subject's "own
assets" bonus. Do not add `rand`, `sdl2_image` or `sdl2_ttf`.

### Module map

| Module | Responsibility |
|---|---|
| `config.rs` | Every tunable constant and derived geometry value |
| `vehicle.rs` | `Direction`/`Turn`/`Route`/`Speed` and their logic; `Vehicle::update` |
| `path.rs` | Lane coordinates and the arc-length path; the geometry tests live here |
| `spawner.rs` | Creating vehicles, and the two anti-stacking rules |
| `simulation.rs` | The world: vehicle list, shared clock, per-frame update |
| `conflict.rs` | Where routes cross, as distance ranges; stop lines; precomputed once |
| `controller.rs` | The smart intersection algorithm: sets every `target_speed` each frame |
| `stats.rs` | What gets recorded, and its formatting as label/value rows |
| `font.rs` | Hand-drawn 5x7 bitmap font: capitals, digits, `. : / -` |
| `render.rs` | Road, markings, route arrows, car sprites, statistics screen |
| `input.rs` | Keyboard events to `Command` |
| `main.rs` | Window, game loop, statistics window, and the `--snapshot` dev path |

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

**Done — the smart intersection algorithm.** `controller::update` sets every
`target_speed` before the vehicles move. Two rules:

- *Following* — behind another vehicle in its lane, take the fastest speed
  that still leaves `SAFE_DISTANCE` after `FOLLOW_HORIZON` (1s), so a closing
  vehicle steps Fast -> Medium -> Slow -> `Stopped`. Only the same route is
  compared: a lane carries exactly one route.
- *Crossing* — `conflict.rs` samples every pair of paths once at startup
  (each car body as two discs) and records, per pair, the distance range on
  each path where they could touch, plus a stop line per route. A vehicle
  crosses its stop line **only at Fast**, and only if its time window in
  every zone misses every other vehicle's window by `CROSSING_MARGIN`.
  Past the line it is committed and holds Fast until out of its last zone.

Why it is safe: committed vehicles never change speed, so predictions about
them are exact, and nothing commits without checking against all of them.
Because everything past the line drives Fast, a committed vehicle can never
close on its leader either. Why Fast only: letting vehicles crawl into the
box at Slow fed on itself (each slow crosser made the next one slow) and
pushed worst-case trips from ~6s to ~26s. Slow and Medium are for the
*approach* — timing arrival at the line to meet a gap — and for following.

Priority is first come, first served by id: committed vehicles plan first,
then approaching ones oldest first. A vehicle waiting at the line claims its
zones from its earliest possible arrival onwards, so younger vehicles cannot
starve it. Right turns cross nothing and never wait.

Invariants that bit during development, keep them: a vehicle is committed
only when *strictly* past its stop line (creeping stops exactly on it), and
the float tolerance in following must not leak into the stop-line approach.

`simulation.rs` tests drive saturated and `R`-rate traffic and check real
rotated-rectangle overlap (deliberately not the disc model), tailgating, and
throughput / trip time.

**Done — statistics.** `Esc`, or closing the window, ends the simulation,
closes its window and opens a second one with the statistics, drawn in the
bitmap font. `Esc` or its close button quits. Definitions, chosen against the
subject text:

- *Time* is `Vehicle::time`: from spawning (when the controller first sees
  it) until it is removed from the canvas, as the subject specifies. Not
  from entering the box.
- *Min velocity* is the slowest speed actually reached, so 0 as soon as any
  vehicle has had to stop.
- *Close call* is a pair of vehicles breaking the safe distance: same lane
  with a gap under `SAFE_DISTANCE`, or crossing routes both inside the
  stretch where they meet (`Conflicts::both_inside`). Each pair counts once.
  The controller prevents both, so it reads 0; the simulation tests assert it.
- Extra (bonus): vehicles spawned, average time, vehicles that stopped,
  longest wait, most vehicles on the road at once, simulation length.

`simulation.rs` also has tests for two audit questions: 2 Up + 2 Right
presses give exactly 4 passed, and a single vehicle's max time equals its
min time and matches its path length at Fast.

**Not done — bonus: acceleration.** Vehicles still snap to their target speed
in `Vehicle::update`. The crossing algorithm relies on that (a vehicle goes
from stopped to Fast at the line, and its plan windows assume constant
speeds), so adding acceleration means adding braking and acceleration
distances to `speed_before` and to `Claim::window`.

## Working agreement

The user drives this project step by step and **makes all commits themselves**
— do not run `git commit`. Suggest a commit message when a step is finished.

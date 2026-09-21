//! Drawing the world. Everything is built from the geometry in `config` and
//! `path`, so the picture always matches what the vehicles actually do.

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget};

use crate::config::*;
use crate::path;
use crate::vehicle::{Direction, Route, Turn};

const GRASS: Color = Color::RGB(36, 72, 46);
const ASPHALT: Color = Color::RGB(58, 58, 62);
const BOX: Color = Color::RGB(70, 70, 76);
const MARKING: Color = Color::RGB(226, 226, 226);
const CENTER_LINE: Color = Color::RGB(232, 190, 60);
const ARROW: Color = Color::RGB(200, 200, 205);

const DASH_LEN: f32 = 18.0;
const DASH_GAP: f32 = 14.0;
const MARK_THICK: f32 = 3.0;

pub fn draw_scene<T: RenderTarget>(canvas: &mut Canvas<T>) {
    canvas.set_draw_color(GRASS);
    canvas.clear();

    let w = WINDOW_WIDTH as f32;
    let h = WINDOW_HEIGHT as f32;

    // The two roads, then the box where they overlap.
    fill(canvas, ASPHALT, INTER_MIN, 0.0, ROAD_WIDTH, h);
    fill(canvas, ASPHALT, 0.0, INTER_MIN, w, ROAD_WIDTH);
    fill(canvas, BOX, INTER_MIN, INTER_MIN, ROAD_WIDTH, ROAD_WIDTH);

    draw_markings(canvas);

    for from in Direction::ALL {
        for turn in Turn::ALL {
            draw_route_marker(canvas, Route { from, turn });
        }
    }
}

/// Lane separators, outer edges and the centre line, on all four arms. The
/// markings stop at the intersection box: inside it, nothing is painted.
fn draw_markings<T: RenderTarget>(canvas: &mut Canvas<T>) {
    let w = WINDOW_WIDTH as f32;
    let h = WINDOW_HEIGHT as f32;

    // Offsets from a road's centre line: lane separators, then the kerb.
    for &off in &[LANE_WIDTH, LANE_WIDTH * 2.0] {
        for &sign in &[-1.0f32, 1.0] {
            let x = CENTER_X + sign * off;
            dashed(canvas, MARKING, x, 0.0, INTER_MIN, true);
            dashed(canvas, MARKING, x, INTER_MAX, h, true);

            let y = CENTER_Y + sign * off;
            dashed(canvas, MARKING, y, 0.0, INTER_MIN, false);
            dashed(canvas, MARKING, y, INTER_MAX, w, false);
        }
    }

    for &sign in &[-1.0f32, 1.0] {
        let x = CENTER_X + sign * ROAD_HALF;
        solid(canvas, MARKING, x, 0.0, INTER_MIN, true);
        solid(canvas, MARKING, x, INTER_MAX, h, true);

        let y = CENTER_Y + sign * ROAD_HALF;
        solid(canvas, MARKING, y, 0.0, INTER_MIN, false);
        solid(canvas, MARKING, y, INTER_MAX, w, false);
    }

    // Double yellow line splitting the two directions of travel.
    for &sign in &[-1.0f32, 1.0] {
        let x = CENTER_X + sign * 3.0;
        solid(canvas, CENTER_LINE, x, 0.0, INTER_MIN, true);
        solid(canvas, CENTER_LINE, x, INTER_MAX, h, true);

        let y = CENTER_Y + sign * 3.0;
        solid(canvas, CENTER_LINE, y, 0.0, INTER_MIN, false);
        solid(canvas, CENTER_LINE, y, INTER_MAX, w, false);
    }
}

/// The arrow painted on the road that tells you what a lane is for: straight
/// shaft along the approach, bending into the direction the lane leaves by.
fn draw_route_marker<T: RenderTarget>(canvas: &mut Canvas<T>, route: Route) {
    let heading = route.entry_heading();
    let exit = route.exit_heading();

    let (ux, uy) = heading.unit();
    let along_unit = if heading.is_vertical() { uy } else { ux };
    let edge = if along_unit > 0.0 { INTER_MIN } else { INTER_MAX };
    let start = path::entry_point(route, edge - along_unit * 150.0);

    let shaft = 40.0;
    let bend = (start.0 + ux * shaft, start.1 + uy * shaft);
    thick_seg(canvas, ARROW, start, bend, 5.0);

    let (ex, ey) = exit.unit();
    let tip = if route.turn == Turn::Straight {
        bend
    } else {
        let t = (bend.0 + ex * 10.0, bend.1 + ey * 10.0);
        thick_seg(canvas, ARROW, bend, t, 5.0);
        t
    };
    arrow_head(canvas, ARROW, tip, exit);
}

// ─── primitives ────────────────────────────────────────────────────────────

fn fill<T: RenderTarget>(canvas: &mut Canvas<T>, color: Color, x: f32, y: f32, w: f32, h: f32) {
    canvas.set_draw_color(color);
    let _ = canvas.fill_rect(Rect::new(
        x.round() as i32,
        y.round() as i32,
        w.round().max(1.0) as u32,
        h.round().max(1.0) as u32,
    ));
}

/// A solid line. `across` is the fixed coordinate, `from`..`to` the extent
/// along the line; `vertical` says which axis is which.
fn solid<T: RenderTarget>(canvas: &mut Canvas<T>, color: Color, across: f32, from: f32, to: f32, vertical: bool) {
    let t = MARK_THICK;
    if vertical {
        fill(canvas, color, across - t / 2.0, from, t, to - from);
    } else {
        fill(canvas, color, from, across - t / 2.0, to - from, t);
    }
}

fn dashed<T: RenderTarget>(canvas: &mut Canvas<T>, color: Color, across: f32, from: f32, to: f32, vertical: bool) {
    let mut at = from;
    while at < to {
        let end = (at + DASH_LEN).min(to);
        solid(canvas, color, across, at, end, vertical);
        at = end + DASH_GAP;
    }
}

/// An axis-aligned segment of the given thickness between two points.
fn thick_seg<T: RenderTarget>(canvas: &mut Canvas<T>, color: Color, a: (f32, f32), b: (f32, f32), t: f32) {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    fill(
        canvas,
        color,
        x0 - t / 2.0,
        y0 - t / 2.0,
        (x1 - x0) + t,
        (y1 - y0) + t,
    );
}

/// A triangular head at `tip`, pointing along `dir`, drawn as stacked rows.
fn arrow_head<T: RenderTarget>(canvas: &mut Canvas<T>, color: Color, tip: (f32, f32), dir: Direction) {
    let (ux, uy) = dir.unit();
    let rows = 10.0;
    let mut i = 0.0;
    while i < rows {
        let half = rows - i;
        let p = (tip.0 + ux * i, tip.1 + uy * i);
        if dir.is_vertical() {
            fill(canvas, color, p.0 - half, p.1, half * 2.0, 1.0);
        } else {
            fill(canvas, color, p.0, p.1 - half, 1.0, half * 2.0);
        }
        i += 1.0;
    }
}

//! Drawing the world. Everything is built from the geometry in `config` and
//! `path`, so the picture always matches what the vehicles actually do.

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{BlendMode, Canvas, RenderTarget, Texture, TextureCreator};
use sdl2::surface::Surface;

use crate::config::*;
use crate::font;
use crate::path;
use crate::simulation::Simulation;
use crate::stats::Statistics;
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
    fill(canvas, ASPHALT, BOX_LEFT, 0.0, ROAD_WIDTH, h);
    fill(canvas, ASPHALT, 0.0, BOX_TOP, w, ROAD_WIDTH);
    fill(canvas, BOX, BOX_LEFT, BOX_TOP, ROAD_WIDTH, ROAD_WIDTH);

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
            dashed(canvas, MARKING, x, 0.0, BOX_TOP, true);
            dashed(canvas, MARKING, x, BOX_BOTTOM, h, true);

            let y = CENTER_Y + sign * off;
            dashed(canvas, MARKING, y, 0.0, BOX_LEFT, false);
            dashed(canvas, MARKING, y, BOX_RIGHT, w, false);
        }
    }

    for &sign in &[-1.0f32, 1.0] {
        let x = CENTER_X + sign * ROAD_HALF;
        solid(canvas, MARKING, x, 0.0, BOX_TOP, true);
        solid(canvas, MARKING, x, BOX_BOTTOM, h, true);

        let y = CENTER_Y + sign * ROAD_HALF;
        solid(canvas, MARKING, y, 0.0, BOX_LEFT, false);
        solid(canvas, MARKING, y, BOX_RIGHT, w, false);
    }

    // Double yellow line splitting the two directions of travel.
    for &sign in &[-1.0f32, 1.0] {
        let x = CENTER_X + sign * 3.0;
        solid(canvas, CENTER_LINE, x, 0.0, BOX_TOP, true);
        solid(canvas, CENTER_LINE, x, BOX_BOTTOM, h, true);

        let y = CENTER_Y + sign * 3.0;
        solid(canvas, CENTER_LINE, y, 0.0, BOX_LEFT, false);
        solid(canvas, CENTER_LINE, y, BOX_RIGHT, w, false);
    }
}

/// The arrow painted on the road that tells you what a lane is for: straight
/// shaft along the approach, bending into the direction the lane leaves by.
fn draw_route_marker<T: RenderTarget>(canvas: &mut Canvas<T>, route: Route) {
    let heading = route.entry_heading();
    let exit = route.exit_heading();

    let (ux, uy) = heading.unit();
    let along_unit = if heading.is_vertical() { uy } else { ux };
    let start = path::entry_point(route, path::near_edge(heading) - along_unit * 150.0);

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

// ─── vehicles ──────────────────────────────────────────────────────────────

/// Body colours, cycled through so neighbouring vehicles stay tellable apart.
const CAR_COLORS: [Color; 6] = [
    Color::RGB(198, 62, 58),
    Color::RGB(58, 110, 198),
    Color::RGB(222, 178, 52),
    Color::RGB(226, 226, 232),
    Color::RGB(62, 158, 96),
    Color::RGB(214, 118, 46),
];

const GLASS: Color = Color::RGB(132, 186, 218);
const TYRE: Color = Color::RGB(26, 26, 30);
const HEADLIGHT: Color = Color::RGB(255, 242, 186);
const TAILLIGHT: Color = Color::RGB(214, 58, 48);

/// The car sprites, drawn once at start-up rather than loaded from files.
pub struct Sprites<'a> {
    cars: Vec<Texture<'a>>,
}

impl<'a> Sprites<'a> {
    pub fn new<C>(creator: &'a TextureCreator<C>) -> Sprites<'a> {
        let cars = CAR_COLORS
            .iter()
            .map(|&body| {
                let surface = car_surface(body);
                let mut texture = creator.create_texture_from_surface(&surface).unwrap();
                texture.set_blend_mode(BlendMode::Blend);
                texture
            })
            .collect();
        Sprites { cars }
    }

    pub fn draw_vehicles<T: RenderTarget>(&self, canvas: &mut Canvas<T>, sim: &Simulation) {
        for vehicle in &sim.vehicles {
            let texture = &self.cars[vehicle.id as usize % self.cars.len()];
            let dst = Rect::new(
                (vehicle.position.0 - VEHICLE_LENGTH / 2.0).round() as i32,
                (vehicle.position.1 - VEHICLE_WIDTH / 2.0).round() as i32,
                VEHICLE_LENGTH as u32,
                VEHICLE_WIDTH as u32,
            );
            // copy_ex rotates clockwise, which is the same convention the
            // path uses for its heading angle.
            let _ = canvas.copy_ex(texture, None, Some(dst), vehicle.angle, None, false, false);
        }
    }
}

/// Draw one car, nose pointing east, on a transparent surface.
fn car_surface(body: Color) -> Surface<'static> {
    let l = VEHICLE_LENGTH as u32;
    let w = VEHICLE_WIDTH as u32;
    let mut surface = Surface::new(l, w, PixelFormatEnum::RGBA32).unwrap();
    let mut put = |x: i32, y: i32, rw: u32, rh: u32, color: Color| {
        let _ = surface.fill_rect(Rect::new(x, y, rw, rh), color);
    };

    put(0, 0, l, w, Color::RGBA(0, 0, 0, 0));
    put(5, 0, 6, 3, TYRE);
    put(5, 17, 6, 3, TYRE);
    put(21, 0, 6, 3, TYRE);
    put(21, 17, 6, 3, TYRE);
    put(1, 2, 30, 16, shade(body, 0.55)); // outline
    put(2, 3, 28, 14, body);
    put(9, 4, 13, 12, shade(body, 0.8)); // cabin
    put(19, 5, 4, 10, GLASS); // windscreen
    put(8, 5, 3, 10, GLASS); // rear window
    put(28, 4, 2, 3, HEADLIGHT);
    put(28, 13, 2, 3, HEADLIGHT);
    put(2, 4, 2, 3, TAILLIGHT);
    put(2, 13, 2, 3, TAILLIGHT);

    surface
}

fn shade(color: Color, factor: f32) -> Color {
    Color::RGB(
        (color.r as f32 * factor) as u8,
        (color.g as f32 * factor) as u8,
        (color.b as f32 * factor) as u8,
    )
}

// ─── statistics window ─────────────────────────────────────────────────────

const PANEL: Color = Color::RGB(30, 31, 35);
const PANEL_STRIPE: Color = Color::RGB(38, 39, 44);
const LABEL: Color = Color::RGB(190, 190, 196);
const VALUE: Color = Color::RGB(250, 250, 250);
const FAINT: Color = Color::RGB(120, 120, 128);

/// The statistics screen shown when the simulation ends. `elapsed` is how
/// long the simulation ran, in seconds.
pub fn draw_statistics<T: RenderTarget>(canvas: &mut Canvas<T>, stats: &Statistics, elapsed: f32) {
    let w = STATS_WIDTH as i32;
    let margin = 40;
    canvas.set_draw_color(PANEL);
    canvas.clear();

    let title = "STATISTICS";
    font::draw_text(canvas, title, (w - font::text_width(title, 5) as i32) / 2, 30, 5, CENTER_LINE);
    fill(canvas, CENTER_LINE, margin as f32, 82.0, (w - 2 * margin) as f32, 3.0);

    let mut y = 104;
    draw_rows(canvas, &stats.required_rows(), &mut y);

    y += 14;
    font::draw_text(canvas, "MORE", margin, y, 2, FAINT);
    fill(canvas, FAINT, (margin + 64) as f32, (y + 6) as f32, (w - 2 * margin - 64) as f32, 1.0);
    y += 28;
    draw_rows(canvas, &stats.extra_rows(elapsed), &mut y);

    let footer = "PRESS ESC TO CLOSE";
    let footer_y = STATS_HEIGHT as i32 - 34;
    font::draw_text(canvas, footer, (w - font::text_width(footer, 2) as i32) / 2, footer_y, 2, FAINT);
}

/// Label on the left, value right-aligned, one row per pair, on alternating
/// stripes. Advances `y` past the rows.
fn draw_rows<T: RenderTarget>(canvas: &mut Canvas<T>, rows: &[(&str, String)], y: &mut i32) {
    const SCALE: u32 = 3;
    const ROW: i32 = 34;
    let margin = 40;
    let right = STATS_WIDTH as i32 - margin;
    let text_h = (font::GLYPH_HEIGHT * SCALE) as i32;

    for (i, (label, value)) in rows.iter().enumerate() {
        if i % 2 == 0 {
            fill(canvas, PANEL_STRIPE, (margin - 10) as f32, *y as f32, (right - margin + 20) as f32, ROW as f32);
        }
        let text_y = *y + (ROW - text_h) / 2;
        font::draw_text(canvas, label, margin, text_y, SCALE, LABEL);
        let value_x = right - font::text_width(value, SCALE) as i32;
        font::draw_text(canvas, value, value_x, text_y, SCALE, VALUE);
        *y += ROW;
    }
}

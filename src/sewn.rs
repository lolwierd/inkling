//! Your blocks, sewn: a checklist's box and its cross-stitch, a callout's patch, a
//! quote's seam, a code block's ground. What tells them from Inkling's marks is colour,
//! not stitching: they are sewn in your ink, in tacking thread, or in the thread of
//! their kind, and never in madder or rose, which are only ever Inkling's.
//!
//! The geometry is pure (and tested); the painting takes a window. The sizes follow
//! the explorations board (`design/explorations/index.html`, "04 Other blocks").

use gpui_kit::*;
use std::time::{Duration, Instant};

use crate::store::{Callout, InkColor};
use crate::theme::{Palette, ink_color};

// ---------------------------------------------------------------- colours

/// A callout's thread: indigo for a note, ochre for a warning, sage for a tip.
pub fn callout_thread(kind: Callout, p: &Palette) -> Hsla {
    ink_color(
        match kind {
            Callout::Note => InkColor::Blue,
            Callout::Warning => InkColor::Orange,
            Callout::Tip => InkColor::Green,
        },
        p,
    )
}

/// How much of its thread a callout's patch takes on.
pub const PATCH_TINT: f32 = 0.06;

/// The chip under inline code: ink at 6% (8% on the dark page, where 6% barely shows).
pub fn chip(p: &Palette) -> Hsla {
    p.ink.opacity(if p.dark { 0.08 } else { 0.06 })
}

// ---------------------------------------------------------------- block geometry

/// A checklist item's text starts this far in; its box sits in the space before it.
pub const CHECK_INDENT: f32 = 26.;
/// The box is drawn in an 18 px square at the start of the line, its stitches 3 px in.
pub const BOX: f32 = 18.;

/// Space above a callout's patch, beyond the gap the line above leaves.
pub const CALLOUT_MARGIN: f32 = 4.;
/// Inside the patch: above the kind's name, the name's own line, and below the words.
pub const CALLOUT_PAD_TOP: f32 = 10.;
pub const CALLOUT_LABEL: f32 = 19.;
pub const CALLOUT_PAD_BOTTOM: f32 = 12.;
/// Space after a patch before the next line: with the margin above the next patch,
/// two callouts in a row sit 10 px apart.
pub const CALLOUT_AFTER: f32 = 6.;
/// A callout's words are inset this much from the patch's sides.
pub const CALLOUT_INSET: f32 = 16.;

/// Space above a code block's ground, beyond the gap the line above leaves.
pub const CODE_MARGIN: f32 = 4.;
/// The band at the top of a code block that holds its language and Copy.
pub const CODE_HEAD: f32 = 30.;
/// Inside the ground, under the last line.
pub const CODE_PAD: f32 = 10.;
/// Space after the ground before the next line.
pub const CODE_AFTER: f32 = 14.;
/// The code's own column starts here; line numbers are right-aligned before it.
pub const CODE_INDENT: f32 = 40.;
/// Line numbers end this far from the ground's left edge.
pub const NUMBER_END: f32 = 26.;
pub const CODE_RIGHT: f32 = 14.;
/// The Copy button: its height, its distance from the ground's top right corner, and
/// the width a click counts in (it reads Copied for a moment after).
pub const COPY_H: f32 = 22.;
pub const COPY_INSET: f32 = 4.;
pub const COPY_HIT_W: f32 = 56.;
pub const COPIED_FOR: Duration = Duration::from_millis(1200);

/// The rectangle a click on a checklist box counts in, relative to the editor: a little
/// larger than the box, so it is easy to hit.
pub fn box_hit(line_top: Pixels, line_height: Pixels) -> Bounds<Pixels> {
    let cy = line_top + line_height / 2.;
    Bounds::new(point(px(-2.), cy - px(BOX / 2. + 2.)), size(px(BOX + 4.), px(BOX + 4.)))
}

/// The Copy button at the top right of a code block's ground.
pub fn copy_hit(ground: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(ground.origin.x + ground.size.width - px(COPY_INSET + COPY_HIT_W), ground.origin.y + px(COPY_INSET)),
        size(px(COPY_HIT_W), px(COPY_H)),
    )
}

// ---------------------------------------------------------------- stitches, laid out

/// A stitch: thread on the surface between two holes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stitch {
    pub a: Point<f32>,
    pub b: Point<f32>,
}

/// A repeatable random number in 0..1 for stitch `i`, from the app's own jitter.
fn rand(seed: u64, i: u32) -> f32 {
    crate::stitch::jitter(seed, i) + 0.5
}

/// Stitches along a straight run from `x0` to `x1` at `y`: lengths and gaps vary a
/// little, seeded, so a redraw never moves one; each sits a fraction of a pixel off
/// the line, the way a hand-sewn row does.
pub fn lay_run(x0: f32, x1: f32, y: f32, len: f32, gap: f32, wobble: f32, seed: u64) -> Vec<Stitch> {
    let mut out = vec![];
    let mut x = x0;
    let mut i = 0u32;
    while x < x1 - 1.5 {
        let l = (len * (0.88 + rand(seed, i) * 0.24)).min(x1 - x);
        if l < len * 0.45 {
            break;
        }
        let dy = (rand(seed, i + 101) - 0.5) * wobble;
        out.push(Stitch { a: point(x, y + dy), b: point(x + l, y + dy) });
        x += l + gap * (0.85 + rand(seed, i + 211) * 0.3);
        i += 1;
    }
    out
}

/// Stitches along any polyline (a patch's edge): on for about `len`, under the cloth
/// for about `gap`. A stitch that turns a corner is a straight chord across it.
pub fn lay_along(pts: &[Point<f32>], len: f32, gap: f32, seed: u64) -> Vec<Stitch> {
    let mut out = vec![];
    let Some(&first) = pts.first() else { return out };
    let mut i = 0u32;
    let next_target = |on: bool, i: &mut u32| {
        *i += 1;
        (if on { len } else { gap.max(0.01) }) * (0.88 + rand(seed, *i) * 0.24)
    };
    let (mut on, mut acc, mut start) = (true, 0f32, first);
    let mut target = next_target(true, &mut i);
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let d = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let mut used = 0f32;
        while used < d - 1e-4 {
            let take = (d - used).min(target - acc);
            used += take;
            acc += take;
            let t = used / d;
            let p = point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            if acc >= target - 0.01 {
                if on {
                    out.push(Stitch { a: start, b: p });
                }
                on = !on;
                acc = 0.;
                start = p;
                target = next_target(on, &mut i);
            }
        }
    }
    out
}

/// A rounded rectangle inset by `inset` inside `w` x `h`, clockwise from its top left,
/// as points (each corner an arc of six).
pub fn rounded_rect(origin: Point<f32>, w: f32, h: f32, inset: f32, r: f32) -> Vec<Point<f32>> {
    let (x0, y0, x1, y1) = (origin.x + inset, origin.y + inset, origin.x + w - inset, origin.y + h - inset);
    let r = r.min((x1 - x0) / 2.).min((y1 - y0) / 2.).max(0.);
    let mut pts = vec![point(x0 + r, y0), point(x1 - r, y0)];
    let arc = |cx: f32, cy: f32, a0: f32, pts: &mut Vec<Point<f32>>| {
        for j in 1..=6 {
            let a = a0 + std::f32::consts::FRAC_PI_2 * j as f32 / 6.;
            pts.push(point(cx + r * a.cos(), cy + r * a.sin()));
        }
    };
    use std::f32::consts::PI;
    arc(x1 - r, y0 + r, -PI / 2., &mut pts);
    pts.push(point(x1, y1 - r));
    arc(x1 - r, y1 - r, 0., &mut pts);
    pts.push(point(x0 + r, y1));
    arc(x0 + r, y1 - r, PI / 2., &mut pts);
    pts.push(point(x0, y0 + r));
    arc(x0 + r, y0 + r, PI, &mut pts);
    pts
}

// ---------------------------------------------------------------- painting

fn px_point(p: Point<f32>) -> Point<Pixels> {
    point(px(p.x), px(p.y))
}

/// The same colour with the light on it: mixed 46% with white, as the board lights a
/// stitch's upper side.
fn lit(c: Hsla) -> Hsla {
    let r = c.to_rgb();
    let m = |v: f32| v * 0.46 + 0.54;
    Hsla::from(Rgba { r: m(r.r), g: m(r.g), b: m(r.b), a: r.a })
}

/// One stitch from `s.a` towards `s.b`, grown to `k` of its length (0..1): narrow at
/// each hole, full in the middle, with the light catching its upper side.
pub fn lens(window: &mut Window, origin: Point<Pixels>, s: Stitch, w: f32, color: Hsla, k: f32) {
    if k <= 0.001 {
        return;
    }
    let k = k.min(1.);
    let (x0, y0) = (s.a.x, s.a.y);
    let (x1, y1) = (x0 + (s.b.x - x0) * k, y0 + (s.b.y - y0) * k);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let l = (dx * dx + dy * dy).sqrt();
    if l < 0.3 {
        return;
    }
    let (nx, ny) = (-dy / l, dx / l);
    let (mx, my) = ((x0 + x1) / 2., (y0 + y1) / 2.);
    let (h, he, bulge) = (w / 2., w * 0.2, 1.5);
    let at = |x: f32, y: f32| origin + px_point(point(x, y));
    let mut b = PathBuilder::fill();
    b.move_to(at(x0 + nx * he, y0 + ny * he));
    b.curve_to(at(x1 + nx * he, y1 + ny * he), at(mx + nx * h * bulge, my + ny * h * bulge));
    b.line_to(at(x1 - nx * he, y1 - ny * he));
    b.curve_to(at(x0 - nx * he, y0 - ny * he), at(mx - nx * h * bulge, my - ny * h * bulge));
    b.close();
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
    // the highlight, on the side facing up
    let (mut nx, mut ny) = (nx, ny);
    if ny > 0. || (ny == 0. && nx > 0.) {
        nx = -nx;
        ny = -ny;
    }
    let o = w * 0.2;
    let mut hl = PathBuilder::stroke(px(w * 0.24));
    hl.move_to(at(x0 + dx * 0.26 + nx * o, y0 + dy * 0.26 + ny * o));
    hl.line_to(at(x0 + dx * 0.68 + nx * o, y0 + dy * 0.68 + ny * o));
    if let Ok(path) = hl.build() {
        window.paint_path(path, lit(color).opacity(0.72));
    }
}

/// A stitch as a capsule (thread has round ends) when it runs straight across or down,
/// which is cheap to draw; any other way it is a lens.
pub fn stitch(window: &mut Window, origin: Point<Pixels>, s: Stitch, w: f32, color: Hsla) {
    let (dx, dy) = (s.b.x - s.a.x, s.b.y - s.a.y);
    if dy.abs() < 0.05 || dx.abs() < 0.05 {
        let (x0, x1) = (s.a.x.min(s.b.x), s.a.x.max(s.b.x));
        let (y0, y1) = (s.a.y.min(s.b.y), s.a.y.max(s.b.y));
        let b = if dy.abs() < 0.05 {
            Bounds::new(origin + px_point(point(x0, y0 - w / 2.)), size(px(x1 - x0), px(w)))
        } else {
            Bounds::new(origin + px_point(point(x0 - w / 2., y0)), size(px(w), px(y1 - y0)))
        };
        window.paint_quad(fill(b, color).corner_radii(px(w / 2.)));
    } else {
        lens(window, origin, s, w, color, 1.);
    }
}

/// A checklist's box: four stitches in graphite, a 12 px square inside an 18 px one
/// whose top left is `at`.
pub fn check_box(window: &mut Window, at: Point<Pixels>, color: Hsla) {
    let sq = [((3., 3.), (15., 3.)), ((15., 3.), (15., 15.)), ((15., 15.), (3., 15.)), ((3., 15.), (3., 3.))];
    for ((x0, y0), (x1, y1)) in sq {
        lens(window, at, Stitch { a: point(x0, y0), b: point(x1, y1) }, 1.5, color, 1.);
    }
}

/// The cross-stitch that marks an item done: the first leg, then the second laid over
/// it, each grown to `k1` and `k2` of its length.
pub fn cross(window: &mut Window, at: Point<Pixels>, color: Hsla, k1: f32, k2: f32) {
    lens(window, at, Stitch { a: point(5.2, 12.8), b: point(12.8, 5.2) }, 2.3, color, k1);
    lens(window, at, Stitch { a: point(5.2, 5.2), b: point(12.8, 12.8) }, 2.3, color, k2);
}

/// A quote's seam: back stitch (a solid line of short stitches) down `x`, from `y0` to
/// `y1`, where the 2 px bar used to be.
pub fn seam(window: &mut Window, x: Pixels, y0: Pixels, y1: Pixels, color: Hsla, seed: u64) {
    let (x, y0, y1) = (x.as_f32(), y0.as_f32(), y1.as_f32());
    let mut y = y0;
    let mut i = 0u32;
    while y < y1 - 1. {
        let l = (5.2 * (0.9 + rand(seed, i) * 0.2)).min(y1 - y);
        let dx = (rand(seed, i + 101) - 0.5) * 0.5;
        stitch(window, point(px(0.), px(0.)), Stitch { a: point(x + dx, y), b: point(x + dx, y + l) }, 2., color);
        y += l + 0.8;
        i += 1;
    }
}

/// A callout's patch: a 6% tint of its thread, and a running stitch along its edge
/// 5 px in.
pub fn patch(window: &mut Window, b: Bounds<Pixels>, thread: Hsla, seed: u64) {
    window.paint_quad(fill(b, thread.opacity(PATCH_TINT)).corner_radii(px(8.)));
    let pts = rounded_rect(point(b.origin.x.as_f32(), b.origin.y.as_f32()), b.size.width.as_f32(), b.size.height.as_f32(), 5., 4.5);
    for s in lay_along(&pts, 6., 3.4, seed) {
        stitch(window, point(px(0.), px(0.)), s, 1.75, thread.opacity(0.92));
    }
}

// ---------------------------------------------------------------- the check, in motion

/// A checklist item that changed a moment ago: it sews in, or comes out.
#[derive(Clone, Copy, Debug)]
pub struct Toggle {
    pub at: Instant,
    pub done: bool,
}

/// `INKLING_SLOWMO=<factor>` slows this too, like every thread animation.
fn slowmo() -> f32 {
    static F: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *F.get_or_init(|| std::env::var("INKLING_SLOWMO").ok().and_then(|v| v.parse::<f32>().ok()).filter(|f| *f >= 1.).unwrap_or(1.))
}

/// Seconds since `at`, in animation time.
pub fn since(at: Instant) -> f32 {
    at.elapsed().as_secs_f32() / slowmo()
}

fn ease_out(k: f32) -> f32 {
    1. - (1. - k.clamp(0., 1.)).powi(3)
}

fn ease_in(k: f32) -> f32 {
    k.clamp(0., 1.).powi(2)
}

/// When the cross is in and the line starts being tacked through.
const CROSS_IN: f32 = 0.18;
const LEG: f32 = 0.1;
/// Each stitch of the tacked line starts this long after the one before, and takes
/// `GROW` to sew.
const EVERY: f32 = 0.022;
const GROW: f32 = 0.13;
/// Coming out: the last stitch first, one every `UNPICK_EVERY`, each gone in `OUT`.
const UNPICK_EVERY: f32 = 0.018;
const OUT: f32 = 0.11;

/// How far each leg of the cross is sewn, `t` seconds into a toggle (done or undone).
pub fn cross_at(t: f32, done: bool) -> (f32, f32) {
    if done {
        (ease_out(t / LEG), ease_out((t - (CROSS_IN - LEG)) / LEG))
    } else {
        // the second leg comes out first, then the first
        (1. - ease_in((t - UNPICK_EVERY) / OUT), 1. - ease_in(t / OUT))
    }
}

/// How far stitch `i` of the `n` that tack a done line is sewn, `t` seconds in.
pub fn tack_at(i: usize, n: usize, t: f32, done: bool) -> f32 {
    if done {
        ease_out((t - CROSS_IN - i as f32 * EVERY) / GROW)
    } else {
        1. - ease_in((t - (n.saturating_sub(1 + i)) as f32 * UNPICK_EVERY) / OUT)
    }
}

/// Whether a toggle with `n` tacking stitches is still moving `t` seconds in.
pub fn moving(t: f32, n: usize, done: bool) -> bool {
    if done { t < CROSS_IN + n as f32 * EVERY + GROW } else { t < n.max(2) as f32 * UNPICK_EVERY + OUT }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn a_run_stays_on_its_line_and_never_moves() {
        let a = lay_run(0., 200., 10., 10., 3., 0.5, 7);
        assert_eq!(a, lay_run(0., 200., 10., 10., 3., 0.5, 7), "seeded: the same every frame");
        assert!(a.len() > 10);
        for s in &a {
            assert!(s.a.x >= 0. && s.b.x <= 200. && s.b.x > s.a.x);
            assert!((s.a.y - 10.).abs() <= 0.25 && s.a.y == s.b.y);
        }
        assert!(lay_run(0., 2., 10., 10., 3., 0.5, 7).is_empty(), "no room, no stitch");
    }

    #[std::prelude::v1::test]
    fn a_patch_edge_goes_all_the_way_round() {
        let pts = rounded_rect(point(0., 0.), 300., 80., 5., 4.5);
        assert_eq!(pts.first().copied(), Some(point(9.5, 5.)));
        let last = pts.last().copied().unwrap();
        assert!((last.x - 9.5).abs() < 0.01 && (last.y - 5.).abs() < 0.01, "it closes where it starts: {last:?}");
        let stitches = lay_along(&pts, 6., 3.4, 3);
        // the perimeter is about 2 x (290 + 70); on for ~6 of every ~9.4
        let sewn: f32 = stitches.iter().map(|s| ((s.b.x - s.a.x).powi(2) + (s.b.y - s.a.y).powi(2)).sqrt()).sum();
        assert!((440. ..=500.).contains(&sewn), "{sewn}");
        for s in &stitches {
            for q in [s.a, s.b] {
                assert!(q.x >= 4.99 && q.x <= 295.01 && q.y >= 4.99 && q.y <= 75.01, "{q:?} is on the inset edge");
            }
        }
        // a tiny box still gets a sensible outline
        assert!(rounded_rect(point(0., 0.), 8., 8., 5., 4.5).iter().all(|p| p.x.is_finite() && p.y.is_finite()));
    }

    #[std::prelude::v1::test]
    fn the_cross_sews_in_leg_by_leg_then_the_line_is_tacked() {
        assert_eq!(cross_at(0., true), (0., 0.));
        let (a, b) = cross_at(0.05, true);
        assert!(a > 0.5 && b == 0., "the first leg first: {a} {b}");
        assert_eq!(cross_at(0.18, true), (1., 1.), "both legs in by 180 ms");
        assert_eq!(tack_at(0, 10, 0.17, true), 0., "the line waits for the cross");
        assert!(tack_at(0, 10, 0.25, true) > 0.5);
        assert!(tack_at(9, 10, 0.25, true) == 0., "left to right");
        assert!(moving(0.3, 10, true) && !moving(0.6, 10, true));
        // unchecking unpicks, last stitch first
        assert!(tack_at(9, 10, 0.05, false) < 1. && tack_at(0, 10, 0.05, false) == 1.);
        let (a, b) = cross_at(0.05, false);
        assert!(b < a, "the second leg comes out before the first: {a} {b}");
        assert_eq!(cross_at(1., false), (0., 0.));
        assert!(!moving(0.4, 10, false));
    }

    #[std::prelude::v1::test]
    fn hit_areas() {
        let b = box_hit(px(100.), px(28.));
        assert!(b.contains(&point(px(9.), px(114.))) && !b.contains(&point(px(30.), px(114.))));
        let ground = Bounds::new(point(px(0.), px(50.)), size(px(560.), px(200.)));
        let c = copy_hit(ground);
        assert!(c.contains(&point(px(540.), px(60.))) && !c.contains(&point(px(300.), px(60.))));
    }
}

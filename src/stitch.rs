//! Loose Threads drawing: the thread, the stitches, the knots, the needle and the
//! sampler border. Everything Inkling marks is made of thread, so these are the only
//! shapes it draws in the note.
//!
//! The thread is a small rope simulation rather than a curve, because a curve can only
//! look like thread when it is standing still. Real thread sags under its own weight,
//! never kinks, sways when touched, pulls straight when tied off, and drops when cut.

use gpui_kit::*;
use std::f32::consts::PI;

// ---------------------------------------------------------------- hand-made, but steady

/// FNV-1a: a stable seed per thread, so its stitches look hand-made but never move
/// between frames or launches.
pub fn seed_of(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// A repeatable wobble in -0.5..0.5 for stitch `i` of a thread.
pub fn jitter(seed: u64, i: u32) -> f32 {
    let mut x = seed ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    (x & 0xffff) as f32 / 65535. - 0.5
}

/// The same hue, a little lighter: the light catching the twist of the thread.
fn lighter(c: Hsla, by: f32) -> Hsla {
    Hsla { l: (c.l + by).min(0.92), ..c }
}

// ---------------------------------------------------------------- stitches

/// Room a correction's cross-stitch takes at the start of its words.
const CROSS_ROOM: f32 = 11.;

/// One stitch of a running stitch: thread on the surface from `x0` to `x1`, `dy` off the
/// line (the hand is steady, not exact).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dash {
    pub x0: f32,
    pub x1: f32,
    pub dy: f32,
}

/// The stitches of a running stitch from `x0` to `x1`: thread on the surface for a
/// stitch, then under the cloth for a gap. `skip_first` leaves room for a cross-stitch at
/// the start.
pub fn stitch_dashes(x0: f32, x1: f32, seed: u64, skip_first: bool) -> Vec<Dash> {
    let mut out = vec![];
    let mut x = x0 + if skip_first { CROSS_ROOM } else { 0. };
    let mut i = 0u32;
    while x < x1 - 2.5 {
        let end = (x + 6.6 + jitter(seed, i) * 1.4).min(x1);
        out.push(Dash { x0: x, x1: end, dy: jitter(seed, i + 101) * 0.7 });
        x = end + 3.7 + jitter(seed, i + 211) * 0.8;
        i += 1;
    }
    out
}

/// One stitch, along `y`. Each is a capsule, because thread has round ends.
pub fn paint_dash(window: &mut Window, d: Dash, y: Pixels, color: Hsla) {
    let h = px(1.8);
    let b = Bounds::new(point(px(d.x0), y - h / 2. + px(d.dy)), size(px(d.x1 - d.x0), h));
    window.paint_quad(fill(b, color).corner_radii(h / 2.));
}

/// A running stitch from `x0` to `x1` along `y` (see [`stitch_dashes`]).
pub fn running_stitch(window: &mut Window, x0: Pixels, x1: Pixels, y: Pixels, color: Hsla, seed: u64, skip_first: bool) {
    for d in stitch_dashes(x0.as_f32(), x1.as_f32(), seed, skip_first) {
        paint_dash(window, d, y, color);
    }
}

/// Your own thread's stitches: seed stitches, small dots in pairs, two to each stitch of
/// the running stitch it stands in for, so the row keeps the same rhythm and a thread of
/// yours differs from a mind's in shape and not only in colour. Returns each dot's centre.
pub fn seed_dots(d: Dash) -> Vec<f32> {
    const R: f32 = SEED_D / 2.;
    if d.x1 - d.x0 < 2. * SEED_D + 0.6 {
        return vec![(d.x0 + d.x1) / 2.];
    }
    vec![d.x0 + R, d.x1 - R]
}

/// A seed stitch is a little wider than the thread is thick, so it reads as a dot.
const SEED_D: f32 = 2.2;

/// One stitch of your own thread, as its dots (see [`seed_dots`]).
pub fn paint_seed(window: &mut Window, d: Dash, y: Pixels, color: Hsla) {
    let r = px(SEED_D / 2.);
    for cx in seed_dots(d) {
        let b = Bounds::new(point(px(cx) - r, y - r + px(d.dy)), size(r * 2., r * 2.));
        window.paint_quad(fill(b, color).corner_radii(r));
    }
}

/// One thread's words on one row of text: where its stitches run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    pub x0: f32,
    pub x1: f32,
    pub seed: u64,
    /// A correction's first row starts with a cross-stitch: no stitch in its first 11 px.
    pub cross: bool,
}

/// The stitches for every thread on one row of text, each with the index of the run that
/// owns it. A thread alone keeps its own running stitch. Where threads share words (Ask
/// the room puts several minds on the same quote), they share one running stitch there
/// and take its stitches in turn, a stitch each in the order given, so every mind's
/// colour shows rather than only the one drawn last.
pub fn shared_stitches(runs: &[Run]) -> Vec<(usize, Dash)> {
    let mut order: Vec<usize> = (0..runs.len()).collect();
    order.sort_by(|&a, &b| runs[a].x0.total_cmp(&runs[b].x0).then(a.cmp(&b)));
    let mut out = vec![];
    let mut i = 0;
    while i < order.len() {
        // the runs that overlap this one, directly or through each other
        let mut group = vec![order[i]];
        let mut end = runs[order[i]].x1;
        let mut j = i + 1;
        while j < order.len() && runs[order[j]].x0 < end - 0.5 {
            end = end.max(runs[order[j]].x1);
            group.push(order[j]);
            j += 1;
        }
        let first = runs[order[i]];
        if group.len() == 1 {
            out.extend(stitch_dashes(first.x0, first.x1, first.seed, first.cross).into_iter().map(|d| (order[i], d)));
        } else {
            group.sort();
            for (k, d) in stitch_dashes(first.x0, end, first.seed, false).into_iter().enumerate() {
                if group.iter().any(|&g| runs[g].cross && d.x1 > runs[g].x0 && d.x0 < runs[g].x0 + CROSS_ROOM) {
                    continue;
                }
                let mid = (d.x0 + d.x1) / 2.;
                let here: Vec<usize> = group.iter().copied().filter(|&g| runs[g].x0 <= mid && mid < runs[g].x1).collect();
                if !here.is_empty() {
                    out.push((here[k % here.len()], d));
                }
            }
        }
        i = j;
    }
    out
}

/// Where the keyboard is: a running stitch round a control, 3 px outside `bounds`, its
/// corners following the control's own `radius`.
pub fn focus_stitch(window: &mut Window, bounds: Bounds<Pixels>, radius: Pixels, color: Hsla) {
    let out = px(3.);
    let b = Bounds::new(bounds.origin - point(out, out), size(bounds.size.width + out * 2., bounds.size.height + out * 2.));
    let r = (radius + out).min(b.size.height / 2.).min(b.size.width / 2.).max(px(0.));
    let corner = point(r, r);
    let mut p = PathBuilder::stroke(px(1.5)).dash_array(&[px(4.5), px(3.)]);
    p.move_to(point(b.left() + r, b.top()));
    p.line_to(point(b.right() - r, b.top()));
    p.arc_to(corner, px(0.), false, true, point(b.right(), b.top() + r));
    p.line_to(point(b.right(), b.bottom() - r));
    p.arc_to(corner, px(0.), false, true, point(b.right() - r, b.bottom()));
    p.line_to(point(b.left() + r, b.bottom()));
    p.arc_to(corner, px(0.), false, true, point(b.left(), b.bottom() - r));
    p.line_to(point(b.left(), b.top() + r));
    p.arc_to(corner, px(0.), false, true, point(b.left() + r, b.top()));
    if let Ok(path) = p.build() {
        window.paint_path(path, color);
    }
}

/// A cross-stitch: marks a correction, so a correction differs from a question in shape
/// and not only in colour.
pub fn cross_stitch(window: &mut Window, center: Point<Pixels>, half: Pixels, color: Hsla) {
    let mut b = PathBuilder::stroke(px(1.75));
    b.move_to(center - point(half, half * 0.9));
    b.line_to(center + point(half, half * 0.9));
    b.move_to(center + point(half, -half * 0.9));
    b.line_to(center + point(-half, half * 0.9));
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// A tacking stitch: the quiet mark beside a paragraph Inkling has read and has
/// nothing to ask about.
pub fn tacking_stitch(window: &mut Window, center: Point<Pixels>, color: Hsla) {
    let h = px(1.8);
    let b = Bounds::new(center - point(px(4.), h / 2.), size(px(8.), h));
    window.paint_quad(fill(b, color).corner_radii(h / 2.));
}

/// A French knot: a bead of thread wound round the needle. Marks a paragraph with a
/// question, ties off a resolved thread, and dots the i of the wordmark.
pub fn french_knot(window: &mut Window, center: Point<Pixels>, r: Pixels, color: Hsla, scale: f32) {
    let r = r * scale.max(0.);
    if r < px(0.3) {
        return;
    }
    window.paint_quad(fill(Bounds::new(center - point(r, r), size(r * 2., r * 2.)), color).corner_radii(r));
    // one coil of lighter thread, from the middle out
    let mut b = PathBuilder::stroke((r * 0.22).max(px(0.6)));
    let steps = 22;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let a = -2.0 + t * 1.3 * 2. * PI;
        let rr = r * (0.16 + 0.6 * t);
        let pt = center + point(rr * a.cos(), rr * a.sin());
        if i == 0 { b.move_to(pt) } else { b.line_to(pt) }
    }
    if let Ok(path) = b.build() {
        window.paint_path(path, lighter(color, 0.16));
    }
}

/// The needle, going in and out of the cloth while Inkling reads a paragraph.
/// `phase` runs 0..1 and loops.
pub fn needle(window: &mut Window, base: Point<Pixels>, phase: f32, steel: Hsla) {
    let lift = px(3.5 * (phase * 2. * PI).sin());
    let tip = base + point(px(0.), lift + px(7.));
    let eye = base + point(px(1.6), lift - px(9.));
    let mut b = PathBuilder::stroke(px(1.4));
    b.move_to(tip);
    b.line_to(eye);
    if let Ok(path) = b.build() {
        window.paint_path(path, steel);
    }
    // the eye
    let mut e = PathBuilder::stroke(px(0.9));
    e.move_to(eye + point(px(-0.9), px(-2.6)));
    e.cubic_bezier_to(eye + point(px(-0.9), px(1.6)), eye + point(px(1.4), px(-2.6)), eye + point(px(1.4), px(1.6)));
    if let Ok(path) = e.build() {
        window.paint_path(path, steel);
    }
}

/// One motif of the sampler border: a small cross-stitched diamond. A note gets one
/// for every thread tied off in it.
pub fn border_motif(window: &mut Window, center: Point<Pixels>, color: Hsla, scale: f32) {
    let s = 4.4 * scale.clamp(0., 1.);
    if s <= 0.05 {
        return;
    }
    for (dx, dy) in [(0., -1.), (-1., 0.), (1., 0.), (0., 1.), (0., 0.)] {
        cross_stitch(window, center + point(px(dx * s * 1.55), px(dy * s * 1.55)), px(s * 0.52), color);
    }
}

// ---------------------------------------------------------------- the thread

/// Points in the rope. Enough to curve smoothly over a few hundred pixels.
const POINTS: usize = 24;
/// Pixels per second squared. Thread is light: it sags, it does not plummet.
const GRAVITY: f32 = 760.;
/// Fraction of velocity kept each step: sways for about half a second, then settles.
const DAMPING: f32 = 0.965;
/// Constraint passes per step: more passes, stiffer thread.
const ITERATIONS: usize = 16;
/// A light pull toward the line between neighbours, so small wobbles iron out.
const BENDING: f32 = 0.04;
/// The sharpest a joint may bend, as the closest two points one apart may come (a
/// fraction of two segments). Limits kinks without shortening the thread.
const MIN_SPAN: f32 = 1.955;
/// Extra length over the straight distance when the thread hangs loose.
const SLACK: f32 = 0.08;
/// ...but never more than this, so a long thread does not droop like a washing line.
const MAX_SLACK: f32 = 30.;
/// Fixed simulation step; frames run one or more of these.
const DT: f32 = 1. / 120.;
/// Settling stops once the thread has been this still for [`QUIET_STEPS`] steps in a row:
/// the most any point moved in a step, in pixels.
const QUIET: f32 = 0.02;
const QUIET_STEPS: usize = 12;
/// ...and never runs longer than this, whatever the thread does.
const SETTLE_MAX: usize = 480;

/// One loose thread from the hole it comes up through (`a`) to the tag's loop (`b`).
#[derive(Clone, Debug)]
pub struct Rope {
    pts: Vec<Point<f32>>,
    prev: Vec<Point<f32>>,
    a: Point<f32>,
    b: Point<f32>,
    /// 1 hangs loose, 0 is pulled straight (tied off).
    slack: f32,
    /// Where `slack` is heading, and how fast (per second).
    slack_to: f32,
    /// False once the thread is cut: the tag end falls.
    pinned_b: bool,
    /// Set while anything is still moving; the page asks for frames until it clears.
    pub moving: bool,
}

fn p32(p: Point<Pixels>) -> Point<f32> {
    point(p.x.as_f32(), p.y.as_f32())
}

impl Rope {
    /// A thread already at rest between `a` and `b`.
    pub fn new(a: Point<Pixels>, b: Point<Pixels>) -> Self {
        let (a, b) = (p32(a), p32(b));
        let mut r = Rope { pts: vec![], prev: vec![], a, b, slack: 1., slack_to: 1., pinned_b: true, moving: false };
        r.lay_out();
        r.settle();
        r
    }

    /// Lay the points on a gentle bow between the ends, bowed across the line between
    /// them on the side gravity pulls toward. Sagging the points straight down instead
    /// bunches them along a steep thread, and the extra length then buckles into a
    /// zig-zag that never settles.
    fn lay_out(&mut self) {
        let d = dist(self.a, self.b).max(0.001);
        let (ux, uy) = ((self.b.x - self.a.x) / d, (self.b.y - self.a.y) / d);
        let (mut nx, mut ny) = (-uy, ux);
        if ny < 0. || (ny.abs() < 1e-3 && nx * (self.b.x - self.a.x) < 0.) {
            nx = -nx;
            ny = -ny;
        }
        let bow = (d * 0.16).min(MAX_SLACK * 1.4);
        self.pts = (0..POINTS)
            .map(|i| {
                let t = i as f32 / (POINTS - 1) as f32;
                let k = bow * 4. * t * (1. - t);
                point(self.a.x + (self.b.x - self.a.x) * t + nx * k, self.a.y + (self.b.y - self.a.y) * t + ny * k)
            })
            .collect();
        self.prev = self.pts.clone();
    }

    fn segment(&self) -> f32 {
        let d = dist(self.a, self.b);
        // Tied off, it is a hair shorter than the gap, so it pulls straight under tension.
        (d * (1. - 0.005 * (1. - self.slack)) + (d * SLACK).min(MAX_SLACK) * self.slack) / (POINTS - 1) as f32
    }

    /// Move the ends. Typing shifts the words and the tags; the thread follows without
    /// swaying (`animate` false), because motion must never pull the eye while you type.
    pub fn set_ends(&mut self, a: Point<Pixels>, b: Point<Pixels>, animate: bool) {
        let (a, b) = (p32(a), p32(b));
        let moved = dist(a, self.a) > 0.5 || (self.pinned_b && dist(b, self.b) > 0.5);
        if !moved {
            return;
        }
        let far = dist(a, self.a) > 160. || dist(b, self.b) > 160.;
        let shift = point(a.x - self.a.x, a.y - self.a.y);
        // Both ends moved together (the words above reflowed, the page moved): the thread
        // keeps its shape and goes with them, nothing to solve.
        let together = self.pinned_b && (b.x - self.b.x - shift.x).abs() < 0.01 && (b.y - self.b.y - shift.y).abs() < 0.01;
        self.a = a;
        if self.pinned_b {
            self.b = b;
        }
        if together {
            for p in self.pts.iter_mut().chain(self.prev.iter_mut()) {
                p.x += shift.x;
                p.y += shift.y;
            }
        } else if animate && !far {
            // mid-animation (a tag folding while its thread ties off): let it follow
            self.moving = true;
        } else {
            // Re-hang it from a fresh sag. Letting the solver drag a settled thread to
            // ends that jumped can leave it zig-zagged; a fresh sag always settles smooth.
            self.lay_out();
            self.settle();
        }
    }

    /// Tie off: pull the slack out over about 320 ms.
    pub fn tie(&mut self) {
        self.slack_to = 0.;
        self.moving = true;
    }

    /// Untie (a reopened thread hangs loose again).
    pub fn loosen(&mut self) {
        self.slack_to = 1.;
        self.moving = true;
    }

    pub fn is_tied(&self) -> bool {
        self.slack_to == 0. && self.slack < 0.02
    }

    /// Cut at the tag: the free end drops.
    pub fn cut(&mut self) {
        self.pinned_b = false;
        self.moving = true;
    }

    /// Advance by `seconds` of simulated time.
    pub fn advance(&mut self, seconds: f32) {
        let steps = ((seconds / DT).round() as usize).clamp(1, 8);
        for _ in 0..steps {
            self.step();
        }
    }

    /// Run the thread to rest: until no point has moved more than [`QUIET`] for
    /// [`QUIET_STEPS`] steps running, or [`SETTLE_MAX`] steps. Returns the steps it took.
    fn settle(&mut self) -> usize {
        let mut quiet = 0;
        let mut steps = 0;
        while steps < SETTLE_MAX {
            let motion = self.step();
            steps += 1;
            quiet = if motion < QUIET { quiet + 1 } else { 0 };
            if quiet >= QUIET_STEPS {
                break;
            }
        }
        self.prev = self.pts.clone();
        self.moving = false;
        steps
    }

    /// One step of the simulation; returns the most any point moved, in pixels.
    fn step(&mut self) -> f32 {
        // ease the slack toward its target (tie-off takes about 0.32 s)
        if self.slack != self.slack_to {
            let rate = DT / 0.32;
            self.slack = if self.slack > self.slack_to { (self.slack - rate).max(self.slack_to) } else { (self.slack + rate).min(self.slack_to) };
        }
        let last = POINTS - 1;
        let mut motion = 0f32;
        for i in 1..POINTS {
            if i == last && self.pinned_b {
                continue;
            }
            let p = self.pts[i];
            let v = point((p.x - self.prev[i].x) * DAMPING, (p.y - self.prev[i].y) * DAMPING);
            self.prev[i] = p;
            // A taut thread barely feels its weight; a loose one sags.
            let g = GRAVITY * (0.03 + 0.97 * self.slack);
            self.pts[i] = point(p.x + v.x, p.y + v.y + g * DT * DT);
            motion = motion.max(v.x.abs() + v.y.abs());
        }
        let seg = self.segment();
        for _ in 0..ITERATIONS {
            self.pts[0] = self.a;
            if self.pinned_b {
                self.pts[last] = self.b;
            }
            for i in 0..last {
                let (p, q) = (self.pts[i], self.pts[i + 1]);
                let d = dist(p, q).max(0.0001);
                let diff = (d - seg) / d;
                let fixed_p = i == 0;
                let fixed_q = i + 1 == last && self.pinned_b;
                let (wp, wq) = match (fixed_p, fixed_q) {
                    (true, true) => (0., 0.),
                    (true, false) => (0., 1.),
                    (false, true) => (1., 0.),
                    (false, false) => (0.5, 0.5),
                };
                let dx = (q.x - p.x) * diff;
                let dy = (q.y - p.y) * diff;
                self.pts[i] = point(p.x + dx * wp, p.y + dy * wp);
                self.pts[i + 1] = point(q.x - dx * wq, q.y - dy * wq);
            }
            // no sharp joints: points two apart are kept at least MIN_SPAN segments apart
            for i in 0..last - 1 {
                let (p, q) = (self.pts[i], self.pts[i + 2]);
                let d = dist(p, q).max(0.0001);
                let min = seg * MIN_SPAN;
                if d >= min {
                    continue;
                }
                let diff = (d - min) / d * 0.5;
                let fixed_p = i == 0;
                let fixed_q = i + 2 == last && self.pinned_b;
                let (wp, wq) = match (fixed_p, fixed_q) {
                    (true, true) => (0., 0.),
                    (true, false) => (0., 1.),
                    (false, true) => (1., 0.),
                    (false, false) => (0.5, 0.5),
                };
                let dx = (q.x - p.x) * diff;
                let dy = (q.y - p.y) * diff;
                self.pts[i] = point(p.x + dx * wp, p.y + dy * wp);
                self.pts[i + 2] = point(q.x - dx * wq, q.y - dy * wq);
            }
            // and a light, even pull toward the middle of each point's neighbours
            let mids: Vec<Point<f32>> = (1..last)
                .map(|i| point((self.pts[i - 1].x + self.pts[i + 1].x) / 2., (self.pts[i - 1].y + self.pts[i + 1].y) / 2.))
                .collect();
            for i in 1..last {
                if i + 1 == last && !self.pinned_b {
                    continue;
                }
                let mid = mids[i - 1];
                self.pts[i] = point(self.pts[i].x + (mid.x - self.pts[i].x) * BENDING, self.pts[i].y + (mid.y - self.pts[i].y) * BENDING);
            }
        }
        self.pts[0] = self.a;
        if self.pinned_b {
            self.pts[last] = self.b;
        }
        let slack_moving = (self.slack - self.slack_to).abs() > 0.001;
        self.moving = slack_moving || motion > QUIET || !self.pinned_b;
        motion
    }

    /// The thread as drawn: the simulated points, smoothed twice with a 1-2-1 filter
    /// (ends kept), so sub-pixel jitter in the simulation never shows as a wobble.
    pub fn shape(&self) -> Vec<Point<f32>> {
        let mut pts = self.pts.clone();
        for _ in 0..2 {
            let prev = pts.clone();
            for i in 1..pts.len() - 1 {
                pts[i] = point((prev[i - 1].x + 2. * prev[i].x + prev[i + 1].x) / 4., (prev[i - 1].y + 2. * prev[i].y + prev[i + 1].y) / 4.);
            }
        }
        pts
    }

    /// The tag end of the thread (where the knot goes once it is tied).
    pub fn end(&self) -> Point<Pixels> {
        let e = self.pts[POINTS - 1];
        point(px(e.x), px(e.y))
    }

    /// The lowest point a cut thread has fallen to, relative to where it hangs from.
    #[cfg(test)]
    pub fn drop_depth(&self) -> f32 {
        self.pts[POINTS - 1].y - self.a.y
    }
}

fn dist(a: Point<f32>, b: Point<f32>) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Draw a thread: a soft shadow on the cloth, the thread itself, and a faint dashed
/// highlight along it that reads as twisted strands. `origin` maps rope space to the
/// window.
pub fn paint_rope(window: &mut Window, origin: Point<Pixels>, rope: &Rope, color: Hsla, shadow: Hsla, opacity: f32) {
    let pts: Vec<Point<Pixels>> = rope.shape().iter().map(|p| origin + point(px(p.x), px(p.y))).collect();
    let smooth = |b: &mut PathBuilder, off: Point<Pixels>| {
        b.move_to(pts[0] + off);
        for i in 1..pts.len() - 1 {
            let mid = point((pts[i].x + pts[i + 1].x) / 2., (pts[i].y + pts[i + 1].y) / 2.);
            b.curve_to(mid + off, pts[i] + off);
        }
        b.line_to(pts[pts.len() - 1] + off);
    };
    let mut s = PathBuilder::stroke(px(2.2));
    smooth(&mut s, point(px(0.6), px(1.5)));
    if let Ok(path) = s.build() {
        window.paint_path(path, shadow.opacity(opacity));
    }
    let mut b = PathBuilder::stroke(px(1.75));
    smooth(&mut b, point(px(0.), px(0.)));
    if let Ok(path) = b.build() {
        window.paint_path(path, color.opacity(opacity));
    }
    let mut t = PathBuilder::stroke(px(0.7)).dash_array(&[px(1.3), px(2.9)]);
    smooth(&mut t, point(px(0.15), px(-0.35)));
    if let Ok(path) = t.build() {
        window.paint_path(path, lighter(color, 0.2).opacity(0.55 * opacity));
    }
}

/// The hole the thread comes up through: a tiny shadowed dot in the cloth.
pub fn hole(window: &mut Window, at: Point<Pixels>, color: Hsla) {
    let r = px(1.5);
    window.paint_quad(fill(Bounds::new(at - point(r, r), size(r * 2., r * 2.)), color).corner_radii(r));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn len(r: &Rope) -> f32 {
        r.pts.windows(2).map(|w| dist(w[0], w[1])).sum()
    }

    /// Changes of bend direction along the drawn thread, ignoring sub-pixel noise.
    fn turns(r: &Rope) -> usize {
        let pts = r.shape();
        let mut turns = 0;
        let mut last = 0f32;
        for w in pts.windows(3) {
            let c = (w[2].x - w[1].x) * (w[1].y - w[0].y) - (w[2].y - w[1].y) * (w[1].x - w[0].x);
            if c.abs() < 2. {
                continue;
            }
            if c * last < 0. {
                turns += 1;
            }
            last = c;
        }
        turns
    }

    #[std::prelude::v1::test]
    fn seed_stitches_stay_on_their_words_with_an_even_step() {
        let dashes = stitch_dashes(10., 210., seed_of("own"), false);
        let dots: Vec<f32> = dashes.iter().flat_map(|d| seed_dots(*d)).collect();
        assert!(dots.len() >= 30, "two dots a stitch: {}", dots.len());
        for (d, c) in dashes.iter().flat_map(|d| seed_dots(*d).into_iter().map(move |c| (*d, c))) {
            assert!(c - SEED_D / 2. >= d.x0 - 0.01 && c + SEED_D / 2. <= d.x1 + 0.01, "a dot inside its stitch");
        }
        for w in dots.windows(2) {
            let step = w[1] - w[0];
            assert!((3.0..=7.5).contains(&step), "even enough to read as one dotted row: {step}");
        }
        // a stitch cut short by the end of the words keeps one dot, centred
        assert_eq!(seed_dots(Dash { x0: 0., x1: 3., dy: 0. }), vec![1.5]);
    }

    #[std::prelude::v1::test]
    fn a_loose_thread_hangs_below_the_straight_line_and_never_kinks() {
        let r = Rope::new(point(px(0.), px(0.)), point(px(200.), px(-40.)));
        assert!(!r.moving, "settled before first paint");
        // ends are pinned
        assert!(dist(r.pts[0], point(0., 0.)) < 0.01 && dist(r.pts[POINTS - 1], point(200., -40.)) < 0.01);
        // it sags: the middle sits below the chord
        let mid = r.pts[POINTS / 2];
        assert!(mid.y > -20. + 4., "mid {mid:?} should hang below the chord");
        // longer than the straight distance, by about the slack
        let straight = dist(point(0., 0.), point(200., -40.));
        assert!(len(&r) > straight + 4. && len(&r) < straight + MAX_SLACK + 2., "length {} vs {straight}", len(&r));
        assert!(turns(&r) <= 1, "smooth: {} changes of bend", turns(&r));
        // no kinks: neighbouring segments never turn more than ~35 degrees
        for w in r.pts.windows(3) {
            let (u, v) = (point(w[1].x - w[0].x, w[1].y - w[0].y), point(w[2].x - w[1].x, w[2].y - w[1].y));
            let cos = (u.x * v.x + u.y * v.y) / (dist(w[0], w[1]) * dist(w[1], w[2])).max(1e-4);
            assert!(cos > 0.82, "kink: cos {cos}");
        }
    }

    #[std::prelude::v1::test]
    fn tying_off_pulls_it_straight() {
        let mut r = Rope::new(point(px(0.), px(0.)), point(px(240.), px(30.)));
        r.tie();
        for _ in 0..90 {
            r.advance(1. / 60.);
        }
        let straight = dist(point(0., 0.), point(240., 30.));
        assert!(len(&r) < straight + 0.5, "taut: {} vs {straight}", len(&r));
        // every point within a pixel of the straight line
        for p in &r.pts {
            let off = (30. * p.x - 240. * p.y).abs() / straight;
            assert!(off < 1., "point {p:?} is {off} px off the line");
        }
        assert!(r.is_tied());
    }

    #[std::prelude::v1::test]
    fn a_cut_thread_falls() {
        let mut r = Rope::new(point(px(0.), px(0.)), point(px(160.), px(0.)));
        let before = r.drop_depth();
        r.cut();
        for _ in 0..60 {
            r.advance(1. / 60.);
        }
        assert!(r.drop_depth() > before + 40., "fell from {before} to {}", r.drop_depth());
    }

    #[std::prelude::v1::test]
    fn settling_stops_once_the_thread_is_still_and_lands_where_the_long_run_would() {
        let mut r = Rope::new(point(px(0.), px(0.)), point(px(220.), px(-30.)));
        r.lay_out();
        let steps = r.settle();
        assert!(steps < SETTLE_MAX / 2, "{steps} steps to rest");
        let early = r.pts.clone();
        r.lay_out();
        for _ in 0..SETTLE_MAX {
            r.step();
        }
        for (e, l) in early.iter().zip(&r.pts) {
            assert!(dist(*e, *l) < 0.5, "early {e:?} vs long {l:?}");
        }
    }

    #[std::prelude::v1::test]
    fn a_thread_whose_ends_move_together_keeps_its_shape() {
        let mut r = Rope::new(point(px(10.), px(20.)), point(px(200.), px(60.)));
        let before = r.pts.clone();
        r.set_ends(point(px(10.), px(44.)), point(px(200.), px(84.)), false);
        assert!(!r.moving);
        for (b, p) in before.iter().zip(&r.pts) {
            assert!((p.x - b.x).abs() < 1e-4 && (p.y - b.y - 24.).abs() < 1e-4, "{b:?} -> {p:?}");
        }
        // one end alone moving is a new hang, not a shift
        let before = r.pts.clone();
        r.set_ends(point(px(10.), px(44.)), point(px(240.), px(84.)), false);
        assert!(before.iter().zip(&r.pts).any(|(b, p)| dist(*b, *p) > 1.));
    }

    #[std::prelude::v1::test]
    fn a_tag_that_jumps_leaves_no_zigzag() {
        // tag 2 pushed 100 px down when tag 1 opens above it
        let mut r = Rope::new(point(px(950.), px(370.)), point(px(990.), px(413.)));
        r.set_ends(point(px(950.), px(370.)), point(px(990.), px(513.)), false);
        assert!(turns(&r) <= 1, "{} changes of bend: zig-zag", turns(&r));
    }

    #[std::prelude::v1::test]
    fn moving_the_ends_while_typing_does_not_sway() {
        let mut r = Rope::new(point(px(0.), px(0.)), point(px(200.), px(0.)));
        r.set_ends(point(px(0.), px(24.)), point(px(200.), px(24.)), false);
        assert!(!r.moving);
        assert!((r.pts[0].y - 24.).abs() < 0.01);
    }

    fn run(x0: f32, x1: f32) -> Run {
        Run { x0, x1, seed: 7, cross: false }
    }

    #[std::prelude::v1::test]
    fn a_thread_alone_keeps_its_own_running_stitch() {
        let alone = shared_stitches(&[Run { x0: 10., x1: 210., seed: 42, cross: true }]);
        let own: Vec<(usize, Dash)> = stitch_dashes(10., 210., 42, true).into_iter().map(|d| (0, d)).collect();
        assert_eq!(alone, own);
        // two threads on different words of the same row each keep theirs
        let apart = shared_stitches(&[run(0., 80.), Run { x0: 120., x1: 200., seed: 9, cross: false }]);
        assert!(apart.iter().filter(|(i, _)| *i == 0).all(|(_, d)| d.x1 <= 80.));
        assert!(apart.iter().filter(|(i, _)| *i == 1).all(|(_, d)| d.x0 >= 120.));
        assert_eq!(apart.iter().filter(|(i, _)| *i == 1).count(), stitch_dashes(120., 200., 9, false).len());
    }

    #[std::prelude::v1::test]
    fn threads_on_the_same_words_take_the_stitches_in_turn() {
        let two = shared_stitches(&[run(0., 300.), run(0., 300.)]);
        let owners: Vec<usize> = two.iter().map(|(i, _)| *i).collect();
        assert!(owners.windows(2).all(|w| w[0] != w[1]), "turn and turn about: {owners:?}");
        // one running stitch: no stitch drawn over another
        assert!(two.windows(2).all(|w| w[1].1.x0 >= w[0].1.x1), "stitches overlap");
        // three minds each get a third
        let three = shared_stitches(&[run(0., 300.), run(0., 300.), run(0., 300.)]);
        let counts: Vec<usize> = (0..3).map(|m| three.iter().filter(|(i, _)| *i == m).count()).collect();
        assert!(counts.iter().all(|c| *c >= 8) && counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1, "{counts:?}");
    }

    #[std::prelude::v1::test]
    fn words_shared_in_part_share_only_the_part() {
        let s = shared_stitches(&[run(0., 120.), run(60., 200.)]);
        for (i, d) in &s {
            let mid = (d.x0 + d.x1) / 2.;
            if mid < 60. {
                assert_eq!(*i, 0, "only the first thread before the second starts");
            } else if mid >= 120. {
                assert_eq!(*i, 1, "only the second after the first ends");
            }
        }
        let shared: Vec<usize> = s.iter().filter(|(_, d)| (60.0..120.0).contains(&((d.x0 + d.x1) / 2.))).map(|(i, _)| *i).collect();
        assert!(shared.contains(&0) && shared.contains(&1), "both colours where both threads are: {shared:?}");
    }

    #[std::prelude::v1::test]
    fn a_shared_correction_keeps_room_for_its_cross() {
        let s = shared_stitches(&[Run { x0: 40., x1: 240., seed: 3, cross: true }, run(40., 240.)]);
        assert!(s.iter().all(|(_, d)| d.x1 <= 40. || d.x0 >= 40. + CROSS_ROOM));
        assert!(s.iter().any(|(i, _)| *i == 0) && s.iter().any(|(i, _)| *i == 1));
    }

    #[std::prelude::v1::test]
    fn stitches_are_steady() {
        assert_eq!(jitter(seed_of("t1"), 3), jitter(seed_of("t1"), 3));
        assert_ne!(seed_of("t1"), seed_of("t2"));
        for i in 0..100 {
            let j = jitter(42, i);
            assert!((-0.5..=0.5).contains(&j));
        }
    }
}

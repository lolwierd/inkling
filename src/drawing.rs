//! Ink drawings. The mind sends a small SVG whose groups are steps
//! (`<g data-step data-caption>`). We parse the subset we ask for, flatten every
//! stroke to a polyline, and paint it so each step draws itself like ink.
//!
//! The SVG comes from a model, so the parser expects to be surprised: transforms,
//! wrapper groups, styles on groups, missing sizes, broken numbers. Whatever it keeps
//! is finite and of sane size, because the tessellator downstream asserts on NaN
//! (and panics in debug builds).

use gpui_kit::*;
use std::{rc::Rc, str::FromStr, time::Instant};

use crate::theme::{SANS, Theme};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tone {
    Ink,
    Graphite,
    Faint,
    Rule,
    Wash,
    Paper,
    None,
}

/// A colour name (ours, a CSS variable, or a few CSS ones) as a tone; anything
/// else keeps `default`. Hex colours map by lightness, so a pale hex fill becomes a
/// wash rather than a block of ink over the rest of the sketch.
fn tone(name: Option<&str>, default: Tone, fill: bool) -> Tone {
    let Some(raw) = name else { return default };
    let n = raw.trim().to_ascii_lowercase();
    let n = n.trim_start_matches("var(--").trim_end_matches(')');
    match n {
        "none" | "transparent" => Tone::None,
        "ink" | "riddle" | "currentcolor" | "black" => Tone::Ink,
        "graphite" | "gray" | "grey" => Tone::Graphite,
        "faint" => Tone::Faint,
        "rule" => Tone::Rule,
        "wash" => Tone::Wash,
        "paper" | "white" => Tone::Paper,
        _ => match hex_lightness(n) {
            Some(l) if l < 0.3 => Tone::Ink,
            Some(l) if l < 0.55 => Tone::Graphite,
            Some(l) if l < 0.8 => Tone::Faint,
            Some(_) if fill => Tone::Wash,
            Some(_) => Tone::Rule,
            None => default,
        },
    }
}

/// Lightness (0 black, 1 white) of `#rgb` or `#rrggbb`.
fn hex_lightness(s: &str) -> Option<f32> {
    let hex = s.strip_prefix('#')?;
    let channels: Vec<u8> = match hex.len() {
        3 => hex.chars().map(|c| c.to_digit(16).map(|d| (d * 17) as u8)).collect::<Option<_>>()?,
        6 => (0..3).map(|i| u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()).collect::<Option<_>>()?,
        _ => return None,
    };
    let max = *channels.iter().max()? as f32;
    let min = *channels.iter().min()? as f32;
    Some((max + min) / 2. / 255.)
}

#[derive(Clone, Debug)]
pub enum Shape {
    /// A stroked (and maybe filled) polyline per subpath, in viewBox units.
    Path { subpaths: Vec<(Vec<(f32, f32)>, bool)>, stroke: Tone, fill: Tone, width: f32 },
    Text { x: f32, y: f32, text: String, size: f32, voice: bool, anchor: TextAnchor, fill: Tone, weight_bold: bool },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub caption: String,
    pub shapes: Vec<Shape>,
}

#[derive(Clone, Debug)]
pub struct Drawing {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
    pub steps: Vec<Step>,
}

/// Largest coordinate kept, in viewBox units. Sketches live within a few hundred
/// units; far beyond that a number is broken, and stroke lengths (squares of
/// coordinates) would overflow to infinity.
const LIMIT: f32 = 100_000.;

/// The size used when an SVG gives neither a viewBox nor its content anything to measure.
const DEFAULT_SIZE: (f32, f32) = (320., 200.);

/// An SVG `transform`: x' = a·x + c·y + e, y' = b·x + d·y + f.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Affine {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Affine {
    const IDENTITY: Affine = Affine { a: 1., b: 0., c: 0., d: 1., e: 0., f: 0. };

    /// `self` applied after `inner` (a parent's transform, then its child's).
    fn then(self, inner: Affine) -> Affine {
        Affine {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }

    fn apply(self, (x, y): (f32, f32)) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// How much lengths grow under the transform (the square root of its area scale).
    fn scale(self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
}

/// The node's own `transform`, or the identity when it has none or it does not parse.
fn transform_of(node: &roxmltree::Node) -> Affine {
    let Some(t) = node.attribute("transform").and_then(|raw| svgtypes::Transform::from_str(raw).ok()) else {
        return Affine::IDENTITY;
    };
    let m = Affine { a: t.a as f32, b: t.b as f32, c: t.c as f32, d: t.d as f32, e: t.e as f32, f: t.f as f32 };
    if [m.a, m.b, m.c, m.d, m.e, m.f].iter().all(|v| v.is_finite()) { m } else { Affine::IDENTITY }
}

/// A presentation attribute, from `style="…"` first (CSS beats attributes), then the attribute.
fn attr<'a>(node: &'a roxmltree::Node, name: &str) -> Option<&'a str> {
    node.attribute("style")
        .and_then(|style| {
            style.split(';').find_map(|decl| {
                let (k, v) = decl.split_once(':')?;
                (k.trim() == name).then(|| v.trim())
            })
        })
        .or_else(|| node.attribute(name))
}

/// A finite number, allowing a `px` unit.
fn number(raw: &str) -> Option<f32> {
    let v: f32 = raw.trim().trim_end_matches("px").trim().parse().ok()?;
    v.is_finite().then_some(v)
}

/// A numeric attribute; missing or broken reads as 0, as SVG does for missing ones.
fn num(node: &roxmltree::Node, name: &str) -> f32 {
    attr(node, name).and_then(number).unwrap_or(0.)
}

/// Presentation attributes in effect on a node: its own, else its group's.
#[derive(Clone, Copy, Debug)]
struct Style {
    stroke: Tone,
    /// None until something sets it: then shapes are unfilled and text is ink.
    fill: Option<Tone>,
    width: f32,
    font_size: f32,
    anchor: TextAnchor,
    sans: bool,
    bold: bool,
}

impl Style {
    const ROOT: Style =
        Style { stroke: Tone::Ink, fill: None, width: 1.2, font_size: 14., anchor: TextAnchor::Start, sans: false, bold: false };

    fn of(self, node: &roxmltree::Node) -> Style {
        let class = node.attribute("class").unwrap_or("");
        let family = attr(node, "font-family");
        Style {
            stroke: tone(attr(node, "stroke"), self.stroke, false),
            fill: attr(node, "fill").map(|f| tone(Some(f), self.fill.unwrap_or(Tone::None), true)).or(self.fill),
            width: attr(node, "stroke-width").and_then(number).map_or(self.width, |w| w.clamp(0.1, 20.)),
            font_size: attr(node, "font-size").and_then(number).map_or(self.font_size, |s| s.clamp(8., 28.)),
            anchor: match attr(node, "text-anchor") {
                Some("middle") => TextAnchor::Middle,
                Some("end") => TextAnchor::End,
                Some("start") => TextAnchor::Start,
                _ => self.anchor,
            },
            sans: if class.contains("sans") || family.is_some_and(|f| f.contains("sans")) {
                true
            } else if class.contains("voice") || family.is_some() {
                false
            } else {
                self.sans
            },
            bold: attr(node, "font-weight")
                .map_or(self.bold, |w| w == "bold" || w == "bolder" || w.parse::<u32>().is_ok_and(|n| n >= 600)),
        }
    }
}

fn flatten_path(d: &str) -> Vec<(Vec<(f32, f32)>, bool)> {
    use svgtypes::{SimplePathSegment as S, SimplifyingPathParser};
    let mut out: Vec<(Vec<(f32, f32)>, bool)> = vec![];
    let mut cur: Vec<(f32, f32)> = vec![];
    let mut last = (0f32, 0f32);
    let mut start = (0f32, 0f32);
    for seg in SimplifyingPathParser::from(d) {
        let Ok(seg) = seg else { break };
        match seg {
            S::MoveTo { x, y } => {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), false));
                }
                cur.clear();
                last = (x as f32, y as f32);
                start = last;
                cur.push(last);
            }
            S::LineTo { x, y } => {
                last = (x as f32, y as f32);
                cur.push(last);
            }
            S::Quadratic { x1, y1, x, y } => {
                let (p0, c, p1) = (last, (x1 as f32, y1 as f32), (x as f32, y as f32));
                for i in 1..=12 {
                    let t = i as f32 / 12.;
                    let mt = 1. - t;
                    cur.push((
                        mt * mt * p0.0 + 2. * mt * t * c.0 + t * t * p1.0,
                        mt * mt * p0.1 + 2. * mt * t * c.1 + t * t * p1.1,
                    ));
                }
                last = p1;
            }
            S::CurveTo { x1, y1, x2, y2, x, y } => {
                let (p0, c1, c2, p1) = (last, (x1 as f32, y1 as f32), (x2 as f32, y2 as f32), (x as f32, y as f32));
                for i in 1..=16 {
                    let t = i as f32 / 16.;
                    let mt = 1. - t;
                    cur.push((
                        mt * mt * mt * p0.0 + 3. * mt * mt * t * c1.0 + 3. * mt * t * t * c2.0 + t * t * t * p1.0,
                        mt * mt * mt * p0.1 + 3. * mt * mt * t * c1.1 + 3. * mt * t * t * c2.1 + t * t * t * p1.1,
                    ));
                }
                last = p1;
            }
            S::ClosePath => {
                if !cur.is_empty() {
                    cur.push(start);
                    out.push((std::mem::take(&mut cur), true));
                }
                last = start;
            }
        }
    }
    if cur.len() > 1 {
        out.push((cur, false));
    }
    out
}

fn ellipse_points(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    let n = 40;
    (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

fn rect_points(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<(f32, f32)> {
    let r = r.min(w / 2.).min(h / 2.).max(0.);
    if r < 0.5 {
        return vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
    }
    let mut pts = vec![];
    let corner = |pts: &mut Vec<(f32, f32)>, cx: f32, cy: f32, a0: f32| {
        for i in 0..=6 {
            let a = a0 + i as f32 / 6. * std::f32::consts::FRAC_PI_2;
            pts.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    };
    use std::f32::consts::PI;
    corner(&mut pts, x + w - r, y + r, -PI / 2.);
    corner(&mut pts, x + w - r, y + h - r, 0.);
    corner(&mut pts, x + r, y + h - r, PI / 2.);
    corner(&mut pts, x + r, y + r, PI);
    pts.push(pts[0]);
    pts
}

fn points_attr(s: &str) -> Vec<(f32, f32)> {
    let nums: Vec<f32> = s
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    nums.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect()
}

fn sane(v: f32) -> bool {
    v.is_finite() && v.abs() <= LIMIT
}

/// One drawable element in viewBox units, with `m` (its transform and its
/// groups') applied; None for anything we do not draw or cannot trust.
fn shape_of(node: &roxmltree::Node, style: Style, m: Affine) -> Option<Shape> {
    let tag = node.tag_name().name();
    let path = |subpaths: Vec<(Vec<(f32, f32)>, bool)>| {
        let subpaths: Vec<_> = subpaths
            .into_iter()
            .map(|(pts, closed)| (pts.into_iter().map(|p| m.apply(p)).collect::<Vec<_>>(), closed))
            .filter(|(pts, _)| pts.len() > 1)
            .collect();
        let ok = !subpaths.is_empty() && subpaths.iter().all(|(pts, _)| pts.iter().all(|&(x, y)| sane(x) && sane(y)));
        let width = (style.width * m.scale()).clamp(0.1, 20.);
        ok.then_some(Shape::Path { subpaths, stroke: style.stroke, fill: style.fill.unwrap_or(Tone::None), width })
    };
    match tag {
        "path" => path(flatten_path(attr(node, "d")?)),
        "line" => path(vec![(vec![(num(node, "x1"), num(node, "y1")), (num(node, "x2"), num(node, "y2"))], false)]),
        "polyline" => path(vec![(points_attr(attr(node, "points")?), false)]),
        "polygon" => {
            let mut pts = points_attr(attr(node, "points")?);
            if let Some(first) = pts.first().copied() {
                pts.push(first);
            }
            path(vec![(pts, true)])
        }
        "rect" => {
            let (w, h) = (num(node, "width"), num(node, "height"));
            if w <= 0. || h <= 0. {
                return None;
            }
            let r = attr(node, "rx").or(attr(node, "ry")).and_then(number).unwrap_or(0.);
            path(vec![(rect_points(num(node, "x"), num(node, "y"), w, h, r), true)])
        }
        "circle" => {
            let r = num(node, "r");
            if r <= 0. {
                return None;
            }
            path(vec![(ellipse_points(num(node, "cx"), num(node, "cy"), r, r), true)])
        }
        "ellipse" => {
            let (rx, ry) = (num(node, "rx"), num(node, "ry"));
            if rx <= 0. || ry <= 0. {
                return None;
            }
            path(vec![(ellipse_points(num(node, "cx"), num(node, "cy"), rx, ry), true)])
        }
        "text" => {
            let text: String = node
                .descendants()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .collect::<Vec<_>>()
                .join("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if text.is_empty() {
                return None;
            }
            let (x, y) = m.apply((num(node, "x"), num(node, "y")));
            if !sane(x) || !sane(y) {
                return None;
            }
            // A label's own fill counts (bar "none": text is never invisible); a fill
            // it gets from its group counts only in a text colour, since a group's
            // wash or paper is meant for its shapes.
            let own = attr(node, "fill").is_some();
            let fill = match style.fill {
                None | Some(Tone::None) => Tone::Ink,
                Some(t) if own || matches!(t, Tone::Ink | Tone::Graphite | Tone::Faint) => t,
                Some(_) => Tone::Ink,
            };
            Some(Shape::Text {
                x,
                y,
                text,
                // Rotation and skew are not drawn; position and size follow the transform.
                size: (style.font_size * m.scale()).clamp(8., 28.),
                voice: !style.sans,
                anchor: style.anchor,
                fill,
                weight_bold: style.bold,
            })
        }
        _ => None,
    }
}

/// Containers whose children we draw. Everything else that is not a shape
/// (defs, style, script, foreignObject, title, use, …) is skipped with its children.
fn is_group(node: &roxmltree::Node) -> bool {
    matches!(node.tag_name().name(), "g" | "a")
}

fn hidden(node: &roxmltree::Node) -> bool {
    attr(node, "display") == Some("none") || attr(node, "visibility") == Some("hidden")
}

struct Collected {
    steps: Vec<Step>,
    loose: Vec<Shape>,
}

/// Walk `node`'s children. The outermost groups with `data-step` become steps,
/// wherever they sit (models like to wrap them in a positioning `<g>`); a step
/// inside a step is part of it. Shapes outside any step are loose.
fn collect(node: &roxmltree::Node, style: Style, m: Affine, step: Option<usize>, out: &mut Collected) {
    for child in node.children().filter(|n| n.is_element() && !hidden(n)) {
        let style = style.of(&child);
        let m = m.then(transform_of(&child));
        if is_group(&child) {
            let step = match step {
                None if child.attribute("data-step").is_some() => {
                    let caption = child.attribute("data-caption").unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
                    out.steps.push(Step { caption, shapes: vec![] });
                    Some(out.steps.len() - 1)
                }
                inside => inside,
            };
            collect(&child, style, m, step, out);
        } else if let Some(shape) = shape_of(&child, style, m) {
            match step {
                Some(i) => out.steps[i].shapes.push(shape),
                None => out.loose.push(shape),
            }
        }
    }
}

/// The viewBox, when it is four finite numbers with a positive size.
fn view_box(root: &roxmltree::Node) -> Option<(f32, f32, f32, f32)> {
    let vb: Vec<f32> = root
        .attribute("viewBox")?
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(number)
        .collect::<Option<_>>()?;
    match vb[..] {
        [x, y, w, h] if sane(x) && sane(y) && w > 0. && h > 0. && sane(w) && sane(h) => Some((x, y, w, h)),
        _ => None,
    }
}

/// The box around everything drawn, a little margin for text included.
fn content_box(steps: &[Step]) -> Option<(f32, f32, f32, f32)> {
    let mut b: Option<(f32, f32, f32, f32)> = None;
    let mut add = |x0: f32, y0: f32, x1: f32, y1: f32| {
        b = Some(match b {
            None => (x0, y0, x1, y1),
            Some((a, c, d, e)) => (a.min(x0), c.min(y0), d.max(x1), e.max(y1)),
        });
    };
    for shape in steps.iter().flat_map(|s| &s.shapes) {
        match shape {
            Shape::Path { subpaths, .. } => {
                for &(x, y) in subpaths.iter().flat_map(|(pts, _)| pts) {
                    add(x, y, x, y);
                }
            }
            Shape::Text { x, y, text, size, .. } => {
                let w = text.chars().count() as f32 * size * 0.55;
                add(x - w, y - size, x + w, y + size * 0.3);
            }
        }
    }
    let (x0, y0, x1, y1) = b?;
    let pad = 8.;
    Some((x0 - pad, y0 - pad, (x1 - x0) + 2. * pad, (y1 - y0) + 2. * pad))
}

pub fn parse(svg: &str) -> Option<Drawing> {
    let doc = roxmltree::Document::parse(svg).ok()?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return None;
    }
    let mut found = Collected { steps: vec![], loose: vec![] };
    collect(&root, Style::ROOT.of(&root), Affine::IDENTITY, None, &mut found);
    let Collected { mut steps, loose } = found;
    if !loose.is_empty() {
        if steps.is_empty() {
            steps.push(Step { caption: String::new(), shapes: loose });
        } else {
            steps[0].shapes.splice(0..0, loose);
        }
    }
    if steps.iter().all(|s| s.shapes.is_empty()) {
        return None;
    }
    // Without a viewBox, the size attributes give the coordinate space (an SVG of
    // width 100 draws 0..100); without either, the drawing's own extent does.
    let size_attr = |name: &str| root.attribute(name).and_then(number).filter(|v| *v > 0. && sane(*v));
    let (min_x, min_y, width, height) = view_box(&root)
        .or_else(|| Some((0., 0., size_attr("width")?, size_attr("height")?)))
        .or_else(|| content_box(&steps).filter(|&(_, _, w, h)| w > 0. && h > 0.))
        .unwrap_or((0., 0., DEFAULT_SIZE.0, DEFAULT_SIZE.1));
    Some(Drawing { min_x, min_y, width, height, steps })
}

/// Playback state for one drawing.
pub struct Playback {
    pub drawing: Rc<Drawing>,
    mode: PlayMode,
}

#[derive(Clone, Copy, Debug)]
enum PlayMode {
    /// Drawing itself, step after step, since this instant.
    Auto(Instant),
    /// On a step the person chose; it draws in from `since` (None: already drawn).
    Manual { step: usize, since: Option<Instant> },
}

const STEP_EVERY: f32 = 1.35;
const STEP_DRAW: f32 = 0.75;

impl Playback {
    pub fn new(drawing: Drawing, animate: bool) -> Self {
        let mode = if animate {
            PlayMode::Auto(Instant::now())
        } else {
            PlayMode::Manual { step: drawing.steps.len().max(1), since: None }
        };
        Self { drawing: Rc::new(drawing), mode }
    }

    pub fn steps(&self) -> usize {
        self.drawing.steps.len()
    }

    /// The last step number; at least 1, so the arithmetic below never meets an empty drawing.
    fn last(&self) -> usize {
        self.steps().max(1)
    }

    /// (current step 1-based, drawing progress of that step 0..1). Progress is
    /// below 1 exactly while the picture is still changing: during auto-play, once
    /// a step is drawn the next one becomes current at 0 and waits a moment before
    /// it draws, so a canvas can stop asking for frames as soon as progress is 1.
    pub fn state(&self) -> (usize, f32) {
        self.state_at(Instant::now())
    }

    fn state_at(&self, now: Instant) -> (usize, f32) {
        let n = self.last();
        let since = |t: Instant| now.saturating_duration_since(t).as_secs_f32();
        match self.mode {
            PlayMode::Auto(t0) => {
                let t = since(t0);
                let slot = ((t / STEP_EVERY) as usize + 1).min(n);
                let p = ((t - (slot - 1) as f32 * STEP_EVERY) / STEP_DRAW).clamp(0., 1.);
                if p >= 1. && slot < n { (slot + 1, 0.) } else { (slot, p) }
            }
            PlayMode::Manual { step, since: None } => (step.min(n), 1.),
            PlayMode::Manual { step, since: Some(at) } => (step.min(n), (since(at) / STEP_DRAW).clamp(0., 1.)),
        }
    }

    pub fn animating(&self) -> bool {
        self.state().1 < 1.
    }

    pub fn replay(&mut self) {
        self.mode = PlayMode::Auto(Instant::now());
    }

    pub fn go(&mut self, step: usize) {
        self.go_at(step, Instant::now());
    }

    fn go_at(&mut self, step: usize, now: Instant) {
        let target = step.clamp(1, self.last());
        // ‹ on the first step and › on the last change nothing, rather than wiping
        // the step and drawing it again.
        if target == self.state_at(now).0 {
            return;
        }
        self.mode = PlayMode::Manual { step: target, since: Some(now) };
    }

    pub fn prev(&mut self) {
        let (s, _) = self.state();
        self.go(s.saturating_sub(1));
    }

    pub fn next(&mut self) {
        let (s, _) = self.state();
        self.go(s + 1);
    }

    pub fn caption(&self) -> String {
        let (s, _) = self.state();
        self.drawing.steps.get(s - 1).map(|st| st.caption.clone()).unwrap_or_default()
    }
}

fn tone_color(t: Tone, theme: &Theme) -> Option<Hsla> {
    let p = theme.p;
    match t {
        Tone::Ink => Some(theme.voice_ink),
        Tone::Graphite => Some(p.graphite),
        Tone::Faint => Some(p.faint),
        Tone::Rule => Some(p.rule),
        Tone::Wash => Some(p.wash.opacity(0.75)),
        Tone::Paper => Some(p.paper),
        Tone::None => None,
    }
}

/// The closed polyline `pts` (viewBox units) as a filled path, placed by `map`.
fn fill_path(pts: &[(f32, f32)], map: impl Fn(f32, f32) -> Point<Pixels>) -> Option<Path<Pixels>> {
    if pts.len() < 3 {
        return None;
    }
    let mut b = PathBuilder::fill();
    b.move_to(map(pts[0].0, pts[0].1));
    for q in &pts[1..] {
        b.line_to(map(q.0, q.1));
    }
    b.close();
    b.build().ok()
}

/// The first `f` (0..1) of the polyline `pts`, by length, as a stroke `width` wide.
/// `stitched` draws it as a running stitch: thread on the surface, then under the cloth.
fn stroke_path(pts: &[(f32, f32)], f: f32, width: Pixels, stitched: bool, map: impl Fn(f32, f32) -> Point<Pixels>) -> Option<Path<Pixels>> {
    let seg_len = |a: (f32, f32), b: (f32, f32)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let total: f32 = pts.windows(2).map(|w| seg_len(w[0], w[1])).sum();
    let mut budget = total * f.clamp(0., 1.);
    // Nothing to draw yet (or a length that is not a number).
    if !(budget > 0.) {
        return None;
    }
    let mut b = if stitched {
        PathBuilder::stroke(width).dash_array(&[width * 3.4, width * 2.1])
    } else {
        PathBuilder::stroke(width)
    };
    b.move_to(map(pts[0].0, pts[0].1));
    for seg in pts.windows(2) {
        if budget <= 0. {
            break;
        }
        let len = seg_len(seg[0], seg[1]);
        if len <= budget {
            b.line_to(map(seg[1].0, seg[1].1));
            budget -= len;
        } else {
            let k = budget / len;
            b.line_to(map(seg[0].0 + (seg[1].0 - seg[0].0) * k, seg[0].1 + (seg[1].1 - seg[0].1) * k));
            budget = 0.;
        }
    }
    b.build().ok()
}

/// Tallest a drawing may be, as a multiple of its width. The prompt asks for at
/// most 320×520; a much taller viewBox is scaled down to fit rather than pushing the
/// rest of the margin off the page.
const MAX_ASPECT: f32 = 2.;

/// (pixels per viewBox unit, horizontal offset that centres a height-limited drawing).
fn fit(drawing: &Drawing, width: f32) -> (f32, f32) {
    let scale = (width / drawing.width).min(width * MAX_ASPECT / drawing.height);
    if !(scale.is_finite() && scale > 0.) {
        return (0., 0.);
    }
    (scale, (width - drawing.width * scale) / 2.)
}

/// A canvas that paints `drawing` up to `step` (with `progress` of the last step drawn).
/// It asks for another frame while `progress` is below 1 (see [`Playback::state`]).
pub fn canvas_for(drawing: Rc<Drawing>, step: usize, progress: f32, width: Pixels, theme: Theme) -> impl IntoElement {
    let (scale, offset) = fit(&drawing, width.as_f32());
    let height = px(drawing.height * scale);
    let animating = progress < 1.;
    // Clipped to its box, as an SVG viewport is: strokes beyond the viewBox stay off the page.
    div().w(width).h(height).flex_none().overflow_hidden().child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                let d = drawing.clone();
                let o = bounds.origin;
                let map = |x: f32, y: f32| point(o.x + px(offset + (x - d.min_x) * scale), o.y + px((y - d.min_y) * scale));
                for (i, st) in d.steps.iter().enumerate().take(step) {
                    let f = if i + 1 == step { progress } else { 1. };
                    for shape in &st.shapes {
                        match shape {
                            Shape::Path { subpaths, stroke, fill, width } => {
                                if let Some(fc) = tone_color(*fill, &theme) {
                                    for (pts, _) in subpaths {
                                        if let Some(path) = fill_path(pts, &map) {
                                            window.paint_path(path, fc.opacity(fc.a * f));
                                        }
                                    }
                                }
                                let Some(sc) = tone_color(*stroke, &theme) else { continue };
                                let w = px((width * scale).clamp(0.8, 2.4));
                                // the mind's own lines are stitched; thin guides stay drawn
                                let stitched = *stroke == Tone::Ink && w >= px(1.2);
                                for (pts, _) in subpaths {
                                    if let Some(path) = stroke_path(pts, f, w, stitched, &map) {
                                        window.paint_path(path, sc);
                                    }
                                }
                            }
                            Shape::Text { x, y, text, size, voice, anchor, fill, weight_bold } => {
                                let Some(color) = tone_color(*fill, &theme) else { continue };
                                let alpha = ((f - 0.25) / 0.6).clamp(0., 1.);
                                if alpha <= 0. {
                                    continue;
                                }
                                let fs = px((size * scale).max(7.));
                                let family: SharedString = if *voice { theme.voice_font.clone() } else { SANS.into() };
                                let run = TextRun {
                                    len: text.len(),
                                    font: Font {
                                        weight: if *weight_bold { FontWeight::SEMIBOLD } else { FontWeight::NORMAL },
                                        ..font(family)
                                    },
                                    color: color.opacity(alpha),
                                    background_color: None,
                                    underline: None,
                                    strikethrough: None,
                                };
                                let shaped = window.text_system().shape_line(text.clone().into(), fs, &[run], None);
                                let at = map(*x, *y);
                                let dx = match anchor {
                                    TextAnchor::Start => px(0.),
                                    TextAnchor::Middle => shaped.width / 2.,
                                    TextAnchor::End => shaped.width,
                                };
                                // SVG y is the baseline; paint() takes the line's top.
                                let lh = fs * 1.25;
                                let ascent_guess = fs * 0.78 + (lh - fs) / 2.;
                                let _ = shaped.paint(point(at.x - dx, at.y - ascent_guess), lh, TextAlign::Left, None, window, cx);
                            }
                        }
                    }
                }
                if animating {
                    window.request_animation_frame();
                }
            },
        )
        .size_full(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[std::prelude::v1::test]
    fn parses_steps_and_shapes() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 200">
<g data-step="1" data-caption="One"><rect x="10" y="10" width="40" height="20" rx="4" stroke="ink" fill="none"/><text x="30" y="24" class="voice" font-size="14" fill="ink" text-anchor="middle">A</text></g>
<g data-step="2" data-caption="Two"><path d="M10 50 Q60 40 110 50 M104 46 L110 50 L104 54" stroke="graphite"/><circle cx="160" cy="60" r="10" fill="wash" stroke="ink"/></g>
</svg>"#;
        let d = parse(svg).expect("parses");
        assert_eq!(d.steps.len(), 2);
        assert_eq!(d.steps[0].caption, "One");
        assert_eq!(d.steps[0].shapes.len(), 2);
        assert_eq!(d.steps[1].shapes.len(), 2);
        match &d.steps[1].shapes[0] {
            Shape::Path { subpaths, stroke, .. } => {
                assert_eq!(subpaths.len(), 2);
                assert_eq!(*stroke, Tone::Graphite);
            }
            _ => panic!("expected a path"),
        }
    }

    #[std::prelude::v1::test]
    fn welcome_sketch_parses() {
        let note = crate::welcome::welcome_note(crate::store::Mode::ThinksWithYou);
        let svg = note.threads[0].drawing.clone().unwrap();
        let d = parse(&svg).expect("welcome drawing parses");
        assert_eq!(d.steps.len(), 3);
        assert_eq!((d.min_x, d.min_y, d.width, d.height), (0., 0., 320., 236.));
        assert_eq!(d.steps.iter().map(|s| s.shapes.len()).collect::<Vec<_>>(), vec![3, 6, 6]);
        assert!(d.steps.iter().all(|s| !s.caption.is_empty()));
        assert_tessellates(&d);
    }

    fn points(shape: &Shape) -> Vec<(f32, f32)> {
        match shape {
            Shape::Path { subpaths, .. } => subpaths.iter().flat_map(|(p, _)| p.clone()).collect(),
            Shape::Text { x, y, .. } => vec![(*x, *y)],
        }
    }

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    /// Every shape becomes paths the way the canvas builds them, at several
    /// widths and stages of drawing, and every vertex is a real number.
    fn assert_tessellates(d: &Drawing) {
        for width in [230., 600.] {
            let (scale, offset) = fit(d, width);
            assert!(scale.is_finite() && scale > 0.);
            let map = |x: f32, y: f32| point(px(offset + (x - d.min_x) * scale), px((y - d.min_y) * scale));
            for shape in d.steps.iter().flat_map(|s| &s.shapes) {
                let Shape::Path { subpaths, width, .. } = shape else { continue };
                let w = px((width * scale).clamp(0.8, 2.4));
                for (pts, _) in subpaths {
                    let mut built = vec![fill_path(pts, &map)];
                    for f in [0., 0.01, 0.5, 1.] {
                        built.push(stroke_path(pts, f, w, true, &map));
                    }
                    for path in built.into_iter().flatten() {
                        assert!(path.vertices.iter().all(|v| v.xy_position.x.as_f32().is_finite() && v.xy_position.y.as_f32().is_finite()));
                    }
                }
            }
        }
    }

    #[std::prelude::v1::test]
    fn size_without_a_view_box() {
        let d = parse(r#"<svg width="100px" height="50"><line x1="0" y1="0" x2="100" y2="50"/></svg>"#).unwrap();
        assert_eq!((d.min_x, d.min_y, d.width, d.height), (0., 0., 100., 50.), "width and height are the coordinate space");
        // No size at all: the drawing's own extent, with a margin.
        let d = parse(r#"<svg><rect x="500" y="300" width="100" height="40"/></svg>"#).unwrap();
        assert!(d.min_x < 500. && d.min_x > 480. && d.width > 100. && d.width < 140., "{d:?}");
        // A broken viewBox falls back the same way.
        for vb in ["0 0 0 200", "0 0 -320 200", "0 0 NaN 200", "0 0 inf 200", "0 0 320", "a b c d", "0 0 1e39 200"] {
            let d = parse(&format!(r#"<svg viewBox="{vb}" width="320" height="200"><line x1="1" y1="1" x2="9" y2="9"/></svg>"#)).unwrap();
            assert_eq!((d.width, d.height), (320., 200.), "{vb}");
        }
        let d = parse(r#"<svg viewBox="-10,-10, 20 20"><circle r="5"/></svg>"#).unwrap();
        assert_eq!((d.min_x, d.min_y, d.width, d.height), (-10., -10., 20., 20.));
    }

    #[std::prelude::v1::test]
    fn transforms_move_shapes_and_text() {
        let svg = r#"<svg viewBox="0 0 320 200">
  <g transform="translate(100 50)">
    <g data-step="1" data-caption="Moved" transform="scale(2)">
      <line x1="0" y1="0" x2="10" y2="0"/>
      <text x="5" y="5" font-size="10">label</text>
    </g>
    <g data-step="2" data-caption="Turned">
      <line x1="0" y1="0" x2="10" y2="0" transform="rotate(90)"/>
      <rect x="0" y="0" width="10" height="10" transform="translate(-5,-5) rotate(45 5 5)"/>
    </g>
  </g>
  <g transform="matrix(1 0 0 1 nonsense)"><line x1="1" y1="1" x2="2" y2="2"/></g>
</svg>"#;
        let d = parse(svg).unwrap();
        assert_eq!(d.steps.iter().map(|s| s.caption.as_str()).collect::<Vec<_>>(), vec!["Moved", "Turned"]);
        let first = points(&d.steps[0].shapes[1]);
        assert!(close(first[0], (100., 50.)) && close(first[1], (120., 50.)), "{first:?}");
        match &d.steps[0].shapes[2] {
            Shape::Text { x, y, size, .. } => {
                assert!(close((*x, *y), (110., 60.)));
                assert!((size - 20.).abs() < 1e-3, "font size follows the scale");
            }
            other => panic!("{other:?}"),
        }
        match &d.steps[0].shapes[1] {
            Shape::Path { width, .. } => assert!((width - 2.4).abs() < 1e-3, "stroke width follows the scale"),
            other => panic!("{other:?}"),
        }
        let turned = points(&d.steps[1].shapes[0]);
        assert!(close(turned[0], (100., 50.)) && close(turned[1], (100., 60.)), "{turned:?}");
        // A transform that does not parse is ignored, not fatal: that loose line
        // went to the front of step 1.
        assert!(close(points(&d.steps[0].shapes[0])[0], (1., 1.)));
        assert_tessellates(&d);
    }

    #[std::prelude::v1::test]
    fn steps_inside_wrappers_and_loose_shapes() {
        let svg = r#"<svg viewBox="0 0 320 200">
  <circle cx="10" cy="10" r="5"/>
  <g><g data-step="2" data-caption="  Second,
     wrapped  "><line x1="0" y1="0" x2="5" y2="5"/>
       <g data-step="9"><line x1="1" y1="1" x2="6" y2="6"/></g></g></g>
  <g data-step="3" data-caption="Third"/>
  <a href="https://example.com"><g data-step="4"><rect width="3" height="3"/></g></a>
</svg>"#;
        let d = parse(svg).unwrap();
        let shape_counts: Vec<usize> = d.steps.iter().map(|s| s.shapes.len()).collect();
        assert_eq!(shape_counts, vec![3, 0, 1], "loose circle joins the first step; a nested step is part of its step");
        assert_eq!(d.steps[0].caption, "Second, wrapped");
        let none = parse(r#"<svg viewBox="0 0 10 10"><line x1="0" y1="0" x2="5" y2="5"/><text x="1" y="1">hi</text></svg>"#).unwrap();
        assert_eq!(none.steps.len(), 1, "no data-step at all: one step");
        assert_eq!(none.steps[0].caption, "");
    }

    #[std::prelude::v1::test]
    fn path_commands_flatten() {
        let d = parse(r#"<svg viewBox="0 0 100 100"><path d="M10 10 a 5 5 0 0 1 10 0 s 5 5 10 0 t 10 0 h 5 v 5 z m 20 20 l 5 5 Z L 90 90"/></svg>"#).unwrap();
        let Shape::Path { subpaths, .. } = &d.steps[0].shapes[0] else { panic!() };
        assert_eq!(subpaths.iter().map(|(_, closed)| *closed).collect::<Vec<_>>(), vec![true, true, false]);
        // The relative `m` after `z` starts from the first subpath's start (10,10).
        assert!(close(subpaths[1].0[0], (30., 30.)), "{:?}", subpaths[1].0[0]);
        // A line after `Z` starts where that subpath began.
        assert!(close(subpaths[2].0[0], (30., 30.)) && close(*subpaths[2].0.last().unwrap(), (90., 90.)));
        // A path that goes wrong midway keeps what came before.
        let d = parse(r#"<svg viewBox="0 0 100 100"><path d="M0 0 L 10 10 L 20 oops 30"/></svg>"#).unwrap();
        assert_eq!(points(&d.steps[0].shapes[0]).len(), 2);
        assert!(parse(r#"<svg viewBox="0 0 100 100"><path d="garbage"/><path/></svg>"#).is_none());
    }

    #[std::prelude::v1::test]
    fn styles_on_shapes_and_groups() {
        let svg = r##"<svg viewBox="0 0 100 100">
  <g data-step="1" stroke="faint" stroke-width="1.5" fill="wash" font-size="12" class="sans" text-anchor="middle" font-weight="bold">
    <rect width="10" height="10"/>
    <rect width="10" height="10" fill="none" style="fill: var(--ink); stroke:graphite"/>
    <text x="1" y="1">inherits</text>
    <text x="1" y="1" class="voice" font-size="40px" text-anchor="end" font-weight="400" style="fill:graphite">own</text>
    <line x1="0" y1="0" x2="1" y2="1" stroke-width="2px" stroke="#333"/>
    <rect width="10" height="10" fill="#f6f6f0" stroke="#999999"/>
  </g>
</svg>"##;
        let d = parse(svg).unwrap();
        let s = &d.steps[0].shapes;
        assert!(matches!(s[0], Shape::Path { stroke: Tone::Faint, fill: Tone::Wash, width, .. } if (width - 1.5).abs() < 1e-6));
        assert!(matches!(s[1], Shape::Path { stroke: Tone::Graphite, fill: Tone::Ink, .. }), "style wins over attributes");
        assert!(matches!(&s[2], Shape::Text { size, voice: false, anchor: TextAnchor::Middle, fill: Tone::Ink, weight_bold: true, .. } if *size == 12.), "a group's wash is not for its labels");
        assert!(matches!(&s[3], Shape::Text { size, voice: true, anchor: TextAnchor::End, fill: Tone::Graphite, weight_bold: false, .. } if *size == 28.));
        assert!(matches!(s[4], Shape::Path { stroke: Tone::Ink, width, .. } if width == 2.));
        assert!(matches!(s[5], Shape::Path { stroke: Tone::Faint, fill: Tone::Wash, .. }), "a pale hex fill is a wash, not a block of ink");
        assert_eq!(tone(Some("#000"), Tone::None, false), Tone::Ink);
        assert_eq!(tone(Some("#fff"), Tone::None, true), Tone::Wash);
        assert_eq!(tone(Some("#12345"), Tone::Faint, false), Tone::Faint, "not a colour: keep the default");
        assert_eq!(tone(Some("rgb(1,2,3)"), Tone::Graphite, false), Tone::Graphite);
    }

    #[std::prelude::v1::test]
    fn text_edge_cases() {
        let svg = r#"<svg viewBox="0 0 100 100"><g data-step="1">
  <text x="1" y="1"></text>
  <text x="1" y="1">   </text>
  <text x="1" y="1">two <tspan dy="12">lines</tspan>
     joined</text>
  <text x="NaN" y="inf">at the origin</text>
  <text fill="none">still visible</text>
</g></svg>"#;
        let d = parse(svg).unwrap();
        let texts: Vec<(String, (f32, f32), Tone)> = d.steps[0]
            .shapes
            .iter()
            .map(|s| match s {
                Shape::Text { text, x, y, fill, .. } => (text.clone(), (*x, *y), *fill),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(texts, vec![
            ("two lines joined".into(), (1., 1.), Tone::Ink),
            ("at the origin".into(), (0., 0.), Tone::Ink),
            ("still visible".into(), (0., 0.), Tone::Ink),
        ]);
    }

    #[std::prelude::v1::test]
    fn hostile_numbers_never_reach_the_tessellator() {
        let svg = r#"<svg viewBox="0 0 320 200"><g data-step="1">
  <line x1="NaN" y1="0" x2="inf" y2="-inf"/>
  <line x1="0" y1="0" x2="1e39" y2="5"/>
  <path d="M 0 0 L 1e30 1e30"/>
  <path d="M 0 0 C 1e38 1e38 -1e38 -1e38 10 10"/>
  <polyline points="0,0 NaN,1 2,inf 1e40,4"/>
  <circle cx="10" cy="10" r="1e39"/>
  <circle cx="10" cy="10" r="-5"/>
  <rect x="0" y="0" width="-10" height="10"/>
  <ellipse cx="5" cy="5" rx="0" ry="5"/>
  <rect x="0" y="0" width="50" height="20" rx="NaN" stroke-width="NaN"/>
  <g transform="scale(1e30)"><line x1="0" y1="0" x2="1" y2="1"/></g>
  <g transform="scale(0)"><text x="1" y="1" font-size="NaN">squashed</text></g>
  <line x1="10" y1="10" x2="20" y2="20" stroke-width="1e9"/>
  <text x="5" y="5" font-size="-3">tiny</text>
</g></svg>"#;
        let d = parse(svg).expect("the sane shapes survive");
        for shape in d.steps.iter().flat_map(|s| &s.shapes) {
            for (x, y) in points(shape) {
                assert!(sane(x) && sane(y), "{shape:?}");
            }
            if let Shape::Path { width, .. } = shape {
                assert!(width.is_finite() && *width <= 20.);
            }
            if let Shape::Text { size, .. } = shape {
                assert!((8.0..=28.).contains(size));
            }
        }
        assert_tessellates(&d);
        // A path whose length overflows f32 when squared is dropped, not drawn as NaN.
        assert!(parse(r#"<svg viewBox="0 0 1 1"><path d="M -3e19 0 L 3e19 0"/></svg>"#).is_none());
    }

    #[std::prelude::v1::test]
    fn not_svg_or_not_drawable() {
        for bad in [
            "",
            "not xml at all",
            "<svg><g data-step=\"1\"><line x1='0'",
            "<html><body/></html>",
            "<svg viewBox=\"0 0 10 10\"></svg>",
            "<svg viewBox=\"0 0 10 10\"><g data-step=\"1\" data-caption=\"empty\"/></svg>",
            "<!DOCTYPE svg [<!ENTITY a \"aaaa\">]><svg viewBox=\"0 0 10 10\"><text>&a;</text></svg>",
            r#"<svg viewBox="0 0 10 10"><script>alert(1)</script><foreignObject><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject><style>line{stroke:red}</style><defs><line x1="0" y1="0" x2="1" y2="1"/></defs><image href="x.png"/></svg>"#,
            r#"<svg viewBox="0 0 10 10"><line x1="0" y1="0" x2="1" y2="1" display="none"/><g style="display:none"><line x1="0" y1="0" x2="1" y2="1"/></g></svg>"#,
        ] {
            assert!(parse(bad).is_none(), "{bad}");
        }
        // Scripts and foreign content are skipped, and what is drawable stays.
        let d = parse(r#"<svg viewBox="0 0 10 10"><script>alert(1)</script><foreignObject><p>x</p></foreignObject><line x1="0" y1="0" x2="1" y2="1"/></svg>"#).unwrap();
        assert_eq!(d.steps[0].shapes.len(), 1);
    }

    #[std::prelude::v1::test]
    fn many_shapes_parse_and_tall_drawings_fit() {
        let lines: String = (0..2000).map(|i| format!("<line x1=\"{i}\" y1=\"0\" x2=\"{i}\" y2=\"10\"/>")).collect();
        let d = parse(&format!("<svg viewBox=\"0 0 2000 10\"><g data-step=\"1\">{lines}</g></svg>")).unwrap();
        assert_eq!(d.steps[0].shapes.len(), 2000);
        let tall = parse(r#"<svg viewBox="0 0 320 20000"><line x1="0" y1="0" x2="320" y2="20000"/></svg>"#).unwrap();
        let (scale, offset) = fit(&tall, 230.);
        assert!((tall.height * scale - 460.).abs() < 1e-3, "no taller than twice the width");
        assert!(offset > 0., "and centred");
        let (scale, offset) = fit(&parse(r#"<svg viewBox="0 0 320 200"><circle r="1"/></svg>"#).unwrap(), 230.);
        assert!((scale - 230. / 320.).abs() < 1e-6 && offset == 0.);
    }

    fn drawing(steps: usize) -> Drawing {
        Drawing {
            min_x: 0.,
            min_y: 0.,
            width: 10.,
            height: 10.,
            steps: (1..=steps).map(|i| Step { caption: format!("step {i}"), shapes: vec![] }).collect(),
        }
    }

    fn at(t0: Instant, secs: f32) -> Instant {
        t0 + Duration::from_secs_f32(secs)
    }

    #[std::prelude::v1::test]
    fn auto_play_draws_each_step_then_stops() {
        let mut pb = Playback::new(drawing(3), true);
        let t0 = Instant::now();
        pb.mode = PlayMode::Auto(t0);
        assert_eq!(pb.state_at(t0), (1, 0.));
        let (s, p) = pb.state_at(at(t0, STEP_DRAW / 2.));
        assert_eq!(s, 1);
        assert!((p - 0.5).abs() < 1e-3);
        // Between steps the next one is current and waiting, so frames keep coming.
        assert_eq!(pb.state_at(at(t0, STEP_DRAW + 0.1)), (2, 0.));
        assert_eq!(pb.state_at(at(t0, STEP_EVERY + STEP_DRAW + 0.1)), (3, 0.));
        let (s, p) = pb.state_at(at(t0, 2. * STEP_EVERY + STEP_DRAW / 2.));
        assert_eq!(s, 3);
        assert!(p > 0.4 && p < 0.6);
        assert_eq!(pb.state_at(at(t0, 2. * STEP_EVERY + STEP_DRAW)), (3, 1.));
        assert_eq!(pb.state_at(at(t0, 60.)), (3, 1.), "and stays on the last step, done");
    }

    #[std::prelude::v1::test]
    fn stepping_by_hand_stops_at_the_ends() {
        let mut pb = Playback::new(drawing(3), false);
        let t0 = Instant::now();
        assert_eq!(pb.state_at(t0), (3, 1.), "a stored drawing opens finished");
        pb.go_at(4, t0);
        assert_eq!(pb.state_at(t0), (3, 1.), "› on the last step does not redraw it");
        pb.go_at(2, t0);
        assert_eq!(pb.state_at(t0), (2, 0.));
        assert_eq!(pb.state_at(at(t0, STEP_DRAW)), (2, 1.), "a step chosen by hand draws in, then holds still");
        pb.go_at(1, at(t0, 1.));
        pb.go_at(0, at(t0, 5.));
        assert_eq!(pb.state_at(at(t0, 5.)), (1, 1.), "‹ on the first step leaves it drawn");
        pb.go_at(2, at(t0, 6.));
        pb.go_at(3, at(t0, 6.2));
        assert_eq!(pb.state_at(at(t0, 6.2)), (3, 0.));
        // › while auto-play is still drawing the last step leaves auto-play to finish.
        pb.mode = PlayMode::Auto(t0);
        pb.go_at(3, at(t0, 2. * STEP_EVERY + 0.1));
        assert!(matches!(pb.mode, PlayMode::Auto(_)));
        // ‹ during auto-play takes over by hand.
        pb.go_at(1, at(t0, 2. * STEP_EVERY + 0.2));
        assert!(matches!(pb.mode, PlayMode::Manual { step: 1, .. }));
        pb.replay();
        assert!(matches!(pb.mode, PlayMode::Auto(_)));
        assert_eq!(pb.state().0, 1);
        assert_eq!(pb.caption(), "step 1");
    }

    #[std::prelude::v1::test]
    fn one_step_and_empty_drawings() {
        let mut one = Playback::new(drawing(1), true);
        let t0 = Instant::now();
        one.mode = PlayMode::Auto(t0);
        assert_eq!(one.state_at(at(t0, 10.)), (1, 1.));
        one.next();
        one.prev();
        assert!(matches!(one.mode, PlayMode::Auto(_)), "nowhere to go");
        assert_eq!(one.caption(), "step 1");

        // parse never returns one, but the struct allows it: nothing panics.
        let mut empty = Playback::new(drawing(0), true);
        assert_eq!(empty.steps(), 0);
        empty.next();
        empty.prev();
        empty.go(7);
        empty.replay();
        assert_eq!(empty.state().0, 1);
        assert_eq!(empty.caption(), "");
        let still = Playback::new(drawing(0), false);
        assert!(!still.animating());
    }

    #[std::prelude::v1::test]
    fn transforms_compose_in_order() {
        let t = Affine { a: 1., b: 0., c: 0., d: 1., e: 10., f: 0. };
        let s = Affine { a: 2., b: 0., c: 0., d: 2., e: 0., f: 0. };
        assert!(close(t.then(s).apply((1., 1.)), (12., 2.)), "scale first, then move");
        assert!(close(s.then(t).apply((1., 1.)), (22., 2.)));
        assert_eq!(Affine::IDENTITY.then(t), t);
        assert!((s.scale() - 2.).abs() < 1e-6);
    }
}

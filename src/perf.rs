//! Frame and layout counters for performance work, on with `INKLING_FRAME_LOG=1`: once a
//! second, how many frames the window drew and what the editor's layouts cost, plus any
//! single layout slower than a few milliseconds. Off (the default), nothing here runs.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("INKLING_FRAME_LOG").is_ok_and(|v| !v.is_empty() && v != "0"))
}

/// A layout slower than this is logged on its own, so the first paint of a long note shows.
const SLOW_LAYOUT: Duration = Duration::from_millis(4);

#[derive(Default)]
struct Second {
    since: Option<Instant>,
    frames: u32,
    layouts: u32,
    layout_lines: usize,
    /// Lines shaped afresh (the rest were carried over from the layout before).
    layout_shaped: usize,
    layout_total: Duration,
    layout_max: Duration,
}

static SECOND: Mutex<Second> = Mutex::new(Second {
    since: None,
    frames: 0,
    layouts: 0,
    layout_lines: 0,
    layout_shaped: 0,
    layout_total: Duration::ZERO,
    layout_max: Duration::ZERO,
});

fn stats() -> std::sync::MutexGuard<'static, Second> {
    SECOND.lock().unwrap_or_else(|e| e.into_inner())
}

/// The window drew a frame.
pub fn frame() {
    if !on() {
        return;
    }
    let mut s = stats();
    s.frames += 1;
    report(&mut s);
}

/// The editor laid out `lines` lines, shaping `shaped` of them afresh, which took `took`.
pub fn layout(lines: usize, shaped: usize, took: Duration) {
    if !on() {
        return;
    }
    if took >= SLOW_LAYOUT {
        log::info!("perf: layout of {lines} lines ({shaped} shaped) took {:.2} ms", took.as_secs_f64() * 1000.);
    }
    let mut s = stats();
    s.layouts += 1;
    s.layout_lines = lines;
    s.layout_shaped += shaped;
    s.layout_total += took;
    s.layout_max = s.layout_max.max(took);
}

/// Once a second has passed since the last report, write one and start the next second.
fn report(s: &mut Second) {
    let now = Instant::now();
    let Some(since) = s.since else {
        s.since = Some(now);
        return;
    };
    let span = now - since;
    if span < Duration::from_secs(1) {
        return;
    }
    let ms = |d: Duration| d.as_secs_f64() * 1000.;
    log::info!(
        "perf: {} frames in {:.2} s; {} layouts of {} lines ({} shaped), {:.2} ms in all, slowest {:.2} ms",
        s.frames,
        span.as_secs_f64(),
        s.layouts,
        s.layout_lines,
        s.layout_shaped,
        ms(s.layout_total),
        ms(s.layout_max)
    );
    *s = Second { since: Some(now), ..Second::default() };
}

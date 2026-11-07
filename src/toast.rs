//! Toasts: a short message at the bottom of the window, on a small fabric label with a
//! running-stitch edge. Everything the window says in passing goes in one stack, centred
//! under the page, so two messages never cover each other: a note deleted, a thread
//! dismissed, a problem from the brain, an import, a copy.
//!
//! One label per kind. A second message of a kind takes the first one's place: its words
//! crossfade in 120 ms and it does not rise again. A new kind rises into the stack, above
//! the ones already there, so nothing that is being read moves.
//!
//! Motion: in over 180 ms rising 8 px, out over 160 ms
//! dropping 4 px; with reduced motion, fades only. A toast stays 3 s, 8 s when it offers
//! Undo, and 12 s for a problem, which can also be closed (its ×, or a click anywhere on
//! it). The pointer on a toast holds it.

use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

use crate::store::InkColor;
use crate::theme::{SANS, Theme, ink_color, ts};
use crate::ui::{self, icon};

/// Arriving: fades in and rises 8 px.
pub const IN: Duration = Duration::from_millis(180);
/// Leaving: fades out and drops 4 px.
pub const OUT: Duration = Duration::from_millis(160);
/// Taking the place of one of its kind: the words crossfade, nothing moves.
pub const SWAP: Duration = Duration::from_millis(120);
/// With reduced motion every change is a fade this long.
const FADE: Duration = Duration::from_millis(120);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Something done: "Copied as Markdown", "Saved Plan.md", an import.
    Done,
    /// "Note deleted", with Undo.
    NoteDeleted,
    /// "Thread dismissed", with Undo and its keys (⌥⌘Z).
    ThreadDismissed,
    /// Something went wrong: an ochre thread, and a close control.
    Problem,
    /// "Your reply is in the note, ready to rework.", with Undo (Write it in).
    WrittenIn,
}

impl Kind {
    /// How long a toast of this kind stays; the pointer on it holds it longer.
    pub fn life(self) -> Duration {
        match self {
            Kind::Done => Duration::from_secs(3),
            Kind::NoteDeleted | Kind::ThreadDismissed => Duration::from_secs(8),
            Kind::Problem => Duration::from_secs(12),
            Kind::WrittenIn => Duration::from_secs(8),
        }
    }

    fn undo(self) -> bool {
        matches!(self, Kind::NoteDeleted | Kind::ThreadDismissed | Kind::WrittenIn)
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Done => "done",
            Kind::NoteDeleted => "note_deleted",
            Kind::ThreadDismissed => "thread_dismissed",
            Kind::Problem => "problem",
            Kind::WrittenIn => "written_in",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub kind: Kind,
    pub text: String,
    /// When it arrived, or took the place of one of its kind.
    shown: Instant,
    /// The words it replaced, fading out while its own fade in.
    replaced: Option<String>,
    /// Time spent under the pointer, which does not count against its life.
    held: Duration,
    hovered: Option<Instant>,
    /// When it began to leave.
    leaving: Option<Instant>,
    /// Where it was drawn last, for the page's dump.
    bounds: Option<Bounds<Pixels>>,
}

impl Toast {
    /// How long it has been showing, not counting the time the pointer held it.
    fn age(&self, now: Instant) -> Duration {
        let held = self.held + self.hovered.map_or(Duration::ZERO, |h| now.saturating_duration_since(h));
        now.saturating_duration_since(self.shown).saturating_sub(held)
    }
}

/// The window's toasts, oldest first.
#[derive(Default)]
pub struct Toasts {
    list: Vec<Toast>,
    next: u64,
    /// A wake-up for when the next toast is due to leave.
    wake: Option<(Instant, Task<()>)>,
}

impl Global for Toasts {}

/// What the stack is doing after [`Toasts::step`].
#[derive(Debug, PartialEq)]
pub struct Step {
    /// Something is arriving, leaving or crossfading: draw the next frame.
    pub moving: bool,
    /// Else, when the next toast is due to leave.
    pub due: Option<Instant>,
}

impl Toasts {
    /// Show `text` as a toast of `kind`: in place of the one of its kind if there is one,
    /// else in a label of its own at the top of the stack.
    pub fn push(&mut self, kind: Kind, text: impl Into<String>, now: Instant) {
        let text = text.into();
        if let Some(t) = self.list.iter_mut().find(|t| t.kind == kind && t.leaving.is_none()) {
            t.replaced = Some(std::mem::replace(&mut t.text, text));
            t.shown = now;
            t.held = Duration::ZERO;
            t.hovered = t.hovered.map(|_| now);
            return;
        }
        self.next += 1;
        self.list.push(Toast { id: self.next, kind, text, shown: now, replaced: None, held: Duration::ZERO, hovered: None, leaving: None, bounds: None });
    }

    /// Send the toast of `kind` on its way (its Undo was used, or it was closed).
    pub fn close(&mut self, kind: Kind, now: Instant) {
        for t in self.list.iter_mut().filter(|t| t.kind == kind && t.leaving.is_none()) {
            t.leaving = Some(now);
        }
    }

    fn close_id(&mut self, id: u64, now: Instant) {
        for t in self.list.iter_mut().filter(|t| t.id == id && t.leaving.is_none()) {
            t.leaving = Some(now);
        }
    }

    /// The pointer came onto a toast (it stops counting) or left it (it goes on).
    pub fn hover(&mut self, id: u64, on: bool, now: Instant) {
        let Some(t) = self.list.iter_mut().find(|t| t.id == id) else { return };
        match (on, t.hovered) {
            (true, None) => t.hovered = Some(now),
            (false, Some(since)) => {
                t.held += now.saturating_duration_since(since);
                t.hovered = None;
            }
            _ => {}
        }
    }

    /// The toast of `kind` that is showing and not on its way out.
    pub fn live(&self, kind: Kind) -> Option<&Toast> {
        self.list.iter().find(|t| t.kind == kind && t.leaving.is_none())
    }

    /// Time moves on: a toast past its life starts to leave, one that has left is dropped.
    pub fn step(&mut self, now: Instant) -> Step {
        for t in &mut self.list {
            if t.leaving.is_none() && t.hovered.is_none() && t.age(now) >= t.kind.life() {
                t.leaving = Some(now);
            }
            if t.replaced.is_some() && now.saturating_duration_since(t.shown) >= SWAP {
                t.replaced = None;
            }
        }
        self.list.retain(|t| t.leaving.is_none_or(|l| now.saturating_duration_since(l) < OUT));
        let moving = self.list.iter().any(|t| t.leaving.is_some() || t.replaced.is_some() || now.saturating_duration_since(t.shown) < IN);
        let due = self
            .list
            .iter()
            .filter(|t| t.hovered.is_none())
            .map(|t| now + t.kind.life().saturating_sub(t.age(now)))
            .min();
        Step { moving, due: if moving { None } else { due } }
    }
}

// ----- the app's door ---------------------------------------------------------------

/// Show a toast in the window.
pub fn show(cx: &mut App, kind: Kind, text: impl Into<String>) {
    cx.default_global::<Toasts>().push(kind, text, Instant::now());
    cx.refresh_windows();
}

/// Take the toast of `kind` away.
pub fn close(cx: &mut App, kind: Kind) {
    cx.default_global::<Toasts>().close(kind, Instant::now());
    cx.refresh_windows();
}

/// Whether a toast of `kind` is showing (an Undo is only good while its toast is).
pub fn live(cx: &App, kind: Kind) -> bool {
    cx.try_global::<Toasts>().is_some_and(|t| t.live(kind).is_some())
}

/// Whether any toast is on screen, leaving ones included (the corner hint waits for them).
pub fn any(cx: &App) -> bool {
    cx.try_global::<Toasts>().is_some_and(|t| !t.list.is_empty())
}

/// The words of the toast of `kind` that is showing.
pub fn text_of(cx: &App, kind: Kind) -> Option<String> {
    cx.try_global::<Toasts>().and_then(|t| t.live(kind)).map(|t| t.text.clone())
}

/// The stack as the page's dump reports it: every label, oldest first, where it was drawn.
pub fn describe(cx: &App) -> Value {
    let Some(toasts) = cx.try_global::<Toasts>() else { return json!([]) };
    toasts
        .list
        .iter()
        .map(|t| {
            let b = t.bounds;
            json!({
                "kind": t.kind.name(),
                "text": t.text,
                "live": t.leaving.is_none(),
                "held": t.hovered.is_some(),
                "top": b.map(|b| b.top().as_f32()),
                "bottom": b.map(|b| b.bottom().as_f32()),
                "left": b.map(|b| b.left().as_f32()),
                "right": b.map(|b| b.right().as_f32()),
            })
        })
        .collect()
}

// ----- drawing ----------------------------------------------------------------------

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    1. - (1. - t).powi(3)
}

fn ease_in(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t
}

fn frac(since: Duration, of: Duration) -> f32 {
    (since.as_secs_f32() / of.as_secs_f32()).clamp(0., 1.)
}

/// The stack, for the window's view to draw over everything else. `right` is the width
/// the chat takes on the right, so the stack centres under the page; `undo` runs a toast's
/// Undo.
pub fn stack<V: 'static>(
    right: Pixels,
    window: &mut Window,
    cx: &mut Context<V>,
    undo: impl Fn(&mut V, Kind, &mut Window, &mut Context<V>) + Clone + 'static,
) -> Option<AnyElement> {
    let now = Instant::now();
    let step = cx.default_global::<Toasts>().step(now);
    if step.moving {
        window.request_animation_frame();
    } else if let Some(due) = step.due {
        let toasts = cx.global::<Toasts>();
        if toasts.wake.as_ref().is_none_or(|(at, _)| *at != due) {
            let task = cx.spawn_in(window, async move |_, cx| {
                cx.background_executor().timer(due.saturating_duration_since(Instant::now())).await;
                let _ = cx.update(|window, _| window.refresh());
            });
            cx.global_mut::<Toasts>().wake = Some((due, task));
        }
    }
    let list = cx.global::<Toasts>().list.clone();
    if list.is_empty() {
        return None;
    }
    let t = Theme::get(cx).clone();
    let p = t.p;
    let reduced = cx.reduce_motion();
    let ochre = ink_color(InkColor::Orange, &p);
    let undo_keys = crate::shortcuts::keys_for("app::UndoDismiss", cx);
    let mut column = div().flex().flex_col().items_center().gap(px(8.));
    // newest on top, so the ones already there do not move
    for toast in list.iter().rev() {
        let id = toast.id;
        let kind = toast.kind;
        let problem = kind == Kind::Problem;
        // arriving (a replacement does not rise again), then leaving
        let (mut opacity, mut dy) = (1., 0.);
        if toast.replaced.is_none() {
            let since = now.saturating_duration_since(toast.shown);
            if reduced {
                opacity = frac(since, FADE);
            } else {
                let k = ease_out(frac(since, IN));
                opacity = k;
                dy = 8. * (1. - k);
            }
        }
        if let Some(l) = toast.leaving {
            let since = now.saturating_duration_since(l);
            if reduced {
                opacity *= 1. - frac(since, FADE);
            } else {
                let k = ease_in(frac(since, OUT));
                opacity *= 1. - k;
                dy += 4. * k;
            }
        }
        let swap = toast.replaced.as_ref().map(|_| frac(now.saturating_duration_since(toast.shown), SWAP)).unwrap_or(1.);
        let words = div()
            .relative()
            .max_w(px(460.))
            .child(div().opacity(swap).child(toast.text.clone()))
            .children(toast.replaced.clone().map(|old| div().absolute().top_0().left_0().w_full().opacity(1. - swap).child(old)));
        let undo = undo.clone();
        let label = div()
            .id(("toast", id as usize))
            .relative()
            .top(px(dy))
            .opacity(opacity)
            .flex()
            .items_center()
            .gap(px(10.))
            .pl(px(14.))
            .pr(px(if problem || kind.undo() { 8. } else { 14. }))
            .py(px(8.))
            .rounded(px(8.))
            .bg(p.card)
            .shadow({
                let mut s = ui::popover_shadow(&p);
                s.push(BoxShadow { color: p.shadow, offset: point(px(0.), px(1.)), blur_radius: px(3.), spread_radius: px(0.), inset: false });
                s
            })
            .font_family(SANS)
            .text_size(px(ts::UI))
            .line_height(px(18.))
            .text_color(p.ink)
            .occlude()
            .on_hover(cx.listener(move |_, on: &bool, _, cx| {
                cx.global_mut::<Toasts>().hover(id, *on, Instant::now());
                cx.notify();
            }))
            // the label's edge: a running stitch, ochre for a problem
            .child(
                div()
                    .absolute()
                    .top(px(3.))
                    .left(px(3.))
                    .right(px(3.))
                    .bottom(px(3.))
                    .rounded(px(6.))
                    .border_1()
                    .border_dashed()
                    .border_color(if problem { ochre.opacity(0.8) } else { p.faint.opacity(0.45) }),
            )
            // a problem says so in shape as well as colour
            .when(problem, |d| d.child(icon("circle-alert", 14., ochre)))
            .child(words)
            .when(kind.undo(), |d| {
                d.child(
                    div()
                        .id(("toast-undo", id as usize))
                        .flex_none()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(5.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .active(|s| s.opacity(0.7))
                        .child("Undo")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            undo(this, kind, window, cx);
                        })),
                )
                // the keys close the label as Undo's own padding would: as far in as the words start
                .when(kind == Kind::ThreadDismissed, |d| d.children(undo_keys.clone().map(|k| ui::key_hint(k, &t).pr(px(6.)))))
            })
            .when(problem, |d| {
                d.cursor_pointer()
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.global_mut::<Toasts>().close_id(id, Instant::now());
                        cx.notify();
                    }))
                    .child(ui::icon_button(("toast-close", id as usize), "x", 12., &t).tooltip(crate::app::tip("Close", None)).on_click(cx.listener(move |_, _, _, cx| {
                        cx.stop_propagation();
                        cx.global_mut::<Toasts>().close_id(id, Instant::now());
                        cx.notify();
                    })))
            })
            // where it was drawn, for the dump
            .child(
                canvas(
                    move |b, _, cx| {
                        if let Some(t) = cx.global_mut::<Toasts>().list.iter_mut().find(|t| t.id == id) {
                            t.bounds = Some(b);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        column = column.child(label);
    }
    // 12 px above the sheet's bottom edge, as the corner hint sits
    Some(div().absolute().bottom(px(24.)).left_0().right(right).flex().justify_center().child(column).into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[std::prelude::v1::test]
    fn one_label_per_kind_and_a_second_takes_its_place() {
        let t0 = Instant::now();
        let mut s = Toasts::default();
        s.push(Kind::ThreadDismissed, "Thread dismissed", t0);
        s.push(Kind::Problem, "Claude is overloaded", at(t0, 100));
        s.push(Kind::ThreadDismissed, "Thread dismissed", at(t0, 2000));
        assert_eq!(s.list.len(), 2, "two kinds, two labels");
        let first = &s.list[0];
        assert_eq!((first.kind, first.id), (Kind::ThreadDismissed, 1), "the replacement keeps its place in the stack");
        assert!(first.replaced.is_some(), "and crossfades its words");
        // its time starts again: still there 8 s after the second dismiss, not the first
        s.step(at(t0, 9000));
        assert!(s.live(Kind::ThreadDismissed).is_some());
        s.step(at(t0, 10_100));
        assert!(s.live(Kind::ThreadDismissed).is_none());
    }

    #[std::prelude::v1::test]
    fn a_reply_written_in_offers_undo_for_8_s() {
        let t0 = Instant::now();
        let mut s = Toasts::default();
        s.push(Kind::WrittenIn, "Your reply is in the note, ready to rework.", t0);
        assert!(Kind::WrittenIn.undo());
        s.step(at(t0, 7900));
        assert!(s.live(Kind::WrittenIn).is_some());
        s.step(at(t0, 8050));
        assert!(s.live(Kind::WrittenIn).is_none());
    }

    #[std::prelude::v1::test]
    fn each_kind_stays_its_own_time_then_leaves_in_160_ms() {
        let t0 = Instant::now();
        let mut s = Toasts::default();
        s.push(Kind::Done, "Copied as Markdown", t0);
        s.push(Kind::NoteDeleted, "Note deleted", t0);
        s.push(Kind::Problem, "Couldn't reach the model", t0);
        s.step(at(t0, 3050));
        assert!(s.live(Kind::Done).is_none() && s.list.len() == 3, "a plain toast leaves after 3 s, and is still drawn while it goes");
        s.step(at(t0, 3300));
        assert_eq!(s.list.len(), 2, "gone 160 ms later");
        s.step(at(t0, 8050));
        assert!(s.live(Kind::NoteDeleted).is_none() && s.live(Kind::Problem).is_some(), "Undo stays 8 s, a problem 12 s");
        s.step(at(t0, 12_050));
        assert!(s.live(Kind::Problem).is_none());
    }

    #[std::prelude::v1::test]
    fn the_pointer_holds_a_toast() {
        let t0 = Instant::now();
        let mut s = Toasts::default();
        s.push(Kind::Done, "Saved", t0);
        s.hover(1, true, at(t0, 1000));
        assert_eq!(s.step(at(t0, 10_000)).due, None, "nothing is due while it is held");
        assert!(s.live(Kind::Done).is_some(), "held for 9 s, still there");
        s.hover(1, false, at(t0, 10_000));
        let step = s.step(at(t0, 10_000));
        assert_eq!(step.due, Some(at(t0, 12_000)), "the 2 s it had left start again when the pointer leaves");
        s.step(at(t0, 12_010));
        assert!(s.live(Kind::Done).is_none());
    }

    #[std::prelude::v1::test]
    fn closing_sends_it_out_and_a_new_one_of_the_kind_gets_a_new_label() {
        let t0 = Instant::now();
        let mut s = Toasts::default();
        s.push(Kind::Problem, "Sign in first", t0);
        s.close(Kind::Problem, at(t0, 500));
        assert!(s.live(Kind::Problem).is_none());
        assert!(s.step(at(t0, 550)).moving, "it is on its way out");
        s.push(Kind::Problem, "Overloaded", at(t0, 600));
        assert_eq!(s.list.len(), 2, "the one leaving finishes; the new one arrives beside it");
        assert_eq!(s.live(Kind::Problem).map(|t| t.text.as_str()), Some("Overloaded"));
        s.step(at(t0, 900));
        assert_eq!(s.list.len(), 1);
    }

    #[std::prelude::v1::test]
    fn arriving_and_leaving_ask_for_frames_and_a_settled_stack_waits() {
        let t0 = Instant::now();
        let mut s = Toasts::default();
        assert_eq!(s.step(t0), Step { moving: false, due: None });
        s.push(Kind::Done, "Copied", t0);
        assert!(s.step(at(t0, 100)).moving, "rising in");
        assert_eq!(s.step(at(t0, 500)), Step { moving: false, due: Some(at(t0, 3000)) });
    }
}

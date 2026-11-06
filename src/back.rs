//! Turn it over: the back of the note. Embroiderers judge work by its back, where every
//! knot and jump shows; the back of a note lists every thread it has had, oldest first,
//! with how each one ended (tied off, let stand, snipped, or still open), the words it was
//! about before and after, and your reasons for letting a point stand, which you can add
//! or change here. Nothing is stored for it: it reads the threads (`Thread::ending`,
//! `ended_at`, `first_quote`, `reason`).
//!
//! The page keeps one [`Back`] and swaps its contents for [`view`] while it shows; the
//! sheet, its border and the linen stay. The pure parts (the [`Entry`] model, dates, the
//! turn's timing, the Markdown) are tested below. Design: `design/features/let-it-stand.md`,
//! sections 1.2, 1.3 and 7.

use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Instant;

use crate::note_page::NotePage;
use crate::store::{Ending, Message, Note, Role, Store, ThoughtKind, Thread};
use crate::theme::{DISPLAY, Palette, SANS, SERIF, Theme, ts};
use crate::ui::{self, Keyed};

/// What the back says when the note has had no thread yet.
pub const EMPTY: &str = "Nothing here yet. Every thread a mind opens on this note will be listed here, however it ends.";
/// The reason box's placeholder.
const REASON_HINT: &str = "Why your words stand\u{2026}";
/// What an entry says instead of Reopen or Show on the page once its words are gone.
const WORDS_GONE: &str = "Its words are no longer in the note.";

/// The turn: the page's contents fade out over the first half while the fold's shadow
/// sweeps across the sheet, and the other side fades in over the second half. Seconds.
const TURN: f32 = 0.28;
const HALF: f32 = 0.14;
/// With reduced motion the turn is a crossfade this long.
const FADE: f32 = 0.12;
/// How strong a snipped thread's loose tail is drawn: fainter than the other marks, and
/// still 3:1 on the page in every thread (sage in linen is the closest, at 3.3:1).
pub const TAIL: f32 = 0.78;

// ----- the back's state, kept by the page ---------------------------------------------

/// The back of the note, as the page keeps it.
pub struct Back {
    /// The back is on the sheet (or the sheet is turning to it).
    pub shown: bool,
    /// When the sheet last began to turn, either way.
    turned_at: Option<Instant>,
    /// The side drawn last frame: the back starts at its top when it first shows.
    drawn_back: bool,
    /// Where the front was scrolled when it was turned over, to go back to.
    front_scroll: Option<Point<Pixels>>,
    /// The entry whose reason is being written.
    pub editing: Option<Editing>,
    /// Entries whose messages between the first and the last are unfolded.
    pub unfolded: HashSet<String>,
    /// The back has the keyboard while it shows, so typing never reaches the hidden note.
    pub focus: FocusHandle,
}

/// The one-line box a reason is written in.
pub struct Editing {
    pub id: String,
    pub input: Entity<InputState>,
    _sub: Subscription,
}

impl Back {
    pub fn new(cx: &mut App) -> Self {
        Self { shown: false, turned_at: None, drawn_back: false, front_scroll: None, editing: None, unfolded: HashSet::new(), focus: cx.focus_handle() }
    }

    /// Turn to the back, remembering where the front was scrolled.
    pub fn show(&mut self, front_scroll: Point<Pixels>) {
        if self.shown {
            return;
        }
        self.shown = true;
        self.turned_at = Some(Instant::now());
        self.front_scroll = Some(front_scroll);
        self.editing = None;
    }

    /// Turn to the front: where the front was scrolled, if the back showed.
    pub fn hide(&mut self) -> Option<Point<Pixels>> {
        if !self.shown {
            return None;
        }
        self.shown = false;
        self.turned_at = Some(Instant::now());
        self.editing = None;
        self.front_scroll.take()
    }

    /// Another note: its front, with nothing turning.
    pub fn reset(&mut self) {
        self.shown = false;
        self.turned_at = None;
        self.drawn_back = false;
        self.front_scroll = None;
        self.editing = None;
        self.unfolded.clear();
    }

    /// The sheet now: `since` turns an instant into seconds of animation time (the page's
    /// slow motion included).
    pub fn turn_now(&self, since: impl Fn(Instant) -> f32, reduced: bool) -> Turn {
        turn(self.shown, self.turned_at.map(since), reduced)
    }

    /// Whether the back is drawn for the first time this frame (and the page should show
    /// it from its top); remembers the side drawn.
    pub fn first_drawn(&mut self, turn: &Turn) -> bool {
        let first = turn.back && !self.drawn_back;
        self.drawn_back = turn.back;
        first
    }
}

/// The sheet partway through a turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    /// The back is on the sheet; else the front.
    pub back: bool,
    /// How much of that side shows, 0..1.
    pub opacity: f32,
    /// Where the fold's shadow is while it sweeps, from the sheet's left (0) to its right (1).
    pub fold: Option<f32>,
}

impl Turn {
    pub fn moving(&self) -> bool {
        self.fold.is_some() || self.opacity < 1.
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// The sheet `since` seconds after it began to turn toward the back (`to_back`) or the
/// front. Turning over, the front fades out while the fold sweeps from the right edge to
/// the left, and the back fades in over the second half. Turning back, the front is on
/// the sheet from the first frame, fading in as the fold sweeps the other way, so a key
/// pressed straight after reaches the note. Reduced motion: a 120 ms crossfade.
pub fn turn(to_back: bool, since: Option<f32>, reduced: bool) -> Turn {
    let Some(s) = since.map(|s| s.max(0.)) else { return Turn { back: to_back, opacity: 1., fold: None } };
    let (len, half) = if reduced { (FADE, FADE / 2.) } else { (TURN, HALF) };
    let fold = (!reduced && s < TURN).then(|| if to_back { 1. - ease(s / TURN) } else { ease(s / TURN) });
    if !to_back {
        return Turn { back: false, opacity: (s / len).min(1.), fold };
    }
    if s < half {
        Turn { back: false, opacity: 1. - s / half, fold }
    } else {
        Turn { back: true, opacity: ((s - half) / half).min(1.), fold }
    }
}

// ----- the entries ------------------------------------------------------------------

/// One thread as the back shows it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub ending: Ending,
    pub opened: DateTime<Utc>,
    /// When it was tied off or dismissed; none if it ended before threads kept the date.
    pub ended: Option<DateTime<Utc>>,
    /// The mind's name as the thread keeps it (empty on old threads: the note's own mind).
    pub mind: String,
    pub kind: ThoughtKind,
    /// The words it was opened on, when they have changed since: struck through, above.
    pub before: Option<String>,
    /// The words it is on now.
    pub after: String,
    pub messages: Vec<Message>,
    pub reason: Option<String>,
    /// Its words are still in the note, so it can be shown there or opened on them again.
    pub words_found: bool,
    pub sketch: bool,
}

/// What an entry offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Show on the page: open and tied-off threads.
    Show,
    /// Add a reason, or Change the reason: points let stand.
    Reason,
    /// Reopen: points let stand and snipped threads, while their words are in the note.
    Reopen,
    /// Unfold the messages between the first and the last.
    More,
}

impl Act {
    fn name(self) -> &'static str {
        match self {
            Act::Show => "show",
            Act::Reason => "reason",
            Act::Reopen => "reopen",
            Act::More => "more",
        }
    }
}

/// A kind as the tags write it.
pub fn kind_label(kind: ThoughtKind) -> &'static str {
    match kind {
        ThoughtKind::Question => "question",
        ThoughtKind::Correction => "correction",
        ThoughtKind::Ask => "you asked",
        ThoughtKind::Sketch => "sketch",
        ThoughtKind::Answer => "answer",
    }
}

/// A date as the back writes it, "6 Nov", with the year when it is not `this_year`. None
/// for a date a thread never kept (older threads load with the epoch).
pub fn day<Tz: TimeZone>(at: DateTime<Utc>, tz: &Tz, this_year: i32) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    if at.timestamp() == 0 {
        return None;
    }
    let local = at.with_timezone(tz);
    Some(if local.year() == this_year { local.format("%-d %b").to_string() } else { local.format("%-d %b %Y").to_string() })
}

impl Entry {
    pub fn of(t: &Thread, text: &str) -> Self {
        Self {
            id: t.id.clone(),
            ending: t.ending(),
            opened: t.created_at,
            ended: t.ended_at,
            mind: t.mind_name.clone(),
            kind: t.kind,
            before: t.first_quote.clone().filter(|b| *b != t.quote),
            after: t.quote.clone(),
            messages: t.messages.clone(),
            reason: t.reason.clone(),
            words_found: !t.quote.is_empty() && text.contains(t.quote.as_str()),
            sketch: t.drawing.is_some(),
        }
    }

    /// The mind's name as the entry shows it.
    pub fn who(&self) -> &str {
        if self.mind.is_empty() { "Inkling" } else { &self.mind }
    }

    /// The words it was opened on.
    pub fn words_before(&self) -> &str {
        self.before.as_deref().unwrap_or(&self.after)
    }

    /// After the mind's name on the top row: " · question · 5 Nov".
    pub fn meta(&self, tz: &impl TimeZone<Offset: std::fmt::Display>, this_year: i32) -> String {
        let mut s = format!("\u{b7} {}", kind_label(self.kind));
        if let Some(d) = day(self.opened, tz, this_year) {
            s.push_str(&format!(" \u{b7} {d}"));
        }
        s
    }

    /// The ending as a status line: "Tied off · 6 Nov", "Still open".
    pub fn ending_line(&self, tz: &impl TimeZone<Offset: std::fmt::Display>, this_year: i32) -> String {
        match self.ended.filter(|_| self.ending != Ending::Open).and_then(|at| day(at, tz, this_year)) {
            Some(d) => format!("{} \u{b7} {d}", self.ending.label()),
            None => self.ending.label().to_string(),
        }
    }

    /// Whose thread it was, for the Markdown: "Skeptic’s question", "You asked Skeptic".
    pub fn whose(&self) -> String {
        match self.kind {
            ThoughtKind::Ask => format!("You asked {}", self.who()),
            k => format!("{}\u{2019}s {}", self.who(), kind_label(k)),
        }
    }

    /// Replies, counted as the tag counts them (an asked thread starts with two messages).
    pub fn replies(&self) -> usize {
        self.messages.len().saturating_sub(if self.kind == ThoughtKind::Ask { 2 } else { 1 })
    }

    /// The mind's last word on a thread that was tied off, when it is not also its first:
    /// it goes under the ending line, because it says why the thread was tied off.
    pub fn why(&self) -> Option<&Message> {
        let last = self.messages.last().filter(|m| m.role == Role::Mind)?;
        (self.ending == Ending::TiedOff && self.messages.len() > 1).then_some(last)
    }

    /// The messages above the ending line: the first, then the last, with how many fold
    /// between them (all of them when `unfolded`). The last is left out when it goes under
    /// the ending line instead ([`Self::why`]).
    pub fn said(&self, unfolded: bool) -> (Vec<&Message>, usize) {
        let mut list: Vec<&Message> = self.messages.iter().collect();
        if self.why().is_some() {
            list.pop();
        }
        if unfolded || list.len() <= 2 {
            return (list, 0);
        }
        let folded = list.len() - 2;
        (vec![list[0], list[list.len() - 1]], folded)
    }

    /// What the entry offers, in order.
    pub fn acts(&self) -> Vec<Act> {
        let mut out = vec![];
        if matches!(self.ending, Ending::Open | Ending::TiedOff) && self.words_found {
            out.push(Act::Show);
        }
        if self.ending == Ending::LetStand {
            out.push(Act::Reason);
        }
        if matches!(self.ending, Ending::LetStand | Ending::Snipped) && self.words_found {
            out.push(Act::Reopen);
        }
        out
    }
}

/// Every thread the note has had, in the order they were opened. A thread that never got
/// a word (its first answer did not come) is left out: there is nothing to read.
pub fn entries(threads: &[Thread], text: &str) -> Vec<Entry> {
    let mut out: Vec<Entry> = threads.iter().filter(|t| !t.messages.is_empty() || t.drawing.is_some()).map(|t| Entry::of(t, text)).collect();
    // stable: threads opened in the same instant keep the order they were made in
    out.sort_by_key(|e| e.opened);
    out
}

/// The back as the dump shows it: `{id, ending, reason, before, after, mind, kind, ...}`.
pub fn dump(threads: &[Thread], text: &str) -> Value {
    let short = |id: &str| id.chars().take(8).collect::<String>();
    Value::Array(
        entries(threads, text)
            .iter()
            .map(|e| {
                json!({
                    "id": short(&e.id), "ending": format!("{:?}", e.ending), "reason": e.reason,
                    "before": e.words_before(), "after": e.after, "changed": e.before.is_some(),
                    "mind": e.who(), "kind": format!("{:?}", e.kind), "found": e.words_found,
                    "ended": e.ended.is_some(), "acts": e.acts().iter().map(|a| a.name()).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// The page's dump gains `turned` (the back shows), `back` (its entries, in order),
/// `back_editing` (the entry whose reason box is open), and `focus` says "back" or
/// "back:reason-box" when the back or the box has the keyboard.
pub fn dump_into(state: &mut Value, page: &NotePage, window: &Window, cx: &App) {
    state["turned"] = json!(page.back.shown);
    state["back"] = dump(&page.note.threads, page.editor.read(cx).text());
    let editing = page.back.editing.as_ref();
    state["back_editing"] = json!(editing.map(|e| e.id.chars().take(8).collect::<String>()));
    if editing.is_some_and(|e| e.input.read(cx).focus_handle(cx).is_focused(window)) {
        state["focus"] = json!("back:reason-box");
    } else if page.back.focus.is_focused(window) {
        state["focus"] = json!("back");
    }
}

// ----- the back as Markdown (⇧⌘C on the back) -------------------------------------

/// The back as Markdown: a heading per thread with how it ended, whose it was and when it
/// was opened; its words (struck through, then as they are now, when they changed); every
/// message; and the reason for a point let stand.
pub fn to_markdown(note: &Note) -> String {
    markdown_in(note, &Local, Local::now().year())
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn markdown_in(note: &Note, tz: &impl TimeZone<Offset: std::fmt::Display>, this_year: i32) -> String {
    let title = note.title.trim();
    let mut out = format!("# The back of \u{201c}{}\u{201d}\n", if title.is_empty() { "Untitled" } else { title });
    let list = entries(&note.threads, &note.joined_text());
    if list.is_empty() {
        out.push_str(&format!("\n{EMPTY}\n"));
        return out;
    }
    for e in &list {
        let mut head = format!("{} \u{b7} {}", e.ending.label(), e.whose());
        if let Some(d) = day(e.opened, tz, this_year) {
            head.push_str(&format!(" \u{b7} {d}"));
        }
        out.push_str(&format!("\n## {head}\n\n"));
        match &e.before {
            Some(b) => out.push_str(&format!("~~{}~~\n{}\n", one_line(b), one_line(&e.after))),
            None => out.push_str(&format!("\u{201c}{}\u{201d}\n", one_line(&e.after))),
        }
        if !e.messages.is_empty() {
            let quoted: Vec<String> = e
                .messages
                .iter()
                .map(|m| {
                    let who = if m.role == Role::Mind { e.who() } else { "You" };
                    let text = format!("*{who}:* {}", m.text.trim());
                    text.lines().map(|l| if l.trim().is_empty() { ">\n".to_string() } else { format!("> {l}\n") }).collect()
                })
                .collect();
            out.push('\n');
            out.push_str(&quoted.join(">\n"));
        }
        if e.sketch {
            out.push_str("\n*With a sketch.*\n");
        }
        if e.ending == Ending::LetStand {
            match &e.reason {
                Some(r) => out.push_str(&format!("\nYour reason: {r}\n")),
                None => out.push_str("\nNo reason given.\n"),
            }
        }
    }
    out
}

// ----- the view ---------------------------------------------------------------------

/// How an entry is shown, and what its controls do.
pub struct Look<V> {
    /// Every message shows, not just the first and the last.
    pub unfolded: bool,
    /// The box a reason is being written in, shown in place of the reason.
    pub reason_box: Option<AnyElement>,
    /// Runs one of the entry's controls: (the thread's id, which control).
    pub on: Rc<dyn Fn(&mut V, &str, Act, &mut Window, &mut Context<V>)>,
}

/// One entry: the mind and kind and when, the words before and after, what was said, how
/// it ended, the reason, and its controls; its ending's mark in the gutter to its left.
/// Written for any view, so a list of threads elsewhere can show the same entry.
pub fn entry<V: 'static>(e: &Entry, color: Hsla, t: &Theme, look: Look<V>, cx: &mut Context<V>) -> AnyElement {
    let p = t.p;
    let year = Local::now().year();
    let words = t.note_size - px(2.);
    let words_lh = px((words.as_f32() * 1.5).round());
    let control = |act: Act, label: String, cx: &mut Context<V>| {
        let (on, id) = (look.on.clone(), e.id.clone());
        ui::ghost_button(SharedString::from(format!("back-{}-{}", act.name(), e.id)), label, t)
            .keyed_as(format!("back:{}-{}", act.name(), e.id), 4., color, cx, move |this, window, cx| on(this, &id, act, window, cx))
    };
    let mut col = div().relative().flex().flex_col().font_family(SANS);
    col = col.child(mark(e, color, &p));
    // the mind, in its thread, then the kind and when it was opened
    col = col.child(
        div()
            .h(px(18.))
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(ts::SMALL))
            .child(div().min_w(px(0.)).truncate().font_weight(FontWeight::SEMIBOLD).text_color(color).child(e.who().to_string()))
            .child(div().flex_none().text_color(p.faint).child(e.meta(&Local, year))),
    );
    // the words, in your typeface: the old ones struck through above the ones there now
    let mut said = div().mt(px(5.)).flex().flex_col().gap(px(2.));
    if let Some(before) = &e.before {
        said = said.child(div().font_family(SERIF).text_size(words).line_height(words_lh).text_color(p.faint).line_through().child(before.clone()));
    }
    said = said.child(div().font_family(SERIF).text_size(words).line_height(words_lh).text_color(p.ink).child(e.after.clone()));
    col = col.child(said);
    // what was said: the first message, then the last, the rest folded between them
    let (shown, folded) = e.said(look.unfolded);
    let mut talk = div().mt(px(8.)).flex().flex_col().gap(px(6.));
    for (i, m) in shown.iter().enumerate() {
        if i == 1 && folded > 0 {
            talk = talk.child(div().flex().child(control(Act::More, format!("{folded} more"), cx)));
        }
        talk = talk.child(message(m, t));
    }
    if e.sketch {
        talk = talk.child(div().text_size(px(ts::SMALL)).text_color(p.faint).child("With a sketch"));
    }
    if !shown.is_empty() || e.sketch {
        col = col.child(talk);
    }
    // how it ended, set like a status line, and what goes under it
    col = col.child(div().mt(px(10.)).text_size(px(ts::SMALL)).line_height(px(16.)).font_weight(FontWeight::MEDIUM).text_color(p.graphite).child(e.ending_line(&Local, year)));
    let under: Option<AnyElement> = match e.ending {
        Ending::TiedOff => e.why().map(|m| message(m, t).into_any_element()),
        Ending::LetStand => Some(match (look.reason_box, &e.reason) {
            (Some(b), _) => b,
            (None, Some(r)) => div().font_family(SERIF).text_size(words).line_height(words_lh).text_color(p.ink).child(format!("\u{201c}{r}\u{201d}")).into_any_element(),
            (None, None) => div().text_size(px(ts::SMALL)).text_color(p.faint).child("No reason given.").into_any_element(),
        }),
        Ending::Open => (e.replies() > 0).then(|| {
            let n = e.replies();
            div().text_size(px(ts::SMALL)).text_color(p.faint).child(if n == 1 { "1 reply".to_string() } else { format!("{n} replies") }).into_any_element()
        }),
        Ending::Snipped => None,
    };
    if let Some(u) = under {
        col = col.child(div().mt(px(4.)).child(u));
    }
    // the quiet controls, as the tags have them
    let mut row = div().mt(px(8.)).flex().flex_wrap().items_center().gap(px(14.));
    for act in e.acts() {
        let label = match act {
            Act::Show => "Show on the page".to_string(),
            Act::Reason if e.reason.is_some() => "Change the reason".to_string(),
            Act::Reason => "Add a reason".to_string(),
            Act::Reopen => "Reopen".to_string(),
            Act::More => continue,
        };
        row = row.child(control(act, label, cx));
    }
    if !e.words_found {
        row = row.child(div().text_size(px(ts::SMALL)).text_color(p.faint).child(WORDS_GONE));
    }
    col.child(row).into_any_element()
}

/// A message as the tag sets it: the mind's words in its lettering, yours beside "You".
fn message(m: &Message, t: &Theme) -> Div {
    let p = t.p;
    match m.role {
        Role::Mind => div().font_family(t.voice_font.clone()).text_size(px(ts::BODY)).line_height(px(20.)).text_color(p.ink).child(m.text.clone()),
        Role::You => div()
            .flex()
            .gap(px(6.))
            .text_size(px(ts::UI))
            .line_height(px(18.))
            .text_color(p.graphite)
            .child(div().flex_none().text_color(p.faint).child("You"))
            .child(div().flex_1().min_w(px(0.)).child(m.text.clone())),
    }
}

/// The entry's ending in the gutter, where the front draws its read marks, level with its
/// top row, in the mind's thread: a French knot for tied off, a tacking stitch for let
/// stand (the words held), a loose tail for snipped, and the tab's own ? or × while open.
/// The four differ in shape, so the colour is never the only signal.
fn mark(e: &Entry, color: Hsla, p: &Palette) -> AnyElement {
    let at = div().absolute().top_0().left(px(-26. - 8.)).w(px(16.)).h(px(18.));
    match e.ending {
        Ending::Open => at
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(ts::UI))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(color)
            .child(if e.kind == ThoughtKind::Correction { "\u{d7}" } else { "?" })
            .into_any_element(),
        ending => {
            let shadow = p.shadow;
            at.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let c = bounds.center();
                        match ending {
                            Ending::TiedOff => crate::stitch::french_knot(window, c, px(4.2), color, 1.),
                            Ending::LetStand => crate::stitch::tacking_stitch(window, c, color),
                            _ => loose_tail(window, c, color.opacity(TAIL), shadow),
                        }
                    },
                )
                .size_full(),
            )
            .into_any_element()
        }
    }
}

/// A snipped thread's end: a short tail of thread coming up through the cloth and cut,
/// its last fibres splayed where the scissors went through.
fn loose_tail(window: &mut Window, c: Point<Pixels>, color: Hsla, shadow: Hsla) {
    let at = |x: f32, y: f32| c + point(px(x), px(y));
    let strand = |b: &mut PathBuilder, off: Point<Pixels>| {
        b.move_to(at(-5., -2.5) + off);
        b.cubic_bezier_to(at(4., 1.5) + off, at(-1.5, -6.) + off, at(0.5, 4.) + off);
    };
    let mut s = PathBuilder::stroke(px(2.));
    strand(&mut s, point(px(0.5), px(1.2)));
    if let Ok(path) = s.build() {
        window.paint_path(path, shadow);
    }
    let mut b = PathBuilder::stroke(px(1.75));
    strand(&mut b, point(px(0.), px(0.)));
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
    let mut f = PathBuilder::stroke(px(0.8));
    f.move_to(at(4., 1.5));
    f.line_to(at(6.2, 0.6));
    f.move_to(at(4., 1.5));
    f.line_to(at(5.9, 3.3));
    if let Ok(path) = f.build() {
        window.paint_path(path, color);
    }
    // the hole it comes up through
    crate::stitch::hole(window, at(-5., -2.5), shadow.opacity(2.));
}

/// The fold's soft shadow, `at` of the way across a sheet `width` wide, from its left.
pub fn fold_shadow(at: f32, width: f32, p: &Palette) -> AnyElement {
    let band = 150.;
    let deep = p.shadow.opacity(if p.dark { 1.25 } else { 1.9 });
    let clear = deep.opacity(0.);
    let x = at * width - band / 2.;
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded(px(4.))
        .overflow_hidden()
        .child(div().absolute().top_0().bottom_0().left(px(x)).w(px(band / 2.)).bg(linear_gradient(90., linear_color_stop(clear, 0.), linear_color_stop(deep, 1.))))
        .child(div().absolute().top_0().bottom_0().left(px(x + band / 2.)).w(px(band / 2.)).bg(linear_gradient(90., linear_color_stop(deep, 0.), linear_color_stop(clear, 1.))))
        .into_any_element()
}

/// The back of the page's note, in its column: the header (what this is, Turn back), the
/// note's title in graphite, because this is the record of the note and not the note, and
/// every entry. `left`, `column` and `page` are the page's geometry.
pub fn view(page: &mut NotePage, own: &str, left: f32, column: f32, width: f32, cx: &mut Context<NotePage>) -> AnyElement {
    let t = Theme::get(cx).clone();
    let p = t.p;
    let chosen = cx.global::<Store>().settings.ink;
    let (title, text) = {
        let ed = page.editor.read(cx);
        (ed.title(), ed.text().to_string())
    };
    let list = entries(&page.note.threads, &text);
    let header = div()
        .flex()
        .items_center()
        .min_h(px(24.))
        .text_size(px(ts::UI))
        .opacity(page.chrome.get())
        .child(div().text_color(p.faint).child("Every thread, oldest first"))
        .child(div().flex_1())
        .child(
            div()
                .id("turn-back")
                .h(px(24.))
                .flex()
                .items_center()
                .text_color(p.graphite)
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .active(|s| s.opacity(0.7))
                .child("Turn back")
                .keyed_as("back:turn-back", 4., t.voice_ink, cx, |page, window, cx| page.turn_back(window, cx)),
        );
    let n = t.note_size.as_f32();
    let title = div()
        .font_family(DISPLAY)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px((n * 1.9).round()))
        .line_height(px((n * 1.9 * 1.22).round()))
        .text_color(p.graphite)
        .child(if title.trim().is_empty() { "Untitled".to_string() } else { title });
    let on: Rc<dyn Fn(&mut NotePage, &str, Act, &mut Window, &mut Context<NotePage>)> = Rc::new(act_on);
    let mut body = div().mt(px((n * 1.1).round())).flex().flex_col().gap(px(30.));
    if list.is_empty() {
        body = body.child(div().text_size(px(ts::BODY)).line_height(px(20.)).text_color(p.graphite).child(EMPTY));
    }
    for e in &list {
        let color = crate::note_page::mind_thread(&e.mind, own, chosen, &t);
        let reason_box = page.back.editing.as_ref().filter(|ed| ed.id == e.id).map(|ed| reason_box(&ed.input, color, &t));
        let look = Look { unfolded: page.back.unfolded.contains(&e.id), reason_box, on: on.clone() };
        body = body.child(entry(e, color, &t, look, cx));
    }
    div()
        .relative()
        .w(px(width))
        .flex()
        .flex_col()
        .font_family(SANS)
        .child(div().ml(px(left)).w(px(column)).tab_group().tab_index(ui::tab::HEADER).child(header))
        .child(div().h(px(18.)))
        .child(div().ml(px(left)).w(px(column)).tab_group().tab_index(ui::tab::NOTE).child(title).child(body))
        .child(div().h(px(220.)))
        .into_any_element()
}

/// The one-line box a reason is written in, where the reason goes.
fn reason_box(input: &Entity<InputState>, color: Hsla, t: &Theme) -> AnyElement {
    let p = t.p;
    div()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .bg(p.card)
        .border_1()
        .border_color(color.opacity(0.5))
        .font_family(SERIF)
        .text_color(p.ink)
        .child(Input::new(input).appearance(false).px_0().text_size(t.note_size - px(2.)))
        .into_any_element()
}

/// One of an entry's controls, pressed.
fn act_on(page: &mut NotePage, id: &str, act: Act, window: &mut Window, cx: &mut Context<NotePage>) {
    match act {
        Act::Show => page.from_back(id, false, window, cx),
        Act::Reopen => page.from_back(id, true, window, cx),
        Act::Reason => edit_reason(page, id, window, cx),
        Act::More => {
            if !page.back.unfolded.remove(id) {
                page.back.unfolded.insert(id.to_string());
            }
            cx.notify();
        }
    }
}

/// Add a reason, or Change the reason: the reason line becomes a one-line box with the
/// reason in it. Return keeps it (`keep_reason`); Esc leaves it as it was.
pub fn edit_reason(page: &mut NotePage, id: &str, window: &mut Window, cx: &mut Context<NotePage>) {
    let now = page.note.threads.iter().find(|t| t.id == id).and_then(|t| t.reason.clone()).unwrap_or_default();
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(REASON_HINT));
    input.update(cx, |i, cx| {
        i.set_value(now, window, cx);
        i.focus(window, cx);
    });
    let sub = cx.subscribe_in(&input, window, |page: &mut NotePage, _, ev: &InputEvent, window, cx| {
        if let InputEvent::PressEnter { .. } = ev {
            keep_reason(page, window, cx);
        }
    });
    page.back.editing = Some(Editing { id: id.to_string(), input, _sub: sub });
    cx.notify();
}

/// Keep what the box says as the reason (an empty box takes the reason away). Every mind
/// reads it from the next request; the keyboard goes back to the entry's control.
pub fn keep_reason(page: &mut NotePage, window: &mut Window, cx: &mut Context<NotePage>) {
    let Some(ed) = page.back.editing.take() else { return };
    let text = ed.input.read(cx).value().to_string();
    if let Some(t) = page.note.threads.iter_mut().find(|t| t.id == ed.id) {
        t.set_reason(&text);
    }
    page.flush(cx);
    ui::key(format!("back:reason-{}", ed.id), cx).focus(window, cx);
    cx.notify();
}

/// Esc on the back: the reason box closes, leaving the reason as it was; else the sheet
/// turns back to the front.
pub fn escape(page: &mut NotePage, window: &mut Window, cx: &mut Context<NotePage>) {
    match page.back.editing.take() {
        Some(ed) => ui::key(format!("back:reason-{}", ed.id), cx).focus(window, cx),
        None => page.turn_back(window, cx),
    }
    cx.notify();
}

/// The back's script steps, on entry N of the back (from 1): `back-reason: N text` writes
/// a reason as the box does, `back-reopen: N` and `back-show: N` press those controls.
pub fn script_step(page: &mut NotePage, step: &crate::script::Step, window: &mut Window, cx: &mut Context<NotePage>) {
    use crate::script::Step;
    let (n, act) = match step {
        Step::BackReason(n, _) => (*n, Act::Reason),
        Step::BackReopen(n) => (*n, Act::Reopen),
        Step::BackShow(n) => (*n, Act::Show),
        _ => return,
    };
    if !page.back.shown {
        log::warn!("script: the note is not turned over");
        return;
    }
    let text = page.editor.read(cx).text().to_string();
    let Some(e) = n.checked_sub(1).and_then(|i| entries(&page.note.threads, &text).into_iter().nth(i)) else {
        log::warn!("script: no entry {n} on the back");
        return;
    };
    if !e.acts().contains(&act) {
        log::warn!("script: entry {n} on the back has no {} control", act.name());
        return;
    }
    match step {
        Step::BackReason(_, reason) => {
            edit_reason(page, &e.id, window, cx);
            if let Some(ed) = &page.back.editing {
                ed.input.update(cx, |i, cx| i.set_value(reason.clone(), window, cx));
            }
            keep_reason(page, window, cx);
        }
        _ => act_on(page, &e.id, act, window, cx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Side, ThreadStatus};

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
    }

    fn thread(id: &str, quote: &str, kind: ThoughtKind, status: ThreadStatus, opened: DateTime<Utc>, said: &[&str]) -> Thread {
        Thread {
            id: id.into(),
            start: 0,
            end: quote.len(),
            quote: quote.into(),
            kind,
            messages: said
                .iter()
                .map(|s| match s.strip_prefix("you: ") {
                    Some(mine) => Message { role: Role::You, text: mine.into(), at: opened },
                    None => Message { role: Role::Mind, text: s.to_string(), at: opened },
                })
                .collect(),
            drawing: None,
            status,
            side: Side::Right,
            mind_name: "Inkling".into(),
            tag_offset: (0., 0.),
            created_at: opened,
            reason: None,
            ended_at: None,
            first_quote: None,
        }
    }

    /// The design board's note: the correction on "5:30" fixed and tied off, the question
    /// on the long run let stand.
    fn running_plan() -> Note {
        let mut note = Note::new("Sub-50 10k by March", crate::store::Mode::ThinksWithYou, None);
        note.blocks.push(crate::store::Block {
            kind: crate::store::BlockKind::Paragraph,
            text: "If I hold 4:55 for the whole race, I\u{2019}ll come in just under 50. Sunday is the long run. I\u{2019}ll add a kilometre every week until I reach 16.".into(),
        });
        let mut t1 = thread(
            "t1",
            "If I hold 5:30 for the whole race, I\u{2019}ll come in just under 50",
            ThoughtKind::Correction,
            ThreadStatus::Open,
            at(4, 9),
            &["At 5:30 a km, 10 km takes 55:00.", "4:55 a km comes to 49:10. That works."],
        );
        t1.set_quote("If I hold 4:55 for the whole race, I\u{2019}ll come in just under 50");
        t1.status = ThreadStatus::Resolved;
        t1.ended_at = Some(at(5, 9));
        let mut t2 = thread("t2", "add a kilometre every week until I reach 16", ThoughtKind::Question, ThreadStatus::Dismissed, at(6, 9), &["Starting from how far?"]);
        t2.ended_at = Some(at(6, 10));
        t2.set_reason("Ten weeks out, from 8 km.");
        note.threads = vec![t2, t1];
        note
    }

    #[std::prelude::v1::test]
    fn entries_come_in_the_order_they_were_opened() {
        let note = running_plan();
        let list = entries(&note.threads, &note.joined_text());
        assert_eq!(list.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["t1", "t2"]);
        // threads opened in the same instant keep the order they were made in
        let same = [
            thread("a", "x", ThoughtKind::Question, ThreadStatus::Open, at(6, 9), &["?"]),
            thread("b", "x", ThoughtKind::Question, ThreadStatus::Open, at(6, 9), &["?"]),
            thread("c", "x", ThoughtKind::Question, ThreadStatus::Open, at(5, 9), &["?"]),
        ];
        assert_eq!(entries(&same, "x").iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["c", "a", "b"]);
        // a thread that never got its first word has nothing to show
        let empty = [thread("e", "x", ThoughtKind::Question, ThreadStatus::Open, at(6, 9), &[])];
        assert!(entries(&empty, "x").is_empty());
    }

    #[std::prelude::v1::test]
    fn an_entry_reads_the_shared_data() {
        let note = running_plan();
        let list = entries(&note.threads, &note.joined_text());
        let (tied, stood) = (&list[0], &list[1]);
        assert_eq!(tied.ending, Ending::TiedOff);
        assert_eq!(tied.before.as_deref(), Some("If I hold 5:30 for the whole race, I\u{2019}ll come in just under 50"));
        assert!(tied.after.contains("4:55") && tied.words_before().contains("5:30"));
        assert_eq!((stood.ending, stood.reason.as_deref(), stood.before.as_deref()), (Ending::LetStand, Some("Ten weeks out, from 8 km."), None));
        assert_eq!(stood.words_before(), stood.after, "unchanged words are the words before and after");
        // first words kept but the same as now: nothing to strike through
        let mut same = thread("s", "four days", ThoughtKind::Question, ThreadStatus::Open, at(6, 9), &["?"]);
        same.first_quote = Some("four days".into());
        assert_eq!(Entry::of(&same, "four days").before, None);
        // the words are looked for in the note as it is now
        assert!(stood.words_found);
        assert!(!Entry::of(&note.threads[0], "a note without those words").words_found);
        assert!(!Entry::of(&thread("q", "", ThoughtKind::Question, ThreadStatus::Dismissed, at(6, 9), &["?"]), "anything").words_found);
        // an old thread with no mind's name is the note's own mind, Inkling by default
        let mut old = thread("o", "x", ThoughtKind::Question, ThreadStatus::Open, at(6, 9), &["?"]);
        old.mind_name.clear();
        assert_eq!(Entry::of(&old, "x").who(), "Inkling");
    }

    #[std::prelude::v1::test]
    fn dates_drop_the_year_this_year_and_old_threads_have_none() {
        assert_eq!(day(at(6, 9), &Utc, 2026).as_deref(), Some("6 Nov"));
        assert_eq!(day(at(6, 9), &Utc, 2027).as_deref(), Some("6 Nov 2026"));
        assert_eq!(day(DateTime::<Utc>::default(), &Utc, 2026), None, "a thread saved before threads kept the date");
        // the day is the writer's own: late on the 6th in UTC is the 7th in India
        let ist = chrono::FixedOffset::east_opt(5 * 3600 + 1800).unwrap();
        assert_eq!(day(at(6, 22), &ist, 2026).as_deref(), Some("7 Nov"));
    }

    #[std::prelude::v1::test]
    fn the_top_row_and_the_ending_line() {
        let note = running_plan();
        let list = entries(&note.threads, &note.joined_text());
        assert_eq!(list[0].meta(&Utc, 2026), "\u{b7} correction \u{b7} 4 Nov");
        assert_eq!(list[0].ending_line(&Utc, 2026), "Tied off \u{b7} 5 Nov");
        assert_eq!(list[1].ending_line(&Utc, 2026), "Let stand \u{b7} 6 Nov");
        let mut e = list[1].clone();
        e.ended = None;
        assert_eq!(e.ending_line(&Utc, 2026), "Let stand", "ended before threads kept the date");
        e.ending = Ending::Open;
        e.ended = Some(at(6, 9));
        assert_eq!(e.ending_line(&Utc, 2026), "Still open");
        let mut asked = thread("a", "x", ThoughtKind::Ask, ThreadStatus::Dismissed, at(6, 9), &["you: Why?", "Because."]);
        asked.mind_name = "Skeptic".into();
        let asked = Entry::of(&asked, "x");
        assert_eq!((asked.ending, asked.meta(&Utc, 2026), asked.whose()), (Ending::Snipped, "\u{b7} you asked \u{b7} 6 Nov".to_string(), "You asked Skeptic".to_string()));
        assert_eq!(list[1].whose(), "Inkling\u{2019}s question");
    }

    #[std::prelude::v1::test]
    fn the_first_and_last_messages_show_and_the_rest_fold() {
        let said = ["one", "you: two", "three", "you: four", "five"];
        let e = Entry::of(&thread("o", "x", ThoughtKind::Question, ThreadStatus::Open, at(6, 9), &said), "x");
        let (shown, folded) = e.said(false);
        assert_eq!((shown.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), folded), (vec!["one", "five"], 3));
        assert_eq!(e.said(true).0.len(), 5);
        assert_eq!(e.replies(), 4);
        // tied off: the mind's last word goes under the ending line, not above it
        let e = Entry::of(&thread("t", "x", ThoughtKind::Question, ThreadStatus::Resolved, at(6, 9), &said), "x");
        assert_eq!(e.why().map(|m| m.text.as_str()), Some("five"));
        let (shown, folded) = e.said(false);
        assert_eq!((shown.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), folded), (vec!["one", "four"], 2));
        // ...unless it is also the first, or the last word was yours
        let one = Entry::of(&thread("t", "x", ThoughtKind::Question, ThreadStatus::Resolved, at(6, 9), &["only"]), "x");
        assert!(one.why().is_none() && one.said(false).0.len() == 1);
        let mine = Entry::of(&thread("t", "x", ThoughtKind::Question, ThreadStatus::Resolved, at(6, 9), &["q", "you: done"]), "x");
        assert!(mine.why().is_none() && mine.said(false).0.len() == 2);
        // an asked thread starts with your question and its answer
        let asked = Entry::of(&thread("a", "x", ThoughtKind::Ask, ThreadStatus::Open, at(6, 9), &["you: Why?", "Because."]), "x");
        assert_eq!(asked.replies(), 0);
    }

    #[std::prelude::v1::test]
    fn each_ending_offers_its_own_controls() {
        let e = |kind, status, found: &str| Entry::of(&thread("x", "the words", kind, status, at(6, 9), &["?"]), found).acts();
        use ThoughtKind::*;
        assert_eq!(e(Question, ThreadStatus::Open, "the words"), [Act::Show]);
        assert_eq!(e(Correction, ThreadStatus::Resolved, "the words"), [Act::Show]);
        assert_eq!(e(Question, ThreadStatus::Dismissed, "the words"), [Act::Reason, Act::Reopen]);
        assert_eq!(e(Sketch, ThreadStatus::Dismissed, "the words"), [Act::Reopen]);
        // once the words are gone, nothing to show or open them on; a reason can still be given
        assert_eq!(e(Question, ThreadStatus::Dismissed, "gone"), [Act::Reason]);
        assert!(e(Ask, ThreadStatus::Dismissed, "gone").is_empty());
        assert!(e(Question, ThreadStatus::Open, "gone").is_empty());
    }

    #[std::prelude::v1::test]
    fn the_back_copies_as_markdown() {
        let md = markdown_in(&running_plan(), &Utc, 2026);
        assert_eq!(
            md,
            "# The back of \u{201c}Sub-50 10k by March\u{201d}\n\
             \n\
             ## Tied off \u{b7} Inkling\u{2019}s correction \u{b7} 4 Nov\n\
             \n\
             ~~If I hold 5:30 for the whole race, I\u{2019}ll come in just under 50~~\n\
             If I hold 4:55 for the whole race, I\u{2019}ll come in just under 50\n\
             \n\
             > *Inkling:* At 5:30 a km, 10 km takes 55:00.\n\
             >\n\
             > *Inkling:* 4:55 a km comes to 49:10. That works.\n\
             \n\
             ## Let stand \u{b7} Inkling\u{2019}s question \u{b7} 6 Nov\n\
             \n\
             \u{201c}add a kilometre every week until I reach 16\u{201d}\n\
             \n\
             > *Inkling:* Starting from how far?\n\
             \n\
             Your reason: Ten weeks out, from 8 km.\n"
        );
        // no reason, your own words, a sketch, a message of two lines, and an untitled note
        let mut note = running_plan();
        note.title.clear();
        note.threads[0].reason = None;
        note.threads[0].messages.push(Message { role: Role::You, text: "It is ten weeks.\nFrom 8 km.".into(), at: at(6, 9) });
        note.threads[0].drawing = Some("<svg/>".into());
        let md = markdown_in(&note, &Utc, 2026);
        assert!(md.starts_with("# The back of \u{201c}Untitled\u{201d}\n"), "{md}");
        assert!(md.contains("> *Inkling:* Starting from how far?\n>\n> *You:* It is ten weeks.\n> From 8 km.\n\n*With a sketch.*\n\nNo reason given.\n"), "{md}");
        // a note with no threads
        note.threads.clear();
        assert_eq!(markdown_in(&note, &Utc, 2026), format!("# The back of \u{201c}Untitled\u{201d}\n\n{EMPTY}\n"));
    }

    #[std::prelude::v1::test]
    fn the_sheet_turns_over_and_back() {
        // at rest, the side it is on, whole
        assert_eq!(turn(false, None, false), Turn { back: false, opacity: 1., fold: None });
        assert_eq!(turn(true, Some(5.), false), Turn { back: true, opacity: 1., fold: None });
        // turning over: the front fades out, then the back fades in, as the fold sweeps
        // from the right edge to the left
        let a = turn(true, Some(0.), false);
        assert!(!a.back && a.opacity == 1. && a.fold == Some(1.));
        let b = turn(true, Some(0.07), false);
        assert!(!b.back && (b.opacity - 0.5).abs() < 1e-4 && b.fold.is_some_and(|f| f < 1. && f > 0.5));
        let c = turn(true, Some(0.21), false);
        assert!(c.back && (c.opacity - 0.5).abs() < 1e-4 && c.fold.is_some_and(|f| f < 0.5 && f > 0.));
        assert!(!turn(true, Some(0.28), false).moving());
        // turning back: the front is on the sheet from the first frame, so a key pressed
        // straight after reaches the note; the fold goes the other way
        let d = turn(false, Some(0.), false);
        assert!(!d.back && d.opacity == 0. && d.fold == Some(0.));
        assert!(turn(false, Some(0.2), false).fold.is_some_and(|f| f > 0.5));
        assert!(!turn(false, Some(0.3), false).moving());
        // reduced motion: a 120 ms crossfade, no fold
        let e = turn(true, Some(0.03), true);
        assert!(!e.back && e.fold.is_none() && (e.opacity - 0.5).abs() < 1e-4);
        assert!(turn(true, Some(0.09), true).back);
        assert!(!turn(true, Some(0.12), true).moving() && !turn(false, Some(0.12), true).moving());
    }

    /// WCAG contrast of `fg` (with its alpha) laid on the opaque `bg`.
    fn contrast_on(fg: Hsla, bg: Hsla) -> f32 {
        let (f, b) = (fg.to_rgb(), bg.to_rgb());
        let mix = |x: f32, y: f32| x * f.a + y * (1. - f.a);
        let lum = |r: f32, g: f32, b: f32| {
            let c = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
            0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b)
        };
        let lf = lum(mix(f.r, b.r), mix(f.g, b.g), mix(f.b, b.b));
        let lb = lum(b.r, b.g, b.b);
        (lf.max(lb) + 0.05) / (lf.min(lb) + 0.05)
    }

    #[std::prelude::v1::test]
    fn the_faint_tail_still_reads_as_a_mark() {
        // marks are at least 3:1 on the page, the snipped tail too, in every thread and light
        for p in [Palette::light(), Palette::dark()] {
            for &ink in crate::store::InkColor::ALL {
                let c = crate::theme::ink_color(ink, &p).opacity(TAIL);
                assert!(contrast_on(c, p.paper) >= 3., "{ink:?} on the {} page: {}", if p.dark { "dark" } else { "light" }, contrast_on(c, p.paper));
            }
        }
    }
}

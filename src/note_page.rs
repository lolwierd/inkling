//! One note on the page: the header row (mind, brief, mode), the editor, and the
//! margin where the mind's threads live. Also the reading loop, the selection
//! toolbar, Ask, Sketch it, and the chat panel.

use chrono::Utc;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use gpui_kit::prelude::*;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
    time::Duration,
};

use crate::brain::{Brain, BrainMsg};
use crate::drawing::{self, Playback};
use crate::editor::{Anchor, AnchorStyle, EditorEvent, GutterMark, NoteEditor};
use crate::margin::{self, Heights};
use crate::store::{
    Block, BlockKind, ChatMessage, Mark, Message, Mind, Mode, Note, Pace, Placement, Role, Side, Store,
    Thread, ThoughtKind, ThreadStatus, hash_text, new_id, plain_text_of,
};
use crate::stitch::{self, Rope};
use crate::theme::{SANS, TEXT_COLORS, Theme, ts};
use crate::ui::{self, Keyed, icon};

mod show_working;
mod goto;
/// Threads between notes: the reading loop's look at other notes after a read.
mod between;
/// Threads between notes, as the page shows and follows them.
mod ends;
mod read_through;
pub(crate) mod own;
mod answers;
mod write_in;

const CARD_GAP: f32 = 14.;
pub(crate) const CHAT_WIDTH: f32 = 310.;
/// The band of cloth along the top of the window that holds the traffic lights, ☰ and
/// Chat. The page scrolls under its edge, never under the controls.
pub(crate) const CHROME_H: f32 = 40.;
/// The sheet's top sits this far down the window, and its bottom this far up (when the
/// note is short, or scrolled to its end).
const SHEET_TOP: f32 = 44.;
const SHEET_BOTTOM: f32 = 12.;
/// The cloth shows around the page: this much page beyond the margins on each side.
const SHEET_PAD: f32 = 28.;
/// The narrowest margin: room for a folded tab.
const NARROW_MARGIN: f32 = 60.;
/// With no margin on the left, the column keeps this much more paper beside it, so the
/// gutter's marks (drawn 26 px left of the words, about 5 px either side) sit on the page
/// and not on the sheet's edge.
const GUTTER_LEAD: f32 = 14.;
/// An open tag is at least this wide, however slim the margin it hangs from.
const PANEL_MIN: f32 = 200.;
/// A tied-off tag stays this long, so you can watch the thread pull taut and knot.
const TIE_SHOW: Duration = Duration::from_millis(1700);
/// A snipped thread falls and fades over this long.
const SNIP_FADE: Duration = Duration::from_millis(900);
/// How long the open tag's Copy shows its tick.
const COPIED_FOR: Duration = Duration::from_millis(1500);
/// A question that arrives while you type waits, folded, until you pause this long.
const QUIET_WHILE_TYPING: Duration = Duration::from_millis(2000);
/// The most minds that can answer one question side by side.
const ROOM_MAX: usize = 3;
/// A folded tag: just its tab.
const TAB_W: f32 = 54.;
const TAB_H: f32 = 26.;
/// A left-margin thread comes up this far left of the column: past the gutter's knot and
/// number (the editor draws them 26 px out, about 6 px either side).
const GUTTER_CLEAR: f32 = 36.;
/// ...and its tab hangs this much further out than a right tab hangs from the column, so
/// the thread between them still shows.
const LEFT_TAB_OUT: f32 = 24.;
/// After the pointer leaves a tag, it stays open this long (moving onto its panel).
const FOLD_DELAY: Duration = Duration::from_millis(250);
/// How long the pointer rests on a tag before it unfolds, so passing over the margin
/// opens nothing.
const HOVER_REST: Duration = Duration::from_millis(120);
/// Drop: a tag opens on a spring that overshoots about 4% and settles (stiffness,
/// damping), and folds on a stiffer one that does not bounce.
const DROP_OPEN: (f32, f32) = (300., 25.);
const DROP_CLOSE: (f32, f32) = (700., 53.);


/// A tag being moved around the margin.
#[derive(Clone, Debug)]
struct TagDrag {
    id: String,
    start: Point<Pixels>,
    origin: (f32, f32),
    moved: bool,
    /// Where the tab sat in the window when the drag began, to keep it on screen.
    tab: Option<Bounds<Pixels>>,
}

/// A thread's colour. The note's own mind wears the thread you chose; every other mind
/// has its own, the same one every time, so you can see at a glance who asked what.
pub(crate) fn mind_thread(mind_name: &str, own: &str, chosen: crate::store::InkColor, t: &Theme) -> Hsla {
    use crate::store::InkColor as C;
    if mind_name.is_empty() || mind_name == own {
        return t.voice_ink;
    }
    let known = match mind_name {
        "Inkling" => Some(C::Ink),
        "Skeptic" => Some(C::Blue),
        "Socratic tutor" | "Tutor" => Some(C::Orange),
        "Editor" => Some(C::Green),
        "Staff engineer" => Some(C::Purple),
        "Investor" => Some(C::Brown),
        _ => None,
    };
    // A known mind whose thread the note's own mind is wearing takes rose, the one colour
    // no known mind owns, so it never lands on another known mind's colour.
    let known = known.map(|c| if c == chosen { C::Red } else { c });
    let pick = known.filter(|c| *c != chosen).unwrap_or_else(|| {
        let others: Vec<C> = [C::Blue, C::Orange, C::Green, C::Purple, C::Brown, C::Red, C::Ink].into_iter().filter(|c| *c != chosen).collect();
        others[(stitch::seed_of(mind_name) % others.len() as u64) as usize]
    });
    crate::theme::ink_color(pick, &t.p)
}

/// What Marks only puts after a thread's words: a ? for a question, the × its tab shows
/// for a correction, and its knot while it is being tied off.
fn mark_glyph(th: &Thread) -> &'static str {
    match (th.status, th.kind) {
        (ThreadStatus::Resolved, _) => "\u{2022}",
        (_, ThoughtKind::Correction) => "\u{d7}",
        _ => "?",
    }
}

/// `text` as a title of at most `max` characters: whole words, then an ellipsis.
pub(crate) fn short_title(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    let words = match cut.rfind(char::is_whitespace) {
        Some(at) if at > max / 2 => &cut[..at],
        _ => cut.as_str(),
    };
    format!("{}\u{2026}", words.trim_end_matches(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == ':'))
}

/// `INKLING_SLOWMO=<factor>` slows every thread animation down, for screenshots and the
/// walkthrough video. 1 is real time.
/// The narrowest an unfolded tag gets when its side is short of room; its words wrap
/// taller instead.
const PANEL_FLOOR: f32 = 160.;

/// How wide an unfolded tag is and which way it opens. A thread never covers your words
/// unless you dragged its tag onto them (the owner's rule), so a tag opens outward, away
/// from the column: right from a right tab, left from a left one. Short of room, in this
/// order: it hangs over the sheet's edge onto the cloth, as far as 8 px inside the window
/// (`out` is the room to there, paper included); then it narrows, down to
/// [`PANEL_FLOOR`]. Only a tag you dragged opens back across the words (`back`, the room
/// on the paper that way), and only when not even the narrowest fits outward.
fn panel_fit(right: bool, width: f32, out: f32, back: f32, dragged: bool) -> (f32, bool) {
    if out >= width {
        (width, right)
    } else if out >= PANEL_FLOOR {
        (out, right)
    } else if dragged && back > out {
        (width.min(back).max(PANEL_FLOOR), !right)
    } else {
        (PANEL_FLOOR, right)
    }
}

/// A tooltip for a key in the selection menu: what it makes, and its keys if it has
/// any, as the keymap has them now (Settings › Shortcuts can change them).
fn key_tip(name: &'static str, keys: Option<&'static dyn Action>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |window, cx| {
        let tip = gpui_kit::component::tooltip::Tooltip::new(name);
        match keys {
            Some(a) => tip.action(a, Some(crate::editor::CONTEXT)),
            None => tip,
        }
        .build(window, cx)
    }
}

pub(crate) fn slowmo() -> f32 {
    static F: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *F.get_or_init(|| std::env::var("INKLING_SLOWMO").ok().and_then(|v| v.parse::<f32>().ok()).filter(|f| *f >= 1.).unwrap_or(1.))
}

/// Seconds since `at`, in animation time.
fn anim_since(at: std::time::Instant, now: std::time::Instant) -> f32 {
    now.duration_since(at).as_secs_f32() / slowmo()
}

/// The knot pops in just after the thread pulls straight: a small overshoot, then rest.
fn knot_scale(t: f32) -> f32 {
    if t <= 0. {
        return 0.;
    }
    let k = (t / 0.2).min(1.);
    if k < 0.6 { k / 0.6 * 1.3 } else { 1.3 - (k - 0.6) / 0.4 * 0.3 }
}

/// A tied-off tag stays long enough to watch, then fades out of the margin.
fn tag_fade(since: Option<f32>) -> f32 {
    match since {
        None => 1.,
        Some(s) => ((TIE_SHOW.as_secs_f32() - s) / 0.45).clamp(0., 1.),
    }
}
/// How long a request may run before the page stops waiting for it. A read
/// holds the one read-along slot, so it gets less rope than a reply or a sketch.
const READ_TIMEOUT: Duration = Duration::from_secs(120);
const TASK_TIMEOUT: Duration = Duration::from_secs(600);
/// Lines kept for a second look (left behind by the caret, or finished while a
/// read was in flight).
const MAX_WAITING: usize = 16;

/// Page geometry for the current window width: the writing column in the
/// middle and a margin on each side, sized about 4:2 column to
/// margin, shrinking together on narrow windows). When even the narrowest
/// pair of margins does not fit, the left one goes and every card sits on the
/// right; narrower still (a small window with the chat open), the margin slims to
/// a strip for the folded tabs, so the page never runs off the window or under
/// the chat.
#[derive(Clone, Copy, Debug)]
struct Metrics {
    column: f32,
    margin: f32,
    gap: f32,
    /// A margin on each side; false means one margin, on the right.
    both: bool,
}

impl Metrics {
    /// The writing column takes about half the window, up to a readable 760 px (about
    /// 75 characters of Fraunces at 17 px); tags sit folded, so the margins can be slim.
    fn for_width(available: f32) -> Self {
        let gap = 36.;
        let avail = (available - 48.).max(0.);
        let column = (avail * 0.52).clamp(420., 760.);
        let margin = ((avail - column - 2. * gap) / 2.).clamp(200., 280.);
        if column + 2. * (margin + gap) <= avail {
            return Self { column, margin, gap, both: true };
        }
        let margin = ((avail - gap) * 0.32).clamp(200., 280.);
        let column = (avail - gap - margin - GUTTER_LEAD).clamp(300., 760.);
        if GUTTER_LEAD + column + gap + margin <= avail {
            return Self { column, margin, gap, both: false };
        }
        // Narrower still: first the margin slims to a strip that holds the folded tabs (an
        // open tag is drawn over the page), then the gap tightens, then the sheet takes
        // most of the cloth kept around it, and last the column gives way.
        let margin = (avail - gap - 300. - GUTTER_LEAD).clamp(NARROW_MARGIN, 200.);
        let gap = (avail - margin - 300. - GUTTER_LEAD).clamp(24., 36.);
        // (the gutter's room comes out of the cloth, not out of the words)
        let room = avail.max((300. + gap + margin + GUTTER_LEAD).min(available - 12. + GUTTER_LEAD));
        Self { column: (room - gap - margin - GUTTER_LEAD).max(160.), margin, gap, both: false }
    }
    /// Where a popover of `width` hangs from the page's left: in the margin when it fits
    /// there, else as far right as the sheet allows, over the column.
    fn popover_left(&self, width: f32) -> f32 {
        self.right_left().min(self.page() + SHEET_PAD - width).max(0.)
    }
    fn page(&self) -> f32 {
        if self.both { self.margin * 2. + self.gap * 2. + self.column } else { GUTTER_LEAD + self.column + self.gap + self.margin }
    }
    fn column_left(&self) -> f32 {
        if self.both { self.margin + self.gap } else { GUTTER_LEAD }
    }
    fn right_left(&self) -> f32 {
        self.column_left() + self.column + self.gap
    }
}

/// Sentence-ending marks: a paragraph that ends with one is finished.
const END_MARKS: [char; 7] = ['.', '?', '!', ':', ')', '"', '”'];

/// What a pause does with one paragraph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    /// Nothing to do: empty, already read, too short once left, or not read in this mode.
    Skip,
    /// Not yet: still being written, or another read is in flight.
    Later,
    /// It calls the mind by name: answer it.
    Answer,
    Read,
}

/// The reading loop's rule for one paragraph (`text` already trimmed). A
/// paragraph is finished when it ends a sentence or the caret has moved on;
/// read-along waits for at least four words, and one read at a time.
fn judge(text: &str, under_caret: bool, already_read: bool, calls: bool, reads_along: bool, busy: bool) -> Verdict {
    if text.is_empty() || already_read {
        return Verdict::Skip;
    }
    let words = text.split_whitespace().count();
    let finished = text.ends_with(END_MARKS) || !under_caret;
    if calls {
        // Wait for the whole question, not the first pause inside it.
        return match (finished, words >= 2) {
            (false, _) => Verdict::Later,
            (true, true) => Verdict::Answer,
            (true, false) => Verdict::Skip,
        };
    }
    if !reads_along {
        return Verdict::Skip;
    }
    if words < 4 {
        return if under_caret { Verdict::Later } else { Verdict::Skip };
    }
    if !finished || busy {
        return Verdict::Later;
    }
    Verdict::Read
}

/// Does `text` call the mind named `names` (lowercase): "Inkling, …",
/// "Inkling: …", "Hey Inkling …", or "@Inkling" anywhere as its own word.
fn calls_by_name(text: &str, names: &[String]) -> bool {
    let lower = text.to_lowercase();
    let ends_word = |rest: &str| rest.chars().next().is_none_or(|c| !c.is_alphanumeric());
    names.iter().filter(|n| !n.trim().is_empty()).any(|n| {
        let n = n.trim();
        lower.starts_with(&format!("{n},"))
            || lower.starts_with(&format!("{n}:"))
            || lower.strip_prefix(&format!("hey {n}")).is_some_and(ends_word)
            || lower.match_indices(&format!("@{n}")).any(|(i, m)| {
                let before = lower[..i].chars().next_back();
                before.is_none_or(|c| !c.is_alphanumeric() && c != '.' && c != '_') && ends_word(&lower[i + m.len()..])
            })
    })
}

/// The occurrence of `needle` in `hay` that starts closest to `near`.
fn nearest_match(hay: &str, needle: &str, near: usize) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    hay.match_indices(needle).map(|(i, _)| i).min_by_key(|i| i.abs_diff(near))
}

/// Byte range of each line of the editor's text (lines joined by '\n').
fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut start = 0;
    text.split('\n')
        .map(|l| {
            let r = start..start + l.len();
            start = r.end + 1;
            r
        })
        .collect()
}

fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Where a thought about `target` (the paragraph that was read, which was
/// line `hint` when it was sent) should anchor, quoting `quote`. The paragraph
/// is found first, nearest to where it was, so a quote that also appears
/// elsewhere in the note lands in the right place; if the model's quote is not
/// in it, the whole paragraph is the anchor.
fn locate_thought(text: &str, lines: &[Range<usize>], hint: usize, target: &str, quote: &str) -> Option<Range<usize>> {
    let target = target.trim();
    let within = |r: Range<usize>| -> Option<Range<usize>> {
        let s = &text[r.clone()];
        let i = s.find(quote).filter(|_| !quote.is_empty())?;
        Some(r.start + i..r.start + i + quote.len())
    };
    if !target.is_empty() {
        let found = lines
            .iter()
            .enumerate()
            .filter_map(|(ix, lr)| text[lr.clone()].find(target).map(|i| (ix, lr.start + i)))
            .min_by_key(|(ix, _)| ix.abs_diff(hint));
        if let Some((ix, at)) = found {
            let span = at..at + target.len();
            return within(span.clone()).or_else(|| within(lines[ix].clone())).or(Some(span));
        }
    }
    // The paragraph changed while it was being read: settle for the quote.
    let hint_line = lines.get(hint).cloned();
    hint_line
        .clone()
        .and_then(within)
        .or_else(|| nearest_match(text, quote, hint_line.map_or(0, |r| r.start)).map(|i| i..i + quote.len()))
}

#[derive(Clone, Debug)]
pub enum PageEvent {
    /// Something listed in the sidebar changed (title, a new note).
    NotesChanged,
    OpenNote(String),
    OpenSettings(crate::settings::Tab),
    /// The about-you paragraph picked up something new.
    Learned,
    /// A short confirmation for the window's toast ("Chat copied").
    Toast(String),
}

impl EventEmitter<PageEvent> for NotePage {}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Menu {
    Mind,
    Mode,
    Thread,
}

#[derive(Clone, Debug)]
enum Pending {
    Thinking,
    Streaming(String),
    Sketching,
}

struct Asking {
    range: Range<usize>,
    input: Entity<InputState>,
    /// The minds that will answer, side by side (Ask the room); at least one.
    room: Vec<Mind>,
    _sub: Subscription,
}

/// The read in flight: its request, the line it reads, and that line's hash.
struct Reading {
    req: String,
    line: usize,
    hash: u64,
}

pub struct NotePage {
    pub editor: Entity<NoteEditor>,
    pub note: Note,
    mind: Mind,
    scroll: ScrollHandle,
    active: Option<String>,
    replies: HashMap<String, Entity<InputState>>,
    reply_subs: HashMap<String, Subscription>,
    pending: HashMap<String, (String, Pending)>,
    playbacks: HashMap<String, Playback>,
    heights: Heights,
    last_edited: Option<usize>,
    /// Paragraphs to look at again at the next pause or when the read in flight ends.
    waiting: Vec<usize>,
    /// The editor's line count at the last edit, to follow lines that move.
    lines_seen: usize,
    reading: Option<Reading>,
    pause_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    show_toolbar: bool,
    asking: Option<Asking>,
    menu: Option<Menu>,
    /// The menu a press outside it just closed, and when: a press on its own button
    /// closes it rather than opening it again.
    menu_closed: Option<(Menu, std::time::Instant)>,
    /// An open menu has the keyboard: its card's focus, the item the arrow keys are on,
    /// and where the keyboard goes back to when it closes.
    menu_focus: FocusHandle,
    menu_pick: usize,
    menu_return: Option<FocusHandle>,
    thread_menu_for: Option<String>,
    brief_input: Option<(Entity<InputState>, Subscription)>,
    pub chat_open: bool,
    chat_input: Entity<InputState>,
    /// The chat's scroll position: it follows the conversation to its newest line.
    chat_scroll: ScrollHandle,
    /// Why the last chat question got no answer, shown under it with Try again.
    chat_error: Option<String>,
    /// Threads whose question or reply got no answer: the request to send again, and why,
    /// shown in the tag under your words with Try again, as the chat does.
    unanswered: HashMap<String, (Value, String)>,
    chat_pending: Option<(String, String)>,
    pub just_my_words: bool,
    /// How present the window's controls are, 0..1: the app fades them while you type
    /// and brings them back when the pointer moves. The page's header row follows it.
    pub chrome: Rc<std::cell::Cell<f32>>,
    enlarged: Option<String>,
    /// The back of the note (Turn over): every thread it has had, and how each ended.
    pub(crate) back: crate::back::Back,
    /// The thread whose Copy was just pressed, and when: it shows a tick for a moment.
    copied: Option<(String, std::time::Instant)>,
    marks_open: HashSet<String>,
    m: Metrics,
    /// Each visible thread's rope, kept between frames so it can sway and settle.
    ropes: HashMap<String, Rope>,
    /// Threads being tied off right now, and when it started.
    tying: HashMap<String, std::time::Instant>,
    /// Cut threads still falling: the rope, its colour, when it was cut.
    snipped: Vec<(Rope, Hsla, std::time::Instant)>,
    /// Open threads numbered in reading order (the gutter and the tags agree).
    numbers: HashMap<String, usize>,
    /// The tag under the pointer: its thread comes up a little from the background.
    /// The last thread dismissed, its words and when: Undo puts it back for a while.
    last_dismissed: Option<(String, Range<usize>, std::time::Instant)>,
    /// The last keystroke in the note.
    last_typed: Option<std::time::Instant>,
    /// Skip the next scroll to the caret (opening the welcome guide at its top).
    hold_scroll: bool,
    /// Questions that arrived while you were typing: they wait, folded, until you pause.
    arrived: HashMap<String, std::time::Instant>,
    /// Open threads whose words you edited, with the words as they were before: looked
    /// at again at the next pause. (The thread's own quote follows the words on save.)
    rechecks: HashMap<String, (std::time::Instant, String)>,
    /// Threads being looked at again right now (the needle works beside them).
    rechecking: HashSet<String>,
    settle_task: Option<Task<()>>,
    /// Minds a script seats for its next ask.
    pending_room: Vec<String>,
    /// How far each tag is unfolded (0 folded, 1 open), eased toward its target each frame.
    unfold: HashMap<String, f32>,
    /// Tags under the pointer, and when the pointer last left each.
    /// Which parts of which tags the pointer is over: "<id>:tab" or "<id>:panel".
    hover_tag: HashSet<String>,
    hover_left: HashMap<String, std::time::Instant>,
    /// When the pointer came onto each tag it is over (any part of it).
    hover_at: HashMap<String, std::time::Instant>,
    /// Each tag's unfold speed, for its spring.
    unfold_v: HashMap<String, f32>,
    /// A tag being dragged, and when the last drag ended (so its click is ignored).
    drag: Option<TagDrag>,
    last_drag_end: Option<std::time::Instant>,
    /// Natural height of each unfolded tag, measured, to unfold toward.
    panel_heights: Heights,
    /// Each tab's bounds in the window at the last paint: where its panel has room to open.
    tab_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    /// The sheet's bounds in the window at the last paint: tags are kept on the paper.
    sheet_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Each unfolded tag's bounds at the last paint, and the column of your words, for
    /// the dump: no tag you have not dragged may cover the column.
    panel_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    column_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    viewport: Size<Pixels>,
    last_frame: Option<std::time::Instant>,
    _subs: Vec<Subscription>,
    /// Threads whose next reply says the working does not hold (Disagree).
    disputes: HashSet<String>,
    /// A thread gone to from Loose ends, whose words are brought into view once the note
    /// is laid out (`goto.rs`).
    goto: Option<goto::Goto>,
    /// The look at other notes in flight (threads between notes), one at a time, and the
    /// newest paragraph read meanwhile, which waits for it.
    relating: Option<Reading>,
    relate_next: Option<(usize, String)>,
    /// Threads between notes: the thread being followed into the next note to open.
    ends: ends::Ends,
    /// Read it through: the pass running on this note, if any (`read_through.rs`).
    read_through: Option<read_through::Pass>,
    /// Your own threads a mind is answering in now, and which mind (`own::call_into_own`).
    own_calls: HashMap<String, String>,
    /// Paragraphs a note that waits to be asked has read for answers only (by hash), so
    /// each version is checked once (`answers.rs`).
    answer_checks: HashSet<u64>,
    /// The reply written in last (Write it in), while the toast's Undo can take it out.
    written_in: Option<write_in::WrittenIn>,
}

fn role_wire(r: Role) -> &'static str {
    match r {
        Role::You => "you",
        Role::Mind => "mind",
    }
}

impl NotePage {
    pub fn new(note: Note, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(NoteEditor::new);
        let chat_input = cx.new(|cx| InputState::new(window, cx).placeholder("Ask about this note…"));
        let mut subs = vec![];
        subs.push(cx.subscribe_in(&editor, window, Self::on_editor_event));
        subs.push(cx.subscribe_in(&chat_input, window, |this, _, ev: &InputEvent, window, cx| match ev {
            InputEvent::PressEnter { .. } => this.send_chat(None, window, cx),
            // the send button lights up once there is something to send
            InputEvent::Change => cx.notify(),
            _ => {}
        }));
        // ⌘Q right after typing would otherwise beat the save debounce.
        subs.push(cx.on_app_quit(|page, cx| {
            page.flush(cx);
            async {}
        }));
        let mind = cx.global::<Store>().mind(&note.mind_id);
        let mut page = Self {
            editor,
            note,
            mind,
            scroll: ScrollHandle::new(),
            active: None,
            replies: HashMap::new(),
            reply_subs: HashMap::new(),
            pending: HashMap::new(),
            playbacks: HashMap::new(),
            heights: Rc::new(RefCell::new(HashMap::new())),
            last_edited: None,
            waiting: vec![],
            lines_seen: 0,
            reading: None,
            pause_task: None,
            save_task: None,
            show_toolbar: false,
            asking: None,
            menu: None,
            menu_closed: None,
            menu_focus: cx.focus_handle(),
            menu_pick: 0,
            menu_return: None,
            thread_menu_for: None,
            brief_input: None,
            chat_open: false,
            chat_input,
            chat_scroll: ScrollHandle::new(),
            chat_error: None,
            unanswered: HashMap::new(),
            chat_pending: None,
            just_my_words: false,
            enlarged: None,
            back: crate::back::Back::new(cx),
            copied: None,
            marks_open: HashSet::new(),
            m: Metrics::for_width(1320.),
            ropes: HashMap::new(),
            tying: HashMap::new(),
            snipped: vec![],
            numbers: HashMap::new(),
            last_dismissed: None,
            last_typed: None,
            hold_scroll: false,
            chrome: Rc::new(std::cell::Cell::new(1.)),
            arrived: HashMap::new(),
            rechecks: HashMap::new(),
            rechecking: HashSet::new(),
            settle_task: None,
            pending_room: vec![],
            unfold: HashMap::new(),
            hover_tag: HashSet::new(),
            hover_left: HashMap::new(),
            hover_at: HashMap::new(),
            unfold_v: HashMap::new(),
            drag: None,
            last_drag_end: None,
            panel_heights: Rc::new(RefCell::new(HashMap::new())),
            tab_bounds: Rc::new(RefCell::new(HashMap::new())),
            sheet_bounds: Rc::new(Cell::new(None)),
            panel_bounds: Rc::new(RefCell::new(HashMap::new())),
            column_bounds: Rc::new(Cell::new(None)),
            viewport: size(px(1200.), px(800.)),
            last_frame: None,
            _subs: subs,
            disputes: HashSet::new(),
            goto: None,
            relating: None,
            relate_next: None,
            ends: Default::default(),
            read_through: None,
            own_calls: HashMap::new(),
            answer_checks: HashSet::new(),
            written_in: None,
        };
        page.load_into_editor(cx);
        page
    }

    pub fn note_id(&self) -> &str {
        &self.note.id
    }

    fn load_into_editor(&mut self, cx: &mut Context<Self>) {
        crate::between::add_visitors(&mut self.note, &cx.global::<Store>().notes);
        let lines = self.note.blocks.iter().map(|b| (b.kind, b.text.clone())).collect();
        let text = self.note.joined_text();
        let anchors = self
            .note
            .threads
            .iter_mut()
            .filter(|t| t.status != ThreadStatus::Dismissed)
            .map(|t| {
                // Re-find the quote if the stored offsets no longer match it,
                // taking the occurrence nearest to where it was.
                let ok = text.get(t.start..t.end).is_some_and(|s| s == t.quote);
                if !ok {
                    if let Some(i) = nearest_match(&text, &t.quote, t.start) {
                        t.start = i;
                        t.end = i + t.quote.len();
                    } else {
                        t.start = floor_char_boundary(&text, t.start);
                        t.end = t.start;
                    }
                }
                Anchor { id: t.id.clone(), range: t.start..t.end, open: t.status == ThreadStatus::Open }
            })
            .collect::<Vec<_>>();
        let anchors = [anchors, answers::load_answers(&mut self.note, &text)].concat();
        let marks = self.note.marks.clone();
        self.playbacks.clear();
        for t in &self.note.threads {
            if let Some(svg) = &t.drawing {
                if let Some(d) = drawing::parse(svg) {
                    self.playbacks.insert(t.id.clone(), Playback::new(d, false));
                }
            }
        }
        self.editor.update(cx, |e, cx| e.load(lines, marks, anchors, cx));
        self.lines_seen = self.editor.read(cx).line_count();
    }

    /// Switch the page to another note, saving this one first.
    pub fn open(&mut self, note: Note, window: &mut Window, cx: &mut Context<Self>) {
        self.leave_read_through(cx);
        // The read in flight will never land: leave its paragraph unread.
        if let Some(r) = self.reading.take() {
            self.note.read_hashes.retain(|h| *h != r.hash);
            self.cancel_request(&r.req, cx);
        }
        self.relate_next = None;
        if let Some(r) = self.relating.take() {
            self.cancel_request(&r.req, cx);
        }
        // Threads still waiting for their first words (a loose thread, a sketch, a
        // call by name) would be saved as empty cards: drop them with their requests.
        for (id, (req, _)) in std::mem::take(&mut self.pending) {
            self.cancel_request(&req, cx);
            self.drop_if_empty(&id, cx);
        }
        self.own_calls.clear();
        self.flush(cx);
        if let Some((req, _)) = self.chat_pending.take() {
            self.cancel_request(&req, cx);
        }
        // A pause timer started on this note must not read the next one.
        self.pause_task = None;
        self.last_edited = None;
        self.waiting.clear();
        self.chat_error = None;
        // as the chat's failure line, a tag's goes with the note it was on
        self.unanswered.clear();
        self.mind = cx.global::<Store>().mind(&note.mind_id);
        self.note = note;
        self.active = None;
        self.replies.clear();
        self.reply_subs.clear();
        self.asking = None;
        self.menu = None;
        self.brief_input = None;
        self.show_toolbar = false;
        self.enlarged = None;
        // another note opens on its front
        self.back.reset();
        self.heights.borrow_mut().clear();
        self.marks_open.clear();
        self.load_into_editor(cx);
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.focus_editor_at_end(window, cx);
        self.arrive(window, cx);
        cx.notify();
    }

    pub fn focus_editor_at_end(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let empty_title = self.editor.read(cx).title().is_empty();
        // A note opens at its top, title and header in view, however long it is; its
        // caret waits at the end, and the page follows it once you type or move it.
        self.hold_scroll = true;
        self.editor.update(cx, |e, cx| {
            let target = if empty_title { 0 } else { e.text().len() };
            e.move_cursor_to(target, cx);
            e.focus(window, cx);
        });
    }

    // ----- saving ------------------------------------------------------------

    fn sync_from_editor(&mut self, cx: &App) {
        let ed = self.editor.read(cx);
        let lines = ed.lines();
        self.note.blocks = lines.into_iter().map(|(kind, text)| Block { kind, text }).collect();
        self.note.title = self.note.blocks.first().map(|b| b.text.clone()).unwrap_or_default();
        self.note.marks = ed.marks().to_vec();
        let text = ed.text();
        for a in ed.anchors() {
            if let Some(t) = self.note.threads.iter_mut().find(|t| t.id == a.id) {
                t.start = a.range.start;
                t.end = a.range.end;
                if !a.range.is_empty() && a.range.end <= text.len() {
                    t.set_quote(&text[a.range.clone()]);
                }
            }
        }
        answers::sync_answers(&mut self.note, text, ed.anchors());
    }

    pub fn flush(&mut self, cx: &mut Context<Self>) {
        self.save_task = None;
        self.sync_from_editor(cx);
        // Leaving a note you only looked at keeps the day it was last edited.
        let kept = {
            let clean = crate::between::without_visitors(&self.note);
            cx.global::<Store>().note(&clean.id).filter(|n| n.same_writing(&clean)).map(|n| n.updated_at)
        };
        self.note.updated_at = kept.unwrap_or_else(Utc::now);
        let note = crate::between::save_visitors_home(&self.note, cx.global_mut::<Store>());
        let store = cx.global_mut::<Store>();
        match store.notes.iter_mut().find(|n| n.id == note.id) {
            Some(n) => *n = note.clone(),
            None => store.notes.insert(0, note.clone()),
        }
        store.save_note(&note);
        store.settings.last_note = Some(note.id.clone());
        store.save_settings();
        cx.emit(PageEvent::NotesChanged);
    }

    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(600)).await;
            let _ = this.update(cx, |page, cx| page.flush(cx));
        }));
    }

    // ----- editor events and the reading loop ---------------------------------

    fn on_editor_event(&mut self, _: &Entity<NoteEditor>, ev: &EditorEvent, window: &mut Window, cx: &mut Context<Self>) {
        match ev {
            EditorEvent::Edited { line } => {
                self.last_typed = Some(std::time::Instant::now());
                self.note_rechecks(*line, cx);
                // questions that arrived while you typed open once you pause
                self.settle_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(QUIET_WHILE_TYPING + Duration::from_millis(60)).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                }));
                self.note_edit(*line, cx);
                self.show_toolbar = false;
                cx.notify();
                self.schedule_save(cx);
                self.schedule_pause(cx);
                if *line == 0 {
                    cx.emit(PageEvent::NotesChanged);
                }
            }
            EditorEvent::SelectionChanged => {
                if self.editor.read(cx).selection().is_empty() {
                    self.show_toolbar = false;
                }
                self.keep_cursor_visible(window, cx);
                cx.notify();
            }
            EditorEvent::SelectionSettled => {
                self.show_toolbar = !self.editor.read(cx).selection().is_empty();
                cx.notify();
            }
            EditorEvent::Escape => {
                self.escape(window, cx);
            }
            EditorEvent::AnchorClicked(id) => {
                let id = id.clone();
                self.show_toolbar = false;
                self.activate(Some(id.clone()), window, cx);
                if let Some(input) = self.replies.get(&id).cloned() {
                    input.update(cx, |i, cx| i.focus(window, cx));
                }
            }
            EditorEvent::LayoutChanged => cx.notify(),
            EditorEvent::Focused | EditorEvent::Blurred => {}
        }
    }

    /// Escape closes the innermost thing the page has open, one per press: the sketch
    /// seen larger, a menu, the question box, the brief being edited, the open thread,
    /// the selection toolbar, and last the chat when its box has the keyboard. The
    /// keyboard goes back to the note. Returns whether anything closed.
    pub fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // On the back: the chat when its box has the keyboard, else the reason box, else the back.
        if self.back.shown {
            if self.chat_open && self.chat_input.read(cx).focus_handle(cx).is_focused(window) {
                self.chat_open = false;
                self.back.focus.focus(window, cx);
                cx.notify();
            } else {
                crate::back::escape(self, window, cx);
            }
            return true;
        }
        // a tag that only unfolded under the pointer folds first
        let hover_open = self.hover_at.keys().chain(self.hover_left.keys()).any(|id| {
            self.active.as_deref() != Some(id.as_str()) && self.unfold.get(id).copied().unwrap_or(0.) > 0.5
        });
        self.hover_at.clear();
        self.hover_left.clear();
        if hover_open {
            cx.notify();
            return true;
        }
        let menu_open = self.enlarged.is_none() && self.menu.is_some();
        let closed = if self.enlarged.take().is_some() || self.menu.take().is_some() || self.asking.take().is_some() || self.brief_input.take().is_some() {
            true
        } else if self.active.is_some() || !self.marks_open.is_empty() {
            self.close_thread(cx);
            true
        } else if self.show_toolbar {
            self.show_toolbar = false;
            true
        } else if self.chat_open && self.chat_input.read(cx).focus_handle(cx).is_focused(window) {
            self.chat_open = false;
            true
        } else {
            false
        };
        if closed {
            if menu_open && self.menu_focus.is_focused(window) {
                self.leave_menu(window, cx);
            } else if !self.editor.read(cx).is_focused(window) {
                self.editor.update(cx, |e, cx| e.focus(window, cx));
            }
            cx.notify();
        }
        closed
    }

    /// Fold the open thread back into the margin (and, in Marks only, put its tag away).
    fn close_thread(&mut self, cx: &mut Context<Self>) {
        self.active = None;
        self.marks_open.clear();
        self.editor.update(cx, |e, _| e.active_anchor = None);
        cx.notify();
    }

    /// Hide threads (or show them again). Hidden, nothing of Inkling's stays open: no
    /// thread, question box, toolbar or menu waits behind the clean page to be dismissed
    /// from the keyboard without being seen.
    pub fn set_hidden(&mut self, hidden: bool, cx: &mut Context<Self>) {
        self.just_my_words = hidden;
        if hidden {
            // hiding threads on the back turns the note to its front first
            self.leave_back(cx);
            self.menu = None;
            self.asking = None;
            self.brief_input = None;
            self.show_toolbar = false;
            self.enlarged = None;
            self.close_thread(cx);
        }
        cx.notify();
    }

    /// The keyboard has nowhere to be (the box that had it went away): back to the note,
    /// the caret where it was.
    pub fn restore_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.back.shown {
            self.back.focus.focus(window, cx);
            return;
        }
        self.editor.update(cx, |e, cx| e.focus(window, cx));
    }

    /// Turn over (⇧⌘B, View › Turn Over, the header's Turn over): the back of the note,
    /// every thread it has had and how each ended (`back.rs`). Again, and it turns back.
    /// What the front had open over the words (a menu, the question box, the toolbar, a
    /// sketch seen larger) closes; the open tag and the caret stay for the way back.
    pub fn turn_over(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.back.shown {
            return self.turn_back(window, cx);
        }
        self.sync_from_editor(cx);
        self.menu = None;
        self.asking = None;
        self.brief_input = None;
        self.show_toolbar = false;
        self.enlarged = None;
        self.back.show(self.scroll.offset());
        // the keyboard leaves the note, so nothing typed lands in it unseen
        self.back.focus.focus(window, cx);
        cx.notify();
    }

    /// Back to the front, scrolled as it was, with the caret where you left it.
    pub(crate) fn turn_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.leave_back(cx);
        self.editor.update(cx, |e, cx| e.focus(window, cx));
    }

    fn leave_back(&mut self, cx: &mut Context<Self>) {
        if let Some(at) = self.back.hide() {
            self.scroll.set_offset(at);
            cx.notify();
        }
    }

    /// From the back: turn to the front and open the thread's tag at its words, as ⌥⌘↓
    /// does (Show on the page); `reopen` first opens a thread that ended on its words again,
    /// found where they are now, its reason and date gone (Reopen).
    pub(crate) fn from_back(&mut self, id: &str, reopen: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.turn_back(window, cx);
        if self.just_my_words {
            self.set_hidden(false, cx);
        }
        let anchored = self.editor.read(cx).anchors().iter().any(|a| a.id == id);
        if reopen && !anchored {
            let text = self.editor.read(cx).text().to_string();
            let Some(range) = self.note.threads.iter().find(|t| t.id == id).and_then(|t| nearest_match(&text, &t.quote, t.start).map(|i| i..i + t.quote.len())) else { return };
            // its Undo, if it was the last one dismissed, would put it back a second time
            if self.last_dismissed.as_ref().is_some_and(|(d, _, _)| d == id) {
                self.last_dismissed = None;
                crate::toast::close(cx, crate::toast::Kind::ThreadDismissed);
            }
            self.editor.update(cx, |e, cx| e.add_anchor(Anchor { id: id.to_string(), range, open: true }, cx));
        }
        if reopen {
            self.set_status(id, ThreadStatus::Open, cx);
            // Let stand left its reason in the reply box (for Undo): the thread opens on an empty one
            self.replies.remove(id);
            self.reply_subs.remove(id);
        }
        self.show_thread(id, window, cx);
    }

    fn keep_cursor_visible(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.hold_scroll) {
            return;
        }
        let Some(rect) = self.editor.read(cx).cursor_window_rect() else { return };
        let view = window.viewport_size();
        let offset = self.scroll.offset();
        let margin = px(80.);
        if rect.bottom() > view.height - margin {
            self.scroll.set_offset(point(offset.x, offset.y - (rect.bottom() - (view.height - margin))));
        } else if rect.top() < px(70.) {
            self.scroll.set_offset(point(offset.x, (offset.y + (px(70.) - rect.top())).min(px(0.))));
        }
    }

    fn schedule_pause(&mut self, cx: &mut Context<Self>) {
        let delay = match cx.global::<Store>().settings.pace {
            Pace::Regular => Duration::from_millis(2200),
            Pace::Slow => Duration::from_millis(4600),
        };
        self.pause_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |page, cx| page.on_pause(cx));
        }));
    }

    fn calls_mind(&self, text: &str) -> bool {
        calls_by_name(text, &["inkling".to_string(), self.mind.name.to_lowercase()])
    }

    /// Track an edit at `line` for the reading loop.
    fn note_edit(&mut self, line: usize, cx: &mut Context<Self>) {
        // Lines below the edit moved when lines were added or merged: keep the
        // remembered ones on the paragraphs they meant.
        let count = self.editor.read(cx).line_count();
        let shift = count as isize - self.lines_seen as isize;
        self.lines_seen = count;
        if shift != 0 {
            let moved = |l: usize| if l > line { (l as isize + shift).max(line as isize) as usize } else { l };
            let mut waiting: Vec<usize> = vec![];
            for l in self.waiting.drain(..).map(moved) {
                if !waiting.contains(&l) {
                    waiting.push(l);
                }
            }
            self.waiting = waiting;
            self.last_edited = self.last_edited.map(moved);
            if let Some(r) = self.reading.as_mut() {
                r.line = moved(r.line);
            }
            self.shift_read_through(&moved);
        }
        // Writing moved on to another paragraph without a pause: the one left
        // behind may be finished, so keep it for the next look.
        if let Some(prev) = self.last_edited.filter(|p| *p != line) {
            if !self.waiting.contains(&prev) && self.waiting.len() < MAX_WAITING {
                self.waiting.push(prev);
            }
        }
        self.last_edited = Some(line);
    }

    fn on_pause(&mut self, cx: &mut Context<Self>) {
        let Some(edited) = self.last_edited.take() else { return };
        let cursor_line = {
            let ed = self.editor.read(cx);
            ed.line_of(ed.cursor())
        };
        // Look at the line being written now, the line that was edited, and the
        // ones kept from before: after Enter the finished paragraph is the
        // edited one; after a paste or a dictated sentence it is the one under
        // the caret; after writing straight on, it is the one left behind.
        let mut lines = vec![cursor_line, edited];
        lines.extend(std::mem::take(&mut self.waiting));
        // A paragraph whose question is being looked at again is not also read from
        // scratch: one look per edit.
        let due: Vec<(String, String)> = self.rechecks.drain().map(|(id, (_, before))| (id, before)).collect();
        let skip: Vec<usize> = {
            let ed = self.editor.read(cx);
            due.iter().filter_map(|(id, _)| ed.anchors().iter().find(|a| &a.id == id).map(|a| ed.line_of(a.range.start))).collect()
        };
        for line in &skip {
            let text = self.editor.read(cx).line_text(*line);
            let hash = hash_text(text.trim());
            if !self.note.read_hashes.contains(&hash) {
                self.note.read_hashes.push(hash);
            }
        }
        self.consider(lines, cx);
        for (id, before) in due {
            self.recheck(&id, before, cx);
        }
    }

    pub fn last_typed(&self) -> Option<std::time::Instant> {
        // nothing is written on the back of the note, so the controls are not kept out of the way there
        self.last_typed.filter(|_| !self.back.shown)
    }

    fn typing_recently(&self) -> bool {
        self.last_typed.is_some_and(|t| t.elapsed() < QUIET_WHILE_TYPING)
    }

    /// Nothing is scheduled or in flight, so the page will not change on its own: no
    /// pause pending, no read, no thread or chat request, not typing, no thread being gone
    /// to from Loose ends (its words scrolled in, its reply box given the keyboard). A script's `settle`
    /// waits for this. Learning about the writer runs untracked, and saves wait 600 ms
    /// (quitting flushes them); neither shows on the page.
    pub(crate) fn settled(&self) -> bool {
        self.last_edited.is_none() && self.reading.is_none() && self.pending.is_empty() && self.chat_pending.is_none() && !self.typing_recently() && self.goto.is_none()
            && self.relating.is_none() && self.relate_next.is_none()
            && self.read_through.is_none()
    }

    /// The words of an open thread were edited: look at them again at the next pause.
    fn note_rechecks(&mut self, line: usize, cx: &mut Context<Self>) {
        let ed = self.editor.read(cx);
        if line >= ed.line_count() {
            return;
        }
        let lr = ed.line_range(line);
        let text = ed.text().to_string();
        let touched: Vec<(String, Range<usize>)> = ed
            .anchors()
            .iter()
            .filter(|a| a.open && a.range.start >= lr.start && a.range.end <= lr.end)
            .map(|a| (a.id.clone(), a.range.clone()))
            .collect();
        for (id, range) in touched {
            let Some(th) = self.note.threads.iter().find(|t| t.id == id && t.status == ThreadStatus::Open && !t.own) else { continue };
            if text.get(range).is_some_and(|now| now != th.quote) {
                // keep the words from before the first edit of this run of typing
                let before = th.quote.clone();
                self.rechecks.entry(id).or_insert((std::time::Instant::now(), before));
            }
        }
    }

    /// Fix the words, the question settles: you edited the words a thread is about, so
    /// Inkling reads them again. If the edit answers it, the thread ties itself off with
    /// one line saying why; if not, it says what is still missing and stays open.
    fn recheck(&mut self, id: &str, before: String, cx: &mut Context<Self>) {
        if self.pending.contains_key(id) {
            return;
        }
        let Some(t) = self.note.threads.iter().find(|t| t.id == id && t.status == ThreadStatus::Open).cloned() else { return };
        let now_text = {
            let ed = self.editor.read(cx);
            let Some(range) = ed.anchors().iter().find(|a| a.id == id).map(|a| a.range.clone()) else { return };
            let Some(words) = ed.text().get(range).map(str::to_string) else { return };
            words
        };
        if now_text.trim().is_empty() || now_text == before {
            return;
        }
        let message = format!(
            "I changed these words to: \u{201c}{}\u{201d}. If that settles what you asked, say so in one short sentence and mark it resolved. If it does not, say in one sentence what is still missing.",
            now_text.trim()
        );
        let input = json!({
            "task": "reply",
            "ctx": self.context(cx),
            "quote": before,
            "messages": t.messages.iter().map(|m| json!({"role": role_wire(m.role), "text": crate::working::wire_text(m)})).collect::<Vec<_>>(),
            "message": message,
        });
        let input = self.between_reply(&t, input, cx);
        let tid = id.to_string();
        self.rechecking.insert(tid.clone());
        let req = self.run(input, false, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                page.rechecking.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if let Some(t) = page.thread_mut(&tid) {
                    t.set_quote(&now_text);
                }
                if !body.is_empty() {
                    page.push_message(&tid, Role::Mind, body, cx);
                }
                if v["resolved"].as_bool() == Some(true) {
                    if let Some(t) = page.thread_mut(&tid) {
                        t.tied_by = Some(crate::store::TiedBy::Edit);
                    }
                    page.set_status(&tid, ThreadStatus::Resolved, cx);
                }
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                page.rechecking.remove(&tid);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.pending.insert(id.to_string(), (req, Pending::Thinking));
        cx.notify();
    }

    /// When a read ends, look at the paragraphs that were finished meanwhile, then let Read
    /// it through take the next one.
    fn after_read(&mut self, cx: &mut Context<Self>) {
        if self.reading.is_none() && !self.waiting.is_empty() {
            let lines = std::mem::take(&mut self.waiting);
            self.consider(lines, cx);
        }
        self.step_through(cx);
    }

    fn consider(&mut self, lines: Vec<usize>, cx: &mut Context<Self>) {
        // A block of code is read whole, from its first line, once the caret has left it;
        // its lines are left alone when every one of them is.
        let (cursor_line, candidates) = {
            let ed = self.editor.read(cx);
            let count = ed.line_count();
            let cursor_unit = ed.unit_of(ed.line_of(ed.cursor()));
            let mut seen = vec![];
            let candidates: Vec<(usize, String, bool, bool)> = lines
                .into_iter()
                .filter(|l| *l > 0 && *l < count)
                .map(|l| ed.unit_of(l))
                .filter(|u| !seen.contains(&u.start) && {
                    seen.push(u.start);
                    true
                })
                .map(|u| {
                    let code = ed.line_kind(u.start).is_code();
                    let alone = code && u.clone().map(|l| ed.line_text(l)).filter(|t| !t.trim().is_empty()).all(|t| self.note.left_alone.contains(&hash_text(&t)));
                    (u.start, ed.lines_text(u.clone()).trim().to_string(), code && u.start == cursor_unit.start, alone)
                })
                .collect();
            (cursor_unit.start, candidates)
        };
        let mut marked = false;
        let waits = self.note.mode == Mode::WhenYouAsk;
        let listens = !waits || self.listens_for_answers();
        for (line, text, writing_code, alone) in candidates {
            let hash = hash_text(&text);
            let already = alone || self.note.read_hashes.contains(&hash) || self.note.left_alone.contains(&hash) || (waits && self.answer_checks.contains(&hash));
            let verdict = if writing_code && !already {
                Verdict::Later
            } else {
                judge(&text, line == cursor_line, already, self.calls_mind(&text), listens, self.reading.is_some())
            };
            match verdict {
                Verdict::Skip => {}
                Verdict::Later => {
                    if !self.waiting.contains(&line) && self.waiting.len() < MAX_WAITING {
                        self.waiting.push(line);
                    }
                }
                Verdict::Answer => {
                    self.note.read_hashes.push(hash);
                    marked = true;
                    self.answer_call(line, text, cx);
                }
                Verdict::Read if waits => self.read_for_answers(line, text, hash, cx),
                Verdict::Read => {
                    self.note.read_hashes.push(hash);
                    marked = true;
                    self.read_line(line, text, hash, cx);
                }
            }
        }
        // A silent read changes nothing else, so save the "already read" mark
        // itself; otherwise a relaunch would read the paragraph again.
        if marked {
            self.schedule_save(cx);
        }
    }

    fn context(&self, cx: &App) -> Value {
        self.context_with(&self.mind, cx)
    }

    /// The context as `mind` would read the note (Ask the room sends one per mind).
    fn context_with(&self, mind: &Mind, cx: &App) -> Value {
        let s = &cx.global::<Store>().settings;
        let ed = self.editor.read(cx);
        let blocks: Vec<Block> = ed.lines().into_iter().map(|(kind, text)| Block { kind, text }).collect();
        let threads: Vec<Value> = self
            .note
            .threads
            .iter()
            .filter(|t| t.status != ThreadStatus::Dismissed)
            .map(|t| {
                json!({
                    "id": t.id,
                    // words here can answer it: open, a mind's (not your own), and not visiting
                    // from another note (answered in its home note)
                    "open": crate::answers::answerable(t) && !self.note.elsewhere_of(&t.id).is_some_and(|e| e.visiting),
                    "notAnswers": t.not_answers,
                    "quote": t.quote,
                    "resolved": t.status == ThreadStatus::Resolved,
                    "messages": t.messages.iter().map(own::message_json).collect::<Vec<_>>(),
                    "elsewhere": crate::between::summary_wire(&self.note, &t.id),
                    "own": t.own,
                })
            })
            .collect();
        json!({
            "noteTitle": ed.title(),
            "noteText": plain_text_of(&blocks),
            "brief": self.note.brief,
            "mind": {
                "name": mind.name,
                "description": mind.description,
                "howItThinks": mind.how_it_thinks,
                "voice": mind.voice.wire(),
                "frequency": mind.frequency.wire(),
            },
            "aboutMe": if s.about_me.trim().is_empty() { Value::Null } else { Value::String(s.about_me.clone()) },
            "threads": threads,
            "now": chrono::Local::now().format("%A %-d %B %Y").to_string(),
            "letStand": crate::stand::points(&self.note.threads, ed.text()),
        })
    }

    /// Send a task to the brain; `on` runs on this page if it still shows the same note.
    fn run(
        &mut self,
        input: Value,
        heavy: bool,
        cx: &mut Context<Self>,
        on: impl FnMut(&mut NotePage, BrainMsg, &mut Context<NotePage>) + 'static,
    ) -> String {
        self.send_task(input, heavy, true, cx, on)
    }

    /// Like `run`, for answers about the writer rather than this note: they
    /// still count after the page moves to another note.
    fn run_any_note(
        &mut self,
        input: Value,
        heavy: bool,
        cx: &mut Context<Self>,
        on: impl FnMut(&mut NotePage, BrainMsg, &mut Context<NotePage>) + 'static,
    ) -> String {
        self.send_task(input, heavy, false, cx, on)
    }

    fn send_task(
        &mut self,
        input: Value,
        heavy: bool,
        this_note_only: bool,
        cx: &mut Context<Self>,
        on: impl FnMut(&mut NotePage, BrainMsg, &mut Context<NotePage>) + 'static,
    ) -> String {
        let (model, effort) = {
            let s = &cx.global::<Store>().settings;
            (s.model.clone(), if heavy { s.effort.clone() } else { s.effort_read.clone() })
        };
        let this = cx.entity().downgrade();
        let note_id = self.note.id.clone();
        let on = Rc::new(RefCell::new(on));
        let finished = Rc::new(Cell::new(false));
        let deliver = {
            let (this, finished) = (this.clone(), finished.clone());
            Rc::new(move |msg: BrainMsg, cx: &mut App| {
                if finished.get() {
                    return;
                }
                if !matches!(msg, BrainMsg::Delta(_)) {
                    finished.set(true);
                }
                let Some(this) = this.upgrade() else { return };
                this.update(cx, |page, cx| {
                    if !this_note_only || page.note.id == note_id {
                        (on.borrow_mut())(page, msg, cx);
                    }
                });
            })
        };
        // Brain::run reports a request it could not send (no Bun, a pipe to a
        // brain that just died) by calling back before it returns, while this
        // page is still being updated: updating it again from there panics, and
        // the callback would clear state the caller has not set yet. Hold such
        // messages for the next effect cycle.
        let sending = Rc::new(Cell::new(true));
        let req = {
            let (deliver, sending) = (deliver.clone(), sending.clone());
            Brain::global(cx).update(cx, |b, cx| {
                b.run(input, &model, &effort, cx, move |msg, cx| {
                    if sending.get() {
                        let deliver = deliver.clone();
                        cx.defer(move |cx| deliver(msg, cx));
                    } else {
                        deliver(msg, cx);
                    }
                })
            })
        };
        sending.set(false);
        // A brain that never answers would hold the read-along slot or a
        // thread's thinking dot forever: give up on the request after a while.
        let timeout = if heavy { TASK_TIMEOUT } else { READ_TIMEOUT };
        let req_id = req.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timeout).await;
            if finished.get() {
                return;
            }
            let _ = this.update(cx, |page, cx| {
                if page.tracks(&req_id) {
                    log::warn!("request {req_id} timed out after {}s", timeout.as_secs());
                    page.cancel_request(&req_id, cx);
                    let deliver = deliver.clone();
                    cx.defer(move |cx| {
                        deliver(BrainMsg::Error { kind: "timeout".into(), message: "the model took too long to answer".into() }, cx)
                    });
                }
            });
        })
        .detach();
        req
    }

    /// Is `req` still something this page is waiting on?
    fn tracks(&self, req: &str) -> bool {
        self.reading.as_ref().is_some_and(|r| r.req == req)
            || self.relating.as_ref().is_some_and(|r| r.req == req)
            || self.pending.values().any(|(r, _)| r == req)
            || self.chat_pending.as_ref().is_some_and(|(r, _)| r == req)
    }

    fn cancel_request(&self, req: &str, cx: &mut Context<Self>) {
        Brain::global(cx).update(cx, |b, cx| b.cancel(req, cx));
    }

    /// Run `f` on this page after the current update finishes (safe to start
    /// another brain request from inside a brain callback).
    fn later(&self, cx: &mut Context<Self>, f: impl FnOnce(&mut NotePage, &mut Context<NotePage>) + 'static) {
        let this = cx.entity().downgrade();
        cx.defer(move |cx| {
            if let Some(this) = this.upgrade() {
                this.update(cx, f);
            }
        });
    }

    /// Tell the writer a request failed, in words that fit the kind of failure
    /// (the brain classifies it; see brain/src/protocol.ts `ErrorKind`).
    fn report(&mut self, kind: &str, message: &str, cx: &mut Context<Self>) {
        // After this update: a failure can arrive from inside the brain's own update (a
        // brain that stops fails every waiting request), and describing it reads the brain.
        let (kind, message) = (kind.to_string(), message.to_string());
        self.later(cx, move |page, cx| {
            let short = page.describe_failure(&kind, &message, cx);
            page.notice(short, cx);
        });
    }

    /// One plain sentence for a failed request (see [`Self::report`]).
    fn describe_failure(&self, kind: &str, message: &str, cx: &App) -> String {
        if message.to_lowercase().contains("bun is not installed") {
            "Inkling’s brain needs Bun. Install it from bun.sh.".to_string()
        } else {
            match kind {
                "auth" => "Sign in to Claude Code first: run claude in Terminal, then /login.".to_string(),
                // Say when it comes back, when the brain has told us.
                "rate_limit" => match Brain::global(cx).read(cx).limit.as_ref().and_then(|l| l.resets_at).and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)) {
                    Some(at) => {
                        let at = at.with_timezone(&chrono::Local);
                        let today = at.date_naive() == chrono::Local::now().date_naive();
                        format!("You’ve reached your Claude usage limit. It resets {}.", at.format(if today { "at %H:%M" } else { "%a %-d %b at %H:%M" }))
                    }
                    None => "You’ve reached your Claude usage limit for now. Try again later.".to_string(),
                },
                "overloaded" => "Claude is overloaded right now. Try again in a moment.".to_string(),
                "bad_output" => "The answer came back in a shape Inkling can’t use. Try again.".to_string(),
                "timeout" => "The model took too long to answer, so Inkling stopped waiting.".to_string(),
                _ => {
                    // short, but cut between words, and marked as cut
                    let mut m: String = message.chars().take(140).collect();
                    if message.chars().count() > 140 {
                        if let Some(i) = m.rfind(' ') {
                            m.truncate(i);
                        }
                        m.push('\u{2026}');
                    }
                    // The sidecar itself is down: say so, rather than blame the model.
                    if matches!(Brain::global(cx).read(cx).state, crate::brain::BrainState::Failed(_)) {
                        // "the brain stopped (exit status: 3)" reads as one sentence, not nested
                        // in another's brackets
                        match m.strip_prefix("the brain stopped") {
                            Some(rest) => {
                                let end = if rest.ends_with('\u{2026}') { "" } else { "." };
                                format!("Inkling’s brain stopped{}{end} Settings \u{203a} Model has more.", rest.trim_end_matches('.'))
                            }
                            None => format!("Inkling’s brain isn’t running ({m}). Settings \u{203a} Model has more."),
                        }
                    } else {
                        format!("Couldn’t reach the model: {m}")
                    }
                }
            }
        }
    }

    /// A problem, in the window's toasts (ochre, with a close control).
    fn notice(&mut self, text: String, cx: &mut Context<Self>) {
        crate::toast::show(cx, crate::toast::Kind::Problem, text);
        cx.notify();
    }

    fn read_line(&mut self, line: usize, text: String, hash: u64, cx: &mut Context<Self>) {
        // The first sentence of a new notebook is always asked about.
        let task = if self.note.first_run { "probe" } else { "read" };
        let mut input = json!({"task": task, "ctx": self.context(cx), "target": text});
        // Read it through: a read of a finished draft, and a failure ends the pass.
        let through = self.reads_through(hash);
        if through {
            input["through"] = json!(true);
        }
        let target = text.clone();
        let reads = task == "read";
        let req = self.run(input, false, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.reading = None;
                if let Some(n) = v["repeats"].as_u64() {
                    log::info!("read held back: it makes let-stand point {n} again");
                }
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if v["speak"].as_bool() == Some(true) && !body.is_empty() {
                    let quote = v["quote"].as_str().unwrap_or("").to_string();
                    let kind = if v["kind"] == "correction" { ThoughtKind::Correction } else { ThoughtKind::Question };
                    let sketch = v["sketch"].as_bool() == Some(true);
                    if let Some(id) = page.add_thought(line, &target, &quote, kind, Some(body), cx) {
                        page.attach_check(&id, &v["check"], cx);
                        page.arrived.insert(id.clone(), std::time::Instant::now());
                        if sketch {
                            page.later(cx, move |page, cx| page.request_drawing(&id, None, cx));
                        }
                    }
                }
                if page.note.first_run {
                    page.note.first_run = false;
                    page.schedule_save(cx);
                }
                if reads {
                    let (target, note) = (target.clone(), page.note.id.clone());
                    page.later(cx, move |page, cx| {
                        if page.note.id == note {
                            page.look_elsewhere(line, target, cx)
                        }
                    });
                }
                page.take_answers(line, &target, &v, cx);
                page.later(cx, |page, cx| page.after_read(cx));
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.reading = None;
                // It was never read: let a later pause try this paragraph again.
                page.note.read_hashes.retain(|h| *h != hash);
                page.schedule_save(cx);
                if through {
                    page.pass_failed(&kind, &message, cx);
                } else if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                page.later(cx, |page, cx| page.after_read(cx));
            }
            BrainMsg::Delta(_) => {}
        });
        self.reading = Some(Reading { req, line, hash });
        cx.notify();
    }

    fn answer_call(&mut self, line: usize, text: String, cx: &mut Context<Self>) {
        let Some(id) = self.add_thought(line, &text, &text, ThoughtKind::Answer, None, cx) else { return };
        let input = json!({"task": "answer", "ctx": self.context(cx), "target": text});
        self.stream_into(id, input, cx);
    }

    /// Locate `quote` inside the paragraph `line_text` (line `line_hint` when it
    /// was sent) and open a thread there.
    fn add_thought(
        &mut self,
        line_hint: usize,
        line_text: &str,
        quote: &str,
        kind: ThoughtKind,
        first: Option<String>,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.editor.read(cx).text().to_string();
        let lines = line_ranges(&text);
        let range = locate_thought(&text, &lines, line_hint, line_text, quote)?;
        Some(self.open_thread(range, kind, first, cx))
    }

    fn pick_side(&self, range: &Range<usize>, cx: &App) -> Side {
        match cx.global::<Store>().settings.placement {
            Placement::Right | Placement::Marks => return Side::Right,
            Placement::Both => {}
        }
        let ed = self.editor.read(cx);
        let y = ed
            .layout
            .as_ref()
            .and_then(|l| l.rects(range.clone()).first().map(|r| r.origin.y.as_f32()))
            .unwrap_or(0.);
        let near = |side: Side| {
            self.note
                .threads
                .iter()
                .filter(|t| t.status == ThreadStatus::Open && t.side == side)
                .filter(|t| {
                    ed.anchor(&t.id)
                        .and_then(|a| ed.layout.as_ref().and_then(|l| l.rects(a.range.clone()).first().map(|r| r.origin.y.as_f32())))
                        .is_some_and(|ty| (ty - y).abs() < 160.)
                })
                .count()
        };
        let (l, r) = (near(Side::Left), near(Side::Right));
        if l < r {
            Side::Left
        } else if r < l {
            Side::Right
        } else {
            // Alternate so a long note fills both margins.
            let open = self.note.threads.iter().filter(|t| t.status == ThreadStatus::Open).count();
            if open % 2 == 0 { Side::Left } else { Side::Right }
        }
    }

    fn open_thread(&mut self, range: Range<usize>, kind: ThoughtKind, first: Option<String>, cx: &mut Context<Self>) -> String {
        let id = new_id();
        let quote = self.editor.read(cx).text()[range.clone()].to_string();
        let side = self.pick_side(&range, cx);
        let mut messages = vec![];
        if let Some(text) = first {
            messages.push(Message::new(Role::Mind, text));
        }
        self.note.threads.push(Thread {
            id: id.clone(),
            start: range.start,
            end: range.end,
            quote,
            kind,
            messages,
            drawing: None,
            status: ThreadStatus::Open,
            side,
            mind_name: self.mind.name.clone(),
            tag_offset: (0., 0.),
            created_at: Utc::now(),
            reason: None,
            ended_at: None,
            first_quote: None,
            ..Default::default()
        });
        self.editor.update(cx, |e, cx| e.add_anchor(Anchor { id: id.clone(), range, open: true }, cx));
        self.schedule_save(cx);
        cx.notify();
        id
    }

    fn thread_mut(&mut self, id: &str) -> Option<&mut Thread> {
        self.note.threads.iter_mut().find(|t| t.id == id)
    }

    fn push_message(&mut self, id: &str, role: Role, text: String, cx: &mut Context<Self>) {
        if let Some(t) = self.thread_mut(id) {
            t.messages.push(Message::new(role, text));
        }
        self.schedule_save(cx);
        cx.notify();
    }

    /// A thread opened to hold an answer that never came (an error, or an empty
    /// reply) would sit in the margin as an empty card: take it away again.
    fn drop_if_empty(&mut self, id: &str, cx: &mut Context<Self>) {
        let empty = self.note.threads.iter().any(|t| t.id == id && t.messages.is_empty() && t.drawing.is_none());
        if !empty {
            return;
        }
        self.note.threads.retain(|t| t.id != id);
        self.editor.update(cx, |e, cx| e.remove_anchor(id, cx));
        if self.active.as_deref() == Some(id) {
            self.active = None;
            self.editor.update(cx, |e, _| e.active_anchor = None);
        }
        self.replies.remove(id);
        self.reply_subs.remove(id);
        self.marks_open.remove(id);
        self.heights.borrow_mut().remove(id);
        self.schedule_save(cx);
        cx.notify();
    }

    /// A question or reply on this thread got no answer: the tag says why under your
    /// words and offers Try again, which sends `again`. Set after the update the failure
    /// came in (describing it reads the brain, as `report` does).
    fn fail_thread(&mut self, tid: String, kind: String, message: String, again: Value, cx: &mut Context<Self>) {
        self.later(cx, move |page, cx| {
            if page.note.threads.iter().any(|t| t.id == tid) {
                let why = page.describe_failure(&kind, &message, cx);
                page.unanswered.insert(tid, (again, why));
                cx.notify();
            }
        });
    }

    /// Send a thread's unanswered question or reply again.
    fn retry_thread(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.pending.contains_key(id) {
            return;
        }
        let Some((mut input, _)) = self.unanswered.remove(id) else { return };
        if input["task"] == "reply" {
            // a reply that argued with the working (Disagree) is sent again as one
            let disputed = input.as_object_mut().and_then(|o| o.remove("disputed")).is_some_and(|v| v == true);
            let asked = input["message"].as_str().unwrap_or_default().to_string();
            self.run_reply(id.to_string(), input, asked, disputed, cx);
        } else {
            self.stream_into(id.to_string(), input, cx);
        }
    }

    /// Run a streaming task whose answer becomes the thread's next message.
    fn stream_into(&mut self, thread: String, input: Value, cx: &mut Context<Self>) {
        let tid = thread.clone();
        let again = input.clone();
        self.unanswered.remove(&thread);
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Delta(d) => {
                if let Some((_, Pending::Streaming(s))) = page.pending.get_mut(&tid) {
                    s.push_str(&d);
                } else if let Some(p) = page.pending.get_mut(&tid) {
                    p.1 = Pending::Streaming(d);
                }
                cx.notify();
            }
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let text = v["text"].as_str().unwrap_or("").trim().to_string();
                if text.is_empty() {
                    page.drop_if_empty(&tid, cx);
                    page.notice("Inkling came back with nothing to say. Try asking again.".into(), cx);
                } else {
                    page.push_message(&tid, Role::Mind, text, cx);
                }
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                page.drop_if_empty(&tid, cx);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                    page.fail_thread(tid.clone(), kind, message, again.clone(), cx);
                }
                cx.notify();
            }
        });
        self.pending.insert(thread, (req, Pending::Thinking));
        cx.notify();
    }

    fn request_drawing(&mut self, thread: &str, request: Option<String>, cx: &mut Context<Self>) {
        let Some(t) = self.note.threads.iter().find(|t| t.id == thread) else { return };
        let thought = t.messages.iter().rev().find(|m| m.role == Role::Mind).map(|m| m.text.clone()).unwrap_or_default();
        let input = json!({
            "task": "draw",
            "ctx": self.context(cx),
            "quote": t.quote,
            "thought": thought,
            "request": request,
        });
        let tid = thread.to_string();
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let svg = v["svg"].as_str().unwrap_or("").to_string();
                match drawing::parse(&svg) {
                    Some(d) => {
                        if let Some(t) = page.thread_mut(&tid) {
                            t.drawing = Some(svg);
                        }
                        page.playbacks.insert(tid.clone(), Playback::new(d, true));
                        page.schedule_save(cx);
                    }
                    None => {
                        page.drop_if_empty(&tid, cx);
                        page.notice("The sketch came back in a shape Inkling can’t read.".into(), cx);
                    }
                }
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                page.drop_if_empty(&tid, cx);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.pending.insert(thread.to_string(), (req, Pending::Sketching));
        cx.notify();
    }

    // ----- selection actions --------------------------------------------------

    fn selection_range(&self, cx: &App) -> Option<Range<usize>> {
        let ed = self.editor.read(cx);
        let r = ed.selection();
        if r.is_empty() {
            return None;
        }
        // Trim surrounding whitespace so the anchor hugs the words.
        let text = ed.text();
        let s = &text[r.clone()];
        let lead = s.len() - s.trim_start().len();
        let trail = s.len() - s.trim_end().len();
        let r = r.start + lead..r.end - trail;
        (!r.is_empty()).then_some(r)
    }

    /// "Find a loose thread": Inkling looks for the one thing in the selection that
    /// does not hold up yet, and always asks about it.
    pub fn find_loose_thread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // the front is not on the sheet: nothing selected there can be asked about
        if self.back.shown {
            return;
        }
        // ⌘↩ with nothing selected and a thread open: that thread is done.
        if self.editor.read(cx).selection().is_empty() {
            if let Some(id) = self.active.clone() {
                self.set_status(&id, ThreadStatus::Resolved, cx);
            }
            return;
        }
        let Some(range) = self.selection_range(cx) else { return };
        let target = self.editor.read(cx).text()[range.clone()].to_string();
        let id = self.open_thread(range.clone(), ThoughtKind::Question, None, cx);
        self.show_toolbar = false;
        self.asking = None;
        self.editor.update(cx, |e, cx| e.move_cursor_to(range.end, cx));
        self.activate(Some(id.clone()), window, cx);
        let input = json!({"task": "probe", "ctx": self.context(cx), "target": target});
        let tid = id.clone();
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if v["repeats"].as_u64().is_some_and(|n| n > 0) {
                    page.drop_if_empty(&tid, cx);
                    crate::toast::show(cx, crate::toast::Kind::Done, crate::stand::NOTHING_NEW);
                    return;
                }
                if body.is_empty() {
                    page.drop_if_empty(&tid, cx);
                    page.notice("Inkling came back with nothing to say. Try again.".into(), cx);
                    return;
                }
                if v["kind"] == "correction" {
                    if let Some(t) = page.thread_mut(&tid) {
                        t.kind = ThoughtKind::Correction;
                    }
                }
                page.push_message(&tid, Role::Mind, body, cx);
                page.attach_check(&tid, &v["check"], cx);
                if v["sketch"].as_bool() == Some(true) {
                    let id = tid.clone();
                    page.later(cx, move |page, cx| page.request_drawing(&id, None, cx));
                }
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                page.drop_if_empty(&tid, cx);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.pending.insert(id, (req, Pending::Thinking));
        cx.notify();
    }

    fn start_ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(range) = self.selection_range(cx) else { return };
        // a mind's long name would run out of the box: then the box just asks
        let placeholder = if self.mind.name.chars().count() <= 20 { format!("Ask {} about this\u{2026}", self.mind.name) } else { "Ask about this\u{2026}".to_string() };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.submit_ask(window, cx);
            }
        });
        input.update(cx, |i, cx| i.focus(window, cx));
        self.asking = Some(Asking { range, input, room: vec![self.mind.clone()], _sub: sub });
        self.show_toolbar = false;
        self.menu = None;
        cx.notify();
    }

    fn submit_ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(asking) = self.asking.take() else { return };
        let question = asking.input.read(cx).value().trim().to_string();
        if question.is_empty() {
            return;
        }
        let Some(target) = self.editor.read(cx).text().get(asking.range.clone()).map(str::to_string) else {
            log::warn!("ask: the selection changed under the question; not sent");
            return;
        };
        if target.trim().is_empty() {
            return;
        }
        if asking.room.is_empty() {
            self.hang_own(asking.range.clone(), question, window, cx);
            return;
        }
        let end = asking.range.end;
        self.editor.update(cx, |e, cx| e.move_cursor_to(end, cx));
        // One thread per mind in the room, each in its own colour, answering side by side.
        let mut first = None;
        for mind in &asking.room {
            let id = self.open_thread(asking.range.clone(), ThoughtKind::Ask, None, cx);
            if let Some(t) = self.thread_mut(&id) {
                t.messages.push(Message::new(Role::You, question.clone()));
                t.side = Side::Right;
                t.mind_name = mind.name.clone();
            }
            let input = json!({"task": "ask", "ctx": self.context_with(mind, cx), "target": target, "question": question});
            self.stream_into(id.clone(), input, cx);
            first.get_or_insert(id);
        }
        if let Some(id) = first {
            self.activate(Some(id), window, cx);
        }
    }

    /// Minds that can sit in the room: yours, then the templates you have not made yet.
    fn room_candidates(&self, cx: &App) -> Vec<Mind> {
        let store = cx.global::<Store>();
        let mut all: Vec<Mind> = store.minds.clone();
        if !all.iter().any(|m| m.name == self.mind.name) {
            all.insert(0, self.mind.clone());
        }
        if !all.iter().any(|m| m.name == "Inkling") {
            all.insert(0, Mind::inkling());
        }
        // A template with no way of thinking (Blank) is a place to start writing a
        // mind, not a reader: it has nothing to bring to the room.
        for tmpl in crate::store::mind_templates().into_iter().filter(|t| !t.how_it_thinks.trim().is_empty()) {
            if !all.iter().any(|m| m.name == tmpl.name) {
                let id = format!("room-{}", tmpl.name.to_lowercase().replace(' ', "-"));
                all.push(Mind { id, ..tmpl });
            }
        }
        all.truncate(7);
        all
    }

    fn toggle_room(&mut self, mind: Mind, cx: &mut Context<Self>) {
        let Some(asking) = self.asking.as_mut() else { return };
        if let Some(i) = asking.room.iter().position(|m| m.name == mind.name) {
            asking.room.remove(i);
        } else if asking.room.len() < ROOM_MAX {
            asking.room.push(mind);
        }
        cx.notify();
    }

    /// Dismiss the open thread from the keyboard (⌘⇧⌫).
    pub fn dismiss_active(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.clone().filter(|_| !self.back.shown) {
            self.set_status(&id, ThreadStatus::Dismissed, cx);
        }
    }

    /// Put the last dismissed thread back (⌥⌘Z, or Undo), while its toast shows.
    pub fn undo_dismiss(&mut self, cx: &mut Context<Self>) {
        let Some((id, range, _)) = self.last_dismissed.take() else { return };
        if !crate::toast::live(cx, crate::toast::Kind::ThreadDismissed) {
            return;
        }
        crate::toast::close(cx, crate::toast::Kind::ThreadDismissed);
        let Some(quote) = self.thread_mut(&id).map(|t| {
            t.set_status(ThreadStatus::Open);
            t.quote.clone()
        }) else {
            return;
        };
        // the words may have moved while the toast showed: find them again, nearest to where they were
        let text = self.editor.read(cx).text();
        let range = if text.get(range.clone()) == Some(quote.as_str()) { range } else { nearest_match(text, &quote, range.start).map_or(range, |i| i..i + quote.len()) };
        self.editor.update(cx, |e, cx| e.add_anchor(Anchor { id: id.clone(), range, open: true }, cx));
        self.snipped.clear();
        self.schedule_save(cx);
        cx.notify();
    }

    fn sketch_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(range) = self.selection_range(cx) else { return };
        let target = self.editor.read(cx).text()[range.clone()].to_string();
        let id = self.open_thread(range.clone(), ThoughtKind::Sketch, None, cx);
        self.show_toolbar = false;
        self.asking = None;
        self.editor.update(cx, |e, cx| e.move_cursor_to(range.end, cx));
        self.activate(Some(id.clone()), window, cx);
        let input = json!({"task": "sketch", "ctx": self.context(cx), "target": target});
        let tid = id.clone();
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if !body.is_empty() {
                    page.push_message(&tid, Role::Mind, body, cx);
                }
                let id = tid.clone();
                page.later(cx, move |page, cx| page.request_drawing(&id, None, cx));
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                page.drop_if_empty(&tid, cx);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.pending.insert(id, (req, Pending::Thinking));
        cx.notify();
    }

    // ----- threads ------------------------------------------------------------

    fn activate(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.active = id.clone();
        self.menu = None;
        self.editor.update(cx, |e, _| e.active_anchor = id.clone());
        if let Some(id) = id {
            if !self.replies.contains_key(&id) {
                let hint = if self.is_own(&id) { "Add to it\u{2026}" } else { "Write back\u{2026}" };
                let input = cx.new(|cx| InputState::new(window, cx).placeholder(hint));
                let tid = id.clone();
                let sub = cx.subscribe_in(&input, window, move |this, _, ev: &InputEvent, window, cx| match ev {
                    InputEvent::PressEnter { .. } => this.send_reply(&tid, window, cx),
                    // its send arrow lights up once there is something to send, as the chat's does
                    InputEvent::Change => cx.notify(),
                    _ => {}
                });
                self.reply_subs.insert(id.clone(), sub);
                self.replies.insert(id.clone(), input);
            }
        }
        cx.notify();
    }

    /// "Inkling learns about you": when a message says something lasting about
    /// the writer, fold it into the about-you paragraph in the background.
    fn maybe_learn(&mut self, message: &str, cx: &mut Context<Self>) {
        let s = &cx.global::<Store>().settings;
        if !s.learns_about_me {
            return;
        }
        let lower = format!(" {} ", message.to_lowercase());
        let personal = [" i ", " i'm ", " im ", " i am ", " my ", " i've ", " i'd ", " me ", " i work", " i study", " i prefer"]
            .iter()
            .any(|k| lower.contains(k));
        if !personal || message.split_whitespace().count() < 4 {
            return;
        }
        let about = s.about_me.clone();
        let input = json!({"task": "learn", "aboutMe": if about.trim().is_empty() { Value::Null } else { Value::String(about) }, "message": message});
        self.run_any_note(input, false, cx, |_, msg, cx| {
            if let BrainMsg::Done(v) = msg {
                if v["update"].as_bool() == Some(true) {
                    let text = v["aboutMe"].as_str().unwrap_or("").trim().to_string();
                    if !text.is_empty() {
                        let store = cx.global_mut::<Store>();
                        store.settings.about_me = text;
                        store.save_settings();
                        cx.emit(PageEvent::Learned);
                    }
                }
            }
        });
    }

    fn send_reply(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_own(id) {
            return self.add_to_own(id, window, cx);
        }
        let Some(input) = self.replies.get(id).cloned() else { return };
        let text = input.read(cx).value().trim().to_string();
        if text.is_empty() || self.pending.contains_key(id) {
            return;
        }
        input.update(cx, |i, cx| i.set_value("", window, cx));
        let Some(t) = self.note.threads.iter().find(|t| t.id == id).cloned() else { return };
        self.push_message(id, Role::You, text.clone(), cx);
        self.maybe_learn(&text, cx);
        let dispute = self.dispute_message(id, &text, window, cx);
        let disputed = dispute.is_some();
        let input = json!({
            "task": "reply",
            "ctx": self.context(cx),
            "quote": t.quote,
            "messages": t.messages.iter().map(|m| json!({"role": role_wire(m.role), "text": crate::working::wire_text(m)})).collect::<Vec<_>>(),
            "message": dispute.unwrap_or(text.clone()),
        });
        let input = self.between_reply(&t, input, cx);
        self.run_reply(id.to_string(), input, text, disputed, cx);
    }

    /// Send a reply on a thread (`input`, the reply task) and take in the answer.
    /// `disputed`: the reply argues with the mind's working (Disagree), so the answer may
    /// take the correction back.
    fn run_reply(&mut self, tid: String, input: Value, asked: String, disputed: bool, cx: &mut Context<Self>) {
        let id = tid.clone();
        // what Try again sends, the dispute remembered (taken off again before it is sent)
        let mut again = input.clone();
        if disputed {
            again["disputed"] = json!(true);
        }
        self.unanswered.remove(&tid);
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if body.is_empty() {
                    page.notice("Inkling came back with nothing to say. Try again.".into(), cx);
                } else {
                    page.push_message(&tid, Role::Mind, body, cx);
                }
                if disputed {
                    page.after_dispute(&tid, v["resolved"].as_bool() == Some(true), cx);
                }
                if v["resolved"].as_bool() == Some(true) {
                    if let Some(t) = page.thread_mut(&tid) {
                        t.tied_by = Some(crate::store::TiedBy::Reply);
                        t.set_status(ThreadStatus::Resolved);
                    }
                    let id = tid.clone();
                    page.editor.update(cx, |e, cx| e.set_anchor_open(&id, false, cx));
                }
                if v["redraw"].as_bool() == Some(true) {
                    let id = tid.clone();
                    let ask = asked.clone();
                    page.later(cx, move |page, cx| page.request_drawing(&id, Some(ask), cx));
                }
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                    page.fail_thread(tid.clone(), kind, message, again.clone(), cx);
                }
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.pending.insert(id, (req, Pending::Thinking));
        cx.notify();
    }

    fn set_status(&mut self, id: &str, status: ThreadStatus, cx: &mut Context<Self>) {
        self.before_status(id, status, cx);
        // a mind's open point is let stand, with what you typed in its tag's box as your reason
        let typed = self.replies.get(id).map(|i| i.read(cx).value().to_string()).unwrap_or_default();
        let mut said = crate::stand::DISMISSED;
        if let Some(t) = self.thread_mut(id) {
            if status == ThreadStatus::Dismissed {
                said = crate::stand::dismiss(t, &typed);
            } else {
                t.set_status(status);
            }
        }
        // A dismissed thread will never show its answer: stop asking for it.
        if status == ThreadStatus::Dismissed {
            if let Some((req, _)) = self.pending.remove(id) {
                self.cancel_request(&req, cx);
            }
        }
        // The thread answers for it: tied off, cut, or loosened again.
        match status {
            ThreadStatus::Resolved => {
                self.tying.insert(id.to_string(), std::time::Instant::now());
                if let Some(r) = self.ropes.get_mut(id) {
                    r.tie();
                }
            }
            ThreadStatus::Dismissed => {
                let range = self.editor.read(cx).anchors().iter().find(|a| a.id == id).map(|a| a.range.clone());
                if let Some(range) = range {
                    self.last_dismissed = Some((id.to_string(), range, std::time::Instant::now()));
                    crate::toast::show(cx, crate::toast::Kind::ThreadDismissed, said);
                }
                if let Some(mut r) = self.ropes.remove(id) {
                    r.cut();
                    let t = Theme::get(cx);
                    let hue = if self.is_own(id) { own::ink(t) } else { t.voice_ink };
                    self.snipped.push((r, hue, std::time::Instant::now()));
                }
            }
            ThreadStatus::Open => {
                self.tying.remove(id);
                if let Some(r) = self.ropes.get_mut(id) {
                    r.loosen();
                }
            }
        }
        let open = status == ThreadStatus::Open;
        let id2 = id.to_string();
        self.editor.update(cx, |e, cx| {
            if status == ThreadStatus::Dismissed {
                e.remove_anchor(&id2, cx);
            } else {
                e.set_anchor_open(&id2, open, cx);
            }
        });
        if self.active.as_deref() == Some(id) {
            self.active = None;
            self.editor.update(cx, |e, _| e.active_anchor = None);
        }
        self.menu = None;
        self.schedule_save(cx);
        cx.notify();
    }

    /// Copy says it copied: a tick in place of its icon for a moment, and "Thread copied" in
    /// the toasts, as the chat's and the note's copies do.
    fn show_copied(&mut self, id: &str, cx: &mut Context<Self>) {
        crate::toast::show(cx, crate::toast::Kind::Done, "Thread copied");
        let at = std::time::Instant::now();
        self.copied = Some((id.to_string(), at));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if this.copied.as_ref().is_some_and(|(_, t)| *t == at) {
                    this.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// The "?" after a quote in Marks-only placement: show or hide its card.
    fn toggle_mark(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.marks_open.contains(id) {
            self.marks_open.remove(id);
            self.activate(None, window, cx);
        } else {
            self.marks_open.insert(id.to_string());
            self.activate(Some(id.to_string()), window, cx);
        }
    }

    fn thread_text(&self, id: &str) -> String {
        let Some(t) = self.note.threads.iter().find(|t| t.id == id) else { return String::new() };
        let mut out = format!("> {}\n\n", t.quote);
        // named as Export names them: the mind (Inkling when an old thread has no name), and Me
        for m in &t.messages {
            match m.role {
                Role::Mind => {
                    out.push_str(&format!("{}: {}\n\n", t.author(m), m.text));
                    for line in Self::check_lines(m) {
                        out.push_str(&format!("{line}\n\n"));
                    }
                }
                Role::You => out.push_str(&format!("Me: {}\n\n", m.text)),
            }
        }
        out.trim_end().to_string()
    }

    fn make_note_from_thread(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(t) = self.note.threads.iter().find(|t| t.id == id).cloned() else { return };
        let mode = cx.global::<Store>().settings.default_mode;
        // A quote can span paragraphs; blocks are single lines. A long one makes a title
        // cut at a word, with an ellipsis, rather than mid-word.
        let quote = t.quote.replace('\n', " ");
        let title = short_title(&quote, 60);
        let mut note = Note::new(&title, mode, self.note.folder.clone());
        note.blocks = vec![Block { kind: BlockKind::Title, text: title }];
        note.blocks.push(Block { kind: BlockKind::Quote, text: quote });
        for m in &t.messages {
            let who = if m.role == Role::Mind { t.author(m) } else { "Me".into() };
            note.blocks.push(Block { kind: BlockKind::Paragraph, text: format!("{who}: {}", m.text.replace('\n', " ")) });
        }
        note.blocks.push(Block { kind: BlockKind::Paragraph, text: String::new() });
        for b in &note.blocks {
            note.read_hashes.push(hash_text(&b.text));
        }
        let new_id = note.id.clone();
        let store = cx.global_mut::<Store>();
        store.save_note(&note);
        store.notes.insert(0, note);
        self.menu = None;
        cx.emit(PageEvent::NotesChanged);
        cx.emit(PageEvent::OpenNote(new_id));
    }

    // ----- note settings ------------------------------------------------------

    fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        self.note.mode = mode;
        self.menu = None;
        self.schedule_save(cx);
        cx.notify();
    }

    fn set_mind(&mut self, id: &str, cx: &mut Context<Self>) {
        self.note.mind_id = id.to_string();
        self.mind = cx.global::<Store>().mind(id);
        self.menu = None;
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn refresh_mind(&mut self, cx: &mut Context<Self>) {
        self.mind = cx.global::<Store>().mind(&self.note.mind_id);
        cx.notify();
    }

    fn edit_brief(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.note.brief.clone().unwrap_or_default();
        let input = cx.new(|cx| {
            let mut i = InputState::new(window, cx).placeholder("Prep for a system design interview. Be tough on hand-waving.");
            i.set_value(current, window, cx);
            i
        });
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.save_brief(window, cx);
            }
        });
        input.update(cx, |i, cx| i.focus(window, cx));
        self.brief_input = Some((input, sub));
        cx.notify();
    }

    fn save_brief(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((input, _)) = self.brief_input.take() {
            let v = input.read(cx).value().trim().to_string();
            self.note.brief = (!v.is_empty()).then_some(v);
            self.schedule_save(cx);
        }
        self.editor.update(cx, |e, cx| e.focus(window, cx));
        cx.notify();
    }

    // ----- chat ---------------------------------------------------------------

    pub fn toggle_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.chat_open = !self.chat_open;
        if self.chat_open {
            self.chat_scroll.scroll_to_bottom();
            self.chat_input.update(cx, |i, cx| i.focus(window, cx));
        } else {
            self.editor.update(cx, |e, cx| e.focus(window, cx));
        }
        cx.notify();
    }

    fn clear_chat(&mut self, cx: &mut Context<Self>) {
        // An answer still on its way belongs to the conversation being cleared.
        if let Some((req, _)) = self.chat_pending.take() {
            self.cancel_request(&req, cx);
        }
        self.note.chat.clear();
        self.chat_error = None;
        self.schedule_save(cx);
        cx.notify();
    }

    /// Send the question the chat failed on again.
    fn retry_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.chat_pending.is_some() || self.note.chat.last().is_none_or(|m| m.role != Role::You) {
            return;
        }
        let Some(last) = self.note.chat.pop() else { return };
        self.chat_error = None;
        self.send_chat(Some(last.text), window, cx);
    }

    fn send_chat(&mut self, preset: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.chat_pending.is_some() {
            return;
        }
        let text = match preset {
            Some(t) => t,
            None => self.chat_input.read(cx).value().trim().to_string(),
        };
        if text.is_empty() {
            return;
        }
        self.chat_input.update(cx, |i, cx| i.set_value("", window, cx));
        let history: Vec<Value> = self
            .note
            .chat
            .iter()
            .map(|m| json!({"role": role_wire(m.role), "text": m.text}))
            .collect();
        self.note.chat.push(ChatMessage { role: Role::You, text: text.clone(), at: Utc::now() });
        self.chat_scroll.scroll_to_bottom();
        self.maybe_learn(&text, cx);
        let ed = self.editor.read(cx);
        let near = ed.line_text(ed.line_of(ed.cursor()));
        let input = json!({
            "task": "chat",
            "ctx": self.context(cx),
            "history": history,
            "message": text,
            "near": if near.trim().is_empty() { Value::Null } else { Value::String(near) },
        });
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Delta(d) => {
                if let Some((_, s)) = page.chat_pending.as_mut() {
                    s.push_str(&d);
                }
                page.chat_scroll.scroll_to_bottom();
                cx.notify();
            }
            BrainMsg::Done(v) => {
                page.chat_pending = None;
                page.chat_scroll.scroll_to_bottom();
                let text = v["text"].as_str().unwrap_or("").trim().to_string();
                if text.is_empty() {
                    page.chat_error = Some("Inkling came back with nothing to say.".into());
                } else {
                    page.note.chat.push(ChatMessage { role: Role::Mind, text, at: Utc::now() });
                    page.schedule_save(cx);
                }
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.chat_pending = None;
                // Said in the chat, under the question it failed on, with Try again (after
                // this update, as `report` does).
                if kind != "cancelled" {
                    page.later(cx, move |page, cx| {
                        page.chat_error = Some(page.describe_failure(&kind, &message, cx));
                        page.chat_scroll.scroll_to_bottom();
                        cx.notify();
                    });
                }
                cx.notify();
            }
        });
        self.chat_error = None;
        self.chat_pending = Some((req, String::new()));
        self.schedule_save(cx);
        cx.notify();
    }

    // ----- scripted steps (INKLING_SCRIPT) ------------------------------------

    pub(crate) fn script_step(&mut self, step: &crate::script::Step, window: &mut Window, cx: &mut Context<Self>) {
        use crate::script::Step;
        match step {
            Step::Type(text) => {
                self.editor.update(cx, |e, cx| {
                    e.focus(window, cx);
                    e.append_paragraph(text, cx);
                });
            }
            Step::Select(phrase) => {
                let found = self.editor.update(cx, |e, cx| {
                    e.focus(window, cx);
                    e.select_phrase(phrase, cx)
                });
                if !found {
                    log::warn!("script: phrase not found: {phrase}");
                }
            }
            Step::Riddle => self.find_loose_thread(window, cx),
            Step::AskYourself(q) => {
                self.ask_yourself(window, cx);
                if let Some(input) = self.asking.as_ref().map(|a| a.input.clone()) {
                    input.update(cx, |i, cx| i.set_value(q.clone(), window, cx));
                    self.submit_ask(window, cx);
                }
            }
            Step::Call(name) => {
                if !self.call_by_name(name, cx) {
                    log::warn!("script: no thread of yours open, or no mind named {name:?}");
                }
            }
            Step::Visualize => self.sketch_selection(window, cx),
            Step::Ask(q) => {
                self.start_ask(window, cx);
                // a `room:` step before this ask seats those minds
                let names = std::mem::take(&mut self.pending_room);
                if !names.is_empty() {
                    let minds: Vec<Mind> = self
                        .room_candidates(cx)
                        .into_iter()
                        .filter(|m| names.iter().any(|n| m.name.to_lowercase().contains(&n.to_lowercase())))
                        .take(ROOM_MAX)
                        .collect();
                    if let (Some(a), false) = (self.asking.as_mut(), minds.is_empty()) {
                        a.room = minds;
                    }
                }
                if let Some(a) = &self.asking {
                    let input = a.input.clone();
                    input.update(cx, |i, cx| i.set_value(q.clone(), window, cx));
                }
                self.submit_ask(window, cx);
            }
            Step::Activate(n) => {
                let id = self
                    .note
                    .threads
                    .iter()
                    .filter(|t| t.status != ThreadStatus::Dismissed)
                    .nth(n.saturating_sub(1))
                    .map(|t| t.id.clone());
                self.activate(id, window, cx);
            }
            Step::See(n, m) => self.see_step(*n, *m, window, cx),
            Step::Disagree(n) => self.disagree_step(*n, window, cx),
            Step::Reply(text) => {
                if let Some(id) = self.active.clone() {
                    if let Some(input) = self.replies.get(&id).cloned() {
                        input.update(cx, |i, cx| i.set_value(text.clone(), window, cx));
                    }
                    self.send_reply(&id, window, cx);
                }
            }
            Step::Chat(text) => {
                if !self.chat_open {
                    self.toggle_chat(window, cx);
                }
                self.send_chat(Some(text.clone()), window, cx);
            }
            Step::Mode(m) => {
                let mode = if m.contains("ask") { Mode::WhenYouAsk } else { Mode::ThinksWithYou };
                self.set_mode(mode, cx);
            }
            Step::Brief(b) => {
                self.note.brief = (!b.is_empty()).then(|| b.clone());
                self.schedule_save(cx);
                cx.notify();
            }
            Step::Mind(name) => {
                let id = cx.global::<Store>().minds.iter().find(|m| m.name.eq_ignore_ascii_case(name)).map(|m| m.id.clone());
                if let Some(id) = id {
                    self.set_mind(&id, cx);
                }
            }
            Step::Keys(text) => {
                // Typed through the input handler, exactly as keystrokes arrive.
                self.editor.update(cx, |e, cx| {
                    e.focus(window, cx);
                    for (i, part) in text.split('\n').enumerate() {
                        if i > 0 {
                            e.replace_text_in_range(None, "\n", window, cx);
                        }
                        if !part.is_empty() {
                            e.replace_text_in_range(None, part, window, cx);
                        }
                    }
                });
            }
            Step::Enter => self.editor.update(cx, |e, cx| {
                e.focus(window, cx);
                e.replace_text_in_range(None, "\n", window, cx);
            }),
            Step::Caret(at) => {
                let ok = self.editor.update(cx, |e, cx| {
                    let target = match at.as_str() {
                        "end" => Some(e.text().len()),
                        "start" => Some(e.line_range(1).start),
                        phrase => e.text().find(phrase).map(|i| i + phrase.len()),
                    };
                    if let Some(t) = target {
                        e.focus(window, cx);
                        e.move_cursor_to(t, cx);
                    }
                    target.is_some()
                });
                if !ok {
                    log::warn!("script: caret target not found: {at}");
                }
            }
            Step::Edit(old, new) => {
                let ok = self.editor.update(cx, |e, cx| {
                    let Some(i) = e.text().find(old.as_str()) else { return false };
                    e.focus(window, cx);
                    e.select_range(i..i + old.len(), cx);
                    e.replace_text_in_range(None, new, window, cx);
                    true
                });
                if !ok {
                    log::warn!("script: edit target not found: {old}");
                }
            }
            Step::Escape => {
                self.escape(window, cx);
            }
            Step::Turn => self.turn_over(window, cx),
            Step::BackReason(..) | Step::BackReopen(_) | Step::BackShow(_) => crate::back::script_step(self, step, window, cx),
            Step::Resolve(n) | Step::Reopen(n) | Step::Dismiss(n) | Step::Mark(n) | Step::Copy(n) | Step::MakeNote(n) | Step::Retry(n) => {
                let Some(id) = self.nth_thread(*n) else {
                    log::warn!("script: no thread {n}");
                    return;
                };
                match step {
                    Step::Resolve(_) => self.set_status(&id, ThreadStatus::Resolved, cx),
                    Step::Reopen(_) => self.set_status(&id, ThreadStatus::Open, cx),
                    Step::Dismiss(_) => self.set_status(&id, ThreadStatus::Dismissed, cx),
                    Step::Mark(_) => self.toggle_mark(&id, window, cx),
                    Step::Copy(_) => {
                        log::info!("script copy: {:?}", self.copy_text(&id, cx));
                        self.show_copied(&id, cx);
                    }
                    Step::Retry(_) => self.retry_thread(&id, cx),
                    _ => self.make_note_from_thread(&id, cx),
                }
            }
            Step::ClearChat => self.clear_chat(cx),
            Step::Switch(title) => {
                let id = cx.global::<Store>().notes.iter().find(|n| n.title == *title).map(|n| n.id.clone());
                match id {
                    Some(id) => cx.emit(PageEvent::OpenNote(id)),
                    None => log::warn!("script: no note titled {title:?}"),
                }
            }
            Step::Set(key, value) => {
                let s = &mut cx.global_mut::<Store>().settings;
                match (key.as_str(), value.as_str()) {
                    ("pace", "slow") => s.pace = Pace::Slow,
                    ("pace", _) => s.pace = Pace::Regular,
                    ("placement", "right") => s.placement = Placement::Right,
                    ("placement", "marks") => s.placement = Placement::Marks,
                    ("placement", _) => s.placement = Placement::Both,
                    ("learn", v) => s.learns_about_me = v == "on",
                    ("mode", v) => s.default_mode = if v.contains("ask") { Mode::WhenYouAsk } else { Mode::ThinksWithYou },
                    ("appearance", v) => {
                        use crate::store::Appearance;
                        s.appearance = Appearance::ALL.iter().copied().find(|a| a.label().eq_ignore_ascii_case(v)).unwrap_or_default()
                    }
                    ("size", v) => {
                        use crate::store::TextSize;
                        s.text_size = TextSize::ALL.iter().copied().find(|a| a.label().eq_ignore_ascii_case(v)).unwrap_or_default()
                    }
                    ("ink", v) => {
                        use crate::store::InkColor;
                        s.ink = InkColor::ALL.iter().copied().find(|a| a.label().eq_ignore_ascii_case(v)).unwrap_or_default()
                    }
                    ("lettering", v) => {
                        s.lettering = if v.contains("note") { crate::store::Lettering::LikeNotes } else { crate::store::Lettering::Own }
                    }
                    ("pointer", v) => s.ink_from_pointer = v == "on",
                    ("web", v) => s.look_up = v == "on",
                    _ => log::warn!("script: unknown setting {key}={value}"),
                }
                cx.global::<Store>().save_settings();
            }
            Step::Width(w) => {
                let h = window.viewport_size().height.as_f32();
                crate::app::script_resize(window, *w, h);
            }
            Step::Dump(label) => log::info!("script dump {label}: {}", self.debug_state(window, cx)),
            Step::Room(names) => self.pending_room = names.clone(),
            Step::UndoDismiss => self.undo_dismiss(cx),
            Step::Stand(n, reason) => {
                let Some(id) = self.nth_thread(*n) else {
                    log::warn!("script: no thread {n}");
                    return;
                };
                // as a person does it: open the tag, type in its box, Let stand
                self.activate(Some(id.clone()), window, cx);
                if let Some(input) = self.replies.get(&id).cloned() {
                    input.update(cx, |i, cx| i.set_value(reason.clone(), window, cx));
                }
                self.set_status(&id, ThreadStatus::Dismissed, cx);
            }
            Step::WriteIn | Step::UndoWriteIn => self.write_in_step(step, window, cx),
            Step::Jump(forward) => self.jump_to_open(*forward, window, cx),
            Step::Hover(n) => {
                let ids: Vec<String> = self.hover_tag.drain().filter_map(|k| k.split_once(':').map(|(id, _)| id.to_string())).collect();
                for id in ids {
                    self.hover_left.insert(id, std::time::Instant::now());
                }
                if let Some(id) = (*n > 0).then(|| self.nth_thread(*n)).flatten() {
                    self.set_tag_hover(&id, "tab", true, cx);
                }
            }
            Step::Drag(n, dx, dy) => {
                // the same path a pointer drag takes, so it is kept on screen the same way
                if let Some(id) = (*n > 0).then(|| self.nth_thread(*n)).flatten() {
                    let origin = self.note.threads.iter().find(|t| t.id == id).map(|t| t.tag_offset).unwrap_or_default();
                    let tab = self.tab_bounds.borrow().get(&id).copied();
                    self.drag = Some(TagDrag { id, start: point(px(0.), px(0.)), origin, moved: false, tab });
                    self.drag_tag_by(point(px(*dx), px(*dy)), cx);
                    self.end_tag_drag(cx);
                }
            }
            Step::Follow(n) => match self.nth_thread(*n) {
                Some(id) if self.far_end(&id).is_some() => self.follow(&id, cx),
                _ => log::warn!("script: no thread between notes {n}"),
            },
            Step::ReadThrough => self.toggle_read_through(cx),
            Step::Log(text) => log::info!("script log: {text}"),
            Step::Knot(n) => self.click_knot(*n, window, cx),
            Step::HoverKnot(n) => self.hover_knot(*n, cx),
            Step::Export(format) => {
                self.sync_from_editor(cx);
                let text = match format.as_str() {
                    "html" => crate::export::to_html(&self.note, true),
                    "word" => crate::export::to_word_html(&self.note, true),
                    _ => crate::export::to_markdown(&self.note, true),
                };
                log::info!("script export {format}: {text:?}");
            }
            _ => {}
        }
        cx.notify();
    }

    fn nth_thread(&self, n: usize) -> Option<String> {
        self.note
            .threads
            .iter()
            .filter(|t| t.status != ThreadStatus::Dismissed)
            .nth(n.saturating_sub(1))
            .map(|t| t.id.clone())
    }

    /// The page as the script sees it: what is pending, where the cards sit.
    fn debug_state(&mut self, window: &mut Window, cx: &mut Context<Self>) -> String {
        let short = |id: &str| id.chars().take(8).collect::<String>();
        let placement = cx.global::<Store>().settings.placement;
        let viewport = window.viewport_size().width.as_f32();
        let available = viewport - if self.chat_open { CHAT_WIDTH } else { 0. };
        // the page's geometry as `render` works it out, so the dump matches the screen
        let m = Metrics::for_width(available - 2. * (SHEET_PAD + 14.));
        let fits = m.page() + 2. * SHEET_PAD <= available - 16.;
        let (shown, slots, boxes) = self.margin_layout(placement, m, cx);
        // Marks only: what follows each thread's words
        let marks: Vec<String> = if placement == Placement::Marks {
            shown.iter().filter(|t| boxes.get(&t.id).is_some_and(|r| !r.is_empty())).map(|t| format!("{}:{}", short(&t.id), mark_glyph(t))).collect()
        } else {
            vec![]
        };
        // where each tag's thread comes up and where it ties on, in the page body
        let ropes: Vec<Value> = slots
            .iter()
            .filter_map(|s| {
                let rects = boxes.get(&s.id).filter(|r| !r.is_empty())?;
                let (a, b) = self.thread_ends(s, rects, m, cx);
                Some(json!({"id": short(&s.id), "side": format!("{:?}", s.side), "from": [a.x.as_f32(), a.y.as_f32()], "to": [b.x.as_f32(), b.y.as_f32()]}))
            })
            .collect();
        let ed = self.editor.read(cx);
        let error = crate::toast::text_of(cx, crate::toast::Kind::Problem);
        let mut state = json!({
            "note": {"title": ed.title(), "mode": format!("{:?}", self.note.mode), "mind": self.mind.name},
            "reading": self.reading.as_ref().map(|r| json!({"req": r.req, "line": r.line})),
            "waiting": self.waiting,
            "pending": self.pending.iter().map(|(id, (req, p))| json!({
                "thread": short(id),
                "req": req,
                "state": match p { Pending::Thinking => "thinking".to_string(), Pending::Streaming(s) => format!("streaming:{s}"), Pending::Sketching => "sketching".to_string() },
            })).collect::<Vec<_>>(),
            "chat_pending": self.chat_pending.as_ref().map(|(req, s)| json!({"req": req, "partial": s})),
            "chat": self.note.chat.len(),
            "active": self.active.as_deref().map(short),
            "error": error,
            "toolbar": self.show_toolbar,
            "selection": [ed.selection().start, ed.selection().end],
            "editor_focused": ed.is_focused(window),
            // the control the keyboard is on: a name from ui::key, "note", "reply", "chat",
            // "menu" (with the item the arrow keys are on), or null
            "focus": ui::focused_key(window, cx).map(|k| k.to_string()).or_else(|| {
                if ed.is_focused(window) {
                    Some("note".into())
                } else if self.menu_focus.is_focused(window) {
                    Some(format!("menu:{}", self.menu_pick))
                } else if self.active.as_ref().and_then(|a| self.replies.get(a)).is_some_and(|i| i.read(cx).focus_handle(cx).is_focused(window)) {
                    Some("reply".into())
                } else if self.chat_input.read(cx).focus_handle(cx).is_focused(window) {
                    Some("chat".into())
                } else {
                    window.focused(cx).map(|_| "elsewhere".into())
                }
            }),
            "menu": self.menu.map(|m| format!("{m:?}")),
            "enlarged": self.enlarged.as_deref().map(short),
            // the thread whose Copy shows its tick right now
            "copied": self.copied.as_ref().filter(|(_, at)| at.elapsed() < COPIED_FOR).map(|(id, _)| short(id)),
            "brief_editing": self.brief_input.is_some(),
            "chat_open": self.chat_open,
            "chat_error": self.chat_error,
            "hidden": self.just_my_words,
            "asking": self.asking.is_some(),
            "last_edited": self.last_edited,
            "cursor_line": ed.line_of(ed.cursor()),
            "kinds": ed.lines().iter().map(|(k, _)| crate::store::kind_name(*k)).collect::<Vec<_>>(),
            "metrics": {"viewport": viewport, "available": available, "column": m.column, "margin": m.margin, "both": m.both, "page": m.page(), "fits": fits, "column_left": m.column_left(), "right_left": m.right_left()},
            "marks": marks,
            "ropes": ropes,
            "slots": slots.iter().map(|s| json!({"id": short(&s.id), "side": format!("{:?}", s.side), "anchor": s.anchor_top.as_f32(), "top": s.top.as_f32(), "h": s.height.as_f32()})).collect::<Vec<_>>(),
            "threads": self.note.threads.iter().map(|t| json!({
                "id": short(&t.id), "kind": format!("{:?}", t.kind), "status": format!("{:?}", t.status), "side": format!("{:?}", t.side),
                "quote": t.quote, "msgs": t.messages.len(), "drawing": t.drawing.is_some(), "own": t.own,
                "ending": format!("{:?}", t.ending()), "reason": t.reason, "before": t.first_quote,
                "check": t.messages.iter().find_map(|m| m.check.as_ref()).map(crate::working::summary),
            })).collect::<Vec<_>>(),
            "anchors": ed.anchors().iter().map(|a| json!({"id": short(&a.id), "range": [a.range.start, a.range.end], "open": a.open})).collect::<Vec<_>>(),
            "read_hashes": self.note.read_hashes.len(),
            "left_alone": self.note.left_alone.len(),
            // how far the page is scrolled down, in pixels (0 = at the top)
            "scrolled": -self.scroll.offset().y.as_f32(),
            "waiting_tags": if self.typing_recently() { self.arrived.keys().map(|id| short(id)).collect::<Vec<_>>() } else { vec![] },
            "typing": self.typing_recently(),
            "rechecking": self.rechecking.iter().map(|id| short(id)).collect::<Vec<_>>(),
            "undo": self.last_dismissed.as_ref().filter(|_| crate::toast::live(cx, crate::toast::Kind::ThreadDismissed)).map(|(id, _, _)| short(id)),
            // the window's toasts, oldest first, with where each was drawn
            "toasts": crate::toast::describe(cx),
            "first_run": self.note.first_run,
            "statuses": self.note.threads.iter().map(|t| format!("{}:{:?}:{}", short(&t.id), t.status, t.mind_name)).collect::<Vec<_>>(),
            // tags more than half unfolded, and tags moved from where the margin puts them
            "unfolded": self.unfold.iter().filter(|(_, u)| **u > 0.5).map(|(id, _)| short(id)).collect::<Vec<_>>(),
            "moved_tags": self.note.threads.iter().filter(|t| t.tag_offset != (0., 0.)).map(|t| format!("{}:{:.0},{:.0}", short(&t.id), t.tag_offset.0, t.tag_offset.1)).collect::<Vec<_>>(),
        });
        // the paper you can see and each tab, in the window at the last paint: a dragged tag
        // stays on the paper (set apart, as one more key is past what json! can expand)
        let (l, t, r, b) = self.paper();
        state["paper"] = json!([l, t, r, b]);
        // each unfolded tag where it was drawn, and the column of your words from the header
        // row to the last line, in the window: no tag you have not dragged may cover it
        state["panels"] = Value::Object(
            self.panel_bounds
                .borrow()
                .iter()
                .map(|(id, b)| (short(id), json!([b.origin.x.as_f32(), b.origin.y.as_f32(), b.size.width.as_f32(), b.size.height.as_f32()])))
                .collect(),
        );
        if let (Some(c), Some(s)) = (self.column_bounds.get(), self.sheet_bounds.get()) {
            let top = s.origin.y.as_f32() + 50.;
            state["column"] = json!([c.origin.x.as_f32(), top, c.size.width.as_f32(), (c.origin.y + c.size.height).as_f32() - top]);
        }
        // the sidebar: asked open, and how much of it shows (0 to 1) as it slides
        state["sidebar"] = cx.try_global::<crate::app::SidebarShown>().map_or(Value::Null, |s| json!({"open": s.open, "shown": s.shown}));
        // threads whose question or reply got no answer, with why (Try again in the tag)
        state["unanswered"] = self.unanswered.iter().map(|(id, (_, why))| json!({"id": short(id), "why": why})).collect();
        // whether the open tag's second button reads Let stand (else Dismiss), or null with none open
        state["lets_stand"] = json!(self.active.as_ref().and_then(|a| self.note.threads.iter().find(|t| &t.id == a)).map(crate::stand::lets_stand));
        // Loose ends, open or as it would open now (see loose_ends.rs)
        state["loose_ends"] = crate::loose_ends::describe(&self.note, window, cx);
        // the back of the note: `turned`, and `back`, its entries in order
        crate::back::dump_into(&mut state, self, window, cx);
        self.between_state(&mut state, cx);
        state["read_through"] = self.read_through_state();
        state["room"] = json!(self.asking.as_ref().map(|a| a.room.iter().map(|m| m.name.clone()).collect::<Vec<_>>()).unwrap_or_default());
        self.dump_answers(&mut state, cx);
        state["tabs"] = Value::Object(
            self.tab_bounds
                .borrow()
                .iter()
                .map(|(id, b)| (short(id), json!([b.origin.x.as_f32(), b.origin.y.as_f32(), b.size.width.as_f32(), b.size.height.as_f32()])))
                .collect(),
        );
        // threads between notes: each one's far end as its tag shows it, and following
        state["ends"] = self.ends_state(window, cx);
        self.dump_write_in(&mut state, cx);
        state.to_string()
    }

    // ----- export -------------------------------------------------------------

    pub fn markdown(&mut self, with_threads: bool, cx: &mut Context<Self>) -> String {
        self.sync_from_editor(cx);
        crate::export::to_markdown(&self.note, with_threads)
    }

    /// How far in from the right of the page's area the sheet's padding ends: where
    /// something sitting in the sheet's bottom corner (the window's corner hint) lines
    /// up, so it rests on the paper and never on the cloth or across the sheet's edge.
    pub fn sheet_inner_right(&self) -> f32 {
        let width = self.viewport.width.as_f32() - if self.chat_open { CHAT_WIDTH } else { 0. };
        let sheet_w = self.m.page() + 2. * SHEET_PAD;
        let x = if sheet_w <= width - 16. { (width - sheet_w) / 2. } else { 12. };
        (width - x - sheet_w + SHEET_PAD).max(SHEET_PAD)
    }

    pub fn note_snapshot(&mut self, cx: &mut Context<Self>) -> Note {
        self.sync_from_editor(cx);
        self.note.clone()
    }
}

// ----- rendering ---------------------------------------------------------------

impl NotePage {
    fn header(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let mode = self.note.mode;
        let mind_name = self.mind.name.clone();
        let has_brief = self.note.brief.is_some();
        let jmw = self.just_my_words;
        let open = self.note.threads.iter().filter(|th| th.status == ThreadStatus::Open).count();
        let menu = self.menu;
        // Every control in the row is quiet until the pointer is on it, and stays lit
        // while its menu is open.
        let control = move |id: &'static str, lit: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(24.))
                .text_size(px(ts::UI))
                .text_color(if lit { p.ink } else { p.graphite })
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .active(|s| s.opacity(0.7))
        };
        let mind_menu = (menu == Some(Menu::Mind)).then(|| self.menu_card(Menu::Mind, cx));
        let mode_menu = (menu == Some(Menu::Mode)).then(|| self.menu_card(Menu::Mode, cx));
        // Each menu hangs so its words line up with its button's: the mind's dot and name
        // over the button's dot and name; a mode's title under "Reads along", its icon
        // out to the left (the card's edge, an item's inset, then a 12 px icon and a gap).
        let (mind_dx, mode_dx) = (-(1. + 5. + 9.), -(1. + 5. + 9. + 12. + 7.));
        div()
            .flex()
            .flex_row()
            // a narrow column (a small window with the chat open) wraps the row
            .flex_wrap()
            .items_center()
            .gap_x(px(16.))
            .gap_y(px(2.))
            .min_h(px(24.))
            // the controls step back while you write, like the window's own
            .opacity(self.chrome.get())
            .font_family(SANS)
            .when(!jmw, |d| {
                d.child(
                    div()
                        .relative()
                        .child(
                            control("mind-picker", menu == Some(Menu::Mind))
                                .gap(px(6.))
                                .text_color(p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(ui::small_dot(t.voice_ink, 7.))
                                // a mind with a long name gives way, so the row still fits the column
                                .child(div().max_w(px(220.)).truncate().child(mind_name))
                                .child(icon("chevron-down", 12., p.faint))
                                .keyed_as("header:mind", 4., t.voice_ink, cx, |this, window, cx| this.toggle_menu(Menu::Mind, window, cx)),
                        )
                        .children(mind_menu.map(|card| self.drop_down(card, mind_dx, cx))),
                )
                .when(!has_brief, |d| {
                    d.child(
                        control("tell", false)
                            .child(icon("pencil", 12., p.faint))
                            .child("Tell Inkling")
                            .keyed_as("header:tell", 4., t.voice_ink, cx, |this, window, cx| this.edit_brief(window, cx)),
                    )
                })
                .child(
                    div()
                        .relative()
                        .child(
                            control("mode", menu == Some(Menu::Mode))
                                .child(match mode {
                                    Mode::ThinksWithYou => "Reads along",
                                    Mode::WhenYouAsk => "Waits to be asked",
                                })
                                .child(icon("chevron-down", 12., p.faint))
                                .keyed_as("header:mode", 4., t.voice_ink, cx, |this, window, cx| this.toggle_menu(Menu::Mode, window, cx)),
                        )
                        .children(mode_menu.map(|card| self.drop_down(card, mode_dx, cx))),
                )
            })
            // the open count and Hide threads wrap together, and keep to the right edge on
            // whichever line they land (a spacer that wrapped with them, or did not, put
            // them left or right by turns as the window narrowed)
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .when(!jmw && open > 0, |d| {
                        d.child(
                            control("open-count", false)
                                .gap(px(6.))
                                .child(ui::small_dot(t.voice_ink, 7.))
                                .child(if open == 1 { "1 loose thread".to_string() } else { format!("{open} loose threads") })
                                .keyed_as("header:open-count", 4., t.voice_ink, cx, |this, window, cx| this.jump_to_open(true, window, cx)),
                        )
                    })
                    .when(jmw || self.note.visible_threads() > 0, |d| {
                        d.child(
                            control("hide-threads", false)
                                .child(if jmw { "Show threads" } else { "Hide threads" })
                                .keyed_as("header:hide-threads", 4., t.voice_ink, cx, move |this, _, cx| this.set_hidden(!jmw, cx)),
                        )
                    })
                    // the back of the note, once it has had a thread (where Turn back sits on the back)
                    .when(!jmw && !self.note.threads.is_empty(), |d| {
                        d.child(control("turn-over", false).child("Turn over").keyed_as("header:turn-over", 4., t.voice_ink, cx, |this, window, cx| this.turn_over(window, cx)))
                    }),
            )
            .into_any_element()
    }

    /// Open or close a header menu. A press on its own button while it is open closes it:
    /// the press outside the menu has just closed it, and must not open it again. An open
    /// menu takes the keyboard, on the item chosen now, so the arrow keys work at once.
    fn toggle_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        let just_closed = self.menu_closed.is_some_and(|(m, at)| m == menu && at.elapsed() < Duration::from_millis(400));
        self.menu_closed = None;
        self.menu = if self.menu == Some(menu) || just_closed { None } else { Some(menu) };
        if self.menu.is_some() {
            self.menu_pick = self.menu_now(menu, cx);
            if !self.menu_focus.is_focused(window) {
                self.menu_return = window.focused(cx);
            }
            self.menu_focus.focus(window, cx);
        } else {
            self.leave_menu(window, cx);
        }
        cx.notify();
    }

    /// How many items a header menu has, and which one is chosen now. The mind menu ends
    /// with New mind and Manage, the mode menu with Read it through.
    fn menu_len(&self, menu: Menu, cx: &App) -> usize {
        match menu {
            Menu::Mind => cx.global::<Store>().minds.len() + 2,
            _ => 3,
        }
    }

    fn menu_now(&self, menu: Menu, cx: &App) -> usize {
        match menu {
            Menu::Mind => cx.global::<Store>().minds.iter().position(|m| m.id == self.note.mind_id).unwrap_or(0),
            _ => usize::from(self.note.mode == Mode::WhenYouAsk),
        }
    }

    /// An open menu from the keyboard: ↑ ↓ (and Home, End) move, Return or Space chooses.
    /// Esc (through `escape`) and Tab (through the window, `close_menu`) close it.
    fn menu_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu else { return };
        let n = self.menu_len(menu, cx);
        let k = &ev.keystroke;
        match k.key.as_str() {
            "down" => self.menu_pick = (self.menu_pick + 1) % n,
            "up" => self.menu_pick = (self.menu_pick + n - 1) % n,
            "home" => self.menu_pick = 0,
            "end" => self.menu_pick = n - 1,
            _ if ui::is_press(k) => self.choose(menu, self.menu_pick, window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Item `i` of a header menu, chosen with the keyboard or a click.
    fn choose(&mut self, menu: Menu, i: usize, window: &mut Window, cx: &mut Context<Self>) {
        match menu {
            Menu::Mind => {
                let minds = cx.global::<Store>().minds.clone();
                match i.checked_sub(minds.len()) {
                    None => self.set_mind(&minds[i].id, cx),
                    Some(0) => cx.emit(PageEvent::OpenSettings(crate::settings::Tab::NewMind)),
                    Some(_) => cx.emit(PageEvent::OpenSettings(crate::settings::Tab::Minds)),
                }
            }
            _ if i == 2 => self.toggle_read_through(cx),
            _ => self.set_mode(if i == 0 { Mode::ThinksWithYou } else { Mode::WhenYouAsk }, cx),
        }
        self.menu = None;
        self.leave_menu(window, cx);
    }

    /// Whether an open header menu has the keyboard.
    pub fn menu_has_keyboard(&self, window: &Window) -> bool {
        self.menu.is_some() && self.menu_focus.is_focused(window)
    }

    /// Close the open header menu (Tab, which then moves on from its button).
    pub fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.leave_menu(window, cx);
        cx.notify();
    }

    /// A menu closed: the keyboard goes back to where it was when it opened (the button,
    /// from the keyboard; the note, from a click).
    fn leave_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let back = self.menu_return.take();
        if self.menu_focus.is_focused(window) {
            match back {
                Some(h) => h.focus(window, cx),
                None => self.editor.update(cx, |e, cx| e.focus(window, cx)),
            }
        }
    }

    /// A header menu, hung under the button that opened it, `dx` from its left edge.
    fn drop_down(&self, card: AnyElement, dx: f32, cx: &mut Context<Self>) -> AnyElement {
        deferred(
            div()
                .absolute()
                .top(px(26.))
                .left(px(dx))
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    if let Some(m) = this.menu.take() {
                        this.menu_closed = Some((m, std::time::Instant::now()));
                    }
                    cx.notify();
                }))
                .child(card),
        )
        .with_priority(2)
        .into_any_element()
    }

    /// The next (or previous) open thread after the one you are on, in reading order.
    pub fn jump_to_open(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let mut order: Vec<(usize, String)> = self.numbers.iter().map(|(id, n)| (*n, id.clone())).collect();
        order.sort();
        if order.is_empty() {
            return;
        }
        let current = self.active.as_ref().and_then(|a| self.numbers.get(a)).copied();
        let next = match (current, forward) {
            (Some(n), true) => order.iter().find(|(m, _)| *m > n).or(order.first()),
            (Some(n), false) => order.iter().rev().find(|(m, _)| *m < n).or(order.last()),
            (None, true) => order.first(),
            (None, false) => order.last(),
        };
        if let Some((_, id)) = next.cloned() {
            // from the back, the sheet turns to the front first
            if self.back.shown {
                self.turn_back(window, cx);
            }
            self.show_thread(&id, window, cx);
        }
    }

    /// Open a thread's tag at its words: the caret at their start, the page scrolled to
    /// them, the keyboard in its reply box (⌥⌘↓, Show on the page).
    pub(crate) fn show_thread(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let start = self.editor.read(cx).anchors().iter().find(|a| a.id == id).map(|a| a.range.start);
        self.activate(Some(id.to_string()), window, cx);
        if let Some(start) = start {
            self.editor.update(cx, |e, cx| e.move_cursor_to(start, cx));
            self.keep_cursor_visible(window, cx);
        }
        if let Some(input) = self.replies.get(id).cloned() {
            input.update(cx, |i, cx| i.focus(window, cx));
        }
    }

    fn brief_line(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = Theme::get(cx).clone();
        let p = t.p;
        // The brief is the writer's note to Inkling, set like the rest of the interface
        // (Inkling's lettering is for Inkling's own words).
        if let Some(input) = self.brief_input.as_ref().map(|(i, _)| i.clone()) {
            return Some(
                div()
                    .mt(px(8.))
                    .px(px(12.))
                    .pt(px(10.))
                    .pb(px(10.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(p.rule)
                    .bg(p.card)
                    .child(
                        div()
                            .text_size(px(ts::META))
                            .text_color(p.graphite)
                            .child("What’s this note for, or how should Inkling be here? It keeps this in mind."),
                    )
                    .child(div().mt(px(4.)).text_color(p.ink).child(Input::new(&input).appearance(false).px_0().text_size(px(ts::BODY))))
                    .child(
                        div()
                            .mt(px(6.))
                            .flex()
                            .justify_end()
                            .items_center()
                            .gap(px(14.))
                            .child(ui::ghost_button("brief-cancel", "Cancel", &t).keyed_as("brief:cancel", 4., t.voice_ink, cx, |this, window, cx| {
                                this.brief_input = None;
                                this.editor.update(cx, |e, cx| e.focus(window, cx));
                                cx.notify();
                            }))
                            .child(ui::primary_button("brief-save", "Save", &t).keyed_as("brief:save", 12., t.voice_ink, cx, |this, window, cx| this.save_brief(window, cx))),
                    )
                    .into_any_element(),
            );
        }
        let brief = self.note.brief.clone()?;
        Some(
            div()
                .id("brief")
                .group("brief")
                .mt(px(4.))
                .flex()
                .flex_row()
                // the label sits beside the brief's first line, however many it wraps to
                .items_start()
                .gap(px(8.))
                .cursor_pointer()
                .child(
                    div()
                        .h(px(19.))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(4.))
                        .text_size(px(ts::META))
                        .text_color(p.faint)
                        .group_hover("brief", move |s| s.text_color(p.ink))
                        .child(icon("pencil", 10., p.faint))
                        .child("Tell Inkling"),
                )
                .child(div().min_w(px(0.)).text_size(px(ts::UI)).line_height(px(19.)).text_color(p.graphite).child(brief))
                .keyed_as("header:brief", 4., t.voice_ink, cx, |this, window, cx| this.edit_brief(window, cx))
                .into_any_element(),
        )
    }

    /// The header's menus hang under their own buttons (see [`Self::drop_down`]), so
    /// nothing is drawn for them at the page level any more.
    fn menus(&mut self, _cx: &mut Context<Self>) -> Option<AnyElement> {
        None
    }

    /// The card for a header menu: the minds, each in its own thread colour, or the modes.
    fn menu_card(&mut self, menu: Menu, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        match menu {
            Menu::Mind => {
                let (minds, chosen) = {
                    let store = cx.global::<Store>();
                    (store.minds.clone(), store.settings.ink)
                };
                let current = self.note.mind_id.clone();
                let own = self.mind.name.clone();
                let pick = self.menu_pick;
                let n = minds.len();
                let mut c = ui::menu_card(&t).w(px(248.)).track_focus(&self.menu_focus).on_key_down(cx.listener(Self::menu_key));
                for (i, m) in minds.iter().enumerate() {
                    if i == 1 {
                        c = c.child(div().mx(px(14.)).mt(px(6.)).mb(px(2.)).text_size(px(ts::META)).text_color(p.faint).child("Yours"));
                    }
                    let id = m.id.clone();
                    // The note's mind wears the thread you chose; others keep their own.
                    let color = if m.id == current { t.voice_ink } else { mind_thread(&m.name, &own, chosen, &t) };
                    c = c.child(
                        ui::menu_item(SharedString::from(format!("mind-{}", m.id)), &t)
                            .relative()
                            .when(pick == i, |d| d.bg(p.hover).child(ui::focus_ring(&self.menu_focus, 6., color)))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(ui::small_dot(color, 6.))
                                    .child(div().flex_1().min_w(px(0.)).truncate().child(m.name.clone()))
                                    .when(m.id == current, |d| d.child(icon("check", 12., p.ink))),
                            )
                            .when(!m.description.is_empty(), |d| {
                                d.child(div().pl(px(13.)).text_size(px(ts::META)).line_height(px(15.)).text_color(p.graphite).child(m.description.clone()))
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.set_mind(&id, cx);
                                this.leave_menu(window, cx);
                            })),
                    );
                }
                c.child(div().mt(px(4.)).border_t_1().border_color(p.rule))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .px(px(14.))
                            .pt(px(7.))
                            .pb(px(3.))
                            .child(
                                ui::ghost_button("new-mind", "New mind", &t)
                                    .relative()
                                    .when(pick == n, |d| d.text_color(p.ink).child(ui::focus_ring(&self.menu_focus, 4., t.voice_ink)))
                                    .on_click(cx.listener(move |this, _, window, cx| this.choose(Menu::Mind, n, window, cx))),
                            )
                            .child(
                                ui::ghost_button("manage-minds", "Manage", &t)
                                    .relative()
                                    .when(pick == n + 1, |d| d.text_color(p.ink).child(ui::focus_ring(&self.menu_focus, 4., t.voice_ink)))
                                    .on_click(cx.listener(move |this, _, window, cx| this.choose(Menu::Mind, n + 1, window, cx))),
                            ),
                    )
                    .into_any_element()
            }
            Menu::Mode | Menu::Thread => {
                let current = self.note.mode;
                let pick = self.menu_pick;
                let focus = self.menu_focus.clone();
                // The same words as Settings › New notes start in.
                let item = |id: &'static str, mode: Mode, title: &'static str, body: &'static str, cx: &mut Context<Self>| {
                    ui::menu_item(id, &t)
                        .relative()
                        .when(pick == usize::from(mode == Mode::WhenYouAsk), |d| d.bg(p.hover).child(ui::focus_ring(&focus, 6., t.voice_ink)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .child(icon(if mode == Mode::ThinksWithYou { "lightbulb" } else { "message-circle" }, 12., p.graphite))
                                .child(div().flex_1().child(title))
                                .when(current == mode, |d| d.child(icon("check", 12., p.ink))),
                        )
                        .child(div().pl(px(19.)).text_size(px(ts::META)).line_height(px(15.)).text_color(p.graphite).child(body))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.set_mode(mode, cx);
                            this.leave_menu(window, cx);
                        }))
                };
                ui::menu_card(&t)
                    .w(px(236.))
                    .track_focus(&self.menu_focus)
                    .on_key_down(cx.listener(Self::menu_key))
                    .child(item("mode-thinks", Mode::ThinksWithYou, "Reads along", "Speaks up when something’s worth a second look.", cx))
                    .child(item("mode-ask", Mode::WhenYouAsk, "Waits to be asked", "Quiet until you say its name, select words, or open the chat.", cx))
                    .child(self.read_through_item(pick, cx))
                    .into_any_element()
            }
        }
    }

    fn message_view(&self, m: &Message, t: &Theme, align_right: bool) -> Div {
        let p = t.p;
        match m.role {
            // Inkling's words are for reading: ink, not thread.
            Role::Mind => div()
                .font_family(t.voice_font.clone())
                .text_size(px(ts::BODY))
                .line_height(px(20.))
                .text_color(p.ink)
                .when(align_right, |d| d.text_right())
                .child(m.text.clone()),
            // your reply wraps under itself, beside "You", inside the tag
            Role::You => div()
                .flex()
                .gap(px(6.))
                .font_family(SANS)
                .text_size(px(ts::UI))
                .line_height(px(18.))
                .text_color(p.graphite)
                .when(align_right, |d| d.text_right())
                .child(div().flex_none().text_color(p.faint).child("You"))
                .child(div().flex_1().min_w(px(0.)).child(m.text.clone())),
        }
    }

    fn thinking_dot(t: &Theme, id: impl Into<ElementId>) -> impl IntoElement {
        let color = t.voice_ink;
        ui::small_dot(color, 5.).with_animation(
            id,
            Animation::new(Duration::from_millis(1100)).repeat(),
            move |d, delta| {
                let a = 0.3 + 0.6 * (0.5 + 0.5 * (delta * std::f32::consts::TAU).sin());
                d.bg(color.opacity(a))
            },
        )
    }

    fn drawing_block(&mut self, id: &str, width: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.drawing_view(id, width, false, cx)
    }

    /// A sketch with its caption and its controls: ‹ step › and replay, and (in the
    /// margin) a way to see it larger. The controls are proper buttons, not bare glyphs.
    fn drawing_view(&mut self, id: &str, width: f32, enlarged: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let pb = self.playbacks.get(id)?;
        let (step, progress) = pb.state();
        let n = pb.steps();
        let caption = pb.caption();
        let canvas = drawing::canvas_for(pb.drawing.clone(), step, progress, px(width), t.clone());
        let tag = if enlarged { "big" } else { "tag" };
        let control = |name: &'static str, glyph: &'static str, id: &str, cx: &mut Context<Self>, f: fn(&mut Playback)| {
            let tid = id.to_string();
            let what = match name {
                "prev" => "Previous step",
                "next" => "Next step",
                _ => "Play it again",
            };
            ui::icon_button(SharedString::from(format!("{name}-{tag}-{id}")), glyph, 12., &t).tooltip(crate::app::tip(what, None)).on_click(cx.listener(move |this, _, _, cx| {
                if let Some(pb) = this.playbacks.get_mut(&tid) {
                    f(pb);
                }
                cx.notify();
            }))
        };
        let id_large = id.to_string();
        Some(
            div()
                .mt(px(10.))
                .flex()
                .flex_col()
                .child(canvas)
                .child(
                    div()
                        .mt(px(6.))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex_1()
                                // let the caption wrap, so the step controls keep their room
                                .min_w(px(0.))
                                .font_family(t.voice_font.clone())
                                .text_size(px(if enlarged { ts::UI } else { ts::SMALL }))
                                .line_height(px(16.))
                                .text_color(p.graphite)
                                .child(caption),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .when(n > 1, |d| {
                                    d.child(control("prev", "chevron-left", id, cx, Playback::prev))
                                        .child(div().px(px(2.)).text_size(px(ts::META)).text_color(p.faint).child(format!("{step}/{n}")))
                                        .child(control("next", "chevron-right", id, cx, Playback::next))
                                })
                                .child(control("replay", "rotate-ccw", id, cx, Playback::replay)),
                        ),
                )
                .when(!enlarged, |d| {
                    d.child(
                        div()
                            .id(SharedString::from(format!("large-{id_large}")))
                            .mt(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(ts::META))
                            .text_color(p.faint)
                            .cursor_pointer()
                            .hover(move |s| s.text_color(p.ink))
                            .active(|s| s.opacity(0.7))
                            .child(icon("maximize-2", 10., p.faint))
                            .child("See it larger")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.enlarged = Some(id_large.clone());
                                if let Some(pb) = this.playbacks.get_mut(&id_large) {
                                    pb.replay();
                                }
                                cx.notify();
                            })),
                    )
                })
                .into_any_element(),
        )
    }

    /// A thread's tag in the margin. Folded, it is a small stitched tab (the loop the
    /// thread is tied to, its number, ? for a question, × for a correction), so the margin
    /// stays quiet. Pointing at it unfolds the whole tag; clicking keeps it open with the
    /// reply box. Drag the tab (or an open tag's top row) to move it anywhere in the
    /// margin; the thread follows.
    fn card(&mut self, thread: &Thread, side: Side, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let id = thread.id.clone();
        let is_active = self.active.as_deref() == Some(id.as_str());
        let tied = thread.status == ThreadStatus::Resolved;
        let thread_color = if thread.own { own::ink(&t) } else { mind_thread(&thread.mind_name, &self.mind.name, cx.global::<Store>().settings.ink, &t) };
        let unfold = self.unfold.get(&id).copied().unwrap_or(0.);
        let since = self.tying.get(&id).map(|at| anim_since(*at, std::time::Instant::now()));
        let fade = if tied && !is_active { tag_fade(since) } else { 1. };
        let number = self.numbers.get(&id).copied();
        let correction = thread.kind == ThoughtKind::Correction;
        let right = side == Side::Right;
        // a thread between notes has a second loop, and its number and mark move over for it
        let between = self.far_end(&id).is_some();

        // ---- the folded tab ----
        let glyph: AnyElement = if tied {
            div().size(px(7.)).rounded_full().bg(thread_color).into_any_element()
        } else {
            div()
                .text_size(px(ts::UI))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(thread_color)
                .child(if correction { "\u{d7}" } else { "?" })
                .into_any_element()
        };
        let label = div()
            .absolute()
            .top(px(4.))
            .when(right, |d| d.left(px(22.)))
            .when(!right, |d| d.right(px(22.)))
            .when(between, |d| if right { d.left(px(22. + ends::LOOP_STEP)) } else { d.right(px(22. + ends::LOOP_STEP)) })
            .h(px(18.))
            .flex()
            .items_center()
            .gap(px(4.))
            .font_family(SANS)
            .children(number.map(|n| div().text_size(px(ts::SMALL)).font_weight(FontWeight::SEMIBOLD).text_color(p.graphite).child(n.to_string())))
            .child(glyph);
        let tab = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .rounded(px(8.))
            .bg(p.card)
            .shadow(vec![BoxShadow { color: p.shadow, offset: point(px(0.), px(1.)), blur_radius: px(4.), spread_radius: px(0.), inset: false }])
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
                    .border_color(thread_color.opacity(0.4)),
            )
            .child(Self::tag_ring(right, thread_color))
            .when(between, |d| d.child(Self::second_loop(right, thread_color)))
            .child(label);

        let panel = (unfold > 0.01).then(|| self.tag_panel(thread, side, thread_color, unfold, cx));

        let id_hover = id.clone();
        let id_down = id.clone();
        let id_click = id.clone();
        let offset = thread.tag_offset;
        let bounds_of = self.tab_bounds.clone();
        let id_bounds = id.clone();
        let where_am_i = canvas(move |b, _, _| {
            bounds_of.borrow_mut().insert(id_bounds, b);
        }, |_, _, _, _| {})
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        div()
            .id(SharedString::from(format!("card-{id}")))
            .relative()
            .w(px(TAB_W))
            .h(px(TAB_H))
            .when(!right, |d| d.ml(px(self.m.margin - TAB_W - LEFT_TAB_OUT)))
            .opacity(fade)
            .cursor_pointer()
            // a tag dragged over your words must not put the caret under it
            .occlude()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                this.set_tag_hover(&id_hover, "tab", *hovered, cx);
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, _| {
                let tab = this.tab_bounds.borrow().get(&id_down).copied();
                this.drag = Some(TagDrag { id: id_down.clone(), start: ev.position, origin: offset, moved: false, tab });
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                if this.drag_moved_recently() {
                    return;
                }
                this.toggle_tag(&id_click, window, cx);
            }))
            .child(where_am_i)
            .child(tab)
            .children(panel)
            .into_any_element()
    }

    /// The loop at the tag's corner: where its thread is tied on.
    fn tag_ring(right: bool, color: Hsla) -> Div {
        div()
            .absolute()
            .top(px(9.))
            .when(right, |d| d.left(px(9.)))
            .when(!right, |d| d.right(px(9.)))
            .size(px(8.))
            .rounded_full()
            .border(px(1.5))
            .border_color(color.opacity(0.9))
    }

    /// The unfolded tag, drawn over its neighbours. It grows out of the tab: the stitched
    /// edge widens and lengthens with it, and the words fade in once there is room.
    fn tag_panel(&mut self, thread: &Thread, side: Side, thread_color: Hsla, unfold: f32, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let id = thread.id.clone();
        let is_active = self.active.as_deref() == Some(id.as_str());
        let pending = self.pending.get(&id).map(|(_, s)| s.clone());
        // an asked thread starts with your question and its answer; replies are what follows
        let replies = thread.messages.len().saturating_sub(if thread.kind == ThoughtKind::Ask { 2 } else { 1 });
        let mind = thread.asker();
        let kind = match thread.kind {
            ThoughtKind::Correction => "correction",
            ThoughtKind::Sketch => "sketch",
            ThoughtKind::Ask => "you asked",
            ThoughtKind::Answer => "answer",
            ThoughtKind::Question => "question",
        };
        let kind = self.between_kind(&id).unwrap_or(kind);
        let tied = thread.status == ThreadStatus::Resolved;
        let right = side == Side::Right;
        let mut width = self.m.margin.max(PANEL_MIN);
        let natural = self.panel_heights.borrow().get(&id).copied().unwrap_or(px(120.));
        // the spring's own position: it passes 1 by about 4% as the tag drops open
        let e = unfold.max(0.);
        let ec = e.min(1.);

        // A tag opens away from the words, never across them unless you dragged it there
        // (see `panel_fit`), and it slides up when it would hang below the paper's edge.
        let mut open_right = right;
        let mut lift = px(0.);
        if let Some(b) = self.tab_bounds.borrow().get(&id).copied() {
            let (left, top, right_edge, bottom) = self.paper();
            let (tab_l, tab_r) = (b.origin.x.as_f32(), (b.origin.x + b.size.width).as_f32());
            // outward, the paper and then the cloth up to 8 px inside the window; back, the
            // paper on the words' side
            let window_w = self.viewport.width.as_f32();
            let (out, back) = if right { (window_w - 8. - tab_l, tab_r - (left + 8.)) } else { (tab_r - 8., right_edge - 8. - tab_l) };
            (width, open_right) = panel_fit(right, width, out, back, thread.tag_offset != (0., 0.));
            let below = px(bottom - 8.) - b.origin.y;
            if natural > below {
                lift = (natural - below).min((b.origin.y - px(top + 4.)).max(px(0.)));
            }
        }
        let lift = lift * e;
        // the tab's loop, in the panel's coordinates
        let w = TAB_W + (width - TAB_W) * e;
        let ring_x = if right { 13. } else { TAB_W - 13. } - if open_right { 0. } else { TAB_W - w };
        let ring_on_left = ring_x < w / 2.;
        // the header keeps clear of the loop: 12 px past it, less the tag's own padding
        let ring_pad = if lift < px(6.) { (if ring_on_left { ring_x } else { w - ring_x }) + 1. } else { 0. };
        // ...and clear of the second loop of a thread between notes
        let between = self.far_end(&id).is_some();
        let ring_pad = ring_pad + if between && lift < px(6.) { ends::LOOP_STEP } else { 0. };

        // the top row: name and kind, number; it is also where an open tag is dragged from
        let id_down = id.clone();
        let id_head = id.clone();
        let offset = thread.tag_offset;
        let mut body = div().flex().flex_col().gap(px(8.));
        body = body.child(
            div()
                .id(SharedString::from(format!("tag-top-{id}")))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .when(!ring_on_left, |d| d.pr(px(ring_pad)))
                .when(ring_on_left, |d| d.pl(px(ring_pad)))
                .cursor_grab()
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, _| {
                    let tab = this.tab_bounds.borrow().get(&id_down).copied();
                this.drag = Some(TagDrag { id: id_down.clone(), start: ev.position, origin: offset, moved: false, tab });
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    if !this.drag_moved_recently() {
                        this.toggle_tag(&id_head, window, cx);
                    }
                }))
                // a long mind's name gives way (with an ellipsis); its kind and number stay
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(ts::SMALL))
                        .child(div().min_w(px(0.)).truncate().font_weight(FontWeight::SEMIBOLD).text_color(thread_color).child(mind))
                        .child(div().flex_none().text_color(p.faint).child(if Self::taken_back(thread) {
                            "\u{b7} taken back".to_string()
                        } else if tied {
                            "\u{b7} tied off".to_string()
                        } else {
                            format!("\u{b7} {kind}")
                        })),
                )
                .children(self.numbers.get(&id).map(|n| div().flex_none().text_size(px(ts::SMALL)).text_color(p.faint).child(n.to_string()))),
        );
        if tied && !is_active {
            let first = thread.messages.first().map(|m| m.text.clone()).unwrap_or_default();
            let short: String = first.chars().take(64).collect::<String>() + if first.chars().count() > 64 { "\u{2026}" } else { "" };
            body = body.child(div().text_size(px(ts::UI)).line_height(px(18.)).text_color(p.faint).child(short));
        } else {
            // Folded, a tag shows what it says: a question's question, an asked mind's answer.
            let messages: Vec<&Message> = if is_active {
                thread.messages.iter().collect()
            } else if thread.kind == ThoughtKind::Ask {
                thread.messages.iter().rev().find(|m| m.role == Role::Mind).or(thread.messages.first()).into_iter().collect()
            } else {
                thread.messages.iter().take(1).collect()
            };
            // a thread between notes quotes the other end under what it asks
            let mut far = self.far_end_block(thread, thread_color, cx);
            for m in messages {
                body = body.child(self.thread_message(thread, m, &t, cx));
                if let Some(w) = self.working_block(thread, m, thread_color, is_active, width - 30., cx) {
                    body = body.child(w);
                }
                body = body.children(far.take());
            }
            body = body.children(far);
            if let Some(Pending::Streaming(s)) = &pending {
                body = body.child(self.thread_message(thread, &self.partial(&id, s.clone()), &t, cx));
            }
            body = body.children(is_active.then(|| self.answer_row(thread, thread_color, cx)).flatten());
            if let Some(d) = self.drawing_block(&id, width - 36., cx) {
                body = body.child(d);
            }
            match &pending {
                Some(Pending::Thinking) | Some(Pending::Streaming(_)) => {
                    body = body.child(div().flex().child(Self::thinking_dot(&t, SharedString::from(format!("think-{id}")))));
                }
                Some(Pending::Sketching) => {
                    body = body.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().text_size(px(ts::UI)).text_color(p.faint).child("Stitching a sketch\u{2026}"))
                            .child(Self::thinking_dot(&t, SharedString::from(format!("sketch-{id}")))),
                    );
                }
                None => {}
            }
            // A question or reply that got no answer says why under your words, with Try
            // again once the tag is open, as the chat does under its question.
            if let (None, Some((_, why))) = (&pending, self.unanswered.get(&id)) {
                let id_retry = id.clone();
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(4.))
                        .text_size(px(ts::SMALL))
                        .line_height(px(17.))
                        .text_color(p.graphite)
                        .child(why.clone())
                        .when(is_active, |d| {
                            d.child(
                                ui::ghost_button(SharedString::from(format!("retry-{id}")), "Try again", &t)
                                    .keyed_as("tag:retry", 4., thread_color, cx, move |this, _, cx| this.retry_thread(&id_retry, cx)),
                            )
                        }),
                );
            }
            body = body.children(self.write_in_offer(thread, thread_color, cx));
            if !is_active && replies > 0 {
                body = body.child(
                    div()
                        .text_size(px(ts::SMALL))
                        .text_color(p.faint)
                        .child(Self::more_label(thread, replies)),
                );
            }
        }
        if is_active {
            if let Some(input) = self.replies.get(&id).cloned() {
                let id_send = id.clone();
                let can_send = pending.is_none() && !input.read(cx).value().trim().is_empty();
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .mt(px(2.))
                        .px(px(10.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .bg(p.paper)
                        .border_1()
                        .border_color(p.rule)
                        // the box's padding is the only inset, as in the ask box and the chat
                        .child(div().flex_1().child(Input::new(&input).appearance(false).px_0().text_size(px(ts::BODY))))
                        .child(
                            div()
                                .id(SharedString::from(format!("send-{id_send}")))
                                .p(px(2.))
                                .m(px(-2.))
                                .rounded(px(4.))
                                .child(icon("send", 13., if can_send { p.ink } else { p.faint }))
                                // nothing to send, or an answer on its way: quiet, as the chat's arrow is
                                .when(!can_send, |d| d.opacity(0.6))
                                .when(can_send, |d| {
                                    d.cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .active(|s| s.opacity(0.7))
                                        .on_click(cx.listener(move |this, _, window, cx| this.send_reply(&id_send, window, cx)))
                                }),
                        ),
                );
            }
            let id_res = id.clone();
            let id_dis = id.clone();
            let lets_stand = crate::stand::lets_stand(thread);
            let id_copy = id.clone();
            let copied = self.copied.as_ref().is_some_and(|(c, at)| *c == id && at.elapsed() < COPIED_FOR);
            let id_note = id.clone();
            body = body.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        ui::ghost_button(SharedString::from(format!("res-{id_res}")), if tied { "Reopen" } else { "Resolve" }, &t)
                            .keyed_as("tag:resolve", 4., thread_color, cx, move |this, _, cx| {
                                let st = if tied { ThreadStatus::Open } else { ThreadStatus::Resolved };
                                this.set_status(&id_res, st, cx)
                            }),
                    )
                    .child(
                        ui::ghost_button(SharedString::from(format!("dis-{id_dis}")), if lets_stand { crate::stand::BUTTON } else { "Dismiss" }, &t)
                            .when(lets_stand, |b| b.tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new(crate::stand::TOOLTIP).build(window, cx)))
                            .keyed_as("tag:dismiss", 4., thread_color, cx, move |this, _, cx| this.set_status(&id_dis, ThreadStatus::Dismissed, cx)),
                    )
                    .children((thread.own && !tied).then(|| self.own_ask_button(&id, thread_color, cx)))
                    .child(div().flex_1())
                    .child(
                        ui::ghost_button(SharedString::from(format!("note-{id_note}")), "Make a note", &t)
                            .keyed_as("tag:note", 4., thread_color, cx, move |this, _, cx| this.make_note_from_thread(&id_note, cx)),
                    )
                    // Copy is the least used of the four: an icon, so the row fits a margin
                    .child(
                        div()
                            .id(SharedString::from(format!("copy-{id_copy}")))
                            .cursor_pointer()
                            .p(px(2.))
                            .rounded(px(4.))
                            .hover(move |s| s.bg(p.hover))
                            .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Copy the thread").build(window, cx))
                            // a tick where the pointer is, the words in the toasts (the row has no room)
                            .child(icon(if copied { "check" } else { "copy" }, 12., if copied { p.ink } else { p.faint }))
                            .keyed_as("tag:copy", 4., thread_color, cx, move |this, _, cx| {
                                let text = this.copy_text(&id_copy, cx);
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                                this.show_copied(&id_copy, cx);
                            }),
                    ),
            );
        }

        // the words, laid out at full width and revealed as the tag grows
        let owner: AnyView = cx.entity().into();
        let content = div().w(px(width)).px(px(15.)).pt(px(9.)).pb(px(13.)).font_family(SANS).tab_group().tab_index(ui::tab::TAG).child(body);
        let h = px(TAB_H) + (natural.max(px(TAB_H)) - px(TAB_H)) * e;
        let words = ((unfold - 0.32) / 0.68).clamp(0., 1.);
        let id_hover = id.clone();
        let id_open = id.clone();
        deferred(
            div()
                .id(SharedString::from(format!("panel-{id}")))
                .occlude()
                .cursor_pointer()
                .absolute()
                .top(-lift)
                .when(open_right, |d| d.left_0())
                .when(!open_right, |d| d.right_0())
                .w(px(w))
                .h(h)
                .overflow_hidden()
                .rounded(px(8. + 2. * ec))
                .bg(p.card)
                .shadow(vec![BoxShadow {
                    color: p.shadow.opacity(0.6 + 0.4 * ec),
                    offset: point(px(0.), px(1. + 3. * ec)),
                    blur_radius: px(4. + 10. * ec),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    this.set_tag_hover(&id_hover, "panel", *hovered, cx);
                }))
                // a click on a tag that only unfolded under the pointer opens it for a reply
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.active.as_deref() != Some(id_open.as_str()) && !this.drag_moved_recently() {
                        this.toggle_tag(&id_open, window, cx);
                    }
                }))
                // where it was drawn, for the dump
                .child({
                    let (panels, id) = (self.panel_bounds.clone(), id.clone());
                    canvas(move |b, _, _| panels.borrow_mut().insert(id, b), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
                })
                // the stitched edge grows with the tag
                .child(
                    div()
                        .absolute()
                        .top(px(4.))
                        .left(px(4.))
                        .right(px(4.))
                        .bottom(px(4.))
                        .rounded(px(6. + ec))
                        .border_1()
                        .border_dashed()
                        .border_color(thread_color.opacity(0.42)),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .when(open_right, |d| d.left_0())
                        .when(!open_right, |d| d.right_0())
                        .opacity(words)
                        .child(margin::measured(id.clone(), content, self.panel_heights.clone(), owner)),
                )
                // the loop stays where the thread is tied on; a lifted tag covers it
                .when(lift < px(6.), |d| {
                    d.child(
                        div()
                            .absolute()
                            .top(px(9.) + lift)
                            .left(px(ring_x - 4.))
                            .size(px(8.))
                            .rounded_full()
                            .border(px(1.5))
                            .border_color(thread_color.opacity(0.9)),
                    )
                })
                .when(between && lift < px(6.), |d| d.child(Self::panel_second_loop(ring_x, ring_on_left, px(9.) + lift, thread_color))),
        )
        .with_priority(2)
        .into_any_element()
    }

    /// Pointer over a tag (its tab or its open panel). Leaving starts a short grace
    /// period so moving from the tab onto the panel does not fold it.
    fn set_tag_hover(&mut self, id: &str, part: &str, hovered: bool, cx: &mut Context<Self>) {
        let key = format!("{id}:{part}");
        if hovered {
            self.hover_tag.insert(key);
            self.hover_at.entry(id.to_string()).or_insert_with(std::time::Instant::now);
        } else {
            let was_open = self.tag_rested(id);
            self.hover_tag.remove(&key);
            if !self.tag_pointed(id) {
                self.hover_at.remove(id);
                if was_open {
                    self.hover_left.insert(id.to_string(), std::time::Instant::now());
                }
            }
        }
        cx.notify();
    }

    fn tag_pointed(&self, id: &str) -> bool {
        self.hover_tag.contains(&format!("{id}:tab")) || self.hover_tag.contains(&format!("{id}:panel"))
    }

    /// The pointer has rested on this tag long enough to unfold it, and you have not
    /// typed since it came to rest (a key in the note folds it).
    fn tag_rested(&self, id: &str) -> bool {
        let Some(at) = self.hover_at.get(id) else { return false };
        self.tag_pointed(id) && at.elapsed() >= HOVER_REST && self.last_typed.is_none_or(|k| k < *at)
    }

    /// Click a tab: open it (and its reply box), or close it if it is the open one.
    fn toggle_tag(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.active.as_deref() == Some(id) {
            self.activate(None, window, cx);
            return;
        }
        self.activate(Some(id.to_string()), window, cx);
        if let Some(input) = self.replies.get(id).cloned() {
            input.update(cx, |i, cx| i.focus(window, cx));
        }
    }

    /// The paper you can see, in the window: (left, top, right, bottom). Tags hang on the
    /// page, so a dragged tag stays on it and an open one looks for room on it, never out
    /// over the cloth. Before the first paint, the window less its band of cloth.
    fn paper(&self) -> (f32, f32, f32, f32) {
        let (w, h) = (self.viewport.width.as_f32(), self.viewport.height.as_f32());
        match self.sheet_bounds.get() {
            Some(s) => {
                let (x, y) = (s.origin.x.as_f32(), s.origin.y.as_f32());
                (x, y.max(CHROME_H), x + s.size.width.as_f32(), (y + s.size.height.as_f32()).min(h))
            }
            None => (0., CHROME_H, w, h),
        }
    }

    /// Move the tag being dragged by `delta` from where the drag began, kept on the paper,
    /// 8 px in from its edges. A press that moves less than 4 px is still a click.
    fn drag_tag_by(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(d) = self.drag.as_mut() else { return };
        if !d.moved && (delta.x.abs() + delta.y.abs()) < px(4.) {
            return;
        }
        d.moved = true;
        let (id, origin, tab) = (d.id.clone(), d.origin, d.tab);
        let (mut dx, mut dy) = (delta.x.as_f32(), delta.y.as_f32());
        if let Some(b) = tab {
            let (left, top, right, bottom) = self.paper();
            let (x0, y0) = (b.origin.x.as_f32(), b.origin.y.as_f32());
            let (w, h) = (b.size.width.as_f32(), b.size.height.as_f32());
            let (lo_x, lo_y) = (left + 8. - x0, top + 8. - y0);
            dx = dx.clamp(lo_x, (right - 8. - w - x0).max(lo_x));
            dy = dy.clamp(lo_y, (bottom - 8. - h - y0).max(lo_y));
        }
        if let Some(t) = self.thread_mut(&id) {
            t.tag_offset = (origin.0 + dx, origin.1 + dy);
        }
        cx.notify();
    }

    fn end_tag_drag(&mut self, cx: &mut Context<Self>) {
        if self.drag.take().is_some_and(|d| d.moved) {
            self.last_drag_end = Some(std::time::Instant::now());
            self.schedule_save(cx);
            cx.notify();
        }
    }

    fn drag_moved_recently(&self) -> bool {
        self.drag.as_ref().is_some_and(|d| d.moved) || self.last_drag_end.is_some_and(|t| t.elapsed() < Duration::from_millis(150))
    }

    /// How far each tag should be unfolded: open while pointed at (and a moment after),
    /// while it is the open thread, and while it is being dragged.
    fn unfold_target(&self, th: &Thread) -> f32 {
        let id = th.id.as_str();
        if self.active.as_deref() == Some(id) || self.drag.as_ref().is_some_and(|d| d.id == id && d.moved) {
            return 1.;
        }
        if th.status == ThreadStatus::Resolved {
            return 0.;
        }
        if self.tag_rested(id) {
            return 1.;
        }
        // a tag the pointer just left stays open a moment, unless another one has opened:
        // one tag unfolded at a time
        let lingering = self.hover_left.get(id).is_some_and(|t| t.elapsed() < FOLD_DELAY);
        let other = self.hover_at.keys().any(|o| o != id && self.tag_rested(o));
        if lingering && !other && self.last_typed.is_none_or(|k| k.elapsed() > FOLD_DELAY) { 1. } else { 0. }
    }

    fn toolbar(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.show_toolbar || self.just_my_words {
            return None;
        }
        let ed = self.editor.read(cx);
        let rects = ed.selection_rects();
        let first = rects.first()?;
        let top = first.origin.y;
        let kind = ed.current_line_kind();
        let t = Theme::get(cx).clone();
        let p = t.p;
        let key = |id: &'static str, on: bool| {
            div()
                .id(id)
                .w(px(24.))
                .h(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .cursor_pointer()
                .when(on, |d| d.bg(p.ink).text_color(p.paper))
                .when(!on, |d| d.text_color(p.graphite).hover(move |s| s.bg(p.hover).text_color(p.ink)))
                .active(|s| s.opacity(0.7))
        };
        let block = |id: &'static str, label: &'static str, name: &'static str, k: BlockKind, cx: &mut Context<Self>| {
            let glyph = matches!(k, BlockKind::Quote | BlockKind::Bullet | BlockKind::Check(_));
            // ⌘L makes a line a checklist item, so that key says so
            let keys: Option<&'static dyn Action> = matches!(k, BlockKind::Check(_)).then_some(&crate::editor::ToggleCheck);
            key(id, kind.same_kind(k)).text_size(px(if glyph { ts::BODY } else { ts::META })).tooltip(key_tip(name, keys)).child(label).on_click(cx.listener(move |this, _, _, cx| {
                this.editor.update(cx, |e, cx| e.set_selection_kind(k, cx));
                this.show_toolbar = true;
                cx.notify();
            }))
        };
        // Each mark's button wears the mark itself: B is bold, I italic, and so on.
        let mark = |id: &'static str, label: &'static str, name: &'static str, keys: &'static dyn Action, m: Mark, cx: &mut Context<Self>| {
            let on = self.editor.read(cx).has_mark(m);
            key(id, on)
                .tooltip(key_tip(name, Some(keys)))
                .text_size(px(ts::UI))
                .map(|d| match m {
                    Mark::Bold => d.font_weight(FontWeight::BOLD),
                    Mark::Italic => d.italic().font_family(crate::theme::SERIF),
                    Mark::Underline => d.underline(),
                    Mark::Strike => d.line_through(),
                    _ => d.text_size(px(ts::META)),
                })
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.editor.update(cx, |e, cx| e.toggle_mark(m, cx));
                    this.show_toolbar = true;
                    cx.notify();
                }))
        };
        let mut colors = div().flex().items_center().gap(px(3.));
        for (i, (hex, _, name)) in TEXT_COLORS.iter().enumerate() {
            let c = crate::theme::text_color(*hex, &p);
            let value = *hex;
            let on = value != 0 && self.editor.read(cx).has_mark(Mark::Color(value));
            colors = colors.child(
                div()
                    .id(SharedString::from(format!("color-{i}")))
                    .w(px(22.))
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .cursor_pointer()
                    // the colour your words wear is filled in, as every current key is
                    .when(on, |d| d.bg(c))
                    .when(!on, |d| d.hover(move |s| s.bg(p.hover)))
                    .active(|s| s.opacity(0.7))
                    .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(*name).build(window, cx))
                    .child(div().text_size(px(ts::UI)).font_weight(FontWeight::SEMIBOLD).text_color(if on { p.paper } else { c }).child("A"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.editor.update(cx, |e, cx| e.toggle_mark(Mark::Color(value), cx));
                        this.show_toolbar = true;
                        cx.notify();
                    })),
            );
        }
        let find_keys: SharedString = crate::shortcuts::keys_for("app::FindLooseThread", cx).unwrap_or_default().into();
        let own_keys: SharedString = crate::shortcuts::keys_for("app::AskYourself", cx).unwrap_or_default().into();
        let action = |id: &'static str, title: SharedString, hint: SharedString| {
            div()
                .id(id)
                .mx(px(4.))
                .px(px(8.))
                .py(px(5.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .active(|s| s.opacity(0.8))
                // ("Ask" and a mind's long name is cut short, and the hint after it)
                .child(div().flex_none().max_w(px(150.)).truncate().text_size(px(ts::UI)).text_color(p.ink).child(title))
                .child(div().min_w(px(0.)).truncate().text_size(px(ts::META)).text_color(p.faint).child(hint))
        };
        let ask = SharedString::from(format!("Ask {}", self.mind.name));
        let card = ui::menu_card(&t)
            .w(px(236.))
            .child(
                div()
                    .px(px(5.))
                    .flex()
                    .gap(px(2.))
                    .child(block("tb-p", "T", "Text", BlockKind::Paragraph, cx))
                    .child(block("tb-h1", "H1", "Heading 1", BlockKind::H1, cx))
                    .child(block("tb-h2", "H2", "Heading 2", BlockKind::H2, cx))
                    .child(block("tb-h3", "H3", "Heading 3", BlockKind::H3, cx))
                    .child(block("tb-q", "\u{201c}", "Quote", BlockKind::Quote, cx))
                    .child(block("tb-ul", "\u{2022}", "Bulleted list", BlockKind::Bullet, cx))
                    .child(block("tb-ol", "1.", "Numbered list", BlockKind::Numbered, cx))
                    .child(block("tb-check", "\u{2610}", "Checklist", BlockKind::Check(false), cx)),
            )
            .child(self.sewn_keys(kind, cx))
            .child(
                div()
                    .px(px(5.))
                    .pt(px(2.))
                    .flex()
                    .gap(px(2.))
                    .child(mark("tb-b", "B", "Bold", &crate::editor::ToggleBold, Mark::Bold, cx))
                    .child(mark("tb-i", "I", "Italic", &crate::editor::ToggleItalic, Mark::Italic, cx))
                    .child(mark("tb-u", "U", "Underline", &crate::editor::ToggleUnderline, Mark::Underline, cx))
                    .child(mark("tb-s", "S", "Strikethrough", &crate::editor::ToggleStrike, Mark::Strike, cx))
                    .child(mark("tb-c", "</>", "Inline code", &crate::editor::ToggleCode, Mark::Code, cx)),
            )
            .child(div().px(px(5.)).pt(px(2.)).child(colors))
            .child(div().mx(px(9.)).my(px(5.)).border_t_1().border_color(p.rule))
            .child(action("tb-riddle", "Find a loose thread".into(), find_keys).on_click(cx.listener(|this, _, window, cx| this.find_loose_thread(window, cx))))
            .child(action("tb-ask", ask, "Answered in the margin".into()).on_click(cx.listener(|this, _, window, cx| this.start_ask(window, cx))))
            .child(action("tb-own", "Ask yourself".into(), own_keys).on_click(cx.listener(|this, _, window, cx| this.ask_yourself(window, cx))))
            .child(action("tb-viz", "Sketch it".into(), "Stitched step by step".into()).on_click(cx.listener(|this, _, window, cx| this.sketch_selection(window, cx))));
        Some(
            deferred(div().absolute().top(top - px(6.)).left(px(self.m.popover_left(236.))).occlude().child(card))
                .with_priority(1)
                .into_any_element(),
        )
    }

    /// The selection menu's row of sewn blocks: a callout of each kind, named in its
    /// thread, and a block of code.
    fn sewn_keys(&self, kind: BlockKind, cx: &mut Context<Self>) -> AnyElement {
        use crate::store::{Callout, Lang};
        let p = Theme::get(cx).p;
        let mut row = div().px(px(5.)).pt(px(2.)).flex().gap(px(2.));
        let keys = Callout::ALL
            .iter()
            .map(|c| (*c, BlockKind::Callout(*c)))
            .map(|(c, k)| (c.label(), k, crate::sewn::callout_thread(c, &p), crate::theme::SANS))
            .chain(std::iter::once(("Code", BlockKind::Code(Lang::default()), p.graphite, crate::theme::MONO)));
        for (label, k, color, family) in keys {
            // "Code" here is a block of code; </> beside B and I is code in a line
            let name = if k.is_code() { "Code block" } else { "Callout" };
            let on = kind.same_kind(k);
            row = row.child(
                div()
                    .id(SharedString::from(format!("tb-{}", label.to_lowercase())))
                    .h(px(22.))
                    .px(px(7.))
                    .flex()
                    .items_center()
                    .rounded(px(5.))
                    .cursor_pointer()
                    .font_family(family)
                    .text_size(px(ts::SMALL))
                    .font_weight(FontWeight::MEDIUM)
                    // the current block's key is filled with its thread, as T is with the ink
                    .when(on, |d| d.bg(color).text_color(p.paper))
                    .when(!on, |d| d.text_color(color).hover(move |s| s.bg(p.hover)))
                    .active(|s| s.opacity(0.7))
                    .tooltip(key_tip(name, None))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.editor.update(cx, |e, cx| e.set_selection_kind(k, cx));
                        this.show_toolbar = true;
                        cx.notify();
                    })),
            );
        }
        row.into_any_element()
    }

    fn ask_box(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let asking = self.asking.as_ref()?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        let ed = self.editor.read(cx);
        let top = ed.layout.as_ref().and_then(|l| l.rects(asking.range.clone()).first().map(|r| r.origin.y)).unwrap_or(px(0.));
        let input = asking.input.clone();
        let room: Vec<String> = asking.room.iter().map(|m| m.name.clone()).collect();
        let candidates = self.room_candidates(cx);
        let own = self.mind.name.clone();
        let chosen = cx.global::<Store>().settings.ink;
        let full = room.len() >= ROOM_MAX;
        // Ask the room: up to three minds answer the same question side by side.
        let mut chips = div().flex().flex_wrap().gap(px(6.)).mt(px(10.));
        for mind in candidates {
            let on = room.contains(&mind.name);
            // A fourth mind cannot sit down until one of the three gets up.
            let off = full && !on;
            let color = mind_thread(&mind.name, &own, chosen, &t);
            let name = mind.name.clone();
            chips = chips.child(
                div()
                    .id(SharedString::from(format!("room-{}", name)))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .px(px(9.))
                    .py(px(3.))
                    .rounded_full()
                    .border_1()
                    .border_color(if on { color.opacity(0.7) } else { p.rule })
                    .bg(if on { color.opacity(0.08) } else { p.card })
                    .text_size(px(ts::SMALL))
                    .text_color(if on { p.ink } else { p.graphite })
                    .when(off, |d| d.opacity(0.45))
                    .when(!off, |d| {
                        d.cursor_pointer()
                            .hover(move |s| s.border_color(if on { color } else { p.faint }).text_color(p.ink))
                            .active(|s| s.opacity(0.8))
                            .keyed_as(format!("ask:room-{name}"), 12., color, cx, move |this, _, cx| this.toggle_room(mind.clone(), cx))
                    })
                    .child(ui::small_dot(color, 6.))
                    // a mind with a long name keeps its chip inside the box
                    .max_w_full()
                    .child(div().min_w(px(0.)).truncate().child(name)),
            );
        }
        let n = room.len();
        let hint = match n {
            0 => "No mind answers. Add one to ask it.".to_string(),
            1 => "Add minds to ask the room".to_string(),
            n => format!("{n} minds answer side by side"),
        };
        let dot = if n == 0 { own::ink(&t) } else { t.voice_ink };
        Some(
            deferred(
                div()
                    .absolute()
                    .tab_group()
                    .tab_index(ui::tab::ASK)
                    .top(top - px(4.))
                    .left(px(self.m.popover_left(self.m.margin.max(236.))))
                    .w(px(self.m.margin.max(236.)))
                    .occlude()
                    .child(
                        ui::menu_card(&t)
                            .px(px(12.))
                            .py(px(10.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(ui::small_dot(dot, 6.))
                                    .child(div().flex_1().child(Input::new(&input).appearance(false).px_0().text_size(px(ts::BODY)))),
                            )
                            .child(chips)
                            .child(
                                div()
                                    .mt(px(10.))
                                    .flex()
                                    .justify_between()
                                    .gap(px(8.))
                                    .text_size(px(ts::META))
                                    .text_color(p.faint)
                                    // a long hint wraps rather than push the key off the box
                                    .child(div().flex_1().min_w(px(0.)).child(hint))
                                    .child(div().flex_none().child("\u{21a9} ask")),
                            ),
                    )
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.asking = None;
                        cx.notify();
                    })),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }

    fn chat_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let title = self.editor.read(cx).title();
        let visible = self.note.visible_threads();
        let open = self.note.open_threads();
        let empty = self.note.chat.is_empty() && self.chat_pending.is_none();
        let busy = self.chat_pending.is_some();
        let can_send = !busy && !self.chat_input.read(cx).value().trim().is_empty();
        let mut messages = div().flex().flex_col().gap(px(14.)).px(px(16.)).py(px(12.));
        if empty {
            messages = messages
                .child(
                    div()
                        .font_family(t.voice_font.clone())
                        .text_size(px(ts::TITLE))
                        .line_height(px(21.))
                        .text_color(p.ink)
                        .child(if visible == 0 {
                            "I’ve read the whole note. Ask me anything about it."
                        } else {
                            "I’ve read the whole note and its threads. Ask me anything about it."
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(6.))
                        .children(
                            ["What am I still misunderstanding?", "What have we figured out so far?", "What should I work on next?"]
                                .into_iter()
                                .enumerate()
                                .map(|(i, q)| {
                                    ui::pill(SharedString::from(format!("suggest-{i}")), q, false, &t)
                                        .keyed_as(format!("chat:suggest-{i}"), 12., t.voice_ink, cx, move |this, window, cx| this.send_chat(Some(q.to_string()), window, cx))
                                }),
                        ),
                );
        }
        for (i, m) in self.note.chat.iter().enumerate() {
            messages = messages.child(match m.role {
                Role::You => div()
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .max_w(px(236.))
                            .px(px(10.))
                            .py(px(6.))
                            .rounded(px(10.))
                            .bg(p.shelf)
                            .text_size(px(ts::UI))
                            .line_height(px(18.))
                            .text_color(p.ink)
                            .child(m.text.clone()),
                    )
                    .into_any_element(),
                Role::Mind => div()
                    .id(SharedString::from(format!("chat-{i}")))
                    .font_family(t.voice_font.clone())
                    .text_size(px(ts::BODY))
                    .line_height(px(20.))
                    .text_color(p.ink)
                    .child(m.text.clone())
                    .into_any_element(),
            });
        }
        if let Some((_, partial)) = &self.chat_pending {
            if partial.is_empty() {
                messages = messages.child(div().flex().child(Self::thinking_dot(&t, "chat-thinking")));
            } else {
                messages = messages.child(
                    div()
                        .font_family(t.voice_font.clone())
                        .text_size(px(ts::BODY))
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .child(partial.clone()),
                );
            }
        }
        if let Some(err) = self.chat_error.clone() {
            messages = messages.child(
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(4.))
                    .text_size(px(ts::SMALL))
                    .line_height(px(17.))
                    .text_color(p.graphite)
                    .child(err)
                    .child(ui::ghost_button("chat-retry", "Try again", &t).keyed_as("chat:retry", 4., t.voice_ink, cx, |this, window, cx| this.retry_chat(window, cx))),
            );
        }
        let input = self.chat_input.clone();
        let title = if title.trim().is_empty() { "Untitled".to_string() } else { title };
        let threads = match (visible, open) {
            (0, _) => "no threads yet".to_string(),
            (v, o) => format!("{v} thread{}, {o} open", if v == 1 { "" } else { "s" }),
        };
        div()
            .w(px(CHAT_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .tab_group()
            .tab_index(ui::tab::CHAT)
            .border_l_1()
            .border_color(p.rule)
            .bg(p.paper)
            .font_family(SANS)
            .child(
                div()
                    .px(px(16.))
                    // its row centred on the traffic lights' line, where Chat was
                    .pt(px(10.))
                    .pb(px(12.))
                    .child(
                        div()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(ui::small_dot(t.voice_ink, 7.))
                                    .child(div().text_size(px(ts::BODY)).font_weight(FontWeight::MEDIUM).text_color(p.ink).truncate().child(self.mind.name.clone())),
                            )
                            .map(|d| {
                                if self.note.chat.is_empty() {
                                    d.child(ui::ghost_button_off("Copy", &t)).child(ui::ghost_button_off("Clear", &t))
                                } else {
                                    d.child(ui::ghost_button("chat-copy", "Copy", &t).keyed_as("chat:copy", 4., t.voice_ink, cx, |this, _, cx| {
                                        let text = this
                                            .note
                                            .chat
                                            .iter()
                                            .map(|m| format!("{}: {}", if m.role == Role::You { "Me" } else { this.mind.name.as_str() }, m.text))
                                            .collect::<Vec<_>>()
                                            .join("\n\n");
                                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                                        cx.emit(PageEvent::Toast("Chat copied".into()));
                                    }))
                                    .child(ui::ghost_button("chat-clear", "Clear", &t).keyed_as("chat:clear", 4., t.voice_ink, cx, |this, _, cx| this.clear_chat(cx)))
                                }
                            })
                            .child(div().mr(px(-4.)).child(ui::icon_button("chat-close", "x", 13., &t).tooltip(crate::app::tip("Close", Some(&crate::app::ToggleChat))).keyed_as("chat:close", 5., t.voice_ink, cx, |this, window, cx| this.toggle_chat(window, cx)))),
                    )
                    .child(
                        div()
                            .pl(px(14.))
                            .text_size(px(ts::META))
                            .text_color(p.faint)
                            .truncate()
                            .child(format!("On \u{201c}{title}\u{201d} \u{b7} {threads}")),
                    ),
            )
            .child(div().id("chat-scroll").flex_1().overflow_y_scroll().track_scroll(&self.chat_scroll).child(messages))
            .child(
                div()
                    .border_t_1()
                    .border_color(p.rule)
                    .px(px(14.))
                    .py(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().child(Input::new(&input).appearance(false).px_0().text_size(px(ts::BODY))))
                    .child(
                        div()
                            .id("chat-send")
                            .size(px(24.))
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(p.ink)
                            .map(|d| {
                                if can_send {
                                    d.cursor_pointer()
                                        .hover(|s| s.opacity(0.88))
                                        .active(|s| s.opacity(0.75))
                                        .on_click(cx.listener(|this, _, window, cx| this.send_chat(None, window, cx)))
                                } else {
                                    d.opacity(0.3)
                                }
                            })
                            .child(icon("arrow-up", 12., p.paper)),
                    ),
            )
            .into_any_element()
    }

    fn enlarged_drawing(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = self.enlarged.clone()?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        let block = self.drawing_view(&id, 460., true, cx)?;
        let close = |this: &mut NotePage, cx: &mut Context<NotePage>| {
            this.enlarged = None;
            cx.notify();
        };
        Some(
            deferred(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(p.dim)
                    .occlude()
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| close(this, cx)))
                    .child(
                        div()
                            .id("enlarged")
                            .w(px(520.))
                            .px(px(30.))
                            .pt(px(14.))
                            .pb(px(24.))
                            // a dialog's corner, as ⌘K and Export have it
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.rule)
                            .shadow(ui::modal_shadow(&p))
                            .font_family(SANS)
                            // A press inside is for the sketch and its controls, not a
                            // press outside that closes it.
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                div()
                                    .mr(px(-16.))
                                    .flex()
                                    .justify_end()
                                    .child(ui::icon_button("enlarged-close", "x", 14., &t).tooltip(crate::app::tip("Close", None)).keyed_as("sketch:close", 5., t.voice_ink, cx, move |this, _, cx| close(this, cx))),
                            )
                            .child(block),
                    ),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }
}

type AnchorBoxes = HashMap<String, Vec<Bounds<Pixels>>>;

impl NotePage {
    /// The threads shown in the margin, where their cards go, and the boxes of
    /// their anchored words (relative to the editor).
    fn margin_layout(&self, placement: Placement, m: Metrics, cx: &App) -> (Vec<Thread>, Vec<margin::Slot>, AnchorBoxes) {
        let ed = self.editor.read(cx);
        let mut items = vec![];
        let threads: Vec<Thread> = self
            .note
            .threads
            .iter()
            .filter(|t| {
                t.status == ThreadStatus::Open
                    || (t.status == ThreadStatus::Resolved && (self.active.as_deref() == Some(&t.id) || self.tying.contains_key(&t.id)))
            })
            .cloned()
            .collect();
        let mut anchor_boxes: AnchorBoxes = HashMap::new();
        for th in &threads {
            let rects = ed.anchor_rects(&th.id);
            let Some(first) = rects.first().copied() else { continue };
            // One margin (the setting, or a window too narrow for two): all on the right.
            let side = if placement == Placement::Both && m.both { th.side } else { Side::Right };
            let visible = match placement {
                Placement::Marks => self.active.as_deref() == Some(th.id.as_str()) || self.marks_open.contains(&th.id),
                _ => true,
            };
            if visible {
                let start = ed.anchors().iter().find(|a| a.id == th.id).map_or(usize::MAX, |a| a.range.start);
                items.push((start, th.id.clone(), side, first.origin.y));
            }
            anchor_boxes.insert(th.id.clone(), rects);
        }
        // Tags on one line stack in reading order, as they are numbered (`place` keeps
        // this order among tags level with each other); threads on the same words keep
        // the order they were made in.
        items.sort_by_key(|(start, ..)| *start);
        let items = items.into_iter().map(|(_, id, side, y)| (id, side, y)).collect();
        let slots = margin::place(items, &self.heights.borrow(), self.active.as_deref(), px(CARD_GAP));
        (threads, slots, anchor_boxes)
    }
}

impl NotePage {
    /// Where a thread comes up through the cloth and where it ties onto its tag. It
    /// never runs across words on the front: it surfaces beside the column, level with its
    /// first stitched row; on the left, past the gutter, so it never crosses the
    /// paragraph's knot and number.
    fn thread_ends(&self, s: &margin::Slot, rects: &[Bounds<Pixels>], m: Metrics, _cx: &App) -> (Point<Pixels>, Point<Pixels>) {
        let col = px(m.column_left());
        let (dx, dy) = self.note.threads.iter().find(|t| t.id == s.id).map(|t| t.tag_offset).unwrap_or((0., 0.));
        // the loop sits 13 px into the tab (whose top is 3 px above the slot), plus the drag
        let tag_y = s.top + px(10. + dy);
        // The first stitched row is the one level with the tag (tags line up with the
        // first row of their words), and the edge of the column is the nearest point to
        // the margin that crosses no words.
        let first = rects[0];
        let y = first.origin.y + first.size.height - px(3.2);
        match s.side {
            Side::Right => (point(col + px(m.column) + px(9.), y), point(px(m.right_left()) + px(13. + dx), tag_y)),
            Side::Left => (point(col - px(GUTTER_CLEAR), y), point(px(m.margin - LEFT_TAB_OUT) - px(13. - dx), tag_y)),
        }
    }
}

impl Render for NotePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let placement = cx.global::<Store>().settings.placement;
        let viewport = window.viewport_size();
        self.viewport = viewport;
        self.finish_goto(window, cx);
        let width = viewport.width.as_f32() - if self.chat_open { CHAT_WIDTH } else { 0. };
        // The page sits on the linen with a strip of cloth showing on each side.
        self.m = Metrics::for_width(width - 2. * (SHEET_PAD + 14.));
        let m = self.m;
        let now = std::time::Instant::now();
        let dt = self.last_frame.map(|l| anim_since(l, now)).unwrap_or(1. / 60.).clamp(1. / 2400., 1. / 30.);
        self.last_frame = Some(now);
        self.tying.retain(|_, at| anim_since(*at, now) < TIE_SHOW.as_secs_f32());
        // Questions that waited while you typed open once you pause.
        if !self.typing_recently() {
            self.arrived.clear();
        }
        // a thread between notes: its far end's side the first time it shows here
        self.side_new_far_ends(cx);

        // The paragraph being read, and words being looked at again: the needle works beside them.
        let thinking: Vec<usize> = {
            let ed = self.editor.read(cx);
            let mut at: Vec<usize> = self.reading.as_ref().filter(|r| r.line < ed.line_count()).map(|r| ed.line_range(r.line).start).into_iter().collect();
            at.extend(ed.anchors().iter().filter(|a| self.rechecking.contains(&a.id)).map(|a| a.range.start));
            at
        };
        let jmw = self.just_my_words;
        let thread_color = t.voice_ink;
        let own_mind = self.mind.name.clone();
        let chosen_ink = cx.global::<Store>().settings.ink;
        let color_of = |th: &Thread| if th.own { own::ink(&t) } else { mind_thread(&th.mind_name, &own_mind, chosen_ink, &t) };

        // How each thread's words are stitched, the open ones numbered in reading
        // order, and the gutter: a tacking stitch for read paragraphs, a knot and its
        // number for a paragraph with a question.
        let (styles, gutter, numbers) = {
            let ed = self.editor.read(cx);
            let mut styles = HashMap::new();
            let mut open: Vec<(usize, String, Hsla)> = vec![];
            for th in &self.note.threads {
                if th.status == ThreadStatus::Dismissed {
                    continue;
                }
                let color = color_of(th);
                styles.insert(th.id.clone(), AnchorStyle { color, correction: th.kind == ThoughtKind::Correction, own: th.own });
                if th.status == ThreadStatus::Open {
                    if let Some(a) = ed.anchors().iter().find(|a| a.id == th.id) {
                        open.push((a.range.start, th.id.clone(), color));
                    }
                }
            }
            // reading order; threads on the same words keep the order they were made in,
            // which is also how their tags stack in the margin
            open.sort_by_key(|a| a.0);
            let numbers: HashMap<String, usize> = open.iter().enumerate().map(|(i, (_, id, _))| (id.clone(), i + 1)).collect();
            let mut gutter = vec![];
            for line in 1..ed.line_count() {
                let range = ed.line_range(line);
                if let Some(n) = open.iter().position(|(s, _, _)| *s >= range.start && *s <= range.end) {
                    gutter.push((line, GutterMark::Question { number: n + 1, color: open[n].2 }));
                    continue;
                }
                let unit = ed.unit_of(line);
                if unit.start != line {
                    continue;
                }
                let text = ed.lines_text(unit);
                let text = text.trim();
                if !text.is_empty() && self.note.read_hashes.contains(&hash_text(text)) {
                    gutter.push((line, GutterMark::Read));
                }
            }
            (styles, gutter, numbers)
        };
        self.numbers = numbers;
        let hint = self.note.first_run.then(|| "One sentence about something you\u{2019}re working on, or something you believe.".to_string());
        self.editor.update(cx, |e, _| {
            e.thinking = thinking;
            e.hide_anchors = jmw;
            e.anchor_styles = styles;
            e.gutter = gutter;
            e.placeholder_hint = hint;
        });

        // Margin tags: folded tabs that drop open on a spring toward their target.
        let (threads, slots, anchor_boxes) = self.margin_layout(placement, m, cx);
        let mut unfolding = false;
        for th in &threads {
            let target = self.unfold_target(th);
            let x = self.unfold.entry(th.id.clone()).or_insert(0.);
            let v = self.unfold_v.entry(th.id.clone()).or_insert(0.);
            if (*x - target).abs() < 0.002 && v.abs() < 0.02 {
                *x = target;
                *v = 0.;
                continue;
            }
            let (k, c) = if target > 0.5 { DROP_OPEN } else { DROP_CLOSE };
            // small steps, so a slow frame does not throw the spring
            let steps = (dt * 240.).ceil().max(1.);
            let h = dt / steps;
            for _ in 0..steps as usize {
                *v += (k * (target - *x) - c * *v) * h;
                *x += *v * h;
            }
            unfolding = true;
        }
        self.unfold.retain(|id, _| threads.iter().any(|t| &t.id == id));
        self.unfold_v.retain(|id, _| threads.iter().any(|t| &t.id == id));
        // tabs and panels drawn this frame put their bounds back as they paint
        self.tab_bounds.borrow_mut().retain(|id, _| !jmw && slots.iter().any(|s| &s.id == id));
        self.panel_bounds.borrow_mut().retain(|id, _| !jmw && self.unfold.get(id).is_some_and(|u| *u > 0.01));
        // a pointer resting on a tag, or one that just left, changes it without an event
        if self.hover_left.values().any(|t| t.elapsed() < FOLD_DELAY) || self.hover_at.values().any(|t| t.elapsed() < HOVER_REST) {
            unfolding = true;
        }
        self.hover_left.retain(|_, t| t.elapsed() < FOLD_DELAY * 4);
        if unfolding {
            window.request_animation_frame();
        }
        let owner: AnyView = cx.entity().into();
        let mut cards = vec![];
        if !jmw {
            for slot in &slots {
                let Some(th) = threads.iter().find(|t| t.id == slot.id).cloned() else { continue };
                let card = self.card(&th, slot.side, cx);
                let left = if slot.side == Side::Left { px(0.) } else { px(m.right_left()) };
                let (dx, dy) = th.tag_offset;
                cards.push(
                    div()
                        .absolute()
                        .top(slot.top - px(3.) + px(dy))
                        .left(left + px(dx))
                        .w(px(self.m.margin))
                        .child(margin::measured(slot.id.clone(), card, self.heights.clone(), owner.clone()))
                        .into_any_element(),
                );
            }
        }

        // Marks-only placement: a small ? after each quote opens its tag; a correction gets
        // the × its tab would show, and a thread being tied off its knot.
        let mut marks = vec![];
        if placement == Placement::Marks && !jmw {
            for th in &threads {
                let Some(rects) = anchor_boxes.get(&th.id) else { continue };
                let Some(last) = rects.last() else { continue };
                let id = th.id.clone();
                let color = color_of(th);
                let glyph = if th.status == ThreadStatus::Resolved {
                    div().size(px(5.)).rounded_full().bg(color).into_any_element()
                } else {
                    mark_glyph(th).into_any_element()
                };
                // Set as a superscript: small, raised above the x-height, in the space
                // after the words, so it never sits on the next word's letters.
                marks.push(
                    div()
                        .id(SharedString::from(format!("mark-{id}")))
                        .absolute()
                        .top(last.origin.y - px(1.))
                        .left(px(m.column_left()) + last.origin.x + last.size.width - px(1.))
                        .h(px(14.))
                        .px(px(2.))
                        .flex()
                        .items_center()
                        .rounded(px(3.))
                        .font_family(SANS)
                        .text_size(px(ts::META))
                        .font_weight(FontWeight::BOLD)
                        .text_color(color)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .child(glyph)
                        .on_click(cx.listener(move |this, _, window, cx| this.toggle_mark(&id, window, cx)))
                        .into_any_element(),
                );
            }
        }

        // The threads: one rope per visible tag, kept between frames.
        let mut ropes: Vec<(Rope, f32, Option<f32>, Hsla)> = vec![];
        let mut holes: Vec<(Point<Pixels>, f32)> = vec![];
        if !jmw && placement != Placement::Marks {
            let mut alive = HashSet::new();
            for s in &slots {
                let Some(rects) = anchor_boxes.get(&s.id) else { continue };
                if rects.is_empty() {
                    continue;
                }
                let Some(th) = threads.iter().find(|t| t.id == s.id) else { continue };
                let (a, b) = self.thread_ends(s, rects, m, cx);
                alive.insert(s.id.clone());
                let rope = self.ropes.entry(s.id.clone()).or_insert_with(|| Rope::new(a, b));
                let dragging = self.drag.as_ref().is_some_and(|d| d.id == s.id && d.moved);
                let animate = rope.moving || dragging;
                rope.set_ends(a, b, animate);
                if th.status == ThreadStatus::Resolved && !rope.is_tied() && !rope.moving {
                    rope.tie();
                }
                if rope.moving {
                    rope.advance(dt);
                }
                let since = self.tying.get(&s.id).map(|at| anim_since(*at, now));
                let knot = (th.status == ThreadStatus::Resolved).then(|| knot_scale(since.unwrap_or(10.) - 0.34));
                // A thread is quiet until you look at it: faint at rest, stronger while its
                // tag unfolds under the pointer, full strength while it is open, and while it
                // is being tied off.
                let unfolded = self.unfold.get(&s.id).copied().unwrap_or(0.);
                let opacity = if th.status == ThreadStatus::Resolved {
                    tag_fade(since)
                } else if self.active.as_deref() == Some(s.id.as_str()) {
                    1.
                } else {
                    0.22 + 0.4 * unfolded
                };
                ropes.push((rope.clone(), opacity, knot, color_of(th)));
                holes.push((a, opacity));
            }
            self.ropes.retain(|id, _| alive.contains(id));
        }
        for (r, _, _) in self.snipped.iter_mut() {
            r.advance(dt);
        }
        self.snipped.retain(|(_, _, at)| anim_since(*at, now) < SNIP_FADE.as_secs_f32());
        let falling: Vec<(Rope, Hsla, f32)> = self
            .snipped
            .iter()
            .map(|(r, c, at)| (r.clone(), *c, 1. - anim_since(*at, now) / SNIP_FADE.as_secs_f32()))
            .collect();
        if self.ropes.values().any(|r| r.moving) || !self.snipped.is_empty() || !self.tying.is_empty() {
            window.request_animation_frame();
        }
        let shadow = p.shadow;
        let hole_color = p.graphite.opacity(0.5);
        let tethers = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                for (at, opacity) in &holes {
                    stitch::hole(window, bounds.origin + *at, hole_color.opacity(*opacity));
                }
                for (rope, opacity, knot, color) in &ropes {
                    stitch::paint_rope(window, bounds.origin, rope, *color, shadow, *opacity);
                    if let Some(k) = knot {
                        stitch::french_knot(window, bounds.origin + rope.end(), px(4.4), color.opacity(*opacity), *k);
                    }
                }
                for (rope, color, opacity) in &falling {
                    stitch::paint_rope(window, bounds.origin, rope, *color, shadow, opacity.max(0.));
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        // The sampler border along the top of the page: one cross-stitched motif for
        // every thread tied off in this note.
        let tied_count = self.note.threads.iter().filter(|t| t.status == ThreadStatus::Resolved).count();
        let newest = self.tying.values().map(|at| anim_since(*at, now)).fold(None, |acc: Option<f32>, s| Some(acc.map_or(s, |a| a.min(s))));
        let border_line = p.faint.opacity(0.42);
        let border = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let y = bounds.origin.y + px(24.);
                let x1 = bounds.origin.x + bounds.size.width - px(SHEET_PAD);
                let mut x = bounds.origin.x + px(SHEET_PAD + 9.);
                for i in 0..tied_count {
                    let scale = if i + 1 == tied_count { newest.map_or(1., |s| ((s - 0.55) / 0.4).clamp(0., 1.)) } else { 1. };
                    stitch::border_motif(window, point(x, y), thread_color, scale);
                    x += px(24.);
                }
                let start = if tied_count > 0 { x - px(6.) } else { bounds.origin.x + px(SHEET_PAD) };
                stitch::running_stitch(window, start, x1, y, border_line, 7, false);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .w_full()
        .h(px(44.));

        // The linen around the page: the only place the weave shows.
        let weave_color = p.weave;
        let weave = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let step = px(3.);
                let mut y = bounds.origin.y;
                while y < bounds.origin.y + bounds.size.height {
                    window.paint_quad(fill(Bounds::new(point(bounds.origin.x, y), size(bounds.size.width, px(1.))), weave_color));
                    y += step;
                }
                let mut x = bounds.origin.x;
                while x < bounds.origin.x + bounds.size.width {
                    window.paint_quad(fill(Bounds::new(point(x, bounds.origin.y), size(px(1.), bounds.size.height)), weave_color));
                    x += step;
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let header = self.header(cx);
        let brief = self.brief_line(cx);
        let menus = self.menus(cx);
        let toolbar = self.toolbar(cx);
        let ask = self.ask_box(cx);
        let enlarged = self.enlarged_drawing(cx);
        let chat = self.chat_open.then(|| self.chat_panel(cx));
        let knots = if jmw { vec![] } else { self.answer_knots(m, cx) };
        let body = div()
            .relative()
            .w(px(m.page()))
            .child(tethers)
            .child(div().ml(px(m.column_left())).w(px(m.column)).tab_group().tab_index(ui::tab::NOTE).child(self.editor.clone()).child({
                let column = self.column_bounds.clone();
                canvas(move |b, _, _| column.set(Some(b)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
            }))
            .children(cards)
            .children(marks)
            .children(knots)
            .children(toolbar)
            .children(ask);

        let page = div()
            .relative()
            .w(px(m.page()))
            .flex()
            .flex_col()
            .child(div().relative().ml(px(m.column_left())).w(px(m.column)).tab_group().tab_index(ui::tab::HEADER).child(header).when(!jmw, |d| d.children(brief)))
            .children(menus)
            .child(div().h(px(18.)))
            .child(body)
            .child(div().h(px(220.)));
        // Turned over, the sheet holds the back instead (`back.rs`), and either side fades
        // while the sheet turns; the back shows from its top.
        let turn = self.back.turn_now(|at| anim_since(at, now), cx.reduce_motion());
        if self.back.first_drawn(&turn) {
            self.scroll.set_offset(point(px(0.), px(0.)));
        }
        if turn.moving() {
            window.request_animation_frame();
        }
        let page = if turn.back { crate::back::view(self, &own_mind, m.column_left(), m.column, m.page(), cx) } else { page.into_any_element() };
        let page = div().opacity(turn.opacity).when(self.back.shown, |d| d.track_focus(&self.back.focus)).child(page);

        let sheet = div()
            .relative()
            .w(px(m.page() + 2. * SHEET_PAD))
            .min_h(viewport.height - px(SHEET_TOP + SHEET_BOTTOM))
            .px(px(SHEET_PAD))
            .pt(px(50.))
            .bg(p.paper)
            .rounded(px(4.))
            .shadow(vec![
                BoxShadow { color: p.shadow, offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
                BoxShadow { color: p.shadow.opacity(0.6), offset: point(px(0.), px(12.)), blur_radius: px(32.), spread_radius: px(-12.), inset: false },
            ])
            .child(border)
            .child(page)
            .children(turn.fold.map(|at| crate::back::fold_shadow(at, m.page() + 2. * SHEET_PAD, &p)))
            .child({
                let sheet_bounds = self.sheet_bounds.clone();
                canvas(move |b, _, _| sheet_bounds.set(Some(b)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
            });
        let fits = m.page() + 2. * SHEET_PAD <= width - 16.;
        // Scrolled, the sheet slides under the cloth along the top, which lays a soft
        // shadow on it there; the shadow comes in over the first few pixels of scroll.
        let sheet_w = m.page() + 2. * SHEET_PAD;
        let sheet_x = if fits { (width - sheet_w) / 2. } else { 12. };
        let scrolled = (-self.scroll.offset().y.as_f32() / 24.).clamp(0., 1.);
        let hem = (scrolled > 0.).then(|| {
            div()
                .absolute()
                .top(px(CHROME_H))
                .left(px(sheet_x))
                .w(px(sheet_w))
                .h(px(10.))
                .opacity(scrolled)
                .bg(linear_gradient(180., linear_color_stop(p.shadow, 0.), linear_color_stop(p.shadow.opacity(0.), 1.)))
        });

        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(p.cloth)
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                let Some(start) = this.drag.as_ref().map(|d| d.start) else { return };
                if ev.dragging() {
                    this.drag_tag_by(ev.position - start, cx);
                } else {
                    // the button came up outside the window
                    this.end_tag_drag(cx);
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.end_tag_drag(cx)))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .h_full()
                    .min_w(px(0.))
                    .child(weave)
                    // The page scrolls below the band of cloth that holds the traffic lights,
                    // the sidebar toggle and Chat, so no words ever run under them.
                    .child(
                        div()
                            .id("page-scroll")
                            .absolute()
                            .top(px(CHROME_H))
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .pt(px(SHEET_TOP - CHROME_H))
                                    .pb(px(SHEET_BOTTOM))
                                    // Centred while it fits; past that keep the column in view.
                                    .map(|d| if fits { d.justify_center() } else { d.justify_start().pl(px(12.)) })
                                    .child(sheet),
                            ),
                    )
                    .children(hem)
                    .children(enlarged),
            )
            .children(chat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn inkling_takes_rose_when_the_notes_own_mind_wears_madder() {
        use crate::store::{Appearance, InkColor, Lettering, TextSize};
        let t = Theme::resolve(Appearance::Light, WindowAppearance::Light, InkColor::Ink, Lettering::Own, TextSize::Default);
        let inkling = mind_thread("Inkling", "Coach", InkColor::Ink, &t);
        assert_eq!(inkling, crate::theme::ink_color(InkColor::Red, &t.p));
        assert_ne!(inkling, mind_thread("Socratic tutor", "Coach", InkColor::Ink, &t));
        assert_ne!(inkling, mind_thread("Coach", "Coach", InkColor::Ink, &t));
    }

    #[std::prelude::v1::test]
    fn a_long_quote_makes_a_title_cut_at_a_word() {
        assert_eq!(short_title("  Short quote ", 60), "Short quote");
        let long = "If I hold 5:30 for the whole race, I'll come in just under 50 and then some";
        let t = short_title(long, 60);
        assert_eq!(t, "If I hold 5:30 for the whole race, I'll come in just under\u{2026}");
        assert!(t.chars().count() <= 61);
        assert_eq!(short_title(&"x".repeat(80), 10), format!("{}\u{2026}", "x".repeat(10)));
    }

    #[std::prelude::v1::test]
    fn a_paragraph_is_read_once_it_is_finished() {
        let read = |text: &str, under_caret: bool| judge(text, under_caret, false, false, true, false);
        // Ends a sentence: read even with the caret still on it.
        assert_eq!(read("We reset the counter every minute.", true), Verdict::Read);
        // Still being written.
        assert_eq!(read("We reset the counter every", true), Verdict::Later);
        // The caret moved on: finished without punctuation.
        assert_eq!(read("We reset the counter every", false), Verdict::Read);
        // Too short: wait while it is being written, drop it once left.
        assert_eq!(read("Short one.", true), Verdict::Later);
        assert_eq!(read("Short one.", false), Verdict::Skip);
        assert_eq!(read("", true), Verdict::Skip);
        // Already read, or another read in flight.
        assert_eq!(judge("We reset the counter every minute.", true, true, false, true, false), Verdict::Skip);
        assert_eq!(judge("We reset the counter every minute.", true, false, false, true, true), Verdict::Later);
        // When-you-ask mode stays quiet.
        assert_eq!(judge("We reset the counter every minute.", false, false, false, false, false), Verdict::Skip);
    }

    #[std::prelude::v1::test]
    fn a_call_by_name_waits_for_the_whole_question() {
        let call = |text: &str, under_caret: bool, reads_along: bool| judge(text, under_caret, false, true, reads_along, true);
        assert_eq!(call("Inkling, what about", true, true), Verdict::Later);
        assert_eq!(call("Inkling, what about the cache?", true, true), Verdict::Answer);
        // In either mode, and even while a read is in flight.
        assert_eq!(call("Inkling, what about the cache?", true, false), Verdict::Answer);
        assert_eq!(call("Inkling, summarize", false, false), Verdict::Answer);
        assert_eq!(call("Inkling,", false, true), Verdict::Skip);
    }

    #[std::prelude::v1::test]
    fn calls_by_name_need_the_name_as_a_word() {
        let names = ["inkling".to_string(), "staff engineer".to_string()];
        for yes in [
            "Inkling, is five minutes too long?",
            "inkling: summarize this",
            "Hey Inkling what do you think?",
            "Hey inkling, really?",
            "What do you think @Inkling?",
            "@inkling",
            "Staff engineer, would this page anyone?",
            "cc @staff engineer.",
        ] {
            assert!(calls_by_name(yes, &names), "{yes}");
        }
        for no in [
            "Inklings of an idea.",
            "Hey inklingly bright.",
            "Mail me at me@inkling.dev please.",
            "The inkling, as I see it",
            "Staff engineers, in general, page.",
        ] {
            assert!(!calls_by_name(no, &names), "{no}");
        }
        // An empty mind name must not turn every "hey" or "@" into a call.
        assert!(!calls_by_name("hey there @ noon", &["inkling".into(), "".into()]));
    }

    fn lines_of(text: &str) -> Vec<Range<usize>> {
        line_ranges(text)
    }

    #[std::prelude::v1::test]
    fn thoughts_anchor_in_their_own_paragraph() {
        let text = "Title\nWe know the cache is cold.\nWe think the cache stays warm.\n";
        let lines = lines_of(text);
        assert_eq!(lines.len(), 4);
        assert_eq!(&text[lines[2].clone()], "We think the cache stays warm.");
        let target = "We think the cache stays warm.";
        // The read was sent as line 2, but a line was inserted above meanwhile:
        // the hint now points at the other paragraph, which has the same quote.
        let shifted = format!("Title\n\n{}", &text[6..]);
        let shifted_lines = lines_of(&shifted);
        let r = locate_thought(&shifted, &shifted_lines, 2, target, "the cache").unwrap();
        assert_eq!(&shifted[r.clone()], "the cache");
        assert!(shifted[..r.start].contains("We know the cache is cold."), "anchored in the wrong paragraph");
        // A quote that is not in the paragraph: the whole paragraph is the anchor.
        let r = locate_thought(text, &lines, 2, target, "something else").unwrap();
        assert_eq!(&text[r], target);
        // An empty quote, too.
        let r = locate_thought(text, &lines, 2, target, "").unwrap();
        assert_eq!(&text[r], target);
        // The paragraph was edited while it was read: settle for the quote near it.
        let edited = "Title\nWe know the cache is cold.\nWe think the cache stays warm, mostly.\n";
        let r = locate_thought(edited, &lines_of(edited), 2, target, "stays warm").unwrap();
        assert_eq!(&edited[r], "stays warm");
        // Nothing left to anchor to.
        assert_eq!(locate_thought(edited, &lines_of(edited), 2, target, "gone"), None);
    }

    #[std::prelude::v1::test]
    fn stale_quotes_are_found_nearest_where_they_were() {
        let text = "alpha beta first. alpha beta second.";
        assert_eq!(nearest_match(text, "alpha beta", 26), Some(18));
        assert_eq!(nearest_match(text, "alpha beta", 3), Some(0));
        assert_eq!(nearest_match(text, "", 3), None);
        assert_eq!(nearest_match(text, "gamma", 3), None);
        let s = "café";
        assert_eq!(floor_char_boundary(s, 4), 3);
        assert_eq!(floor_char_boundary(s, 99), s.len());
    }

    #[std::prelude::v1::test]
    fn the_page_never_runs_off_the_window() {
        // Every width the page can get, from a 720 px window with the chat open up: what
        // `render` passes is the window less the chat and the cloth around the sheet.
        let narrowest = 720. - CHAT_WIDTH - 2. * (SHEET_PAD + 14.);
        let mut last: Option<Metrics> = None;
        for w in (narrowest as i32..=2400).map(|w| w as f32) {
            let m = Metrics::for_width(w);
            // `render` centres the sheet when it fits: page + padding <= window - 16
            assert!(m.page() + 2. * SHEET_PAD <= w + 2. * (SHEET_PAD + 14.) - 16., "width {w}: page {} runs off", m.page());
            assert!(m.column >= 220. && m.column <= 760., "width {w}: column {}", m.column);
            assert!(m.margin >= NARROW_MARGIN && m.margin <= 280., "width {w}: margin {}", m.margin);
            assert!(m.gap >= 24., "width {w}: gap {}", m.gap);
            assert!((m.right_left() - (m.column_left() + m.column + m.gap)).abs() < 0.01);
            // a popover the width of the toolbar stays on the sheet
            assert!(m.popover_left(236.) + 236. <= m.page() + SHEET_PAD + 0.01, "width {w}: popover off the sheet");
            // narrowing the window takes from the margin, the gap and the cloth before the
            // words, and never jumps
            if let Some(l) = last.filter(|_| !m.both) {
                assert!(m.column + 0.01 >= l.column || l.both, "width {w}: column {} after {}", m.column, l.column);
                assert!((m.page() - l.page()).abs() <= 1.01 || l.both, "width {w}: page jumped from {} to {}", l.page(), m.page());
            }
            last = Some(m);
        }
        // A 720 px window with the chat open still gets words a line can hold, and a margin
        // a folded tab fits in.
        let tight = Metrics::for_width(narrowest);
        assert!(tight.column >= 220. && tight.margin >= TAB_W);
        // The narrow layout only starts where the one-margin page no longer fits.
        assert_eq!((Metrics::for_width(584. + GUTTER_LEAD).column, Metrics::for_width(584. + GUTTER_LEAD).margin), (300., 200.));
        // Wide: a margin on each side, as before.
        let wide = Metrics::for_width(1320.);
        assert!(wide.both);
        assert!(wide.column > 600., "a laptop window gives the words more than 600 px: {}", wide.column);
        assert_eq!(wide.column_left(), wide.margin + wide.gap);
        // A big screen: the column stops at a readable width.
        assert_eq!(Metrics::for_width(2400.).column, 760.);
        // Narrow, or the chat open on a laptop-sized window: one margin, on the right.
        let narrow = Metrics::for_width(1100. - CHAT_WIDTH);
        assert!(!narrow.both);
        assert_eq!(narrow.column_left(), GUTTER_LEAD);
        assert!(narrow.column >= 400.);
    }

    #[std::prelude::v1::test]
    fn the_gutter_marks_sit_on_the_paper() {
        // The editor draws a paragraph's knot and number 26 px left of the words, about
        // 5 px either side: at every width they keep 8 px of paper from the sheet's edge.
        let narrowest = 720. - CHAT_WIDTH - 2. * (SHEET_PAD + 14.);
        for w in (narrowest as i32..=2400).map(|w| w as f32) {
            let m = Metrics::for_width(w);
            assert!(SHEET_PAD + m.column_left() - 26. - 5. >= 8., "width {w}: the gutter is on the sheet's edge");
        }
    }

    #[std::prelude::v1::test]
    fn no_tag_opens_across_the_words() {
        // The owner's rule: a thread never covers your words unless you dragged its tag onto
        // them. At every window width, with the chat open and closed, a right tab (either
        // layout) and a left tab (two margins) open outward, clear of the column, at least
        // PANEL_FLOOR wide and 8 px inside the window.
        for window in (720..=1800).map(|w| w as f32) {
            for chat in [false, true] {
                let width = window - if chat { CHAT_WIDTH } else { 0. };
                let m = Metrics::for_width(width - 2. * (SHEET_PAD + 14.));
                let sheet_w = m.page() + 2. * SHEET_PAD;
                let sheet_x = if sheet_w <= width - 16. { (width - sheet_w) / 2. } else { 12. };
                let page_x = sheet_x + SHEET_PAD;
                let (col_l, col_r) = (page_x + m.column_left(), page_x + m.column_left() + m.column);
                let full = m.margin.max(PANEL_MIN);
                let tab_l = page_x + m.right_left();
                let (w, open_right) = panel_fit(true, full, window - 8. - tab_l, tab_l + TAB_W - (sheet_x + 8.), false);
                assert!(open_right && tab_l >= col_r && tab_l + w <= window - 8. + 0.01 && w >= PANEL_FLOOR, "{window} px, chat {chat}: a right tag {w} wide, opens right: {open_right}");
                if m.both {
                    let tab_r = page_x + m.margin - LEFT_TAB_OUT;
                    let (w, open_right) = panel_fit(false, full, tab_r - 8., sheet_x + sheet_w - 8. - (tab_r - TAB_W), false);
                    assert!(!open_right && tab_r <= col_l && tab_r - w >= 8. - 0.01 && w >= PANEL_FLOOR, "{window} px, chat {chat}: a left tag {w} wide, opens right: {open_right}");
                }
            }
        }
        // short of room outward it hangs over the sheet's edge before it narrows
        assert_eq!(panel_fit(false, 249., 253., 600., false), (249., false));
        assert_eq!(panel_fit(true, 249., 180., 600., false), (180., true));
        // with no room outward, only a tag you dragged opens back over the words
        assert_eq!(panel_fit(true, 249., 60., 600., true), (249., false));
        assert_eq!(panel_fit(true, 249., 60., 600., false), (PANEL_FLOOR, true));
    }
}

//! One note on the page: the header row (mind, brief, mode), the editor, and the
//! margin where the mind's threads live. Also the reading loop, the selection
//! toolbar, Ask, Visualize, and the chat panel.

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
use crate::theme::{SANS, TEXT_COLORS, Theme};
use crate::ui::{self, icon};

const CARD_GAP: f32 = 14.;
const CHAT_WIDTH: f32 = 310.;
/// The cloth shows around the page: this much page beyond the margins on each side.
const SHEET_PAD: f32 = 28.;
/// A tied-off tag stays this long, so you can watch the thread pull taut and knot.
const TIE_SHOW: Duration = Duration::from_millis(1700);
/// A snipped thread falls and fades over this long.
const SNIP_FADE: Duration = Duration::from_millis(900);
/// How long "Thread dismissed · Undo" stays.
const UNDO_FOR: Duration = Duration::from_secs(8);
/// A question that arrives while you type waits, folded, until you pause this long.
const QUIET_WHILE_TYPING: Duration = Duration::from_millis(2000);
/// The most minds that can answer one question side by side.
const ROOM_MAX: usize = 3;
/// A folded tag: just its tab.
const TAB_W: f32 = 54.;
const TAB_H: f32 = 26.;
/// After the pointer leaves a tag, it stays open this long (moving onto its panel).
const FOLD_DELAY: Duration = Duration::from_millis(220);

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    1. - (1. - t).powi(3)
}

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
fn mind_thread(mind_name: &str, own: &str, chosen: crate::store::InkColor, t: &Theme) -> Hsla {
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
    let pick = known.filter(|c| *c != chosen).unwrap_or_else(|| {
        let others: Vec<C> = [C::Blue, C::Orange, C::Green, C::Purple, C::Brown, C::Red, C::Ink].into_iter().filter(|c| *c != chosen).collect();
        others[(stitch::seed_of(mind_name) % others.len() as u64) as usize]
    });
    crate::theme::ink_color(pick, &t.p)
}

/// `INKLING_SLOWMO=<factor>` slows every thread animation down, for screenshots and the
/// walkthrough video. 1 is real time.
fn slowmo() -> f32 {
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
/// right, so the page never runs off the window.
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
        let column = (avail - gap - margin).clamp(300., 760.);
        Self { column, margin, gap, both: false }
    }
    fn page(&self) -> f32 {
        if self.both { self.margin * 2. + self.gap * 2. + self.column } else { self.column + self.gap + self.margin }
    }
    fn column_left(&self) -> f32 {
        if self.both { self.margin + self.gap } else { 0. }
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
    thread_menu_for: Option<String>,
    brief_input: Option<(Entity<InputState>, Subscription)>,
    pub chat_open: bool,
    chat_input: Entity<InputState>,
    chat_pending: Option<(String, String)>,
    pub just_my_words: bool,
    /// How present the window's controls are, 0..1: the app fades them while you type
    /// and brings them back when the pointer moves. The page's header row follows it.
    pub chrome: Rc<std::cell::Cell<f32>>,
    enlarged: Option<String>,
    error: Option<(String, std::time::Instant)>,
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
    /// A tag being dragged, and when the last drag ended (so its click is ignored).
    drag: Option<TagDrag>,
    last_drag_end: Option<std::time::Instant>,
    /// Natural height of each unfolded tag, measured, to unfold toward.
    panel_heights: Heights,
    /// Each tab's bounds in the window at the last paint: where its panel has room to open.
    tab_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    viewport: Size<Pixels>,
    last_frame: Option<std::time::Instant>,
    _subs: Vec<Subscription>,
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
        subs.push(cx.subscribe_in(&chat_input, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.send_chat(None, window, cx);
            }
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
            thread_menu_for: None,
            brief_input: None,
            chat_open: false,
            chat_input,
            chat_pending: None,
            just_my_words: false,
            enlarged: None,
            error: None,
            marks_open: HashSet::new(),
            m: Metrics::for_width(1320.),
            ropes: HashMap::new(),
            tying: HashMap::new(),
            snipped: vec![],
            numbers: HashMap::new(),
            last_dismissed: None,
            last_typed: None,
            chrome: Rc::new(std::cell::Cell::new(1.)),
            arrived: HashMap::new(),
            rechecks: HashMap::new(),
            rechecking: HashSet::new(),
            settle_task: None,
            pending_room: vec![],
            unfold: HashMap::new(),
            hover_tag: HashSet::new(),
            hover_left: HashMap::new(),
            drag: None,
            last_drag_end: None,
            panel_heights: Rc::new(RefCell::new(HashMap::new())),
            tab_bounds: Rc::new(RefCell::new(HashMap::new())),
            viewport: size(px(1200.), px(800.)),
            last_frame: None,
            _subs: subs,
        };
        page.load_into_editor(cx);
        page
    }

    pub fn note_id(&self) -> &str {
        &self.note.id
    }

    fn load_into_editor(&mut self, cx: &mut Context<Self>) {
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
            .collect();
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
        // The read in flight will never land: leave its paragraph unread.
        if let Some(r) = self.reading.take() {
            self.note.read_hashes.retain(|h| *h != r.hash);
            self.cancel_request(&r.req, cx);
        }
        // Threads still waiting for their first words (a loose thread, a sketch, a
        // call by name) would be saved as empty cards: drop them with their requests.
        for (id, (req, _)) in std::mem::take(&mut self.pending) {
            self.cancel_request(&req, cx);
            self.drop_if_empty(&id, cx);
        }
        self.flush(cx);
        if let Some((req, _)) = self.chat_pending.take() {
            self.cancel_request(&req, cx);
        }
        // A pause timer started on this note must not read the next one.
        self.pause_task = None;
        self.last_edited = None;
        self.waiting.clear();
        self.error = None;
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
        self.heights.borrow_mut().clear();
        self.marks_open.clear();
        self.load_into_editor(cx);
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.focus_editor_at_end(window, cx);
        cx.notify();
    }

    pub fn focus_editor_at_end(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let empty_title = self.editor.read(cx).title().is_empty();
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
                    t.quote = text[a.range.clone()].to_string();
                }
            }
        }
    }

    pub fn flush(&mut self, cx: &mut Context<Self>) {
        self.save_task = None;
        self.sync_from_editor(cx);
        self.note.updated_at = Utc::now();
        let note = self.note.clone();
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
            EditorEvent::Escape => self.on_escape(cx),
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

    fn on_escape(&mut self, cx: &mut Context<Self>) {
        if self.asking.take().is_some() || self.active.take().is_some() || self.show_toolbar {
            self.show_toolbar = false;
            self.editor.update(cx, |e, _| e.active_anchor = None);
            cx.notify();
        }
    }

    fn keep_cursor_visible(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        self.last_typed
    }

    fn typing_recently(&self) -> bool {
        self.last_typed.is_some_and(|t| t.elapsed() < QUIET_WHILE_TYPING)
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
            let Some(th) = self.note.threads.iter().find(|t| t.id == id && t.status == ThreadStatus::Open) else { continue };
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
            "messages": t.messages.iter().map(|m| json!({"role": role_wire(m.role), "text": m.text})).collect::<Vec<_>>(),
            "message": message,
        });
        let tid = id.to_string();
        self.rechecking.insert(tid.clone());
        let req = self.run(input, false, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                page.rechecking.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if let Some(t) = page.thread_mut(&tid) {
                    t.quote = now_text.clone();
                }
                if !body.is_empty() {
                    page.push_message(&tid, Role::Mind, body, cx);
                }
                if v["resolved"].as_bool() == Some(true) {
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

    /// When a read ends, look at the paragraphs that were finished meanwhile.
    fn after_read(&mut self, cx: &mut Context<Self>) {
        if self.reading.is_some() || self.waiting.is_empty() {
            return;
        }
        let lines = std::mem::take(&mut self.waiting);
        self.consider(lines, cx);
    }

    fn consider(&mut self, lines: Vec<usize>, cx: &mut Context<Self>) {
        let (cursor_line, candidates) = {
            let ed = self.editor.read(cx);
            let count = ed.line_count();
            let mut seen = vec![];
            let candidates: Vec<(usize, String)> = lines
                .into_iter()
                .filter(|l| *l > 0 && *l < count && !seen.contains(l) && {
                    seen.push(*l);
                    true
                })
                .map(|l| (l, ed.line_text(l).trim().to_string()))
                .collect();
            (ed.line_of(ed.cursor()), candidates)
        };
        let mut marked = false;
        for (line, text) in candidates {
            let hash = hash_text(&text);
            let verdict = judge(
                &text,
                line == cursor_line,
                self.note.read_hashes.contains(&hash),
                self.calls_mind(&text),
                self.note.mode == Mode::ThinksWithYou,
                self.reading.is_some(),
            );
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
                    "quote": t.quote,
                    "resolved": t.status == ThreadStatus::Resolved,
                    "messages": t.messages.iter().map(|m| json!({"role": role_wire(m.role), "text": m.text})).collect::<Vec<_>>(),
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
        let short = if message.to_lowercase().contains("bun is not installed") {
            "Inkling's brain needs Bun. Install it from bun.sh.".to_string()
        } else {
            match kind {
                "auth" => "Sign in to Claude Code first: run `claude` and /login.".to_string(),
                "rate_limit" => "You've reached your Claude usage limit for now. Try again later.".to_string(),
                "overloaded" => "Claude is overloaded right now. Try again in a moment.".to_string(),
                "bad_output" => "The answer came back in a shape Inkling can't use. Try again.".to_string(),
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
                    format!("Couldn't reach the model: {m}")
                }
            }
        };
        self.notice(short, cx);
    }

    fn notice(&mut self, text: String, cx: &mut Context<Self>) {
        self.error = Some((text, std::time::Instant::now()));
        cx.notify();
    }

    fn read_line(&mut self, line: usize, text: String, hash: u64, cx: &mut Context<Self>) {
        // The first sentence of a new notebook is always asked about.
        let task = if self.note.first_run { "probe" } else { "read" };
        let input = json!({"task": task, "ctx": self.context(cx), "target": text});
        let target = text.clone();
        let req = self.run(input, false, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.reading = None;
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if v["speak"].as_bool() == Some(true) && !body.is_empty() {
                    let quote = v["quote"].as_str().unwrap_or("").to_string();
                    let kind = if v["kind"] == "correction" { ThoughtKind::Correction } else { ThoughtKind::Question };
                    let sketch = v["sketch"].as_bool() == Some(true);
                    if let Some(id) = page.add_thought(line, &target, &quote, kind, Some(body), cx) {
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
                page.later(cx, |page, cx| page.after_read(cx));
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.reading = None;
                // It was never read: let a later pause try this paragraph again.
                page.note.read_hashes.retain(|h| *h != hash);
                page.schedule_save(cx);
                if kind != "cancelled" {
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

    /// Run a streaming task whose answer becomes the thread's next message.
    fn stream_into(&mut self, thread: String, input: Value, cx: &mut Context<Self>) {
        let tid = thread.clone();
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
                        page.notice("The sketch came back in a shape Inkling can't read.".into(), cx);
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
        // ⌘↵ with nothing selected and a thread open: that thread is done.
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
        self.editor.update(cx, |e, cx| e.move_cursor_to(range.end, cx));
        self.activate(Some(id.clone()), window, cx);
        let input = json!({"task": "probe", "ctx": self.context(cx), "target": target});
        let tid = id.clone();
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
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
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Ask Inkling about this…"));
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.submit_ask(window, cx);
            }
        });
        input.update(cx, |i, cx| i.focus(window, cx));
        self.asking = Some(Asking { range, input, room: vec![self.mind.clone()], _sub: sub });
        self.show_toolbar = false;
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
        for tmpl in crate::store::mind_templates() {
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
            if asking.room.len() > 1 {
                asking.room.remove(i);
            }
        } else if asking.room.len() < ROOM_MAX {
            asking.room.push(mind);
        }
        cx.notify();
    }

    /// Dismiss the open thread from the keyboard (⌘⇧⌫).
    pub fn dismiss_active(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.clone() {
            self.set_status(&id, ThreadStatus::Dismissed, cx);
        }
    }

    /// Put the last dismissed thread back (⌥⌘Z, or Undo), within a few seconds.
    pub fn undo_dismiss(&mut self, cx: &mut Context<Self>) {
        let Some((id, range, at)) = self.last_dismissed.take() else { return };
        if at.elapsed() > UNDO_FOR {
            return;
        }
        if let Some(t) = self.thread_mut(&id) {
            t.status = ThreadStatus::Open;
        }
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
                let input = cx.new(|cx| InputState::new(window, cx).placeholder("Write back…"));
                let tid = id.clone();
                let sub = cx.subscribe_in(&input, window, move |this, _, ev: &InputEvent, window, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.send_reply(&tid, window, cx);
                    }
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
        let Some(input) = self.replies.get(id).cloned() else { return };
        let text = input.read(cx).value().trim().to_string();
        if text.is_empty() || self.pending.contains_key(id) {
            return;
        }
        input.update(cx, |i, cx| i.set_value("", window, cx));
        let Some(t) = self.note.threads.iter().find(|t| t.id == id).cloned() else { return };
        self.push_message(id, Role::You, text.clone(), cx);
        self.maybe_learn(&text, cx);
        let input = json!({
            "task": "reply",
            "ctx": self.context(cx),
            "quote": t.quote,
            "messages": t.messages.iter().map(|m| json!({"role": role_wire(m.role), "text": m.text})).collect::<Vec<_>>(),
            "message": text,
        });
        let tid = id.to_string();
        let asked = text.clone();
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                let body = v["text"].as_str().unwrap_or("").trim().to_string();
                if body.is_empty() {
                    page.notice("Inkling came back with nothing to say. Try again.".into(), cx);
                } else {
                    page.push_message(&tid, Role::Mind, body, cx);
                }
                if v["resolved"].as_bool() == Some(true) {
                    if let Some(t) = page.thread_mut(&tid) {
                        t.status = ThreadStatus::Resolved;
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
                }
                cx.notify();
            }
            BrainMsg::Delta(_) => {}
        });
        self.pending.insert(id.to_string(), (req, Pending::Thinking));
        cx.notify();
    }

    fn set_status(&mut self, id: &str, status: ThreadStatus, cx: &mut Context<Self>) {
        if let Some(t) = self.thread_mut(id) {
            t.status = status;
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
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(UNDO_FOR + Duration::from_millis(60)).await;
                        let _ = this.update(cx, |_, cx| cx.notify());
                    })
                    .detach();
                }
                if let Some(mut r) = self.ropes.remove(id) {
                    r.cut();
                    self.snipped.push((r, Theme::get(cx).voice_ink, std::time::Instant::now()));
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
        for m in &t.messages {
            match m.role {
                Role::Mind => out.push_str(&format!("{}: {}\n\n", t.mind_name, m.text)),
                Role::You => out.push_str(&format!("You: {}\n\n", m.text)),
            }
        }
        out.trim_end().to_string()
    }

    fn make_note_from_thread(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(t) = self.note.threads.iter().find(|t| t.id == id).cloned() else { return };
        let mode = cx.global::<Store>().settings.default_mode;
        // A quote can span paragraphs; blocks are single lines.
        let quote = t.quote.replace('\n', " ");
        let title: String = quote.chars().take(60).collect();
        let mut note = Note::new(&title, mode, self.note.folder.clone());
        note.blocks = vec![Block { kind: BlockKind::Title, text: title }];
        note.blocks.push(Block { kind: BlockKind::Quote, text: quote });
        for m in &t.messages {
            let who = if m.role == Role::Mind { t.mind_name.clone() } else { "Me".into() };
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
        self.schedule_save(cx);
        cx.notify();
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
                cx.notify();
            }
            BrainMsg::Done(v) => {
                page.chat_pending = None;
                let text = v["text"].as_str().unwrap_or("").trim().to_string();
                if text.is_empty() {
                    page.notice("Inkling came back with nothing to say. Try again.".into(), cx);
                } else {
                    page.note.chat.push(ChatMessage { role: Role::Mind, text, at: Utc::now() });
                    page.schedule_save(cx);
                }
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.chat_pending = None;
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                cx.notify();
            }
        });
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
            Step::Escape => self.on_escape(cx),
            Step::Resolve(n) | Step::Reopen(n) | Step::Dismiss(n) | Step::Mark(n) | Step::Copy(n) | Step::MakeNote(n) => {
                let Some(id) = self.nth_thread(*n) else {
                    log::warn!("script: no thread {n}");
                    return;
                };
                match step {
                    Step::Resolve(_) => self.set_status(&id, ThreadStatus::Resolved, cx),
                    Step::Reopen(_) => self.set_status(&id, ThreadStatus::Open, cx),
                    Step::Dismiss(_) => self.set_status(&id, ThreadStatus::Dismissed, cx),
                    Step::Mark(_) => self.toggle_mark(&id, window, cx),
                    Step::Copy(_) => log::info!("script copy: {:?}", self.thread_text(&id)),
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
                    _ => log::warn!("script: unknown setting {key}={value}"),
                }
                cx.global::<Store>().save_settings();
            }
            Step::Width(w) => {
                let h = window.viewport_size().height;
                window.resize(size(px(*w), h));
            }
            Step::Dump(label) => log::info!("script dump {label}: {}", self.debug_state(window, cx)),
            Step::Room(names) => self.pending_room = names.clone(),
            Step::UndoDismiss => self.undo_dismiss(cx),
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
            Step::Log(text) => log::info!("script log: {text}"),
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
        let m = Metrics::for_width(available);
        let (_, slots, _) = self.margin_layout(placement, m, cx);
        let ed = self.editor.read(cx);
        let error = self.error.as_ref().filter(|(_, at)| at.elapsed() < Duration::from_secs(12)).map(|(m, _)| m.clone());
        json!({
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
            "asking": self.asking.is_some(),
            "last_edited": self.last_edited,
            "cursor_line": ed.line_of(ed.cursor()),
            "metrics": {"viewport": viewport, "available": available, "column": m.column, "margin": m.margin, "both": m.both, "page": m.page(), "fits": m.page() <= available, "column_left": m.column_left(), "right_left": m.right_left()},
            "slots": slots.iter().map(|s| json!({"id": short(&s.id), "side": format!("{:?}", s.side), "anchor": s.anchor_top.as_f32(), "top": s.top.as_f32(), "h": s.height.as_f32()})).collect::<Vec<_>>(),
            "threads": self.note.threads.iter().map(|t| json!({
                "id": short(&t.id), "kind": format!("{:?}", t.kind), "status": format!("{:?}", t.status), "side": format!("{:?}", t.side),
                "quote": t.quote, "msgs": t.messages.len(), "drawing": t.drawing.is_some(),
            })).collect::<Vec<_>>(),
            "anchors": ed.anchors().iter().map(|a| json!({"id": short(&a.id), "range": [a.range.start, a.range.end], "open": a.open})).collect::<Vec<_>>(),
            "read_hashes": self.note.read_hashes.len(),
            "waiting_tags": if self.typing_recently() { self.arrived.keys().map(|id| short(id)).collect::<Vec<_>>() } else { vec![] },
            "typing": self.typing_recently(),
            "rechecking": self.rechecking.iter().map(|id| short(id)).collect::<Vec<_>>(),
            "undo": self.last_dismissed.as_ref().filter(|(_, _, at)| at.elapsed() < UNDO_FOR).map(|(id, _, _)| short(id)),
            "first_run": self.note.first_run,
            "statuses": self.note.threads.iter().map(|t| format!("{}:{:?}:{}", short(&t.id), t.status, t.mind_name)).collect::<Vec<_>>(),
            // tags more than half unfolded, and tags moved from where the margin puts them
            "unfolded": self.unfold.iter().filter(|(_, u)| **u > 0.5).map(|(id, _)| short(id)).collect::<Vec<_>>(),
            "moved_tags": self.note.threads.iter().filter(|t| t.tag_offset != (0., 0.)).map(|t| format!("{}:{:.0},{:.0}", short(&t.id), t.tag_offset.0, t.tag_offset.1)).collect::<Vec<_>>(),
        })
        .to_string()
    }

    // ----- export -------------------------------------------------------------

    pub fn markdown(&mut self, with_threads: bool, cx: &mut Context<Self>) -> String {
        self.sync_from_editor(cx);
        crate::export::to_markdown(&self.note, with_threads)
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
        let label = |s: &'static str| div().text_size(px(13.)).text_color(p.graphite).child(s);
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(16.))
            .h(px(24.))
            // the controls step back while you write, like the window's own
            .opacity(self.chrome.get())
            .font_family(SANS)
            .when(!jmw, |d| {
                d.child(
                    div()
                        .id("mind-picker")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .child(ui::small_dot(t.voice_ink, 7.))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(p.ink).child(mind_name))
                        .child(icon("chevron-down", 12., p.faint))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.menu = if this.menu == Some(Menu::Mind) { None } else { Some(Menu::Mind) };
                            cx.notify();
                        })),
                )
                .when(!has_brief, |d| {
                    d.child(
                        div()
                            .id("tell")
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .child(icon("pencil", 12., p.faint))
                            .child(label("Tell Inkling"))
                            .on_click(cx.listener(|this, _, window, cx| this.edit_brief(window, cx))),
                    )
                })
                .child(
                    div()
                        .id("mode")
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .cursor_pointer()
                        .child(label(match mode {
                            Mode::ThinksWithYou => "Reads along",
                            Mode::WhenYouAsk => "Waits to be asked",
                        }))
                        .child(icon("chevron-down", 12., p.faint))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.menu = if this.menu == Some(Menu::Mode) { None } else { Some(Menu::Mode) };
                            cx.notify();
                        })),
                )
            })
            .child(div().flex_1())
            .when(!jmw && open > 0, |d| {
                d.child(
                    div()
                        .id("open-count")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .child(ui::small_dot(t.voice_ink, 7.))
                        .child(div().text_size(px(13.)).text_color(p.graphite).child(if open == 1 { "1 loose thread".to_string() } else { format!("{open} loose threads") }))
                        .on_click(cx.listener(|this, _, window, cx| this.jump_to_open(true, window, cx))),
                )
            })
            .child(
                div()
                    .id("hide-threads")
                    .cursor_pointer()
                    .child(label(if jmw { "Show threads" } else { "Hide threads" }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.just_my_words = !this.just_my_words;
                        cx.notify();
                    })),
            )
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
            let start = self.editor.read(cx).anchors().iter().find(|a| a.id == id).map(|a| a.range.start);
            self.activate(Some(id.clone()), window, cx);
            if let Some(start) = start {
                self.editor.update(cx, |e, cx| e.move_cursor_to(start, cx));
                self.keep_cursor_visible(window, cx);
            }
            if let Some(input) = self.replies.get(&id).cloned() {
                input.update(cx, |i, cx| i.focus(window, cx));
            }
        }
    }

    fn brief_line(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = Theme::get(cx).clone();
        let p = t.p;
        if let Some(input) = self.brief_input.as_ref().map(|(i, _)| i.clone()) {
            return Some(
                div()
                    .mt(px(6.))
                    .p(px(10.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(p.rule)
                    .bg(p.card)
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(p.faint)
                            .child("What's this note for, or how should Inkling be here? It keeps this in mind."),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .font_family(t.voice_font.clone())
                            .text_size(px(14.))
                            .text_color(p.ink)
                            .child(Input::new(&input).appearance(false)),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .flex()
                            .justify_end()
                            .items_center()
                            .gap(px(12.))
                            .child(ui::ghost_button("brief-cancel", "Cancel", &t).on_click(cx.listener(|this, _, window, cx| {
                                this.brief_input = None;
                                this.editor.update(cx, |e, cx| e.focus(window, cx));
                                cx.notify();
                            })))
                            .child(ui::primary_button("brief-save", "Save", &t).on_click(cx.listener(|this, _, window, cx| this.save_brief(window, cx)))),
                    )
                    .into_any_element(),
            );
        }
        let brief = self.note.brief.clone()?;
        Some(
            div()
                .id("brief")
                .mt(px(6.))
                .flex()
                .flex_row()
                .items_start()
                .gap(px(7.))
                .cursor_pointer()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .pt(px(3.))
                        .flex_none()
                        .child(icon("pencil", 10., p.faint))
                        .child(div().text_size(px(10.5)).text_color(p.faint).child("Tell Inkling")),
                )
                .child(div().font_family(t.voice_font.clone()).text_size(px(14.)).text_color(p.graphite).child(brief))
                .on_click(cx.listener(|this, _, window, cx| this.edit_brief(window, cx)))
                .into_any_element(),
        )
    }

    fn menus(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        let card = match menu {
            Menu::Mind => {
                let minds = cx.global::<Store>().minds.clone();
                let current = self.note.mind_id.clone();
                let mut c = ui::menu_card(&t).w(px(232.));
                for (i, m) in minds.iter().enumerate() {
                    if i == 1 {
                        c = c.child(div().mx(px(14.)).mt(px(4.)).mb(px(2.)).text_size(px(10.)).text_color(p.faint).child("Yours"));
                    }
                    let id = m.id.clone();
                    c = c.child(
                        ui::menu_item(SharedString::from(format!("mind-{}", m.id)), &t)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(ui::small_dot(p.ink, 5.))
                                    .child(div().flex_1().child(m.name.clone()))
                                    .when(m.id == current, |d| d.child(icon("check", 12., p.ink))),
                            )
                            .when(!m.description.is_empty(), |d| {
                                d.child(div().pl(px(11.)).text_size(px(10.5)).text_color(p.faint).child(m.description.clone()))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| this.set_mind(&id, cx))),
                    );
                }
                c.child(div().mt(px(4.)).border_t_1().border_color(p.rule))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .px(px(14.))
                            .pt(px(6.))
                            .pb(px(2.))
                            .child(ui::ghost_button("new-mind", "+ New mind", &t).on_click(cx.listener(|this, _, _, cx| {
                                this.menu = None;
                                cx.emit(PageEvent::OpenSettings(crate::settings::Tab::NewMind));
                            })))
                            .child(ui::ghost_button("manage-minds", "Manage", &t).on_click(cx.listener(|this, _, _, cx| {
                                this.menu = None;
                                cx.emit(PageEvent::OpenSettings(crate::settings::Tab::Minds));
                            }))),
                    )
                    .into_any_element()
            }
            Menu::Mode => {
                let current = self.note.mode;
                let item = |id: &'static str, mode: Mode, title: &'static str, body: &'static str, cx: &mut Context<Self>| {
                    ui::menu_item(id, &t)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(match mode {
                                    Mode::ThinksWithYou => icon("lightbulb", 12., p.bulb),
                                    Mode::WhenYouAsk => icon("message-circle", 11., p.faint),
                                })
                                .child(div().flex_1().child(title))
                                .when(current == mode, |d| d.child(icon("check", 12., p.ink))),
                        )
                        .child(div().pl(px(18.)).text_size(px(10.5)).text_color(p.faint).child(body))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_mode(mode, cx)))
                };
                ui::menu_card(&t)
                    .w(px(220.))
                    .child(item("mode-thinks", Mode::ThinksWithYou, "Reads along", "Reads along and speaks up when something's worth a second look.", cx))
                    .child(item("mode-ask", Mode::WhenYouAsk, "Waits to be asked", "Quiet until you ask: say its name, select words, or open the chat.", cx))
                    .into_any_element()
            }
            Menu::Thread => return None,
        };
        let left = match menu {
            Menu::Mind => px(self.m.column_left() - 6.),
            _ => px(self.m.column_left() + 150.),
        };
        Some(
            deferred(
                div()
                    .absolute()
                    .top(px(26.))
                    .left(left)
                    .occlude()
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.menu = None;
                        cx.notify();
                    }))
                    .child(card),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }

    fn message_view(&self, m: &Message, t: &Theme, align_right: bool) -> Div {
        let p = t.p;
        match m.role {
            // Inkling's words are for reading: ink, not thread.
            Role::Mind => div()
                .font_family(t.voice_font.clone())
                .text_size(px(14.5))
                .line_height(px(21.))
                .text_color(p.ink)
                .when(align_right, |d| d.text_right())
                .child(m.text.clone()),
            Role::You => div()
                .flex()
                .gap(px(6.))
                .font_family(SANS)
                .text_size(px(13.5))
                .line_height(px(19.))
                .text_color(p.graphite)
                .when(align_right, |d| d.text_right())
                .child(div().flex_none().text_color(p.faint).child("You"))
                .child(div().child(m.text.clone())),
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
        let t = Theme::get(cx).clone();
        let p = t.p;
        let pb = self.playbacks.get(id)?;
        let (step, progress) = pb.state();
        let n = pb.steps();
        let caption = pb.caption();
        let canvas = drawing::canvas_for(pb.drawing.clone(), step, progress, px(width), t.clone());
        let id_prev = id.to_string();
        let id_next = id.to_string();
        let id_replay = id.to_string();
        let id_large = id.to_string();
        Some(
            div()
                .mt(px(10.))
                .flex()
                .flex_col()
                .child(canvas)
                .child(
                    div()
                        .mt(px(8.))
                        .flex()
                        .flex_row()
                        .items_start()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .font_family(t.voice_font.clone())
                                .text_size(px(11.5))
                                .line_height(px(15.))
                                .text_color(p.graphite)
                                .child(caption),
                        )
                        .when(n > 1, |d| {
                            d.child(
                                div()
                                    .flex()
                                    .flex_none()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(10.))
                                    .text_color(p.faint)
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("prev-{id_prev}")))
                                            .cursor_pointer()
                                            .child(icon("chevron-left", 11., p.faint))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(pb) = this.playbacks.get_mut(&id_prev) {
                                                    pb.prev();
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .child(format!("{step}/{n}"))
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("next-{id_next}")))
                                            .cursor_pointer()
                                            .child(icon("chevron-right", 11., p.faint))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(pb) = this.playbacks.get_mut(&id_next) {
                                                    pb.next();
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("replay-{id_replay}")))
                                            .cursor_pointer()
                                            .child(icon("rotate-ccw", 10., p.faint))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if let Some(pb) = this.playbacks.get_mut(&id_replay) {
                                                    pb.replay();
                                                }
                                                cx.notify();
                                            })),
                                    ),
                            )
                        }),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("large-{id_large}")))
                        .mt(px(4.))
                        .text_size(px(10.))
                        .text_color(p.faint)
                        .cursor_pointer()
                        .hover(move |s| s.text_color(p.ink))
                        .child("See it larger")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.enlarged = Some(id_large.clone());
                            if let Some(pb) = this.playbacks.get_mut(&id_large) {
                                pb.replay();
                            }
                            cx.notify();
                        })),
                )
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
        let thread_color = mind_thread(&thread.mind_name, &self.mind.name, cx.global::<Store>().settings.ink, &t);
        let unfold = self.unfold.get(&id).copied().unwrap_or(0.);
        let since = self.tying.get(&id).map(|at| anim_since(*at, std::time::Instant::now()));
        let fade = if tied && !is_active { tag_fade(since) } else { 1. };
        let number = self.numbers.get(&id).copied();
        let correction = thread.kind == ThoughtKind::Correction;
        let right = side == Side::Right;

        // ---- the folded tab ----
        let glyph: AnyElement = if tied {
            div().size(px(7.)).rounded_full().bg(thread_color).into_any_element()
        } else {
            div()
                .text_size(px(13.))
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
            .h(px(18.))
            .flex()
            .items_center()
            .gap(px(4.))
            .font_family(SANS)
            .children(number.map(|n| div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(p.graphite).child(n.to_string())))
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
            .when(!right, |d| d.ml(px(self.m.margin - TAB_W)))
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
        let mind = if thread.mind_name.is_empty() { "Inkling".to_string() } else { thread.mind_name.clone() };
        let kind = match thread.kind {
            ThoughtKind::Correction => "correction",
            ThoughtKind::Sketch => "sketch",
            ThoughtKind::Ask => "you asked",
            ThoughtKind::Answer => "answer",
            ThoughtKind::Question => "question",
        };
        let tied = thread.status == ThreadStatus::Resolved;
        let right = side == Side::Right;
        let width = self.m.margin;
        let natural = self.panel_heights.borrow().get(&id).copied().unwrap_or(px(120.));
        let e = ease_out_cubic(unfold);

        // A tag opens away from the words, unless that runs off the window: then it opens
        // the other way, and it slides up when it would hang below the bottom edge.
        let mut open_right = right;
        let mut lift = px(0.);
        if let Some(b) = self.tab_bounds.borrow().get(&id).copied() {
            let vp = self.viewport;
            let room_right = vp.width - px(8.) - b.origin.x;
            let room_left = b.origin.x + b.size.width - px(8.);
            if open_right && room_right < px(width) && room_left > room_right {
                open_right = false;
            } else if !open_right && room_left < px(width) && room_right > room_left {
                open_right = true;
            }
            let below = vp.height - px(8.) - b.origin.y;
            if natural > below {
                lift = (natural - below).min((b.origin.y - px(44.)).max(px(0.)));
            }
        }
        let lift = lift * e;
        // the tab's loop, in the panel's coordinates
        let w = TAB_W + (width - TAB_W) * e;
        let ring_x = if right { 13. } else { TAB_W - 13. } - if open_right { 0. } else { TAB_W - w };
        let ring_on_left = ring_x < w / 2.;
        // the header keeps clear of the loop: 12 px past it, less the tag's own padding
        let ring_pad = if lift < px(6.) { (if ring_on_left { ring_x } else { w - ring_x }) + 1. } else { 0. };

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
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .child(div().font_weight(FontWeight::SEMIBOLD).text_color(thread_color).child(mind))
                        .child(div().text_color(p.faint).child(if tied { "\u{b7} tied off".to_string() } else { format!("\u{b7} {kind}") })),
                )
                .children(self.numbers.get(&id).map(|n| div().text_size(px(12.5)).text_color(p.faint).child(n.to_string()))),
        );
        if tied && !is_active {
            let first = thread.messages.first().map(|m| m.text.clone()).unwrap_or_default();
            let short: String = first.chars().take(64).collect::<String>() + if first.chars().count() > 64 { "\u{2026}" } else { "" };
            body = body.child(div().text_size(px(13.)).line_height(px(18.)).text_color(p.faint).child(short));
        } else {
            // Folded, a tag shows what it says: a question's question, an asked mind's answer.
            let messages: Vec<&Message> = if is_active {
                thread.messages.iter().collect()
            } else if thread.kind == ThoughtKind::Ask {
                thread.messages.iter().rev().find(|m| m.role == Role::Mind).or(thread.messages.first()).into_iter().collect()
            } else {
                thread.messages.iter().take(1).collect()
            };
            for m in messages {
                body = body.child(self.message_view(m, &t, false));
            }
            if let Some(Pending::Streaming(s)) = &pending {
                body = body.child(self.message_view(&Message::new(Role::Mind, s.clone()), &t, false));
            }
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
                            .child(div().text_size(px(13.)).text_color(p.faint).child("Stitching a sketch\u{2026}"))
                            .child(Self::thinking_dot(&t, SharedString::from(format!("sketch-{id}")))),
                    );
                }
                None => {}
            }
            if !is_active && replies > 0 {
                body = body.child(
                    div()
                        .text_size(px(12.5))
                        .text_color(p.faint)
                        .child(if replies == 1 { "1 reply".to_string() } else { format!("{replies} replies") }),
                );
            }
        }
        if is_active {
            if let Some(input) = self.replies.get(&id).cloned() {
                let id_send = id.clone();
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .mt(px(2.))
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .bg(p.paper)
                        .border_1()
                        .border_color(p.rule)
                        .child(div().flex_1().text_size(px(13.5)).child(Input::new(&input).appearance(false)))
                        .child(
                            div()
                                .id(SharedString::from(format!("send-{id_send}")))
                                .cursor_pointer()
                                .child(icon("send", 13., p.graphite))
                                .on_click(cx.listener(move |this, _, window, cx| this.send_reply(&id_send, window, cx))),
                        ),
                );
            }
            let id_res = id.clone();
            let id_dis = id.clone();
            let id_copy = id.clone();
            let id_note = id.clone();
            body = body.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        ui::ghost_button(SharedString::from(format!("res-{id_res}")), if tied { "Reopen" } else { "Resolve" }, &t)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let st = if tied { ThreadStatus::Open } else { ThreadStatus::Resolved };
                                this.set_status(&id_res, st, cx)
                            })),
                    )
                    .child(
                        ui::ghost_button(SharedString::from(format!("dis-{id_dis}")), "Dismiss", &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_status(&id_dis, ThreadStatus::Dismissed, cx))),
                    )
                    .child(div().flex_1())
                    .child(
                        ui::ghost_button(SharedString::from(format!("note-{id_note}")), "Make a note", &t)
                            .on_click(cx.listener(move |this, _, _, cx| this.make_note_from_thread(&id_note, cx))),
                    )
                    // Copy is the least used of the four: an icon, so the row fits a margin
                    .child(
                        div()
                            .id(SharedString::from(format!("copy-{id_copy}")))
                            .cursor_pointer()
                            .child(icon("copy", 12., p.faint))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let text = this.thread_text(&id_copy);
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                            })),
                    ),
            );
        }

        // the words, laid out at full width and revealed as the tag grows
        let owner: AnyView = cx.entity().into();
        let content = div().w(px(width)).px(px(15.)).pt(px(9.)).pb(px(13.)).font_family(SANS).child(body);
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
                .rounded(px(8. + 2. * e))
                .bg(p.card)
                .shadow(vec![BoxShadow {
                    color: p.shadow.opacity(0.6 + 0.4 * e),
                    offset: point(px(0.), px(1. + 3. * e)),
                    blur_radius: px(4. + 10. * e),
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
                // the stitched edge grows with the tag
                .child(
                    div()
                        .absolute()
                        .top(px(4.))
                        .left(px(4.))
                        .right(px(4.))
                        .bottom(px(4.))
                        .rounded(px(6. + e))
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
                }),
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
        } else {
            self.hover_tag.remove(&key);
            if !self.tag_pointed(id) {
                self.hover_left.insert(id.to_string(), std::time::Instant::now());
            }
        }
        cx.notify();
    }

    fn tag_pointed(&self, id: &str) -> bool {
        self.hover_tag.contains(&format!("{id}:tab")) || self.hover_tag.contains(&format!("{id}:panel"))
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

    /// Move the tag being dragged by `delta` from where the drag began, kept inside the
    /// window. A press that moves less than 4 px is still a click.
    fn drag_tag_by(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(d) = self.drag.as_mut() else { return };
        if !d.moved && (delta.x.abs() + delta.y.abs()) < px(4.) {
            return;
        }
        d.moved = true;
        let (id, origin, tab) = (d.id.clone(), d.origin, d.tab);
        let (mut dx, mut dy) = (delta.x.as_f32(), delta.y.as_f32());
        if let Some(b) = tab {
            let vp = self.viewport;
            let (x0, y0) = (b.origin.x.as_f32(), b.origin.y.as_f32());
            let (w, h) = (b.size.width.as_f32(), b.size.height.as_f32());
            let (lo_x, lo_y) = (8. - x0, 44. - y0);
            dx = dx.clamp(lo_x, (vp.width.as_f32() - 8. - w - x0).max(lo_x));
            dy = dy.clamp(lo_y, (vp.height.as_f32() - 8. - h - y0).max(lo_y));
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
        let pointed = self.tag_pointed(id) || self.hover_left.get(id).is_some_and(|t| t.elapsed() < FOLD_DELAY);
        if pointed { 1. } else { 0. }
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
        let block = |id: &'static str, label: &'static str, k: BlockKind, cx: &mut Context<Self>| {
            let on = kind == k;
            div()
                .id(id)
                .w(px(22.))
                .h(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .text_size(px(10.5))
                .cursor_pointer()
                .when(on, |d| d.bg(p.ink).text_color(p.paper))
                .when(!on, |d| d.text_color(p.graphite).hover(move |s| s.bg(p.wash)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.editor.update(cx, |e, cx| e.set_selection_kind(k, cx));
                    this.show_toolbar = true;
                    cx.notify();
                }))
        };
        let mark = |id: &'static str, label: &'static str, m: Mark, cx: &mut Context<Self>| {
            let on = self.editor.read(cx).has_mark(m);
            div()
                .id(id)
                .w(px(22.))
                .h(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .text_size(px(11.))
                .cursor_pointer()
                .text_color(if on { p.ink } else { p.graphite })
                .when(on, |d| d.bg(p.wash))
                .hover(move |s| s.bg(p.wash))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.editor.update(cx, |e, cx| e.toggle_mark(m, cx));
                    this.show_toolbar = true;
                    cx.notify();
                }))
        };
        let mut colors = div().flex().items_center().gap(px(3.));
        for (i, (hex, _name)) in TEXT_COLORS.iter().enumerate() {
            let c: Hsla = if *hex == 0 { p.ink } else { rgb(*hex).into() };
            let value = *hex;
            colors = colors.child(
                div()
                    .id(SharedString::from(format!("color-{i}")))
                    .w(px(18.))
                    .h(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.wash))
                    .child(div().text_size(px(11.)).text_color(c).child("A"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.editor.update(cx, |e, cx| e.toggle_mark(Mark::Color(value), cx));
                        this.show_toolbar = true;
                        cx.notify();
                    })),
            );
        }
        let action = |id: &'static str, title: &'static str, hint: &'static str| {
            div()
                .id(id)
                .mx(px(4.))
                .px(px(7.))
                .py(px(5.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .hover(move |s| s.bg(p.wash))
                .gap(px(8.))
                .child(div().flex_none().font_family(t.voice_font.clone()).text_size(px(13.)).text_color(p.ink).child(title))
                .child(div().text_size(px(9.5)).text_color(p.faint).truncate().child(hint))
        };
        let card = ui::menu_card(&t)
            .w(px(184.))
            .child(
                div()
                    .px(px(5.))
                    .flex()
                    .flex_wrap()
                    .gap(px(1.))
                    .child(block("tb-p", "T", BlockKind::Paragraph, cx))
                    .child(block("tb-h1", "H1", BlockKind::H1, cx))
                    .child(block("tb-h2", "H2", BlockKind::H2, cx))
                    .child(block("tb-h3", "H3", BlockKind::H3, cx))
                    .child(block("tb-q", "❝", BlockKind::Quote, cx))
                    .child(block("tb-ul", "•", BlockKind::Bullet, cx))
                    .child(block("tb-ol", "1.", BlockKind::Numbered, cx)),
            )
            .child(div().mx(px(8.)).my(px(4.)).border_t_1().border_color(p.rule))
            .child(
                div()
                    .px(px(5.))
                    .flex()
                    .gap(px(1.))
                    .child(mark("tb-b", "B", Mark::Bold, cx))
                    .child(mark("tb-i", "I", Mark::Italic, cx))
                    .child(mark("tb-u", "U", Mark::Underline, cx))
                    .child(mark("tb-s", "S", Mark::Strike, cx))
                    .child(mark("tb-c", "</>", Mark::Code, cx)),
            )
            .child(div().px(px(6.)).pt(px(4.)).child(colors))
            .child(div().mx(px(8.)).my(px(4.)).border_t_1().border_color(p.rule))
            .child(action("tb-riddle", "Find a loose thread", "⌘↵").on_click(cx.listener(|this, _, window, cx| this.find_loose_thread(window, cx))))
            .child(action("tb-ask", "Ask Inkling", "Your own question").on_click(cx.listener(|this, _, window, cx| this.start_ask(window, cx))))
            .child(action("tb-viz", "Sketch it", "Stitched out step by step").on_click(cx.listener(|this, _, window, cx| this.sketch_selection(window, cx))));
        Some(
            deferred(div().absolute().top(top - px(6.)).left(px(self.m.right_left())).occlude().child(card))
                .with_priority(1)
                .into_any_element(),
        )
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
        // Ask the room: up to three minds answer the same question side by side.
        let mut chips = div().flex().flex_wrap().gap(px(6.)).mt(px(8.));
        for mind in candidates {
            let on = room.contains(&mind.name);
            let color = mind_thread(&mind.name, &own, chosen, &t);
            let name = mind.name.clone();
            chips = chips.child(
                div()
                    .id(SharedString::from(format!("room-{}", name)))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .px(px(8.))
                    .py(px(3.))
                    .rounded(px(999.))
                    .border_1()
                    .border_color(if on { color.opacity(0.7) } else { p.rule })
                    .bg(if on { color.opacity(0.08) } else { p.card })
                    .cursor_pointer()
                    .text_size(px(12.5))
                    .text_color(if on { p.ink } else { p.graphite })
                    .child(ui::small_dot(color, 6.))
                    .child(name)
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_room(mind.clone(), cx))),
            );
        }
        let n = room.len();
        Some(
            deferred(
                div()
                    .absolute()
                    .top(top - px(4.))
                    .left(px(self.m.right_left()))
                    .w(px(self.m.margin + 10.))
                    .occlude()
                    .child(
                        ui::menu_card(&t)
                            .px(px(12.))
                            .py(px(10.))
                            .font_family(SANS)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(ui::small_dot(t.voice_ink, 6.))
                                    .child(div().flex_1().text_size(px(14.)).child(Input::new(&input).appearance(false))),
                            )
                            .child(chips)
                            .child(
                                div()
                                    .mt(px(8.))
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(if n > 1 { format!("{n} minds will answer side by side \u{b7} Enter to ask \u{b7} Esc to cancel") } else { "Pick up to three minds \u{b7} Enter to ask \u{b7} Esc to cancel".to_string() }),
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
        let mut messages = div().flex().flex_col().gap(px(14.)).px(px(16.)).py(px(12.));
        if empty {
            messages = messages
                .child(
                    div()
                        .font_family(t.voice_font.clone())
                        .text_size(px(17.))
                        .line_height(px(23.))
                        .text_color(p.ink)
                        .child("I've read the whole note and its threads. Ask me anything about it."),
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
                                        .on_click(cx.listener(move |this, _, window, cx| this.send_chat(Some(q.to_string()), window, cx)))
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
                            .max_w(px(230.))
                            .px(px(10.))
                            .py(px(6.))
                            .rounded(px(10.))
                            .bg(p.shelf)
                            .text_size(px(12.5))
                            .line_height(px(17.))
                            .text_color(p.ink)
                            .child(m.text.clone()),
                    )
                    .into_any_element(),
                Role::Mind => div()
                    .id(SharedString::from(format!("chat-{i}")))
                    .font_family(t.voice_font.clone())
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(p.ink)
                    .child(m.text.clone())
                    .into_any_element(),
            });
        }
        if let Some((_, partial)) = &self.chat_pending {
            if partial.is_empty() {
                messages = messages.child(Self::thinking_dot(&t, "chat-thinking"));
            } else {
                messages = messages.child(
                    div()
                        .font_family(t.voice_font.clone())
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .child(partial.clone()),
                );
            }
        }
        let input = self.chat_input.clone();
        div()
            .w(px(CHAT_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(p.rule)
            .bg(p.paper)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .px(px(16.))
                    .pt(px(14.))
                    .pb(px(10.))
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(ui::small_dot(t.voice_ink, 6.))
                                    .child(div().font_family(t.voice_font.clone()).text_size(px(14.)).text_color(p.ink).child(self.mind.name.clone())),
                            )
                            .child(div().mt(px(3.)).text_size(px(10.)).text_color(p.faint).child(format!(
                                "On “{}”. Reading this note: {} thread{} ({} open)",
                                if title.is_empty() { "Untitled".to_string() } else { title },
                                visible,
                                if visible == 1 { "" } else { "s" },
                                open
                            ))),
                    )
                    .child(ui::ghost_button("chat-copy", "Copy", &t).on_click(cx.listener(|this, _, _, cx| {
                        let text = this
                            .note
                            .chat
                            .iter()
                            .map(|m| format!("{}: {}", if m.role == Role::You { "You" } else { this.mind.name.as_str() }, m.text))
                            .collect::<Vec<_>>()
                            .join("\n\n");
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    })))
                    .child(ui::ghost_button("chat-clear", "Clear", &t).on_click(cx.listener(|this, _, _, cx| this.clear_chat(cx))))
                    .child(
                        div()
                            .id("chat-close")
                            .cursor_pointer()
                            .child(icon("x", 13., p.faint))
                            .on_click(cx.listener(|this, _, window, cx| this.toggle_chat(window, cx))),
                    ),
            )
            .child(div().id("chat-scroll").flex_1().overflow_y_scroll().child(messages))
            .child(
                div()
                    .border_t_1()
                    .border_color(p.rule)
                    .px(px(14.))
                    .py(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().text_size(px(12.5)).child(Input::new(&input).appearance(false)))
                    .child(
                        div()
                            .id("chat-send")
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(p.ink)
                            .cursor_pointer()
                            .child(icon("arrow-up", 12., p.paper))
                            .on_click(cx.listener(|this, _, window, cx| this.send_chat(None, window, cx))),
                    ),
            )
            .into_any_element()
    }

    fn enlarged_drawing(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = self.enlarged.clone()?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        let block = self.drawing_block(&id, 460., cx)?;
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
                    .child(
                        div()
                            .id("enlarged")
                            .w(px(520.))
                            .p(px(30.))
                            .rounded(px(14.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.rule)
                            .child(
                                div().flex().justify_end().child(
                                    div()
                                        .id("enlarged-close")
                                        .cursor_pointer()
                                        .child(icon("x", 14., p.faint))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.enlarged = None;
                                            cx.notify();
                                        })),
                                ),
                            )
                            .child(block),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.enlarged = None;
                            cx.notify();
                        }),
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
                items.push((th.id.clone(), side, first.origin.y));
            }
            anchor_boxes.insert(th.id.clone(), rects);
        }
        let slots = margin::place(items, &self.heights.borrow(), self.active.as_deref(), px(CARD_GAP));
        (threads, slots, anchor_boxes)
    }
}

impl NotePage {
    /// Where a thread comes up through the cloth and where it ties onto its tag. It
    /// never runs across words on the front: it surfaces right after its stitches when
    /// they end the paragraph, otherwise at the edge of the column, level with them.
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
            Side::Left => (point(col - px(9.), y), point(px(m.margin) - px(13. - dx), tag_y)),
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
        let color_of = |name: &str| mind_thread(name, &own_mind, chosen_ink, &t);

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
                let color = color_of(&th.mind_name);
                styles.insert(th.id.clone(), AnchorStyle { color, correction: th.kind == ThoughtKind::Correction });
                if th.status == ThreadStatus::Open {
                    if let Some(a) = ed.anchors().iter().find(|a| a.id == th.id) {
                        open.push((a.range.start, th.id.clone(), color));
                    }
                }
            }
            open.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let numbers: HashMap<String, usize> = open.iter().enumerate().map(|(i, (_, id, _))| (id.clone(), i + 1)).collect();
            let mut gutter = vec![];
            for line in 1..ed.line_count() {
                let range = ed.line_range(line);
                if let Some(n) = open.iter().position(|(s, _, _)| *s >= range.start && *s <= range.end) {
                    gutter.push((line, GutterMark::Question { number: n + 1, color: open[n].2 }));
                    continue;
                }
                let text = ed.line_text(line);
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

        // Margin tags: folded tabs that unfold toward their target, eased each frame.
        let (threads, slots, anchor_boxes) = self.margin_layout(placement, m, cx);
        let ease = 1. - (-dt * 15.).exp();
        let mut unfolding = false;
        for th in &threads {
            let target = self.unfold_target(th);
            let u = self.unfold.entry(th.id.clone()).or_insert(0.);
            if (*u - target).abs() < 0.004 {
                *u = target;
            } else {
                *u += (target - *u) * ease;
                unfolding = true;
            }
        }
        self.unfold.retain(|id, _| threads.iter().any(|t| &t.id == id));
        // tags the pointer just left stay open a moment, then fold
        if self.hover_left.values().any(|t| t.elapsed() < FOLD_DELAY) {
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

        // Marks-only placement: a small knot after each quote opens its tag.
        let mut marks = vec![];
        if placement == Placement::Marks && !jmw {
            for th in &threads {
                let Some(rects) = anchor_boxes.get(&th.id) else { continue };
                let Some(last) = rects.last() else { continue };
                let id = th.id.clone();
                marks.push(
                    div()
                        .id(SharedString::from(format!("mark-{id}")))
                        .absolute()
                        .top(last.origin.y + px(2.))
                        .left(px(m.column_left()) + last.origin.x + last.size.width + px(3.))
                        .size(px(9.))
                        .rounded_full()
                        .bg(thread_color)
                        .cursor_pointer()
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
                ropes.push((rope.clone(), opacity, knot, color_of(&th.mind_name)));
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
        let error = self.error.as_ref().filter(|(_, at)| at.elapsed() < Duration::from_secs(12)).map(|(m, _)| m.clone());
        // "Thread dismissed · Undo" for a few seconds after a dismiss.
        let undo = self.last_dismissed.as_ref().filter(|(_, _, at)| at.elapsed() < UNDO_FOR).map(|_| {
            div()
                .absolute()
                .bottom(px(18.))
                .left(px(18.))
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(14.))
                .py(px(9.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.rule)
                .shadow(vec![BoxShadow { color: p.shadow, offset: point(px(0.), px(4.)), blur_radius: px(14.), spread_radius: px(0.), inset: false }])
                .font_family(SANS)
                .text_size(px(13.))
                .text_color(p.graphite)
                .child("Thread dismissed")
                .child(
                    div()
                        .id("undo-dismiss")
                        .cursor_pointer()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.ink)
                        .child("Undo")
                        .on_click(cx.listener(|this, _, _, cx| this.undo_dismiss(cx))),
                )
                .child(div().text_color(p.faint).child("\u{2325}\u{2318}Z"))
        });

        let body = div()
            .relative()
            .w(px(m.page()))
            .child(tethers)
            .child(div().ml(px(m.column_left())).w(px(m.column)).child(self.editor.clone()))
            .children(cards)
            .children(marks)
            .children(toolbar)
            .children(ask);

        let page = div()
            .relative()
            .w(px(m.page()))
            .flex()
            .flex_col()
            .child(div().relative().ml(px(m.column_left())).w(px(m.column)).child(header).when(!jmw, |d| d.children(brief)))
            .children(menus)
            .child(div().h(px(18.)))
            .child(body)
            .child(div().h(px(220.)));

        let sheet = div()
            .relative()
            .w(px(m.page() + 2. * SHEET_PAD))
            .min_h(viewport.height - px(56.))
            .px(px(SHEET_PAD))
            .pt(px(50.))
            .bg(p.paper)
            .rounded(px(4.))
            .shadow(vec![
                BoxShadow { color: p.shadow, offset: point(px(0.), px(1.)), blur_radius: px(2.), spread_radius: px(0.), inset: false },
                BoxShadow { color: p.shadow.opacity(0.6), offset: point(px(0.), px(12.)), blur_radius: px(32.), spread_radius: px(-12.), inset: false },
            ])
            .child(border)
            .child(page);
        let fits = m.page() + 2. * SHEET_PAD <= width - 16.;

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
                    .child(
                        div()
                            .id("page-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    // the traffic lights, sidebar toggle and Chat sit on the cloth above
                                    .pt(px(44.))
                                    .pb(px(28.))
                                    // Centred while it fits; past that keep the column in view.
                                    .map(|d| if fits { d.justify_center() } else { d.justify_start().pl(px(12.)) })
                                    .child(sheet),
                            ),
                    )
                    .children(error.map(|m| {
                        div()
                            .absolute()
                            .bottom(px(16.))
                            .left(px(16.))
                            .max_w(px(420.))
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(8.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.rule)
                            .text_size(px(13.))
                            .text_color(p.graphite)
                            .child(m)
                    }))
                    .children(undo)
                    .children(enlarged),
            )
            .children(chat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for w in [410., 520., 600., 690., 720., 800., 868., 900., 1010., 1100., 1320., 1600., 2400.] {
            let m = Metrics::for_width(w);
            let usable = (w - 48f32).max(0.);
            if usable >= 300. + 36. + 200. {
                assert!(m.page() <= usable + 0.01, "width {w}: page {} > {usable}", m.page());
            }
            assert!(m.column >= 300. && m.column <= 760., "width {w}: column {}", m.column);
            assert!(m.margin >= 200. && m.margin <= 280., "width {w}: margin {}", m.margin);
            assert!((m.right_left() - (m.column_left() + m.column + m.gap)).abs() < 0.01);
        }
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
        assert_eq!(narrow.column_left(), 0.);
        assert!(narrow.column >= 400.);
    }
}

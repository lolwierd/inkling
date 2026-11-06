//! Loose ends (⇧⌘K): every open thread in the notebook, oldest first, in ⌘K's card. A row
//! shows the words a thread hangs from and never what Inkling said, because Inkling speaks
//! only in the margin of the page you are on. Choosing a row goes to those words with the
//! tag open (`NotePage::go_to_thread`). The list is built from the notes already in memory,
//! the open note from the page's own copy; there is no index. The design and its reasons:
//! `design/features/loose-ends.md`.

use std::collections::HashMap;
use std::ops::Range;

use chrono::{DateTime, Local, TimeZone, Utc};
use gpui_kit::base::FocusTrapElement;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{Value, json};

use crate::note_page::NotePage;
use crate::store::{Folder, Note, Store, Thread, ThoughtKind, ThreadStatus};
use crate::theme::{SANS, SERIF, Theme, ts};
use crate::ui;

// ----- the list ------------------------------------------------------------------------

/// A listed thread. A thread's id is only unique within its note, so a row is both.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub note: String,
    pub thread: String,
}

/// How a listed thread stands now. The rows stay put while the card is open, so a thread
/// that ends meanwhile says so in its row instead of leaving it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Open,
    Tied,
    Dismissed,
    /// Its note, or the thread itself, is no longer there.
    Gone,
}

impl State {
    fn of(thread: Option<&Thread>) -> State {
        match thread.map(|t| t.status) {
            Some(ThreadStatus::Open) => State::Open,
            Some(ThreadStatus::Resolved) => State::Tied,
            // any other way a thread ends reads as put aside
            Some(_) => State::Dismissed,
            None => State::Gone,
        }
    }

    /// The word a row shows in place of its age, and the dump's.
    pub fn word(self) -> &'static str {
        match self {
            State::Open => "open",
            State::Tied => "tied off",
            State::Dismissed => "dismissed",
            State::Gone => "gone",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub key: Key,
    /// The words the thread hangs from, on one line.
    pub quote: String,
    pub correction: bool,
    /// The thread's mind as stored (empty in old notes, meaning the note's own), for its colour.
    pub mind_name: String,
    /// The mind as the tag's header names it.
    pub mind: String,
    /// The note's own mind, which wears your thread.
    pub own_mind: String,
    /// The note's title, "Untitled" when it has none.
    pub title: String,
    pub folder: Option<String>,
    /// When the thread was opened; `None` when its file does not say.
    pub opened: Option<DateTime<Utc>>,
    /// On the note the page has open.
    pub here: bool,
    pub state: State,
    /// What typing searches, lowercased: the quote, the note's title, its folder, the mind.
    hay: String,
    /// A thread of your own (Ask yourself): named You, and in your ink rather than a mind's
    /// thread.
    pub own: bool,
}

/// Who hung the thread, as its tag's header names it: You for a thread of your own, else
/// its mind; a thread with no mind's name (an old note's) says Inkling.
pub fn who(thread: &Thread) -> String {
    thread.asker()
}

/// Whitespace runs, line breaks included, folded to one space: a quote across paragraphs
/// reads as one line.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A date before 1970 or at it is a file that did not say (serde's default).
fn known(at: DateTime<Utc>) -> Option<DateTime<Utc>> {
    (at.timestamp() > 0).then_some(at)
}

/// Every open thread in `notes`, oldest first. `live` is the page's copy of the note it has
/// open, newer than the store's until the next save; `mind_of(id)` names a note's mind.
/// Threads opened in the same instant go by the older note, then reading order. This is the
/// one place that decides what is listed: a thread is listed when its status is
/// `ThreadStatus::Open`, so any new way for a thread to end stays out without a change here.
pub fn collect(notes: &[Note], folders: &[Folder], live: Option<&Note>, mind_of: impl Fn(&str) -> String) -> Vec<Row> {
    let mut from: Vec<&Note> = notes
        .iter()
        .map(|n| match live {
            Some(l) if l.id == n.id => l,
            _ => n,
        })
        .collect();
    if let Some(l) = live.filter(|l| !notes.iter().any(|n| n.id == l.id)) {
        from.push(l);
    }
    let mut rows: Vec<(Option<DateTime<Utc>>, DateTime<Utc>, &str, usize, Row)> = vec![];
    for note in from {
        let folder = note.folder.as_ref().and_then(|id| folders.iter().find(|f| &f.id == id)).map(|f| f.name.clone());
        let title = if note.title.trim().is_empty() { "Untitled".to_string() } else { one_line(&note.title) };
        let own_mind = mind_of(&note.mind_id);
        let here = live.is_some_and(|l| l.id == note.id);
        // a thread between notes is listed once, under its home: the open note's copy also
        // holds the far ends of threads that live in other notes (visiting ends), never saved
        let visiting = |id: &str| note.elsewhere_of(id).is_some_and(|e| e.visiting);
        for t in note.threads.iter().filter(|t| t.status == ThreadStatus::Open && !visiting(&t.id)) {
            let quote = one_line(&t.quote);
            let mind = who(t);
            let hay = format!("{quote}\n{title}\n{}\n{mind}", folder.as_deref().unwrap_or("")).to_lowercase();
            let row = Row {
                key: Key { note: note.id.clone(), thread: t.id.clone() },
                quote,
                correction: t.kind == ThoughtKind::Correction,
                mind_name: t.mind_name.clone(),
                mind,
                own_mind: own_mind.clone(),
                title: title.clone(),
                folder: folder.clone(),
                opened: known(t.created_at),
                here,
                state: State::Open,
                hay,
                own: t.own,
            };
            rows.push((row.opened, note.created_at, note.id.as_str(), t.start, row));
        }
    }
    rows.sort_by(|a, b| (a.0, a.1, a.2, a.3, &a.4.key.thread).cmp(&(b.0, b.1, b.2, b.3, &b.4.key.thread)));
    rows.into_iter().map(|r| r.4).collect()
}

/// The rows (by index) in which every word of `query` appears in the quote, the note's
/// title, its folder or the mind's name, ignoring case.
pub fn narrow(rows: &[Row], query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    rows.iter()
        .enumerate()
        .filter(|(_, r)| words.iter().all(|w| r.hay.contains(w.as_str())))
        .map(|(i, _)| i)
        .collect()
}

/// Read each row's state again from the notes as they are now (`live` for the page's note).
pub fn refresh(rows: &mut [Row], notes: &[Note], live: Option<&Note>) {
    let by_id: HashMap<&str, &Note> = notes.iter().map(|n| (n.id.as_str(), n)).collect();
    for r in rows {
        let note = match live {
            Some(l) if l.id == r.key.note => Some(l),
            _ => by_id.get(r.key.note.as_str()).copied(),
        };
        r.state = State::of(note.and_then(|n| n.threads.iter().find(|t| t.id == r.key.thread)));
    }
}

/// Where the highlight starts when the card opens: on the row after the thread you went to
/// last (`last`: its key and the row it was on), or on the row now in its place if it has
/// left the list, so ⇧⌘K and Return walk the list from the oldest. The first row otherwise.
pub fn start_at(rows: &[Row], last: Option<&(Key, usize)>) -> usize {
    let Some((key, at)) = last else { return 0 };
    let Some(end) = rows.len().checked_sub(1) else { return 0 };
    match rows.iter().position(|r| &r.key == key) {
        Some(i) => (i + 1).min(end),
        None => (*at).min(end),
    }
}

/// How long ago a thread was opened, in your calendar days: "today", "yesterday", "3 days
/// ago" (to 13), "2 weeks ago" (to 8), "2 months ago" (to 11), "a year ago", "3 years
/// ago". A time after `now` (a clock that moved) is today.
pub fn age_words<Tz: TimeZone>(then: DateTime<Utc>, now: &DateTime<Tz>) -> String {
    let days = (now.date_naive() - then.with_timezone(&now.timezone()).date_naive()).num_days();
    match days {
        i64::MIN..=0 => "today".into(),
        1 => "yesterday".into(),
        2..=13 => format!("{days} days ago"),
        14..=59 => format!("{} weeks ago", days / 7),
        60..=364 => format!("{} months ago", (days / 30).min(11)),
        _ if days < 730 => "a year ago".into(),
        _ => format!("{} years ago", days / 365),
    }
}

fn mark(r: &Row) -> &'static str {
    if r.correction { "\u{d7}" } else { "?" }
}

fn row_json(r: &Row, now: &DateTime<Local>) -> Value {
    json!({
        "note": r.title,
        "folder": r.folder,
        "thread": r.key.thread.chars().take(8).collect::<String>(),
        "quote": r.quote,
        "mind": r.mind,
        "mark": mark(r),
        "state": r.state.word(),
        "here": r.here,
        "age": r.opened.map(|t| age_words(t, now)),
    })
}

/// The list for the page's dump (`loose_ends`): the card as it is on screen, or with the
/// card closed, the rows it would show if it opened now.
/// (The page asks while it is being updated, so it hands over its note: reading the page
/// from here would panic.)
pub fn describe(live: &Note, window: &Window, cx: &App) -> Value {
    if let Some(card) = cx.try_global::<Shown>().and_then(|s| s.0.upgrade()) {
        return card.read(cx).describe(live, window, cx);
    }
    let store = cx.global::<Store>();
    let rows = collect(&store.notes, &store.folders, Some(live), |id| store.mind(id).name);
    let now = Local::now();
    json!({
        "open": false,
        "focused": false,
        "query": "",
        "selected": null,
        "count": rows.len(),
        "rows": rows.iter().map(|r| row_json(r, &now)).collect::<Vec<_>>(),
    })
}

// ----- the card ------------------------------------------------------------------------

/// Rows a Page Up or Page Down moves: about a cardful.
const PAGE: usize = 7;
/// The list's height at most: seven rows and its padding.
const LIST_MAX: f32 = 7. * 49. + 8.;

#[derive(Clone, Debug)]
pub enum CardEvent {
    /// Go to this thread's words. The row it was on (unnarrowed), for walking the list.
    Go(Key, usize),
    Dismiss,
}

impl EventEmitter<CardEvent> for Card {}

/// The card on screen, for the dump.
struct Shown(WeakEntity<Card>);

impl Global for Shown {}

/// ⌘K's card turned to Loose ends.
pub struct Card {
    input: Entity<InputState>,
    /// The words the rows were last narrowed by.
    query: String,
    /// Every thread open when the card opened, oldest first, fixed while it is open; each
    /// row's state is read again as the page behind changes.
    rows: Vec<Row>,
    /// The rows that match what is typed, by index into `rows`.
    shown: Vec<usize>,
    /// The highlighted row, by index into `shown`.
    selected: usize,
    scroll: UniformListScrollHandle,
    /// Tab stays in the card while it is open, as in ⌘K.
    trap: FocusHandle,
    page: WeakEntity<NotePage>,
    _subs: Vec<Subscription>,
}

impl Card {
    /// Open on every thread open now, narrowed by `query` (what ⌘K had typed), the highlight
    /// on the row after `last` (see [`start_at`]).
    pub fn new(page: &Entity<NotePage>, query: &str, last: Option<&(Key, usize)>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find a loose end"));
        if !query.is_empty() {
            // (setting the value sends no Change: the rows are narrowed below)
            input.update(cx, |i, cx| i.set_value(query.to_string(), window, cx));
        }
        let subs = vec![
            cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, _, cx| match ev {
                InputEvent::PressEnter { .. } => this.choose(cx),
                InputEvent::Change => this.narrow_to_input(cx),
                _ => {}
            }),
            // the page behind the card ties threads off: each row says how it stands now
            cx.observe(page, |_, _, cx| cx.notify()),
        ];
        let rows = {
            let store = cx.global::<Store>();
            collect(&store.notes, &store.folders, Some(&page.read(cx).note), |id| store.mind(id).name)
        };
        let shown = narrow(&rows, query);
        let selected = if query.trim().is_empty() { start_at(&rows, last) } else { 0 };
        let scroll = UniformListScrollHandle::new();
        scroll.scroll_to_item(selected, ScrollStrategy::Center);
        input.update(cx, |i, cx| i.focus(window, cx));
        let me = cx.entity().downgrade();
        cx.set_global(Shown(me));
        Self { input, query: query.to_string(), rows, shown, selected, scroll, trap: cx.focus_handle(), page: page.downgrade(), _subs: subs }
    }

    /// Give the search box the keyboard (again).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |i, cx| i.focus(window, cx));
    }

    /// What is typed in the search box, for ⌘K to carry on with.
    pub fn query(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    fn narrow_to_input(&mut self, cx: &mut Context<Self>) {
        let query = self.query(cx);
        if query == self.query {
            return;
        }
        self.shown = narrow(&self.rows, &query);
        self.query = query;
        self.select(0, cx);
    }

    fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected = ix.min(self.shown.len().saturating_sub(1));
        self.scroll.scroll_to_item(self.selected, ScrollStrategy::Nearest);
        cx.notify();
    }

    fn choose(&mut self, cx: &mut Context<Self>) {
        let Some(&at) = self.shown.get(self.selected) else { return };
        let row = &self.rows[at];
        if row.state != State::Gone {
            cx.emit(CardEvent::Go(row.key.clone(), at));
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let to = match ev.keystroke.key.as_str() {
            "down" => self.selected + 1,
            "up" => self.selected.saturating_sub(1),
            "pagedown" => self.selected + PAGE,
            "pageup" => self.selected.saturating_sub(PAGE),
            "escape" => {
                cx.stop_propagation();
                cx.emit(CardEvent::Dismiss);
                return;
            }
            _ => return,
        };
        cx.stop_propagation();
        self.select(to, cx);
    }

    /// Read each row's state again from the notes, the page's own copy for its note.
    fn refresh_rows(&mut self, cx: &App) {
        let store = cx.global::<Store>();
        let page = self.page.upgrade();
        refresh(&mut self.rows, &store.notes, page.as_ref().map(|p| &p.read(cx).note));
    }

    /// The dump's view of the card, each row's state read from `live` (the page's note) and
    /// the store as they are now.
    fn describe(&self, live: &Note, window: &Window, cx: &App) -> Value {
        let mut rows = self.rows.clone();
        refresh(&mut rows, &cx.global::<Store>().notes, Some(live));
        let now = Local::now();
        json!({
            "open": true,
            "focused": self.input.read(cx).focus_handle(cx).is_focused(window),
            "query": self.query,
            "selected": (!self.shown.is_empty()).then_some(self.selected),
            "count": rows.len(),
            "rows": self.shown.iter().map(|&i| row_json(&rows[i], &now)).collect::<Vec<_>>(),
        })
    }

    /// The rows in view, built as the list scrolls (a `uniform_list`: hundreds of threads
    /// cost what seven do).
    fn rows_in(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let ink = cx.global::<Store>().settings.ink;
        let now = Local::now();
        let mut out = vec![];
        for ix in range {
            let Some(r) = self.shown.get(ix).and_then(|&i| self.rows.get(i)) else { continue };
            let on = ix == self.selected;
            let open = r.state == State::Open;
            // the colour the page draws this thread in, so a row and its tag agree
            let color = if r.own { crate::note_page::own::ink(&t) } else { crate::note_page::mind_thread(&r.mind_name, &r.own_mind, ink, &t) };
            // the mark its folded tab shows: ? or ×, or the knot once it is tied off
            let mark_el: AnyElement = match r.state {
                State::Tied => div().size(px(7.)).rounded_full().bg(color).into_any_element(),
                _ => div()
                    .text_size(px(ts::UI))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if open { color } else { p.faint })
                    .child(mark(r))
                    .into_any_element(),
            };
            let tail = if open { r.opened.map(|at| age_words(at, &now)) } else { Some(r.state.word().to_string()) };
            let place = if r.here { "This note".to_string() } else { r.title.clone() };
            let folder = r.folder.clone().filter(|_| !r.here);
            let dot = || div().flex_none().px(px(5.)).text_color(p.faint).child("\u{b7}");
            let meta = div()
                .mt(px(1.))
                .h(px(16.))
                .flex()
                .items_center()
                .font_family(SANS)
                .text_size(px(ts::META))
                .text_color(p.graphite)
                // a long folder name gives way before its slash, a long title before the mind
                .when_some(folder, |d, f| {
                    d.child(
                        div()
                            .flex_none()
                            .max_w(px(110.))
                            .flex()
                            .mr(px(4.))
                            .gap(px(4.))
                            .text_color(p.faint)
                            .child(div().min_w(px(0.)).truncate().child(f))
                            .child(div().flex_none().child("/")),
                    )
                })
                .child(div().min_w(px(0.)).truncate().child(place))
                .child(dot())
                .child(div().flex_none().max_w(px(140.)).truncate().font_weight(FontWeight::MEDIUM).text_color(if open { color } else { p.faint }).child(r.mind.clone()))
                .when_some(tail, |d, words| d.child(dot()).child(div().flex_none().text_color(p.faint).child(words)));
            out.push(
                // A row in a uniform list is laid out on its own: it takes the list's whole
                // width, so long words are cut inside the card, not by its edge.
                div()
                    .id(("loose-row", ix))
                    .w_full()
                    .px(px(6.))
                    .cursor_pointer()
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(6.))
                            .rounded(px(7.))
                            .when(on, |d| d.bg(p.hover))
                            .when(!on, |d| d.hover(move |s| s.bg(p.hover.opacity(0.5))))
                            .flex()
                            .flex_row()
                            .items_start()
                            .gap(px(8.))
                            .child(div().flex_none().w(px(14.)).h(px(20.)).flex().items_center().justify_center().child(mark_el))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    // your words, set as on the page
                                    .child(
                                        div()
                                            .h(px(20.))
                                            .flex()
                                            .items_center()
                                            .font_family(SERIF)
                                            .text_size(px(ts::BODY))
                                            .text_color(if open { p.ink } else { p.faint })
                                            .child(div().min_w(px(0.)).truncate().child(r.quote.clone())),
                                    )
                                    .child(meta),
                            )
                            .when(on && r.state != State::Gone, |d| d.child(div().flex_none().h(px(20.)).flex().items_center().child(ui::key_hint("\u{21a9}", &t)))),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = ix;
                        this.choose(cx);
                    }))
                    .into_any_element(),
            );
        }
        out
    }
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        self.refresh_rows(cx);
        let total = self.rows.len();
        let on_row = self.shown.get(self.selected).is_some_and(|&i| self.rows[i].state != State::Gone);
        let list: AnyElement = if self.shown.is_empty() {
            let words = if total == 0 {
                "No open threads in any note.".to_string()
            } else {
                format!("No open thread matches \u{201c}{}\u{201d}.", self.query.trim())
            };
            div().px(px(16.)).py(px(14.)).text_size(px(ts::UI)).text_color(p.graphite).child(words).into_any_element()
        } else {
            div()
                .py(px(4.))
                .child(
                    uniform_list("loose-ends-list", self.shown.len(), cx.processor(|this, range: Range<usize>, _, cx| this.rows_in(range, cx)))
                        .track_scroll(&self.scroll)
                        .with_sizing_behavior(ListSizingBehavior::Infer)
                        .max_h(px(LIST_MAX)),
                )
                .into_any_element()
        };
        // what is still open: a row tied off while the card is open stays, but is not counted
        let open = |i: &usize| self.rows[*i].state == State::Open;
        let (all_open, shown_open) = ((0..total).filter(open).count(), self.shown.iter().filter(|i| open(i)).count());
        let count = match (all_open, self.query.trim().is_empty()) {
            (0, _) => None,
            (_, true) => Some(format!("{all_open} open, oldest first")),
            (_, false) => Some(format!("{shown_open} of {all_open} open")),
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_center()
            .items_start()
            .pt(px(120.))
            .bg(p.dim.opacity(0.25))
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.emit(CardEvent::Dismiss)))
            .child(
                div()
                    .id("loose-ends")
                    .focus_trap("loose-ends-trap", &self.trap)
                    .w(px(460.))
                    .flex()
                    .flex_col()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.rule)
                    .shadow(ui::modal_shadow(&p))
                    .font_family(SANS)
                    .capture_key_down(cx.listener(Self::on_key))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .border_b_1()
                            .border_color(p.rule)
                            // the spool, where ⌘K has its magnifier: the two lists apart at a glance
                            .child(ui::icon("spool", 14., p.faint))
                            .child(div().flex_1().child(Input::new(&self.input).appearance(false).px_0().text_size(px(ts::BODY)))),
                    )
                    .child(list)
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(7.))
                            .flex()
                            .gap(px(14.))
                            .border_t_1()
                            .border_color(p.rule)
                            .text_size(px(ts::META))
                            .text_color(p.faint)
                            .when(on_row, |d| d.child("\u{21a9} go to the words"))
                            .children(count)
                            .child(div().flex_1())
                            .children(crate::shortcuts::keys_for("app::OpenPalette", cx).map(|k| format!("{k} notes"))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Block, BlockKind, Mode, Side};
    use chrono::FixedOffset;

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn thread(id: &str, quote: &str, status: ThreadStatus, mind: &str, opened: &str, start: usize) -> Thread {
        Thread {
            id: id.into(),
            start,
            end: start + quote.len(),
            quote: quote.into(),
            kind: ThoughtKind::Question,
            messages: vec![],
            drawing: None,
            status,
            side: Side::Right,
            mind_name: mind.into(),
            tag_offset: (0., 0.),
            created_at: if opened.is_empty() { DateTime::<Utc>::default() } else { at(opened) },
            reason: None,
            ended_at: None,
            first_quote: None,
            ..Default::default()
        }
    }

    fn note(id: &str, title: &str, folder: Option<&str>, threads: Vec<Thread>) -> Note {
        let mut n = Note::new(title, Mode::ThinksWithYou, folder.map(Into::into));
        n.id = id.into();
        n.blocks = vec![Block { kind: BlockKind::Title, text: title.into() }];
        n.threads = threads;
        n.created_at = at("2026-01-01T00:00:00Z");
        n
    }

    fn mind_of(id: &str) -> String {
        if id == "skeptic" { "Skeptic".into() } else { "Inkling".into() }
    }

    fn keys(rows: &[Row]) -> Vec<String> {
        rows.iter().map(|r| format!("{}/{}", r.key.note, r.key.thread)).collect()
    }

    use ThreadStatus::{Dismissed, Open, Resolved};

    #[std::prelude::v1::test]
    fn lists_only_open_threads_oldest_first() {
        let notes = vec![
            note("plan", "Sub-50 10k by March", None, vec![
                thread("t1", "5:30", Open, "Inkling", "2026-10-10T09:00:00Z", 40),
                thread("t2", "add a kilometre", Open, "Inkling", "2026-10-12T09:00:00Z", 80),
                thread("t3", "tied", Resolved, "Inkling", "2026-09-01T09:00:00Z", 10),
            ]),
            note("budget", "Q3 budget", None, vec![
                thread("b1", "one node", Open, "Skeptic", "2026-09-20T09:00:00Z", 30),
                thread("b2", "snipped", Dismissed, "Skeptic", "2026-08-01T09:00:00Z", 5),
            ]),
            note("empty", "Groceries", None, vec![]),
        ];
        let rows = collect(&notes, &[], None, mind_of);
        assert_eq!(keys(&rows), ["budget/b1", "plan/t1", "plan/t2"]);
        assert!(rows.iter().all(|r| r.state == State::Open && !r.here));
        // ties: the same instant goes by the older note, then reading order
        let same = "2026-11-06T12:00:00Z";
        let mut older = note("older", "Older", None, vec![thread("z", "z", Open, "", same, 900)]);
        older.created_at = at("2025-01-01T00:00:00Z");
        let notes = vec![
            note("newer", "Newer", None, vec![thread("b", "b", Open, "", same, 50), thread("a", "a", Open, "", same, 10)]),
            older,
        ];
        assert_eq!(keys(&collect(&notes, &[], None, mind_of)), ["older/z", "newer/a", "newer/b"]);
    }

    #[std::prelude::v1::test]
    fn a_thread_of_your_own_is_a_loose_end_named_you() {
        let mut mine = thread("y1", "four days a week", Open, "", "2026-10-01T09:00:00Z", 20);
        mine.own = true;
        let mut cut = thread("y2", "a cut one", Dismissed, "", "2026-09-01T09:00:00Z", 60);
        cut.own = true;
        let notes = vec![note("plan", "Plan", None, vec![mine, cut, thread("t1", "5:30", Open, "Inkling", "2026-10-10T09:00:00Z", 40)])];
        let rows = collect(&notes, &[], None, mind_of);
        assert_eq!(keys(&rows), ["plan/y1", "plan/t1"], "listed while open, oldest first, like any thread");
        assert_eq!((rows[0].mind.as_str(), rows[0].own, mark(&rows[0])), ("You", true, "?"));
        assert!(!rows[1].own && rows[1].mind == "Inkling");
        assert_eq!(narrow(&rows, "you"), [0], "typing You finds your own");
    }

    #[std::prelude::v1::test]
    fn the_pages_copy_wins_for_the_open_note() {
        let stored = note("plan", "Plan", None, vec![thread("t1", "a", Open, "Inkling", "2026-10-10T09:00:00Z", 0)]);
        let mut live = stored.clone();
        live.threads[0].status = Resolved;
        live.threads.push(thread("t9", "just now", Open, "Inkling", "2026-11-07T09:00:00Z", 5));
        let other = note("other", "Other", None, vec![thread("o1", "o", Open, "Inkling", "2026-10-01T09:00:00Z", 0)]);
        let rows = collect(&[stored, other], &[], Some(&live), mind_of);
        assert_eq!(keys(&rows), ["other/o1", "plan/t9"], "tied off on the page before the save; opened a moment ago");
        assert!(rows[1].here && !rows[0].here);
        // a note the store does not have yet (made a moment ago) still counts
        let fresh = note("fresh", "Fresh", None, vec![thread("f1", "f", Open, "", "2026-11-07T10:00:00Z", 0)]);
        assert_eq!(keys(&collect(&[], &[], Some(&fresh), mind_of)), ["fresh/f1"]);
    }

    #[std::prelude::v1::test]
    fn a_thread_between_notes_is_one_row_under_its_home() {
        let home = note("plan", "Plan", None, vec![thread("b1", "Saturday 14 March", Open, "Inkling", "2026-11-06T18:00:00Z", 0)]);
        let far = note("cal", "Calendar", None, vec![]);
        // the calendar open on the page: the thread visits it on its own words
        let mut live = far.clone();
        live.threads.push(thread("b1", "Friday 13 March", Open, "Inkling", "2026-11-06T18:00:00Z", 0));
        live.elsewhere.push(crate::store::Elsewhere { thread: "b1".into(), note: "plan".into(), visiting: true, ..Default::default() });
        assert_eq!(keys(&collect(&[home, far], &[], Some(&live), mind_of)), ["plan/b1"]);
    }

    #[std::prelude::v1::test]
    fn the_same_thread_id_in_two_notes_is_two_rows() {
        let notes = vec![
            note("a", "A", None, vec![thread("h01", "first", Open, "Inkling", "2026-10-01T09:00:00Z", 0)]),
            note("b", "B", None, vec![thread("h01", "second", Open, "Inkling", "2026-10-02T09:00:00Z", 0)]),
        ];
        let mut rows = collect(&notes, &[], None, mind_of);
        assert_eq!(keys(&rows), ["a/h01", "b/h01"]);
        // and each reads its own state
        let mut notes2 = notes.clone();
        notes2[1].threads[0].status = Resolved;
        refresh(&mut rows, &notes2, None);
        assert_eq!((rows[0].state, rows[1].state), (State::Open, State::Tied));
    }

    #[std::prelude::v1::test]
    fn rows_name_the_folder_note_and_mind() {
        let folders = vec![Folder { id: "f-work".into(), name: "Work".into(), collapsed: true }];
        let mut budget = note("budget", "Q3 budget", Some("f-work"), vec![thread("b1", "We cut the bill\n\nby  moving", Open, "Skeptic", "2026-10-01T09:00:00Z", 0)]);
        budget.mind_id = "skeptic".into();
        let notes = vec![
            budget,
            note("untitled", "  ", None, vec![thread("u1", "u", Open, "", "2026-10-02T09:00:00Z", 0)]),
            note("orphan", "Lost", Some("removed-folder"), vec![thread("o1", "o", Open, "Editor", "2026-10-03T09:00:00Z", 0)]),
        ];
        let rows = collect(&notes, &folders, None, mind_of);
        assert_eq!((rows[0].folder.as_deref(), rows[0].title.as_str(), rows[0].mind.as_str()), (Some("Work"), "Q3 budget", "Skeptic"));
        assert_eq!(rows[0].quote, "We cut the bill by moving", "a quote across paragraphs reads as one line");
        assert_eq!(rows[0].own_mind, "Skeptic");
        assert_eq!((rows[1].title.as_str(), rows[1].mind.as_str(), rows[1].mind_name.as_str()), ("Untitled", "Inkling", ""), "a thread with no mind's name says Inkling, as its tag does");
        assert_eq!(rows[2].folder, None, "a removed folder is not named");
        assert!(!rows[0].correction && mark(&rows[0]) == "?");
        let mut fix = notes[2].clone();
        fix.threads[0].kind = ThoughtKind::Correction;
        assert_eq!(mark(&collect(&[fix], &[], None, mind_of)[0]), "\u{d7}");
    }

    #[std::prelude::v1::test]
    fn ages_in_calendar_days() {
        let ist = FixedOffset::east_opt(5 * 3600 + 1800).unwrap();
        let now = at("2026-11-07T10:00:00Z").with_timezone(&ist); // 15:30 in IST
        let age = |s: &str| age_words(at(s), &now);
        assert_eq!(age("2026-11-07T09:59:00Z"), "today");
        assert_eq!(age("2026-11-06T19:00:00Z"), "today", "00:30 on the 7th in IST: today, not yesterday");
        assert_eq!(age("2026-11-06T17:00:00Z"), "yesterday");
        assert_eq!(age("2026-11-04T10:00:00Z"), "3 days ago");
        assert_eq!(age("2026-10-25T10:00:00Z"), "13 days ago");
        assert_eq!(age("2026-10-24T10:00:00Z"), "2 weeks ago");
        assert_eq!(age("2026-09-09T10:00:00Z"), "8 weeks ago");
        assert_eq!(age("2026-09-08T10:00:00Z"), "2 months ago");
        assert_eq!(age("2025-11-08T10:00:00Z"), "11 months ago");
        assert_eq!(age("2025-11-07T10:00:00Z"), "a year ago");
        assert_eq!(age("2023-10-01T10:00:00Z"), "3 years ago");
        assert_eq!(age("2026-11-09T10:00:00Z"), "today", "a clock that moved");
        // a thread with no date shows no age and sorts first
        let notes = vec![note("n", "N", None, vec![thread("dated", "d", Open, "", "2026-01-01T00:00:00Z", 0), thread("undated", "u", Open, "", "", 9)])];
        let rows = collect(&notes, &[], None, mind_of);
        assert_eq!(keys(&rows), ["n/undated", "n/dated"]);
        assert_eq!(rows[0].opened, None);
    }

    #[std::prelude::v1::test]
    fn typing_narrows_by_every_word() {
        let folders = vec![Folder { id: "f".into(), name: "Work".into(), collapsed: false }];
        let notes = vec![
            note("budget", "Q3 budget", Some("f"), vec![thread("b1", "one node", Open, "Skeptic", "2026-10-01T09:00:00Z", 0)]),
            note("plan", "Sub-50 10k by March", None, vec![
                thread("t2", "add a kilometre every week", Open, "Inkling", "2026-10-02T09:00:00Z", 0),
                thread("k", "\u{212A}elvin and İstanbul", Open, "Inkling", "2026-10-03T09:00:00Z", 9),
            ]),
        ];
        let rows = collect(&notes, &folders, None, mind_of);
        let found = |q: &str| narrow(&rows, q).into_iter().map(|i| rows[i].key.thread.clone()).collect::<Vec<_>>();
        assert_eq!(found(""), ["b1", "t2", "k"]);
        assert_eq!(found("   "), ["b1", "t2", "k"]);
        assert_eq!(found("KILO"), ["t2"], "the quote, ignoring case");
        assert_eq!(found("skeptic budget"), ["b1"], "every word, anywhere: the mind and the title");
        assert_eq!(found("work"), ["b1"], "the folder");
        assert_eq!(found("march week"), ["t2"]);
        assert_eq!(found("skeptic march"), Vec::<String>::new());
        assert_eq!(found("kelvin"), ["k"], "the Kelvin sign lowercases to k");
        assert_eq!(found("İstanbul"), ["k"]);
    }

    #[std::prelude::v1::test]
    fn the_rows_stay_and_their_state_is_read_again() {
        let mut notes = vec![
            note("a", "A", None, vec![thread("1", "one", Open, "", "2026-10-01T09:00:00Z", 0), thread("2", "two", Open, "", "2026-10-02T09:00:00Z", 5)]),
            note("b", "B", None, vec![thread("3", "three", Open, "", "2026-10-03T09:00:00Z", 0)]),
        ];
        let mut rows = collect(&notes, &[], None, mind_of);
        notes[0].threads[0].status = Resolved;
        notes[0].threads[1].status = Dismissed;
        notes.remove(1);
        refresh(&mut rows, &notes, None);
        assert_eq!(rows.iter().map(|r| r.state.word()).collect::<Vec<_>>(), ["tied off", "dismissed", "gone"]);
        // the page's copy is read for its note
        let mut live = notes[0].clone();
        live.threads[0].status = Open;
        refresh(&mut rows, &notes, Some(&live));
        assert_eq!(rows[0].state, State::Open);
    }

    #[std::prelude::v1::test]
    fn reopening_walks_the_list() {
        let notes = vec![note("n", "N", None, (0..4).map(|i| thread(&format!("t{i}"), "q", Open, "", &format!("2026-10-0{}T09:00:00Z", i + 1), 0)).collect())];
        let rows = collect(&notes, &[], None, mind_of);
        let key = |t: &str| Key { note: "n".into(), thread: t.into() };
        assert_eq!(start_at(&rows, None), 0);
        assert_eq!(start_at(&rows, Some(&(key("t1"), 1))), 2, "the row after the one you went to");
        assert_eq!(start_at(&rows, Some(&(key("t3"), 3))), 3, "the last row stays the last");
        assert_eq!(start_at(&rows, Some(&(key("gone"), 2))), 2, "tied off: the row now in its place");
        assert_eq!(start_at(&rows, Some(&(key("gone"), 9))), 3);
        assert_eq!(start_at(&[], Some(&(key("t1"), 1))), 0);
    }

    #[std::prelude::v1::test]
    fn a_thousand_notes_build_and_narrow_quickly() {
        let words = ["pace", "budget", "cache", "bridge", "kilometre", "node", "week", "goal"];
        let notes: Vec<Note> = (0..1000)
            .map(|n| {
                let threads = (0..20)
                    .map(|i| {
                        let status = if i % 10 == 0 { Open } else if i % 3 == 0 { Dismissed } else { Resolved };
                        let quote = format!("the {} for note {n} and thread {i}", words[(n + i) % words.len()]);
                        let day = 1 + (n * 20 + i) % 28;
                        thread(&format!("t{i}"), &quote, status, "Inkling", &format!("2026-10-{day:02}T09:00:00Z"), i * 40)
                    })
                    .collect();
                note(&format!("n{n:04}"), &format!("Note {n}"), None, threads)
            })
            .collect();
        let started = std::time::Instant::now();
        let rows = collect(&notes, &[], None, mind_of);
        let built = started.elapsed();
        let started = std::time::Instant::now();
        let shown = narrow(&rows, "kilometre note");
        let narrowed = started.elapsed();
        let mut again = rows.clone();
        let started = std::time::Instant::now();
        refresh(&mut again, &notes, None);
        let refreshed = started.elapsed();
        assert_eq!(rows.len(), 2000);
        assert!(!shown.is_empty() && shown.len() < rows.len());
        eprintln!("loose ends, 1,000 notes, 20,000 threads, 2,000 open: built {built:?}, narrowed {narrowed:?}, refreshed {refreshed:?}");
        // generous bounds (a debug build on a busy machine); the real numbers are printed
        assert!(built.as_millis() < 250 && narrowed.as_millis() < 50 && refreshed.as_millis() < 50, "{built:?} {narrowed:?} {refreshed:?}");
    }
}

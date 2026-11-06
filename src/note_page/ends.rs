//! Threads between notes, as the page shows them: a thread that also quotes words in
//! another note (`design/features/threads-between-notes.md`).
//!
//! Its tab carries a second loop beside the first. Unfolded, its tag says `between notes`
//! and quotes the other words in the writer's typeface, on a seam in the thread's colour,
//! with the other note's title and when it was last edited, and Go there (⌥⌘→), which opens
//! that note at those words with the same thread open. A reply or a second look on it is
//! answered by the thread's own mind and carries the other end as it is now.
//!
//! The data is `crate::between`'s: the far end (`Note.elsewhere`), the visiting ends a note
//! is given when it opens, and what goes home when it saves. This is the page's side.

use chrono::{Datelike, Local, NaiveDate};
use gpui_kit::*;
use gpui_kit::prelude::*;
use serde_json::{Value, json};

use super::{CHROME_H, NotePage, PageEvent};
use crate::store::{Elsewhere, Mind, Placement, Side, Store, Thread, ThreadStatus};
use crate::theme::{SERIF, Theme, ts};
use crate::ui::{Keyed, icon};

/// How far the second loop sits from the first: they overlap like two links of a chain.
pub(super) const LOOP_STEP: f32 = 5.;

/// The page's part in threads between notes.
#[derive(Default)]
pub(super) struct Ends {
    /// The thread followed from the note just left: it opens once its other end is in.
    following: Option<String>,
    /// Words to bring into view once the note is laid out: the followed thread's.
    reveal: Option<String>,
}

impl NotePage {
    /// The far end of thread `id`, when it is a thread between notes.
    pub(super) fn far_end(&self, id: &str) -> Option<&Elsewhere> {
        self.note.elsewhere_of(id)
    }

    /// What a thread between notes says after its mind's name on its tag.
    pub(super) fn between_kind(&self, id: &str) -> Option<&'static str> {
        self.far_end(id).map(|_| "between notes")
    }

    /// The line under the other words, and whether that note is still there to go to.
    /// The title is looked up by id each time, so it follows a rename.
    fn far_line_of(&self, e: &Elsewhere, cx: &App) -> (String, bool) {
        let today = Local::now().date_naive();
        match crate::between::other_note(&cx.global::<Store>().notes, e) {
            Some(n) => (far_line(&n.title, Some(n.updated_at.with_timezone(&Local).date_naive()), today), true),
            None => (far_line(&e.title, None, today), false),
        }
    }

    /// The other note's title as it is now (as last seen, once it is deleted).
    fn other_title(&self, e: &Elsewhere, cx: &App) -> String {
        crate::between::other_note(&cx.global::<Store>().notes, e).map_or_else(|| e.title.clone(), |n| n.title.clone())
    }

    // ----- following -------------------------------------------------------------------

    /// Go there: open the other note at the other end's words, with this thread open
    /// there. Once the other note is deleted there is nowhere to go.
    pub(super) fn follow(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(other) = self.far_end(id).map(|e| e.note.clone()) else { return };
        if cx.global::<Store>().note(&other).is_none() {
            return;
        }
        self.ends.following = Some(id.to_string());
        cx.emit(PageEvent::OpenNote(other));
    }

    /// ⌥⌘→: follow the open thread to its other end.
    pub fn follow_active(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.clone() {
            self.follow(&id, cx);
        }
    }

    /// A note has just opened (`open`). A thread followed here opens with its reply box
    /// taking the keyboard, as ⌥⌘↓ leaves it, and its words come into view once the note
    /// is laid out. The thread dismissed last, if it has an end here, is put back on this
    /// note's words by Undo.
    pub(super) fn arrive(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((id, range, _)) = self.last_dismissed.as_mut() {
            if let Some(t) = self.note.threads.iter().find(|t| t.id == *id) {
                *range = t.start..t.end;
            }
        }
        let Some(id) = self.ends.following.take() else { return };
        let Some(start) = self.editor.read(cx).anchor(&id).map(|a| a.range.start) else { return };
        self.activate(Some(id.clone()), window, cx);
        self.editor.update(cx, |e, cx| e.move_cursor_to(start, cx));
        if let Some(input) = self.replies.get(&id).cloned() {
            input.update(cx, |i, cx| i.focus(window, cx));
        }
        self.ends.reveal = Some(id);
    }

    /// Bring a followed thread's words into view once the editor has laid the note out
    /// (called as the page renders): about a third of the way down the window, so the open
    /// tag has room under them.
    pub(super) fn reveal_followed(&mut self, window: &mut Window, cx: &App) {
        let Some(id) = self.ends.reveal.clone() else { return };
        let ed = self.editor.read(cx);
        let Some(origin) = ed.layout.as_ref().map(|l| l.origin) else {
            // laid out in this frame: look again in the next
            window.request_animation_frame();
            return;
        };
        let top = ed.anchor_rects(&id).first().map(|r| (origin.y + r.origin.y).as_f32());
        self.ends.reveal = None;
        if let Some(dy) = top.and_then(|top| reveal_by(top, window.viewport_size().height.as_f32())) {
            let offset = self.scroll.offset();
            self.scroll.set_offset(point(offset.x, (offset.y + px(dy)).min(px(0.))));
        }
    }

    /// A far end shown in this note for the first time hangs in the margin less crowded
    /// near its words, and when neither is, on the side its home end hangs, so following it
    /// keeps your eye on the same side. Chosen once its words are laid out, and marked
    /// `placed`: the next save takes the side home, and from then on it is its own.
    pub(super) fn side_new_far_ends(&mut self, cx: &mut Context<Self>) {
        if self.editor.read(cx).layout.is_none() {
            return;
        }
        let fresh: Vec<(String, Side)> =
            self.note.elsewhere.iter().filter(|e| e.visiting && !e.placed).map(|e| (e.thread.clone(), e.side)).collect();
        for (id, home) in fresh {
            // a far end with no words here (dismissed) waits until it has some
            let Some(side) = self.first_side(&id, home, cx) else { continue };
            if let Some(t) = self.note.threads.iter_mut().find(|t| t.id == id) {
                t.side = side;
            }
            if let Some(e) = self.note.elsewhere.iter_mut().find(|e| e.visiting && e.thread == id) {
                e.placed = true;
            }
        }
    }

    /// Where thread `id` (a far end shown here for the first time) hangs: as `pick_side`
    /// weighs a new thread, the margin with fewer open threads near its words, but on a tie
    /// the side of its home end (`home`) rather than taking turns. `None` while it has no
    /// words laid out here.
    fn first_side(&self, id: &str, home: Side, cx: &App) -> Option<Side> {
        let ed = self.editor.read(cx);
        let layout = ed.layout.as_ref()?;
        let top = |t: &str| ed.anchor(t).and_then(|a| layout.rects(a.range.clone()).first().map(|r| r.origin.y.as_f32()));
        let y = top(id)?;
        if cx.global::<Store>().settings.placement != Placement::Both {
            return Some(Side::Right);
        }
        let near = |side: Side| {
            self.note
                .threads
                .iter()
                .filter(|t| t.id != id && t.status == ThreadStatus::Open && t.side == side)
                .filter(|t| top(&t.id).is_some_and(|ty| (ty - y).abs() < 160.))
                .count()
        };
        Some(match near(Side::Left).cmp(&near(Side::Right)) {
            std::cmp::Ordering::Less => Side::Left,
            std::cmp::Ordering::Greater => Side::Right,
            std::cmp::Ordering::Equal => home,
        })
    }

    // ----- replies -----------------------------------------------------------------------

    /// A reply or a second look on a thread between notes is answered by the thread's own
    /// mind (the home note's, from either end) and carries the other end as it is now.
    pub(super) fn between_reply(&self, t: &Thread, mut input: Value, cx: &App) -> Value {
        if self.far_end(&t.id).is_none() {
            return input;
        }
        let store = cx.global::<Store>();
        if let Some(mind) = mind_named(&store.minds, &t.mind_name).filter(|m| m.name != self.mind.name) {
            input["ctx"] = self.context_with(&mind, cx);
        }
        if let Some(e) = crate::between::elsewhere_wire(&store.notes, &self.note, &t.id) {
            input["elsewhere"] = e;
        }
        input
    }

    /// What Copy puts on the clipboard: the thread, and for a thread between notes a line
    /// naming the other end under its words, as Export prints it.
    pub(super) fn copy_text(&self, id: &str, cx: &App) -> String {
        let text = self.thread_text(id);
        let (Some(e), Some(t)) = (self.far_end(id), self.note.threads.iter().find(|t| t.id == id)) else { return text };
        with_far_line(&text, &t.quote, &self.other_title(e, cx), &e.quote)
    }

    // ----- drawing -----------------------------------------------------------------------

    /// The second loop on a folded tab, beside the first: this thread goes on to another
    /// note. Drawn like the first, so the difference is shape, not colour.
    pub(super) fn second_loop(right: bool, color: Hsla) -> Div {
        Self::tag_ring(right, color).when(right, |d| d.left(px(9. + LOOP_STEP))).when(!right, |d| d.right(px(9. + LOOP_STEP)))
    }

    /// The second loop on an unfolded tag, beside the loop at `ring_x` (its centre), on the
    /// side away from the tag's edge.
    pub(super) fn panel_second_loop(ring_x: f32, ring_on_left: bool, top: Pixels, color: Hsla) -> Div {
        let x = ring_x + if ring_on_left { LOOP_STEP } else { -LOOP_STEP };
        div().absolute().top(top).left(px(x - 4.)).size(px(8.)).rounded_full().border(px(1.5)).border_color(color.opacity(0.9))
    }

    /// The far end inside an unfolded tag: the other words in the writer's typeface, on a
    /// seam in the thread's colour, and under them the other note's title and when it was
    /// edited, with Go there. Once that note is deleted: `· deleted`, the words kept, and no
    /// Go there.
    pub(super) fn far_end_block(&mut self, thread: &Thread, color: Hsla, cx: &mut Context<Self>) -> Option<AnyElement> {
        let e = self.far_end(&thread.id)?.clone();
        let p = Theme::get(cx).p;
        let (line, there) = self.far_line_of(&e, cx);
        let seed = crate::stitch::seed_of(&thread.id);
        // a quote's seam, as the note sews one beside a quote, in the thread that ties them
        let seam = canvas(
            |_, _, _| (),
            move |b, _, window, _| crate::sewn::seam(window, b.origin.x + px(1.5), b.origin.y + px(3.), b.origin.y + b.size.height - px(3.), color, seed),
        )
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .w(px(4.));
        let open = self.active.as_deref() == Some(thread.id.as_str());
        let go = there.then(|| {
            let id = thread.id.clone();
            let tip: SharedString = format!("Open {} at these words. The thread runs under the cloth to them.", self.other_title(&e, cx)).into();
            let go = div()
                .id(SharedString::from(format!("go-{}", thread.id)))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(3.))
                .text_size(px(ts::SMALL))
                .text_color(p.graphite)
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .active(|s| s.opacity(0.7))
                .child("Go there")
                .child(icon("arrow-right", 11., p.graphite))
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip.clone()).action(&crate::app::FollowThread, None).build(window, cx)
                });
            // the open tag's controls take the keyboard; a tag unfolded under the pointer is
            // clicked, and the click goes no further than Go there (not opening the tag here)
            if open {
                go.keyed_as("tag:go", 4., color, cx, move |this, _, cx| {
                    cx.stop_propagation();
                    this.follow(&id, cx)
                })
            } else {
                go.on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.follow(&id, cx)
                }))
            }
        });
        Some(
            div()
                .relative()
                .pl(px(13.))
                .py(px(1.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(seam)
                .child(div().font_family(SERIF).italic().text_size(px(15.)).line_height(px(21.)).text_color(p.ink).child(e.quote.replace('\n', " ")))
                // Go there beside the title and date when they fit on one line, else under them
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap_x(px(10.))
                        .gap_y(px(2.))
                        .child(div().min_w(px(0.)).max_w_full().truncate().text_size(px(ts::SMALL)).text_color(p.faint).child(line))
                        .children(go),
                )
                .into_any_element(),
        )
    }

    // ----- the dump ----------------------------------------------------------------------

    /// The dump's `ends`: each thread between notes on this page as its tag shows it (`line`
    /// under the other words, `go` when Go there is offered), the thread being followed and
    /// whether the open thread's words are on screen (`in_view`), and the sidebar's counts
    /// of open threads by title (`sidebar`, notes with none left out).
    pub(super) fn ends_state(&self, window: &Window, cx: &App) -> Value {
        let short = |id: &str| id.chars().take(8).collect::<String>();
        let tags: Vec<Value> = self
            .note
            .elsewhere
            .iter()
            .map(|e| {
                let (line, there) = self.far_line_of(e, cx);
                let side = self.note.threads.iter().find(|t| t.id == e.thread).map(|t| format!("{:?}", t.side));
                json!({"thread": short(&e.thread), "label": self.between_kind(&e.thread), "line": line, "go": there, "side": side})
            })
            .collect();
        let in_view = self.active.as_ref().and_then(|id| {
            let ed = self.editor.read(cx);
            let origin = ed.layout.as_ref()?.origin;
            let rects = ed.anchor_rects(id);
            let (first, last) = (rects.first()?, rects.last()?);
            let top = (origin.y + first.origin.y).as_f32();
            let bottom = (origin.y + last.origin.y + last.size.height).as_f32();
            Some(top >= CHROME_H && bottom <= window.viewport_size().height.as_f32())
        });
        let notes = &cx.global::<Store>().notes;
        let sidebar: serde_json::Map<String, Value> = notes
            .iter()
            .filter_map(|n| {
                let open = crate::between::open_here(notes, n);
                (open > 0).then(|| (n.title.clone(), json!(open)))
            })
            .collect();
        json!({
            "tags": tags,
            "following": self.ends.following.as_deref().map(short),
            "reveal": self.ends.reveal.as_deref().map(short),
            "in_view": in_view,
            "sidebar": sidebar,
        })
    }
}

/// The mind called `name`: one of yours (Inkling among them), or a template seated in a
/// room and never made into one of yours.
fn mind_named(minds: &[Mind], name: &str) -> Option<Mind> {
    let name = if name.trim().is_empty() { "Inkling" } else { name };
    minds
        .iter()
        .find(|m| m.name == name)
        .cloned()
        .or_else(|| (name == "Inkling").then(Mind::inkling))
        .or_else(|| crate::store::mind_templates().into_iter().find(|m| m.name == name))
}

/// The line under the other words: the other note's title, then when it was last edited
/// (`today`, `yesterday`, else `edited` and the date, with its year when it is not this
/// one's), or `deleted` once the note is gone (`edited` is `None`).
pub(super) fn far_line(title: &str, edited: Option<NaiveDate>, today: NaiveDate) -> String {
    let title = if title.trim().is_empty() { "Untitled" } else { title.trim() };
    let Some(day) = edited else { return format!("{title} \u{b7} deleted") };
    let when = if day == today {
        "today".to_string()
    } else if today.pred_opt() == Some(day) {
        "yesterday".to_string()
    } else if day.year() == today.year() {
        format!("edited {}", day.format("%-d %b"))
    } else {
        format!("edited {}", day.format("%-d %b %Y"))
    };
    format!("{title} \u{b7} {when}")
}

/// `text`, a copied thread (it starts `> ‹quote›`), with the line naming the other end
/// under its quote: `> *Also in “Race calendar”:* “Friday 13 March”`.
fn with_far_line(text: &str, quote: &str, title: &str, other: &str) -> String {
    let at = "> ".len() + quote.len();
    if !text.starts_with("> ") || at > text.len() || !text.is_char_boundary(at) {
        return text.to_string();
    }
    let title = if title.trim().is_empty() { "Untitled" } else { title.trim() };
    format!("{}\n> *Also in \u{201c}{title}\u{201d}:* \u{201c}{}\u{201d}{}", &text[..at], other.replace('\n', " "), &text[at..])
}

/// How far to move the page (down, when positive) so words whose top is `top` in a window
/// `view` tall sit about a third of the way down; `None` when they already sit in the upper
/// part of the page, clear of the cloth along the top.
fn reveal_by(top: f32, view: f32) -> Option<f32> {
    if top >= CHROME_H + 40. && top <= view * 0.55 {
        return None;
    }
    Some(view * 0.32 - top)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    #[std::prelude::v1::test]
    fn the_far_line_says_when_the_other_note_was_edited_or_that_it_is_gone() {
        let today = day(2026, 11, 7);
        assert_eq!(far_line("Race-week plan", Some(today), today), "Race-week plan \u{b7} today");
        assert_eq!(far_line("Race calendar", Some(day(2026, 11, 6)), today), "Race calendar \u{b7} yesterday");
        assert_eq!(far_line("Race calendar", Some(day(2026, 9, 3)), today), "Race calendar \u{b7} edited 3 Sep");
        assert_eq!(far_line("Race calendar", Some(day(2025, 12, 31)), today), "Race calendar \u{b7} edited 31 Dec 2025");
        // yesterday across a new year
        assert_eq!(far_line("Log", Some(day(2025, 12, 31)), day(2026, 1, 1)), "Log \u{b7} yesterday");
        assert_eq!(far_line("Race calendar", None, today), "Race calendar \u{b7} deleted");
        assert_eq!(far_line("  ", Some(day(2026, 9, 3)), today), "Untitled \u{b7} edited 3 Sep");
    }

    #[std::prelude::v1::test]
    fn a_copied_thread_names_the_other_end_under_its_words() {
        let copied = "> Saturday 14 March\n\nInkling: Which day is right?";
        assert_eq!(
            with_far_line(copied, "Saturday 14 March", "Race calendar", "Friday 13 March"),
            "> Saturday 14 March\n> *Also in \u{201c}Race calendar\u{201d}:* \u{201c}Friday 13 March\u{201d}\n\nInkling: Which day is right?"
        );
        // a quote over two paragraphs stays whole; the other words keep to one line
        let copied = "> one\ntwo\n\nInkling: ?";
        assert_eq!(with_far_line(copied, "one\ntwo", "", "a\nb"), "> one\ntwo\n> *Also in \u{201c}Untitled\u{201d}:* \u{201c}a b\u{201d}\n\nInkling: ?");
        // not a copied thread, or a quote that is not its start: left as it is
        assert_eq!(with_far_line("", "x", "T", "y"), "");
        assert_eq!(with_far_line("> é", "\u{e9}x", "T", "y"), "> é");
    }

    #[std::prelude::v1::test]
    fn followed_words_come_to_a_third_of_the_window_unless_they_are_in_view() {
        // already in the upper part of the page: stay
        assert_eq!(reveal_by(200., 800.), None);
        assert_eq!(reveal_by(CHROME_H + 40., 800.), None);
        // far below: the page moves up to bring them to a third of the way down
        assert_eq!(reveal_by(2000., 800.), Some(256. - 2000.));
        // low in the window: up a little
        assert_eq!(reveal_by(700., 800.), Some(256. - 700.));
        // under the cloth along the top: down
        assert_eq!(reveal_by(10., 800.), Some(246.));
    }

    #[std::prelude::v1::test]
    fn a_thread_is_answered_by_the_mind_that_asked() {
        let skeptic = Mind { id: "m1".into(), name: "Skeptic".into(), how_it_thinks: "Mine.".into(), ..Mind::inkling() };
        let minds = vec![Mind::inkling(), skeptic];
        assert_eq!(mind_named(&minds, "Skeptic").map(|m| (m.id, m.how_it_thinks)), Some(("m1".into(), "Mine.".into())));
        assert_eq!(mind_named(&minds, "Inkling").map(|m| m.id), Some(crate::store::DEFAULT_MIND.into()));
        // an old thread with no name was Inkling's
        assert_eq!(mind_named(&[], "").map(|m| m.name), Some("Inkling".into()));
        // a template seated in a room, never made into a mind of yours
        assert_eq!(mind_named(&minds, "Editor").map(|m| m.name), Some("Editor".into()));
        assert!(mind_named(&minds, "Nobody").is_none());
    }
}

//! The page's side of answers in the note (`design/features/answer-in-the-note.md`): a
//! thread tied off by words you wrote in the note keeps them as its answer, follows them
//! through your edits, and its knot sits after them. Pointing at the knot washes the
//! answer; clicking it opens the thread at its question, whose tag says "Answered in your
//! note:" and takes you back. Write it in sets the same answer for your reply, written in.
//!
//! The rules the app checks on its own are in `crate::answers`; the hooks in
//! `note_page.rs` are one line each where they can be.

use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

use super::{Metrics, NotePage, anim_since, knot_scale, mind_thread, nearest_match, short_title};
use crate::answers as rules;
use crate::editor::Anchor;
use crate::store::{Answer, Note, Store, Thread, ThreadStatus, TiedBy};
use crate::theme::{SANS, SERIF, Theme, ts};
use crate::ui::Keyed;

/// How long an answer stays washed when you are taken to it.
pub(super) const WASH_FOR: Duration = Duration::from_millis(1600);
/// The knot after an answer: its radius, and the box that takes the pointer around it.
const KNOT_R: f32 = 3.4;
const KNOT_BOX: f32 = 14.;
/// Knots on the same words (several minds answered at once) sit side by side.
const KNOT_STEP: f32 = 9.;

/// Threads visiting this note from their home in another (threads between notes). Their
/// answer, if they have one, is in the home note's words: it is left as it is, so saving
/// sends it home unchanged, and words here never answer them.
pub(super) fn visiting_ids(note: &Note) -> std::collections::HashSet<String> {
    note.elsewhere.iter().filter(|e| e.visiting).map(|e| e.thread.clone()).collect()
}

/// The editor anchors for the answers of tied-off threads, when a note is loaded. An
/// answer whose offsets no longer cover its words is found again by its words, nearest
/// where it was; one whose words are gone is let go.
pub(super) fn load_answers(note: &mut Note, text: &str) -> Vec<Anchor> {
    let mut anchors = vec![];
    let away = visiting_ids(note);
    for t in note.threads.iter_mut().filter(|t| t.status == ThreadStatus::Resolved && !away.contains(&t.id)) {
        let Some(a) = t.answer.clone() else { continue };
        let found = if text.get(a.start..a.end).is_some_and(|s| s == a.quote) {
            Some(a.start)
        } else {
            nearest_match(text, &a.quote, a.start)
        };
        match found {
            Some(i) => {
                let range = i..i + a.quote.len();
                anchors.push(Anchor { id: rules::anchor_id(&t.id), range: range.clone(), open: false });
                t.answer = Some(Answer { start: range.start, end: range.end, ..a });
            }
            None => t.answer = None,
        }
    }
    anchors
}

/// Each answer's range and words, back from the editor (as a thread's own are). An answer
/// whose words were all deleted is found again by its words if they are back (a redo),
/// else let go; its knot returns the next time the note opens.
pub(super) fn sync_answers(note: &mut Note, text: &str, anchors: &[Anchor]) {
    let away = visiting_ids(note);
    for t in note.threads.iter_mut().filter(|t| !away.contains(&t.id)) {
        let Some(a) = t.answer.clone() else { continue };
        let id = rules::anchor_id(&t.id);
        let live = anchors.iter().find(|x| x.id == id).map(|x| x.range.clone()).filter(|r| !r.is_empty() && r.end <= text.len());
        t.answer = match live {
            Some(r) => Some(Answer { start: r.start, end: r.end, quote: text[r].to_string() }),
            None => nearest_match(text, &a.quote, a.start).map(|i| Answer { start: i, end: i + a.quote.len(), ..a }),
        };
    }
}

impl NotePage {
    /// Before a thread's status changes. `Thread::set_status` keeps what tied it off (by
    /// hand unless something said otherwise) and lets the answer go when it opens again,
    /// remembering words that tied it off as not its answer; the page takes the answer's
    /// anchor and wash out of the editor.
    pub(super) fn before_status(&mut self, id: &str, status: ThreadStatus, cx: &mut Context<Self>) {
        let answered = self.note.threads.iter().any(|t| t.id == id && t.answer.is_some());
        if status == ThreadStatus::Open && answered {
            let aid = rules::anchor_id(id);
            self.editor.update(cx, |e, cx| {
                e.washes.retain(|(w, _)| *w != aid);
                e.remove_anchor(&aid, cx);
            });
        }
    }

    /// Tied-off threads whose answer's words are in the note: the ones with a knot.
    pub(super) fn knot_threads(&self, cx: &App) -> Vec<Thread> {
        let ed = self.editor.read(cx);
        let away = visiting_ids(&self.note);
        self.note
            .threads
            .iter()
            .filter(|t| t.status == ThreadStatus::Resolved && t.answer.is_some() && !away.contains(&t.id))
            .filter(|t| ed.anchor(&rules::anchor_id(&t.id)).is_some_and(|a| !a.range.is_empty()))
            .cloned()
            .collect()
    }

    fn thread_color(&self, t: &Thread, cx: &App) -> Hsla {
        mind_thread(&t.mind_name, &self.mind.name, cx.global::<Store>().settings.ink, Theme::get(cx))
    }

    fn wash_color(&self, t: &Thread, cx: &App) -> Hsla {
        let dark = Theme::get(cx).p.dark;
        self.thread_color(t, cx).opacity(if dark { 0.12 } else { 0.08 })
    }

    /// The knots: a small French knot after the last word of each answer, in its thread's
    /// colour, popping in as the thread is tied off. Point at one and the answer washes;
    /// click it and the thread opens at its question. Positioned in the page body, like the
    /// marks of Marks only.
    pub(super) fn answer_knots(&mut self, m: Metrics, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let p = Theme::get(cx).p;
        let now = Instant::now();
        let mut placed: Vec<(usize, usize)> = vec![];
        let mut out = vec![];
        for th in self.knot_threads(cx) {
            let aid = rules::anchor_id(&th.id);
            let (end, last) = {
                let ed = self.editor.read(cx);
                let Some(end) = ed.anchor(&aid).map(|a| a.range.end) else { continue };
                let Some(last) = ed.anchor_rects(&aid).last().copied() else { continue };
                (end, last)
            };
            // several knots on the same words sit side by side
            let nth = placed.iter().filter(|(e, _)| *e == end).count();
            placed.push((end, nth));
            let color = self.thread_color(&th, cx);
            let wash = self.wash_color(&th, cx);
            let scale = self.tying.get(&th.id).map_or(1., |at| knot_scale(anim_since(*at, now) - 0.55));
            let label = rules::knot_label(&th);
            let (id_hover, id_click) = (th.id.clone(), th.id.clone());
            let knot = canvas(
                |_, _, _| (),
                move |b, _, window, _| {
                    let c = b.origin + point(px(KNOT_BOX / 2.), px(KNOT_BOX / 2.));
                    crate::stitch::french_knot(window, c, px(KNOT_R), color, scale);
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full();
            out.push(
                div()
                    .id(SharedString::from(format!("knot-{}", th.id)))
                    .absolute()
                    .top(last.origin.y + last.size.height * 0.62 - px(KNOT_BOX / 2.))
                    .left(px(m.column_left()) + last.origin.x + last.size.width + px(nth as f32 * KNOT_STEP))
                    .size(px(KNOT_BOX))
                    .rounded_full()
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(label.clone()).build(window, cx))
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        let aid = rules::anchor_id(&id_hover);
                        this.editor.update(cx, |e, cx| {
                            e.washes.retain(|(w, _)| *w != aid);
                            if *hovered {
                                e.washes.push((aid, wash));
                            }
                            cx.notify();
                        });
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.go_to_question(&id_click, window, cx);
                    }))
                    .child(knot)
                    .into_any_element(),
            );
        }
        out
    }

    /// The open tag's "Answered in your note:" and the first words of the answer, in your
    /// typeface; clicking them (or Return on them) takes you there. Only while the words
    /// are in the note.
    pub(super) fn answer_row(&mut self, thread: &Thread, color: Hsla, cx: &mut Context<Self>) -> Option<AnyElement> {
        let a = thread.answer.as_ref()?;
        if !self.editor.read(cx).anchor(&rules::anchor_id(&thread.id)).is_some_and(|x| !x.range.is_empty()) {
            return None;
        }
        let p = Theme::get(cx).p;
        let words = short_title(&a.quote.replace('\n', " "), 64);
        let id = thread.id.clone();
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(div().font_family(SANS).text_size(px(ts::SMALL)).text_color(p.faint).child("Answered in your note:"))
                .child(
                    div()
                        .id(SharedString::from(format!("answer-{id}")))
                        .cursor_pointer()
                        .rounded(px(3.))
                        .font_family(SERIF)
                        .text_size(px(ts::BODY))
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .hover(move |s| s.bg(p.hover))
                        .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Show your answer").build(window, cx))
                        .child(format!("\u{201c}{words}\u{201d}"))
                        .keyed_as("tag:answer", 3., color, cx, move |this, window, cx| this.go_to_answer(&id, window, cx)),
                )
                .into_any_element(),
        )
    }

    /// Open a tied-off thread at its question: the page scrolls to its words and its tag
    /// opens (in Marks only, its card).
    pub(super) fn go_to_question(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let start = self.editor.read(cx).anchor(id).map(|a| a.range.start);
        self.activate(Some(id.to_string()), window, cx);
        if let Some(start) = start {
            self.editor.update(cx, |e, cx| {
                e.move_cursor_to(start, cx);
                e.focus(window, cx);
            });
            self.keep_cursor_visible(window, cx);
        }
    }

    /// Back to a thread's answer: the tag closes, the caret goes after the answer's last
    /// word, and the answer washes for a moment.
    pub(super) fn go_to_answer(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(end) = self.editor.read(cx).anchor(&rules::anchor_id(id)).filter(|a| !a.range.is_empty()).map(|a| a.range.end) else { return };
        self.close_thread(cx);
        self.editor.update(cx, |e, cx| {
            e.move_cursor_to(end, cx);
            e.focus(window, cx);
        });
        self.keep_cursor_visible(window, cx);
        self.wash_answer(id, WASH_FOR, cx);
    }

    /// Wash a thread's answer in its colour for `how_long` (going to it; Write it in).
    pub(crate) fn wash_answer(&mut self, id: &str, how_long: Duration, cx: &mut Context<Self>) {
        let Some(t) = self.note.threads.iter().find(|t| t.id == id).cloned() else { return };
        let (aid, wash) = (rules::anchor_id(id), self.wash_color(&t, cx));
        self.editor.update(cx, |e, cx| {
            e.washes.retain(|(w, _)| *w != aid);
            e.washes.push((aid.clone(), wash));
            cx.notify();
        });
        let editor = self.editor.clone();
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(how_long).await;
            let _ = editor.update(cx, |e, cx| {
                e.washes.retain(|(w, _)| *w != aid);
                cx.notify();
            });
        })
        .detach();
    }

    /// `knot: N` in a script: click thread N's knot (threads numbered as for `resolve:`).
    pub(super) fn click_knot(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.nth_thread(n).filter(|id| self.knot_threads(cx).iter().any(|t| &t.id == id));
        match id {
            Some(id) => self.go_to_question(&id, window, cx),
            None => log::warn!("script: thread {n} has no knot"),
        }
    }

    /// The answers and knots, for the script's dump.
    pub(super) fn dump_answers(&self, state: &mut Value, cx: &App) {
        let short = |id: &str| id.chars().take(8).collect::<String>();
        let ed = self.editor.read(cx);
        state["answers"] = self
            .note
            .threads
            .iter()
            .filter_map(|t| {
                let a = t.answer.as_ref()?;
                let range = ed.anchor(&rules::anchor_id(&t.id)).map(|x| [x.range.start, x.range.end]);
                Some(json!({"thread": short(&t.id), "quote": a.quote, "range": range, "tied_by": t.tied_by}))
            })
            .collect::<Vec<_>>()
            .into();
        let knots: Vec<String> = if self.just_my_words { vec![] } else { self.knot_threads(cx).iter().map(|t| short(&t.id)).collect() };
        state["knots"] = json!(knots);
        state["tied_by"] = self.note.threads.iter().filter_map(|t| t.tied_by.map(|b| (short(&t.id), json!(b)))).collect::<serde_json::Map<_, _>>().into();
        let washes: Vec<String> = ed.washes.iter().filter_map(|(w, _)| rules::thread_of(w).map(short)).collect();
        state["washed"] = json!(washes);
    }
}

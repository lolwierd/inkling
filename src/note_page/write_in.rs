//! Write it in (`design/features/answer-in-the-note.md`): when your reply in the margin
//! ties a thread off, its open tag offers to put that reply, word for word, under the
//! paragraph the thread is about, for you to rework in your own words. The new paragraph
//! is the thread's answer in the note (`answers.rs`), so its knot points there, and it is
//! not read again: its mind has already answered those words.
//!
//! The offer explains itself where it shows: what (your reply, just above it), why ("Your
//! note doesn't say this yet."), where ("under its paragraph"), how much ("word for word",
//! then the paragraph itself, washed, with the caret in it), and how to take it back (the
//! toast's Undo, or ⌘Z). Passing it by stores nothing.

use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{Value, json};

use super::NotePage;
use super::answers::WASH_FOR;
use crate::answers as rules;
use crate::editor::{Anchor, EditorEvent};
use crate::store::{Answer, Role, Thread, ThreadStatus, TiedBy, hash_text};
use crate::theme::{Theme, ts};
use crate::toast::{self, Kind};
use crate::ui::{self, Keyed};

/// What the toast says once your reply is in.
const WRITTEN: &str = "Your reply is in the note, ready to rework.";
/// The button's tooltip: where the reply goes, why, and that the thread stays tied off.
const WHAT_IT_DOES: &str =
    "Puts your reply under the paragraph this thread is about, for you to rework in your own words. The thread is already tied off.";
/// A reply worth writing in has this many words: the reading loop's floor for a paragraph
/// worth reading ("Yes, exactly." is not).
const MIN_WORDS: usize = 4;
/// The short running stitch that sets the offer apart from the conversation.
const RULE_W: f32 = 46.;

/// The reply written in last, while the toast's Undo can take it out: its note, its
/// thread, and the paragraph as it went in. The first change to that paragraph ends it.
pub(super) struct WrittenIn {
    note: String,
    thread: String,
    words: String,
    _edits: Subscription,
}

/// The reply Write it in would put in the note for `t`: your last reply, when a reply tied
/// a mind's thread off (not your hand, an edit or an answer in the note, and not a thread
/// of your own, where nothing was asked of a mind), the thread's answer is not in the note
/// yet, and the reply has at least [`MIN_WORDS`] words.
pub(super) fn reply_to_write_in(t: &Thread) -> Option<&str> {
    if t.own || t.status != ThreadStatus::Resolved || t.tied_by != Some(TiedBy::Reply) || t.answer.is_some() {
        return None;
    }
    let reply = t.messages.iter().rev().find(|m| m.role == Role::You)?.text.trim();
    (reply.split_whitespace().count() >= MIN_WORDS).then_some(reply)
}

impl NotePage {
    /// Whether thread `id`'s tag offers Write it in now: it is the open tag, threads show,
    /// nothing is on its way to it, its words are in the note, and it has a reply to write
    /// in that the note does not already say (you typed it, or ⌘⇧Z brought it back). A
    /// thread between notes is offered only in its home note: at its far end its answer
    /// belongs to the home note's words.
    fn offers_write_in(&self, id: &str, cx: &App) -> bool {
        let Some(reply) = self.note.threads.iter().find(|t| t.id == id).and_then(reply_to_write_in) else { return false };
        let ed = self.editor.read(cx);
        self.active.as_deref() == Some(id)
            && !self.just_my_words
            && !self.pending.contains_key(id)
            && !self.visiting_here(id)
            && ed.anchor(id).is_some()
            && !ed.text().contains(reply)
    }

    /// The open tag's offer, under the conversation: a short running stitch, "Your note
    /// doesn't say this yet.", and Write it in, "under its paragraph, word for word". In the
    /// tag's own colours; the button's ring is in the thread's.
    pub(super) fn write_in_offer(&mut self, thread: &Thread, color: Hsla, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.offers_write_in(&thread.id, cx) {
            return None;
        }
        let t = Theme::get(cx).clone();
        let p = t.p;
        let seed = crate::stitch::seed_of(&thread.id);
        let rule = canvas(
            |_, _, _| (),
            move |b, _, window, _| {
                let y = b.origin.y + b.size.height / 2.;
                crate::stitch::running_stitch(window, b.origin.x, b.origin.x + px(RULE_W), y, color.opacity(0.42), seed, false);
            },
        )
        .w(px(RULE_W))
        .h(px(4.));
        let id = thread.id.clone();
        let button = ui::ghost_button(SharedString::from(format!("writein-{id}")), "Write it in", &t)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.graphite)
            .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new(WHAT_IT_DOES).max_w(px(250.)).build(window, cx))
            .keyed_as("tag:writein", 4., color, cx, move |this, window, cx| {
                this.write_in(&id, window, cx);
            });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(5.))
                .pt(px(2.))
                .text_size(px(ts::SMALL))
                .line_height(px(16.))
                .child(rule)
                .child(div().text_color(p.faint).child("Your note doesn\u{2019}t say this yet."))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_baseline()
                        .gap_x(px(6.))
                        // wrapped under the button, the words keep clear of its focus ring
                        .gap_y(px(3.))
                        .child(button)
                        .child(div().text_color(p.faint).child("under its paragraph, word for word")),
                )
                .into_any_element(),
        )
    }

    /// Write it in: your reply goes in, word for word, as a paragraph of its own under the
    /// one the thread is about (under the whole list, quote, callout or code it is in). It
    /// becomes the thread's answer and washes in the thread's colour for a moment; the tag
    /// closes and the caret waits at its end; a toast offers Undo. Returns whether it went in.
    pub(super) fn write_in(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.offers_write_in(id, cx) {
            return false;
        }
        let Some(reply) = self.note.threads.iter().find(|t| t.id == id).and_then(reply_to_write_in).map(str::to_string) else {
            return false;
        };
        // its paragraph: the line its words end on
        let line = {
            let ed = self.editor.read(cx);
            ed.anchor(id).map(|a| ed.line_of(if a.range.is_empty() { a.range.start } else { a.range.end - 1 }))
        };
        let Some(line) = line else { return false };
        let Some(new) = self.editor.update(cx, |e, cx| e.insert_paragraph_after(line, &reply, cx)) else { return false };
        let (range, words) = {
            let ed = self.editor.read(cx);
            (ed.line_range(new), ed.line_text(new))
        };
        if let Some(t) = self.thread_mut(id) {
            t.answer = Some(Answer { start: range.start, end: range.end, quote: words.clone() });
        }
        // The words as you replied are not read again: the mind has answered them. Once
        // you rework them they are a new paragraph, read like any other.
        let hash = hash_text(&words);
        if !self.note.read_hashes.contains(&hash) {
            self.note.read_hashes.push(hash);
        }
        let aid = rules::anchor_id(id);
        self.editor.update(cx, |e, cx| {
            e.add_anchor(Anchor { id: aid, range, open: false }, cx);
            e.focus(window, cx);
        });
        self.close_thread(cx);
        self.wash_answer(id, WASH_FOR, cx);
        toast::show(cx, Kind::WrittenIn, WRITTEN);
        let edits = cx.subscribe(&self.editor, |this: &mut Self, _, ev: &EditorEvent, cx| {
            if matches!(ev, EditorEvent::Edited { .. }) {
                this.check_written_in(cx);
            }
        });
        self.written_in = Some(WrittenIn { note: self.note.id.clone(), thread: id.to_string(), words, _edits: edits });
        self.schedule_save(cx);
        cx.notify();
        true
    }

    /// The line of the paragraph written in last, while it reads exactly as it went in.
    fn written_in_line(&self, w: &WrittenIn, cx: &App) -> Option<usize> {
        if w.note != self.note.id {
            return None;
        }
        let ed = self.editor.read(cx);
        let a = ed.anchor(&rules::anchor_id(&w.thread)).filter(|a| !a.range.is_empty())?;
        let line = ed.line_of(a.range.start);
        (ed.line_text(line) == w.words).then_some(line)
    }

    /// After an edit: once the paragraph written in last no longer reads as it went in
    /// (you began to rework it, or ⌘Z took it out), the toast's Undo would take your own
    /// words away, so the toast goes.
    fn check_written_in(&mut self, cx: &mut Context<Self>) {
        let Some(w) = &self.written_in else { return };
        if self.written_in_line(w, cx).is_some() {
            return;
        }
        self.written_in = None;
        if toast::live(cx, Kind::WrittenIn) {
            toast::close(cx, Kind::WrittenIn);
        }
    }

    /// The toast's Undo: take the paragraph written in last out of the note, while its toast
    /// shows and the paragraph reads as it went in, and let the thread's answer go, so the
    /// offer is back the next time its tag is open. Returns whether it took one out.
    pub(crate) fn undo_write_in(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(w) = self.written_in.take() else { return false };
        let live = toast::live(cx, Kind::WrittenIn);
        toast::close(cx, Kind::WrittenIn);
        let Some(line) = self.written_in_line(&w, cx).filter(|_| live) else { return false };
        let aid = rules::anchor_id(&w.thread);
        self.editor.update(cx, |e, cx| {
            e.washes.retain(|(x, _)| *x != aid);
            e.remove_anchor(&aid, cx);
            e.remove_paragraph(line, cx);
        });
        if let Some(t) = self.thread_mut(&w.thread) {
            t.answer = None;
        }
        self.schedule_save(cx);
        cx.notify();
        true
    }

    /// `writein` (press Write it in on the open tag) and `undo-writein` (the toast's Undo)
    /// in a script. Either warns when there is nothing to press.
    pub(super) fn write_in_step(&mut self, step: &crate::script::Step, window: &mut Window, cx: &mut Context<Self>) {
        use crate::script::Step;
        match step {
            Step::WriteIn => match self.active.clone().filter(|id| self.offers_write_in(id, cx)) {
                Some(id) => {
                    self.write_in(&id, window, cx);
                }
                None => log::warn!("script: no open tag offers Write it in"),
            },
            Step::UndoWriteIn => {
                if !self.undo_write_in(cx) {
                    log::warn!("script: no reply written in to take out");
                }
            }
            _ => {}
        }
    }

    /// For the script's dump: `offer`, the open tag that offers Write it in, and
    /// `written_in`, the thread whose reply the toast's Undo can take out.
    pub(super) fn dump_write_in(&self, state: &mut Value, cx: &App) {
        let short = |id: &str| id.chars().take(8).collect::<String>();
        state["offer"] = json!(self.active.as_deref().filter(|id| self.offers_write_in(id, cx)).map(short));
        let undoable = self.written_in.as_ref().filter(|w| toast::live(cx, Kind::WrittenIn) && self.written_in_line(w, cx).is_some());
        state["written_in"] = json!(undoable.map(|w| short(&w.thread)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Message;

    fn tied_by_reply(reply: &str) -> Thread {
        Thread {
            id: "t2".into(),
            quote: "add a kilometre every week until I reach 16".into(),
            status: ThreadStatus::Resolved,
            tied_by: Some(TiedBy::Reply),
            messages: vec![
                Message::new(Role::Mind, "Starting from how far?"),
                Message::new(Role::You, reply),
                Message::new(Role::Mind, "That settles it."),
            ],
            ..Default::default()
        }
    }

    #[std::prelude::v1::test]
    fn your_last_reply_is_offered_when_it_tied_the_thread_off() {
        let t = tied_by_reply("  I start at 8 km in the first week of December.  ");
        assert_eq!(reply_to_write_in(&t), Some("I start at 8 km in the first week of December."));
        // the last of your replies, not the first
        let mut two = t.clone();
        two.messages.insert(1, Message::new(Role::You, "Eight, I think, but not sure."));
        assert_eq!(reply_to_write_in(&two), Some("I start at 8 km in the first week of December."));
        // four words is worth writing in; three is not
        assert!(reply_to_write_in(&tied_by_reply("Start at 8 km.")).is_some());
        assert!(reply_to_write_in(&tied_by_reply("Yes, exactly that.")).is_none());
    }

    #[std::prelude::v1::test]
    fn nothing_is_offered_unless_a_reply_tied_it_and_it_is_not_in_the_note() {
        let t = tied_by_reply("I start at 8 km in the first week of December.");
        for by in [None, Some(TiedBy::Hand), Some(TiedBy::Edit), Some(TiedBy::Answer)] {
            assert!(reply_to_write_in(&Thread { tied_by: by, ..t.clone() }).is_none(), "{by:?}");
        }
        for status in [ThreadStatus::Open, ThreadStatus::Dismissed] {
            assert!(reply_to_write_in(&Thread { status, ..t.clone() }).is_none(), "{status:?}");
        }
        let written = Thread { answer: Some(Answer { start: 1, end: 9, quote: "I start".into() }), ..t.clone() };
        assert!(reply_to_write_in(&written).is_none(), "already in the note");
        let unanswered = Thread { messages: vec![Message::new(Role::Mind, "Starting from how far?")], ..t.clone() };
        assert!(reply_to_write_in(&unanswered).is_none(), "no reply of yours");
        assert!(reply_to_write_in(&Thread { own: true, ..t }).is_none(), "a thread of your own asked nothing of a mind");
    }
}

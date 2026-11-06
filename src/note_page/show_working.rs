//! Show its working, on the page: the check under a correction in its tag (a sum set like
//! a sum, or your own words), Disagree, and showing the words a check rests on. The parts
//! without GPUI are in `src/working.rs`; the brain makes sure of every check
//! (`brain/src/working.ts`).

use super::*;
use crate::store::Check;
use crate::working::{self, Place};

/// Your reply box's words, before and after Disagree.
const REPLY_HINT: &str = "Write back\u{2026}";
const DISPUTE_HINT: &str = "What doesn\u{2019}t hold?";

/// About how wide `s` is at the interface size with tabular figures, with room to spare,
/// so every line of a sum gets the same columns and the = signs line up. A cell is never
/// narrower than its words (it does not wrap), so a low guess only costs the alignment.
fn width_of(s: &str) -> f32 {
    s.chars()
        .map(|c| match c {
            '0'..='9' => 7.4,
            ':' | '.' | ',' | ' ' => 3.6,
            '(' | ')' => 4.6,
            '%' => 10.6,
            _ => 8.0,
        })
        .sum::<f32>()
        * 1.12
        + 4.
}

fn tabular() -> FontFeatures {
    FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)]))
}

impl NotePage {
    /// A correction's check, from the brain's answer, on the thread's latest message from
    /// the mind (the correction itself).
    pub(super) fn attach_check(&mut self, id: &str, wire: &Value, cx: &mut Context<Self>) {
        let Some(check) = working::from_wire(wire) else { return };
        if let Some(m) = self.thread_mut(id).and_then(|t| t.messages.iter_mut().rev().find(|m| m.role == Role::Mind)) {
            m.check = Some(check);
        }
        self.schedule_save(cx);
    }

    /// "· taken back" in the tag's top row, in place of "· tied off".
    pub(super) fn taken_back(thread: &Thread) -> bool {
        thread.messages.iter().any(|m| m.check.as_ref().is_some_and(|c| c.taken_back))
    }

    /// The check under the message that carries it: a seam stitched down its left, the
    /// working set like a sum (or your words), where its numbers came from, and Disagree
    /// while the tag is open.
    pub(super) fn working_block(&mut self, thread: &Thread, m: &Message, color: Hsla, is_active: bool, width: f32, cx: &mut Context<Self>) -> Option<AnyElement> {
        let c: Check = m.check.clone()?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        let at = thread.messages.iter().position(|x| std::ptr::eq(x, m)).unwrap_or(0);
        let key = format!("{}-{at}", thread.id);
        let back = c.taken_back;
        let ink = if back { p.faint } else { p.graphite };
        let text = self.editor.read(cx).text().to_string();
        let brief = self.note.brief.clone();

        let label = if c.sum.is_empty() { "In your note" } else { "Working" };
        let mut head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(div().text_size(px(ts::META)).text_color(p.faint).child(label));
        if is_active && !back && thread.status == ThreadStatus::Open {
            let disputing = self.disputes.contains(&thread.id);
            let id = thread.id.clone();
            head = head.child(
                ui::ghost_button(SharedString::from(format!("disagree-{key}")), "Disagree", &t)
                    .when(disputing, |d| d.text_color(p.ink))
                    .tooltip(|window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new("Say what\u{2019}s wrong with the working. If Inkling agrees, it takes the correction back.").build(window, cx)
                    })
                    .keyed_as("tag:disagree", 4., color, cx, move |this, window, cx| this.disagree(&id, window, cx)),
            );
        }
        let mut block = div()
            .mt(px(-2.))
            .pl(px(10.))
            .border_l(px(1.))
            .border_dashed()
            .border_color(color.opacity(0.45))
            .flex()
            .flex_col()
            .gap(px(2.))
            .font_family(SANS)
            .text_size(px(ts::UI))
            .line_height(px(18.))
            .child(head);

        if !c.sum.is_empty() {
            // the columns of a sum: the expressions right-aligned, the sign, the results
            let ew = c.sum.iter().map(|l| width_of(&l.sum)).fold(0., f32::max);
            let rw = c.sum.iter().map(|l| width_of(&l.is)).fold(0., f32::max);
            let fits = ew + rw + 30. <= width - 11.;
            for line in &c.sum {
                block = block.child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .font_features(tabular())
                        .text_color(ink)
                        .when(back, |d| d.line_through())
                        .child(
                            div()
                                .when(fits, |d| d.min_w(px(ew)).flex_none().whitespace_nowrap().text_right())
                                .when(!fits, |d| d.flex_1().min_w(px(0.)))
                                .child(line.sum.clone()),
                        )
                        .child(div().flex_none().text_color(p.faint).child(working::sign(line)))
                        .child(div().flex_none().whitespace_nowrap().when(fits, |d| d.min_w(px(rw)).text_right()).child(line.is.clone())),
                );
            }
            if !c.givens.is_empty() {
                let mut from = div()
                    .flex()
                    .flex_wrap()
                    .gap_x(px(4.))
                    .text_size(px(ts::SMALL))
                    .line_height(px(17.))
                    .child(div().text_color(p.faint).child("From your words:"));
                for (j, g) in c.givens.iter().enumerate() {
                    if j > 0 {
                        from = from.child(div().text_color(p.faint).child("\u{b7}"));
                    }
                    from = from.child(self.word_link(&key, j, &g.words, None, &text, brief.as_deref(), ink, cx));
                }
                block = block.child(from);
            }
        }
        for (j, w) in c.words.iter().enumerate() {
            let link = self.word_link(&key, c.givens.len() + j, &w.words, w.note.clone(), &text, brief.as_deref(), ink, cx);
            block = block.child(div().when(back, |d| d.line_through()).child(link));
        }
        Some(block.into_any_element())
    }

    /// Words a check rests on, in quotes: a link to them in the note, or marked as changed
    /// once they are no longer there.
    #[allow(clippy::too_many_arguments)]
    fn word_link(&mut self, key: &str, j: usize, words: &str, note: Option<String>, text: &str, brief: Option<&str>, ink: Hsla, cx: &mut Context<Self>) -> AnyElement {
        let p = Theme::get(cx).p;
        let quoted = format!("\u{201c}{words}\u{201d}");
        let place = if note.is_some() { Place::Elsewhere } else { working::place(text, brief, words) };
        let id = SharedString::from(format!("words-{key}-{j}"));
        match place {
            Place::Note(_) => {
                let words = words.to_string();
                div()
                    .id(id)
                    .cursor_pointer()
                    .text_color(ink)
                    .hover(move |s| s.text_color(p.ink))
                    .child(quoted)
                    .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Show these words in the note").build(window, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.show_words(&words, window, cx);
                    }))
                    .into_any_element()
            }
            Place::Changed => div()
                .id(id)
                .text_color(p.faint)
                .child(format!("{quoted} (changed)"))
                .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("These words are no longer in the note").build(window, cx))
                .into_any_element(),
            Place::Brief | Place::Elsewhere => div().text_color(ink).child(quoted).into_any_element(),
        }
    }

    /// Scroll to words a check rests on and select them, as Find would.
    pub(super) fn show_words(&mut self, words: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Place::Note(range) = working::place(self.editor.read(cx).text(), None, words) else { return false };
        self.editor.update(cx, |e, cx| e.select_range(range, cx));
        self.keep_cursor_visible(window, cx);
        cx.notify();
        true
    }

    /// Disagree: your next reply on this thread says the working does not hold. Pressed
    /// again, it goes back to an ordinary reply.
    pub(super) fn disagree(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.active.as_deref() != Some(id) {
            self.activate(Some(id.to_string()), window, cx);
        }
        let on = !self.disputes.remove(id);
        if on {
            self.disputes.insert(id.to_string());
        }
        if let Some(input) = self.replies.get(id).cloned() {
            input.update(cx, |i, cx| {
                i.set_placeholder(if on { DISPUTE_HINT } else { REPLY_HINT }, window, cx);
                i.focus(window, cx);
            });
        }
        cx.notify();
    }

    /// The message a reply after Disagree sends the mind instead of your words alone, or
    /// `None` for an ordinary reply.
    pub(super) fn dispute_message(&mut self, id: &str, reply: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
        if !self.disputes.remove(id) {
            return None;
        }
        if let Some(input) = self.replies.get(id).cloned() {
            input.update(cx, |i, cx| i.set_placeholder(REPLY_HINT, window, cx));
        }
        let checked = self.note.threads.iter().find(|t| t.id == id).is_some_and(|t| t.messages.iter().any(|m| m.check.as_ref().is_some_and(|c| !c.taken_back)));
        checked.then(|| {
            format!(
                "I don\u{2019}t think your working holds: \u{201c}{}\u{201d}. Check each line of it, and the words it rests on, against my note. If I\u{2019}m right, say in one sentence what you got wrong and mark this resolved. If I\u{2019}m not, show me where, in one or two sentences.",
                reply.trim()
            )
        })
    }

    /// The mind answered a dispute: if it agreed, the correction is taken back.
    pub(super) fn after_dispute(&mut self, id: &str, agreed: bool, cx: &mut Context<Self>) {
        if !agreed {
            return;
        }
        if let Some(t) = self.thread_mut(id) {
            for c in t.messages.iter_mut().filter_map(|m| m.check.as_mut()) {
                c.taken_back = true;
            }
        }
        self.schedule_save(cx);
        cx.notify();
    }

    /// The thread's check in words, for Copy.
    pub(super) fn check_lines(m: &Message) -> Vec<String> {
        m.check.as_ref().map(working::text_lines).unwrap_or_default()
    }

    /// `see: n m`: the m-th words thread n's check rests on, as a click on them.
    pub(super) fn see_step(&mut self, n: usize, m: usize, window: &mut Window, cx: &mut Context<Self>) {
        let words = self
            .nth_thread(n)
            .and_then(|id| self.note.threads.iter().find(|t| t.id == id))
            .and_then(|t| t.messages.iter().find_map(|m| m.check.as_ref()))
            .and_then(|c| working::words_of(c).into_iter().nth(m.saturating_sub(1)));
        match words {
            Some((w, _)) if self.show_words(&w, window, cx) => {}
            _ => log::warn!("script: see: thread {n} has no words {m} in the note"),
        }
    }

    /// `disagree: n`: press Disagree on thread n's working.
    pub(super) fn disagree_step(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
        let has_check = |id: &String| self.note.threads.iter().any(|t| &t.id == id && t.messages.iter().any(|m| m.check.is_some()));
        match self.nth_thread(n).filter(has_check) {
            Some(id) => self.disagree(&id, window, cx),
            None => log::warn!("script: disagree: thread {n} has no working"),
        }
    }
}

//! Read it through: one command reads every paragraph of the note that has not been read,
//! top to bottom, once, by the reading loop's own rule (`judge`), and then the note goes
//! back to what it was doing. The needle works down the gutter and threads arrive as each
//! paragraph is read. A pass lives only on the open page: nothing about it is saved, and
//! what it read is recorded where reading along records it (`read_hashes`, threads).

use super::*;

/// A pass of Read it through on the open note.
pub(crate) struct Pass {
    /// The first line it has not looked at yet: it reads down from here.
    next: usize,
    /// Paragraphs read so far, and how many it expects in all, counted again at every step
    /// because you may be writing while it reads.
    done: usize,
    total: usize,
    /// The paragraph being read, by the hash of its words.
    reading: Option<u64>,
    /// The note's threads when it began: an open question or correction not among them at
    /// the end is new.
    had: HashSet<String>,
}

/// What the reading loop reads as one: a paragraph, or a whole code block.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Unit {
    /// Its first line, and the line after its last.
    pub line: usize,
    pub end: usize,
    /// Its words, trimmed.
    pub text: String,
    /// A code block whose every line was left alone (code is left alone line by line).
    pub code_alone: bool,
    /// The caret is in it and you typed a moment ago: you may be mid-sentence.
    pub writing: bool,
    /// It calls the note's mind by name.
    pub calls: bool,
}

/// What a pass does with a unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next {
    Read,
    Answer,
    Skip,
}

/// The reading loop's rule for a pass: read it as reading along would have, with nothing
/// else in flight. Words left alone are the app's (the guide's) and are never read, except
/// in a note brought in from Obsidian, where they are the writer's own and the writer asked.
pub(crate) fn next_for(u: &Unit, read: &[u64], left_alone: &[u64], imported: bool) -> Next {
    let hash = hash_text(&u.text);
    let alone = !imported && (u.code_alone || left_alone.contains(&hash));
    match judge(&u.text, u.writing, alone || read.contains(&hash), u.calls, true, false) {
        Verdict::Read => Next::Read,
        Verdict::Answer => Next::Answer,
        // a paragraph still being written is left for reading along or the next pass
        Verdict::Skip | Verdict::Later => Next::Skip,
    }
}

/// How many of `units` a pass would read (or answer).
pub(crate) fn count(units: &[Unit], read: &[u64], left_alone: &[u64], imported: bool) -> usize {
    units.iter().filter(|u| next_for(u, read, left_alone, imported) != Next::Skip).count()
}

// ----- the words on screen --

pub(crate) const NOTHING_NEW: &str = "Nothing new to read.";

/// The mode menu's line under Read it through.
pub(crate) fn menu_line(n: usize) -> String {
    match n {
        0 => NOTHING_NEW.to_string(),
        1 => "Reads the one paragraph it hasn\u{2019}t read.".to_string(),
        n => format!("Reads the {n} paragraphs it hasn\u{2019}t read, one at a time."),
    }
}

/// Which paragraph of how many it is on (the one being read counts).
fn at(done: usize, total: usize) -> (usize, usize) {
    let total = total.max(done + 1);
    (done + 1, total)
}

/// The pill in the window's top band, while a pass runs.
pub(crate) fn status_words(done: usize, total: usize) -> String {
    let (n, of) = at(done, total);
    format!("Reading it through, {n} of {of}")
}

/// The mode menu's line under Stop reading it through.
pub(crate) fn menu_progress(done: usize, total: usize) -> String {
    let (n, of) = at(done, total);
    format!("Reading {n} of {of}.")
}

/// The toast when the needle has passed the last paragraph. No count of what it found:
/// belief 7, and the header already says how many threads are open.
pub(crate) fn end_words(mind: &str, new_threads: bool, hidden: bool) -> String {
    match (new_threads, hidden) {
        (false, _) => format!("{mind} read it through. Nothing to ask."),
        (true, false) => format!("{mind} read it through. New threads are in the margin."),
        (true, true) => format!("{mind} read it through. Show threads to see what it asked."),
    }
}

/// The toast when you stop a pass.
pub(crate) fn stop_words(done: usize, total: usize) -> String {
    format!("Stopped after {done} of {}. Read it through again to go on.", total.max(done))
}

/// The toast when you open another note while a pass runs.
pub(crate) fn left_words(title: &str) -> String {
    let title = short_title(title, 40);
    if title.is_empty() {
        "Read it through stopped when you left the note.".to_string()
    } else {
        format!("Read it through stopped when you left \u{201c}{title}\u{201d}.")
    }
}

/// The problem toast when a read of the pass fails: where it stopped, then the usual
/// sentence for the failure (`describe_failure`).
pub(crate) fn failure_words(done: usize, total: usize, why: &str) -> String {
    if done == 0 {
        format!("Couldn\u{2019}t read it through. {why}")
    } else {
        format!("Read it through stopped after {done} of {}. {why}", total.max(done))
    }
}

impl NotePage {
    /// ⇧⌘R, the mode menu's last item, and Stop: start a pass, or stop the one running.
    pub fn toggle_read_through(&mut self, cx: &mut Context<Self>) {
        if self.read_through.is_some() {
            return self.stop_read_through(cx);
        }
        let total = self.to_read(cx);
        if total == 0 {
            crate::toast::show(cx, crate::toast::Kind::Done, NOTHING_NEW);
            return;
        }
        let had = self.note.threads.iter().map(|t| t.id.clone()).collect();
        self.read_through = Some(Pass { next: 1, done: 0, total, reading: None, had });
        self.step_through(cx);
        cx.notify();
    }

    /// The units of the note from line `from` down, as the reading loop sees them.
    fn units_from(&self, from: usize, cx: &App) -> Vec<Unit> {
        let ed = self.editor.read(cx);
        let count = ed.line_count();
        let caret = ed.unit_of(ed.line_of(ed.cursor())).start;
        let typing = self.typing_recently();
        let mut out = vec![];
        let mut line = from.max(1);
        while line < count {
            let u = ed.unit_of(line);
            let end = u.end.max(line + 1);
            let text = ed.lines_text(u.clone()).trim().to_string();
            let code_alone = ed.line_kind(u.start).is_code()
                && u.clone().map(|l| ed.line_text(l)).filter(|t| !t.trim().is_empty()).all(|t| self.note.left_alone.contains(&hash_text(&t)));
            let calls = self.calls_mind(&text);
            out.push(Unit { line: u.start, end, text, code_alone, writing: typing && u.start == caret, calls });
            line = end;
        }
        out
    }

    /// How many paragraphs a pass started now would read.
    pub(super) fn to_read(&self, cx: &App) -> usize {
        count(&self.units_from(1, cx), &self.note.read_hashes, &self.note.left_alone, self.note.imported)
    }

    /// Read the next paragraph that wants reading, or end the pass. Runs when a pass starts
    /// and whenever a read ends (`after_read`); it waits while reading along has the page's
    /// one read in flight, so your fresh paragraphs go first.
    pub(super) fn step_through(&mut self, cx: &mut Context<Self>) {
        let in_flight = self.reading.as_ref().map(|r| r.hash);
        let Some(pass) = self.read_through.as_mut() else { return };
        // The pass's read ended well (a failure ends the pass in `pass_failed`).
        if pass.reading.is_some() && pass.reading != in_flight {
            pass.reading = None;
            pass.done += 1;
        }
        if in_flight.is_some() {
            cx.notify();
            return;
        }
        let from = pass.next;
        let units = self.units_from(from, cx);
        let (read, alone, imported) = (&self.note.read_hashes, &self.note.left_alone, self.note.imported);
        let mut calls = vec![];
        let mut pick = None;
        for (i, u) in units.iter().enumerate() {
            match next_for(u, read, alone, imported) {
                Next::Skip => {}
                Next::Answer => calls.push(u.clone()),
                Next::Read => {
                    pick = Some((i, count(&units[i + 1..], read, alone, imported)));
                    break;
                }
            }
        }
        // A paragraph that calls the mind by name is answered, as reading along does, and
        // the pass goes on while the answer streams in.
        for u in calls {
            self.note.read_hashes.push(hash_text(&u.text));
            self.answer_call(u.line, u.text, cx);
        }
        let Some((i, rest)) = pick else { return self.finish_read_through(cx) };
        let u = units[i].clone();
        let hash = hash_text(&u.text);
        if let Some(pass) = self.read_through.as_mut() {
            pass.next = u.end;
            pass.reading = Some(hash);
            pass.total = pass.done + 1 + rest;
        }
        self.note.read_hashes.push(hash);
        self.schedule_save(cx);
        self.read_line(u.line, u.text, hash, cx);
        cx.notify();
    }

    /// Is the read of the paragraph hashed `hash` part of a pass? Then it is a read of a
    /// finished draft (`"through": true`), and a failure ends the pass.
    pub(super) fn reads_through(&self, hash: u64) -> bool {
        self.read_through.as_ref().is_some_and(|p| p.reading == Some(hash))
    }

    /// The needle has passed the last paragraph.
    fn finish_read_through(&mut self, cx: &mut Context<Self>) {
        let Some(pass) = self.read_through.take() else { return };
        let new = self
            .note
            .threads
            .iter()
            .any(|t| !pass.had.contains(&t.id) && t.status == ThreadStatus::Open && matches!(t.kind, ThoughtKind::Question | ThoughtKind::Correction));
        crate::toast::show(cx, crate::toast::Kind::Done, end_words(&self.mind.name, new, self.just_my_words));
        cx.notify();
    }

    /// Stop, ⇧⌘R again: the read in flight is cancelled and its paragraph stays unread;
    /// what was read keeps its stitch and threads, and the next pass goes on from there.
    pub(super) fn stop_read_through(&mut self, cx: &mut Context<Self>) {
        let Some(pass) = self.read_through.take() else { return };
        // A cancelled request never calls back, so the slot is freed here (as `open` does).
        if let Some(r) = self.reading.take_if(|r| Some(r.hash) == pass.reading) {
            self.note.read_hashes.retain(|h| *h != r.hash);
            self.cancel_request(&r.req, cx);
        }
        crate::toast::show(cx, crate::toast::Kind::Done, stop_words(pass.done, pass.total));
        self.schedule_save(cx);
        // paragraphs reading along finished meanwhile were waiting for the slot
        self.later(cx, |page, cx| page.after_read(cx));
        cx.notify();
    }

    /// A read of the pass failed: the pass ends, saying how far it got and why. Carrying on
    /// would only fail every paragraph in turn (a limit, a stopped brain, sign-in).
    pub(super) fn pass_failed(&mut self, kind: &str, message: &str, cx: &mut Context<Self>) {
        let Some(pass) = self.read_through.take() else { return };
        cx.notify();
        if kind == "cancelled" {
            return;
        }
        let (kind, message) = (kind.to_string(), message.to_string());
        // after this update, as `report` does: describing the failure reads the brain
        self.later(cx, move |page, cx| {
            let why = page.describe_failure(&kind, &message, cx);
            page.notice(failure_words(pass.done, pass.total, &why), cx);
        });
    }

    /// The page is opening another note: Inkling speaks only on the page you are on, so a
    /// pass does not go on in a note you cannot see.
    pub(super) fn leave_read_through(&mut self, cx: &mut Context<Self>) {
        if self.read_through.take().is_some() {
            crate::toast::show(cx, crate::toast::Kind::Done, left_words(&self.note.title));
        }
    }

    /// Lines moved above the needle (an edit added or merged lines): its place moves too.
    pub(super) fn shift_read_through(&mut self, moved: &dyn Fn(usize) -> usize) {
        if let Some(p) = self.read_through.as_mut() {
            p.next = moved(p.next);
        }
    }

    /// The pass as a script's `dump:` shows it.
    pub(super) fn read_through_state(&self) -> Value {
        self.read_through.as_ref().map_or(Value::Null, |p| json!({"done": p.done, "total": p.total, "next": p.next}))
    }

    /// While a pass runs: how far it has got, and Stop, on a pill in the window's top band,
    /// centred over the page (`app::chrome_layer` places it). The header row has no room
    /// for it without wrapping, which would move the page when a pass starts and ends; and
    /// the band stays in view however far the page is scrolled, which a long note needs.
    pub(crate) fn read_through_status(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (done, total) = self.read_through.as_ref().map(|p| (p.done, p.total))?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .pl(px(11.))
                .pr(px(3.))
                .py(px(2.))
                .rounded_full()
                .border_1()
                .border_color(p.rule)
                .bg(p.paper.opacity(0.6))
                .text_size(px(ts::SMALL))
                .text_color(p.graphite)
                .child(status_words(done, total))
                .child(
                    div()
                        .id("read-through-stop")
                        .px(px(8.))
                        .py(px(1.))
                        .rounded_full()
                        .text_color(p.ink)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .active(|s| s.opacity(0.7))
                        .child("Stop")
                        .keyed_as("chrome:stop-reading", 10., t.voice_ink, cx, |this, _, cx| this.stop_read_through(cx)),
                )
                .into_any_element(),
        )
    }

    /// The mode menu's last item, under a rule: Read it through, or Stop reading it through
    /// while a pass runs. An action, not a mode, so it never carries the check.
    pub(super) fn read_through_item(&self, pick: usize, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let (glyph, title, line) = match self.read_through.as_ref() {
            Some(pass) => ("circle-stop", "Stop reading it through", menu_progress(pass.done, pass.total)),
            None => ("arrow-down-to-line", "Read it through", menu_line(self.to_read(cx))),
        };
        let keys = crate::shortcuts::keys_for("app::ReadThrough", cx).unwrap_or_default();
        div()
            .child(div().mt(px(4.)).mb(px(4.)).border_t_1().border_color(p.rule))
            .child(
                ui::menu_item("mode-read-through", &t)
                    .relative()
                    .when(pick == 2, |d| d.bg(p.hover).child(ui::focus_ring(&self.menu_focus, 6., t.voice_ink)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(icon(glyph, 12., p.graphite))
                            .child(div().flex_1().child(title))
                            .child(ui::key_hint(keys, &t)),
                    )
                    .child(div().pl(px(19.)).text_size(px(ts::META)).line_height(px(15.)).text_color(p.graphite).child(line))
                    .on_click(cx.listener(|this, _, window, cx| this.choose(Menu::Mode, 2, window, cx))),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(line: usize, text: &str) -> Unit {
        Unit { line, end: line + 1, text: text.to_string(), code_alone: false, writing: false, calls: false }
    }

    #[std::prelude::v1::test]
    fn a_pass_reads_what_reading_along_would_have() {
        let read = vec![hash_text("This one was read along an hour ago.")];
        let units = vec![
            unit(1, "This one was read along an hour ago."),
            unit(2, "This finished paragraph has not been read."),
            unit(3, ""),
            unit(4, "Too short."),
            unit(5, "No full stop at the end but the caret has left it"),
            Unit { calls: true, ..unit(6, "Inkling, is the reset safe under load?") },
            Unit { end: 10, ..unit(7, "fn main() {\n    println!(\"hi\");\n}") },
        ];
        let got: Vec<Next> = units.iter().map(|u| next_for(u, &read, &[], false)).collect();
        assert_eq!(got, vec![Next::Skip, Next::Read, Next::Skip, Next::Skip, Next::Read, Next::Answer, Next::Read]);
        assert_eq!(count(&units, &read, &[], false), 4);
    }

    #[std::prelude::v1::test]
    fn the_guides_words_are_left_alone_and_imported_ones_are_read() {
        let words = "Inkling reads each paragraph when you pause, and marks it.";
        let alone = vec![hash_text(words)];
        let u = unit(3, words);
        assert_eq!(next_for(&u, &[], &alone, false), Next::Skip, "the app's own words are never read");
        assert_eq!(next_for(&u, &[], &alone, true), Next::Read, "words brought in are the writer's, read when asked");
        let code = Unit { code_alone: true, ..unit(4, "let x = 1;\nlet y = x + 1;") };
        assert_eq!(next_for(&code, &[], &[], false), Next::Skip);
        assert_eq!(next_for(&code, &[], &[], true), Next::Read);
        // read is read, imported or not
        assert_eq!(next_for(&u, &alone, &alone, true), Next::Skip);
    }

    #[std::prelude::v1::test]
    fn a_paragraph_mid_sentence_waits_only_while_you_type() {
        let half = unit(5, "We will reset the counter every");
        assert_eq!(next_for(&Unit { writing: true, ..half.clone() }, &[], &[], false), Next::Skip);
        assert_eq!(next_for(&half, &[], &[], false), Next::Read, "the writer asked: a paragraph without a full stop is finished");
        let done = unit(5, "We will reset the counter every minute.");
        assert_eq!(next_for(&Unit { writing: true, ..done }, &[], &[], false), Next::Read, "a finished sentence is read even under the caret");
        let short = unit(6, "Two words");
        assert_eq!(next_for(&Unit { writing: true, ..short.clone() }, &[], &[], false), Next::Skip);
        assert_eq!(next_for(&short, &[], &[], false), Next::Skip);
    }

    #[std::prelude::v1::test]
    fn the_words_on_screen() {
        assert_eq!(menu_line(0), "Nothing new to read.");
        assert_eq!(menu_line(1), "Reads the one paragraph it hasn\u{2019}t read.");
        assert_eq!(menu_line(14), "Reads the 14 paragraphs it hasn\u{2019}t read, one at a time.");
        assert_eq!(status_words(5, 14), "Reading it through, 6 of 14");
        assert_eq!(status_words(0, 0), "Reading it through, 1 of 1", "never past its own count");
        assert_eq!(menu_progress(5, 14), "Reading 6 of 14.");
        assert_eq!(end_words("Inkling", false, false), "Inkling read it through. Nothing to ask.");
        assert_eq!(end_words("Skeptic", true, false), "Skeptic read it through. New threads are in the margin.");
        assert_eq!(end_words("Skeptic", true, true), "Skeptic read it through. Show threads to see what it asked.");
        assert_eq!(stop_words(6, 14), "Stopped after 6 of 14. Read it through again to go on.");
        assert_eq!(left_words("Draft"), "Read it through stopped when you left \u{201c}Draft\u{201d}.");
        assert_eq!(left_words("  "), "Read it through stopped when you left the note.");
        assert!(left_words(&"A very long title that keeps going well past forty characters").ends_with("\u{2026}\u{201d}."));
        assert_eq!(
            failure_words(6, 14, "You\u{2019}ve reached your Claude usage limit for now. Try again later."),
            "Read it through stopped after 6 of 14. You\u{2019}ve reached your Claude usage limit for now. Try again later."
        );
        assert_eq!(failure_words(0, 14, "Claude is overloaded right now. Try again in a moment."), "Couldn\u{2019}t read it through. Claude is overloaded right now. Try again in a moment.");
    }
}

//! Threads of your own: a question you hang on your own words (Ask yourself, ⇧⌘↩). It is
//! sewn in your ink, the colour of your words, so a coloured thread is a mind and an ink
//! one is you; its words get seed stitches rather than a running stitch, so it differs in
//! shape too. No mind answers in it unless you call one: the tag's Ask button, or a line
//! that starts with a mind's name. Only you tie it off or cut it: editing its words starts
//! no reread. The design: `design/features/threads-of-your-own.md`.
//!
//! A child of `note_page`, so it reaches the page's state without widening it; the page
//! itself only hands over to the functions here.

use super::*;

/// The thread your own questions are sewn in: the ink your words are written in, in
/// either light. No mind ever asks in it.
pub(crate) fn ink(t: &Theme) -> Hsla {
    t.p.ink
}

/// A thread message as the brain reads it: its words with the working a correction
/// carries, and who wrote it, when it was a mind you called into your own thread.
pub(crate) fn message_json(m: &Message) -> Value {
    let mut v = json!({"role": role_wire(m.role), "text": crate::working::wire_text(m)});
    if !m.mind.trim().is_empty() {
        v["mind"] = json!(m.mind);
    }
    v
}

/// The mind a line calls by name: "Skeptic, …", "Skeptic: …", "hey Skeptic …" or
/// "@Skeptic …", the rule for calling a mind from the note. The longer of two names that
/// both fit wins.
pub(super) fn called_mind(text: &str, minds: &[Mind]) -> Option<Mind> {
    let mut by_length: Vec<&Mind> = minds.iter().filter(|m| !m.name.trim().is_empty()).collect();
    by_length.sort_by_key(|m| std::cmp::Reverse(m.name.trim().chars().count()));
    by_length.into_iter().find(|m| calls_by_name(text.trim(), &[m.name.trim().to_lowercase()])).cloned()
}

/// What a call into your own thread sends: its first line, your question, and everything
/// said since, in order. The last of those is the line that called, when one did.
pub(super) fn split_call(messages: &[Message]) -> Option<(String, &[Message])> {
    let (first, rest) = messages.split_first()?;
    (first.role == Role::You && !first.text.trim().is_empty()).then(|| (first.text.clone(), rest))
}

/// What the tag says under a folded thread of yours: how much more is in it.
fn more_lines(n: usize) -> String {
    if n == 1 { "1 more line".into() } else { format!("{n} more lines") }
}

impl NotePage {
    /// Ask yourself, from the selection menu or ⇧⌘↩: the question box, no mind seated.
    /// Threads come back first if they were hidden, since you asked for one.
    pub fn ask_yourself(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_range(cx).is_none() {
            return;
        }
        if self.just_my_words {
            self.set_hidden(false, cx);
        }
        self.start_ask(window, cx);
        if let Some(a) = self.asking.as_mut() {
            a.room.clear();
            let input = a.input.clone();
            input.update(cx, |i, cx| i.set_placeholder("Ask yourself about this\u{2026}", window, cx));
        }
        cx.notify();
    }

    /// Return in the question box with nobody seated: your question hangs from the words.
    /// Nothing goes to the brain, the tag stays folded, and the keyboard stays in the note.
    pub(super) fn hang_own(&mut self, range: Range<usize>, question: String, window: &mut Window, cx: &mut Context<Self>) -> String {
        let id = self.open_thread(range.clone(), ThoughtKind::Question, None, cx);
        if let Some(t) = self.thread_mut(&id) {
            t.own = true;
            t.mind_name.clear();
            t.messages.push(Message::new(Role::You, question));
        }
        self.editor.update(cx, |e, cx| {
            e.move_cursor_to(range.end, cx);
            e.focus(window, cx);
        });
        self.schedule_save(cx);
        cx.notify();
        id
    }

    pub(super) fn is_own(&self, id: &str) -> bool {
        self.note.threads.iter().any(|t| t.id == id && t.own)
    }

    /// Every mind a line can call by name: Inkling, the note's mind, yours, and the
    /// built-in ones with a way of thinking.
    fn callable_minds(&self, cx: &App) -> Vec<Mind> {
        let mut all = vec![Mind::inkling(), self.mind.clone()];
        all.extend(cx.global::<Store>().minds.iter().cloned());
        all.extend(crate::store::mind_templates().into_iter().filter(|t| !t.how_it_thinks.trim().is_empty()));
        let mut seen = HashSet::new();
        all.retain(|m| seen.insert(m.name.trim().to_lowercase()));
        all
    }

    /// Return in your own thread's box: the line goes in for you; a mind answers only when
    /// the line starts with its name.
    pub(super) fn add_to_own(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.replies.get(id).cloned() else { return };
        let text = input.read(cx).value().trim().to_string();
        if text.is_empty() || self.pending.contains_key(id) {
            return;
        }
        input.update(cx, |i, cx| i.set_value("", window, cx));
        self.push_message(id, Role::You, text.clone(), cx);
        if let Some(mind) = called_mind(&text, &self.callable_minds(cx)) {
            self.maybe_learn(&text, cx);
            self.call_into_own(id, mind, cx);
        }
    }

    /// Ask <mind> on your own tag: what is in the box goes in first, then the mind answers.
    pub(super) fn ask_into_own(&mut self, id: &str, mind: Mind, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending.contains_key(id) {
            return;
        }
        if let Some(input) = self.replies.get(id).cloned() {
            let text = input.read(cx).value().trim().to_string();
            if !text.is_empty() {
                input.update(cx, |i, cx| i.set_value("", window, cx));
                self.push_message(id, Role::You, text.clone(), cx);
                self.maybe_learn(&text, cx);
            }
        }
        self.call_into_own(id, mind, cx);
    }

    /// The mind answers your thread once, as itself; its answer streams in under its name.
    /// It never ties the thread off: that is yours to decide.
    pub(super) fn call_into_own(&mut self, id: &str, mind: Mind, cx: &mut Context<Self>) {
        if self.pending.contains_key(id) {
            return;
        }
        let Some(t) = self.note.threads.iter().find(|t| t.id == id && t.own).cloned() else { return };
        let Some((question, since)) = split_call(&t.messages) else { return };
        // the words as they are now; a thread whose words were deleted keeps its last ones
        let now = {
            let ed = self.editor.read(cx);
            ed.anchor(id).and_then(|a| ed.text().get(a.range.clone()).map(str::to_string)).filter(|w| !w.trim().is_empty())
        };
        let target = now.unwrap_or_else(|| t.quote.clone());
        if target.trim().is_empty() {
            return;
        }
        let input = json!({
            "task": "ask",
            "ctx": self.context_with(&mind, cx),
            "target": target,
            "question": question,
            "history": since.iter().map(message_json).collect::<Vec<_>>(),
        });
        let (tid, name) = (id.to_string(), mind.name.clone());
        self.own_calls.insert(tid.clone(), name.clone());
        let req = self.run(input, true, cx, move |page, msg, cx| match msg {
            BrainMsg::Delta(d) => {
                match page.pending.get_mut(&tid) {
                    Some((_, Pending::Streaming(s))) => s.push_str(&d),
                    Some(p) => p.1 = Pending::Streaming(d),
                    None => {}
                }
                cx.notify();
            }
            BrainMsg::Done(v) => {
                page.pending.remove(&tid);
                page.own_calls.remove(&tid);
                let text = v["text"].as_str().unwrap_or("").trim().to_string();
                if text.is_empty() {
                    page.notice(format!("{name} came back with nothing to say. Try again."), cx);
                } else {
                    let mut m = Message::new(Role::Mind, text);
                    m.mind = name.clone();
                    if let Some(t) = page.thread_mut(&tid) {
                        t.messages.push(m);
                    }
                    page.schedule_save(cx);
                }
                cx.notify();
            }
            BrainMsg::Error { kind, message } => {
                page.pending.remove(&tid);
                page.own_calls.remove(&tid);
                if kind != "cancelled" {
                    page.report(&kind, &message, cx);
                }
                cx.notify();
            }
        });
        self.pending.insert(id.to_string(), (req, Pending::Thinking));
        cx.notify();
    }

    /// A call by name from a script (`call: Skeptic`), or the note's mind.
    pub(super) fn call_by_name(&mut self, name: &str, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.active.clone().filter(|id| self.is_own(id)) else { return false };
        let wanted = name.trim().to_lowercase();
        let mind = if wanted.is_empty() {
            Some(self.mind.clone())
        } else {
            self.callable_minds(cx).into_iter().find(|m| m.name.trim().to_lowercase() == wanted)
        };
        match mind {
            Some(m) => {
                self.call_into_own(&id, m, cx);
                true
            }
            None => false,
        }
    }

    /// One message in a tag. In your own thread your lines are set as your words are, in
    /// your lettering and ink, and a called mind's answer comes under its name in its
    /// thread; in a mind's thread, as before.
    pub(super) fn thread_message(&self, thread: &Thread, m: &Message, t: &Theme, cx: &App) -> AnyElement {
        if !thread.own {
            return self.message_view(m, t, false).into_any_element();
        }
        match m.role {
            Role::You => div()
                .font_family(crate::theme::SERIF)
                .text_size(px(ts::TITLE))
                .line_height(px(21.))
                .text_color(ink(t))
                .child(m.text.clone())
                .into_any_element(),
            Role::Mind => {
                let name = thread.author(m);
                let color = mind_thread(&name, &self.mind.name, cx.global::<Store>().settings.ink, t);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_size(px(ts::SMALL)).font_weight(FontWeight::SEMIBOLD).text_color(color).truncate().child(name))
                    .child(self.message_view(m, t, false))
                    .into_any_element()
            }
        }
    }

    /// The answer streaming into a thread, as a message: in your own thread it carries the
    /// name of the mind you called.
    pub(super) fn partial(&self, id: &str, text: String) -> Message {
        let mut m = Message::new(Role::Mind, text);
        if let Some(name) = self.own_calls.get(id) {
            m.mind = name.clone();
        }
        m
    }

    /// Under a folded tag: replies in a mind's thread, more lines in yours.
    pub(super) fn more_label(thread: &Thread, replies: usize) -> String {
        if thread.own {
            more_lines(thread.messages.len().saturating_sub(1))
        } else if replies == 1 {
            "1 reply".into()
        } else {
            format!("{replies} replies")
        }
    }

    /// Ask <the note's mind> on your own open tag. A long name gives way to "Ask a mind".
    pub(super) fn own_ask_button(&self, id: &str, ring: Hsla, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let name = self.mind.name.clone();
        let label = if name.chars().count() <= 20 { format!("Ask {name}") } else { "Ask a mind".to_string() };
        let tid = id.to_string();
        let mind = self.mind.clone();
        ui::ghost_button(SharedString::from(format!("own-ask-{id}")), label, &t)
            .keyed_as("tag:ask", 4., ring, cx, move |this, window, cx| this.ask_into_own(&tid, mind.clone(), window, cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mind(name: &str) -> Mind {
        Mind { id: name.to_lowercase(), name: name.into(), ..Mind::inkling() }
    }

    #[std::prelude::v1::test]
    fn a_line_calls_the_mind_it_names() {
        let minds = vec![mind("Inkling"), mind("Skeptic"), mind("Socratic"), mind("Socratic tutor"), mind("Staff engineer")];
        let called = |s: &str| called_mind(s, &minds).map(|m| m.name);
        assert_eq!(called("Skeptic, is two hard days enough?").as_deref(), Some("Skeptic"));
        assert_eq!(called("skeptic: really?").as_deref(), Some("Skeptic"));
        assert_eq!(called("hey staff engineer what breaks").as_deref(), Some("Staff engineer"));
        assert_eq!(called("Ask @Inkling about the pace").as_deref(), Some("Inkling"));
        // the longer name wins where both fit
        assert_eq!(called("Socratic tutor, where do I start?").as_deref(), Some("Socratic tutor"));
        // a line for yourself calls nobody
        assert_eq!(called("Skeptics say four is too many."), None);
        assert_eq!(called("Check the club plan, then decide."), None);
        assert_eq!(called("Inkling"), None);
    }

    #[std::prelude::v1::test]
    fn a_call_sends_your_question_and_everything_since() {
        let mut answer = Message::new(Role::Mind, "Which two are hard?");
        answer.mind = "Inkling".into();
        let thread = vec![
            Message::new(Role::You, "Is four too many?"),
            Message::new(Role::You, "Two easy, two hard."),
            answer,
            Message::new(Role::You, "Skeptic, is that enough?"),
        ];
        let (question, since) = split_call(&thread).unwrap();
        assert_eq!(question, "Is four too many?");
        assert_eq!(since.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(), ["Two easy, two hard.", "Which two are hard?", "Skeptic, is that enough?"]);
        assert_eq!(message_json(&since[1]), json!({"role": "mind", "text": "Which two are hard?", "mind": "Inkling"}));
        assert_eq!(message_json(&since[0]), json!({"role": "you", "text": "Two easy, two hard."}));
        // only a thread that starts with your question can be called into
        assert!(split_call(&[]).is_none());
        assert!(split_call(&thread[2..3]).is_none());
    }

    #[std::prelude::v1::test]
    fn your_thread_is_named_you_and_sewn_in_your_ink() {
        use crate::store::{Appearance, InkColor, Lettering, TextSize};
        let mut own = Thread { id: "o".into(), quote: "four days a week".into(), own: true, ..Default::default() };
        let mut called = Message::new(Role::Mind, "Which two?");
        called.mind = "Skeptic".into();
        own.messages = vec![Message::new(Role::You, "Too many?"), called, Message::new(Role::Mind, "unnamed")];
        assert_eq!(own.asker(), "You");
        assert_eq!(own.messages.iter().map(|m| own.author(m)).collect::<Vec<_>>(), ["You", "Skeptic", "Inkling"]);
        let theirs = Thread { id: "m".into(), quote: "q".into(), mind_name: "Editor".into(), ..Default::default() };
        assert_eq!(theirs.asker(), "Editor");
        assert_eq!(theirs.author(&Message::new(Role::Mind, "Why?")), "Editor");
        assert_eq!(Thread { mind_name: String::new(), ..theirs.clone() }.asker(), "Inkling");
        for (a, w) in [(Appearance::Light, WindowAppearance::Light), (Appearance::Dark, WindowAppearance::Dark)] {
            for chosen in InkColor::ALL.iter().copied() {
                let t = Theme::resolve(a, w, chosen, Lettering::Own, TextSize::Default);
                assert_eq!(ink(&t), t.p.ink);
                for other in InkColor::ALL.iter().copied() {
                    assert_ne!(ink(&t), crate::theme::ink_color(other, &t.p), "no mind asks in your ink");
                }
            }
        }
        assert_eq!(more_lines(1), "1 more line");
        assert_eq!(more_lines(3), "3 more lines");
    }
}

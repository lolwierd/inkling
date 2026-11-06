//! Answers in the note: a thread tied off by words you wrote in the note, which its knot
//! points to (`design/features/answer-in-the-note.md`). The page side (the knot, the tag's
//! "Answered in your note", the checks it sends) is `note_page/answers.rs`; this file holds
//! the rules the app checks on its own, before a paragraph may tie a thread off. The brain's
//! read names candidates and the thread's own mind confirms (`settle`); these can only say no.

use crate::store::{Role, ThoughtKind, Thread, ThreadStatus};

const SUFFIX: &str = "#answer";

/// How many words in a row, taken from the mind's own messages, make a paragraph a copy.
pub const COPY_RUN: usize = 8;
/// The most words a thread remembers as not its answer.
pub const NOT_ANSWERS_MAX: usize = 5;

/// The editor anchor that keeps a thread's answer on its words through edits. It has no
/// stitch style, so the editor draws nothing for it (the page draws the knot), and a click
/// never opens it.
pub fn anchor_id(thread: &str) -> String {
    format!("{thread}{SUFFIX}")
}

/// The thread an answer anchor belongs to.
pub fn thread_of(anchor: &str) -> Option<&str> {
    anchor.strip_suffix(SUFFIX)
}

/// Can words you write in the note answer this thread now? Any open thread, whatever its
/// kind: a mind's question or correction, something you asked, your own question.
pub fn answerable(t: &Thread) -> bool {
    t.status == ThreadStatus::Open
}

/// You asked for an explanation (an ask, a call by name, a sketch): it counts only once it
/// is in your own words.
pub fn writer_asked(kind: ThoughtKind) -> bool {
    matches!(kind, ThoughtKind::Ask | ThoughtKind::Answer | ThoughtKind::Sketch)
}

/// The words of `s`, lower case, letters and digits only.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// Does `paragraph` copy [`COPY_RUN`] words in a row from the mind's messages in `t`,
/// ignoring case and punctuation? Then it is the mind's explanation, not yours.
pub fn copies_mind(paragraph: &str, t: &Thread) -> bool {
    let mine = words(paragraph);
    if mine.len() < COPY_RUN {
        return false;
    }
    let theirs: std::collections::HashSet<String> = t
        .messages
        .iter()
        .filter(|m| m.role == Role::Mind)
        .flat_map(|m| words(&m.text).windows(COPY_RUN).map(|w| w.join(" ")).collect::<Vec<_>>())
        .collect();
    mine.windows(COPY_RUN).any(|w| theirs.contains(&w.join(" ")))
}

/// Has the writer said these words are not `t`'s answer? The same words (ignoring case,
/// spacing and punctuation), or a part of words they turned down. Words that say more
/// than what was turned down are judged afresh.
pub fn is_not_answer(answer: &str, t: &Thread) -> bool {
    let a = words(answer).join(" ");
    !a.is_empty() && t.not_answers.iter().map(|n| words(n).join(" ")).any(|n| !n.is_empty() && n.contains(&a))
}

/// Remember that `words` are not `t`'s answer; past [`NOT_ANSWERS_MAX`] the oldest goes.
pub fn push_not_answer(t: &mut Thread, words: &str) {
    let w = words.trim();
    if w.is_empty() || t.not_answers.iter().any(|n| n == w) {
        return;
    }
    t.not_answers.push(w.to_string());
    if t.not_answers.len() > NOT_ANSWERS_MAX {
        t.not_answers.remove(0);
    }
}

/// A thread's kind as the minds are told it.
pub fn kind_wire(kind: ThoughtKind) -> &'static str {
    match kind {
        ThoughtKind::Question => "question",
        ThoughtKind::Correction => "correction",
        ThoughtKind::Sketch => "sketch",
        ThoughtKind::Ask => "ask",
        ThoughtKind::Answer => "answer",
    }
}

/// What a knot's tooltip says: whose thread it ties off, and on which of your words.
pub fn knot_label(t: &Thread) -> String {
    let mind = if t.mind_name.trim().is_empty() { "Inkling" } else { t.mind_name.trim() };
    let on = crate::note_page::short_title(&t.quote.replace('\n', " "), 56);
    format!("Answers {mind}\u{2019}s thread on \u{201c}{on}\u{201d}. Click to see it.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Message;

    fn thread(kind: ThoughtKind, mind_says: &[&str]) -> Thread {
        Thread {
            id: "t2".into(),
            quote: "add a kilometre every week until I reach 16".into(),
            kind,
            messages: mind_says.iter().map(|m| Message::new(Role::Mind, *m)).collect(),
            mind_name: "Inkling".into(),
            ..Default::default()
        }
    }

    #[std::prelude::v1::test]
    fn an_answer_anchor_names_its_thread() {
        assert_eq!(anchor_id("t2"), "t2#answer");
        assert_eq!(thread_of(&anchor_id("t2")), Some("t2"));
        assert_eq!(thread_of("t2"), None);
    }

    #[std::prelude::v1::test]
    fn only_open_threads_can_be_answered() {
        let mut t = thread(ThoughtKind::Question, &[]);
        assert!(answerable(&t));
        for status in [ThreadStatus::Resolved, ThreadStatus::Dismissed] {
            t.status = status;
            assert!(!answerable(&t));
        }
        assert!(writer_asked(ThoughtKind::Ask) && writer_asked(ThoughtKind::Answer) && writer_asked(ThoughtKind::Sketch));
        assert!(!writer_asked(ThoughtKind::Question) && !writer_asked(ThoughtKind::Correction));
    }

    #[std::prelude::v1::test]
    fn eight_words_in_a_row_from_the_mind_are_a_copy() {
        let t = thread(ThoughtKind::Ask, &["VO2 max is the most oxygen your body can use in a minute of hard running."]);
        // pasted, with its case and punctuation changed
        assert!(copies_mind("vo2 max is the MOST oxygen your body, can use!", &t));
        // seven words in a row is still your sentence
        assert!(!copies_mind("I read that the most oxygen your body can, and then I stopped.", &t));
        // in your own words
        assert!(!copies_mind("VO2 max is the ceiling on how much oxygen I can burn per minute, which caps my pace.", &t));
        // your own words are never a copy of your own question
        let mut asked = thread(ThoughtKind::Ask, &[]);
        asked.messages.push(Message::new(Role::You, "what does vo2 max mean for a runner like me exactly"));
        assert!(!copies_mind("what does vo2 max mean for a runner like me exactly", &asked));
    }

    #[std::prelude::v1::test]
    fn words_turned_down_never_answer_again_but_more_words_are_judged_afresh() {
        let mut t = thread(ThoughtKind::Question, &[]);
        push_not_answer(&mut t, "The long run is on Sunday.");
        assert!(is_not_answer("the long run is on sunday", &t));
        assert!(is_not_answer("long run is on Sunday", &t), "a part of what was turned down");
        assert!(!is_not_answer("The long run is on Sunday and starts at 8 km.", &t));
        assert!(!is_not_answer("", &t));
        for i in 0..7 {
            push_not_answer(&mut t, &format!("words {i}"));
        }
        push_not_answer(&mut t, "words 6");
        assert_eq!(t.not_answers.len(), NOT_ANSWERS_MAX);
        assert_eq!(t.not_answers.first().map(String::as_str), Some("words 2"), "the oldest go first");
    }

    #[std::prelude::v1::test]
    fn a_knot_says_whose_thread_on_which_words() {
        let t = thread(ThoughtKind::Question, &[]);
        assert_eq!(knot_label(&t), "Answers Inkling\u{2019}s thread on \u{201c}add a kilometre every week until I reach 16\u{201d}. Click to see it.");
        let long = Thread { quote: "a very long quote ".repeat(10), mind_name: String::new(), ..t };
        assert!(knot_label(&long).starts_with("Answers Inkling\u{2019}s thread on \u{201c}a very long quote"));
        assert!(knot_label(&long).contains("\u{2026}\u{201d}"));
    }
}

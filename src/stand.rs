//! Let it stand: a mind's point you let stand is remembered, and every mind that reads
//! the note is told it, so none raises it again.
//!
//! The thread itself keeps the decision (`Thread::reason`, `ended_at`, stored as
//! Dismissed); this module decides what the minds are told and what the page says.

use serde::Serialize;

use crate::store::{Ending, Role, ThoughtKind, Thread, ThreadStatus};

/// Every point whose words are still in the note reaches a prompt, up to this many (a
/// guard on the prompt's size, not a budget: a point dropped could be raised again).
pub const TOLD_MAX: usize = 40;
/// Points whose words have gone from the note fill in up to this many in all.
pub const GONE_MAX: usize = 12;

/// The tag's button on a mind's open point, and its tooltip.
pub const BUTTON: &str = "Let stand";
pub const TOOLTIP: &str = "Keep your words as they are. No mind will raise this again, and what you typed above goes with it as your reason.";

/// The toast after Let stand, without and with a reason.
pub const LET_STAND: &str = "Let stand. No mind will raise it again.";
pub const LET_STAND_WHY: &str = "Let stand, with your reason. No mind will raise it again.";
/// The toast after Dismiss on anything that is not a mind's point (as before).
pub const DISMISSED: &str = "Thread dismissed";
/// The toast when Find a loose thread only found a point you let stand.
pub const NOTHING_NEW: &str = "Nothing new to ask here: you let its point stand.";

/// A point you let stand, as the minds are told it (`ctx.letStand` in the brain).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Point {
    /// The words, as they were when you let it stand.
    pub quote: String,
    /// The mind's first message: what it asked or corrected.
    pub point: String,
    pub kind: &'static str,
    /// The mind that raised it.
    pub mind: String,
    pub reason: Option<String>,
}

/// Whether the open thread's second button lets it stand (a mind's point that is still
/// open) rather than dismissing it.
pub fn lets_stand(t: &Thread) -> bool {
    t.status == ThreadStatus::Open && t.is_point()
}

/// Dismiss `t` from its tag. A mind's open point is let stand, with what you typed in the
/// tag's box as your reason; anything else is snipped as before. Returns the toast.
pub fn dismiss(t: &mut Thread, typed: &str) -> &'static str {
    let point = lets_stand(t);
    t.set_status(ThreadStatus::Dismissed);
    if !point {
        return DISMISSED;
    }
    t.set_reason(typed);
    if t.reason.is_some() { LET_STAND_WHY } else { LET_STAND }
}

/// The points let stand in a note, as every mind is told them: every one whose words are
/// still in `text` (the note's text), up to [`TOLD_MAX`], then the ones whose words have
/// gone while fewer than [`GONE_MAX`] are told; newest first within each, each field one
/// line and cut short.
pub fn points(threads: &[Thread], text: &str) -> Vec<Point> {
    let mut stood: Vec<(&Thread, &str)> = threads
        .iter()
        .filter(|t| t.ending() == Ending::LetStand)
        .filter_map(|t| {
            let first = t.messages.iter().find(|m| m.role == Role::Mind)?;
            (!first.text.trim().is_empty() && !t.quote.trim().is_empty()).then_some((t, first.text.as_str()))
        })
        .collect();
    // in the note first; newest first (a thread let stand before dates were kept counts
    // from when it was opened)
    stood.sort_by_key(|(t, _)| (!text.contains(t.quote.as_str()), std::cmp::Reverse(t.ended_at.unwrap_or(t.created_at))));
    let present = stood.iter().filter(|(t, _)| text.contains(t.quote.as_str())).count().min(TOLD_MAX);
    let gone = GONE_MAX.saturating_sub(present);
    stood
        .into_iter()
        .take(present + gone)
        .map(|(t, first)| Point {
            quote: line(&t.quote, 160),
            point: line(first, 240),
            kind: if t.kind == ThoughtKind::Correction { "correction" } else { "question" },
            mind: if t.mind_name.trim().is_empty() { "Inkling".into() } else { line(&t.mind_name, 60) },
            reason: t.reason.as_deref().map(|r| line(r, 200)).filter(|r| !r.is_empty()),
        })
        .collect()
}

/// `s` on one line, cut to `max` characters with an ellipsis.
fn line(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        return s;
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{}\u{2026}", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Message, Side};
    use chrono::{Duration, Utc};

    fn thread(quote: &str, kind: ThoughtKind, status: ThreadStatus, mind: &str, said: &str) -> Thread {
        Thread {
            id: crate::store::new_id(),
            start: 0,
            end: quote.len(),
            quote: quote.into(),
            kind,
            messages: if said.is_empty() { vec![] } else { vec![Message::new(Role::Mind, said)] },
            drawing: None,
            status,
            side: Side::Right,
            mind_name: mind.into(),
            tag_offset: (0., 0.),
            created_at: Utc::now(),
            reason: None,
            ended_at: None,
            first_quote: None,
            ..Default::default()
        }
    }

    fn stood(quote: &str, said: &str, minutes_ago: i64) -> Thread {
        let mut t = thread(quote, ThoughtKind::Question, ThreadStatus::Dismissed, "Skeptic", said);
        t.ended_at = Some(Utc::now() - Duration::minutes(minutes_ago));
        t
    }

    #[std::prelude::v1::test]
    fn a_thread_of_your_own_is_never_let_stand_even_after_a_mind_answered_in_it() {
        // your own question, which you called Skeptic into, then cut
        let mut own = thread("four days a week", ThoughtKind::Question, ThreadStatus::Open, "", "");
        own.own = true;
        own.messages = vec![Message::new(Role::You, "Too many?"), Message::new(Role::Mind, "Two of them easy.")];
        own.messages[1].mind = "Skeptic".into();
        assert!(!lets_stand(&own), "its second button is Dismiss, not Let stand");
        assert_eq!(dismiss(&mut own, "I decided"), DISMISSED);
        assert_eq!((own.ending(), own.reason.as_deref()), (Ending::Snipped, None));
        // and no mind is told it as a point let stand
        let theirs = stood("hold 5:30", "Is 5:30 race pace?", 1);
        assert_eq!(points(&[own, theirs], "four days a week, hold 5:30").iter().map(|p| p.quote.as_str()).collect::<Vec<_>>(), ["hold 5:30"]);
    }

    #[std::prelude::v1::test]
    fn only_points_let_stand_are_told() {
        let threads = vec![
            thread("open words", ThoughtKind::Question, ThreadStatus::Open, "Inkling", "Open?"),
            thread("tied words", ThoughtKind::Correction, ThreadStatus::Resolved, "Inkling", "Tied?"),
            thread("asked words", ThoughtKind::Ask, ThreadStatus::Dismissed, "Inkling", "Snipped."),
            thread("sketched words", ThoughtKind::Sketch, ThreadStatus::Dismissed, "Inkling", "Snipped."),
            // a point whose mind never said anything has nothing to tell
            thread("silent words", ThoughtKind::Question, ThreadStatus::Dismissed, "Inkling", ""),
            thread("four days a week", ThoughtKind::Correction, ThreadStatus::Dismissed, "Skeptic", "Is pace the problem?"),
        ];
        let told = points(&threads, "four days a week");
        assert_eq!(told.len(), 1);
        assert_eq!(
            told[0],
            Point { quote: "four days a week".into(), point: "Is pace the problem?".into(), kind: "correction", mind: "Skeptic".into(), reason: None }
        );
    }

    #[std::prelude::v1::test]
    fn points_in_the_note_come_first_then_newest() {
        let threads = vec![stood("gone words", "Gone, newest?", 1), stood("old words", "Old?", 30), stood("new words", "New?", 5)];
        let told = points(&threads, "Some old words and some new words.");
        let quotes: Vec<&str> = told.iter().map(|p| p.quote.as_str()).collect();
        assert_eq!(quotes, ["new words", "old words", "gone words"]);
    }

    #[std::prelude::v1::test]
    fn every_point_still_in_the_note_and_a_few_gone_each_cut_short() {
        let mut threads: Vec<Thread> = (0..20).map(|i| stood(&format!("words {i}"), "Why?", i)).collect();
        threads[0].quote = "é".repeat(400);
        threads[0].messages[0].text = format!("Line one\n\nline   two {}", "ü".repeat(400));
        threads[0].set_reason(&"ß".repeat(300));
        // none of their words are in the note: the newest few are kept
        let told = points(&threads, "");
        assert_eq!(told.len(), GONE_MAX);
        assert_eq!(told.last().unwrap().quote, "words 11");
        // nineteen of their words are: all nineteen are told, and the gone one is not
        let note: String = (1..20).map(|i| format!("words {i}. ")).collect();
        assert_eq!(points(&threads, &note).len(), 19);
        let p = &told[0];
        assert_eq!((p.quote.chars().count(), p.point.chars().count()), (160, 240));
        assert!(p.quote.ends_with('\u{2026}') && p.point.starts_with("Line one line two üü"));
        assert_eq!(p.reason.as_ref().unwrap().chars().count(), 200);
    }

    #[std::prelude::v1::test]
    fn old_dismissed_points_are_told_without_a_reason() {
        // dismissed before this feature: no reason, no date
        let old = thread("always", ThoughtKind::Question, ThreadStatus::Dismissed, "", "Always? What is the evidence?");
        let told = points(&[old], "Users always read the docs first.");
        assert_eq!(told[0].mind, "Inkling");
        assert_eq!(told[0].reason, None);
    }

    #[std::prelude::v1::test]
    fn dismissing_a_point_lets_it_stand_with_what_you_typed() {
        let mut t = thread("four days a week", ThoughtKind::Question, ThreadStatus::Open, "Skeptic", "Is pace the problem?");
        assert!(lets_stand(&t));
        assert_eq!(dismiss(&mut t, "  Pace is the problem;\nthe walk was pace. "), LET_STAND_WHY);
        assert_eq!((t.status, t.ending()), (ThreadStatus::Dismissed, Ending::LetStand));
        assert_eq!(t.reason.as_deref(), Some("Pace is the problem; the walk was pace."));
        assert!(t.ended_at.is_some());

        let mut quiet = thread("q", ThoughtKind::Correction, ThreadStatus::Open, "Inkling", "Wrong?");
        assert_eq!(dismiss(&mut quiet, "   "), LET_STAND);
        assert_eq!(quiet.reason, None);

        // anything that is not a mind's open point is dismissed as before, and keeps no reason
        let mut asked = thread("q", ThoughtKind::Ask, ThreadStatus::Open, "Inkling", "An answer.");
        assert!(!lets_stand(&asked));
        assert_eq!(dismiss(&mut asked, "typed"), DISMISSED);
        assert_eq!((asked.ending(), asked.reason.as_deref()), (Ending::Snipped, None));
        let mut tied = thread("q", ThoughtKind::Question, ThreadStatus::Resolved, "Inkling", "Settled.");
        assert!(!lets_stand(&tied));
        assert_eq!(dismiss(&mut tied, "typed"), DISMISSED);
        assert_eq!(tied.reason, None);
    }
}

//! Show its working: a correction carries a check you can do yourself, a sum made from your
//! own numbers or your own words from elsewhere in the note. The brain makes sure of it
//! before it leaves (`brain/src/working.ts`); this is the app's side without GPUI: reading
//! the check off the wire, the words it is written in for Copy, exports and the minds, and
//! where the words it rests on are now. The tag draws it in `note_page/show_working.rs`.

use crate::store::{Check, Given, Message, Passage, SumLine};
use serde_json::Value;
use std::ops::Range;

/// The check a `read` or `probe` answer carries, if it has a usable one.
pub fn from_wire(v: &Value) -> Option<Check> {
    let obj = v.as_object()?;
    let text = |v: &Value| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let items = |key: &str| obj.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let sum: Vec<SumLine> = items("sum")
        .iter()
        .filter_map(|l| Some(SumLine { sum: text(&l["sum"])?, is: text(&l["is"])?, rounded: l["rounded"].as_bool() == Some(true) }))
        .collect();
    let givens: Vec<Given> = items("givens").iter().filter_map(|g| Some(Given { value: text(&g["value"])?, words: text(&g["words"])? })).collect();
    let words: Vec<Passage> = items("words").iter().filter_map(text).map(|words| Passage { words, note: None }).collect();
    (!sum.is_empty() || !words.is_empty()).then(|| Check { sum, givens, words, taken_back: false })
}

/// `=`, or `≈` when the result is rounded.
pub fn sign(line: &SumLine) -> &'static str {
    if line.rounded { "\u{2248}" } else { "=" }
}

/// The check in words, one (label, body) per kind, for Copy, exports and the context:
/// ("Working", "5:30 × 10 = 55:00 · 50:00 ÷ 10 = 5:00 (from your words: “hold 5:30”, “10k”)"),
/// ("In your note", "“run four days a week”"). A check taken back says so in its label.
pub fn parts(c: &Check) -> Vec<(String, String)> {
    let quoted = |w: &str| format!("\u{201c}{w}\u{201d}");
    let back = if c.taken_back { ", taken back" } else { "" };
    let mut out = vec![];
    if !c.sum.is_empty() {
        let lines: Vec<String> = c.sum.iter().map(|l| format!("{} {} {}", l.sum, sign(l), l.is)).collect();
        let mut body = lines.join(" \u{b7} ");
        if !c.givens.is_empty() {
            let words: Vec<String> = c.givens.iter().map(|g| quoted(&g.words)).collect();
            body.push_str(&format!(" (from your words: {})", words.join(", ")));
        }
        out.push((format!("Working{back}"), body));
    }
    if !c.words.is_empty() {
        let words: Vec<String> = c.words.iter().map(|p| quoted(&p.words)).collect();
        out.push((format!("In your note{back}"), words.join(", ")));
    }
    out
}

/// The check as plain lines: `Working: 5:30 × 10 = 55:00 (from your words: …)`.
pub fn text_lines(c: &Check) -> Vec<String> {
    parts(c).into_iter().map(|(label, body)| format!("{label}: {body}")).collect()
}

/// A message as the minds read it back: its words, then the working it showed, so a later
/// reply or read knows what was checked (and what was taken back).
pub fn wire_text(m: &Message) -> String {
    match &m.check {
        Some(c) => std::iter::once(m.text.clone()).chain(text_lines(c)).collect::<Vec<_>>().join("\n"),
        None => m.text.clone(),
    }
}

/// Where words a check rests on are now.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// In the note: their byte range in its text (blocks joined by `\n`, title included).
    Note(Range<usize>),
    /// In the brief, above the title.
    Brief,
    /// In another note (Threads between notes).
    Elsewhere,
    /// No longer anywhere: the words were changed.
    Changed,
}

/// Where `words` are: in the note's `text`, its `brief`, or gone.
pub fn place(text: &str, brief: Option<&str>, words: &str) -> Place {
    if words.is_empty() {
        return Place::Changed;
    }
    match text.find(words) {
        Some(at) => Place::Note(at..at + words.len()),
        None if brief.is_some_and(|b| b.contains(words)) => Place::Brief,
        None => Place::Changed,
    }
}

/// Every set of words the check rests on, in the order the tag lists them: the givens'
/// words, then the passages.
pub fn words_of(c: &Check) -> Vec<(String, Option<String>)> {
    c.givens.iter().map(|g| (g.words.clone(), None)).chain(c.words.iter().map(|p| (p.words.clone(), p.note.clone()))).collect()
}

/// A short form for a page dump: `sum:2`, `words:1`, `sum:2:taken back`.
pub fn summary(c: &Check) -> String {
    let kind = if c.sum.is_empty() { format!("words:{}", c.words.len()) } else { format!("sum:{}", c.sum.len()) };
    if c.taken_back { format!("{kind}:taken back") } else { kind }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Role;
    use serde_json::json;

    fn running_plan() -> Check {
        from_wire(&json!({
            "sum": [{"sum": "5:30 × 10", "is": "55:00", "rounded": false}, {"sum": "50:00 ÷ 7", "is": "7:09", "rounded": true}],
            "givens": [{"value": "5:30", "words": "hold 5:30"}, {"value": "10", "words": "10k"}],
            "words": [],
        }))
        .unwrap()
    }

    #[std::prelude::v1::test]
    fn the_wire_check_is_read_and_a_useless_one_is_none() {
        let c = running_plan();
        assert_eq!(c.sum.len(), 2);
        assert!(c.sum[1].rounded && !c.taken_back);
        assert_eq!(c.givens[1], Given { value: "10".into(), words: "10k".into() });
        let words = from_wire(&json!({"words": ["run four days a week", "  "]})).unwrap();
        assert_eq!(words.words, vec![Passage { words: "run four days a week".into(), note: None }]);
        assert_eq!(from_wire(&json!({"sum": [{"sum": "1 + 1"}], "words": []})), None);
        assert_eq!(from_wire(&json!({})), None);
        assert_eq!(from_wire(&Value::Null), None);
        assert_eq!(from_wire(&json!("5:30 × 10 = 55:00")), None);
    }

    #[std::prelude::v1::test]
    fn the_working_in_words() {
        let mut c = running_plan();
        assert_eq!(
            text_lines(&c),
            vec!["Working: 5:30 × 10 = 55:00 \u{b7} 50:00 ÷ 7 \u{2248} 7:09 (from your words: \u{201c}hold 5:30\u{201d}, \u{201c}10k\u{201d})"]
        );
        c.taken_back = true;
        c.words = vec![Passage { words: "four days".into(), note: None }];
        let lines = text_lines(&c);
        assert!(lines[0].starts_with("Working, taken back: 5:30"));
        assert_eq!(lines[1], "In your note, taken back: \u{201c}four days\u{201d}");
        assert_eq!(summary(&c), "sum:2:taken back");
    }

    #[std::prelude::v1::test]
    fn the_minds_read_the_working_back() {
        let mut m = Message::new(Role::Mind, "At 5:30 a km, 10 km takes 55:00.");
        assert_eq!(wire_text(&m), "At 5:30 a km, 10 km takes 55:00.");
        m.check = Some(running_plan());
        assert!(wire_text(&m).starts_with("At 5:30 a km, 10 km takes 55:00.\nWorking: 5:30 × 10 = 55:00"));
    }

    #[std::prelude::v1::test]
    fn words_are_found_in_the_note_the_brief_or_are_changed() {
        let text = "Sub-50 10k\nI hold 5:30 per km.";
        assert_eq!(place(text, None, "hold 5:30"), Place::Note(13..22));
        assert_eq!(place(text, Some("Race on 14 March"), "14 March"), Place::Brief);
        assert_eq!(place(text, None, "hold 5:20"), Place::Changed);
        assert_eq!(place(text, None, ""), Place::Changed);
        let c = running_plan();
        assert_eq!(words_of(&c).iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>(), vec!["hold 5:30", "10k"]);
    }
}

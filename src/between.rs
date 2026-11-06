//! Threads between notes, the data side (design: `design/features/threads-between-notes.md`).
//!
//! Two jobs. **Finding** words in other notes that a paragraph may disagree with: a local
//! index of every paragraph in the notebook, kept in memory and never sent anywhere, scores
//! the paragraph just read against all the others by the uncommon words they share; only a
//! close enough match goes to the model (`relate`). **Keeping** a thread that quotes two
//! notes: the thread lives in its home note with an [`Elsewhere`] beside it; when the other
//! note opens, a visiting end (a clone of the thread on the other words) joins that note's
//! threads, and saving sends its state home, so either end can change the one thread.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::store::{BlockKind, Elsewhere, Mind, Mode, Note, Store, Thread, ThreadStatus};

/// At most this many passages go with one `relate`, and at most `PER_NOTE` from one note.
pub const MAX_PASSAGES: usize = 3;
const PER_NOTE: usize = 2;
/// No new thread between notes while a note has this many open (its own and visiting).
pub const MAX_OPEN: usize = 3;
/// A passage is at most this many characters, cut around the words it shares.
pub const PASSAGE_CHARS: usize = 600;
/// A unit is at least this many words long: shorter lines say too little to clash.
const MIN_WORDS: usize = 4;
/// A shared term counts toward the gate when it is this uncommon in the notebook (an IDF
/// of 1.0: it appears in fewer than about a third of the paragraphs).
const UNCOMMON_IDF: f32 = 1.0;
/// The gate: at least this many uncommon terms shared, and at least this score.
const MIN_SHARED: usize = 2;
const MIN_SCORE: f32 = 0.20;
/// Added to the score for each kind of value both paragraphs carry, at most twice.
const VALUE_BOOST: f32 = 0.04;

// ----- terms ---------------------------------------------------------------------------

const STOP: &[&str] = &[
    "a", "about", "after", "again", "against", "all", "also", "am", "an", "and", "any", "are", "as", "at", "be", "because",
    "been", "before", "being", "below", "between", "both", "but", "by", "can", "could", "d", "did", "do", "does", "doing",
    "don", "down", "during", "each", "even", "few", "for", "from", "get", "got", "had", "has", "have", "having", "he",
    "her", "here", "hers", "him", "his", "how", "i", "if", "in", "into", "is", "it", "its", "just", "ll", "m", "me", "might",
    "more", "most", "much", "must", "my", "no", "nor", "not", "now", "of", "off", "on", "once", "one", "only", "or", "other",
    "our", "out", "over", "own", "re", "s", "same", "she", "should", "so", "some", "still", "such", "t", "than", "that",
    "the", "their", "them", "then", "there", "these", "they", "this", "those", "through", "to", "too", "under", "until",
    "up", "us", "ve", "very", "was", "we", "were", "what", "when", "where", "which", "while", "who", "whom", "why", "will",
    "with", "would", "you", "your", "yours",
];

/// A word without its common English endings, so "runs" and "run", "dates" and "date"
/// meet. Only ASCII words are stemmed; numbers and other scripts stay as written.
fn stem(w: &str) -> String {
    if !w.is_ascii() || w.bytes().any(|b| b.is_ascii_digit()) {
        return w.to_string();
    }
    let undouble = |s: &str| -> String {
        let b = s.as_bytes();
        let n = b.len();
        if n >= 3 && b[n - 1] == b[n - 2] && !b"aeiouls".contains(&b[n - 1]) { s[..n - 1].to_string() } else { s.to_string() }
    };
    let n = w.len();
    if n > 4 && w.ends_with("ies") {
        return format!("{}y", &w[..n - 3]);
    }
    if n > 5 && w.ends_with("ing") {
        return undouble(&w[..n - 3]);
    }
    if n > 4 && w.ends_with("ed") && !w.ends_with("eed") {
        return undouble(&w[..n - 2]);
    }
    if n > 4 && w.ends_with("es") && ["ses", "xes", "zes", "ches", "shes"].iter().any(|e| w.ends_with(e)) {
        return w[..n - 2].to_string();
    }
    if n > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us") && !w.ends_with("is") {
        return w[..n - 1].to_string();
    }
    w.to_string()
}

/// The words of `text`, lower case, split on anything that is not a letter or a digit.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(|w| w.to_lowercase()).collect()
}

/// The terms of `text` and how often each appears: words without the common ones,
/// stemmed. A number stays when it has two digits or more ("15", "2026", "10k"), so a
/// clock's "00" and single digits do not count.
fn term_counts(text: &str) -> HashMap<String, f32> {
    let mut out: HashMap<String, f32> = HashMap::new();
    for w in words(text) {
        if STOP.contains(&w.as_str()) {
            continue;
        }
        let digits = w.chars().all(|c| c.is_ascii_digit());
        if digits && (w.len() < 2 || w.chars().all(|c| c == '0')) {
            continue;
        }
        if !digits && w.chars().count() < 2 {
            continue;
        }
        *out.entry(stem(&w)).or_default() += 1.;
    }
    out
}

// ----- values ----------------------------------------------------------------------------

/// The kinds of value a paragraph carries. Two paragraphs about the same thing that both
/// carry a date (or an amount, a time...) may well give it differently.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ValueKind {
    Date,
    Money,
    Time,
    Percent,
    Quantity,
}

const MONTHS: &[&str] = &[
    "january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december",
    "jan", "feb", "apr", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec",
];
const WEEKDAYS: &[&str] = &["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
const UNITS: &[&str] = &[
    "k", "km", "kms", "m", "mi", "mile", "miles", "kg", "g", "lb", "lbs", "h", "hr", "hrs", "hour", "hours", "min", "mins",
    "minute", "minutes", "s", "sec", "secs", "second", "seconds", "ms", "day", "days", "week", "weeks", "month", "months",
    "year", "years", "gb", "tb", "mb", "kb", "x", "people", "users", "requests", "rps", "qps", "pages", "words", "copies",
    "nodes", "servers", "cores", "runs", "laps", "reps", "sets",
];
const CURRENCY_WORDS: &[&str] = &["dollar", "dollars", "euro", "euros", "pound", "pounds", "rupee", "rupees", "usd", "eur", "gbp", "inr", "bucks"];

fn value_kinds(text: &str) -> HashSet<ValueKind> {
    let mut out = HashSet::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        let digit_after = chars.get(i + 1).is_some_and(|c| c.is_ascii_digit());
        let digit_before = i > 0 && chars[i - 1].is_ascii_digit();
        match c {
            '£' | '$' | '€' | '¥' | '₹' if digit_after || digit_before => {
                out.insert(ValueKind::Money);
            }
            '%' if digit_before => {
                out.insert(ValueKind::Percent);
            }
            ':' if digit_before && digit_after && chars.get(i + 2).is_some_and(|c| c.is_ascii_digit()) => {
                out.insert(ValueKind::Time);
            }
            '-' | '/' if digit_before && digit_after => {
                // 2026-03-15, 15/03
                out.insert(ValueKind::Date);
            }
            _ => {}
        }
    }
    let ws = words(text);
    let number = |w: &str| w.chars().next().is_some_and(|c| c.is_ascii_digit());
    let day_number = |w: &str| {
        let d: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
        let rest = &w[d.len()..];
        !d.is_empty() && d.parse::<u32>().is_ok_and(|n| (1..=31).contains(&n)) && matches!(rest, "" | "st" | "nd" | "rd" | "th")
    };
    for (i, w) in ws.iter().enumerate() {
        let prev = i.checked_sub(1).and_then(|j| ws.get(j)).map(String::as_str).unwrap_or("");
        let next = ws.get(i + 1).map(String::as_str).unwrap_or("");
        let next2 = ws.get(i + 2).map(String::as_str).unwrap_or("");
        if WEEKDAYS.contains(&w.as_str()) {
            out.insert(ValueKind::Date);
        }
        // "15 March", "March 15", "the 15th of March"; "may" only beside a day
        if MONTHS.contains(&w.as_str()) && (day_number(prev) || day_number(next) || (prev == "of" && i >= 2 && day_number(&ws[i - 2]))) {
            out.insert(ValueKind::Date);
        }
        if MONTHS.contains(&w.as_str()) && w != "may" && number(next) && next.len() == 4 {
            out.insert(ValueKind::Date);
        }
        if number(w) {
            let d: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
            let glued = &w[d.len()..];
            if matches!(glued, "am" | "pm") || (glued.is_empty() && matches!(next, "am" | "pm")) {
                out.insert(ValueKind::Time);
            }
            if CURRENCY_WORDS.contains(&next) || CURRENCY_WORDS.contains(&glued) || matches!(glued, "k" if CURRENCY_WORDS.contains(&next)) {
                out.insert(ValueKind::Money);
            }
            if next == "percent" {
                out.insert(ValueKind::Percent);
            }
            if (!glued.is_empty() && UNITS.contains(&glued)) || (glued.is_empty() && UNITS.contains(&next)) || (glued.is_empty() && next == "a" && UNITS.contains(&next2)) {
                out.insert(ValueKind::Quantity);
            }
        }
    }
    out
}

// ----- the index -------------------------------------------------------------------------

/// One paragraph (or list item, checklist item, quote or callout line) of a note.
struct Unit {
    /// Its block in the note (0 is the title).
    block: usize,
    text: String,
    /// The nearest heading above it.
    heading: Option<String>,
    tf: HashMap<String, f32>,
    kinds: HashSet<ValueKind>,
}

struct Indexed {
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    title: String,
    /// Whether its words may be the other end of a thread at all.
    searchable: bool,
    units: Vec<Unit>,
}

/// Every searchable paragraph of the notebook, in memory. It is never written anywhere:
/// each look re-indexes the notes saved since the last one and forgets deleted notes.
#[derive(Default)]
pub struct Index {
    notes: HashMap<String, Indexed>,
    /// In how many units of searchable notes each term appears.
    df: HashMap<String, u32>,
    units: usize,
}

thread_local! {
    static INDEX: RefCell<Index> = RefCell::new(Index::default());
}

/// The app's index (one per thread; the app runs on one).
pub fn with_index<R>(f: impl FnOnce(&mut Index) -> R) -> R {
    INDEX.with(|i| f(&mut i.borrow_mut()))
}

/// The welcome guide: Inkling's own words, never the other end of a thread.
fn is_guide(note: &Note) -> bool {
    note.first_run || (!note.left_alone.is_empty() && matches!(note.title.trim(), "Start here" | "Welcome to Inkling"))
}

/// May this note's words be the other end of a thread between notes? Only a note that
/// reads along: a note that waits to be asked is left alone until it is asked, and the
/// welcome guide is Inkling's words, not yours. Imported words count (they are only ever
/// sent when something you write later shares their subject).
pub fn searchable(note: &Note) -> bool {
    note.mode == Mode::ThinksWithYou && !is_guide(note)
}

/// Lines Inkling wrote: what Make a note puts in a note, "Inkling: …", "Skeptic: …".
fn mind_written(text: &str, minds: &[String]) -> bool {
    text.split_once(':').is_some_and(|(who, _)| minds.iter().any(|m| m == &who.trim().to_lowercase()))
}

fn index_note(note: &Note, minds: &[String]) -> Indexed {
    let searchable = searchable(note);
    let mut units = vec![];
    if searchable {
        let mut heading: Option<String> = None;
        for (i, b) in note.blocks.iter().enumerate().skip(1) {
            let text = b.text.trim();
            match b.kind {
                BlockKind::H1 | BlockKind::H2 | BlockKind::H3 => {
                    heading = (!text.is_empty()).then(|| text.to_string());
                    continue;
                }
                BlockKind::Title | BlockKind::Code(_) => continue,
                _ => {}
            }
            if text.split_whitespace().count() < MIN_WORDS || mind_written(text, minds) {
                continue;
            }
            units.push(Unit { block: i, text: text.to_string(), heading: heading.clone(), tf: term_counts(text), kinds: value_kinds(text) });
        }
    }
    Indexed { updated_at: note.updated_at, created_at: note.created_at, title: note.title.clone(), searchable, units }
}

/// One passage from another note that shares a paragraph's subject: what goes to `relate`.
#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    pub note: String,
    pub note_title: String,
    pub written: DateTime<Utc>,
    pub edited: DateTime<Utc>,
    pub heading: Option<String>,
    /// Its block in that note.
    pub block: usize,
    /// The words, cut to at most [`PASSAGE_CHARS`] around what they share.
    pub text: String,
    pub score: f32,
}

/// Open threads on `note` that run to another note (its own and visiting ones).
pub fn open_between(note: &Note) -> usize {
    note.threads.iter().filter(|t| t.status == ThreadStatus::Open && note.elsewhere_of(&t.id).is_some()).count()
}

impl Index {
    fn learn(&mut self, ix: &Indexed) {
        for u in &ix.units {
            self.units += 1;
            for t in u.tf.keys() {
                *self.df.entry(t.clone()).or_default() += 1;
            }
        }
    }

    fn forget(&mut self, ix: &Indexed) {
        for u in &ix.units {
            self.units = self.units.saturating_sub(1);
            for t in u.tf.keys() {
                if let Some(c) = self.df.get_mut(t) {
                    *c = c.saturating_sub(1);
                    if *c == 0 {
                        self.df.remove(t);
                    }
                }
            }
        }
    }

    /// Bring the index up to date with `notes`: index what was saved since, forget what is gone.
    fn refresh(&mut self, notes: &[Note], minds: &[Mind]) {
        let names: Vec<String> = minds
            .iter()
            .map(|m| m.name.trim().to_lowercase())
            .chain(["inkling".to_string()])
            .filter(|n| !n.is_empty())
            .collect();
        let live: HashSet<&str> = notes.iter().map(|n| n.id.as_str()).collect();
        let gone: Vec<String> = self.notes.keys().filter(|id| !live.contains(id.as_str())).cloned().collect();
        for id in gone {
            if let Some(old) = self.notes.remove(&id) {
                self.forget(&old);
            }
        }
        for n in notes {
            let fresh = self
                .notes
                .get(&n.id)
                .is_some_and(|ix| ix.updated_at == n.updated_at && ix.searchable == searchable(n) && ix.title == n.title);
            if fresh {
                continue;
            }
            if let Some(old) = self.notes.remove(&n.id) {
                self.forget(&old);
            }
            let ix = index_note(n, &names);
            self.learn(&ix);
            self.notes.insert(n.id.clone(), ix);
        }
    }

    fn idf(&self, term: &str) -> f32 {
        let n = self.units.max(1) as f32;
        let df = self.df.get(term).copied().unwrap_or(0) as f32;
        ((n + 1.) / (df + 0.5)).ln().max(0.)
    }

    /// Up to [`MAX_PASSAGES`] passages from other searchable notes that share the subject of
    /// `target` (a paragraph of `here`), best first; empty when nothing clears the gate, so
    /// most paragraphs cost no model call. `here` is the page's note, with its visiting ends:
    /// a paragraph already joined to it by a thread (open, tied off or cut) is never offered
    /// again, and a note with [`MAX_OPEN`] open threads between notes gets none.
    pub fn passages(&mut self, notes: &[Note], minds: &[Mind], here: &Note, target: &str) -> Vec<Passage> {
        if open_between(here) >= MAX_OPEN {
            return vec![];
        }
        self.refresh(notes, minds);
        let query = term_counts(target);
        if query.len() < MIN_SHARED {
            return vec![];
        }
        let weight = |tf: f32, idf: f32| (1. + tf.ln()) * idf;
        let qv: Vec<(&str, f32, f32)> = query.iter().map(|(t, c)| (t.as_str(), weight(*c, self.idf(t)), self.idf(t))).collect();
        let qnorm = qv.iter().map(|(_, w, _)| w * w).sum::<f32>().sqrt();
        if qnorm <= 0. {
            return vec![];
        }
        let qkinds = value_kinds(target);
        let joined: Vec<(&str, &str)> = here
            .elsewhere
            .iter()
            .filter(|e| !e.quote.trim().is_empty())
            .map(|e| (e.note.as_str(), e.quote.as_str()))
            .collect();
        let mut found: Vec<(f32, &str, &Indexed, &Unit, Vec<&str>)> = vec![];
        for (id, ix) in &self.notes {
            if id == &here.id || !ix.searchable {
                continue;
            }
            for u in &ix.units {
                let shared: Vec<&(&str, f32, f32)> = qv.iter().filter(|(t, _, _)| u.tf.contains_key(*t)).collect();
                let mut uncommon: Vec<&(&str, f32, f32)> = shared.iter().copied().filter(|(_, _, idf)| *idf >= UNCOMMON_IDF).collect();
                if uncommon.len() < MIN_SHARED {
                    continue;
                }
                if joined.iter().any(|(n, q)| n == id && u.text.contains(q)) {
                    continue;
                }
                let dot: f32 = shared.iter().map(|(t, w, idf)| w * weight(u.tf[*t], *idf)).sum();
                let unorm = u.tf.iter().map(|(t, c)| weight(*c, self.idf(t)).powi(2)).sum::<f32>().sqrt();
                if unorm <= 0. {
                    continue;
                }
                let both = qkinds.intersection(&u.kinds).count().min(2) as f32;
                let score = dot / (qnorm * unorm) + VALUE_BOOST * both;
                if score < MIN_SCORE {
                    continue;
                }
                // the rarest shared term first: the passage is cut around it
                uncommon.sort_by(|a, b| b.2.total_cmp(&a.2));
                found.push((score, id.as_str(), ix, u, uncommon.iter().map(|(t, _, _)| *t).collect()));
            }
        }
        found.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.2.updated_at.cmp(&a.2.updated_at)).then(a.3.block.cmp(&b.3.block)));
        let mut out: Vec<Passage> = vec![];
        let mut per_note: HashMap<&str, usize> = HashMap::new();
        for (score, id, ix, u, terms) in found {
            if out.len() >= MAX_PASSAGES {
                break;
            }
            let n = per_note.entry(id).or_default();
            if *n >= PER_NOTE {
                continue;
            }
            *n += 1;
            out.push(Passage {
                note: id.to_string(),
                note_title: ix.title.clone(),
                written: ix.created_at,
                edited: ix.updated_at,
                heading: u.heading.clone(),
                block: u.block,
                text: cut_around(&u.text, &terms, PASSAGE_CHARS),
                score,
            });
        }
        out
    }
}

/// `text` cut to at most `max` characters, the window placed so the first of `needles`
/// found (lower case) sits about a third of the way in, with an ellipsis at each cut. Cuts
/// fall between characters, never inside one.
fn cut_around(text: &str, needles: &[&str], max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    // one lower-case char per char, so positions agree
    let lower: Vec<char> = chars.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let lower_s: String = lower.iter().collect();
    let at = needles
        .iter()
        .find_map(|n| lower_s.find(n).map(|b| lower_s[..b].chars().count()))
        .unwrap_or(0);
    let room = max.saturating_sub(2).max(1);
    let mut start = at.saturating_sub(room / 3);
    let mut end = (start + room).min(chars.len());
    if end - start < room {
        start = end.saturating_sub(room);
    }
    if start == 0 {
        end = room.min(chars.len());
    }
    let mut out = String::new();
    if start > 0 {
        out.push('\u{2026}');
    }
    out.extend(&chars[start..end]);
    if end < chars.len() {
        out.push('\u{2026}');
    }
    out
}

/// A date as the prompts write it: "Thursday 3 September 2026", in local time.
pub fn date_words(t: DateTime<Utc>) -> String {
    t.with_timezone(&chrono::Local).format("%A %-d %B %Y").to_string()
}

/// The passages as `relate` sends them (`brain/src/protocol.ts`, `Passage`).
pub fn passages_wire(passages: &[Passage]) -> Value {
    Value::Array(
        passages
            .iter()
            .map(|p| {
                json!({
                    "note": p.note,
                    "noteTitle": p.note_title,
                    "written": date_words(p.written),
                    "edited": date_words(p.edited),
                    "heading": p.heading,
                    "text": p.text,
                })
            })
            .collect(),
    )
}

// ----- where the words are -----------------------------------------------------------------

/// Byte range of each block in the note's joined text.
fn block_ranges(note: &Note) -> Vec<Range<usize>> {
    let mut at = 0;
    note.blocks
        .iter()
        .map(|b| {
            let r = at..at + b.text.len();
            at = r.end + 1;
            r
        })
        .collect()
}

/// Where `quote` sits in block `block` of `note`, as a range of its joined text. The model
/// may carry the ellipsis of a cut passage into its quote; it is left out.
pub fn locate_in_block(note: &Note, block: usize, quote: &str) -> Option<Range<usize>> {
    let r = block_ranges(note).get(block)?.clone();
    let text = note.blocks.get(block)?.text.as_str();
    let q = quote.trim().trim_matches('\u{2026}').trim();
    if q.is_empty() {
        return None;
    }
    text.find(q).map(|i| r.start + i..r.start + i + q.len())
}

/// Where the far end of a new thread goes in `note`: the words `quote` in block `block`
/// (where the passage came from), else those words nearest that block (the note changed
/// since it was searched), else the whole block. Returns the range and the words there.
pub fn locate_other(note: &Note, block: usize, quote: &str) -> Option<(Range<usize>, String)> {
    let text = note.joined_text();
    let ranges = block_ranges(note);
    let br = ranges.get(block)?.clone();
    let range = locate_in_block(note, block, quote)
        .or_else(|| {
            let q = quote.trim().trim_matches('\u{2026}').trim();
            nearest(&text, q, br.start).map(|i| i..i + q.len())
        })
        .unwrap_or_else(|| {
            let b = &note.blocks[block].text;
            let lead = b.len() - b.trim_start().len();
            br.start + lead..br.start + lead + b.trim().len()
        });
    let words = text[range.clone()].to_string();
    Some((range, words))
}

/// The occurrence of `needle` in `hay` that starts closest to `near`.
fn nearest(hay: &str, needle: &str, near: usize) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    hay.match_indices(needle).map(|(i, _)| i).min_by_key(|i| i.abs_diff(near))
}

/// `range` in `text` when it still holds `quote`; else `quote` nearest where it was; else an
/// empty range at its old place (on a character boundary), as threads whose words are gone.
fn refind(text: &str, range: Range<usize>, quote: &str) -> Range<usize> {
    if text.get(range.clone()).is_some_and(|s| s == quote) {
        return range;
    }
    if let Some(i) = nearest(text, quote, range.start) {
        return i..i + quote.len();
    }
    let mut at = range.start.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at..at
}

/// The paragraph that holds `range` of `note` (its block's words), cut to
/// [`PASSAGE_CHARS`] around `quote`.
fn paragraph_at(note: &Note, range: &Range<usize>, quote: &str) -> Option<String> {
    let ranges = block_ranges(note);
    let i = ranges.iter().position(|r| r.start <= range.start && range.start <= r.end).or_else(|| {
        let q = quote.trim();
        (!q.is_empty()).then(|| note.blocks.iter().position(|b| b.text.contains(q))).flatten()
    })?;
    let text = note.blocks[i].text.trim();
    let needle = quote.trim().to_lowercase();
    Some(cut_around(text, &[needle.as_str()], PASSAGE_CHARS))
}

// ----- visiting ends -----------------------------------------------------------------------

/// The note at the other end of `e`, or `None` once it is deleted.
pub fn other_note<'a>(notes: &'a [Note], e: &Elsewhere) -> Option<&'a Note> {
    notes.iter().find(|n| n.id == e.note)
}

/// The threads in other notes whose far end is in `note_id`: (home note, thread, far end).
pub fn visiting<'a>(notes: &'a [Note], note_id: &str) -> Vec<(&'a Note, &'a Thread, &'a Elsewhere)> {
    let mut out = vec![];
    for home in notes.iter().filter(|n| n.id != note_id) {
        for e in home.elsewhere.iter().filter(|e| e.note == note_id && !e.visiting) {
            if let Some(t) = home.threads.iter().find(|t| t.id == e.thread) {
                out.push((home, t, e));
            }
        }
    }
    out
}

/// Open threads on `note`, its own and those visiting it: the sidebar's count.
pub fn open_here(notes: &[Note], note: &Note) -> usize {
    note.threads.iter().filter(|t| t.status == ThreadStatus::Open && !note.elsewhere_of(&t.id).is_some_and(|e| e.visiting)).count()
        + visiting(notes, &note.id).iter().filter(|(_, t, _)| t.status == ThreadStatus::Open).count()
}

/// Take the visiting ends out of `note` (threads and their entries).
fn strip_visitors(note: &mut Note) {
    let ids: HashSet<String> = note.elsewhere.iter().filter(|e| e.visiting).map(|e| e.thread.clone()).collect();
    if ids.is_empty() {
        return;
    }
    note.threads.retain(|t| !ids.contains(&t.id));
    note.elsewhere.retain(|e| !e.visiting);
}

/// `note` without its visiting ends: what is saved for it.
pub fn without_visitors(note: &Note) -> Note {
    let mut out = note.clone();
    strip_visitors(&mut out);
    out
}

/// Add to `note` (a note just loaded into the page) the far ends of threads from other
/// notes that quote it: each a clone of its home thread on this note's words, with an
/// [`Elsewhere`] marked `visiting` that points back at the home words. A far end whose
/// range no longer holds its words is re-found nearest where it was.
pub fn add_visitors(note: &mut Note, notes: &[Note]) {
    strip_visitors(note);
    let text = note.joined_text();
    for (home, t, e) in visiting(notes, &note.id) {
        if note.threads.iter().any(|x| x.id == t.id) {
            continue;
        }
        let range = refind(&text, e.start..e.end, &e.quote);
        let mut end = t.clone();
        end.quote = if range.is_empty() { e.quote.clone() } else { text[range.clone()].to_string() };
        end.start = range.start;
        end.end = range.end;
        end.side = e.side;
        end.tag_offset = e.tag_offset;
        note.threads.push(end);
        note.elsewhere.push(Elsewhere {
            thread: t.id.clone(),
            note: home.id.clone(),
            start: t.start,
            end: t.end,
            quote: t.quote.clone(),
            title: home.title.clone(),
            edited: home.updated_at,
            relation: e.relation,
            side: t.side,
            tag_offset: t.tag_offset,
            visiting: true,
            // whether the far end (this one) has its own side yet: the app reads it here
            placed: e.placed,
        });
    }
}

/// Send each visiting end of `note` home into `notes`: its words' range, quote, side and
/// offset into the home note's [`Elsewhere`], everything else (status, messages, drawing,
/// any field a later build adds) into the home thread. Returns `note` as it is saved (no
/// visiting ends, its own far ends' titles and dates refreshed) and the ids of the home
/// notes that changed.
pub fn send_home(note: &Note, notes: &mut [Note]) -> (Note, Vec<String>) {
    let mut changed: Vec<String> = vec![];
    for v in note.elsewhere.iter().filter(|v| v.visiting) {
        let Some(end) = note.threads.iter().find(|t| t.id == v.thread) else { continue };
        let Some(home) = notes.iter_mut().find(|n| n.id == v.note) else { continue };
        let Some(i) = home.threads.iter().position(|t| t.id == v.thread) else { continue };
        let Some(j) = home.elsewhere.iter().position(|e| e.thread == v.thread && e.note == note.id) else { continue };
        // the thread as the far end has it, on the home words
        let mut thread = end.clone();
        let here = &home.threads[i];
        (thread.start, thread.end, thread.quote, thread.side, thread.tag_offset) = (here.start, here.end, here.quote.clone(), here.side, here.tag_offset);
        let mut far = home.elsewhere[j].clone();
        let words = (far.start, far.end, far.quote.clone(), far.side, far.tag_offset);
        (far.start, far.end, far.quote, far.side, far.tag_offset) = (end.start, end.end, end.quote.clone(), end.side, end.tag_offset);
        let thread_changed = serde_json::to_value(&thread).ok() != serde_json::to_value(&home.threads[i]).ok();
        let words_changed = words != (far.start, far.end, far.quote.clone(), far.side, far.tag_offset) || far.title != note.title;
        // the app laid the far end out for the first time: its side is now its own, even
        // when it is the side it had by default
        let placed_now = v.placed && !far.placed;
        if thread_changed || words_changed || placed_now {
            // the title and date as last seen go home with any other change, so typing in a
            // note does not rewrite every note that links to it
            far.title = note.title.clone();
            far.edited = note.updated_at;
            far.placed = true;
            home.threads[i] = thread;
            home.elsewhere[j] = far;
            if !changed.contains(&home.id) {
                changed.push(home.id.clone());
            }
        }
    }
    let mut saved = note.clone();
    strip_visitors(&mut saved);
    for e in saved.elsewhere.iter_mut() {
        if let Some(other) = notes.iter().find(|n| n.id == e.note) {
            e.title = other.title.clone();
            e.edited = other.updated_at;
        }
    }
    (saved, changed)
}

/// [`send_home`] on the store: saves each home note that changed, and returns `note` as it
/// is to be saved, without its visiting ends.
pub fn save_visitors_home(note: &Note, store: &mut Store) -> Note {
    let (saved, changed) = send_home(note, &mut store.notes);
    for id in changed {
        if let Some(home) = store.note(&id) {
            store.save_note(home);
        }
    }
    saved
}

// ----- on the wire -----------------------------------------------------------------------

/// `{noteTitle, quote}` for a thread between notes, or null: `ctx.threads[i].elsewhere`.
pub fn summary_wire(note: &Note, thread: &str) -> Value {
    match note.elsewhere_of(thread) {
        Some(e) => json!({"noteTitle": e.title, "quote": e.quote}),
        None => Value::Null,
    }
}

/// The other end of `thread` (a thread of `note`, its own or visiting) as it is now, for a
/// `reply` (`ElsewhereEnd` in `brain/src/protocol.ts`); `None` when it is not a thread
/// between notes.
pub fn elsewhere_wire(notes: &[Note], note: &Note, thread: &str) -> Option<Value> {
    let e = note.elsewhere_of(thread)?;
    Some(match other_note(notes, e) {
        Some(other) => {
            // the words as that note holds them now (a visiting end's home is not open)
            let (range, quote) = if e.visiting {
                other.threads.iter().find(|t| t.id == e.thread).map_or((e.start..e.end, e.quote.clone()), |t| (t.start..t.end, t.quote.clone()))
            } else {
                (e.start..e.end, e.quote.clone())
            };
            json!({
                "noteTitle": other.title,
                "edited": date_words(other.updated_at),
                "quote": quote,
                "text": paragraph_at(other, &range, &quote),
                "relation": e.relation.wire(),
            })
        }
        None => json!({
            "noteTitle": e.title,
            "edited": "",
            "quote": e.quote,
            "text": Value::Null,
            "relation": e.relation.wire(),
            "gone": true,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Block, Message, Relation, Role, Side, ThoughtKind};

    fn note(id: &str, title: &str, body: &[&str]) -> Note {
        let mut n = Note::new(title, Mode::ThinksWithYou, None);
        n.id = id.into();
        n.blocks = std::iter::once(Block { kind: BlockKind::Title, text: title.into() })
            .chain(body.iter().map(|t| Block { kind: BlockKind::Paragraph, text: (*t).into() }))
            .collect();
        n
    }

    fn thread(id: &str, note: &Note, quote: &str) -> Thread {
        let text = note.joined_text();
        let start = text.find(quote).expect("the quote is in the note");
        Thread {
            id: id.into(),
            start,
            end: start + quote.len(),
            quote: quote.into(),
            kind: ThoughtKind::Question,
            messages: vec![Message::new(Role::Mind, "Which day is right?")],
            drawing: None,
            status: ThreadStatus::Open,
            side: Side::Right,
            mind_name: "Inkling".into(),
            tag_offset: (0., 0.),
            created_at: Utc::now(),
        }
    }

    const BIB: &str = "I'll pick up my bib on Saturday 14 March, after the shakeout run.";

    /// The notebook of the design: a calendar, the plan being written, a distractor that
    /// shares a month, a journal that waits to be asked, and the welcome guide.
    fn notebook() -> Vec<Note> {
        let calendar = note(
            "cal",
            "Race calendar",
            &["City 10k: Sunday 15 March, 9:00 start from the harbour.", "Pick up the race bib on Friday 13 March at the harbour office.", "Spring half marathon in Leeds on 26 April, if the knee holds."],
        );
        let plan = note("plan", "Race-week plan", &["Race week is about sleep and nothing new.", BIB]);
        let budget = note("budget", "March budget", &["The March budget for running gear is £120, mostly for new shoes.", "Rent and bills go out on the first of the month."]);
        let mut journal = note("journal", "Journal", &["Walked past the harbour office where you pick up the race bib; Friday 13 March is the day."]);
        journal.mode = Mode::WhenYouAsk;
        let mut guide = note("guide", "Start here", &["Pick up the race bib on Friday 13 March, the guide says, at the harbour office."]);
        guide.left_alone = vec![crate::store::hash_text(&guide.blocks[1].text)];
        vec![calendar, plan, budget, journal, guide]
    }

    fn passages(notes: &[Note], here: &str, target: &str) -> Vec<Passage> {
        let here = notes.iter().find(|n| n.id == here).unwrap().clone();
        Index::default().passages(notes, &[], &here, target)
    }

    #[std::prelude::v1::test]
    fn words_are_stemmed_and_common_ones_dropped() {
        let t = term_counts("I'll pick up my bibs on Saturday, running and stopped; the 9:00 start, 2026, 10k.");
        let mut keys: Vec<&str> = t.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["10k", "2026", "bib", "pick", "run", "saturday", "start", "stop"]);
        assert_eq!((stem("dates"), stem("cities"), stem("boxes"), stem("class"), stem("bus"), stem("météo")), ("date".into(), "city".into(), "box".into(), "class".into(), "bus".into(), "météo".into()));
    }

    #[std::prelude::v1::test]
    fn values_are_told_apart_by_kind() {
        use ValueKind::*;
        let k = |s: &str| {
            let mut v: Vec<String> = value_kinds(s).into_iter().map(|k| format!("{k:?}")).collect();
            v.sort();
            v.join(",")
        };
        assert_eq!(k("City 10k: Sunday 15 March, 9:00 start"), "Date,Quantity,Time");
        assert_eq!(k("The budget is £4,000 or 30 percent"), "Money,Percent");
        assert_eq!(k("We may go"), "");
        assert_eq!(k("on the 3rd of May"), "Date");
        assert_eq!(k("ship it 2026-03-15 at 5pm"), "Date,Time");
        assert_eq!(k("four days, 16 km and 3 runs a week"), "Quantity");
        assert!(value_kinds("nothing here at all").is_empty() && !value_kinds("in March 2026").contains(&Money));
    }

    #[std::prelude::v1::test]
    fn the_gate_finds_the_paragraph_that_shares_the_subject_and_nothing_else() {
        let notes = notebook();
        let found = passages(&notes, "plan", BIB);
        assert_eq!(found.len(), 1, "{found:?}");
        let p = &found[0];
        assert_eq!((p.note.as_str(), p.note_title.as_str(), p.block), ("cal", "Race calendar", 2));
        assert_eq!(p.text, "Pick up the race bib on Friday 13 March at the harbour office.");
        assert!(p.score >= MIN_SCORE);
        // never the note itself, a note that waits to be asked, the guide, or one shared word
        assert!(found.iter().all(|p| !["plan", "journal", "guide", "budget"].contains(&p.note.as_str())));
        assert!(passages(&notes, "plan", "The March budget review is next week, with the team.").is_empty(), "one uncommon word is not enough");
    }

    #[std::prelude::v1::test]
    fn code_short_lines_and_minds_words_are_never_passages() {
        let mut notes = notebook();
        let mut other = note("other", "Other", &["bib pickup harbour", "Inkling: pick up the race bib on Friday 13 March at the harbour office.", "x"]);
        other.blocks[3] = Block { kind: BlockKind::Code(crate::store::Lang::new("text")), text: "pick up the race bib on Friday 13 March at the harbour office".into() };
        notes.push(other);
        let found = passages(&notes, "plan", BIB);
        assert!(found.iter().all(|p| p.note != "other"), "{found:?}");
    }

    #[std::prelude::v1::test]
    fn at_most_three_passages_and_two_from_a_note() {
        let mut notes = vec![note("here", "Here", &["The harbour bib pickup on Friday 13 March needs the race number card."])];
        // words are uncommon only against a notebook that mostly talks about other things
        for n in 0..12 {
            notes.push(note(&format!("f{n}"), &format!("Filler {n}"), &["Tomatoes want more sun than the balcony gets in spring.", "The library closes early on public holidays this year."]));
        }
        for n in 0..3 {
            notes.push(note(
                &format!("n{n}"),
                &format!("N{n}"),
                &[
                    "Harbour bib pickup is on Friday 13 March; bring the race number card.",
                    "Bib pickup at the harbour, Friday 13 March: bring your race number card and ID.",
                    "Race number card and bib: the harbour office, Friday 13 March, pickup from noon.",
                ],
            ));
        }
        let found = passages(&notes, "here", &notes[0].blocks[1].text.clone());
        assert_eq!(found.len(), MAX_PASSAGES, "{found:?}");
        let mut per: HashMap<&str, usize> = HashMap::new();
        for p in &found {
            *per.entry(p.note.as_str()).or_default() += 1;
        }
        assert!(per.values().all(|c| *c <= PER_NOTE));
        assert!(found.windows(2).all(|w| w[0].score >= w[1].score), "best first");
    }

    #[std::prelude::v1::test]
    fn a_pair_already_joined_is_never_offered_again() {
        let notes = notebook();
        let mut here = notes.iter().find(|n| n.id == "plan").unwrap().clone();
        let t = thread("t1", &here, "Saturday 14 March");
        here.threads.push(t);
        here.elsewhere.push(Elsewhere { thread: "t1".into(), note: "cal".into(), quote: "Friday 13 March".into(), ..Default::default() });
        for status in [ThreadStatus::Open, ThreadStatus::Resolved, ThreadStatus::Dismissed] {
            here.threads[0].status = status;
            assert!(Index::default().passages(&notes, &[], &here, BIB).is_empty(), "{status:?}");
        }
        // a visiting end joins the pair from the other side too
        let mut cal = notes.iter().find(|n| n.id == "cal").unwrap().clone();
        cal.elsewhere.push(Elsewhere { thread: "t2".into(), note: "plan".into(), quote: "Saturday 14 March".into(), visiting: true, ..Default::default() });
        let back = Index::default().passages(&notes, &[], &cal, "Pick up the race bib on Friday 13 March at the harbour office.");
        assert!(back.iter().all(|p| !(p.note == "plan" && p.text.contains("Saturday 14 March"))), "{back:?}");
    }

    #[std::prelude::v1::test]
    fn a_note_with_three_open_threads_between_notes_gets_no_more() {
        let notes = notebook();
        let mut here = notes.iter().find(|n| n.id == "plan").unwrap().clone();
        for i in 0..MAX_OPEN {
            let t = thread(&format!("t{i}"), &here, "Race week");
            here.threads.push(t);
            here.elsewhere.push(Elsewhere { thread: format!("t{i}"), note: "budget".into(), quote: "first".into(), ..Default::default() });
        }
        assert_eq!(open_between(&here), MAX_OPEN);
        assert!(Index::default().passages(&notes, &[], &here, BIB).is_empty());
        here.threads[0].status = ThreadStatus::Resolved;
        assert_eq!(Index::default().passages(&notes, &[], &here, BIB).len(), 1);
    }

    #[std::prelude::v1::test]
    fn the_index_follows_saves_and_deletes() {
        let mut notes = notebook();
        let here = notes.iter().find(|n| n.id == "plan").unwrap().clone();
        let mut index = Index::default();
        assert_eq!(index.passages(&notes, &[], &here, BIB).len(), 1);
        let units = index.units;
        // the calendar is rewritten: its old paragraph is forgotten, the new one found
        let cal = notes.iter_mut().find(|n| n.id == "cal").unwrap();
        cal.blocks[2].text = "Bib collection moved: pick up race bibs at the harbour office on Friday 13 March only.".into();
        cal.updated_at = cal.updated_at + chrono::Duration::seconds(5);
        let found = index.passages(&notes, &[], &here, BIB);
        assert_eq!(found.len(), 1);
        assert!(found[0].text.starts_with("Bib collection moved"), "{found:?}");
        assert_eq!(index.units, units);
        // deleted: gone from the index and its counts
        notes.retain(|n| n.id != "cal");
        assert!(index.passages(&notes, &[], &here, BIB).is_empty());
        assert!(!index.notes.contains_key("cal") && index.df.get("harbour").copied().unwrap_or(0) < 2);
        // a note switched to wait until asked leaves the index too
        let mut notes = notebook();
        notes.iter_mut().find(|n| n.id == "cal").unwrap().mode = Mode::WhenYouAsk;
        assert!(index.passages(&notes, &[], &here, BIB).is_empty());
    }

    #[std::prelude::v1::test]
    fn long_paragraphs_are_cut_around_the_shared_words_on_char_boundaries() {
        let long = format!("{} the harbour bib is collected on Friday {}", "Ünïcödé ".repeat(120), "é".repeat(700));
        let cut = cut_around(&long, &["bib"], PASSAGE_CHARS);
        assert!(cut.chars().count() <= PASSAGE_CHARS, "{}", cut.chars().count());
        assert!(cut.starts_with('\u{2026}') && cut.ends_with('\u{2026}') && cut.contains("harbour bib"));
        assert_eq!(cut_around("short", &["x"], PASSAGE_CHARS), "short");
    }

    #[std::prelude::v1::test]
    fn quotes_are_located_in_their_block() {
        let notes = notebook();
        let cal = &notes[0];
        let r = locate_in_block(cal, 2, "\u{2026}Friday 13 March").unwrap();
        assert_eq!(&cal.joined_text()[r], "Friday 13 March");
        assert!(locate_in_block(cal, 1, "Friday 13 March").is_none());
        assert!(locate_in_block(cal, 9, "x").is_none());
        // the far end of a new thread: in its block, else nearest it, else the whole block
        assert_eq!(locate_other(cal, 2, "Friday 13 March").unwrap().1, "Friday 13 March");
        assert_eq!(locate_other(cal, 1, "Friday 13 March").unwrap().1, "Friday 13 March");
        assert_eq!(locate_other(cal, 3, "gone words").unwrap().1, "Spring half marathon in Leeds on 26 April, if the knee holds.");
        assert!(locate_other(cal, 9, "x").is_none());
    }

    /// A plan whose thread `b1` joins "Saturday 14 March" to the calendar's "Friday 13 March".
    fn linked() -> Vec<Note> {
        let mut notes = notebook();
        let cal = notes[0].clone();
        let plan = &mut notes[1];
        let t = thread("b1", plan, "Saturday 14 March");
        plan.threads.push(t);
        let far = cal.joined_text().find("Friday 13 March").unwrap();
        plan.elsewhere.push(Elsewhere {
            thread: "b1".into(),
            note: "cal".into(),
            start: far,
            end: far + "Friday 13 March".len(),
            quote: "Friday 13 March".into(),
            title: "Race calendar".into(),
            relation: Relation::Contradicts,
            side: Side::Left,
            tag_offset: (4., 2.),
            ..Default::default()
        });
        notes
    }

    #[std::prelude::v1::test]
    fn a_visiting_end_is_the_thread_on_the_other_words_and_goes_home_on_save() {
        let mut notes = linked();
        let mut cal = notes[0].clone();
        add_visitors(&mut cal, &notes);
        add_visitors(&mut cal, &notes); // again: nothing doubles
        assert_eq!((cal.threads.len(), cal.elsewhere.len()), (1, 1));
        let end = &cal.threads[0];
        assert_eq!((end.id.as_str(), end.quote.as_str(), end.side, end.tag_offset), ("b1", "Friday 13 March", Side::Left, (4., 2.)));
        assert_eq!(&cal.joined_text()[end.start..end.end], "Friday 13 March");
        assert_eq!(end.messages.len(), 1);
        let back = &cal.elsewhere[0];
        assert!(back.visiting && !back.placed, "not laid out yet: the app picks its side");
        assert_eq!((back.note.as_str(), back.quote.as_str(), back.title.as_str()), ("plan", "Saturday 14 March", "Race-week plan"));
        assert_eq!(open_here(&notes, &notes[0]), 1, "the sidebar counts the visiting end");

        // nothing changed: saving the calendar writes no home note and keeps no visitor
        let (saved, changed) = send_home(&cal, &mut notes);
        assert!(changed.is_empty(), "{changed:?}");
        assert!(saved.threads.is_empty() && saved.elsewhere.is_empty());
        assert!(serde_json::to_value(&saved).unwrap().get("elsewhere").is_none());

        // edits at the far end: words moved, a reply, tied off, dragged
        cal.blocks[2].text = format!("Note: {}", cal.blocks[2].text);
        let shift = "Note: ".len();
        let end = &mut cal.threads[0];
        end.start += shift;
        end.end += shift;
        end.messages.push(Message::new(Role::You, "The calendar is right."));
        end.status = ThreadStatus::Resolved;
        end.tag_offset = (9., -3.);
        let (saved, changed) = send_home(&cal, &mut notes);
        assert_eq!(changed, ["plan"]);
        assert!(saved.threads.is_empty());
        let plan = notes.iter().find(|n| n.id == "plan").unwrap();
        let home = &plan.threads[0];
        assert_eq!((home.status, home.messages.len(), home.quote.as_str(), home.side, home.tag_offset), (ThreadStatus::Resolved, 2, "Saturday 14 March", Side::Right, (0., 0.)));
        let far = plan.elsewhere_of("b1").unwrap();
        assert_eq!((far.start, far.quote.as_str(), far.tag_offset, far.side), (cal.joined_text().find("Friday 13 March").unwrap(), "Friday 13 March", (9., -3.), Side::Left));
        assert!(far.placed, "written home: its side and offset are its own now");
        let mut again = notes[0].clone();
        add_visitors(&mut again, &notes);
        assert!(again.elsewhere[0].placed);
    }

    #[std::prelude::v1::test]
    fn a_far_end_laid_out_on_its_default_side_still_goes_home_placed() {
        let mut notes = linked();
        notes[1].elsewhere[0].side = Side::Right;
        notes[1].elsewhere[0].tag_offset = (0., 0.);
        let mut cal = notes[0].clone();
        add_visitors(&mut cal, &notes);
        // shown and saved without being laid out: nothing to write
        assert!(send_home(&cal, &mut notes).1.is_empty());
        assert!(!notes[1].elsewhere[0].placed);
        // the app picks Right, the side it already had, and says so
        cal.elsewhere[0].placed = true;
        let (_, changed) = send_home(&cal, &mut notes);
        assert_eq!(changed, ["plan"]);
        assert!(notes[1].elsewhere[0].placed && notes[1].elsewhere[0].side == Side::Right);
    }

    #[std::prelude::v1::test]
    fn a_far_end_whose_words_moved_is_found_again_and_a_deleted_home_is_skipped() {
        let mut notes = linked();
        notes[0].blocks.insert(1, Block { kind: BlockKind::Paragraph, text: "A new first line.".into() });
        let mut cal = notes[0].clone();
        add_visitors(&mut cal, &notes);
        assert_eq!(&cal.joined_text()[cal.threads[0].start..cal.threads[0].end], "Friday 13 March");
        notes.retain(|n| n.id != "plan");
        let (saved, changed) = send_home(&cal, &mut notes);
        assert!(changed.is_empty() && saved.threads.is_empty());
        let mut again = notes[0].clone();
        add_visitors(&mut again, &notes);
        assert!(again.threads.is_empty(), "the home note was deleted: no visiting end");
    }

    #[std::prelude::v1::test]
    fn the_other_end_goes_on_the_wire_as_it_is_now() {
        let notes = linked();
        let plan = &notes[1];
        let w = elsewhere_wire(&notes, plan, "b1").unwrap();
        assert_eq!(w["noteTitle"], "Race calendar");
        assert_eq!(w["quote"], "Friday 13 March");
        assert_eq!(w["text"], "Pick up the race bib on Friday 13 March at the harbour office.");
        assert_eq!(w["relation"], "contradicts");
        assert!(w.get("gone").is_none());
        assert_eq!(summary_wire(plan, "b1"), json!({"noteTitle": "Race calendar", "quote": "Friday 13 March"}));
        assert_eq!(summary_wire(plan, "nope"), Value::Null);
        assert!(elsewhere_wire(&notes, plan, "nope").is_none());
        // from the far end: the home words
        let mut cal = notes[0].clone();
        add_visitors(&mut cal, &notes);
        let w = elsewhere_wire(&notes, &cal, "b1").unwrap();
        assert_eq!((w["noteTitle"].as_str(), w["quote"].as_str(), w["text"].as_str()), (Some("Race-week plan"), Some("Saturday 14 March"), Some(BIB)));
        // the other note is gone
        let without: Vec<Note> = notes.iter().filter(|n| n.id != "cal").cloned().collect();
        let w = elsewhere_wire(&without, plan, "b1").unwrap();
        assert_eq!((w["gone"].as_bool(), w["text"].is_null(), w["quote"].as_str()), (Some(true), true, Some("Friday 13 March")));
    }

    #[std::prelude::v1::test]
    fn passages_go_on_the_wire_with_titles_and_dates() {
        let notes = notebook();
        let found = passages(&notes, "plan", BIB);
        let w = passages_wire(&found);
        assert_eq!(w[0]["note"], "cal");
        assert_eq!(w[0]["noteTitle"], "Race calendar");
        assert!(w[0]["written"].as_str().unwrap().split(' ').count() == 4, "{}", w[0]["written"]);
        assert!(w[0]["heading"].is_null());
        assert_eq!(w[0]["text"], "Pick up the race bib on Friday 13 March at the harbour office.");
    }
}

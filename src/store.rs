//! Notes, minds and settings, stored as plain JSON files in
//! `~/Library/Application Support/Inkling/`. One file per note keeps writes small and
//! the data easy to read, back up or sync; a personal notebook fits in memory, so
//! search runs over the loaded notes.

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// What a line of a note is. Most kinds are a word in the JSON (`"paragraph"`); the
/// three that carry a value are an object (`{"check": true}`, `{"callout": "tip"}`,
/// `{"code": "rust"}`), so notes saved before them still load as they are.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Title,
    #[default]
    Paragraph,
    H1,
    H2,
    H3,
    Bullet,
    Numbered,
    Quote,
    /// A checklist item, `true` once it is done.
    Check(bool),
    /// A line of a callout. Lines of one kind in a row share one patch.
    Callout(Callout),
    /// A line of code. Lines in one language in a row make one block.
    Code(Lang),
}

impl BlockKind {
    pub fn is_list(self) -> bool {
        matches!(self, BlockKind::Bullet | BlockKind::Numbered | BlockKind::Check(_))
    }

    pub fn is_code(self) -> bool {
        matches!(self, BlockKind::Code(_))
    }

    /// Lines of these kinds in a row make one block: a callout's patch, a code block.
    pub fn joins(self, next: BlockKind) -> bool {
        match (self, next) {
            (BlockKind::Callout(a), BlockKind::Callout(b)) => a == b,
            (BlockKind::Code(a), BlockKind::Code(b)) => a == b,
            _ => false,
        }
    }

    /// The same kind, as the selection menu sees it: a done item is still a checklist
    /// item, and code is code in any language.
    pub fn same_kind(self, other: BlockKind) -> bool {
        match (self, other) {
            (BlockKind::Check(_), BlockKind::Check(_)) | (BlockKind::Code(_), BlockKind::Code(_)) => true,
            (a, b) => a == b,
        }
    }
}

/// A block kind in a word or two, for the log and the script's dump: `paragraph`,
/// `check` or `check:done`, `callout:tip`, `code:rust` (`code` with no language).
pub fn kind_name(kind: BlockKind) -> String {
    match kind {
        BlockKind::Check(done) => if done { "check:done" } else { "check" }.into(),
        BlockKind::Callout(c) => format!("callout:{}", c.name()),
        BlockKind::Code(lang) if lang.is_empty() => "code".into(),
        BlockKind::Code(lang) => format!("code:{}", lang.as_str()),
        other => serde_json::to_value(other).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default(),
    }
}

/// The three kinds of callout. Each is sewn in its own thread: indigo for a note, ochre
/// for a warning, sage for a tip. Never madder, which is Inkling's.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Callout {
    #[default]
    Note,
    Warning,
    Tip,
}

impl Callout {
    pub const ALL: [Callout; 3] = [Callout::Note, Callout::Warning, Callout::Tip];

    /// The name on the patch and in the selection menu.
    pub fn label(self) -> &'static str {
        match self {
            Callout::Note => "Note",
            Callout::Warning => "Warning",
            Callout::Tip => "Tip",
        }
    }

    /// As Markdown writes it: `> [!note]`.
    pub fn name(self) -> &'static str {
        match self {
            Callout::Note => "note",
            Callout::Warning => "warning",
            Callout::Tip => "tip",
        }
    }

    /// One of Obsidian's callout types (any case, its aliases too) as one of the three:
    /// what informs is a note, what cautions is a warning, what helps or is done is a
    /// tip. `None` for a type Obsidian does not have.
    pub fn from_obsidian(kind: &str) -> Option<Callout> {
        Some(match kind.trim().to_lowercase().as_str() {
            "note" | "abstract" | "summary" | "tldr" | "info" | "todo" | "question" | "help" | "faq" | "example" | "quote" | "cite" => Callout::Note,
            "warning" | "caution" | "attention" | "failure" | "fail" | "missing" | "danger" | "error" | "bug" => Callout::Warning,
            "tip" | "hint" | "important" | "success" | "check" | "done" => Callout::Tip,
            _ => return None,
        })
    }
}

/// A code block's language as written after its fence (```` ```rust ````): its first
/// word, lower case, ASCII letters, digits and `+#._-`, at most [`Lang::MAX`] bytes.
/// Kept inline so a block kind stays `Copy`. Empty when the fence names none.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Lang {
    len: u8,
    bytes: [u8; Lang::MAX],
}

impl Lang {
    pub const MAX: usize = 15;

    pub fn new(written: &str) -> Lang {
        let mut lang = Lang::default();
        let word = written.split_whitespace().next().unwrap_or("");
        for c in word.chars().take_while(|c| c.is_ascii_alphanumeric() || "+#._-".contains(*c)).take(Lang::MAX) {
            lang.bytes[lang.len as usize] = c.to_ascii_lowercase() as u8;
            lang.len += 1;
        }
        lang
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl std::fmt::Debug for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl Serialize for Lang {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Lang {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Lang::new(&String::deserialize(d)?))
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    Color(u32),
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct MarkSpan {
    pub start: usize,
    pub end: usize,
    pub mark: Mark,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Block {
    /// A kind this build does not know (one a newer build wrote) reads as a paragraph,
    /// so the note still opens with its words.
    #[serde(default, deserialize_with = "kind_or_paragraph")]
    pub kind: BlockKind,
    #[serde(default)]
    pub text: String,
}

fn kind_or_paragraph<'de, D: serde::Deserializer<'de>>(d: D) -> Result<BlockKind, D::Error> {
    Ok(serde_json::from_value(Value::deserialize(d)?).unwrap_or_default())
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    ThinksWithYou,
    WhenYouAsk,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThoughtKind {
    #[default]
    Question,
    Correction,
    Sketch,
    Ask,
    Answer,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    #[default]
    Open,
    Resolved,
    Dismissed,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Left,
    #[default]
    Right,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Mind,
    You,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub role: Role,
    pub text: String,
    #[serde(default)]
    pub at: DateTime<Utc>,
    /// How a correction shows it is right (`src/working.rs`). Only the brain's checked
    /// answer fills it; a check this build cannot read is left out, not the message.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "check_or_none")]
    pub check: Option<Check>,
}

impl Message {
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Self { role, text: text.into(), at: Utc::now(), check: None }
    }
}

/// Show its working: what a correction carries so you can check it yourself, a sum made
/// from your own numbers or your own words from elsewhere in the note.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Check {
    /// The working, line by line.
    pub sum: Vec<SumLine>,
    /// The numbers the sum took from your words, and those words.
    pub givens: Vec<Given>,
    /// Your words, from elsewhere in the note, that the corrected words contradict.
    pub words: Vec<Passage>,
    /// The mind agreed the check was wrong and took the correction back.
    pub taken_back: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct SumLine {
    pub sum: String,
    pub is: String,
    /// `is` is the result rounded, so the tag shows ≈ rather than =.
    pub rounded: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Given {
    pub value: String,
    pub words: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Passage {
    pub words: String,
    /// Another note's id when the words are there; `None` for this note.
    pub note: Option<String>,
}

fn check_or_none<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Check>, D::Error> {
    Ok(serde_json::from_value::<Option<Check>>(Value::deserialize(d)?).ok().flatten())
}

/// Fields with a sensible empty value default when a file lacks them, so a note
/// written by hand or by another build still opens. A thread keeps its id and quote.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Thread {
    pub id: String,
    /// Byte range of the quote in the note's joined text (blocks joined by '\n').
    #[serde(default)]
    pub start: usize,
    #[serde(default)]
    pub end: usize,
    pub quote: String,
    #[serde(default)]
    pub kind: ThoughtKind,
    #[serde(default)]
    pub messages: Vec<Message>,
    /// A hand-drawn SVG sketch, when the thread has one.
    #[serde(default)]
    pub drawing: Option<String>,
    #[serde(default)]
    pub status: ThreadStatus,
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub mind_name: String,
    /// Where you dragged its tag, relative to where the margin would put it.
    #[serde(default)]
    pub tag_offset: (f32, f32),
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    /// Why you let it stand, in your words: one line, at most [`REASON_MAX`] characters,
    /// from the tag's box when you chose Let stand, or written later on the back of the
    /// note. Only a mind's point that was let stand has one; it goes when the thread opens
    /// again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When it was tied off or dismissed; cleared when it opens again. Threads that ended
    /// before this field existed have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// The words the thread was opened on, kept the first time its words change (you edit
    /// them; Fix the words reads them again). `quote` is always the words now. None while
    /// the words never changed, and for threads whose words changed before this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_quote: Option<String>,
}

/// The most characters a reason for letting a point stand keeps.
pub const REASON_MAX: usize = 200;

/// How a thread ended, as the back of the note and the minds see it. Worked out from the
/// status and the kind, never stored, so it can grow without breaking older builds (a
/// status value an older build does not know would drop the whole note there).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending {
    Open,
    TiedOff,
    LetStand,
    Snipped,
}

impl Ending {
    /// The ending as the back of the note writes it.
    pub fn label(self) -> &'static str {
        match self {
            Ending::Open => "Still open",
            Ending::TiedOff => "Tied off",
            Ending::LetStand => "Let stand",
            Ending::Snipped => "Snipped",
        }
    }
}

impl Thread {
    /// A point a mind raised on its own: a question or a correction (reading along, Find a
    /// loose thread) that the mind has said. Only these are let stand; dismissing anything
    /// else (a loose thread still being looked for included) snips it.
    pub fn is_point(&self) -> bool {
        matches!(self.kind, ThoughtKind::Question | ThoughtKind::Correction) && self.messages.iter().any(|m| m.role == Role::Mind)
    }

    /// Open is still open, Resolved is tied off, and Dismissed is let stand on a point and
    /// snipped on anything else.
    pub fn ending(&self) -> Ending {
        match self.status {
            ThreadStatus::Open => Ending::Open,
            ThreadStatus::Resolved => Ending::TiedOff,
            ThreadStatus::Dismissed if self.is_point() => Ending::LetStand,
            ThreadStatus::Dismissed => Ending::Snipped,
        }
    }

    /// Change the status: tied off or dismissed stamps when it ended; open again clears
    /// that and the reason. Setting the status it already has changes nothing.
    pub fn set_status(&mut self, status: ThreadStatus) {
        if status == self.status {
            return;
        }
        self.status = status;
        if status == ThreadStatus::Open {
            self.ended_at = None;
            self.reason = None;
        } else {
            self.ended_at = Some(Utc::now());
        }
    }

    /// The words the thread is on now. The first time they change, the words it was
    /// opened on are kept in `first_quote`.
    pub fn set_quote(&mut self, words: &str) {
        if words == self.quote {
            return;
        }
        if self.first_quote.is_none() {
            self.first_quote = Some(std::mem::take(&mut self.quote));
        }
        self.quote = words.to_string();
    }

    /// The words it was opened on.
    pub fn words_before(&self) -> &str {
        self.first_quote.as_deref().unwrap_or(&self.quote)
    }

    /// Your reason as one line: line breaks and runs of spaces fold to single spaces, and
    /// it is cut to [`REASON_MAX`] characters. Nothing but spaces clears it.
    pub fn set_reason(&mut self, text: &str) {
        let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        self.reason = (!line.is_empty()).then(|| line.chars().take(REASON_MAX).collect());
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: Role,
    pub text: String,
    #[serde(default)]
    pub at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Note {
    /// Also the file name (`notes/<id>.json`); loading takes it from the file name.
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub marks: Vec<MarkSpan>,
    #[serde(default)]
    pub brief: Option<String>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default = "default_mind_id")]
    pub mind_id: String,
    #[serde(default)]
    pub threads: Vec<Thread>,
    #[serde(default)]
    pub chat: Vec<ChatMessage>,
    /// Hashes of paragraphs already read along, so reopening a note stays quiet.
    #[serde(default)]
    pub read_hashes: Vec<u64>,
    /// Paragraphs Inkling leaves alone without having read them: text that came with
    /// the app, such as the welcome guide. They get no read mark in the gutter.
    #[serde(default)]
    pub left_alone: Vec<u64>,
    /// The first note of a new notebook: the first sentence written in it is always
    /// asked about, so the first question Inkling asks is about your own words.
    #[serde(default)]
    pub first_run: bool,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub updated_at: DateTime<Utc>,
}

fn default_mind_id() -> String {
    DEFAULT_MIND.into()
}

impl Note {
    pub fn new(title: &str, mode: Mode, folder: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: new_id(),
            title: title.into(),
            folder,
            blocks: vec![
                Block { kind: BlockKind::Title, text: title.into() },
                Block { kind: BlockKind::Paragraph, text: String::new() },
            ],
            marks: vec![],
            brief: None,
            mode,
            mind_id: DEFAULT_MIND.into(),
            threads: vec![],
            chat: vec![],
            read_hashes: vec![],
            left_alone: vec![],
            first_run: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// The note's text as one string, blocks joined by newlines.
    pub fn joined_text(&self) -> String {
        self.blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    /// Plain text for the model and for search: the body as light Markdown.
    pub fn plain_text(&self) -> String {
        plain_text_of(&self.blocks)
    }

    pub fn open_threads(&self) -> usize {
        self.threads.iter().filter(|t| t.status == ThreadStatus::Open).count()
    }

    pub fn visible_threads(&self) -> usize {
        self.threads.iter().filter(|t| t.status != ThreadStatus::Dismissed).count()
    }
}

/// The body of a note (everything after the title) as light Markdown.
pub fn plain_text_of(blocks: &[Block]) -> String {
    markdown_blocks(blocks, 0, |i| blocks[i].text.clone(), |_| None).trim_end().to_string()
}

/// The body (blocks after the title) as Markdown, headings `heading_shift` levels
/// deeper. `text_of(i)` is block `i`'s words as they should be written (with their
/// marks, for an export). `after(i)` may add text under block `i`, such as its threads.
/// Items of one list sit on consecutive lines; whatever follows a list starts after a
/// blank line, because Markdown reads a line right under an item as more of that item.
/// Checklist items are `- [ ]` and `- [x]` items of a dash list. A callout is Obsidian's
/// `> [!note]` with a line for each of its lines, and code is fenced with its language,
/// as written (no marks); what goes under any of their lines comes after the whole block.
pub(crate) fn markdown_blocks(
    blocks: &[Block],
    heading_shift: usize,
    mut text_of: impl FnMut(usize) -> String,
    mut after: impl FnMut(usize) -> Option<String>,
) -> String {
    // bullets and checklist items are both dash lists; numbered lists are their own
    let family = |k: BlockKind| match k {
        BlockKind::Bullet | BlockKind::Check(_) => Some(0),
        BlockKind::Numbered => Some(1),
        _ => None,
    };
    let mut out = String::new();
    let mut number = 0;
    let mut list: Option<u8> = None;
    let mut i = 1;
    while i < blocks.len() {
        let b = &blocks[i];
        if matches!(b.kind, BlockKind::Callout(_) | BlockKind::Code(_)) {
            let mut end = i + 1;
            while end < blocks.len() && b.kind.joins(blocks[end].kind) {
                end += 1;
            }
            let shown: Vec<usize> = (i..end).filter(|&j| !blocks[j].text.trim().is_empty()).collect();
            let (Some(&first), Some(&last)) = (shown.first(), shown.last()) else {
                i = end;
                continue;
            };
            if list.take().is_some() {
                out.push('\n');
            }
            number = 0;
            match b.kind {
                BlockKind::Callout(kind) => {
                    out.push_str(&format!("> [!{}]\n", kind.name()));
                    for (n, &j) in shown.iter().enumerate() {
                        if n > 0 {
                            out.push_str(">\n");
                        }
                        out.push_str(&format!("> {}\n", text_of(j)));
                    }
                }
                BlockKind::Code(lang) => {
                    // a fence longer than any run of backticks inside the code
                    let longest = blocks[first..=last].iter().map(|c| longest_run(&c.text, '`')).max().unwrap_or(0);
                    let fence = "`".repeat(longest.max(2) + 1);
                    out.push_str(&format!("{fence}{}\n", lang.as_str()));
                    for c in &blocks[first..=last] {
                        out.push_str(&c.text);
                        out.push('\n');
                    }
                    out.push_str(&fence);
                    out.push('\n');
                }
                _ => {}
            }
            out.push('\n');
            for j in i..end {
                if let Some(extra) = after(j) {
                    out.push_str(&extra);
                }
            }
            i = end;
            continue;
        }
        let at = i;
        i += 1;
        if b.text.trim().is_empty() {
            continue;
        }
        let text = &text_of(at);
        if list.is_some_and(|f| Some(f) != family(b.kind)) {
            out.push('\n');
        }
        if b.kind != BlockKind::Numbered {
            number = 0;
        }
        let heading = |level: usize| "#".repeat(level + heading_shift);
        match b.kind {
            BlockKind::H1 => out.push_str(&format!("{} {text}\n\n", heading(1))),
            BlockKind::H2 => out.push_str(&format!("{} {text}\n\n", heading(2))),
            BlockKind::H3 => out.push_str(&format!("{} {text}\n\n", heading(3))),
            BlockKind::Bullet => out.push_str(&format!("- {text}\n")),
            BlockKind::Check(done) => out.push_str(&format!("- [{}] {text}\n", if done { 'x' } else { ' ' })),
            BlockKind::Numbered => {
                number += 1;
                out.push_str(&format!("{number}. {text}\n"));
            }
            BlockKind::Quote => out.push_str(&format!("> {text}\n\n")),
            BlockKind::Title | BlockKind::Paragraph | BlockKind::Callout(_) | BlockKind::Code(_) => out.push_str(&format!("{text}\n\n")),
        }
        list = family(b.kind);
        if let Some(extra) = after(at) {
            if list.take().is_some() {
                out.push('\n');
            }
            out.push_str(&extra);
        }
    }
    out
}

/// The longest run of `c` in `s`.
fn longest_run(s: &str, c: char) -> usize {
    s.split(|x| x != c).map(|run| run.len()).max().unwrap_or(0)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Folder {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Voice {
    #[default]
    Plain,
    Warm,
    Blunt,
    Riddle,
}

impl Voice {
    pub const ALL: [Voice; 4] = [Voice::Plain, Voice::Warm, Voice::Blunt, Voice::Riddle];
    pub fn label(self) -> &'static str {
        match self {
            Voice::Plain => "Plain",
            Voice::Warm => "Warm",
            Voice::Blunt => "Blunt",
            Voice::Riddle => "Socratic",
        }
    }
    pub fn wire(self) -> &'static str {
        match self {
            Voice::Plain => "plain",
            Voice::Warm => "warm",
            Voice::Blunt => "blunt",
            Voice::Riddle => "riddle",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Rarely,
    #[default]
    WhenItMatters,
    Often,
}

impl Frequency {
    pub const ALL: [Frequency; 3] = [Frequency::Rarely, Frequency::WhenItMatters, Frequency::Often];
    pub fn label(self) -> &'static str {
        match self {
            Frequency::Rarely => "Rarely",
            Frequency::WhenItMatters => "When it matters",
            Frequency::Often => "Often",
        }
    }
    pub fn wire(self) -> &'static str {
        match self {
            Frequency::Rarely => "rarely",
            Frequency::WhenItMatters => "when_it_matters",
            Frequency::Often => "often",
        }
    }
}

pub const DEFAULT_MIND: &str = "inkling";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Mind {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub how_it_thinks: String,
    #[serde(default)]
    pub voice: Voice,
    #[serde(default)]
    pub frequency: Frequency,
}

impl Mind {
    pub fn inkling() -> Self {
        Self {
            id: DEFAULT_MIND.into(),
            name: "Inkling".into(),
            description: "Thinks alongside you, and speaks up when something is worth a second look."
                .into(),
            how_it_thinks: String::new(),
            voice: Voice::Plain,
            frequency: Frequency::WhenItMatters,
        }
    }
}

/// Starting points for a new mind. Wording is Inkling's own.
pub fn mind_templates() -> Vec<Mind> {
    let t = |name: &str, description: &str, how: &str, voice: Voice| Mind {
        id: String::new(),
        name: name.into(),
        description: description.into(),
        how_it_thinks: how.into(),
        voice,
        frequency: Frequency::WhenItMatters,
    };
    vec![
        t(
            "Staff engineer",
            "Reads designs as if it will be on call for them.",
            "Read like a senior engineer reviewing a design doc. Probe scale, failure modes, consistency, operational cost and the edge cases. Push on hand-waving such as \"we'll just cache it\" or \"it should be fast\". Leave naming and style alone.",
            Voice::Blunt,
        ),
        t(
            "Editor",
            "Wants every sentence to earn its place.",
            "Read like a sharp editor. Ask who this is for and what it is trying to say. Point at vague claims, buried points and sentences that say nothing. Never rewrite their words; ask the question that makes them rewrite.",
            Voice::Plain,
        ),
        t(
            "Skeptic",
            "Asks for the evidence.",
            "Question claims that are not supported. Ask what the evidence is, what would prove it wrong, and what else could explain it. Test confident words like always, never and obviously.",
            Voice::Plain,
        ),
        t(
            "Socratic tutor",
            "Never gives the answer; asks the question that leads to it.",
            "The writer is learning. Find the misunderstanding underneath what they wrote and ask the smallest question that would let them discover it. Do not state the answer. Say so when they get it right.",
            Voice::Warm,
        ),
        t(
            "Investor",
            "Thinks about the market, the money and the moat.",
            "Read like a skeptical early-stage investor. Ask who the customer is, why now, how it makes money, what competitors do, and what would have to be true for this to work. Ignore implementation details.",
            Voice::Blunt,
        ),
        t("Blank", "", "", Voice::Plain),
    ]
}

macro_rules! simple_enum {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? } default $default:ident) => {
        #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
        impl Default for $name { fn default() -> Self { $name::$default } }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];
            pub fn label(self) -> &'static str { match self { $($name::$variant => $label),+ } }
        }
    };
}

simple_enum!(Pace { Regular => "Regular", Slow => "Slow" } default Regular);
simple_enum!(Placement { Both => "Both margins", Right => "Right margin", Marks => "Marks only" } default Both);
simple_enum!(Appearance { System => "System", Light => "Light", Dark => "Dark" } default System);
simple_enum!(TextSize { Small => "Small", Default => "Default", Large => "Large" } default Default);
simple_enum!(Lettering { Own => "Inkling’s own", LikeNotes => "Like your notes" } default Own);
// A mind's thread. The variant names are what settings files store; the labels are
// the thread colours of Loose Threads (madder is Inkling's own).
simple_enum!(InkColor {
    Ink => "Madder", Blue => "Indigo", Green => "Sage", Red => "Rose",
    Purple => "Plum", Brown => "Walnut", Orange => "Ochre"
} default Ink);

/// A model Claude Code offers, as the Agent SDK's `supportedModels()` reports it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct ModelChoice {
    /// What to pass as the model: an alias ("opus"), a full id, or "default".
    pub value: String,
    pub name: String,
    pub description: String,
    /// The full id an alias stands for, when the SDK says.
    pub resolved: Option<String>,
    /// The effort levels it takes: None when the SDK did not say, empty when it takes none.
    pub efforts: Option<Vec<String>>,
}

impl ModelChoice {
    /// A specific version ("claude-opus-4-8") rather than a name that moves with releases.
    pub fn is_pinned(&self) -> bool {
        self.value.starts_with("claude-")
    }
}

/// Which row the saved `model` setting picks. Empty is Claude Code's default, which the SDK
/// lists as "default". Otherwise its own row, else a row that stands for it (an alias that
/// resolves to it, the default last), so a saved full id shows as its alias.
pub fn chosen_model(models: &[ModelChoice], model: &str) -> Option<usize> {
    let model = model.trim();
    if model.is_empty() {
        return models.iter().position(|m| m.value == "default");
    }
    let stands_for = |m: &ModelChoice| m.resolved.as_deref() == Some(model);
    models
        .iter()
        .position(|m| m.value == model)
        .or_else(|| models.iter().position(|m| m.value != "default" && stands_for(m)))
        .or_else(|| models.iter().position(stands_for))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub default_mode: Mode,
    pub ink_from_pointer: bool,
    pub pace: Pace,
    pub placement: Placement,
    pub appearance: Appearance,
    pub text_size: TextSize,
    pub lettering: Lettering,
    pub ink: InkColor,
    pub about_me: String,
    pub learns_about_me: bool,
    /// Empty: whatever Claude Code uses by default.
    pub model: String,
    pub effort_read: String,
    pub effort: String,
    pub last_note: Option<String>,
    /// The models Claude Code reported last, shown while the brain is not running.
    pub models: Vec<ModelChoice>,
    /// Shortcuts you changed: action name to keys ("cmd-shift-k"); an empty list is none.
    pub shortcuts: BTreeMap<String, Vec<String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_mode: Mode::ThinksWithYou,
            ink_from_pointer: false,
            pace: Pace::Regular,
            placement: Placement::Both,
            appearance: Appearance::System,
            text_size: TextSize::Default,
            lettering: Lettering::Own,
            ink: InkColor::Ink,
            about_me: String::new(),
            learns_about_me: true,
            model: String::new(),
            effort_read: "low".into(),
            effort: "medium".into(),
            last_note: None,
            models: vec![],
            shortcuts: BTreeMap::new(),
        }
    }
}

pub struct Store {
    dir: PathBuf,
    pub notes: Vec<Note>,
    pub folders: Vec<Folder>,
    pub minds: Vec<Mind>,
    pub settings: Settings,
}

/// Write to a temp file beside `path`, then rename it over `path`: a crash mid-write
/// leaves the old file whole, and the `.tmp` leftover is ignored when loading.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

/// Save `value` as pretty JSON; failures are logged, since a save runs from the UI
/// and there is nobody to return the error to.
fn save_json<T: Serialize + ?Sized>(path: &Path, value: &T) {
    let result = serde_json::to_vec_pretty(value)
        .context("serializing")
        .and_then(|bytes| write_atomic(path, &bytes));
    if let Err(e) = result {
        log::error!("could not save {}: {e:#}", path.display());
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(v) => Some(v),
        Err(e) => {
            log::error!("could not parse {}: {e}", path.display());
            None
        }
    }
}

/// Read `value` field by field over `T::default()`: a value this build does not
/// understand (a newer option, a typo) costs that field, not the whole file. Settings
/// matter most here, because they are saved again on every note save and would
/// otherwise come back as defaults, about-you text included.
fn merge_fields<T: Serialize + DeserializeOwned + Default>(value: Value, what: &str) -> T {
    if let Ok(v) = serde_json::from_value::<T>(value.clone()) {
        return v;
    }
    let (Value::Object(fields), Ok(Value::Object(mut merged))) = (value, serde_json::to_value(T::default())) else {
        log::error!("ignoring {what}: not a JSON object");
        return T::default();
    };
    for (key, v) in fields {
        let previous = merged.insert(key.clone(), v);
        if serde_json::from_value::<T>(Value::Object(merged.clone())).is_err() {
            log::error!("ignoring {what}.{key}: not a value this version understands");
            match previous {
                Some(p) => merged.insert(key, p),
                None => merged.remove(&key),
            };
        }
    }
    serde_json::from_value(Value::Object(merged)).unwrap_or_default()
}

/// A JSON list where each entry is read on its own (see [`merge_fields`]).
fn read_list<T: Serialize + DeserializeOwned + Default>(path: &Path) -> Vec<T> {
    match read_json::<Value>(path) {
        None => vec![],
        Some(Value::Array(items)) => items.into_iter().map(|v| merge_fields(v, &path.display().to_string())).collect(),
        Some(_) => {
            log::error!("ignoring {}: not a JSON list", path.display());
            vec![]
        }
    }
}

/// `INKLING_DATA_DIR` when it is set and not empty, else `~/Library/Application Support/Inkling`.
fn data_dir_from(var: Option<OsString>) -> PathBuf {
    match var {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Inkling"),
    }
}

/// Every readable note in `dir`. A file that does not parse is logged and left alone.
/// The file name is the note's id, whatever the JSON says, so saving and deleting
/// always touch the file the note came from: a duplicated file is its own note, and
/// an id such as `../x` cannot point a save or a delete outside the folder.
fn load_notes(dir: &Path) -> Result<Vec<Note>> {
    let mut notes = vec![];
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else {
            log::error!("skipping {}: the file name is not UTF-8", path.display());
            continue;
        };
        let Some(mut note) = read_json::<Note>(&path) else { continue };
        if note.id != stem {
            log::warn!("{} says its id is {:?}; using the file name", path.display(), note.id);
            note.id = stem;
        }
        drop_broken_marks(&mut note);
        notes.push(note);
    }
    notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(notes)
}

/// Marks are byte ranges of the joined text. One that is empty, past the end, or
/// inside a character (a hand edit, another tool) would crash the editor when it
/// splits the line to draw it, so it is dropped when the note loads.
fn drop_broken_marks(note: &mut Note) {
    let text = note.joined_text();
    let before = note.marks.len();
    note.marks.retain(|m| m.start < m.end && m.end <= text.len() && text.is_char_boundary(m.start) && text.is_char_boundary(m.end));
    if note.marks.len() < before {
        log::warn!("note {}: dropped {} mark(s) that do not fit its text", note.id, before - note.marks.len());
    }
}

impl Store {
    pub fn data_dir() -> PathBuf {
        data_dir_from(std::env::var_os("INKLING_DATA_DIR"))
    }

    pub fn open() -> Result<Self> {
        Self::open_at(Self::data_dir())
    }

    pub fn open_at(dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(dir.join("notes")).with_context(|| format!("creating {}", dir.display()))?;
        let notes = load_notes(&dir.join("notes"))?;
        let mut folders: Vec<Folder> = read_list(&dir.join("folders.json"));
        let mut seen = HashSet::new();
        folders.retain(|f| !f.id.is_empty() && seen.insert(f.id.clone()));
        // The default mind is built in, first, and never read from minds.json.
        let mut minds: Vec<Mind> = read_list(&dir.join("minds.json"));
        let mut seen = HashSet::new();
        minds.retain(|m| !m.id.is_empty() && m.id != DEFAULT_MIND && seen.insert(m.id.clone()));
        minds.insert(0, Mind::inkling());
        let settings = read_json::<Value>(&dir.join("settings.json"))
            .map(|v| merge_fields(v, "settings.json"))
            .unwrap_or_default();
        let mut store = Self { dir, notes, folders, minds, settings };
        if store.notes.is_empty() {
            let welcome = crate::welcome::first_run_note(store.settings.default_mode);
            store.save_note(&welcome);
            store.notes.push(welcome);
        }
        Ok(store)
    }

    pub fn note(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn note_mut(&mut self, id: &str) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    pub fn mind(&self, id: &str) -> Mind {
        self.minds
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .unwrap_or_else(Mind::inkling)
    }

    pub fn save_note(&self, note: &Note) {
        save_json(&self.dir.join("notes").join(format!("{}.json", note.id)), note);
    }

    pub fn delete_note(&mut self, id: &str) {
        self.notes.retain(|n| n.id != id);
        let path = self.dir.join("notes").join(format!("{id}.json"));
        let _ = fs::remove_file(path);
    }

    /// Remove a folder. Its notes are kept and move to the top level.
    pub fn delete_folder(&mut self, id: &str) {
        self.folders.retain(|f| f.id != id);
        let moved: Vec<String> = self
            .notes
            .iter_mut()
            .filter(|n| n.folder.as_deref() == Some(id))
            .map(|n| {
                n.folder = None;
                n.id.clone()
            })
            .collect();
        for note_id in moved {
            if let Some(n) = self.note(&note_id) {
                self.save_note(n);
            }
        }
        self.save_folders();
    }

    pub fn save_folders(&self) {
        save_json(&self.dir.join("folders.json"), &self.folders);
    }

    pub fn save_minds(&self) {
        let custom: Vec<&Mind> = self.minds.iter().filter(|m| m.id != DEFAULT_MIND).collect();
        save_json(&self.dir.join("minds.json"), &custom);
    }

    pub fn save_settings(&self) {
        save_json(&self.dir.join("settings.json"), &self.settings);
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

pub fn hash_text(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.trim().hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh notebook folder under the system temp dir, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("inkling-store-test-{}", new_id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn notes(&self) -> PathBuf {
            self.0.join("notes")
        }
        fn note_files(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(self.notes())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn block(kind: BlockKind, text: &str) -> Block {
        Block { kind, text: text.into() }
    }

    #[std::prelude::v1::test]
    fn first_run_creates_the_welcome_note_once() {
        let tmp = TempDir::new();
        let store = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(store.notes.len(), 1);
        // a new notebook starts with the note that asks for your first sentence
        assert_eq!(store.notes[0].title, "Start here");
        assert!(store.notes[0].first_run);
        assert_eq!(tmp.note_files(), vec![format!("{}.json", store.notes[0].id)]);
        assert_eq!(store.minds.len(), 1);
        assert_eq!(store.minds[0].id, DEFAULT_MIND);
        assert_eq!(store.settings, Settings::default());
        drop(store);
        let again = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(again.notes.len(), 1, "a second launch must not add another welcome note");
    }

    #[std::prelude::v1::test]
    fn broken_note_files_are_skipped_and_left_alone() {
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.notes()).unwrap();
        let good = Note::new("Good", Mode::ThinksWithYou, None);
        fs::write(tmp.notes().join(format!("{}.json", good.id)), serde_json::to_vec(&good).unwrap()).unwrap();
        let full = serde_json::to_string(&good).unwrap();
        fs::write(tmp.notes().join("half.json"), &full[..full.len() / 2]).unwrap();
        fs::write(tmp.notes().join("empty.json"), "").unwrap();
        fs::write(tmp.notes().join("garbage.json"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
        fs::write(tmp.notes().join("wrong-shape.json"), "[1, 2, 3]").unwrap();
        fs::write(tmp.notes().join("bad-enum.json"), full.replace("\"thinks_with_you\"", "\"sometimes\"")).unwrap();
        fs::write(tmp.notes().join("leftover.json.tmp"), &full).unwrap();
        fs::write(tmp.notes().join("notes.txt"), "not a note").unwrap();
        fs::create_dir_all(tmp.notes().join("folder.json")).unwrap();

        let store = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(store.notes.len(), 1);
        assert_eq!(store.notes[0].title, "Good");
        // Nothing was rewritten or removed, so a fixed build can still read them.
        assert_eq!(fs::read_to_string(tmp.notes().join("empty.json")).unwrap(), "");
        assert_eq!(tmp.note_files().len(), 9);
    }

    #[std::prelude::v1::test]
    fn unknown_fields_are_ignored_and_missing_ones_default() {
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.notes()).unwrap();
        let minimal = r#"{"id": "minimal", "blocks": [{"kind": "title", "text": "Hand made"}], "from_the_future": {"x": 1}}"#;
        fs::write(tmp.notes().join("minimal.json"), minimal).unwrap();
        let with_thread = r#"{"id": "t", "title": "T", "blocks": [], "threads": [{"id": "a", "quote": "q", "extra": true}],
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-02T00:00:00Z"}"#;
        fs::write(tmp.notes().join("t.json"), with_thread).unwrap();

        let store = Store::open_at(tmp.0.clone()).unwrap();
        let m = store.note("minimal").expect("a note without dates or mode still opens");
        assert_eq!(m.title, "");
        assert_eq!(m.mode, Mode::ThinksWithYou);
        assert_eq!(m.mind_id, DEFAULT_MIND);
        assert!(m.threads.is_empty() && m.marks.is_empty() && m.read_hashes.is_empty());
        let t = &store.note("t").unwrap().threads[0];
        assert_eq!((t.status, t.kind, t.side, t.messages.len()), (ThreadStatus::Open, ThoughtKind::Question, Side::Right, 0));
        // Newest first.
        assert_eq!(store.notes[0].id, "t");
    }

    #[std::prelude::v1::test]
    fn a_message_keeps_its_check_and_an_unreadable_one_costs_only_the_check() {
        let plain: Message = serde_json::from_str(r#"{"role": "mind", "text": "Why 50?"}"#).unwrap();
        assert_eq!(plain.check, None);
        // a message without a check is written exactly as before
        assert!(!serde_json::to_string(&plain).unwrap().contains("check"));
        let garbled: Message = serde_json::from_str(r#"{"role": "mind", "text": "x", "check": {"sum": 5}}"#).unwrap();
        assert_eq!((garbled.text.as_str(), garbled.check), ("x", None));
        let from_later: Message = serde_json::from_str(r#"{"role": "mind", "text": "x", "check": {"sum": [{"sum": "5:30 × 10", "is": "55:00"}], "source": {"url": "u"}}}"#).unwrap();
        assert_eq!(from_later.check.as_ref().unwrap().sum[0], SumLine { sum: "5:30 × 10".into(), is: "55:00".into(), rounded: false });
        let mut m = Message::new(Role::Mind, "At 5:30 a km, 10 km takes 55:00.");
        m.check = Some(Check {
            sum: vec![SumLine { sum: "5:30 × 10".into(), is: "55:00".into(), rounded: false }],
            givens: vec![Given { value: "5:30".into(), words: "hold 5:30".into() }],
            words: vec![Passage { words: "run four days a week".into(), note: None }],
            taken_back: true,
        });
        let back: Message = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.check, m.check);
    }

    #[std::prelude::v1::test]
    fn the_file_name_is_the_note_id() {
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.notes()).unwrap();
        let mut note = Note::new("Copied in Finder", Mode::ThinksWithYou, None);
        let original = note.id.clone();
        fs::write(tmp.notes().join(format!("{original}.json")), serde_json::to_vec(&note).unwrap()).unwrap();
        fs::write(tmp.notes().join(format!("{original} copy.json")), serde_json::to_vec(&note).unwrap()).unwrap();
        note.id = "../../escaped".into();
        fs::write(tmp.notes().join("sneaky.json"), serde_json::to_vec(&note).unwrap()).unwrap();

        let mut store = Store::open_at(tmp.0.clone()).unwrap();
        let mut ids: Vec<String> = store.notes.iter().map(|n| n.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec![original.clone(), format!("{original} copy"), "sneaky".to_string()]);
        let sneaky = store.note("sneaky").unwrap().clone();
        store.save_note(&sneaky);
        store.delete_note(&format!("{original} copy"));
        assert!(!tmp.0.parent().unwrap().join("escaped.json").exists());
        assert_eq!(tmp.note_files(), vec![format!("{original}.json"), "sneaky.json".to_string()]);
    }

    #[std::prelude::v1::test]
    fn marks_that_do_not_fit_the_text_are_dropped() {
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.notes()).unwrap();
        let mut note = Note::new("Bad mark", Mode::ThinksWithYou, None);
        note.blocks = vec![block(BlockKind::Title, "Bad mark"), block(BlockKind::Paragraph, "café au lait")];
        let e = note.joined_text().find('é').unwrap();
        let mark = |start, end| MarkSpan { start, end, mark: Mark::Bold };
        note.marks = vec![mark(e, e + 2), mark(e + 1, e + 4), mark(5, 5), mark(9, 400), mark(0, 3)];
        fs::write(tmp.notes().join(format!("{}.json", note.id)), serde_json::to_vec(&note).unwrap()).unwrap();
        let store = Store::open_at(tmp.0.clone()).unwrap();
        let kept: Vec<(usize, usize)> = store.notes[0].marks.iter().map(|m| (m.start, m.end)).collect();
        assert_eq!(kept, vec![(e, e + 2), (0, 3)]);
    }

    #[std::prelude::v1::test]
    fn saves_replace_the_file_whole_and_leave_no_temp_file() {
        let tmp = TempDir::new();
        let mut store = Store::open_at(tmp.0.clone()).unwrap();
        let mut note = Note::new("Draft", Mode::WhenYouAsk, Some("folder-1".into()));
        note.blocks.push(block(BlockKind::Bullet, "a point with “quotes” and ✍️"));
        store.save_note(&note);
        note.blocks[1].text = "second version".into();
        store.save_note(&note);
        store.notes.insert(0, note.clone());
        let files = tmp.note_files();
        assert!(files.iter().all(|f| !f.ends_with(".tmp")), "{files:?}");
        let reread: Note = serde_json::from_slice(&fs::read(tmp.notes().join(format!("{}.json", note.id))).unwrap()).unwrap();
        assert_eq!(reread.blocks, note.blocks);
        assert_eq!(reread.mode, Mode::WhenYouAsk);
        assert_eq!(reread.folder.as_deref(), Some("folder-1"));
        assert!(!tmp.0.join("settings.json.tmp").exists());
    }

    #[std::prelude::v1::test]
    fn deleting_notes_and_folders() {
        let tmp = TempDir::new();
        let mut store = Store::open_at(tmp.0.clone()).unwrap();
        let welcome = store.notes[0].id.clone();
        store.folders.push(Folder { id: "f".into(), name: "Work".into(), collapsed: false });
        store.save_folders();
        let filed = Note::new("Filed", Mode::ThinksWithYou, Some("f".into()));
        store.save_note(&filed);
        store.notes.insert(0, filed.clone());

        store.delete_folder("f");
        assert!(store.folders.is_empty());
        assert_eq!(store.note(&filed.id).unwrap().folder, None, "its notes move to the top level");
        let reopened = Store::open_at(tmp.0.clone()).unwrap();
        assert!(reopened.folders.is_empty());
        assert_eq!(reopened.note(&filed.id).unwrap().folder, None);

        store.delete_note(&welcome);
        store.delete_note("no-such-note");
        assert_eq!(store.notes.len(), 1);
        assert_eq!(tmp.note_files(), vec![format!("{}.json", filed.id)]);
    }

    #[std::prelude::v1::test]
    fn the_default_mind_is_built_in_and_never_saved() {
        let tmp = TempDir::new();
        let mut store = Store::open_at(tmp.0.clone()).unwrap();
        let mut skeptic = mind_templates()[2].clone();
        skeptic.id = "skeptic".into();
        store.minds.push(skeptic);
        store.save_minds();
        let saved: Vec<Mind> = serde_json::from_slice(&fs::read(tmp.0.join("minds.json")).unwrap()).unwrap();
        assert_eq!(saved.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec!["skeptic"]);

        // Even when minds.json mentions the default (an older build, a hand edit),
        // the built-in one is used and stays first; entries that cannot be
        // understood fall back field by field instead of dropping every mind.
        fs::write(
            tmp.0.join("minds.json"),
            r#"[{"id": "skeptic", "name": "Skeptic", "voice": "sardonic"},
                {"id": "inkling", "name": "Renamed", "description": "", "how_it_thinks": "", "voice": "plain", "frequency": "often"},
                {"id": "skeptic", "name": "Duplicate"},
                {"name": "No id"},
                "not even an object"]"#,
        )
        .unwrap();
        let mut store = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(store.minds.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec![DEFAULT_MIND, "skeptic"]);
        assert_eq!(store.minds[0].name, "Inkling");
        assert_eq!(store.minds[1].name, "Skeptic");
        assert_eq!(store.minds[1].voice, Voice::Plain);
        assert_eq!(store.mind("deleted-mind").id, DEFAULT_MIND);

        store.minds.retain(|m| m.id != "skeptic");
        store.save_minds();
        assert_eq!(fs::read_to_string(tmp.0.join("minds.json")).unwrap().trim(), "[]");
        let store = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(store.minds.len(), 1);
    }

    #[std::prelude::v1::test]
    fn settings_defaults_and_round_trip() {
        let d = Settings::default();
        assert_eq!(d.default_mode, Mode::ThinksWithYou);
        assert_eq!((d.pace, d.placement, d.appearance), (Pace::Regular, Placement::Both, Appearance::System));
        assert_eq!((d.text_size, d.lettering, d.ink), (TextSize::Default, Lettering::Own, InkColor::Ink));
        assert!(!d.ink_from_pointer && d.learns_about_me && d.about_me.is_empty() && d.last_note.is_none());
        assert_eq!((d.model.as_str(), d.effort_read.as_str(), d.effort.as_str()), ("", "low", "medium"), "Claude Code picks the model");
        assert!(d.models.is_empty() && d.shortcuts.is_empty());
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap(), d);

        let tmp = TempDir::new();
        let mut store = Store::open_at(tmp.0.clone()).unwrap();
        store.settings = Settings {
            default_mode: Mode::WhenYouAsk,
            ink_from_pointer: true,
            pace: Pace::Slow,
            placement: Placement::Marks,
            appearance: Appearance::Dark,
            text_size: TextSize::Large,
            lettering: Lettering::LikeNotes,
            ink: InkColor::Orange,
            about_me: "I teach physics. ünïcödé ✓".into(),
            learns_about_me: false,
            model: "sonnet".into(),
            effort_read: "medium".into(),
            effort: "high".into(),
            last_note: Some("abc".into()),
            models: vec![
                ModelChoice { value: "default".into(), name: "Default".into(), description: "Recommended".into(), resolved: None, efforts: None },
                ModelChoice {
                    value: "sonnet".into(),
                    name: "Sonnet".into(),
                    description: String::new(),
                    resolved: Some("claude-sonnet-x".into()),
                    efforts: Some(vec!["low".into(), "high".into()]),
                },
            ],
            shortcuts: BTreeMap::from([("app::NewNote".to_string(), vec!["alt-cmd-n".to_string()]), ("app::Quit".to_string(), vec![])]),
        };
        store.save_settings();
        let reopened = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(reopened.settings, store.settings);
    }

    #[std::prelude::v1::test]
    fn one_bad_setting_does_not_reset_the_others() {
        let tmp = TempDir::new();
        fs::write(
            tmp.0.join("settings.json"),
            r#"{"appearance": "sepia", "about_me": "Keep this paragraph.", "model": null, "pace": "slow", "new_option": 3}"#,
        )
        .unwrap();
        let store = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(store.settings.about_me, "Keep this paragraph.");
        assert_eq!(store.settings.pace, Pace::Slow);
        assert_eq!(store.settings.appearance, Appearance::System);
        assert_eq!(store.settings.model, "");

        fs::write(tmp.0.join("settings.json"), "{\"about_me\": \"cut off").unwrap();
        assert_eq!(Store::open_at(tmp.0.clone()).unwrap().settings, Settings::default());
        fs::write(tmp.0.join("settings.json"), "[]").unwrap();
        assert_eq!(Store::open_at(tmp.0.clone()).unwrap().settings, Settings::default());
    }

    #[std::prelude::v1::test]
    fn a_saved_model_picks_its_own_row_then_an_alias_then_the_default() {
        let m = |value: &str, resolved: &str| ModelChoice { value: value.into(), resolved: Some(resolved.into()), ..Default::default() };
        let list = vec![
            m("default", "claude-opus-x"),
            m("opus", "claude-opus-x"),
            m("sonnet", "claude-sonnet-x"),
            m("claude-sonnet-x", "claude-sonnet-x"),
            m("claude-haiku-y", "claude-haiku-y"),
        ];
        assert_eq!(chosen_model(&list, ""), Some(0), "empty is Claude Code's default");
        assert_eq!(chosen_model(&list, "default"), Some(0));
        assert_eq!(chosen_model(&list, "opus"), Some(1));
        assert_eq!(chosen_model(&list, " claude-opus-x "), Some(1), "a saved full id shows as its alias, not the default");
        assert_eq!(chosen_model(&list, "claude-sonnet-x"), Some(3), "its own row wins over the alias");
        assert_eq!(chosen_model(&list, "claude-haiku-y"), Some(4));
        assert_eq!(chosen_model(&list, "gpt"), None);
        assert_eq!(chosen_model(&[m("opus", "x")], ""), None);
        assert!(list[4].is_pinned() && !list[1].is_pinned() && !list[0].is_pinned());
        let only_default = vec![m("default", "claude-opus-x")];
        assert_eq!(chosen_model(&only_default, "claude-opus-x"), Some(0), "the default when nothing else stands for it");
    }

    #[std::prelude::v1::test]
    fn shortcuts_and_models_that_do_not_read_cost_only_themselves() {
        let tmp = TempDir::new();
        fs::write(
            tmp.0.join("settings.json"),
            r#"{"about_me": "Kept.", "shortcuts": {"app::NewNote": "cmd-n"}, "models": [{"value": 3}], "pace": "slow"}"#,
        )
        .unwrap();
        let store = Store::open_at(tmp.0.clone()).unwrap();
        assert_eq!(store.settings.about_me, "Kept.");
        assert_eq!(store.settings.pace, Pace::Slow);
        assert!(store.settings.shortcuts.is_empty() && store.settings.models.is_empty());
    }

    #[std::prelude::v1::test]
    fn folders_file_is_read_entry_by_entry() {
        let tmp = TempDir::new();
        fs::write(
            tmp.0.join("folders.json"),
            r#"[{"id": "a", "name": "Work", "collapsed": "yes"}, {"name": "no id"}, {"id": "b"}, {"id": "a", "name": "dup"}]"#,
        )
        .unwrap();
        let store = Store::open_at(tmp.0.clone()).unwrap();
        let got: Vec<(&str, &str, bool)> = store.folders.iter().map(|f| (f.id.as_str(), f.name.as_str(), f.collapsed)).collect();
        assert_eq!(got, vec![("a", "Work", false), ("b", "", false)]);
    }

    #[std::prelude::v1::test]
    fn data_dir_comes_from_inkling_data_dir_when_set() {
        assert_eq!(data_dir_from(Some("/tmp/somewhere".into())), PathBuf::from("/tmp/somewhere"));
        let default = data_dir_from(None);
        assert!(default.ends_with("Inkling"), "{default:?}");
        assert_eq!(data_dir_from(Some(OsString::new())), default, "an empty variable is unset, not the current folder");
    }

    #[std::prelude::v1::test]
    fn plain_text_for_every_block_kind() {
        let blocks = vec![
            block(BlockKind::Title, "The title is not body text"),
            block(BlockKind::H1, "One"),
            block(BlockKind::H2, "Two"),
            block(BlockKind::H3, "Three"),
            block(BlockKind::Paragraph, "Plain words."),
            block(BlockKind::Bullet, "dot"),
            block(BlockKind::Bullet, "dot again"),
            block(BlockKind::Paragraph, "   "),
            block(BlockKind::Numbered, "first"),
            block(BlockKind::Paragraph, ""),
            block(BlockKind::Numbered, "second, across an empty line"),
            block(BlockKind::Paragraph, "After the list."),
            block(BlockKind::Numbered, "numbering starts again"),
            block(BlockKind::Bullet, "a different list"),
            block(BlockKind::Quote, "Said someone."),
            block(BlockKind::Title, "a stray title block reads as a paragraph"),
        ];
        let expected = "# One\n\n## Two\n\n### Three\n\nPlain words.\n\n- dot\n- dot again\n\n1. first\n2. second, across an empty line\n\n\
After the list.\n\n1. numbering starts again\n\n- a different list\n\n> Said someone.\n\na stray title block reads as a paragraph";
        assert_eq!(plain_text_of(&blocks), expected);
        assert_eq!(plain_text_of(&[]), "");
        assert_eq!(plain_text_of(&[block(BlockKind::Title, "Only a title")]), "");
        let note = Note { blocks: blocks.clone(), ..Note::new("x", Mode::ThinksWithYou, None) };
        assert_eq!(note.plain_text(), expected);
    }

    #[std::prelude::v1::test]
    fn joined_text_and_thread_counts() {
        let mut note = Note::new("T", Mode::ThinksWithYou, None);
        note.blocks = vec![block(BlockKind::Title, "T"), block(BlockKind::Paragraph, "é"), block(BlockKind::Bullet, "x")];
        assert_eq!(note.joined_text(), "T\né\nx");
        let welcome = crate::welcome::welcome_note(Mode::ThinksWithYou);
        assert_eq!((welcome.open_threads(), welcome.visible_threads()), (1, 1));
        let t = &welcome.threads[0];
        assert_eq!(&welcome.joined_text()[t.start..t.end], t.quote);
        assert_eq!(hash_text("  same words "), hash_text("same words"));
    }

    #[std::prelude::v1::test]
    fn new_block_kinds_round_trip_and_old_notes_still_load() {
        use BlockKind::{Check, Code, Paragraph};
        // the new kinds as JSON: an object for the ones that carry a value
        let blocks = vec![block(Check(true), "done"), block(BlockKind::Callout(Callout::Warning), "w"), block(Code(Lang::new("Rust")), "fn"), block(Code(Lang::default()), "")];
        let json = serde_json::to_string(&blocks).unwrap();
        assert_eq!(
            json,
            r#"[{"kind":{"check":true},"text":"done"},{"kind":{"callout":"warning"},"text":"w"},{"kind":{"code":"rust"},"text":"fn"},{"kind":{"code":""},"text":""}]"#
        );
        assert_eq!(serde_json::from_str::<Vec<Block>>(&json).unwrap(), blocks);
        // a kind from a newer build, or a broken one, reads as a paragraph, and so does a
        // block without one; a language is cut to its first word and 15 bytes
        let odd = r#"[{"kind":"table","text":"a"},{"kind":{"check":"yes"},"text":"b"},{"text":"c"},{"kind":{"code":"Objective-C++ {x}"},"text":"d"},{"kind":{"code":"averyveryverylonglanguage"}},{"kind":{"callout":"danger"},"text":"e"}]"#;
        let blocks: Vec<Block> = serde_json::from_str(odd).unwrap();
        assert_eq!(
            blocks.iter().map(|b| b.kind).collect::<Vec<_>>(),
            [Paragraph, Paragraph, Paragraph, Code(Lang::new("objective-c++")), Code(Lang::new("averyveryverylo")), Paragraph]
        );
        assert_eq!((blocks[2].text.as_str(), blocks[4].text.as_str()), ("c", ""));
        assert_eq!(Lang::new("averyveryverylonglanguage").as_str().len(), Lang::MAX);
        assert_eq!(format!("{:?}", Lang::new("C#")), "\"c#\"");

        // on disk: a note saved before these kinds, and one with them, both open
        let tmp = TempDir::new();
        fs::create_dir_all(tmp.notes()).unwrap();
        let old = r#"{"id":"old","title":"Old","blocks":[{"kind":"title","text":"Old"},{"kind":"bullet","text":"[ ] still words"},{"kind":"quote","text":"q"}],"marks":[{"start":4,"end":9,"mark":"code"}]}"#;
        fs::write(tmp.notes().join("old.json"), old).unwrap();
        let mut new = Note::new("New", Mode::ThinksWithYou, None);
        new.blocks = vec![block(BlockKind::Title, "New"), block(Check(false), "item"), block(Code(Lang::new("py")), "x = 1")];
        fs::write(tmp.notes().join(format!("{}.json", new.id)), serde_json::to_vec(&new).unwrap()).unwrap();
        let store = Store::open_at(tmp.0.clone()).unwrap();
        let old = store.note("old").expect("the old note opens");
        assert_eq!((old.blocks[1].kind, old.blocks[1].text.as_str(), old.marks.len()), (BlockKind::Bullet, "[ ] still words", 1));
        assert_eq!(store.note(&new.id).unwrap().blocks, new.blocks);
        // the kinds in a word, for the script's dump
        let names: Vec<String> = [BlockKind::Title, Paragraph, BlockKind::H2, Check(false), Check(true), BlockKind::Callout(Callout::Tip), Code(Lang::new("rust")), Code(Lang::default())]
            .into_iter()
            .map(kind_name)
            .collect();
        assert_eq!(names, ["title", "paragraph", "h2", "check", "check:done", "callout:tip", "code:rust", "code"]);
    }

    #[std::prelude::v1::test]
    fn checklists_callouts_and_code_as_markdown() {
        use BlockKind::{Bullet, Check, Code, Paragraph, Quote};
        let rust = Code(Lang::new("rust"));
        let blocks = vec![
            block(BlockKind::Title, "T"),
            block(Paragraph, "Intro."),
            block(Check(true), "bib"),
            block(Check(false), "run"),
            block(Bullet, "loose"),
            block(BlockKind::Callout(Callout::Note), "one"),
            block(BlockKind::Callout(Callout::Note), ""),
            block(BlockKind::Callout(Callout::Note), "two"),
            block(BlockKind::Callout(Callout::Tip), "tip"),
            block(rust, ""),
            block(rust, "fn a() {"),
            block(rust, ""),
            block(rust, "    1 // **not bold**"),
            block(rust, "}"),
            block(rust, "  "),
            block(Paragraph, "After."),
            block(Code(Lang::default()), "```inside"),
            block(Code(Lang::default()), ""),
            block(Quote, "q"),
        ];
        let expected = "Intro.\n\n- [x] bib\n- [ ] run\n- loose\n\n> [!note]\n> one\n>\n> two\n\n> [!tip]\n> tip\n\n\
```rust\nfn a() {\n\n    1 // **not bold**\n}\n```\n\nAfter.\n\n````\n```inside\n````\n\n> q";
        assert_eq!(plain_text_of(&blocks), expected);
        // what goes under a line of a block (its threads) comes after the whole block
        let md = markdown_blocks(&blocks, 0, |i| blocks[i].text.clone(), |i| (i == 5 || i == 11).then(|| format!("[under {i}]\n\n")));
        assert!(md.contains("> two\n\n[under 5]\n\n> [!tip]"), "{md}");
        assert!(md.contains("}\n```\n\n[under 11]\n\nAfter."), "{md}");
        // a block with no words is left out
        assert_eq!(plain_text_of(&[block(BlockKind::Title, "T"), block(rust, " "), block(BlockKind::Callout(Callout::Tip), "")]), "");
    }

    /// A thread on `quote` as it would be saved, of `kind` and `status`.
    fn a_thread(quote: &str, kind: ThoughtKind, status: ThreadStatus) -> Thread {
        Thread {
            id: new_id(),
            start: 0,
            end: quote.len(),
            quote: quote.into(),
            kind,
            messages: vec![Message::new(Role::Mind, "Why?")],
            drawing: None,
            status,
            side: Side::Right,
            mind_name: "Inkling".into(),
            tag_offset: (0., 0.),
            created_at: Utc::now(),
            reason: None,
            ended_at: None,
            first_quote: None,
        }
    }

    #[std::prelude::v1::test]
    fn old_threads_load_without_the_new_fields() {
        let old = r#"{"id": "a", "quote": "four days a week", "kind": "question", "status": "dismissed",
            "messages": [{"role": "mind", "text": "Why four?"}]}"#;
        let t: Thread = serde_json::from_str(old).unwrap();
        assert_eq!((t.reason.as_deref(), t.ended_at, t.first_quote.as_deref()), (None, None, None));
        assert_eq!(t.ending(), Ending::LetStand);
        // saved again, it gains no nulls: an older build reads the same file it wrote
        let saved = serde_json::to_value(&t).unwrap();
        for field in ["reason", "ended_at", "first_quote"] {
            assert!(saved.get(field).is_none(), "{field} was written as {saved}");
        }
        // and the fields come back once they are set
        let mut t = t;
        t.set_reason("Pace is the problem.");
        t.ended_at = Some(Utc::now());
        t.set_quote("five days a week");
        let back: Thread = serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap();
        assert_eq!(back.reason.as_deref(), Some("Pace is the problem."));
        assert_eq!(back.ended_at, t.ended_at);
        assert_eq!((back.first_quote.as_deref(), back.quote.as_str()), (Some("four days a week"), "five days a week"));
    }

    #[std::prelude::v1::test]
    fn endings_follow_status_and_kind() {
        use ThoughtKind::*;
        for kind in [Question, Correction] {
            assert_eq!(a_thread("q", kind, ThreadStatus::Dismissed).ending(), Ending::LetStand);
            assert!(a_thread("q", kind, ThreadStatus::Open).is_point());
        }
        for kind in [Ask, Sketch, Answer] {
            assert_eq!(a_thread("q", kind, ThreadStatus::Dismissed).ending(), Ending::Snipped);
            assert!(!a_thread("q", kind, ThreadStatus::Open).is_point());
        }
        // a loose thread dismissed before its mind said anything raised no point
        let mut empty = a_thread("q", Question, ThreadStatus::Dismissed);
        empty.messages.clear();
        assert_eq!((empty.is_point(), empty.ending()), (false, Ending::Snipped));
        assert_eq!(a_thread("q", Ask, ThreadStatus::Resolved).ending(), Ending::TiedOff);
        assert_eq!(a_thread("q", Question, ThreadStatus::Open).ending(), Ending::Open);
        assert_eq!(
            [Ending::Open, Ending::TiedOff, Ending::LetStand, Ending::Snipped].map(Ending::label),
            ["Still open", "Tied off", "Let stand", "Snipped"]
        );
    }

    #[std::prelude::v1::test]
    fn set_status_stamps_and_clears() {
        let mut t = a_thread("q", ThoughtKind::Question, ThreadStatus::Open);
        t.set_status(ThreadStatus::Dismissed);
        t.set_reason("Deliberate.");
        let stamped = t.ended_at.expect("dismissing stamps when it ended");
        // the same status again keeps the stamp
        t.set_status(ThreadStatus::Dismissed);
        assert_eq!(t.ended_at, Some(stamped));
        // open again: no date, no reason
        t.set_status(ThreadStatus::Open);
        assert_eq!((t.ended_at, t.reason.as_deref()), (None, None));
        t.set_status(ThreadStatus::Resolved);
        assert!(t.ended_at.is_some(), "tying off stamps it too");
        assert_eq!(t.status, ThreadStatus::Resolved);
    }

    #[std::prelude::v1::test]
    fn set_quote_keeps_the_first_words() {
        let mut t = a_thread("hold 5:30", ThoughtKind::Correction, ThreadStatus::Open);
        t.set_quote("hold 5:30");
        assert_eq!(t.first_quote, None, "the same words are no change");
        assert_eq!(t.words_before(), "hold 5:30");
        t.set_quote("hold 5:00");
        t.set_quote("hold 4:55");
        assert_eq!((t.words_before(), t.quote.as_str()), ("hold 5:30", "hold 4:55"));
        assert_eq!(t.first_quote.as_deref(), Some("hold 5:30"), "later changes keep the first words");
    }

    #[std::prelude::v1::test]
    fn set_reason_is_one_line() {
        let mut t = a_thread("q", ThoughtKind::Question, ThreadStatus::Dismissed);
        t.set_reason("  Pace is\nthe   problem.\t ");
        assert_eq!(t.reason.as_deref(), Some("Pace is the problem."));
        // cut by characters, never inside one
        t.set_reason(&"é".repeat(REASON_MAX + 40));
        assert_eq!(t.reason.as_ref().map(|r| r.chars().count()), Some(REASON_MAX));
        t.set_reason(" \n ");
        assert_eq!(t.reason, None);
    }
}

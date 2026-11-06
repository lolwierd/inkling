//! File > Import from Obsidian…: the Markdown notes of a vault become Inkling notes.
//!
//! The aim is what the vault shows in Obsidian's reading view, in Inkling's blocks.
//! Headings, lists, quotes and paragraphs keep their shape; tasks (`- [ ]`, `- [x]`)
//! become checklist items, callouts (`> [!note]`) become callouts, Obsidian's other
//! callout types folded into note, warning and tip, and a code fence becomes a code
//! block in its language, its lines exactly as written. What Obsidian does not show as
//! words goes: front matter, comments, link brackets, emphasis marks, block ids, and
//! embeds and images, which show files rather than words. Math stays as written.
//!
//! The vault is only read. Every imported paragraph is left alone (`Note::left_alone`):
//! reading a whole vault would be hundreds of model calls, so only the words written
//! after the import get read.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use gpui_kit::{App, Task};

use crate::store::{Block, BlockKind, Callout, Folder, Lang, Mode, Note, Store, hash_text, new_id};

/// Imports still reading their vault.
static IMPORTING: AtomicUsize = AtomicUsize::new(0);

/// Is an import still on its way? A script's `settle` waits for it as it waits for the page.
pub fn importing() -> bool {
    IMPORTING.load(Ordering::SeqCst) > 0
}

/// Import the vault at `root`: read it off the main thread, then add its notes to the
/// notebook. Resolves to the line the toast shows.
pub fn import(root: PathBuf, cx: &mut App) -> Task<String> {
    let name = root.file_name().map_or_else(|| root.display().to_string(), |n| n.to_string_lossy().into_owned());
    IMPORTING.fetch_add(1, Ordering::SeqCst);
    let read = cx.background_executor().spawn(async move { read_vault(&root) });
    cx.spawn(async move |cx| {
        let vault = read.await;
        let message = cx.update(|cx| match vault {
            Ok(vault) => {
                let mode = cx.global::<Store>().settings.default_mode;
                let message = add_to_store(cx.global_mut::<Store>(), vault, mode).message(&name);
                log::info!("import: {message}");
                message
            }
            Err(e) => {
                log::error!("import from {name}: {e:#}");
                format!("Couldn’t import {name}: {e:#}")
            }
        });
        IMPORTING.fetch_sub(1, Ordering::SeqCst);
        message
    })
}

// ----- the vault ---------------------------------------------------------------------

/// A note read from a vault and converted, waiting to be added to the notebook.
#[derive(Debug)]
pub struct Imported {
    /// The folders between the vault and the file, outermost first.
    pub folders: Vec<String>,
    /// The title block, then the body.
    pub blocks: Vec<Block>,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
}

/// What reading a vault found.
#[derive(Debug, Default)]
pub struct Vault {
    pub notes: Vec<Imported>,
    /// Files and folders that could not be read.
    pub unreadable: usize,
    /// Notes iCloud keeps only in the cloud: on this Mac they are `.Name.md.icloud`.
    pub in_icloud: usize,
}

/// Read every note in the vault at `root`, folder by folder, a folder before the
/// folders inside it, so their Inkling folders are made in that order. Hidden files and
/// folders are skipped, as Obsidian skips them: its settings (`.obsidian/`), its trash
/// (`.trash/`), `.git/`. So are links, which could loop or lead out of the vault, and
/// every file that is not Markdown. Nothing is written.
pub fn read_vault(root: &Path) -> Result<Vault> {
    let line_breaks = shows_line_breaks(root);
    let mut vault = Vault::default();
    walk(root, &mut vec![], line_breaks, &mut vault).with_context(|| format!("reading {}", root.display()))?;
    vault.notes.sort_by_cached_key(|n| n.folders.iter().map(|f| f.to_lowercase()).collect::<Vec<_>>());
    Ok(vault)
}

fn walk(dir: &Path, folders: &mut Vec<String>, line_breaks: bool, vault: &mut Vault) -> std::io::Result<()> {
    let mut entries = vec![];
    for entry in fs::read_dir(dir)? {
        match entry {
            Ok(entry) => entries.push((entry.file_name().to_string_lossy().into_owned(), entry)),
            Err(_) => vault.unreadable += 1,
        }
    }
    entries.sort_by_cached_key(|(name, _)| name.to_lowercase());
    for (name, entry) in entries {
        if name.starts_with('.') {
            if ends_with_ignoring_case(&name, ".md.icloud") {
                vault.in_icloud += 1;
            }
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            vault.unreadable += 1;
            continue;
        };
        if kind.is_dir() {
            folders.push(name);
            if walk(&entry.path(), folders, line_breaks, vault).is_err() {
                vault.unreadable += 1;
            }
            folders.pop();
        } else if kind.is_file() && is_note(&name) {
            let Ok(bytes) = fs::read(entry.path()) else {
                vault.unreadable += 1;
                continue;
            };
            let meta = entry.metadata().ok();
            let time = |t: std::io::Result<std::time::SystemTime>| t.ok().map(DateTime::<Utc>::from);
            vault.notes.push(Imported {
                folders: folders.clone(),
                blocks: convert(&String::from_utf8_lossy(&bytes), &name[..name.len() - ".md".len()], line_breaks),
                created: meta.as_ref().and_then(|m| time(m.created())),
                modified: meta.as_ref().and_then(|m| time(m.modified())),
            });
        }
    }
    Ok(())
}

/// A Markdown note. Drawings from the Excalidraw plugin are Markdown files too
/// (`*.excalidraw.md`), but Obsidian opens them as drawings, so they stay behind.
fn is_note(name: &str) -> bool {
    ends_with_ignoring_case(name, ".md") && !ends_with_ignoring_case(name, ".excalidraw.md")
}

fn ends_with_ignoring_case(name: &str, suffix: &str) -> bool {
    name.len() >= suffix.len() && name.as_bytes()[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix.as_bytes())
}

/// Whether the vault shows each line of a paragraph on a line of its own. Obsidian
/// does, unless "Strict line breaks" is on in its settings (`.obsidian/app.json`). A
/// folder never opened in Obsidian is plain Markdown, where a line break only wraps.
fn shows_line_breaks(root: &Path) -> bool {
    let settings = root.join(".obsidian");
    if !settings.is_dir() {
        return false;
    }
    let strict = fs::read(settings.join("app.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|app| app.get("strictLineBreaks")?.as_bool());
    !strict.unwrap_or(false)
}

// ----- into the notebook ---------------------------------------------------------------

/// What an import did, for the toast.
#[derive(Debug, Default, PartialEq)]
pub struct Summary {
    pub imported: usize,
    /// Notes the notebook already has word for word, in the same folder.
    pub already_here: usize,
    pub unreadable: usize,
    pub in_icloud: usize,
}

impl Summary {
    /// "Imported 42 notes from Vault", and what was left out.
    pub fn message(&self, vault: &str) -> String {
        let notes = |n: usize| if n == 1 { "1 note".to_string() } else { format!("{n} notes") };
        let mut parts = vec![match (self.imported, self.already_here) {
            (0, 0) => format!("No notes found in {vault}"),
            (0, n) => format!("Nothing new in {vault}: {} already here", notes(n)),
            (n, _) => format!("Imported {} from {vault}", notes(n)),
        }];
        if self.imported > 0 && self.already_here > 0 {
            parts.push(format!("{} already here", self.already_here));
        }
        if self.in_icloud > 0 {
            parts.push(format!("{} only in iCloud", notes(self.in_icloud)));
        }
        if self.unreadable > 0 {
            parts.push(format!("{} could not be read", self.unreadable));
        }
        parts.join("; ")
    }
}

/// Add a vault's notes to the notebook and save them.
///
/// Inkling's folders do not nest, so each vault folder becomes one folder named by its
/// path from the vault, "Projects / Inkling". The whole path keeps two "Meetings"
/// folders in different projects apart, puts a folder's subfolders right after it in
/// the sidebar, and splits back on " / " if folders ever nest, since a file name cannot
/// hold a slash. A folder of the same name is reused. New folders start folded, so a
/// big vault arrives as its outline rather than every note at once.
///
/// A note the notebook already has word for word, in the same folder, is not added
/// again: importing a vault twice does not double it, while a note that changed since
/// comes in beside the old one.
pub fn add_to_store(store: &mut Store, vault: Vault, mode: Mode) -> Summary {
    let mut summary = Summary { unreadable: vault.unreadable, in_icloud: vault.in_icloud, ..Summary::default() };
    let before = store.notes.len();
    let mut new_folders = false;
    let now = Utc::now();
    for Imported { folders, blocks, created, modified } in vault.notes {
        let folder = (!folders.is_empty()).then(|| {
            let name = folders.join(" / ");
            if let Some(f) = store.folders.iter().find(|f| same_folder(&f.name, &name)) {
                return f.id.clone();
            }
            let folder = Folder { id: new_id(), name, collapsed: true };
            let id = folder.id.clone();
            store.folders.push(folder);
            new_folders = true;
            id
        });
        if store.notes[..before].iter().any(|n| n.folder == folder && n.blocks == blocks) {
            summary.already_here += 1;
            continue;
        }
        let mut note = Note::new(blocks.first().map_or("", |b| b.text.as_str()), mode, folder);
        note.left_alone = left_alone(&blocks);
        note.blocks = blocks;
        note.updated_at = modified.unwrap_or(now);
        note.created_at = created.unwrap_or(note.updated_at);
        store.save_note(&note);
        store.notes.push(note);
        summary.imported += 1;
    }
    if new_folders {
        store.save_folders();
    }
    // newest first, the order the notebook opens in
    store.notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    summary
}

/// Folder names match ignoring case, as they do when the palette files a new note.
fn same_folder(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// The hashes of the body's paragraphs, which the reading loop then leaves alone.
fn left_alone(blocks: &[Block]) -> Vec<u64> {
    let mut seen = HashSet::new();
    blocks
        .iter()
        .skip(1)
        .filter(|b| !b.text.trim().is_empty())
        .map(|b| hash_text(&b.text))
        .filter(|h| seen.insert(*h))
        .collect()
}

// ----- one note ----------------------------------------------------------------------

/// A Markdown note as Inkling blocks: the title, then the body.
///
/// The title is a `# ` heading that opens the note, else the file name. A `# ` heading
/// further down heads a section; lifting it out to be the title would leave that
/// section without its heading. `line_breaks`: every line of a paragraph is a block of
/// its own (see [`shows_line_breaks`]); otherwise wrapped lines join into one.
pub fn convert(markdown: &str, file_name: &str, line_breaks: bool) -> Vec<Block> {
    let source = without_comments(without_front_matter(&normalized(markdown)));
    let lines: Vec<&str> = source.lines().collect();
    let mut body: Vec<Block> = parse(&lines, false, line_breaks).into_iter().filter_map(Raw::finish).collect();
    let title = match body.first() {
        Some(b) if b.kind == BlockKind::H1 => body.remove(0).text,
        _ => file_name.trim().to_string(),
    };
    if body.is_empty() {
        body.push(Block { kind: BlockKind::Paragraph, text: String::new() });
    }
    std::iter::once(Block { kind: BlockKind::Title, text: title }).chain(body).collect()
}

fn normalized(markdown: &str) -> String {
    markdown.strip_prefix('\u{feff}').unwrap_or(markdown).replace("\r\n", "\n").replace('\r', "\n")
}

/// The note without its YAML front matter: a first line `---`, through the next `---`
/// (or `...`). With no closing line it is not front matter, and all of it stays.
fn without_front_matter(text: &str) -> &str {
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next().filter(|l| l.trim_end() == "---") else { return text };
    let mut end = first.len();
    for line in lines {
        end += line.len();
        if matches!(line.trim_end(), "---" | "...") {
            return &text[end..];
        }
    }
    text
}

/// The note without the comments Obsidian hides, `%%…%%` and `<!--…-->`, which may
/// span lines. Code is left as written, and a comment that never closes stays as text
/// rather than hiding the rest of the note.
fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence = None;
    let mut line_start = true;
    let mut i = 0;
    while let Some(c) = text[i..].chars().next() {
        let rest = &text[i..];
        if line_start {
            let line = &rest[..rest.find('\n').map_or(rest.len(), |n| n + 1)];
            let code = match fence {
                Some(f) => {
                    if closes_fence(line, f) {
                        fence = None;
                    }
                    true
                }
                None => {
                    fence = opens_fence(line);
                    fence.is_some()
                }
            };
            if code {
                out.push_str(line);
                i += line.len();
                continue;
            }
        }
        let hidden = [("%%", "%%"), ("<!--", "-->")]
            .iter()
            .find_map(|(open, close)| rest.strip_prefix(open)?.find(close).map(|n| open.len() + n + close.len()));
        if let Some(n) = hidden {
            line_start = false;
            i += n;
            continue;
        }
        let copied = if c == '\\' {
            // an escaped character neither opens a comment nor starts code
            1 + rest[1..].chars().next().map_or(0, char::len_utf8)
        } else if c == '`' {
            // code on this line goes across whole
            let run = rest.len() - rest.trim_start_matches('`').len();
            let line_end = rest.find('\n').unwrap_or(rest.len());
            closing_backticks(&rest[run..line_end], run).map_or(run, |n| run + n + run)
        } else {
            c.len_utf8()
        };
        out.push_str(&rest[..copied]);
        line_start = rest[..copied].ends_with('\n');
        i += copied;
    }
    out
}

/// A code fence: "```" or "~~~", three or longer, maybe with a language after it.
fn opens_fence(line: &str) -> Option<(char, usize)> {
    let t = line.trim();
    let mark = t.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let len = t.len() - t.trim_start_matches(mark).len();
    // a backtick after a backtick fence would make it code on one line instead
    (len >= 3 && !(mark == '`' && t[len..].contains('`'))).then_some((mark, len))
}

/// The language a fence names (```` ```rust ````), empty when it names none.
fn fence_lang(line: &str) -> Lang {
    let t = line.trim();
    Lang::new(t.trim_start_matches(['`', '~']))
}

/// A line of code without the indentation its fence had (inside a list item, say), as
/// Obsidian shows it.
fn unindented<'a>(code: &'a str, fence: &str) -> &'a str {
    let indent = fence.len() - fence.trim_start().len();
    let strip = code.len() - code.trim_start_matches(' ').len();
    &code[strip.min(indent)..]
}

fn closes_fence(line: &str, (mark, len): (char, usize)) -> bool {
    let t = line.trim();
    t.len() >= len && t.chars().all(|c| c == mark)
}

/// A block before its inline Markdown is read: its kind and its lines, several when
/// wrapped lines join.
struct Raw {
    kind: BlockKind,
    lines: Vec<String>,
    /// A line of a code fence, kept exactly as written.
    code: bool,
}

impl Raw {
    fn new(kind: BlockKind, line: &str) -> Self {
        Self { kind, lines: vec![line.to_string()], code: false }
    }

    fn code(line: &str) -> Self {
        Self { kind: BlockKind::Paragraph, lines: vec![line.to_string()], code: true }
    }

    fn code_in(lang: Lang, line: &str) -> Self {
        Self { kind: BlockKind::Code(lang), lines: vec![line.to_string()], code: true }
    }

    /// The block as it reads, or nothing when no words are left (a paragraph that was
    /// only an image). A blank line of code stays, to keep the code's shape.
    fn finish(self) -> Option<Block> {
        if self.code {
            return Some(Block { kind: self.kind, text: self.lines.concat() });
        }
        let last = self.lines.len().saturating_sub(1);
        let joined = self
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| match line.trim().strip_suffix('\\') {
                // a backslash that ends a line inside a paragraph is a line break
                Some(rest) if i < last && !rest.ends_with('\\') => rest,
                _ => line.trim(),
            })
            .collect::<Vec<_>>()
            .join(" ");
        let text = without_block_id(&visible_text(&joined)).to_string();
        (!text.is_empty()).then_some(Block { kind: self.kind, text })
    }
}

/// Blocks from lines of Markdown. `in_quote`: inside a quote, where every block becomes
/// a quote line, so a list item keeps its `- ` to stay a list.
fn parse(lines: &[&str], in_quote: bool, line_breaks: bool) -> Vec<Raw> {
    let mut out = vec![];
    // the paragraph or list item that the next line may go on with
    let mut open: Option<Raw> = None;
    let mut i = 0;
    while let Some(&line) = lines.get(i) {
        i += 1;
        let t = line.trim();
        if t.is_empty() {
            out.extend(open.take());
        } else if let Some(fence) = opens_fence(line) {
            out.extend(open.take());
            if in_quote {
                // a quote holds only quote lines: its code stays as written, fences and all
                out.push(Raw::code(line));
                while let Some(&code) = lines.get(i) {
                    i += 1;
                    out.push(Raw::code(code));
                    if closes_fence(code, fence) {
                        break;
                    }
                }
                continue;
            }
            // a code block in the fence's language, to the closing fence or the end of
            // the note; two blocks in a row keep an empty line between them, or they
            // would read as one
            let lang = fence_lang(line);
            if out.last().is_some_and(|r: &Raw| r.kind == BlockKind::Code(lang)) {
                out.push(Raw::code(""));
            }
            while let Some(&code) = lines.get(i) {
                i += 1;
                if closes_fence(code, fence) {
                    break;
                }
                out.push(Raw::code_in(lang, unindented(code, line)));
            }
        } else if t.strip_prefix("$$").is_some_and(|rest| !rest.contains("$$")) && lines[i..].iter().any(|l| l.contains("$$")) {
            // a block of math, LaTeX to the closing `$$`: kept as written, like code
            out.extend(open.take());
            out.push(Raw::code(line));
            while let Some(&math) = lines.get(i) {
                i += 1;
                out.push(Raw::code(math));
                if math.contains("$$") {
                    break;
                }
            }
        } else if let Some(first) = quote_text(line) {
            out.extend(open.take());
            let mut inner = vec![first];
            while let Some(&next) = lines.get(i) {
                match quote_text(next) {
                    Some(text) => inner.push(text),
                    // a line that goes on with the quote's paragraph without its `>`
                    None if !inner.last().is_some_and(|l| l.trim().is_empty()) && !starts_block(next) => inner.push(next),
                    None => break,
                }
                i += 1;
            }
            out.extend(quote(&inner, line_breaks));
        } else if let Some((kind, text)) = atx_heading(t) {
            out.extend(open.take());
            out.push(Raw::new(kind, text));
        } else if let Some(heading) = setext_underline(t)
            .and_then(|kind| open.take_if(|p| p.kind == BlockKind::Paragraph).map(|p| Raw { kind, ..p }))
        {
            out.push(heading);
        } else if is_rule(t) {
            out.extend(open.take());
        } else if t.starts_with('|') {
            out.extend(open.take());
            if !is_table_rule(t) {
                out.push(Raw::new(BlockKind::Paragraph, t));
            }
        } else if let Some((kind, text)) = list_item(t) {
            out.extend(open.take());
            // a task is a checklist item; an empty item has no words, and the next line
            // starts afresh
            let (kind, text) = if in_quote { (kind, t) } else { task(text).map_or((kind, text), |(done, rest)| (BlockKind::Check(done), rest)) };
            if !text.is_empty() {
                open = Some(Raw::new(kind, text));
            }
        } else if let Some(p) = open.as_mut().filter(|p| p.kind != BlockKind::Paragraph || !line_breaks) {
            // a wrapped line: more of the same paragraph or list item
            p.lines.push(t.to_string());
        } else {
            out.extend(open.take());
            open = Some(Raw::new(BlockKind::Paragraph, t));
        }
    }
    out.extend(open);
    out
}

/// A quote's lines, `>` taken off, as quote blocks. A callout's lines are callout
/// lines of its kind instead, its title first when it says more than the kind's name.
fn quote(inner: &[&str], line_breaks: bool) -> Vec<Raw> {
    let head = inner.first().and_then(|line| callout_head(line));
    let body = if head.is_some() { &inner[1..] } else { inner };
    let kind = match &head {
        Some((kind, _)) => BlockKind::Callout(*kind),
        None => BlockKind::Quote,
    };
    head.and_then(|(_, title)| title)
        .map(|t| Raw::new(kind, &t))
        .into_iter()
        .chain(parse(body, true, line_breaks).into_iter().map(|r| Raw { kind, ..r }))
        .collect()
}

/// `[ ] rest` or `[x] rest` at the start of a list item's words: a task, and whether it
/// is done. Obsidian shows any mark but a space as done.
fn task(text: &str) -> Option<(bool, &str)> {
    let rest = text.strip_prefix('[')?;
    let mark = rest.chars().next()?;
    let after = rest[mark.len_utf8()..].strip_prefix(']')?;
    (after.is_empty() || after.starts_with([' ', '\t'])).then(|| (mark != ' ', after.trim()))
}

/// The rest of a `>` line, one level of quote taken off.
fn quote_text(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

/// `[!note] Title`, a callout's first line: its kind, with Obsidian's other types
/// folded into note, warning and tip (one it does not know is a note), and the title
/// Obsidian shows, when that says more than the kind's own name: the type ("Todo") when
/// there is no title. A `+` or `-` after the type only folds it.
fn callout_head(line: &str) -> Option<(Callout, Option<String>)> {
    let (name, rest) = line.trim().strip_prefix("[!")?.split_once(']')?;
    let mut chars = name.chars();
    let first = chars.next()?;
    let kind = Callout::from_obsidian(name).unwrap_or(Callout::Note);
    let title = rest.trim_start_matches(['+', '-']).trim();
    let shown: String = if title.is_empty() { first.to_uppercase().chain(chars).collect() } else { title.to_string() };
    Some((kind, (!shown.eq_ignore_ascii_case(kind.label())).then_some(shown)))
}

/// `# Heading` to `###### Heading`, without closing `#`s. Inkling's headings go three
/// levels deep, so deeper ones become the third.
fn atx_heading(t: &str) -> Option<(BlockKind, &str)> {
    let level = t.len() - t.trim_start_matches('#').len();
    let rest = &t[level..];
    if !(1..=6).contains(&level) || !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    let rest = rest.trim();
    let open = rest.trim_end_matches('#');
    let text = if open.is_empty() || open.ends_with([' ', '\t']) { open.trim_end() } else { rest };
    let kind = match level {
        1 => BlockKind::H1,
        2 => BlockKind::H2,
        _ => BlockKind::H3,
    };
    Some((kind, text))
}

/// A line that makes the paragraph above it a heading: `===` for the first level, `---`
/// for the second. Three or more, as headings are written; a lone `-` under a line is
/// an empty list item, not a heading.
fn setext_underline(t: &str) -> Option<BlockKind> {
    let kind = match t.chars().next()? {
        '=' => BlockKind::H1,
        '-' => BlockKind::H2,
        _ => return None,
    };
    (t.len() >= 3 && t.chars().all(|c| c == t.as_bytes()[0] as char)).then_some(kind)
}

/// `---`, `***` or `___`, spaces allowed between: a rule, which has no words to keep.
fn is_rule(t: &str) -> bool {
    let marks: Vec<char> = t.chars().filter(|c| !c.is_whitespace()).collect();
    marks.len() >= 3 && matches!(marks[0], '-' | '*' | '_') && marks.iter().all(|c| *c == marks[0])
}

/// `|---|:--:|`, the line under a table's header.
fn is_table_rule(t: &str) -> bool {
    t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

/// `- item`, `* item`, `+ item`, `1. item` or `1) item`, at any depth (Inkling's lists
/// are flat): the kind and the item's words.
fn list_item(t: &str) -> Option<(BlockKind, &str)> {
    fn item(rest: &str) -> Option<&str> {
        (rest.is_empty() || rest.starts_with([' ', '\t'])).then(|| rest.trim())
    }
    if let Some(rest) = t.strip_prefix(['-', '*', '+']) {
        return item(rest).map(|text| (BlockKind::Bullet, text));
    }
    let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let rest = t[digits..].strip_prefix(['.', ')'])?;
    (1..=9).contains(&digits).then(|| item(rest)).flatten().map(|text| (BlockKind::Numbered, text))
}

/// Does `line` start a block of its own, so it cannot go on with a quote's paragraph?
fn starts_block(line: &str) -> bool {
    let t = line.trim();
    t.is_empty()
        || opens_fence(line).is_some()
        || quote_text(line).is_some()
        || atx_heading(t).is_some()
        || is_rule(t)
        || t.starts_with('|')
        || list_item(t).is_some()
}

/// The text without a trailing Obsidian block id (` ^abc-123`), which Obsidian hides.
fn without_block_id(text: &str) -> &str {
    match text.rsplit_once('^') {
        Some((before, id))
            if !id.is_empty()
                && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && (before.is_empty() || before.ends_with(' ')) =>
        {
            before.trim_end()
        }
        _ => text,
    }
}

// ----- inline Markdown -----------------------------------------------------------------

/// A character of a paragraph, and whether it may be an emphasis mark: a `*`, `_`, `~`
/// or `=` written as itself, not escaped and not in code, math, a link target or a URL.
type Char = (char, bool);

/// What a paragraph of Markdown shows as words. Links become their text; embeds and
/// images go; the marks of emphasis, highlight (`==`), strikethrough (`~~`) and code go
/// while their words stay; math stays as written; escapes are undone; runs of spaces
/// become one.
fn visible_text(markdown: &str) -> String {
    let mut chars = vec![];
    scan(markdown, &mut chars);
    let keep = unpaired_marks(&chars);
    let mut out = String::with_capacity(markdown.len());
    for (&(c, _), keep) in chars.iter().zip(keep) {
        if !keep {
            continue;
        }
        if c.is_ascii_whitespace() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out.truncate(out.trim_end_matches(' ').len());
    out
}

fn scan(s: &str, out: &mut Vec<Char>) {
    let words = |out: &mut Vec<Char>, text: &str| out.extend(text.chars().map(|c| (c, false)));
    let mut i = 0;
    while let Some(c) = s[i..].chars().next() {
        let rest = &s[i..];
        i += if let Some(escaped) = rest.strip_prefix('\\').and_then(|r| r.chars().next()).filter(char::is_ascii_punctuation) {
            // `\*` is a star, never a mark
            out.push((escaped, false));
            1 + escaped.len_utf8()
        } else if c == '`' {
            // code shows its text as written
            let run = rest.len() - rest.trim_start_matches('`').len();
            match closing_backticks(&rest[run..], run) {
                Some(n) => {
                    words(out, &rest[run..run + n]);
                    run + n + run
                }
                None => {
                    words(out, &rest[..run]);
                    run
                }
            }
        } else if let Some(n) = math(rest) {
            // LaTeX: its backslashes and marks are its own
            words(out, &rest[..n]);
            n
        } else if let Some(n) = rest.strip_prefix("![[").and_then(|r| r.find("]]")) {
            // an embed shows a file, not words
            3 + n + 2
        } else if let Some(n) = rest.strip_prefix("[[").and_then(|r| r.find("]]")) {
            words(out, &wiki_text(&rest[2..2 + n]));
            2 + n + 2
        } else if let Some((_, n)) = rest.strip_prefix('!').and_then(link) {
            // an image
            1 + n
        } else if let Some((text, n)) = link(rest) {
            scan(text, out);
            n
        } else if let Some((address, n)) = autolink(rest) {
            words(out, address);
            n
        } else if let Some(n) = bare_url(rest, out.last().map(|&(c, _)| c)) {
            words(out, &rest[..n]);
            n
        } else {
            out.push((c, matches!(c, '*' | '_' | '~' | '=')));
            c.len_utf8()
        };
    }
}

/// `$$…$$` or `$…$` at the start of `s`, math: its length, signs included. As in Pandoc,
/// `$` math opens before a non-space and closes after one, not right before a digit,
/// so "$5 and $10" stays money.
fn math(s: &str) -> Option<usize> {
    if let Some(rest) = s.strip_prefix("$$") {
        return rest.find("$$").map(|n| 2 + n + 2);
    }
    let rest = s.strip_prefix('$')?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        return None;
    }
    let mut before = '$';
    for (i, c) in rest.char_indices() {
        if c == '$' && !before.is_whitespace() && before != '\\' && !rest[i + 1..].starts_with(|d: char| d.is_ascii_digit()) {
            return Some(1 + i + 1);
        }
        before = c;
    }
    None
}

/// Where a run of exactly `len` backticks starts in `s`.
fn closing_backticks(s: &str, len: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let run = bytes[i..].iter().take_while(|b| **b == b'`').count();
        if run == len {
            return Some(i);
        }
        i += run.max(1);
    }
    None
}

/// What Obsidian shows for `[[inner]]`: the label after `|`, else the target, with a
/// heading in it written "Page > Heading". A block reference (`#^id`) shows the page.
fn wiki_text(inner: &str) -> String {
    // a link inside a table escapes its `|`
    let inner = inner.replace("\\|", "|");
    let (target, label) = inner.split_once('|').unwrap_or((inner.as_str(), ""));
    if !label.trim().is_empty() {
        return label.trim().to_string();
    }
    target.split('#').map(str::trim).filter(|part| !part.is_empty() && !part.starts_with('^')).collect::<Vec<_>>().join(" > ")
}

/// `[text](target)` at the start of `s`: the text, and the length of the whole link.
fn link(s: &str) -> Option<(&str, usize)> {
    let close = matching(s, '[', ']')?;
    let target = matching(&s[close + 1..], '(', ')')?;
    Some((&s[1..close], close + 1 + target + 1))
}

/// Where the bracket that `s` starts with closes, counting nested pairs and skipping
/// escaped brackets.
fn matching(s: &str, open: char, close: char) -> Option<usize> {
    if !s.starts_with(open) {
        return None;
    }
    let mut depth = 0;
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// `<https://…>` or `<name@example.com>`: the address it shows.
fn autolink(s: &str) -> Option<(&str, usize)> {
    let rest = s.strip_prefix('<')?;
    let inner = &rest[..rest.find('>')?];
    let scheme = inner.split_once(':').is_some_and(|(scheme, _)| {
        scheme.len() >= 2
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    });
    let address = !inner.is_empty() && !inner.contains(|c: char| c.is_whitespace() || c == '<');
    (address && (scheme || inner.contains('@'))).then_some((inner, inner.len() + 2))
}

/// A bare web address at the start of `s`, which Obsidian shows as a link: its length,
/// without punctuation that ends the sentence after it. Nothing in it is a mark, so an
/// `_` in an address is not emphasis.
fn bare_url(s: &str, before: Option<char>) -> Option<usize> {
    let scheme = ["https://", "http://"].into_iter().find(|p| s.starts_with(p))?;
    if before.is_some_and(char::is_alphanumeric) {
        return None;
    }
    let end = s.find(|c: char| c.is_whitespace() || c == '<').unwrap_or(s.len());
    let url = s[..end].trim_end_matches(['.', ',', ':', ';', '!', '?', '*', '_', '~', '=', ')', '\'', '"']);
    (url.len() > scheme.len()).then_some(url.len())
}

/// Which characters stay: the marks that open and close an emphasis, highlight or
/// strikethrough go, and every other `*`, `_`, `~` and `=` is text, as in `snake_case`
/// or `2 * 3`. CommonMark's flanking rules, without its rarest cases.
fn unpaired_marks(chars: &[Char]) -> Vec<bool> {
    struct Run {
        mark: char,
        start: usize,
        len: usize,
        opens: bool,
        closes: bool,
        /// marks paired as an opener, taken from the run's end
        opened: usize,
        /// marks paired as a closer, taken from the run's start
        closed: usize,
    }
    impl Run {
        fn left(&self) -> usize {
            self.len - self.opened - self.closed
        }
    }
    let punctuation = |c: char| !c.is_alphanumeric() && !c.is_whitespace();
    let mut runs: Vec<Run> = vec![];
    let mut i = 0;
    while i < chars.len() {
        let (mark, is_mark) = chars[i];
        if !is_mark {
            i += 1;
            continue;
        }
        let len = chars[i..].iter().take_while(|&&(c, m)| c == mark && m).count();
        let before = i.checked_sub(1).map_or(' ', |j| chars[j].0);
        let after = chars.get(i + len).map_or(' ', |c| c.0);
        let left = !after.is_whitespace() && (!punctuation(after) || before.is_whitespace() || punctuation(before));
        let right = !before.is_whitespace() && (!punctuation(before) || after.is_whitespace() || punctuation(after));
        let (opens, closes) = match mark {
            '*' => (left, right),
            // inside a word, `_` is a letter
            '_' => (left && (!right || punctuation(before)), right && (!left || punctuation(after))),
            // `~~` and `==`, exactly two
            _ if len == 2 => (left, right),
            _ => (false, false),
        };
        runs.push(Run { mark, start: i, len, opens, closes, opened: 0, closed: 0 });
        i += len;
    }
    let mut keep = vec![true; chars.len()];
    let mut openers: Vec<usize> = vec![];
    for r in 0..runs.len() {
        while runs[r].closes && runs[r].left() > 0 {
            let Some(at) = openers.iter().rposition(|&o| runs[o].mark == runs[r].mark && runs[o].left() > 0) else {
                break;
            };
            let o = openers[at];
            let n = if runs[o].left() >= 2 && runs[r].left() >= 2 { 2 } else { 1 };
            // the opener's marks nearest the words, and the closer's
            let end = runs[o].start + runs[o].len - runs[o].opened;
            keep[end - n..end].fill(false);
            let start = runs[r].start + runs[r].closed;
            keep[start..start + n].fill(false);
            runs[o].opened += n;
            runs[r].closed += n;
            // marks between the two can no longer pair
            openers.truncate(at + 1);
            if runs[o].left() == 0 {
                openers.pop();
            }
        }
        if runs[r].opens && runs[r].left() > 0 {
            openers.push(r);
        }
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;
    use BlockKind::*;

    fn b(kind: BlockKind, text: &str) -> Block {
        Block { kind, text: text.into() }
    }

    /// The body of `markdown` as plain Markdown, where wrapped lines join.
    fn body(markdown: &str) -> Vec<Block> {
        convert(markdown, "File name", false)[1..].to_vec()
    }

    /// The text of the one block `markdown` makes.
    fn line(markdown: &str) -> String {
        let blocks = body(markdown);
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        blocks[0].text.clone()
    }

    /// A folder under the system temp dir, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(what: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("inkling-import-{what}-{}", new_id()));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn write(&self, path: &str, text: &str) -> PathBuf {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[std::prelude::v1::test]
    fn the_title_is_an_opening_heading_or_the_file_name() {
        let blocks = convert("# Rate limiter\nOne counter per user.", "rate-limiter", false);
        assert_eq!(blocks, vec![b(Title, "Rate limiter"), b(Paragraph, "One counter per user.")]);

        // a heading further down heads a section and stays where it is
        let blocks = convert("Intro.\n\n# Section\nText.", "Meeting", false);
        assert_eq!(blocks, vec![b(Title, "Meeting"), b(Paragraph, "Intro."), b(H1, "Section"), b(Paragraph, "Text.")]);
        assert_eq!(convert("## Not a title\nText.", "Name", false)[..2], [b(Title, "Name"), b(H2, "Not a title")]);

        // its marks and links go; an underlined title counts too
        assert_eq!(convert("# **Loose** [[Threads|threads]]", "x", false)[0], b(Title, "Loose threads"));
        assert_eq!(convert("Old plan\n===\n\nDropped.", "x", false), vec![b(Title, "Old plan"), b(Paragraph, "Dropped.")]);

        // an empty file is a note with an empty first paragraph, like a new one
        assert_eq!(convert("", " Empty ", false), vec![b(Title, "Empty"), b(Paragraph, "")]);
        assert_eq!(convert("# Only a title", "x", false), vec![b(Title, "Only a title"), b(Paragraph, "")]);
    }

    #[std::prelude::v1::test]
    fn front_matter_is_dropped() {
        let note = "---\ntags: [a, b]\ntitle: Not this one\n---\n# Real title\nBody.";
        assert_eq!(convert(note, "x", false), vec![b(Title, "Real title"), b(Paragraph, "Body.")]);
        assert_eq!(convert("---\ncssclass: wide\n...\nBody.", "x", false)[1], b(Paragraph, "Body."));
        // only at the very top, and only when it closes
        assert_eq!(body("Above.\n\n---\nkey: value\n---\n"), vec![b(Paragraph, "Above."), b(H2, "key: value")]);
        assert_eq!(body("---\nnot: closed"), vec![b(Paragraph, "not: closed")]);
        // Windows line ends and a byte order mark
        assert_eq!(convert("\u{feff}---\r\na: 1\r\n---\r\n# T\r\nOne\r\ntwo", "x", false), vec![b(Title, "T"), b(Paragraph, "One two")]);
    }

    #[std::prelude::v1::test]
    fn block_kinds() {
        let note = "## Two\n### Three\n#### Four\n###### Six ######\n#tag is not a heading\n\n\
                    - dash\n* star\n+ plus\n1. one\n2) two\n\n> quoted\n\nplain";
        assert_eq!(
            body(note),
            vec![
                b(H2, "Two"),
                b(H3, "Three"),
                b(H3, "Four"),
                b(H3, "Six"),
                b(Paragraph, "#tag is not a heading"),
                b(Bullet, "dash"),
                b(Bullet, "star"),
                b(Bullet, "plus"),
                b(Numbered, "one"),
                b(Numbered, "two"),
                b(Quote, "quoted"),
                b(Paragraph, "plain"),
            ]
        );
        // nested lists flatten, tasks become checklist items, empty items go
        assert_eq!(
            body("- a\n    - b\n\t- c\n1. d\n   1. e\n- [ ] todo\n- [x] done\n* [X] **also** done\n2. [ ] numbered task\n- [/] half, shown done\n- [ ]\n-\n- \nAfter."),
            vec![
                b(Bullet, "a"),
                b(Bullet, "b"),
                b(Bullet, "c"),
                b(Numbered, "d"),
                b(Numbered, "e"),
                b(Check(false), "todo"),
                b(Check(true), "done"),
                b(Check(true), "also done"),
                b(Check(false), "numbered task"),
                b(Check(true), "half, shown done"),
                b(Paragraph, "After."),
            ]
        );
        // only a box and a space make a task
        assert_eq!(body("- [link](x) and [x]y"), vec![b(Bullet, "link and [x]y")]);
        // things that only look like list items
        assert_eq!(line("-5 degrees and +1 for 3.14, 2026-11-06"), "-5 degrees and +1 for 3.14, 2026-11-06");
        assert_eq!(line("**Bold** at the start"), "Bold at the start");
    }

    #[std::prelude::v1::test]
    fn wrapped_lines_join_unless_the_vault_shows_line_breaks() {
        let note = "One line\nwraps here.\n\n- an item\n  goes on\n> a\n> quote";
        assert_eq!(body(note), vec![b(Paragraph, "One line wraps here."), b(Bullet, "an item goes on"), b(Quote, "a quote")]);
        assert_eq!(
            convert(note, "x", true)[1..],
            [b(Paragraph, "One line"), b(Paragraph, "wraps here."), b(Bullet, "an item goes on"), b(Quote, "a"), b(Quote, "quote")]
        );
        // a backslash at the end of a line is a line break, not a word
        assert_eq!(line("First\\\nsecond"), "First second");
    }

    #[std::prelude::v1::test]
    fn quotes_and_callouts() {
        assert_eq!(body("> one\n>\n> two"), vec![b(Quote, "one"), b(Quote, "two")]);
        // a line without `>` goes on with the quote's paragraph, as in Obsidian
        assert_eq!(body("> starts here\nand goes on\n\nAfter."), vec![b(Quote, "starts here and goes on"), b(Paragraph, "After.")]);
        assert_eq!(body("> > nested\n> - item\n> # Heading"), vec![b(Quote, "nested"), b(Quote, "- item"), b(Quote, "Heading")]);
        // callouts: a title that says more than the kind stays as the first line
        let (note, warning, tip) = (Callout(crate::store::Callout::Note), Callout(crate::store::Callout::Warning), Callout(crate::store::Callout::Tip));
        assert_eq!(body("> [!tip] Why left alone\n> No model calls."), vec![b(tip, "Why left alone"), b(tip, "No model calls.")]);
        assert_eq!(body("> [!warning]-\n> Careful."), vec![b(warning, "Careful.")]);
        assert_eq!(body("> [!NOTE]\n> One.\n>\n> Two."), vec![b(note, "One."), b(note, "Two.")]);
        // Obsidian's other types fold into the three, and keep the name they showed
        assert_eq!(body("> [!todo]+ This week\n> - write it\n> - test it"), vec![b(note, "This week"), b(note, "- write it"), b(note, "- test it")]);
        assert_eq!(body("> [!question]\n> Why?"), vec![b(note, "Question"), b(note, "Why?")]);
        assert_eq!(body("> [!danger] Hot\n> Do not touch."), vec![b(warning, "Hot"), b(warning, "Do not touch.")]);
        assert_eq!(body("> [!bug]\n> x"), vec![b(warning, "Bug"), b(warning, "x")]);
        assert_eq!(body("> [!hint]\n> y"), vec![b(tip, "Hint"), b(tip, "y")]);
        assert_eq!(body("> [!done]\n> z"), vec![b(tip, "Done"), b(tip, "z")]);
        assert_eq!(body("> [!my-own] Mine\n> w"), vec![b(note, "Mine"), b(note, "w")]);
    }

    #[std::prelude::v1::test]
    fn links_show_their_text_and_embeds_go() {
        assert_eq!(
            line("[[Page]], [[Page|the label]], [[Page#Heading]], [[#Local]], [[Page#^abc123]], [[Folder/Page]]"),
            "Page, the label, Page > Heading, Local, Page, Folder/Page"
        );
        assert_eq!(line("Before ![[sketch.png]] and ![[Other note#Part]] after"), "Before and after");
        assert_eq!(body("Text.\n\n![[only an embed.png]]\n\n![](assets/pic.png)"), vec![b(Paragraph, "Text.")]);
        assert_eq!(
            line("Read [the **docs**](https://x.com/a_(b)) and ![alt](i.png) at <https://example.com> or <me@example.com>."),
            "Read the docs and at https://example.com or me@example.com."
        );
        assert_eq!(line("[see [[X]]](url) and [not a link] [^1] <br>"), "see X and [not a link] [^1] <br>");
        assert_eq!(body("| [[Page\\|label]] | **x** |\n|---|:-:|\n| 1 | 2 |"), vec![b(Paragraph, "| label | x |"), b(Paragraph, "| 1 | 2 |")]);
    }

    #[std::prelude::v1::test]
    fn marks_go_and_their_words_stay() {
        assert_eq!(line("**bold** __bold__ *it* _it_ ***both*** ~~gone~~ ==lit== `code`"), "bold bold it it both gone lit code");
        assert_eq!(line("*Note:* this, and a**b**c"), "Note: this, and abc");
        // not marks
        assert_eq!(line("snake_case_name, 2 * 3 * 4, a == b, ~5 min, un**finished"), "snake_case_name, 2 * 3 * 4, a == b, ~5 min, un**finished");
        assert_eq!(line("https://example.com/a_b_c_d and _italic_"), "https://example.com/a_b_c_d and italic");
        assert_eq!(line("**see https://example.com/x_y**, then"), "see https://example.com/x_y, then");
        // code keeps what is inside it
        assert_eq!(line("Use `**not bold** [[not a link]]` or ``a `tick` b``"), "Use **not bold** [[not a link]] or a `tick` b");
        assert_eq!(line("\\*not italic\\* and \\[\\[x\\]\\] and \\a"), "*not italic* and [[x]] and \\a");
        assert_eq!(line("spaces   and\ttabs  collapse "), "spaces and tabs collapse");
    }

    #[std::prelude::v1::test]
    fn code_fences_become_code_blocks_with_their_lines_as_written() {
        let rust = Code(Lang::new("rust"));
        let note = "Before.\n```rust\nfn main() {\n\n    let x = \"**not bold** [[not a link]] %% not a comment %%\";\n}\n```\nAfter.";
        assert_eq!(
            body(note),
            vec![
                b(Paragraph, "Before."),
                b(rust, "fn main() {"),
                b(rust, ""),
                b(rust, "    let x = \"**not bold** [[not a link]] %% not a comment %%\";"),
                b(rust, "}"),
                b(Paragraph, "After."),
            ]
        );
        // tildes, a fence inside a list item (its indentation goes), and one that never
        // closes; two blocks in a row keep an empty line between them
        let plain = Code(Lang::default());
        assert_eq!(
            body("- item\n  ~~~\n  # code\n    indented\n  ~~~\n```\nrest of the note\n# still code"),
            vec![
                b(Bullet, "item"),
                b(plain, "# code"),
                b(plain, "  indented"),
                b(Paragraph, ""),
                b(plain, "rest of the note"),
                b(plain, "# still code"),
            ]
        );
        assert_eq!(body("```TypeScript {1,3}\nconst a = 1\n```"), vec![b(Code(Lang::new("typescript")), "const a = 1")]);
        assert_eq!(body("```\n```\nAfter."), vec![b(Paragraph, "After.")], "an empty block has no lines");
        // a code fence in a quote stays a quote
        assert_eq!(body("> ```\n> x = 1\n> ```"), vec![b(Quote, "```"), b(Quote, "x = 1"), b(Quote, "```")]);
    }

    #[std::prelude::v1::test]
    fn math_is_kept_as_written() {
        // LaTeX backslashes and underscores are not Markdown escapes or marks
        assert_eq!(
            line("Area $\\pi r^2$, $a_1 * b_1$ and $$\\int_0^1 x\\,dx$$; \\$5 and $10, $5/$6 and $ alone are money"),
            "Area $\\pi r^2$, $a_1 * b_1$ and $$\\int_0^1 x\\,dx$$; $5 and $10, $5/$6 and $ alone are money"
        );
        assert_eq!(
            body("So\n$$\n\\frac{a}{b} \\, \\text{**as written**}\n$$\nthen."),
            vec![b(Paragraph, "So"), b(Paragraph, "$$"), b(Paragraph, "\\frac{a}{b} \\, \\text{**as written**}"), b(Paragraph, "$$"), b(Paragraph, "then.")]
        );
        // `$$` that never closes is only text
        assert_eq!(body("$$ is slang for **money**\nand that is all"), vec![b(Paragraph, "$$ is slang for money and that is all")]);
    }

    #[std::prelude::v1::test]
    fn hidden_things_stay_hidden() {
        assert_eq!(line("Shown %%hidden%% text. ^block-1"), "Shown text.");
        assert_eq!(body("Para one.\n%%\nhidden\nlines\n%%\nPara two."), vec![b(Paragraph, "Para one."), b(Paragraph, "Para two.")]);
        assert_eq!(line("<!-- a note to self -->Visible"), "Visible");
        // a comment that never closes, or sits in code, stays as text
        assert_eq!(line("Use `%%` for a percent sign.\nAt 50%% it stays."), "Use %% for a percent sign. At 50%% it stays.");
        assert_eq!(body("Text.\n\n^abc123"), vec![b(Paragraph, "Text.")]);
        assert_eq!(line("x^2 and a ^ b stay"), "x^2 and a ^ b stay");
    }

    #[std::prelude::v1::test]
    fn rules_tables_and_underlined_headings() {
        assert_eq!(body("Above.\n\n---\n\nBelow.\n***\n_ _ _"), vec![b(Paragraph, "Above."), b(Paragraph, "Below.")]);
        assert_eq!(body("Heading\n---\nText."), vec![b(H2, "Heading"), b(Paragraph, "Text.")]);
        assert_eq!(body("- item\n---"), vec![b(Bullet, "item")]);
        assert_eq!(body("| A | B |\n| --- | :-: |\n| **1** | 2 |"), vec![b(Paragraph, "| A | B |"), b(Paragraph, "| 1 | 2 |")]);
    }

    #[std::prelude::v1::test]
    fn blocks_never_hold_a_line_break() {
        // the editor would turn one into a space, and the hash would no longer match
        let note = "a\nb\n> c\nd\n- e\n  f\n```\ng\n```\n%%\nh\n%%";
        for line_breaks in [false, true] {
            for block in convert(note, "x", line_breaks) {
                assert!(!block.text.contains(['\n', '\r']), "{block:?}");
            }
        }
    }

    /// A vault on disk: settings, trash, hidden files, links, iCloud placeholders,
    /// drawings and attachments beside the notes.
    fn sample_vault() -> TempDir {
        let vault = TempDir::new("vault");
        vault.write(".obsidian/app.json", r#"{"strictLineBreaks": true}"#);
        vault.write(".obsidian/plugins/sample/README.md", "# Plugin readme");
        vault.write(".trash/Deleted.md", "# Deleted");
        vault.write(".hidden.md", "hidden");
        vault.write(".Cloud only.md.icloud", "placeholder");
        vault.write("Top.md", "Line one\nline two");
        vault.write("Board.canvas", "{}");
        vault.write("attachments/pic.png", "not really a png");
        vault.write("Projects/Plan.md", "# Plan\n- [[Top|the top note]]");
        vault.write("Projects/Deep/Dive.MD", "Deep.");
        vault.write("Projects/Drawing 1.excalidraw.md", "# Excalidraw Data");
        std::os::unix::fs::symlink(vault.0.join("Top.md"), vault.0.join("Link.md")).unwrap();
        std::os::unix::fs::symlink(&vault.0, vault.0.join("Loop")).unwrap();
        vault
    }

    /// Every path in `dir`, with its size and modification time.
    fn listing(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
        let mut all = vec![];
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let meta = fs::symlink_metadata(entry.path()).unwrap();
            all.push((entry.path(), meta.len(), meta.modified().unwrap()));
            if meta.is_dir() {
                all.extend(listing(&entry.path()));
            }
        }
        all.sort();
        all
    }

    #[std::prelude::v1::test]
    fn reading_a_vault_takes_only_its_notes_and_writes_nothing() {
        let vault = sample_vault();
        let before = listing(&vault.0);
        let read = read_vault(&vault.0).unwrap();
        let found: Vec<(Vec<String>, String)> = read.notes.iter().map(|n| (n.folders.clone(), n.blocks[0].text.clone())).collect();
        let folders = |f: &[&str]| f.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // folder by folder, each before the folders inside it
        assert_eq!(
            found,
            vec![
                (folders(&[]), "Top".to_string()),
                (folders(&["Projects"]), "Plan".to_string()),
                (folders(&["Projects", "Deep"]), "Dive".to_string()),
            ]
        );
        assert_eq!((read.in_icloud, read.unreadable), (1, 0));
        let blocks = |title: &str| &read.notes.iter().find(|n| n.blocks[0].text == title).unwrap().blocks;
        // "Strict line breaks" is on in this vault, so wrapped lines join
        assert_eq!(blocks("Top")[1], b(Paragraph, "Line one line two"));
        assert_eq!(blocks("Plan")[1], b(Bullet, "the top note"));
        assert!(read.notes.iter().all(|n| n.created.is_some() && n.modified.is_some()));
        assert_eq!(listing(&vault.0), before, "the vault must be left exactly as it was");

        assert!(read_vault(&vault.0.join("Top.md")).is_err(), "a file is not a vault");
        assert!(read_vault(&vault.0.join("missing")).is_err());
    }

    #[std::prelude::v1::test]
    fn line_breaks_follow_the_vault_setting() {
        let dir = TempDir::new("breaks");
        assert!(!shows_line_breaks(&dir.0), "a folder never opened in Obsidian is plain Markdown");
        fs::create_dir_all(dir.0.join(".obsidian")).unwrap();
        assert!(shows_line_breaks(&dir.0), "Obsidian shows line breaks unless told not to");
        dir.write(".obsidian/app.json", r#"{"strictLineBreaks": true}"#);
        assert!(!shows_line_breaks(&dir.0));
        dir.write(".obsidian/app.json", r#"{"strictLineBreaks": false, "other": 1}"#);
        assert!(shows_line_breaks(&dir.0));
        dir.write(".obsidian/app.json", "{ not json");
        assert!(shows_line_breaks(&dir.0));
    }

    #[std::prelude::v1::test]
    fn importing_adds_notes_and_folders_and_leaves_their_words_alone() {
        let vault = sample_vault();
        let modified = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        fs::File::options().write(true).open(vault.0.join("Top.md")).unwrap().set_modified(modified).unwrap();
        let notebook = TempDir::new("notebook");
        let mut store = Store::open_at(notebook.0.clone()).unwrap();
        store.folders.push(Folder { id: "mine".into(), name: "projects".into(), collapsed: false });

        let summary = add_to_store(&mut store, read_vault(&vault.0).unwrap(), Mode::WhenYouAsk);
        assert_eq!(summary, Summary { imported: 3, already_here: 0, unreadable: 0, in_icloud: 1 });
        assert_eq!(summary.message("Vault"), "Imported 3 notes from Vault; 1 note only in iCloud");

        // "Projects" goes into the folder already called "projects"; the one below it is new and folded
        let names: Vec<(&str, bool)> = store.folders.iter().map(|f| (f.name.as_str(), f.collapsed)).collect();
        assert_eq!(names, vec![("projects", false), ("Projects / Deep", true)]);
        let note = |title: &str| store.notes.iter().find(|n| n.title == title).unwrap();
        assert_eq!(note("Plan").folder.as_deref(), Some("mine"));
        assert_eq!(note("Dive").folder.as_deref(), Some(store.folders[1].id.as_str()));
        assert_eq!(note("Top").folder, None);
        assert_eq!(note("Top").updated_at, DateTime::<Utc>::from(modified));
        assert_eq!(note("Top").mode, Mode::WhenYouAsk);

        // every imported paragraph is left alone, as the page hashes it; the title is never read
        for title in ["Plan", "Dive", "Top"] {
            let n = note(title);
            let body: Vec<u64> = n.blocks[1..].iter().map(|b| hash_text(b.text.trim())).collect();
            assert_eq!(n.left_alone, body, "{title}");
            assert!(n.read_hashes.is_empty() && !n.first_run);
        }

        // on disk, and in the order the notebook opens in
        let reopened = Store::open_at(notebook.0.clone()).unwrap();
        assert_eq!(reopened.notes.len(), 4);
        assert_eq!(reopened.folders.len(), 2);
        let order = |s: &Store| s.notes.iter().map(|n| n.title.clone()).collect::<Vec<_>>();
        assert_eq!(order(&reopened), order(&store));
        assert_eq!(store.notes.last().unwrap().title, "Top", "the oldest note goes last");

        // a second import adds only what changed
        store.notes.iter_mut().find(|n| n.title == "Plan").unwrap().blocks.push(b(Paragraph, "Written in Inkling."));
        let again = add_to_store(&mut store, read_vault(&vault.0).unwrap(), Mode::WhenYouAsk);
        assert_eq!(again, Summary { imported: 1, already_here: 2, unreadable: 0, in_icloud: 1 });
        assert_eq!(store.folders.len(), 2);
        assert_eq!(store.notes.iter().filter(|n| n.title == "Plan").count(), 2);
    }

    #[std::prelude::v1::test]
    fn messages() {
        let s = |imported, already_here, unreadable, in_icloud| Summary { imported, already_here, unreadable, in_icloud };
        assert_eq!(s(42, 0, 0, 0).message("Vault"), "Imported 42 notes from Vault");
        assert_eq!(s(1, 2, 0, 0).message("Vault"), "Imported 1 note from Vault; 2 already here");
        assert_eq!(s(0, 1, 0, 0).message("Vault"), "Nothing new in Vault: 1 note already here");
        assert_eq!(s(0, 0, 1, 2).message("Vault"), "No notes found in Vault; 2 notes only in iCloud; 1 could not be read");
    }
}

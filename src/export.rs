//! Export a note as Markdown, HTML or Word. Threads (the questions in the margin) can sit under
//! the paragraph they are about. Word goes through macOS `textutil`, which turns
//! HTML into .docx without any extra dependency.
//!
//! Checklists are `- [ ]` items in Markdown, checkboxes in HTML and ☐ ☑ in Word; a
//! callout is Obsidian's `> [!note]`, a box in its thread in HTML, and a quote headed by
//! its kind in Word; code is fenced with its language, or `<pre><code>`.

use crate::store::{BlockKind, Callout, Mark, MarkSpan, Note, Role, Thread, ThreadStatus, markdown_blocks};

/// A mark as an export writes it, in the order marks nest (outermost first), so a stack
/// of open marks closes and reopens cleanly wherever the set changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Inline {
    Bold,
    Italic,
    Strike,
    Underline,
    Color(u32),
    Code,
}

impl Inline {
    fn of(m: Mark) -> Option<Inline> {
        Some(match m {
            Mark::Bold => Inline::Bold,
            Mark::Italic => Inline::Italic,
            Mark::Strike => Inline::Strike,
            Mark::Underline => Inline::Underline,
            Mark::Color(0) => return None,
            Mark::Color(c) => Inline::Color(c),
            Mark::Code => Inline::Code,
        })
    }

    fn rank(self) -> u8 {
        match self {
            Inline::Bold => 0,
            Inline::Italic => 1,
            Inline::Strike => 2,
            Inline::Underline => 3,
            Inline::Color(_) => 4,
            Inline::Code => 5,
        }
    }
}

/// Byte offset of each block in the note's joined text (blocks joined by '\n').
fn block_starts(note: &Note) -> Vec<usize> {
    let mut at = 0;
    note.blocks
        .iter()
        .map(|b| {
            let start = at;
            at += b.text.len() + 1;
            start
        })
        .collect()
}

/// `text` (a block starting at byte `base` of the joined text) cut wherever one of
/// `marks` starts or ends: each piece with the marks on it, outermost first.
fn pieces<'a>(text: &'a str, base: usize, marks: &[MarkSpan]) -> Vec<(&'a str, Vec<Inline>)> {
    let end = base + text.len();
    let here: Vec<&MarkSpan> = marks.iter().filter(|m| m.start < end && m.end > base).collect();
    let mut cuts = vec![0, text.len()];
    for m in &here {
        cuts.push(m.start.max(base) - base);
        cuts.push(m.end.min(end) - base);
    }
    cuts.retain(|c| text.is_char_boundary(*c));
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .map(|w| {
            let (a, b) = (base + w[0], base + w[1]);
            let mut on: Vec<Inline> = here.iter().filter(|m| m.start <= a && m.end >= b).filter_map(|m| Inline::of(m.mark)).collect();
            on.sort_by_key(|m| m.rank());
            on.dedup_by_key(|m| m.rank());
            (&text[w[0]..w[1]], on)
        })
        .collect()
}

/// Write `text` with its marks: `open`/`close` give each mark's tokens and `escape`
/// the words. With `hug`, tokens never touch whitespace on their inner side (Markdown
/// reads "** bold**" as plain asterisks), so whitespace at a piece's edges moves out.
fn inline_text(
    text: &str,
    base: usize,
    marks: &[MarkSpan],
    open: impl Fn(Inline) -> String,
    close: impl Fn(Inline) -> String,
    escape: impl Fn(&str) -> String,
    hug: bool,
) -> String {
    let mut out = String::new();
    let mut stack: Vec<Inline> = vec![];
    let mut pending = String::new();
    for (piece, want) in pieces(text, base, marks) {
        let (lead, core, trail) = if hug {
            let core = piece.trim();
            if core.is_empty() {
                pending.push_str(piece);
                continue;
            }
            let lead = &piece[..piece.len() - piece.trim_start().len()];
            (lead, core, &piece[lead.len() + core.len()..])
        } else {
            ("", piece, "")
        };
        let keep = stack.iter().zip(&want).take_while(|(a, b)| a == b).count();
        while stack.len() > keep {
            let m = stack.pop().expect("a mark to close");
            out.push_str(&close(m));
        }
        out.push_str(&pending);
        pending.clear();
        out.push_str(lead);
        for m in &want[keep..] {
            out.push_str(&open(*m));
            stack.push(*m);
        }
        out.push_str(&escape(core));
        pending.push_str(trail);
    }
    while let Some(m) = stack.pop() {
        out.push_str(&close(m));
    }
    out.push_str(&pending);
    out
}

/// A block's words as Markdown: **bold**, *italic*, ~~struck~~, <u>underlined</u>,
/// `code`. Colours have no Markdown and are left out.
fn markdown_inline(text: &str, base: usize, marks: &[MarkSpan]) -> String {
    let token = |m: Inline, closing: bool| -> String {
        match m {
            Inline::Bold => "**".into(),
            Inline::Italic => "*".into(),
            Inline::Strike => "~~".into(),
            Inline::Underline if closing => "</u>".into(),
            Inline::Underline => "<u>".into(),
            Inline::Color(_) => String::new(),
            Inline::Code => "`".into(),
        }
    };
    inline_text(text, base, marks, |m| token(m, false), |m| token(m, true), |s| s.to_string(), true)
}

/// A block's words as HTML, every mark kept (colours as the stored linen value).
fn html_inline(text: &str, base: usize, marks: &[MarkSpan]) -> String {
    let open = |m: Inline| -> String {
        match m {
            Inline::Bold => "<strong>".into(),
            Inline::Italic => "<em>".into(),
            Inline::Strike => "<s>".into(),
            Inline::Underline => "<u>".into(),
            Inline::Color(c) => format!("<span style=\"color:#{c:06x}\">"),
            Inline::Code => "<code>".into(),
        }
    };
    let close = |m: Inline| -> String {
        match m {
            Inline::Bold => "</strong>".into(),
            Inline::Italic => "</em>".into(),
            Inline::Strike => "</s>".into(),
            Inline::Underline => "</u>".into(),
            Inline::Color(_) => "</span>".into(),
            Inline::Code => "</code>".into(),
        }
    };
    inline_text(text, base, marks, open, close, esc, false)
}

/// The threads to print under each block, by block index, in the order they appear.
/// A thread goes under the printed block it starts in; one that starts in an empty
/// block (which is not printed) goes under the nearest printed block before it, and
/// one on the title goes under the title, so no visible thread is left out.
fn threads_under_blocks(note: &Note) -> Vec<Vec<&Thread>> {
    let mut printed = vec![];
    let mut at = 0;
    for (i, b) in note.blocks.iter().enumerate() {
        if i == 0 || !b.text.trim().is_empty() {
            printed.push((i, at));
        }
        at += b.text.len() + 1;
    }
    let mut threads: Vec<&Thread> = note.threads.iter().filter(|t| t.status != ThreadStatus::Dismissed).collect();
    threads.sort_by_key(|t| t.start);
    let mut out = vec![Vec::new(); note.blocks.len().max(1)];
    for t in threads {
        let i = printed.iter().rev().find(|&&(_, start)| start <= t.start).map_or(0, |&(i, _)| i);
        out[i].push(t);
    }
    out
}

fn title_of(note: &Note) -> &str {
    let title = note.title.trim();
    if title.is_empty() { "Untitled" } else { title }
}

fn mind_name(t: &Thread) -> &str {
    if t.mind_name.trim().is_empty() { "Inkling" } else { &t.mind_name }
}

fn thread_markdown(t: &Thread) -> String {
    let name = mind_name(t);
    let mut out = format!("> **{name}** on “{}”\n", t.quote.replace('\n', " "));
    for m in &t.messages {
        let who = if m.role == Role::Mind { name } else { "Me" };
        out.push_str(&format!(">\n> *{who}:* {}\n", m.text.replace('\n', "\n> ")));
    }
    if t.drawing.is_some() {
        out.push_str(">\n> *(drawing)*\n");
    }
    out.push('\n');
    out
}

pub fn to_markdown(note: &Note, with_threads: bool) -> String {
    let under = threads_under_blocks(note);
    let threads = |i: usize| {
        (with_threads && !under[i].is_empty()).then(|| under[i].iter().map(|t| thread_markdown(t)).collect::<String>())
    };
    let mut out = format!("# {}\n\n", title_of(note));
    if let Some(on_title) = threads(0) {
        out.push_str(&on_title);
    }
    let starts = block_starts(note);
    out.push_str(&markdown_blocks(&note.blocks, 1, |i| markdown_inline(&note.blocks[i].text, starts[i], &note.marks), threads));
    out.trim_end().to_string() + "\n"
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn thread_html(t: &Thread) -> String {
    let name = mind_name(t);
    let mut out = format!(
        "<div class=\"thread\"><div class=\"on\"><span class=\"who\">{}</span> on “{}”</div>\n",
        esc(name),
        esc(&t.quote.replace('\n', " "))
    );
    for m in &t.messages {
        let class = if m.role == Role::Mind { "mind" } else { "me" };
        out.push_str(&format!("<p class=\"{class}\">{}</p>\n", esc(&m.text).replace('\n', "<br>")));
    }
    if t.drawing.is_some() {
        out.push_str("<p class=\"on\">(drawing)</p>\n");
    }
    out.push_str("</div>\n");
    out
}

pub fn to_html(note: &Note, with_threads: bool) -> String {
    html(note, with_threads, false)
}

/// The HTML that becomes the Word file. `textutil` draws neither form controls nor
/// most of the page's style, so a checklist item is a line starting ☐ or ☑ and a callout
/// is a quote headed by its kind, in its thread.
pub fn to_word_html(note: &Note, with_threads: bool) -> String {
    html(note, with_threads, true)
}

/// A callout's thread in the linen palette, as `#rrggbb`.
fn callout_hex(kind: Callout) -> &'static str {
    match kind {
        Callout::Note => "#2e3f7a",
        Callout::Warning => "#8f6214",
        Callout::Tip => "#4f7350",
    }
}

fn html(note: &Note, with_threads: bool, word: bool) -> String {
    let under = threads_under_blocks(note);
    let threads = |i: usize| -> String {
        if with_threads { under[i].iter().map(|t| thread_html(t)).collect() } else { String::new() }
    };
    let starts = block_starts(note);
    let mut body = threads(0);
    // the list being written: its tag and how it opens
    let mut list: Option<(&str, &str)> = None;
    // Items so far in the current run of numbered blocks, as in the Markdown.
    let mut number = 0;
    let close = |list: &mut Option<(&str, &str)>, body: &mut String| {
        if let Some((tag, _)) = list.take() {
            body.push_str(&format!("</{tag}>\n"));
        }
    };
    let mut i = 1;
    while i < note.blocks.len() {
        let b = &note.blocks[i];
        // a callout or a block of code: its lines in a row are one box
        if let BlockKind::Callout(_) | BlockKind::Code(_) = b.kind {
            let mut end = i + 1;
            while end < note.blocks.len() && b.kind.joins(note.blocks[end].kind) {
                end += 1;
            }
            let shown: Vec<usize> = (i..end).filter(|&j| !note.blocks[j].text.trim().is_empty()).collect();
            if let (Some(&first), Some(&last)) = (shown.first(), shown.last()) {
                close(&mut list, &mut body);
                number = 0;
                match b.kind {
                    BlockKind::Callout(kind) if word => {
                        body.push_str(&format!("<blockquote><p><b><span style=\"color:{}\">{}</span></b></p>\n", callout_hex(kind), kind.label()));
                        for &j in &shown {
                            body.push_str(&format!("<p>{}</p>\n", html_inline(&note.blocks[j].text, starts[j], &note.marks)));
                        }
                        body.push_str("</blockquote>\n");
                    }
                    BlockKind::Callout(kind) => {
                        body.push_str(&format!("<div class=\"callout {}\"><p class=\"kind\">{}</p>\n", kind.name(), kind.label()));
                        for &j in &shown {
                            body.push_str(&format!("<p>{}</p>\n", html_inline(&note.blocks[j].text, starts[j], &note.marks)));
                        }
                        body.push_str("</div>\n");
                    }
                    BlockKind::Code(lang) => {
                        let code: Vec<String> = note.blocks[first..=last].iter().map(|c| esc(&c.text)).collect();
                        let class = if lang.is_empty() { String::new() } else { format!(" class=\"language-{}\"", lang.as_str()) };
                        body.push_str(&format!("<pre><code{class}>{}</code></pre>\n", code.join("\n")));
                    }
                    _ => {}
                }
                for j in i..end {
                    body.push_str(&threads(j));
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
        if b.kind != BlockKind::Numbered {
            number = 0;
        }
        let t = html_inline(&b.text, starts[at], &note.marks);
        let item = match b.kind {
            BlockKind::Bullet => Some(("ul", "<ul>")),
            BlockKind::Numbered => Some(("ol", "<ol>")),
            BlockKind::Check(_) if !word => Some(("ul", "<ul class=\"checklist\">")),
            _ => None,
        };
        match (item, b.kind) {
            (Some((tag, open)), kind) => {
                if list.map(|l| l.1) != Some(open) {
                    close(&mut list, &mut body);
                    match number {
                        n if n > 0 && kind == BlockKind::Numbered => body.push_str(&format!("<ol start=\"{}\">\n", n + 1)),
                        _ => body.push_str(&format!("{open}\n")),
                    }
                    list = Some((tag, open));
                }
                match kind {
                    BlockKind::Numbered => {
                        number += 1;
                        body.push_str(&format!("<li>{t}</li>\n"));
                    }
                    BlockKind::Check(done) => {
                        let (class, checked) = if done { (" class=\"done\"", " checked") } else { ("", "") };
                        body.push_str(&format!("<li{class}><input type=\"checkbox\" disabled{checked}> {t}</li>\n"));
                    }
                    _ => body.push_str(&format!("<li>{t}</li>\n")),
                }
                // Threads under an item close the list and the next item reopens it
                // at its number: a list may only hold items, and Word (textutil)
                // numbers whatever sits inside one.
                let under_item = threads(at);
                if !under_item.is_empty() {
                    close(&mut list, &mut body);
                    body.push_str(&under_item);
                }
            }
            (None, kind) => {
                close(&mut list, &mut body);
                match kind {
                    BlockKind::H1 => body.push_str(&format!("<h2>{t}</h2>\n")),
                    BlockKind::H2 => body.push_str(&format!("<h3>{t}</h3>\n")),
                    BlockKind::H3 => body.push_str(&format!("<h4>{t}</h4>\n")),
                    BlockKind::Quote => body.push_str(&format!("<blockquote>{t}</blockquote>\n")),
                    BlockKind::Check(done) => body.push_str(&format!("<p>{} {t}</p>\n", if done { '\u{2611}' } else { '\u{2610}' })),
                    _ => body.push_str(&format!("<p>{t}</p>\n")),
                }
                body.push_str(&threads(at));
            }
        }
    }
    close(&mut list, &mut body);
    // Loose Threads on paper: the writer's words in Fraunces, Inkling's in Figtree, each
    // thread hung off a madder running stitch (the fonts fall back where not installed).
    // Checklists, callouts and code are sewn as in the note: your ink, their kind's
    // thread, a flat ground for code.
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>\
body{{font-family:\"Fraunces Text\",Fraunces,\"Iowan Old Style\",Georgia,serif;font-size:17px;line-height:1.62;\
max-width:640px;margin:48px auto;padding:0 20px;color:#26232b;background:#fbf8f2}}\
h1,h2,h3,h4{{font-family:\"Fraunces Display\",Fraunces,\"Iowan Old Style\",Georgia,serif;font-weight:600;line-height:1.25}}\
blockquote{{margin:0 0 1em;padding-left:16px;border-left:2px dashed #8c8276;color:#57505c;font-style:italic}}\
code{{font-family:\"Spline Sans Mono\",Menlo,monospace;font-size:.86em;background:rgba(38,35,43,.06);padding:1px 4px;border-radius:4px}}\
pre{{background:#f1eadc;border-radius:8px;padding:10px 14px;margin:0 0 1em;white-space:pre-wrap;overflow-wrap:anywhere}}\
pre code{{font-size:13.5px;line-height:22px;background:none;padding:0}}\
.checklist{{list-style:none;padding-left:0}} .checklist input{{margin:0 8px 0 0;accent-color:#26232b}}\
.checklist .done{{color:#57505c;text-decoration:line-through;text-decoration-color:#8c8276}}\
.callout{{margin:0 0 1em;padding:8px 16px 10px;border-radius:8px;border:1.5px dashed var(--k);background:var(--bg)}}\
.callout p{{margin:0}} .callout .kind{{font-family:Figtree,-apple-system,\"Helvetica Neue\",Helvetica,sans-serif;font-size:12px;font-weight:600;color:var(--k)}}\
.callout.note{{--k:#2e3f7a;--bg:rgba(46,63,122,.06)}} .callout.warning{{--k:#8f6214;--bg:rgba(143,98,20,.06)}} .callout.tip{{--k:#4f7350;--bg:rgba(79,115,80,.06)}}\
.thread{{margin:6px 0 18px 18px;padding:2px 0 2px 12px;border-left:2px dashed #a63f32;\
font-family:Figtree,-apple-system,\"Helvetica Neue\",Helvetica,sans-serif;font-size:14px;line-height:1.5}}\
.thread p{{margin:4px 0}} .thread .on{{font-size:12px;color:#6f6862}} .thread .who{{color:#a63f32;font-weight:600}}\
.thread .me{{color:#57505c}}\
</style></head><body><h1>{title}</h1>\n{body}</body></html>",
        title = esc(title_of(note)),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Markdown,
    Html,
    Word,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Markdown => "md",
            Format::Html => "html",
            Format::Word => "docx",
        }
    }
}

pub fn write(note: &Note, with_threads: bool, format: Format, path: &std::path::Path) -> anyhow::Result<()> {
    match format {
        Format::Markdown => std::fs::write(path, to_markdown(note, with_threads))?,
        Format::Html => std::fs::write(path, to_html(note, with_threads))?,
        Format::Word => {
            // A fresh name per export, so two exports of one note cannot share a file.
            let tmp = std::env::temp_dir().join(format!("inkling-{}.html", crate::store::new_id()));
            std::fs::write(&tmp, to_word_html(note, with_threads))?;
            let output = std::process::Command::new("/usr/bin/textutil")
                .args(["-convert", "docx", "-output"])
                .arg(path)
                .arg(&tmp)
                .output();
            let _ = std::fs::remove_file(&tmp);
            let output = output?;
            anyhow::ensure!(
                output.status.success(),
                "textutil could not write the Word file: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Block, Mark, MarkSpan, Message, Mode, Side, ThoughtKind};

    #[std::prelude::v1::test]
    fn markdown_includes_threads_under_their_paragraph() {
        let note = crate::welcome::welcome_note(crate::store::Mode::ThinksWithYou);
        let md = to_markdown(&note, true);
        assert!(md.starts_with("# Welcome to Inkling"));
        let quote_at = md.find("The far side of the Moon").unwrap();
        let thread_at = md.find("**Inkling** on").unwrap();
        assert!(thread_at > quote_at);
        let plain = to_markdown(&note, false);
        assert!(!plain.contains("**Inkling** on"));
    }

    fn b(kind: BlockKind, text: &str) -> Block {
        Block { kind, text: text.into() }
    }

    /// A thread on `quote`, which must appear in the note's joined text.
    fn thread(note: &Note, quote: &str, status: ThreadStatus, messages: &[(Role, &str)]) -> Thread {
        let start = note.joined_text().find(quote).expect("quote is in the note");
        Thread {
            id: crate::store::new_id(),
            start,
            end: start + quote.len(),
            quote: quote.into(),
            kind: ThoughtKind::Question,
            messages: messages.iter().map(|(r, t)| Message::new(*r, *t)).collect(),
            drawing: None,
            status,
            side: Side::Right,
            mind_name: "Editor".into(),
            tag_offset: (0., 0.),
            created_at: chrono::Utc::now(),
            reason: None,
            ended_at: None,
            first_quote: None,
        }
    }

    fn sample() -> Note {
        let mut note = Note::new("Plans <& \"ideas\">", Mode::ThinksWithYou, None);
        note.blocks = vec![
            b(BlockKind::Title, "Plans <& \"ideas\">"),
            b(BlockKind::H1, "Why"),
            b(BlockKind::Paragraph, "Caching fixes it <probably> & fast."),
            b(BlockKind::Numbered, "Measure first"),
            b(BlockKind::Numbered, "Then cache"),
            b(BlockKind::Paragraph, "A paragraph after the list."),
            b(BlockKind::Bullet, "loose idea"),
            b(BlockKind::Paragraph, ""),
            b(BlockKind::Quote, "Premature optimisation…"),
            b(BlockKind::Paragraph, ""),
        ];
        let on_title = thread(&note, "Plans", ThreadStatus::Open, &[(Role::Mind, "Plans for whom?")]);
        let on_cache = thread(&note, "Caching fixes it", ThreadStatus::Open, &[(Role::Mind, "Fixes what, exactly?"), (Role::You, "Latency.\nMostly p99.")]);
        let on_item = thread(&note, "Measure first", ThreadStatus::Resolved, &[(Role::Mind, "Measure what?")]);
        let dismissed = thread(&note, "Then cache", ThreadStatus::Dismissed, &[(Role::Mind, "gone")]);
        let mut drawn = thread(&note, "loose idea", ThreadStatus::Open, &[(Role::Mind, "Like this?")]);
        drawn.drawing = Some("<svg/>".into());
        // A thread whose words were deleted collapses onto the empty line after its
        // block; it still belongs under the nearest block that is printed.
        let mut collapsed = thread(&note, "loose idea", ThreadStatus::Open, &[(Role::Mind, "Still here")]);
        collapsed.start += "loose idea".len() + 1;
        collapsed.end = collapsed.start;
        note.threads = vec![collapsed, dismissed, on_item, drawn, on_cache, on_title];
        note
    }

    #[std::prelude::v1::test]
    fn markdown_lists_numbering_and_thread_placement() {
        let md = to_markdown(&sample(), true);
        let expected = "# Plans <& \"ideas\">\n\n\
> **Editor** on “Plans”\n>\n> *Editor:* Plans for whom?\n\n\
## Why\n\n\
Caching fixes it <probably> & fast.\n\n\
> **Editor** on “Caching fixes it”\n>\n> *Editor:* Fixes what, exactly?\n>\n> *Me:* Latency.\n> Mostly p99.\n\n\
1. Measure first\n\n\
> **Editor** on “Measure first”\n>\n> *Editor:* Measure what?\n\n\
2. Then cache\n\n\
A paragraph after the list.\n\n\
- loose idea\n\n\
> **Editor** on “loose idea”\n>\n> *Editor:* Like this?\n>\n> *(drawing)*\n\n\
> **Editor** on “loose idea”\n>\n> *Editor:* Still here\n\n\
> Premature optimisation…\n";
        assert_eq!(md, expected);
        assert!(!md.contains("gone"), "dismissed threads stay out");

        let plain = to_markdown(&sample(), false);
        assert!(plain.contains("1. Measure first\n2. Then cache\n\nA paragraph after the list.\n\n- loose idea\n\n> Premature"));
        assert!(!plain.contains("**Editor**"));
    }

    #[std::prelude::v1::test]
    fn markdown_of_an_empty_note() {
        let note = Note::new("   ", Mode::ThinksWithYou, None);
        assert_eq!(to_markdown(&note, true), "# Untitled\n");
        let mut no_blocks = note.clone();
        no_blocks.blocks.clear();
        assert_eq!(to_markdown(&no_blocks, true), "# Untitled\n");
        assert!(to_html(&no_blocks, true).contains("<h1>Untitled</h1>"));
    }

    #[std::prelude::v1::test]
    fn html_escapes_and_nests_lists_and_threads() {
        let html = to_html(&sample(), true);
        assert!(html.contains("<title>Plans &lt;&amp; \"ideas\"&gt;</title>"));
        assert!(html.contains("<p>Caching fixes it &lt;probably&gt; &amp; fast.</p>"));
        assert!(!html.contains("<probably>"));
        assert!(html.contains("<p class=\"me\">Latency.<br>Mostly p99.</p>"));
        assert!(html.contains("<ol>\n<li>Measure first</li>\n</ol>\n<div class=\"thread\">"));
        assert!(html.contains("</div>\n<ol start=\"2\">\n<li>Then cache</li>\n</ol>\n<p>A paragraph after the list.</p>"));
        assert!(html.contains("<ul>\n<li>loose idea</li>\n</ul>\n<div class=\"thread\">"));
        assert!(html.contains("<p class=\"on\">(drawing)</p>"));
        assert!(html.contains("Still here</p>\n</div>\n<blockquote>Premature optimisation…</blockquote>"));
        assert!(!html.contains("gone"));
        // Every list holds only items.
        for (at, _) in html.match_indices("<ol").chain(html.match_indices("<ul")) {
            let after_tag = &html[at..][html[at..].find(">\n").unwrap() + 2..];
            assert!(after_tag.starts_with("<li>"), "{after_tag}");
        }
        for (at, _) in html.match_indices("</li>\n") {
            let next = &html[at + "</li>\n".len()..];
            assert!(["<li>", "</ol>", "</ul>"].iter().any(|t| next.starts_with(t)), "{next}");
        }
        assert_eq!(html.matches("<div class=\"thread\">").count(), 5);
        let plain = to_html(&sample(), false);
        assert!(!plain.contains("thread\">"));
        assert!(plain.contains("<li>Measure first</li>\n<li>Then cache</li>"));
    }

    fn marked(text: &str, marks: &[(&str, Mark)]) -> Note {
        let mut note = Note::new("Marks", Mode::ThinksWithYou, None);
        note.blocks = vec![b(BlockKind::Title, "Marks"), b(BlockKind::Paragraph, text)];
        let joined = note.joined_text();
        note.marks = marks
            .iter()
            .map(|(words, mark)| {
                let start = joined.find(words).expect("words are in the note");
                MarkSpan { start, end: start + words.len(), mark: *mark }
            })
            .collect();
        note
    }

    #[std::prelude::v1::test]
    fn marks_survive_export() {
        let note = marked(
            "Keep bold, italic, struck, underlined and code words, coloured too.",
            &[
                ("bold", Mark::Bold),
                ("italic", Mark::Italic),
                ("struck", Mark::Strike),
                ("underlined", Mark::Underline),
                ("code", Mark::Code),
                ("coloured", Mark::Color(0x2e3f7a)),
            ],
        );
        let md = to_markdown(&note, false);
        assert!(md.contains("Keep **bold**, *italic*, ~~struck~~, <u>underlined</u> and `code` words, coloured too."), "{md}");
        let html = to_html(&note, false);
        assert!(
            html.contains("<p>Keep <strong>bold</strong>, <em>italic</em>, <s>struck</s>, <u>underlined</u> and <code>code</code> words, <span style=\"color:#2e3f7a\">coloured</span> too.</p>"),
            "{html}"
        );
    }

    #[std::prelude::v1::test]
    fn overlapping_marks_nest_and_hug_their_words() {
        // Bold over "two words ", italic over "words and": they overlap, and the bold
        // ends on a space, which Markdown will not read inside the asterisks.
        let mut note = marked("one two words and more", &[]);
        let joined = note.joined_text();
        let at = |w: &str| joined.find(w).unwrap();
        note.marks = vec![
            MarkSpan { start: at("two"), end: at("and"), mark: Mark::Bold },
            MarkSpan { start: at("words"), end: at("and") + 3, mark: Mark::Italic },
            MarkSpan { start: 0, end: 3, mark: Mark::Color(0) },
        ];
        assert!(to_markdown(&note, false).contains("one **two *words*** *and* more"), "{}", to_markdown(&note, false));
        assert!(to_html(&note, false).contains("<p>one <strong>two <em>words </em></strong><em>and</em> more</p>"), "{}", to_html(&note, false));
        // marks never cut a character
        let mut wide = marked("日本語の文", &[]);
        wide.marks = vec![MarkSpan { start: "Marks\n".len() + 3, end: "Marks\n".len() + 9, mark: Mark::Bold }];
        assert!(to_markdown(&wide, false).contains("日**本語**の文"), "{}", to_markdown(&wide, false));
    }

    #[std::prelude::v1::test]
    fn writes_markdown_html_and_a_real_word_file() {
        let dir = std::env::temp_dir().join(format!("inkling-export-test-{}", crate::store::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let note = sample();
        write(&note, true, Format::Markdown, &dir.join("n.md")).unwrap();
        write(&note, true, Format::Html, &dir.join("n.html")).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("n.md")).unwrap(), to_markdown(&note, true));
        assert_eq!(std::fs::read_to_string(dir.join("n.html")).unwrap(), to_html(&note, true));
        if std::path::Path::new("/usr/bin/textutil").exists() {
            let docx = dir.join("Plans- ideas.docx");
            write(&note, true, Format::Word, &docx).unwrap();
            let bytes = std::fs::read(&docx).unwrap();
            assert!(bytes.starts_with(b"PK\x03\x04"), "a .docx is a zip file");
            // Read it back the way Word would see the text.
            let txt = dir.join("back.txt");
            let status = std::process::Command::new("/usr/bin/textutil")
                .args(["-convert", "txt", "-output"])
                .arg(&txt)
                .arg(&docx)
                .status()
                .unwrap();
            assert!(status.success());
            let text = std::fs::read_to_string(&txt).unwrap();
            for words in ["Plans <& \"ideas\">", "Caching fixes it <probably> & fast.", "Fixes what, exactly?", "Measure first", "Then cache", "Premature optimisation…"] {
                assert!(text.contains(words), "{words:?} missing from {text}");
            }
            assert!(text.find("Fixes what").unwrap() < text.find("Measure first").unwrap());
            // The thread under item 1 is not numbered as an item of its own.
            assert!(text.contains("2\tThen cache"), "{text}");
            let missing = write(&note, true, Format::Word, &dir.join("no-such-folder").join("x.docx"));
            assert!(missing.is_err());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A note with each new block: a checklist, two callouts, code with a thread on it.
    fn sewn() -> Note {
        let mut note = Note::new("Race week", Mode::ThinksWithYou, None);
        let rust = BlockKind::Code(crate::store::Lang::new("rust"));
        note.blocks = vec![
            b(BlockKind::Title, "Race week"),
            b(BlockKind::Check(true), "Pick up the bib"),
            b(BlockKind::Check(false), "Easy <5 km>"),
            b(BlockKind::Bullet, "a plain item"),
            b(BlockKind::Callout(Callout::Note), "One hill, at 6 km."),
            b(BlockKind::Callout(Callout::Warning), "Nothing new on race day."),
            b(rust, "fn finish(p: Pace) -> u64 {"),
            b(rust, "    p.secs * 10 // a < b && c"),
            b(rust, "}"),
            b(BlockKind::Paragraph, "After the code."),
        ];
        let on_code = thread(&note, "p.secs * 10", ThreadStatus::Open, &[(Role::Mind, "Seconds or minutes?")]);
        note.threads = vec![on_code];
        note
    }

    #[std::prelude::v1::test]
    fn markdown_has_tasks_callouts_and_fenced_code() {
        let md = to_markdown(&sewn(), true);
        let expected = "# Race week\n\n\
- [x] Pick up the bib\n- [ ] Easy <5 km>\n- a plain item\n\n\
> [!note]\n> One hill, at 6 km.\n\n\
> [!warning]\n> Nothing new on race day.\n\n\
```rust\nfn finish(p: Pace) -> u64 {\n    p.secs * 10 // a < b && c\n}\n```\n\n\
> **Editor** on “p.secs * 10”\n>\n> *Editor:* Seconds or minutes?\n\n\
After the code.\n";
        assert_eq!(md, expected);
        // marks are not written inside code
        let mut note = sewn();
        let at = note.joined_text().find("p.secs").unwrap();
        note.marks = vec![MarkSpan { start: at, end: at + 6, mark: Mark::Bold }];
        assert!(to_markdown(&note, false).contains("\n    p.secs * 10 //"));
    }

    #[std::prelude::v1::test]
    fn html_has_checkboxes_callout_boxes_and_pre() {
        let html = to_html(&sewn(), true);
        assert!(html.contains(
            "<ul class=\"checklist\">\n<li class=\"done\"><input type=\"checkbox\" disabled checked> Pick up the bib</li>\n\
<li><input type=\"checkbox\" disabled> Easy &lt;5 km&gt;</li>\n</ul>\n<ul>\n<li>a plain item</li>\n</ul>\n"
        ), "{html}");
        assert!(html.contains("<div class=\"callout note\"><p class=\"kind\">Note</p>\n<p>One hill, at 6 km.</p>\n</div>\n<div class=\"callout warning\">"), "{html}");
        assert!(html.contains(
            "<pre><code class=\"language-rust\">fn finish(p: Pace) -&gt; u64 {\n    p.secs * 10 // a &lt; b &amp;&amp; c\n}</code></pre>\n<div class=\"thread\">"
        ), "{html}");
        // Word gets lines it can draw: ☑ and ☐, and a quote headed by the callout's kind
        let word = to_word_html(&sewn(), false);
        assert!(word.contains("<p>\u{2611} Pick up the bib</p>\n<p>\u{2610} Easy &lt;5 km&gt;</p>\n<ul>\n<li>a plain item</li>"), "{word}");
        assert!(word.contains("<blockquote><p><b><span style=\"color:#2e3f7a\">Note</span></b></p>\n<p>One hill, at 6 km.</p>\n</blockquote>"), "{word}");
        assert!(!word.contains("<input"));
        if std::path::Path::new("/usr/bin/textutil").exists() {
            let dir = std::env::temp_dir().join(format!("inkling-export-sewn-{}", crate::store::new_id()));
            std::fs::create_dir_all(&dir).unwrap();
            write(&sewn(), true, Format::Word, &dir.join("w.docx")).unwrap();
            let txt = dir.join("w.txt");
            let ok = std::process::Command::new("/usr/bin/textutil").args(["-convert", "txt", "-output"]).arg(&txt).arg(dir.join("w.docx")).status().unwrap();
            assert!(ok.success());
            let text = std::fs::read_to_string(&txt).unwrap();
            for words in ["\u{2611} Pick up the bib", "\u{2610} Easy <5 km>", "Note", "One hill, at 6 km.", "Warning", "fn finish(p: Pace) -> u64 {", "    p.secs * 10 // a < b && c", "Seconds or minutes?"] {
                assert!(text.contains(words), "{words:?} missing from {text}");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

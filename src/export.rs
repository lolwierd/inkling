//! Export a note as Markdown, HTML or Word. Threads (the questions in the margin) can sit under
//! the paragraph they are about. Word goes through macOS `textutil`, which turns
//! HTML into .docx without any extra dependency.

use crate::store::{BlockKind, Note, Role, Thread, ThreadStatus, markdown_blocks};

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
    out.push_str(&markdown_blocks(&note.blocks, 1, threads));
    out.trim_end().to_string() + "\n"
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn thread_html(t: &Thread) -> String {
    let name = mind_name(t);
    let mut out = format!(
        "<div class=\"thread\"><div class=\"on\">{} on “{}”</div>\n",
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
    let under = threads_under_blocks(note);
    let threads = |i: usize| -> String {
        if with_threads { under[i].iter().map(|t| thread_html(t)).collect() } else { String::new() }
    };
    let mut body = threads(0);
    let mut list: Option<&str> = None;
    // Items so far in the current run of numbered blocks, as in the Markdown.
    let mut number = 0;
    let close = |list: &mut Option<&str>, body: &mut String| {
        if let Some(tag) = list.take() {
            body.push_str(&format!("</{tag}>\n"));
        }
    };
    for (i, b) in note.blocks.iter().enumerate().skip(1) {
        if b.text.trim().is_empty() {
            continue;
        }
        if b.kind != BlockKind::Numbered {
            number = 0;
        }
        let t = esc(&b.text);
        match b.kind {
            BlockKind::Bullet | BlockKind::Numbered => {
                let tag = if b.kind == BlockKind::Bullet { "ul" } else { "ol" };
                if list != Some(tag) {
                    close(&mut list, &mut body);
                    match number {
                        0 => body.push_str(&format!("<{tag}>\n")),
                        n => body.push_str(&format!("<ol start=\"{}\">\n", n + 1)),
                    }
                    list = Some(tag);
                }
                if b.kind == BlockKind::Numbered {
                    number += 1;
                }
                body.push_str(&format!("<li>{t}</li>\n"));
                // Threads under an item close the list and the next item reopens it
                // at its number: a list may only hold items, and Word (textutil)
                // numbers whatever sits inside one.
                let under_item = threads(i);
                if !under_item.is_empty() {
                    close(&mut list, &mut body);
                    body.push_str(&under_item);
                }
            }
            kind => {
                close(&mut list, &mut body);
                match kind {
                    BlockKind::H1 => body.push_str(&format!("<h2>{t}</h2>\n")),
                    BlockKind::H2 => body.push_str(&format!("<h3>{t}</h3>\n")),
                    BlockKind::H3 => body.push_str(&format!("<h4>{t}</h4>\n")),
                    BlockKind::Quote => body.push_str(&format!("<blockquote>{t}</blockquote>\n")),
                    _ => body.push_str(&format!("<p>{t}</p>\n")),
                }
                body.push_str(&threads(i));
            }
        }
    }
    close(&mut list, &mut body);
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>\
body{{font-family:-apple-system,Helvetica,sans-serif;max-width:640px;margin:40px auto;color:#1d1c1a;line-height:1.6}}\
.thread{{margin:6px 0 16px 18px;padding-left:10px;border-left:2px solid #dedbd4;font-family:Georgia,serif;color:#2a2824}}\
.thread .on{{font-size:12px;color:#8e8a83}} .thread .me{{font-family:-apple-system,Helvetica,sans-serif;color:#5c5953}}\
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
            std::fs::write(&tmp, to_html(note, with_threads))?;
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
    use crate::store::{Block, Message, Mode, Side, ThoughtKind};

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
            created_at: chrono::Utc::now(),
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
}

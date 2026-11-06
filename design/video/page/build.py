"""Builds design/video/page/index.html, the film's own page: the phone-sized film, its
chapters (tap to jump), and every capability with the moment it appears, read from
../coverage.md. Publish index.html with film.mp4 (the 720p encode) and poster.jpg.

The chapters' times come from coverage.md too (render.ts writes it from the cut), so a
re-cut only needs new blurbs here when a chapter's content changes.
"""
import html
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent

# what each chapter shows, by name; the times are the cut's own
BLURBS = {
    "A notebook that asks": "The line is stitched onto the linen and becomes the page's border.",
    "Start here": "A new notebook opens on the guide, one engineer's note; your first sentence gets a question.",
    "Reading along": "A plan to get VM cold starts under two seconds. It reads at your pauses, waits while you type, catches the copy's arithmetic, and ties the thread off when you fix it.",
    "Asking": "A loose thread on the staging plan, your own question, the room of three minds, the Skeptic by name, a sketch of the path from API call to shell, the brief, the chat.",
    "Threads": "Unfold, jump, drag, reply, tie off, snip and undo, reopen, hide them all, where they go.",
    "Minds": "Make a mind of your own, Security, pick it as the note's mind, and see who asked what by colour.",
    "Make it yours": "Light and dark, thread colours, lettering, text size, pace, ink, dictation, what it learns about you, shortcuts, the keyboard's ring.",
    "Your notebook": "Notes and folders, find, copy, export, Obsidian, the guide, the model, then block kinds, inline code, a checklist, a callout and a code block.",
    "The mark": "The last thread is tied off, and its knot becomes the dot of the i: Initial.",
}


def esc(s):
    return html.escape(s, quote=True)


def secs(t):
    m, s = t.split(":")
    return int(m) * 60 + int(s)


def coverage():
    """(chapters [(time, name)], sections [(section, [(capability, time, what)])]) from coverage.md."""
    text = (HERE.parent / "coverage.md").read_text()
    chapters = [("0:00", "A notebook that asks")]
    line = next(l for l in text.splitlines() if l.startswith("Length "))
    chapters += [(t, name) for _, name, t in re.findall(r"(\d+) ([^,(]+?) \((\d+:\d\d)\)", line)]
    mark = re.search(r"then the mark \((\d+:\d\d)\)", line)
    if mark:
        chapters.append((mark.group(1), "The mark"))
    out, section = [], None
    for l in text.splitlines():
        if l.startswith("## "):
            section = (l[3:].strip(), [])
            out.append(section)
            continue
        m = re.match(r"^\| (.+?) \| (\d+:\d\d) \| (.+?) \|$", l)
        if m and section:
            section[1].append((m.group(1), m.group(2), m.group(3)))
    length = re.search(r"Length (\d+:\d\d)", line).group(1)
    return chapters, out, length


def build():
    chapters, sections_in, length = coverage()
    chapter_html = "".join(
        f'<li><button type="button" class="seek" data-t="{secs(t)}"><span class="time">{t}</span>'
        f'<span class="what"><strong>{esc(n)}</strong><span>{esc(BLURBS.get(n, ""))}</span></span></button></li>'
        for t, n in chapters
    )
    sections, total = [], 0
    for name, rows in sections_in:
        total += len(rows)
        items = "".join(
            f'<li><button type="button" class="seek" data-t="{secs(t)}"><span class="time">{t}</span>'
            f'<span class="what"><strong>{esc(c)}</strong><span>{esc(w)}</span></span></button></li>'
            for c, t, w in rows
        )
        sections.append(f'<h3>{esc(name)}</h3><ol class="moments">{items}</ol>')
    page = (HERE / "template.html").read_text()
    page = page.replace("{{CHAPTERS}}", chapter_html).replace("{{COVERAGE}}", "".join(sections)).replace("{{TOTAL}}", str(total)).replace("{{LENGTH}}", length)
    (HERE / "index.html").write_text(page)
    print("wrote", HERE / "index.html", len(page), "bytes;", total, "capabilities;", len(chapters), "chapters")


if __name__ == "__main__":
    build()

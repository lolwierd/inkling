"""Builds design/video/page/index.html, the film's own page: the phone-sized film, its
chapters (tap to jump), and every capability with the moment it appears, read from
../coverage.md. Publish index.html with film.mp4 (the 720p encode) and poster.jpg.
"""
import html
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent

CHAPTERS = [
    ("0:00", "A notebook that asks", "The line is stitched onto the linen and becomes the page's border."),
    ("0:08", "Start here", "A new notebook opens on the guide; your first sentence gets a question."),
    ("0:30", "Reading along", "It reads at your pauses, waits while you type, and ties a thread off when you fix the words."),
    ("1:01", "Asking", "Find a loose thread, your own question, the room of three minds, a mind by name, a sketch, the brief, the chat."),
    ("1:52", "Threads", "Unfold, reply, drag, tie off, snip and undo, hide them all."),
    ("2:22", "Minds", "Pick the note's mind, make your own, and see who asked what by colour."),
    ("2:39", "Make it yours", "Light and dark, thread colours, lettering, text size, pace, dictation."),
    ("2:59", "Your notebook", "Notes and folders, find, copy, export, Obsidian, shortcuts, the model."),
    ("3:17", "The mark", "A tied-off thread grows the border into darns: Darn · Border."),
]


def esc(s):
    return html.escape(s, quote=True)


def secs(t):
    m, s = t.split(":")
    return int(m) * 60 + int(s)


def coverage():
    """(section, [(capability, time, what is on screen)]) from coverage.md."""
    out, section = [], None
    for line in (HERE.parent / "coverage.md").read_text().splitlines():
        if line.startswith("## "):
            section = (line[3:].strip(), [])
            out.append(section)
            continue
        m = re.match(r"^\| (.+?) \| (\d+:\d\d) \| (.+?) \|$", line)
        if m and section:
            section[1].append((m.group(1), m.group(2), m.group(3)))
    return out


def build():
    chapters = "".join(
        f'<li><button type="button" class="seek" data-t="{secs(t)}"><span class="time">{t}</span>'
        f'<span class="what"><strong>{esc(n)}</strong><span>{esc(d)}</span></span></button></li>'
        for t, n, d in CHAPTERS
    )
    sections, total = [], 0
    for name, rows in coverage():
        total += len(rows)
        items = "".join(
            f'<li><button type="button" class="seek" data-t="{secs(t)}"><span class="time">{t}</span>'
            f'<span class="what"><strong>{esc(c)}</strong><span>{esc(w)}</span></span></button></li>'
            for c, t, w in rows
        )
        sections.append(f'<h3>{esc(name)}</h3><ol class="moments">{items}</ol>')
    page = (HERE / "template.html").read_text()
    page = page.replace("{{CHAPTERS}}", chapters).replace("{{COVERAGE}}", "".join(sections)).replace("{{TOTAL}}", str(total))
    (HERE / "index.html").write_text(page)
    print("wrote", HERE / "index.html", len(page), "bytes;", total, "capabilities")


if __name__ == "__main__":
    build()

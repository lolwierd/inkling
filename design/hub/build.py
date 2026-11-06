"""Builds design/hub/index.html: one page for Ayaan's phone with where Inkling stands.

Edit the CONTENT below as work lands, run `python3 build.py`, then republish the page
(same artifact link, with the files in logos/, dots/ and shots/). The manifesto section
is rendered from ../manifesto.md each time.
"""
import html
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
UPDATED = "7 Nov 2026, 04:52 IST"

LINKS = {
    "logos": "https://claude.ai/artifact/HsnfPbYBDSn5Bk5m9gh3bX",
    "motion": "https://claude.ai/artifact/9g3xaUjjCLsiFTHZx6ZGe7",
    "directions": "https://claude.ai/artifact/6m7FAViLq1HpYS1DdJgi1U",
    "film": "https://claude.ai/artifact/KncyvgBKvh3Gx8kxeW5WYD",
}

WAITING = [
    ("One tap: let the new film upload",
     "The new 73-second film (1080p, 60 fps, no jitter) is finished and merged, but every file upload failed after you went to sleep, "
     "even small ones, so the film page still shows the first cut. Approve the upload when it asks, or tell me to try again.",
     None),
    ("Which of the manifesto's six to change",
     "The manifesto argues against six things the app still does; nothing changed there tonight. The full text is at the bottom of this page.",
     ("Read the manifesto", "#manifesto")),
]

RUNNING = []

DONE = [
    ("All nine features from the manifesto", "Answer in the note, Write it in, Let it stand, Turn it over, Threads of your own, Show its working, Loose ends, Read it through and Threads between notes, each built by its own agent, merged one at a time and checked against the others."),
    ("Web sources, when you ask", "Inkling can look things up for what you ask it; reading along never searches. A source shows only when the page really says what it quotes. Capped at $2.50 a call."),
    ("Code health", "The drag now follows every pointer move (before: 1 move in 36). On a long note, idle CPU fell from 95% to 7.5% and typing from 94% to 7.6%; threads are 20 times cheaper to draw; the tag's five hover maps became one state."),
    ("The film, core cut", "73 seconds, one note, 1080p at 60 fps. The jitter was the camera itself: up to 398 px a frame, overshoots and cuts. Now 17 px a frame at most, none of either. Waiting on the upload."),
    ("The icon follows the system", "Initial in light, dark, clear and tinted, compiled with Xcode, so the Dock and Finder switch with your Mac even while Inkling is closed."),
    ("Start here, complete", "The guide teaches all ten new things, with real examples each feature's own checks accept."),
    ("Polish", "An Edit menu, tags that never cover your words, the sidebar's slide, the narrow tag's row, and ⌃Tab reaching Chat after you type."),
    ("The film, first technical cut", "One note about VM cold starts from start to finish, real model words, a quiet score at -20 LUFS (the old one was -16 with constant clicks), and it ends on Initial. On the film's page."),
    ("Show its working", "A correction only reaches the page with a check you can do yourself: the sum laid out from your own numbers, or your own words quoted. A check that does not hold up becomes a question."),
    ("Let it stand", "Dismiss becomes Let stand, with an optional reason; no mind raises that point again, in any words."),
    ("Threads never cover your words", "An open tag hangs outward, over the cloth if it must, or gets narrower; only a tag you dragged onto the text may cover it."),
    ("Small things", "The sidebar slides in and out; the Dock icon follows light and dark while Inkling is open; Start here is one engineer's note now; the Mac menus are complete."),
    ("The critique's five calls", "Three changed: the toolbar's current key is filled in its own colour, a dragged tag stays on the paper, and Settings › Minds shows each mind's colour. Settings keeps its Mac-style buttons, and ⌘K keeps its light dim."),
    ("A fresh UI critique", "33 problems found and 25 fixed. The page scrolls under a band of cloth, so nothing runs under the window's controls; a reply wraps inside its tag; Marks only sets its ? like a footnote; the gutter's marks stay on the paper; long mind names end in an ellipsis; and madder is only ever Inkling's colour. Of the other 8, three are by design and five are small calls for you, under Waiting on you."),
    ("Toasts and keyboard focus", "One stack of stitched labels at the bottom, ochre for problems with a close button, Undo inside; hovering pauses it. A dashed focus ring in your thread colour and a Tab order through everything."),
    ("The README", "Written for people, with the film at the top."),
    ("Sewn blocks", "Checklists that take a cross-stitch, callouts sewn on as patches (note, warning, tip), code blocks with line numbers and thread-colour syntax, and quotes on a back-stitched seam. Type [ ], > [!note] or three backticks."),
    ("The film, re-cut", "3:28, all 44 capabilities on screen, a camera that moves with the thread, and a score made from needle, thread and cloth. It has its own page, linked under The film."),
    ("The logo: Initial", "Your pick, back from round 1: a satin-stitched i dotted with a French knot. It is the app icon now, and the wordmark in the app sets its i in your thread colour with the knot on top."),
    ("Loose Threads, in the app", "Stitches under your words, a hanging thread with real rope physics, tags in the margin, a knot when a thread is answered."),
    ("Tags that fold and drop", "A tag stays folded until you rest on it, drops open on a spring, and drags anywhere."),
    ("An immersive window", "No top bar. The controls fade while you write and come back when you move the pointer."),
    ("Start here", "A new notebook opens on a working tour of everything, with real threads to try."),
    ("Settings, as a Mac window", "Every shortcut can be changed; models come from Claude Code itself."),
    ("Import from Obsidian", "A vault comes in with its folders; Inkling leaves the imported words alone."),
    ("The manifesto", "Eight beliefs, ten features they call for, six things they argue against."),
    ("A UI audit", "59 findings, all closed: 55 fixed, 3 made moot by later changes, 1 written up."),
    ("Built for agents", "A map of the repo and one command that checks everything."),
    ("Checks", "188 Rust tests and 101 brain tests pass; 42 of 43 scenarios pass (the last needs the real model)."),
]

LOGOS = [
    ("initial", "Initial", "A satin-stitched i, dotted with a knot."),
    ("darn-border", "Darn · Border", "One darn per answer along the top."),
    ("darn-word", "Darn · Word", "A word in your line, darned in place."),
    ("darn-hole", "Darn · Hole", "Half-woven, the needle in the open row."),
    ("darn-heel", "Darn · Heel", "The claim a plan leans on, mended."),
    ("darn-margin", "Darn · Margin", "The mend runs on into the margin."),
    ("tied-settled", "Tied off · Settled", "The knot ends the stitch under the word."),
    ("tied-margin", "Tied off · To the margin", "The loose end, tied at the margin."),
    ("tied-finger", "Tied off · Round the finger", "Tied round the i, like a reminder."),
    ("sampler-backstitch", "Sampler · Backstitch", "The name in short stitches, end to end."),
    ("sampler-crosses", "Sampler · Crosses on the line", "Cross-stitch along each stroke."),
]

DOTS = [
    ("stitch", "One stitch"), ("darn", "A tiny darn"), ("cross", "A cross-stitch"),
    ("thread-end", "A thread end"), ("eye", "A needle's eye"), ("font", "The font's own dot"),
]

SHOTS = [
    ("mind-colours", "Settings › Minds now shows the colour each mind asks in."),
    ("toolbar-current", "The selection toolbar fills the current key in its own colour: here, a Note callout."),
    ("scroll-band", "Scrolled, the page slides under a band of cloth that holds the window's controls, with a soft shadow on the paper."),
    ("reply-wraps", "Your reply wraps inside an open tag; it used to run off the edge."),
    ("marks-only", "Marks only: the ? or × sits after the words like a footnote mark."),
    ("toasts", "Toasts: one stack at the bottom; a problem in ochre with a close button, Undo under it."),
    ("focus-ring", "Keyboard focus: a dashed ring in your thread colour, only when you use the keyboard."),
    ("sewn-blocks", "New: a checklist, the three callouts, a code block with a thread on its last line, and a quote on its seam."),
    ("sewn-blocks-dark", "The same blocks in Lamplight."),
    ("start-here", "Start here: a new notebook opens on a working tour, with real threads to try."),
    ("tag-hover", "Rest the pointer on a folded tag and the question drops open."),
    ("tag-open", "Click it to write back; Resolve, Dismiss, Make a note and Copy sit underneath."),
    ("tag-dragged-dark", "Tags drag anywhere. Near an edge, an open tag finds the side with room."),
    ("chrome-shown", "No top bar: the notes toggle and Chat sit beside the traffic lights."),
    ("chrome-faded", "Start typing and they fade, with the row above the title."),
    ("moon-sketch", "A sketch, stitched out step by step, inside its thread."),
    ("sidebar-dark", "Your notes and folders, in Lamplight."),
    ("settings-appearance", "Settings is a Mac window now; every choice shows on your note as you make it."),
    ("settings-shortcuts-dark", "Every shortcut can be changed, with clashes flagged."),
]


def esc(s):
    return html.escape(s, quote=True)


def inline_md(s):
    s = esc(s)
    s = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", s)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    return s


def manifesto_html():
    """The manifesto, from Markdown: headings, paragraphs, lists (one level of nesting)."""
    lines = (HERE.parent / "manifesto.md").read_text().splitlines()
    out, para, lst, sub = [], [], None, []

    def flush_para():
        if para:
            out.append(f"<p>{inline_md(' '.join(para))}</p>")
            para.clear()

    def flush_list():
        nonlocal lst
        if lst is not None:
            tag, items = lst
            body = "".join(f"<li>{t}{''.join(s)}</li>" for t, s in items)
            out.append(f"<{tag}>{body}</{tag}>")
            lst = None

    for raw in lines:
        line = raw.rstrip()
        if not line.strip():
            flush_para()
            continue
        if line.startswith("# "):
            flush_para(); flush_list()
            continue  # the page has its own heading for it
        if line.startswith("## "):
            flush_para(); flush_list()
            out.append(f"<h3>{inline_md(line[3:])}</h3>")
            continue
        m = re.match(r"^(\d+)\.\s+(.*)", line)
        if m:
            flush_para()
            if lst is None or lst[0] != "ol":
                flush_list(); lst = ("ol", [])
            lst[1].append([inline_md(m.group(2)), []])
            continue
        m = re.match(r"^(\s+)- (.*)", line)
        if m and lst is not None:
            item = lst[1][-1]
            if not item[1]:
                item[1].append("<ul>")
            item[1].append(f"<li>{inline_md(m.group(2))}</li>")
            continue
        m = re.match(r"^- (.*)", line)
        if m:
            flush_para()
            if lst is None or lst[0] != "ul":
                flush_list(); lst = ("ul", [])
            lst[1].append([inline_md(m.group(1)), []])
            continue
        if lst is not None and raw.startswith("   ") and lst[1]:
            # a wrapped continuation of a list item (or of its sub-item)
            item = lst[1][-1]
            if item[1]:
                item[1][-1] = item[1][-1].replace("</li>", " " + inline_md(line.strip()) + "</li>")
            else:
                item[0] += " " + inline_md(line.strip())
            continue
        flush_list()
        para.append(line.strip())
    flush_para(); flush_list()
    # close the nested lists opened above
    return "\n".join(o.replace("<ul><li>", "<ul><li>", 1) for o in out).replace("</li></li>", "</li></ul></li>")


def card(title, body, link=None, chip=None):
    chip_html = f'<span class="chip {chip[0]}">{esc(chip[1])}</span>' if chip else ""
    link_html = ""
    if link:
        label, href = link
        ext = "" if href.startswith("#") else ' target="_blank" rel="noopener"'
        link_html = f'<a class="tag-link" href="{esc(href)}"{ext}>{esc(label)}</a>'
    return f'<article class="card">{chip_html}<h3>{esc(title)}</h3><p>{esc(body)}</p>{link_html}</article>'


def build():
    film = LINKS["film"]
    waiting = "".join(card(t, b, link, ("wait", "Waiting on you")) for t, b, link in WAITING)
    running = "".join(card(t, b, None, ("run", "Running")) for t, b in RUNNING) or (
        '<p class="meta">Nothing is running now. All of the work is merged into master.</p>'
    )
    done = "".join(f'<li><strong>{esc(t)}.</strong> {esc(b)}</li>' for t, b in DONE)
    pick = LOGOS[0]
    logos = "".join(
        f'<figure class="logo"><img class="only-light" src="logos/{s}.svg" alt="{esc(n)} icon"><img class="only-dark" src="logos/{s}-dark.svg" alt="{esc(n)} icon, dark">'
        f'<figcaption><strong>{esc(n)}</strong><span>{esc(c)}</span></figcaption></figure>'
        for s, n, c in LOGOS[1:]
    )
    dots = "".join(f'<figure class="dot"><img src="dots/{s}.svg" alt="The wordmark with {esc(n.lower())} for the dot"><figcaption>{esc(n)}</figcaption></figure>' for s, n in DOTS)
    shots = "".join(f'<figure class="shot"><img src="shots/{s}.jpg" alt="{esc(c)}" loading="lazy"><figcaption>{esc(c)}</figcaption></figure>' for s, c in SHOTS)
    film_html = (
        f'<a class="tag-link" href="{esc(film)}" target="_blank" rel="noopener">Watch the film</a>' if film
        else '<p class="meta">Its page is not ready yet: the re-cut is still being made.</p>'
    )
    page = TEMPLATE
    for k, v in {
        "{{UPDATED}}": esc(UPDATED), "{{WAITING}}": waiting, "{{RUNNING}}": running, "{{DONE}}": done,
        "{{PICK_SLUG}}": pick[0], "{{PICK_NAME}}": esc(pick[1]), "{{LOGOS}}": logos, "{{DOTS}}": dots,
        "{{SHOTS}}": shots, "{{FILM}}": film_html, "{{MANIFESTO}}": manifesto_html(),
        "{{LOGO_URL}}": LINKS["logos"], "{{MOTION_URL}}": LINKS["motion"], "{{DIRECTIONS_URL}}": LINKS["directions"],
        "{{N_WAIT}}": str(len(WAITING)), "{{N_RUN}}": str(len(RUNNING)),
    }.items():
        page = page.replace(k, v)
    if not RUNNING:
        # no branches out, so the note about branches would only confuse
        page = page.replace('<p class="section-note">Each is on a branch of its own; I review and merge as they report.</p>', "")
    (HERE / "index.html").write_text(page)
    print("wrote", HERE / "index.html", len(page), "bytes")


TEMPLATE = (HERE / "template.html").read_text() if (HERE / "template.html").exists() else ""

if __name__ == "__main__":
    build()

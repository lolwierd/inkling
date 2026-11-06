"""Builds design/video/page/index.html, the film's own page: the film, and its moments (tap
one to jump there). The moments and their times come from page/moments.json, which
`bun run render.ts moments <scratch>` writes from the cut, so a re-cut only needs a new blurb
here when a moment's content changes. Publish index.html with film.mp4 and poster.jpg.
"""
import html
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

# what each moment shows, by its id in film.js PLAN; the times are the cut's own
BLURBS = {
    "open": "One running stitch, tied off, and the app's page.",
    "write": "A note about getting VM cold starts under two seconds. Inkling reads each paragraph when you pause, and mostly says nothing.",
    "plan": "The plan, with a latency budget. Inkling's read arrives while you are still typing, and waits.",
    "thread": "A thread comes out of your exact words, “which leaves half a second of the 2 s budget”, to a tag in the margin.",
    "working": "The correction, and under it the working, made only from your numbers, each one a link to your words.",
    "fix": "You change the restore to 0.5 s. Inkling reads it again, agrees the sum holds, and the thread ties itself off.",
    "find": "Select “We try it on the staging hosts first” and ask it to find the question in them.",
    "reply": "Write back in the tag. Inkling answers that it's settled, and you tie it off.",
    "end": "It never writes your notes. The last knot becomes the dot of the i.",
}


def esc(s):
    return html.escape(s, quote=True)


def build():
    data = json.loads((HERE / "moments.json").read_text())
    items = "".join(
        f'<li><button type="button" class="seek" data-t="{int(m["t"])}"><span class="time">{m["at"]}</span>'
        f'<span class="what"><strong>{esc(m["name"])}</strong><span>{esc(BLURBS.get(m["id"], m.get("caption") or ""))}</span></span></button></li>'
        for m in data["moments"]
    )
    page = (HERE / "template.html").read_text()
    page = page.replace("{{MOMENTS}}", items).replace("{{LENGTH}}", data["length"])
    (HERE / "index.html").write_text(page)
    print("wrote", HERE / "index.html", len(page), "bytes;", len(data["moments"]), "moments,", data["length"])


if __name__ == "__main__":
    build()

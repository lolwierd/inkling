"""Builds ../logo/index.html, the review page for the logo directions.

The page's words and structure are written out here as plain HTML, so they read in the
source. The SVGs travel once, as one JSON object that the page's script hands to every
<img> as an object URL: each mark appears about thirty times on the page. (It was a gzip'd
blob; a published page may not embed an archive, so it is plain JSON, about 1.2 MB.)

Called by logo_directions.py; the template is logo_page.src.html.
"""
import base64
import gzip
import html
import json
import re
from pathlib import Path

import logo_directions as L
from logos import superellipse

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "logo" / "index.html"
NOW = HERE.parent / "directions" / "assets" / "threads"

# the app's title bar row (src/app.rs, the sidebar header): 46 px tall, the wordmark at
# 19 px, its canvas 1.3 em tall and centred, so the baseline lands 29.9 px down
TB_SIZE, TB_LEFT, TB_BASE = 19.0, 86.0, 29.9


def vb(svg):
    return [float(v) for v in re.search(r'viewBox="([^"]+)"', svg).group(1).split()]


def esc(s):
    return html.escape(s, quote=True)


def now_variants():
    """Today's wordmark has its dark colours behind a media query; fix each version."""
    svg = (NOW / "wordmark.svg").read_text()
    style = re.search(r"<style>(.*?)</style>", svg, re.S).group(1)
    m = re.search(r"@media \(prefers-color-scheme:dark\)\{(.*)\}\s*$", style, re.S)
    light, dark = style[:m.start()], style[:m.start()] + " " + m.group(1)
    return (svg.replace(style, light), svg.replace(style, dark))


def wm_img(key, svg, size, cls="", extra=""):
    """A wordmark <img> at a font size in px: every wordmark is drawn at 100 units an em."""
    x, y, w, h = vb(svg)
    s = size / 100
    return (f'<img class="{cls}" data-a="{key}" width="{w * s:.1f}" height="{h * s:.1f}" '
            f'style="width:{w * s:.1f}px;height:{h * s:.1f}px{extra}" alt="">')


def titlebar(slug, svg, theme):
    x, y, w, h = vb(svg)
    s = TB_SIZE / 100
    left = TB_LEFT + (x + 8) * s
    top = TB_BASE + y * s
    key = f"{slug}:w{'l' if theme == 'l' else 'd'}"
    return (f'<div class="tb {theme}" role="img" aria-label="The title bar in {"light" if theme == "l" else "dark"} mode">'
            f'<div class="tb-side"><span class="lights"><i></i><i></i><i></i></span>'
            f'<img class="tb-wm" data-a="{key}" alt="" style="left:{left:.1f}px;top:{top:.1f}px;width:{w * s:.1f}px;height:{h * s:.1f}px">'
            f'<svg class="tb-x" viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg></div>'
            f'<div class="tb-page"><span class="tb-chat"><i></i>Chat</span></div></div>')


def dock_small(slug, theme):
    k = "il" if theme == "l" else "id"
    items = [f'<span class="dk"><img data-other="{o}" alt=""></span>' for o in "ab"]
    items.append(f'<span class="dk"><img data-a="{slug}:{k}" alt=""><span class="run"></span></span>')
    items += [f'<span class="dk"><img data-other="{o}" alt=""></span>' for o in "de"]
    return (f'<div class="dockwin {theme}" role="img" aria-label="In the Dock, {"light" if theme == "l" else "dark"}">'
            f'<div class="dock">{"".join(items)}</div></div>')


def finder(slug, name, theme, selected):
    k = "il" if theme == "l" else "id"
    rows = [("Hopscotch", "c", "Yesterday at 09:12", "41.8 MB"),
            ("Inkling", None, "Today at 18:20", "38.2 MB"),
            ("Jotter", "b", "2 Nov 2026 at 11:04", "12.6 MB"),
            ("Kiln", "d", "28 Oct 2026 at 16:47", "96.1 MB")]
    out = []
    for nm, o, date, size in rows:
        icon = f'<img data-a="{slug}:{k}" alt="">' if o is None else f'<img data-other="{o}" alt="">'
        sel = " sel" if (o is None and selected) else ""
        out.append(f'<div class="frow{sel}"><span class="nm">{icon}{nm}</span><span class="m">{date}</span>'
                   f'<span class="m">{size}</span><span class="m">Application</span></div>')
    return (f'<div class="finder {theme}" role="img" aria-label="In a Finder list, {"light" if theme == "l" else "dark"}">'
            f'<div class="fbar"><span class="lights"><i></i><i></i><i></i></span><span>Applications</span></div>'
            f'<div class="fhead"><span>Name</span><span>Date Modified</span><span>Size</span><span>Kind</span></div>'
            f'{"".join(out)}</div>')


def build_page(assets):
    data, names = {}, {}
    for d in assets:
        slug = d["slug"]
        data[slug] = {"wl": L.fixed(d["wordmark"], L.LIGHT), "wd": L.fixed(d["wordmark"], L.DARK),
                      "ml": L.fixed(d["mark"], L.LIGHT), "md": L.fixed(d["mark"], L.DARK),
                      "il": d["icon"], "id": d["icon_dark"]}
        names[slug] = f"{slug[:2]} {d['name']}"
    wl, wd = now_variants()
    data["now"] = {"wl": wl, "wd": wd, "il": (NOW / "icon.svg").read_text(), "id": (NOW / "icon-dark.svg").read_text()}
    # "</" would end the <script> it travels in
    blob = json.dumps(data, separators=(",", ":")).replace("</", "<\\/")

    jump = "".join(f'<a href="#d-{d["slug"]}" data-slug="{d["slug"]}" title="{esc(d["name"])}">{d["slug"][:2]}</a>' for d in assets)

    dock = ['<button class="dk" type="button" data-slug="now" aria-label="Today\'s icon"><img data-a="now:i?" alt="">'
            '<span class="tipname">Today</span><span class="run"></span></button><span class="dk sep" aria-hidden="true"></span>']
    for d in assets:
        dock.append(f'<button class="dk" type="button" data-slug="{d["slug"]}" aria-label="{esc(names[d["slug"]])}">'
                    f'<img data-a="{d["slug"]}:i?" alt=""><span class="tipname">{esc(names[d["slug"]])}</span></button>')

    cards, dirs = [], []
    for d in assets:
        slug, n, name = d["slug"], d["slug"][:2], d["name"]
        star = (f'<button class="star" type="button" data-slug="{slug}" aria-pressed="false" aria-label="Shortlist {esc(name)}">'
                f'<span class="k" aria-hidden="true"></span><span class="t">Shortlist</span></button>')
        cards.append(
            f'<article class="card" id="c-{slug}" data-slug="{slug}">'
            f'<div class="card-head"><span class="n">{n}</span><h3 class="serif"><a href="#d-{slug}">{esc(name)}</a></h3>{star}</div>'
            f'<div class="pair"><div class="on-mac-l"><img data-a="{slug}:il" width="128" height="128" alt="{esc(name)} icon, light"></div>'
            f'<div class="on-mac-d"><img data-a="{slug}:id" width="128" height="128" alt="{esc(name)} icon, dark"></div></div>'
            f'<div class="strip" style="background:var(--surface);border:1px solid var(--line)">{wm_img(slug + ":w?", d["wordmark"], 48)}</div>'
            f'<p class="idea">{esc(d["idea"])}</p><p class="risk"><b>Risk.</b> {esc(d["risk"])}</p></article>')

        ladder = []
        for theme, cls, k in (("Light", "on-mac-l", "il"), ("Dark", "on-mac-d", "id")):
            figs = "".join(f'<figure><img data-a="{slug}:{k}" width="{s}" height="{s}" alt="{esc(name)} icon at {s} pt, {theme.lower()}">'
                           f'<figcaption>{s}</figcaption></figure>' for s in (256, 128, 64, 32, 16))
            loupes = "".join(f'<figure class="loupe"><canvas data-loupe="{slug}:{k}:{px}" width="{px}" height="{px}" '
                             f'style="width:{px * 4}px;height:{px * 4}px" aria-label="{esc(name)} at {px} px, magnified four times"></canvas>'
                             f'<figcaption>{px} px, ×4</figcaption></figure>' for px in (32, 16))
            ladder.append(f'<div class="ladder {cls}">{figs}<span class="gap"></span>{loupes}</div>')

        def spec(cls, w, m, theme):
            return (f'<div class="spec {cls}"><span class="cap">{theme}</span>'
                    f'<div class="row">{wm_img(f"{slug}:{w}", d["wordmark"], 72)}</div>'
                    f'<div class="row">{wm_img(f"{slug}:{w}", d["wordmark"], 19)}{wm_img(f"{slug}:{w}", d["wordmark"], 13)}'
                    f'<span class="gap" style="flex:1"></span>'
                    + "".join(f'<img data-a="{slug}:{m}" width="{s}" height="{s}" alt="{esc(name)} mark at {s} px">' for s in (96, 32, 16))
                    + '</div><span class="cap">Wordmark at 72, 19 and 13 px; the mark alone at 96, 32 and 16 px</span></div>')

        dirs.append(
            f'<section class="dir wrap" id="d-{slug}" aria-labelledby="t-{slug}">'
            f'<div class="dir-head"><span class="n">{n}</span><div class="dir-text"><h2 class="serif" id="t-{slug}">{esc(name)}</h2>'
            f'<p><span class="lab">Object</span><span>{esc(d["object"])}</span></p>'
            f'<p class="idea"><span class="lab">Idea</span><span>{esc(d["idea"])}</span></p>'
            f'<p class="risk"><span class="lab">Risk</span><span>{esc(d["risk"])}</span></p></div>'
            f'<div class="pick">{star}<label class="sr" for="n-{slug}">Pointers for {esc(name)}</label>'
            f'<textarea id="n-{slug}" data-note="{slug}" placeholder="Pointers: what to keep, change or combine"></textarea></div></div>'
            f'<div class="block"><span class="lab">The icon at 1024 px, light and dark, shown at 512 pt: one to one on a Retina screen</span>'
            f'<div class="big"><figure class="on-mac-l"><img data-a="{slug}:il" width="512" height="512" alt="{esc(name)} icon, light, 1024 px"></figure>'
            f'<figure class="on-mac-d"><img data-a="{slug}:id" width="512" height="512" alt="{esc(name)} icon, dark, 1024 px"></figure></div></div>'
            f'<div class="block"><span class="lab">Real sizes in points. The magnified squares are the true pixels: a 16 pt Finder icon is drawn from the 32 px one on a Retina screen, from the 16 px one on a 1× screen</span>'
            f'{"".join(ladder)}</div>'
            f'<div class="block"><span class="lab">Wordmark and mark, on linen and lamplight</span>'
            f'<div class="words">{spec("on-linen", "wl", "ml", "Linen")}{spec("on-lamp", "wd", "md", "Lamplight")}</div></div>'
            f'<div class="block"><span class="lab">In place: the title bar (wordmark at 19 px), the Dock (64 pt), a Finder list (16 pt; click a row to select it)</span>'
            f'<div class="ctx">{titlebar(slug, d["wordmark"], "l")}{titlebar(slug, d["wordmark"], "d")}'
            f'{dock_small(slug, "l")}{dock_small(slug, "d")}'
            f'{finder(slug, name, "l", False)}{finder(slug, name, "d", True)}</div></div>'
            f'</section>')

    sq = superellipse(50, 50, 412 / 1024 * 100, 5.0, 96)
    page = (HERE / "logo_page.src.html").read_text()
    for k, v in (("{{JUMP}}", jump), ("{{DOCK}}", "".join(dock)), ("{{CARDS}}", "".join(cards)),
                 ("{{DIRS}}", "\n".join(dirs)), ("{{NAMES}}", json.dumps(names)), ("{{SQUIRCLE}}", json.dumps(sq)),
                 ("{{DATA}}", blob)):
        page = page.replace(k, v)
    OUT.write_text(page)
    print(f"page {OUT} {len(page) // 1024} KB")

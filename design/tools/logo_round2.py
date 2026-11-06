# /// script
# dependencies = ["fonttools", "uharfbuzz"]
# ///
"""Round 2 of the logo directions: the darn, the dot on the i, and three follow-ups.

What a darn means here: a darn is made of two threads. One is laid across the gap
first (the warp), then a second is woven through it, over and under (the weft). In
Inkling the warp is madder, Inkling's question laid across the place your words don't
hold, and the weft is ink, your answer woven through it. The mend stays visible, the
way visible mending is meant to: a repair that is proud of itself.

Every darn variant here points at something by where the darn sits, never by a
caption: a word, the gap still open, the place a plan leans on, the margin, the record.

Writes ../logo/round-2/<slug>/{wordmark,mark,icon,icon-dark}.svg and
../logo/round-2/dots/<slug>.svg; logo_page.py puts round 2 at the top of the page.
"""
import math
import random

import logo_directions as L
from logo_directions import (DARK, LIGHT, catmull, cloth_for, cross, glyphs, hi, icon_svg, running, shade, stitch, thread,
                             wm_view)
from logos import f, resample

OUT = L.OUT / "round-2"


# ================================================================ the weave

def weave(x0, y0, x1, y1, nx, ny, warp, weft, warp_hi=None, weft_hi=None, ratio=.86, rows=None, weft_ratio=None,
          over=lambda i, j: (i + j) % 2 == 0, ext=(.22, .58), seed=7, loops=False, partial=None, edge=.26):
    """A darn over the box (x0, y0)-(x1, y1).

    The warp is one thread laid back and forth across the gap, each pass running on past
    the gap into sound cloth by a hand-made amount, turning in a small loop. The weft is
    one thread woven through it, row by row, turning in a small loop at each side: darners
    leave those loops so the darn does not pucker when it is washed. `over(i, j)` says
    whether warp i lies over weft row j (plain weave by default). `rows` stops the weaving
    early; `partial` is how far (0..1, from the right) the next row has got.
    """
    rnd = random.Random(seed)
    px, py = (x1 - x0) / nx, (y1 - y0) / ny
    twx, twy = px * ratio, py * (weft_ratio or ratio)
    xs = [x0 + (i + .5) * px for i in range(nx)]
    ys = [y0 + (j + .5) * py for j in range(ny)]
    body = []
    # the warp: down, loop, up, loop... each pair of passes shares the end where it turns
    ends = {}
    for i in range(nx - 1):
        side = "bot" if i % 2 == 0 else "top"
        e = py * rnd.uniform(*ext)
        ends[(i, side)] = ends[(i + 1, side)] = e
    for i, x in enumerate(xs):
        top = y0 - ends.get((i, "top"), py * .4)
        bot = y1 + ends.get((i, "bot"), py * .4)
        body.append(stitch((x, top), (x, bot), twx, warp, warp_hi, end=.42, mid=.5))
    if loops:
        for i in range(nx - 1):
            side = "bot" if i % 2 == 0 else "top"
            e = ends[(i, side)]
            yy = y1 + e if side == "bot" else y0 - e
            sgn = 1 if side == "bot" else -1
            pts = [(xs[i] + (xs[i + 1] - xs[i]) * (.5 - .5 * math.cos(math.pi * t / 12)),
                    yy + sgn * math.sin(math.pi * t / 12) * px * .38) for t in range(13)]
            body.append(thread(pts, twx * .62, warp, (0, 0)))
    # the weft, row by row, and the loops where it turns
    n_rows = ny if rows is None else rows
    lx, rx = x0 - px * edge, x1 + px * edge
    for j in range(n_rows + (1 if partial else 0)):
        y = ys[j]
        if j == n_rows:
            # the row being woven now: from the right edge to where the needle has got
            a = rx - (rx - lx) * partial
            body.append(stitch((a, y), (rx, y), twy, weft, weft_hi, end=.42))
        else:
            body.append(stitch((lx, y), (rx, y), twy, weft, weft_hi, end=.42))
        if loops and j + 1 < n_rows + (1 if partial else 0):
            right = j % 2 == 0
            xx = rx if right else lx
            sgn = 1 if right else -1
            pts = [(xx + sgn * math.sin(math.pi * t / 12) * py * .38, ys[j] + (ys[j + 1] - ys[j]) * (.5 - .5 * math.cos(math.pi * t / 12)))
                   for t in range(13)]
            body.append(thread(pts, twy * .62, weft, (0, 0)))
    # where the warp lies on top, draw that piece of it back over the weft
    for j in range(n_rows + (1 if partial else 0)):
        for i, x in enumerate(xs):
            if j == n_rows and x < rx - (rx - lx) * partial:
                continue
            if over(i, j):
                body.append(stitch((x, ys[j] - py * .55), (x, ys[j] + py * .55), twx, warp, warp_hi, end=.46, mid=.5))
    return "".join(body), xs, ys


def hole_shape(cx, cy, rx, ry, fill, fray=None, seed=3, n=16):
    """A worn-through hole: an uneven opening, its rim frayed into short thread ends."""
    rnd = random.Random(seed)
    pts = []
    for k in range(n):
        a = 2 * math.pi * k / n
        r = 1 + (rnd.random() - .5) * .24
        pts.append((cx + math.cos(a) * rx * r, cy + math.sin(a) * ry * r))
    ring = catmull(pts + pts[:1], 8)
    body = f'<path fill="{fill}" d="M{" L".join(f"{f(x)} {f(y)}" for x, y in ring)}Z"/>'
    if fray:
        segs = []
        for k in range(0, len(ring) - 1, 3):
            x, y = ring[k]
            dx, dy = x - cx, y - cy
            ln = math.hypot(dx, dy) or 1
            ux, uy = dx / ln, dy / ln
            fl = rx * (.05 + rnd.random() * .08)
            segs.append(f"M{f(x + ux * 3)} {f(y + uy * 3)}L{f(x - ux * fl)} {f(y - uy * fl)}")
        body += (f'<path stroke="{fray}" stroke-width="{f(rx * .024)}" stroke-linecap="round" fill="none" '
                 f'd="{"".join(segs)}"/>')
    return body


def cls_svg(svg, mapping):
    """Swap literal placeholder colours for classes, so a flat mark follows light and dark."""
    for k, v in mapping.items():
        svg = svg.replace(f'fill="{k}"', f'class="{v}"').replace(f'stroke="{k}"', f'class="s-{v}"')
    return svg


# placeholder colours for flat marks: replaced by classes
W_, I_, WH, IH, P_, H_ = "#W00001", "#I00001", "#W00002", "#I00002", "#P00001", "#H00001"
FLAT = {W_: "mad", I_: "ink", WH: "mad-hi", IH: "ink-hi", P_: "paper", H_: "hole"}


def style_r2(pal):
    """Round 1's classes, and the few round 2 adds: the light on ink thread, a hole."""
    dark = pal is DARK
    return (L.style_for(pal) + " .ink-hi{fill:" + shade(pal["ink"], -.22 if dark else .22) + "}"
            " .s-ink-hi{stroke:" + shade(pal["ink"], -.22 if dark else .22) + "}"
            " .s-mad-hi{stroke:" + shade(pal["madder"], .18) + "}"
            " .hole{fill:" + ("#3A343F" if dark else "#BFB19A") + "}")


def flat(view, body, label="Inkling"):
    """A flat mark that follows the reader's light or dark setting on its own."""
    x, y, w, h = view
    st = style_r2(LIGHT) + " @media (prefers-color-scheme:dark){" + style_r2(DARK) + "}"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{f(x)} {f(y)} {f(w)} {f(h)}" width="{f(w)}" height="{f(h)}" '
            f'role="img" aria-label="{label}"><style>{st}</style>{cls_svg(body, FLAT)}</svg>')


def fixed(svg, pal):
    """The same flat mark with its colours fixed to one palette (for the review page)."""
    import re
    return re.sub(r"<style>.*?</style>", "<style>" + style_r2(pal) + "</style>", svg, flags=re.S)


# ================================================================ the dot on the i

def dot_font(d):
    return None  # the word is set with its own dot


def dot_thread_end(d):
    """No dot: a cut end of madder comes up through the cloth where the dot would be, its
    tip frayed the way a cut thread is, before it is tied in."""
    cx, cy, w = d["cx"], d["cy"], d["w"]
    base, tip = (cx - w * .1, cy + w * .66), (cx + w * .26, cy - w * .66)
    hole = f'<circle cx="{f(base[0])}" cy="{f(base[1])}" r="{f(w * .13)}" fill="{H_}"/>'
    pts = catmull([base, (cx - w * .2, cy + w * .24), (cx + w * .12, cy - w * .16), tip], 12)
    body = hole + thread(pts, w * .3, W_, (0.0, 0.05))
    for dx, dy in ((-.16, -.26), (.08, -.3), (.26, -.18)):
        body += thread([tip, (tip[0] + dx * w * .5, tip[1] + dy * w * .5), (tip[0] + dx * w, tip[1] + dy * w)], w * .08, W_, (0.0, .5))
    return body


def dot_darn(d):
    """A tiny darn: three passes of madder laid down, three rows of ink woven through,
    trimmed square, so it reads as a woven dot and not as a grid."""
    cx, cy, w = d["cx"], d["cy"], d["w"] * 1.06
    body, _, _ = weave(cx - w / 2, cy - w / 2, cx + w / 2, cy + w / 2, 3, 3, W_, I_, ratio=1.0, ext=(.02, .06), edge=.04)
    return body


def dot_cross(d):
    """One cross-stitch, the app's correction mark, top leg lying the same way as ever."""
    cx, cy, w = d["cx"], d["cy"] + d["w"] * .05, d["w"]
    return cross(cx, cy, w * .46, w * .34, W_)


def dot_stitch(d):
    """One stitch, upright, the way it goes into the cloth: the first stitch of everything
    that follows. Upright, because a slanted dash over an i reads as an accent."""
    cx, cy, w = d["cx"], d["cy"], d["w"]
    return stitch((cx + w * .04, cy - w * .66), (cx - w * .04, cy + w * .58), w * .62, W_)


def dot_eye(d):
    """The eye of a needle in place of the dot, with Inkling's thread through it."""
    cx, cy, w = d["cx"], d["cy"] - d["w"] * .08, d["w"]
    rx, ry, sw = w * .36, w * .62, w * .2
    ring = (f'<path fill="{I_}" fill-rule="evenodd" d="M{f(cx - rx)} {f(cy)} A{f(rx)} {f(ry)} 0 1 1 {f(cx + rx)} {f(cy)} '
            f'A{f(rx)} {f(ry)} 0 1 1 {f(cx - rx)} {f(cy)}Z M{f(cx - rx + sw)} {f(cy)} A{f(rx - sw)} {f(ry - sw)} 0 1 1 {f(cx + rx - sw)} {f(cy)} '
            f'A{f(rx - sw)} {f(ry - sw)} 0 1 1 {f(cx - rx + sw)} {f(cy)}Z"/>')
    back = thread([(cx + w * .02, cy - ry * .2), (cx + w * .5, cy - ry * .55), (cx + w * .78, cy - ry * .45)], w * .2, W_, (0, .4))
    front = thread(catmull([(cx - w * .02, cy + ry * .1), (cx - w * .3, cy + ry * .5), (cx - w * .62, cy + ry * .58)], 8), w * .2, W_, (0, .4))
    return back + ring + front


DOTS = [
    ("font", "The font's own dot", "Fraunces' round dot in ink. The mark lives elsewhere; the name is simply set.", dot_font),
    ("thread-end", "A thread end", "No dot: a cut end of madder comes up through the cloth where the dot would be.", dot_thread_end),
    ("darn", "A tiny darn", "Three passes of madder, three rows of ink woven through: the darn, at its smallest.", dot_darn),
    ("cross", "A cross-stitch", "One cross-stitch, the app's own correction mark, as the dot.", dot_cross),
    ("stitch", "One stitch", "One stitch, upright, the way it goes into the cloth: the first of everything that follows.", dot_stitch),
    ("eye", "A needle's eye", "The dot is the eye the thread goes through, Inkling's thread still in it.", dot_eye),
]


def wordmark(dot, x0=0.0, extra="", extra_x1=0.0):
    """The name set in Fraunces Display with one answer to the dot of the first i."""
    if dot == "font":
        word, info = glyphs("inkling", x0, 0)
        body = word
    else:
        word, info = glyphs("inkling", x0, 0, dotless_first_i=True)
        fn = dict((k, fn) for k, _, _, fn in DOTS)[dot]
        body = word + fn(info["dot"])
    return body + extra, info


def dot_wordmark(dot):
    body, info = wordmark(dot)
    return flat(wm_view(0, info["width"]), body)


# ================================================================ the darns

def colours(pal, dark):
    m, ink = pal["madder"], pal["ink"]
    return dict(m=m, mh=hi(m, .18), ink=ink, ih=shade(ink, -.24 if dark else .24), page=pal["paper"], cloth=pal["cloth"])


DARNS = []


def darn_variant(fn):
    DARNS.append(fn)
    return fn


def word_bars(bars, y, h, fill, r=None):
    r = h * .22 if r is None else r
    return "".join(f'<rect x="{f(a)}" y="{f(y - h / 2)}" width="{f(b - a)}" height="{f(h)}" rx="{f(r)}" fill="{fill}"/>' for a, b in bars)


def word_scene(c, lines, darn_line, darn_x, h, nx, ny, hole_fill, hi_on=True, seed=5):
    """A paragraph of your writing with one word darned. The words around it are your
    ink; the darn sits exactly where the weak word was, a little taller than the line,
    because a darn always runs on into sound cloth."""
    body = ""
    for k, (y, bars) in enumerate(lines):
        body += word_bars(bars, y, h, c["ink"])
    y = lines[darn_line][0]
    x0, x1 = darn_x
    pad = h * .36
    body += hole_shape((x0 + x1) / 2, y, (x1 - x0) * .42, h * .62, hole_fill, None, seed=seed, n=12)
    w, _, _ = weave(x0, y - h / 2 - pad, x1, y + h / 2 + pad, nx, ny, c["m"], c["ink"],
                    c["mh"] if hi_on else None, c["ih"] if hi_on else None, seed=seed)
    return body + w


@darn_variant
def darn_word():
    # a close crop of a paragraph: the lines run off the tile; the darned word is the
    # fourth in the middle line
    lines = [(352, [(40, 238), (282, 520), (564, 690), (734, 1000)]),
             (512, [(40, 190), (234, 330)]),
             (672, [(40, 300), (344, 560), (604, 1000)])]
    def icon(pal, dark):
        c = colours(pal, dark)
        lz = [(y, list(b) + ([(722, 1000)] if k == 1 else [])) for k, (y, b) in enumerate(lines)]
        body = word_scene(c, lz, 1, (374, 678), 72, 7, 5, shade(c["page"], -.3 if not dark else .12))
        return icon_svg(c["page"], body)
    ml = [(30, [(6, 26), (30, 52), (56, 94)]), (50, [(6, 30), (34, 44)]), (70, [(6, 36), (40, 66)])]
    mark = word_scene(dict(m=W_, mh=None, ink=I_, ih=None), ml, 1, (49, 80), 9, 5, 5, H_, hi_on=False)
    wm, info = wordmark("darn")
    return dict(slug="darn-word", name="Word", dot="darn",
                means="One word in your line didn't hold. Inkling laid its threads across the gap, and your own ink was woven through them.",
                points="At the word: the darn sits in the paragraph exactly where the weak word was, the sentence intact either side.",
                risk="Bars for words can read as a generic document icon; the darn has to carry it at 16 px.",
                wordmark=flat(wm_view(0, info["width"]), wm), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


def needle_h(x_tip, x_eye, y, w, steel, steel_hi, eye_fill):
    """A needle lying along a row, eye to the right, point to the left."""
    hw = w / 2
    L_ = x_eye - x_tip
    taper = x_tip + L_ * .3
    d = (f"M{f(x_tip)} {f(y)} Q{f(x_tip + L_ * .12)} {f(y - hw * .55)} {f(taper)} {f(y - hw * .92)} L{f(x_eye - hw)} {f(y - hw)} "
         f"A{f(hw)} {f(hw)} 0 0 1 {f(x_eye - hw)} {f(y + hw)} L{f(taper)} {f(y + hw * .92)} Q{f(x_tip + L_ * .12)} {f(y + hw * .55)} {f(x_tip)} {f(y)}Z")
    ex0, ex1 = x_eye - hw - L_ * .14, x_eye - hw * .9
    eye = (f'<rect x="{f(ex0)}" y="{f(y - hw * .32)}" width="{f(ex1 - ex0)}" height="{f(hw * .64)}" rx="{f(hw * .32)}" fill="{eye_fill}"/>')
    shine = f'<rect x="{f(taper)}" y="{f(y - hw * .62)}" width="{f(x_eye - hw - taper - L_ * .2)}" height="{f(hw * .3)}" rx="{f(hw * .15)}" fill="{steel_hi}" opacity=".8"/>'
    return f'<path fill="{steel}" d="{d}"/>' + shine + eye, (ex0 + ex1) / 2


def hole_scene(c, cx, cy, s, steel, steel_hi, hole_fill, fray, hi_on=True, needle=True, seed=4):
    """A hole half darned: every pass of the warp laid across it, three rows woven, the
    fourth under way, the needle lying in that row and pointing at what is still open.
    The warp is laid with gaps, so below the woven rows you can see the hole itself."""
    nx, ny = 6, 7
    x0, x1, y0, y1 = cx - s * .44, cx + s * .44, cy - s * .48, cy + s * .48
    body = hole_shape(cx + s * .01, cy + s * .1, s * .4, s * .33, hole_fill, fray, seed=seed)
    w, xs, ys = weave(x0, y0, x1, y1, nx, ny, c["m"], c["ink"], c["mh"] if hi_on else None, c["ih"] if hi_on else None,
                      rows=3, partial=.42, seed=seed, ratio=.56, weft_ratio=.9)
    body += w
    if needle:
        py = (y1 - y0) / ny
        px = (x1 - x0) / nx
        lx, rx = x0 - px * .26, x1 + px * .26
        a = rx - (rx - lx) * .42
        y = ys[3]
        nd, eye_x = needle_h(x0 - s * .24, a + s * .1, y, py * .78, steel, steel_hi, hole_fill)
        body += nd
        body += thread([(a + 2, y), (eye_x, y)], py * .5, c["ink"], (0, 0))
        for i, x in enumerate(xs):
            if x < a and (i + 3) % 2 == 0:
                body += stitch((x, y - py * .56), (x, y + py * .56), px * .56, c["m"], c["mh"] if hi_on else None, end=.46)
    return body


@darn_variant
def darn_hole():
    def icon(pal, dark):
        c = colours(pal, dark)
        steel = "#C3C8CF" if not dark else "#C3C8CF"
        body = hole_scene(c, 520, 520, 470, steel, "#E3E6EA" if not dark else "#F1F3F6",
                          "#2E2823" if not dark else "#0C0B0E", shade(c["cloth"], -.24 if not dark else .14))
        return icon_svg(c["cloth"], body, cloth=cloth_for(pal, dark))
    mark = hole_scene(dict(m=W_, mh=None, ink=I_, ih=None, cloth="#FFFFFF00"), 54, 50, 80, "#S00001", P_, H_, None, hi_on=False)
    mark = mark.replace('fill="#S00001"', 'class="steel"')
    wm, info = wordmark("eye")
    return dict(slug="darn-hole", name="Hole", dot="eye",
                means="Inkling lays the warp across the hole; the weaving is yours. The mark is the mend halfway, the needle still in it.",
                points="At what is still open: the needle lies in the unfinished row, its point aimed at the part of the hole not yet woven.",
                risk="An open hole can read as damage rather than repair, and below 32 px the needle disappears.",
                wordmark=flat(wm_view(0, info["width"]), wm), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


def sock_outline(cx, cy, s):
    """A sock side on: the leg down from the cuff, the heel, the foot out to the toe."""
    pts = [(-.34, -.86), (.12, -.86), (.13, -.06), (.2, .1), (.5, .14), (.74, .22), (.84, .4), (.78, .58), (.56, .66),
           (-.02, .66), (-.3, .62), (-.44, .46), (-.46, .2), (-.38, -.1), (-.34, -.86)]
    return catmull([(cx + x * s, cy + y * s) for x, y in pts], 10)


def heel_scene(c, cx, cy, s, wool, wool_lo, wool_hi, weft, hi_on=True, clip_id="sk"):
    """A wool sock darned at the heel: the place that takes the weight wears first."""
    out = sock_outline(cx, cy, s)
    d = "M" + " L".join(f"{f(x)} {f(y)}" for x, y in out) + "Z"
    body = f'<clipPath id="{clip_id}"><path d="{d}"/></clipPath><path fill="{wool}" d="{d}"/>'
    # stocking stitch shows as columns; the cuff is ribbed; the toe is knitted darker
    cols = "".join(f"M{f(cx + x * s)} {f(cy - .9 * s)}V{f(cy + .7 * s)}" for x in [k / 20 for k in range(-9, 18)])
    body += f'<g clip-path="url(#{clip_id})"><path stroke="{wool_lo}" stroke-width="{f(s * .006)}" opacity=".35" d="{cols}"/>'
    body += f'<rect x="{f(cx - .5 * s)}" y="{f(cy - .9 * s)}" width="{f(.7 * s)}" height="{f(.2 * s)}" fill="{wool_lo}" opacity=".45"/>'
    ribs = "".join(f"M{f(cx + x * s)} {f(cy - .9 * s)}V{f(cy - .7 * s)}" for x in [k / 30 for k in range(-12, 6)])
    body += f'<path stroke="{wool_hi}" stroke-width="{f(s * .012)}" opacity=".55" d="{ribs}"/>'
    body += f'<path fill="{wool_lo}" d="M{f(cx + .6 * s)} {f(cy + .1 * s)} Q{f(cx + .95 * s)} {f(cy + .3 * s)} {f(cx + .78 * s)} {f(cy + .7 * s)} L{f(cx + .58 * s)} {f(cy + .7 * s)}Z" opacity=".7"/>'
    # the darn on the heel, turned to follow its curve
    hx, hy = cx - .2 * s, cy + .38 * s
    w, _, _ = weave(hx - .24 * s, hy - .2 * s, hx + .24 * s, hy + .2 * s, 6, 5, c["m"], weft,
                    c["mh"] if hi_on else None, shade(weft, -.2) if (hi_on and weft.startswith("#")) else None, seed=9)
    body += f'<g transform="rotate(-24 {f(hx)} {f(hy)})">{w}</g></g>'
    return body


@darn_variant
def darn_heel():
    def icon(pal, dark):
        c = colours(pal, dark)
        wool = "#6E4B2E" if not dark else "#7A5639"
        body = heel_scene(c, 500, 540, 440, wool, shade(wool, -.12), shade(wool, .14), c["page"] if not dark else "#F1E9DC")
        return icon_svg(c["cloth"], body, cloth=cloth_for(pal, dark))
    mark = heel_scene(dict(m=W_, mh=None), 46, 52, 54, "#WAL001", "#WAL002", "#WAL003", P_, hi_on=False, clip_id="skm")
    mark = mark.replace('fill="#WAL001"', 'class="wal"').replace('stroke="#WAL002"', 'class="s-ink"').replace('fill="#WAL002"', 'class="ink"').replace('stroke="#WAL003"', 'class="s-ink"')
    wm, info = wordmark("stitch")
    return dict(slug="darn-heel", name="Heel", dot="stitch",
                means="A plan wears through where it takes the most weight. Inkling darns the claim the rest of it stands on.",
                points="At the load: the darn is on the heel, the one place a sock wears through, because everything rests on it.",
                risk="A sock is cute, and says laundry or clothing before it says writing.",
                wordmark=flat(wm_view(0, info["width"]), wm), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


def margin_scene(c, page, cloth, edge_x, lines, darn_box, y_mid, h, nx, ny, hi_on=True, shadow=None, w_total=1024, top=0, bottom=1024):
    """The page and its margin: your lines on the paper, the margin cloth to the right.
    The darn closes the end of one line and runs on across the page's edge into the
    margin, where the question was asked: one mend, half yours, half Inkling's."""
    body = f'<rect x="{f(edge_x)}" y="{f(top)}" width="{f(w_total - edge_x)}" height="{f(bottom - top)}" fill="{cloth}"/>'
    if shadow:
        body += f'<rect x="{f(edge_x)}" y="{f(top)}" width="{f(h * .1)}" height="{f(bottom - top)}" fill="{shadow}" opacity=".5"/>'
    for y, bars in lines:
        body += word_bars(bars, y, h, c["ink"])
    x0, x1 = darn_box
    pad = (ny - 3) / 2 * (h / 3)
    w, _, _ = weave(x0, y_mid - h / 2 - pad, x1, y_mid + h / 2 + pad, nx, ny, c["m"], c["ink"],
                    c["mh"] if hi_on else None, c["ih"] if hi_on else None, seed=11)
    return body + w


@darn_variant
def darn_margin():
    def icon(pal, dark):
        c = colours(pal, dark)
        lines = [(338, [(-40, 170), (210, 400), (440, 560)]), (512, [(-40, 120), (160, 330), (370, 470)]), (686, [(-40, 210), (250, 420)])]
        body = margin_scene(c, c["page"], pal["cloth"] if not dark else "#141216", 640, lines, (500, 780), 512, 70, 7, 5,
                            shadow=shade(pal["cloth"], -.12 if not dark else -.05))
        return icon_svg(c["page"], body, cloth=None)
    lines = [(28, [(-4, 30), (34, 58)]), (50, [(-4, 22), (26, 44)]), (72, [(-4, 34), (38, 56)])]
    mark = margin_scene(dict(m=W_, mh=None, ink=I_, ih=None), P_, H_, 62, lines, (48, 78), 50, 12, 5, 5, hi_on=False,
                        w_total=100, top=8, bottom=92)
    wm, info = wordmark("thread-end")
    return dict(slug="darn-margin", name="Margin", dot="thread-end",
                means="Every mend reaches back to the question that asked for it: the darn joins your line to the margin.",
                points="At the margin: the darn closes the end of one line and runs on across the page's edge into the margin cloth.",
                risk="Two grounds and three lines are a lot for 16 px, where it can read as a document icon with a tab.",
                wordmark=flat(wm_view(0, info["width"]), wm), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


def border_scene(c, top, x0, x1, n, size, gap, body_lines, line_fill, hi_on=True, h=None, last_rows=3):
    """A note that has been worked through: along its top edge the border Inkling adds to,
    one darn for every question answered, each in a different weave, the way a darning
    sampler kept every kind of mend side by side. The last one is still being woven."""
    plain = lambda i, j: (i + j) % 2 == 0  # noqa: E731
    twill = lambda i, j: (i + j) % 4 < 2  # noqa: E731
    twill_r = lambda i, j: (i - j) % 4 < 2  # noqa: E731
    basket = lambda i, j: ((i // 2) + (j // 2)) % 2 == 0  # noqa: E731
    weaves = [plain, twill, basket, twill_r, plain, twill]
    body = ""
    h = h or size * .1
    for y, bars in body_lines:
        body += word_bars(bars, y, h, line_fill)
    for k in range(n):
        x = x0 + k * (size + gap)
        last = k == n - 1
        w, _, _ = weave(x, top, x + size, top + size, 5, 5, c["m"], c["ink"], c["mh"] if hi_on else None,
                        c["ih"] if hi_on else None, over=weaves[k % len(weaves)], ext=(.18, .42), seed=20 + k,
                        rows=last_rows if last else None, partial=.5 if last else None)
        body += w
    return body


@darn_variant
def darn_border():
    def icon(pal, dark):
        c = colours(pal, dark)
        lines = [(468, [(176, 382), (420, 560), (598, 848)]), (560, [(176, 300), (338, 590), (628, 760)]),
                 (652, [(176, 452), (490, 690)]), (744, [(176, 330), (368, 600), (638, 848)]), (836, [(176, 420)])]
        body = border_scene(c, 196, 168, 856, 4, 148, 32, lines, "#D6CBB8" if not dark else "#423D47", h=36)
        return icon_svg(c["page"], body)
    ml = [(62, [(8, 40), (46, 92)]), (76, [(8, 30), (36, 70)]), (90, [(8, 52)])]
    mark = border_scene(dict(m=W_, mh=None, ink=I_, ih=None), 10, 6, 96, 4, 18.5, 4.6, ml, H_, hi_on=False, h=6)
    wm, info = wordmark("cross")
    return dict(slug="darn-border", name="Border", dot="cross",
                means="Every question answered adds a darn to the note's border, each in a different weave: a finished note shows its mends.",
                points="At the record: the border along the top of a note, one darn per answer, the newest still on the needle.",
                risk="At 16 px the band reads as the top of a notepad, which helps, but the darns become one red stripe.",
                wordmark=flat(wm_view(0, info["width"]), wm), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ tied off

TIED = []


def tied_variant(fn):
    TIED.append(fn)
    return fn


def knot_end(x, y, scale, w, fill, hi_=None, tail=1.0, hang=False, turn=0.0):
    """An overhand knot tied at the end of a thread that arrives from the left; its loose
    end leaves to the right. Drawn so the crossings read over, under, over. `hang` mirrors
    it so its loop hangs below the thread instead of rising across the line above."""
    pts = L.overhand(x, y, scale, tails=tail)
    if hang:
        pts = [(px, 2 * y - py) for px, py in pts]
    if turn:
        pts = L.rot(pts, turn, x, y)
    return L.tied(pts, w, fill, hi_)


def settled_scene(c, lines, line_i, word, h, sw, kscale, kw, hi_on=True):
    """The knot that settled a word: a running stitch under the word, tied off in a knot
    at its end. The stitches stay; the knot says this one is done."""
    body = "".join(word_bars(b, y, h, c["ink"]) for y, b in lines)
    y = lines[line_i][0] + h * .5 + sw * 2.1
    x0, x1 = word
    body += running([(x0, y), (x1 - sw * .6, y)], sw, c["m"], c["mh"] if hi_on else None, sw * 3.6, sw * 2.0, seed=6)
    body += knot_end(x1 + kscale * .5, y - kscale * .05, kscale, kw, c["m"], c["mh"] if hi_on else None, tail=.42, hang=True)
    return body


@tied_variant
def tied_settled():
    lines = [(330, [(60, 270), (310, 470), (510, 1000)]),
             (470, [(60, 200), (240, 520), (700, 1000)]),
             (730, [(60, 380), (420, 600)])]

    def icon(pal, dark):
        c = colours(pal, dark)
        body = settled_scene(c, lines, 1, (240, 520), 56, 22, 62, 22)
        return icon_svg(c["page"], body)
    ml = [(24, [(4, 30), (34, 60), (64, 96)]), (44, [(4, 20), (24, 56), (78, 96)]), (78, [(4, 40), (44, 66)])]
    mark = settled_scene(dict(m=W_, mh=None, ink=I_), ml, 1, (24, 56), 9, 3.2, 9, 3.2, hi_on=False)
    word, info = wordmark("font")
    y = 14.0
    under = running([(4, y), (info["width"] - 2, y)], 3.0, W_, None, 10.5, 6.2, seed=4)
    under += knot_end(info["width"] + 10, y, 8.5, 3.0, W_, None, tail=.42, hang=True)
    return dict(slug="tied-settled", name="Settled", dot="font",
                means="The knot is tied where a question was settled: it marks the word, and the stitches under it stay as the record.",
                points="At the word it settled: the running stitch under the word ends in the knot, right where the word ends.",
                risk="Lines and a red underline can read as a spell-checked document; the knot has to be big enough to matter.",
                wordmark=flat(wm_view(0, info["width"] + 26, y1=44), word + under), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


def margin_tie_scene(c, lines, line_i, word, h, sw, rope_to, kscale, kw, hi_on=True, rule_x=None, rule_fill=None):
    """The loose end leads to the margin: a running stitch under the word, then the
    thread sags across to the margin, where the question was, and is tied off there."""
    body = ""
    if rule_x is not None:
        body += f'<rect x="{f(rule_x)}" y="-20" width="{f(h * .06)}" height="1100" fill="{rule_fill}"/>'
    body += "".join(word_bars(b, y, h, c["ink"]) for y, b in lines)
    y = lines[line_i][0] + h * .5 + sw * 2.1
    x0, x1 = word
    body += running([(x0, y), (x1, y)], sw, c["m"], c["mh"] if hi_on else None, sw * 3.6, sw * 2.0, seed=8)
    kx, ky = rope_to
    sag = catmull([(x1 + sw, y), (x1 + (kx - x1) * .35, y + (ky - y) * .2 + h * 1.4), (x1 + (kx - x1) * .75, ky + h * .9),
                   (kx - kscale * 2.6, ky + kscale * .85)], 14)
    body += thread(sag, kw, c["m"], (0, 0), c["mh"] if hi_on else None)
    body += knot_end(kx, ky, kscale, kw, c["m"], c["mh"] if hi_on else None, tail=.38)
    return body


@tied_variant
def tied_margin():
    lines = [(330, [(60, 250), (290, 470), (510, 600)]),
             (470, [(60, 210), (250, 420)]),
             (610, [(60, 330), (370, 600)]),
             (750, [(60, 190), (230, 470)])]

    def icon(pal, dark):
        c = colours(pal, dark)
        body = margin_tie_scene(c, lines, 1, (250, 420), 52, 20, (770, 600), 50, 20, rule_x=668,
                                rule_fill=shade(c["page"], -.1 if not dark else .08))
        return icon_svg(c["page"], body)
    ml = [(24, [(4, 26), (30, 56)]), (42, [(4, 22), (26, 46)]), (60, [(4, 34), (38, 58)]), (78, [(4, 18), (22, 48)])]
    mark = margin_tie_scene(dict(m=W_, mh=None, ink=I_), ml, 1, (26, 46), 7, 2.6, (80, 58), 6.4, 2.6, hi_on=False)
    word, info = wordmark("font")
    y = 13.0
    under = running([(4, y), (info["width"] - 6, y)], 3.0, W_, None, 10.5, 6.2, seed=4)
    rope = catmull([(info["width"] - 4, y), (info["width"] + 8, y + 6), (info["width"] + 16, y - 4), (info["width"] + 20, -24)], 12)
    under += thread(rope, 2.8, W_, (0, 0))
    under += knot_end(info["width"] + 30, -40, 9, 3.0, W_, None, tail=.36)
    return dict(slug="tied-margin", name="To the margin", dot="font",
                means="The loose end leads to the margin, where the question was asked, and is tied off there: answered.",
                points="At the margin: the stitch under a word, the thread across, the knot sitting past the margin rule.",
                risk="It is a diagram of the app; at 16 px it is lines and a red squiggle.",
                wordmark=flat(wm_view(0, info["width"] + 48, y0=-84, y1=30), word + under), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


def finger_scene(c, cx, base, s, kw, hi_on=True, dot=True):
    """An inkling is a hint you might lose. A thread tied round a finger is how you keep
    one: here, round the stem of the i, the knot sitting on the front of it, both ends
    hanging."""
    letter_d = L.outline("i" if dot else "ı")["glyphs"][0]["d"]
    gx = cx - L.I_STEM_CX * s
    x0, x1 = gx + 8.9 * s, gx + 20.55 * s
    yb = base - 23 * s
    hi_ = c["mh"] if hi_on else None
    # the pass behind the stem shows only past its edges
    back = thread([(x1 + kw * .3, yb - kw * .5), (cx, yb - kw * 1.3), (x0 - kw * .3, yb - kw * .5)], kw, c["m"], (0, 0))
    body = back + f'<path fill="{c["ink"]}" transform="translate({f(gx)} {f(base)}) scale({s:g})" d="{letter_d}"/>'
    # the front pass, drawn tight round the stem, into the knot
    kx, ky = cx + (x1 - x0) * .12, yb + kw * .55
    body += thread([(x0 - kw * .35, yb - kw * .4), (cx - (x1 - x0) * .2, yb + kw * .45), (kx, ky)], kw, c["m"], (0, 0), hi_)
    body += thread([(x1 + kw * .35, yb - kw * .3), (cx + (x1 - x0) * .32, yb + kw * .3), (kx, ky)], kw, c["m"], (0, 0), hi_)
    # the knot itself, small, and the two ends hanging from it
    body += knot_end(kx, ky + kw * .6, (x1 - x0) * .2, kw * .9, c["m"], hi_, tail=.0, hang=True, turn=0)
    body += thread(catmull([(kx - kw * .2, ky + kw * 1.2), (kx - kw * .9, ky + kw * 3.4), (kx - kw * .6, ky + kw * 5.8)], 10), kw * .9, c["m"], (0, .35), hi_)
    body += thread(catmull([(kx + kw * .3, ky + kw * 1.2), (kx + kw * 1.4, ky + kw * 3.2), (kx + kw * 1.7, ky + kw * 5.2)], 10), kw * .9, c["m"], (0, .35), hi_)
    return body


@tied_variant
def tied_finger():
    def icon(pal, dark):
        c = colours(pal, dark)
        body = finger_scene(c, 500, 858, 9.4, 26)
        return icon_svg(c["page"], body)
    mark = finger_scene(dict(m=W_, mh=None, ink=I_), 46, 92, 1.12, 3.0, hi_on=False)
    word, info = glyphs("nkling", 28.2, 0)
    wm = finger_scene(dict(m=W_, mh=None, ink=I_), 14.725, 0, 1.0, 2.4, hi_on=False) + word
    return dict(slug="tied-finger", name="Round the finger", dot="font",
                means="An inkling is a hint you might lose; a thread tied round your finger is how you keep one. Inkling remembers.",
                points="At Inkling itself: the knot is tied round the stem of the i, the way you tie a reminder round a finger.",
                risk="An i on a tile is the info symbol at small sizes, and a knot on a stem can read as a ribbon.",
                wordmark=flat(wm_view(0, 28.2 + info["width"]), wm), mark=flat((0, 0, 100, 100), mark),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ sampler, in the app's own stitches

SAMPLERS = []


def sampler_variant(fn):
    SAMPLERS.append(fn)
    return fn


# Monoline skeletons for a sampler's lowercase: a stitcher follows a centre line, not an
# outline. Units: x-height 46, ascender 72, baseline 0. Each letter is (width, strokes).
SKELETON = {
    "i": (0, [[(0, -46), (0, 0)]]),
    "n": (28, [[(0, -46), (0, 0)], [(0, -31), (5, -42), (14, -47), (23, -44), (28, -34), (28, 0)]]),
    "k": (26, [[(0, -72), (0, 0)], [(25, -46), (13, -34), (2, -22)], [(9, -28), (18, -14), (27, 0)]]),
    "l": (9, [[(0, -72), (0, -7), (3, -1), (9, 0)]]),
    "g": (26, [[(26, -30), (21, -42), (12, -46), (3, -39), (0, -26), (3, -12), (12, -6), (21, -10), (26, -22)],
               [(26, -46), (26, 8), (22, 18), (13, 22), (3, 19)]]),
}


def skeleton_word(word, ox, oy, s, gap=16):
    """Centre lines for a word, and where the first i's dot goes."""
    x, strokes, dot = ox, [], None
    for ch in word:
        w, st = SKELETON[ch]
        for line in st:
            strokes.append(catmull([(x + px * s, oy + py * s) for px, py in line], 10) if len(line) > 2 else
                           [(x + px * s, oy + py * s) for px, py in line])
        if ch == "i" and dot is None:
            dot = {"cx": x, "cy": oy - 62 * s, "w": 15 * s, "h": 15 * s}
        x += (w + gap) * s
    return strokes, dot, x - gap * s - ox


def backstitch(pts, length, w, fill, hi_=None, gap=.14):
    """Backstitch: short straight stitches end to end along the line, each going back into
    the hole the last came out of, so the line is continuous but you can count it."""
    pts = resample(pts, max(.3, length / 12))
    total = sum(((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** .5 for a, b in zip(pts, pts[1:]))
    n = max(1, round(total / length))
    step = total / n
    out, acc, k = [], 0.0, 1
    marks = [pts[0]]
    for a, b in zip(pts, pts[1:]):
        seg = ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** .5
        while acc + seg >= step * k and k < n:
            t = (step * k - acc) / seg
            marks.append((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t))
            k += 1
        acc += seg
    marks.append(pts[-1])
    for a, b in zip(marks, marks[1:]):
        dx, dy = b[0] - a[0], b[1] - a[1]
        g = gap / 2
        out.append(stitch((a[0] + dx * g, a[1] + dy * g), (b[0] - dx * g, b[1] - dy * g), w, fill, hi_, end=.4, mid=.5))
    return "".join(out)


def path_crosses(pts, every, half, w, fill, hi_=None):
    """Cross-stitch laid along a curve instead of on a grid: one cross every so often,
    so the letter keeps its shape and loses the staircase."""
    pts = resample(pts, max(.3, every / 10))
    total = sum(((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** .5 for a, b in zip(pts, pts[1:]))
    n = max(1, round(total / every))
    out, acc, k = [cross(pts[0][0], pts[0][1], half, w, fill, hi_)], 0.0, 1
    for a, b in zip(pts, pts[1:]):
        seg = ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** .5
        while acc + seg >= total / n * k and k <= n:
            t = (total / n * k - acc) / seg
            out.append(cross(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, half, w, fill, hi_))
            k += 1
        acc += seg
    return "".join(out)


def motif(cx, cy, u, w, fill, hi_=None):
    """The app's border motif: five cross-stitches in a diamond (stitch.rs `border_motif`)."""
    return "".join(cross(cx + dx * u * 1.55, cy + dy * u * 1.55, u * .52, w, fill, hi_) for dx, dy in ((0, -1), (-1, 0), (1, 0), (0, 1), (0, 0)))


def sampler_icon(pal, dark, kind):
    m = pal["madder"]
    s = 7.6
    strokes, dot, width = skeleton_word("ik", 0, 0, s)
    ox, oy = 512 - width / 2, 470 + 36 * s
    strokes, dot, _ = skeleton_word("ik", ox, oy, s)
    body = ""
    sw = 7.2 * s
    for st in strokes:
        if kind == "back":
            body += backstitch(st, 10.5 * s, sw, m, hi(m, .18))
        else:
            body += path_crosses(st, 7.6 * s, 3.4 * s, 2.5 * s, m, hi(m, .18))
    d = dict(dot, cx=dot["cx"], cy=dot["cy"], w=13 * s)
    body += (stitch((d["cx"], d["cy"] - 8.5 * s), (d["cx"], d["cy"] + 7.5 * s), sw, m, hi(m, .18)) if kind == "back"
             else cross(d["cx"], d["cy"], 5.8 * s, 3.4 * s, m, hi(m, .18)))
    # the border along the foot of the sampler: running stitch, then the motif the app adds
    by = 862
    if kind == "back":
        body += running([(176, by), (848, by)], 20, m, hi(m, .18), 72, 40, seed=12)
    else:
        for x in (330, 512, 694):
            body += motif(x, by - 6, 22, 9, m)
    return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))


def sampler_wordmark(kind):
    s = 48.2 / 46
    strokes, dot, width = skeleton_word("inkling", 4, 0, s)
    body = ""
    for st in strokes:
        body += backstitch(st, 9.6, 7.6, W_) if kind == "back" else path_crosses(st, 7.4, 3.3, 2.5, W_)
    body += (stitch((dot["cx"], dot["cy"] - 7.6), (dot["cx"], dot["cy"] + 6.6), 7.6, W_) if kind == "back"
             else cross(dot["cx"], dot["cy"], 5.6, 3.2, W_))
    return flat(wm_view(0, width + 8, y0=-84, y1=30), body), width


def sampler_mark(kind):
    s = 1.12
    strokes, dot, width = skeleton_word("ik", 0, 0, s)
    ox, oy = 50 - width / 2, 48 + 36 * s
    strokes, dot, _ = skeleton_word("ik", ox, oy, s)
    body = ""
    for st in strokes:
        body += backstitch(st, 11 * s, 8 * s, W_) if kind == "back" else path_crosses(st, 7.6 * s, 3.4 * s, 2.6 * s, W_)
    body += (stitch((dot["cx"], dot["cy"] - 8 * s), (dot["cx"], dot["cy"] + 7 * s), 8 * s, W_) if kind == "back"
             else cross(dot["cx"], dot["cy"], 6 * s, 3.4 * s, W_))
    return flat((0, 0, 100, 100), body)


@sampler_variant
def sampler_backstitch():
    wm, _ = sampler_wordmark("back")
    return dict(slug="sampler-backstitch", name="Backstitch", dot="stitch",
                means="The name stitched the way samplers letter a verse: short stitches end to end along each stroke, madder on linen.",
                points="At the hand: every letter is a line you can count stitch by stitch, with one upright stitch for the dot.",
                risk="Monoline letters lose Fraunces, so the name no longer matches the writing; at 19 px it is a thin red word.",
                wordmark=wm, mark=sampler_mark("back"), icon=sampler_icon(LIGHT, False, "back"), icon_dark=sampler_icon(DARK, True, "back"))


@sampler_variant
def sampler_crosses():
    wm, _ = sampler_wordmark("cross")
    return dict(slug="sampler-crosses", name="Crosses on the line", dot="cross",
                means="Still cross-stitch, the sampler's own stitch, but laid along each stroke instead of on a grid: no staircase, no 8-bit.",
                points="At the sampler: crosses on a curve, the border motif the app adds under them, one per tied thread.",
                risk="Crosses along a line blur to a dotted stroke below 32 px, and the cross is also the app's correction mark.",
                wordmark=wm, mark=sampler_mark("cross"), icon=sampler_icon(LIGHT, False, "cross"), icon_dark=sampler_icon(DARK, True, "cross"))


# ================================================================ files

def write_files(variants):
    """round-2/<slug>/{wordmark,mark,icon,icon-dark}.svg, and round-2/dots/<dot>.svg."""
    for d in variants:
        folder = OUT / d["slug"]
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "wordmark.svg").write_text(d["wordmark"])
        (folder / "mark.svg").write_text(d["mark"])
        (folder / "icon.svg").write_text(d["icon"])
        (folder / "icon-dark.svg").write_text(d["icon_dark"])
    dots = OUT / "dots"
    dots.mkdir(parents=True, exist_ok=True)
    for k, _, _, _ in DOTS:
        (dots / f"{k}.svg").write_text(dot_wordmark(k))

# /// script
# dependencies = ["fonttools", "uharfbuzz"]
# ///
"""Logo directions for Inkling, drawn in the Loose Threads language.

Writes ../logo/<nn-name>/{wordmark,mark,icon,icon-dark}.svg and ../logo/assets.json,
which build_logo_page.py inlines into ../logo/index.html.

Every direction is drawn from the same few parts the app itself paints (src/stitch.rs):
a stitch is a short capsule of thread, a knot is a bead with one lighter coil, a loose
thread is a tapered ribbon. Letters are Fraunces Display (opsz 72, SOFT 100, WONK 0),
outlined by outline.py from the font the app bundles, so a wordmark matches the title bar.
Colours are the app's palettes (src/theme.rs): linen and lamplight, madder for Inkling.

usage: uv run logo_directions.py
"""
import colorsys
import json
import math
import random
import subprocess
from pathlib import Path

from logos import f, resample, ribbon, superellipse

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
OUT = HERE.parent / "logo"
DISPLAY = ROOT / "assets" / "fonts" / "FrauncesDisplay-Medium.ttf"

# ---------------------------------------------------------------- palette (src/theme.rs)

LIGHT = dict(paper="#FBF8F2", shelf="#F3EDE2", cloth="#EFE8DB", ink="#26232B", graphite="#57505C",
             faint="#6F6862", rule="#E3DACB", card="#FFFDF8",
             madder="#A63F32", indigo="#2E3F7A", sage="#4F7350", plum="#6B3E62", walnut="#6E4B2E",
             ochre="#8F6214", steel="#8E949C")
DARK = dict(paper="#1F1D22", shelf="#1A181D", cloth="#18161A", ink="#EDE6DA", graphite="#BDB4A8",
            faint="#A39A8F", rule="#332F36", card="#26232A",
            madder="#D9705E", indigo="#8C9BE0", sage="#8DB88E", plum="#C08AB4", walnut="#C49A73",
            ochre="#D9A548", steel="#9AA1AA")


def shade(hex_, dl):
    """The same hue, lighter or darker: the app's `lighter()` (src/stitch.rs) in HLS."""
    r, g, b = (int(hex_[i:i + 2], 16) / 255 for i in (1, 3, 5))
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    l = max(0.0, min(0.95, l + dl))
    r, g, b = colorsys.hls_to_rgb(h, l, s)
    return "#%02X%02X%02X" % (round(r * 255), round(g * 255), round(b * 255))


# ---------------------------------------------------------------- letters (outline.py)

_outlines = {}


def outline(text, size=100, font=DISPLAY):
    """Glyph outlines and the dot of each i, from outline.py (cached per run)."""
    key = (str(font), text, size)
    if key not in _outlines:
        out = subprocess.check_output(["uv", "run", "--quiet", str(HERE / "outline.py"), str(font), text, str(size)])
        _outlines[key] = json.loads(out)
    return _outlines[key]


def glyphs(text, x0=0.0, y0=0.0, scale=1.0, cls="ink", dotless_first_i=False, track=0.0):
    """SVG paths for `text` set at 100 units and scaled; the first i can lose its dot.
    Returns (svg, info) where info has each glyph's box and the first i's dot and stem."""
    data = outline(text)
    parts, info = [], {"glyphs": [], "meta": data["meta"]}
    first = True
    for n, g in enumerate(data["glyphs"]):
        gx = x0 + (g["x"] + n * track) * scale
        d = g["d"]
        if g["glyph"] == "i" and first and dotless_first_i:
            first = False
            d = outline("ı")["glyphs"][0]["d"]
            info["dot"] = {"cx": gx + (g["dot"]["cx"] - g["x"]) * scale, "cy": y0 + g["dot"]["cy"] * scale,
                           "w": g["dot"]["w"] * scale, "h": g["dot"]["h"] * scale}
            # the stem proper (without the flag and the foot), measured from the font
            info["stem"] = {"x0": gx + 8.9 * scale, "x1": gx + 20.55 * scale,
                            "top": y0 - 43.15 * scale, "base": y0}
            dx = gx
        else:
            dx = gx - g["x"] * scale
        parts.append(f'<path class="{cls}" transform="translate({f(dx)} {f(y0)}) scale({scale:g})" d="{d}"/>')
        b = g.get("bounds")
        if b:
            info["glyphs"].append({"name": g["glyph"], "x0": gx + (b[0] - g["x"]) * scale, "x1": gx + (b[2] - g["x"]) * scale,
                                   "top": y0 + b[1] * scale, "bottom": y0 + b[3] * scale, "adv": g["adv"] * scale})
    info["width"] = data["meta"]["width"] * scale + track * scale * (len(data["glyphs"]) - 1)
    info["x_height"] = data["meta"]["xHeight"] * scale
    return "\n".join(parts), info


# ---------------------------------------------------------------- thread

def P(pts):
    return "M" + " L".join(f"{f(x)} {f(y)}" for x, y in pts) + " Z"


def stitch(a, b, w, fill, hi=None, end=0.36, mid=0.5):
    """One stitch: thread on the surface between two holes. A grain of rice with round
    ends, because thread has round ends and is fullest in the middle (stitch.rs draws a
    capsule; at icon size the swell is what makes it read as thread, not a dash)."""
    (x0, y0), (x1, y1) = a, b
    L = math.hypot(x1 - x0, y1 - y0) or 1
    ux, uy = (x1 - x0) / L, (y1 - y0) / L
    nx, ny = -uy, ux
    e, c = w * end, w * (2 * mid - end)

    def pt(s, t):
        return x0 + ux * s + nx * t, y0 + uy * s + ny * t

    p0, p1, p2, p3 = pt(0, -e), pt(L, -e), pt(L, e), pt(0, e)
    q0, q1 = pt(L / 2, -c), pt(L / 2, c)
    d = (f"M{f(p0[0])} {f(p0[1])} Q{f(q0[0])} {f(q0[1])} {f(p1[0])} {f(p1[1])} "
         f"A{f(e)} {f(e)} 0 0 1 {f(p2[0])} {f(p2[1])} Q{f(q1[0])} {f(q1[1])} {f(p3[0])} {f(p3[1])} "
         f"A{f(e)} {f(e)} 0 0 1 {f(p0[0])} {f(p0[1])}Z")
    out = f'<path fill="{fill}" d="{d}"/>'
    if hi and L > w * 1.6:
        out += twist(a, b, w, hi)
    return out


def twist(a, b, w, hi, every=0.95, op=0.5):
    """The light on the twist of a thread: short slanted strokes across it."""
    (x0, y0), (x1, y1) = a, b
    L = math.hypot(x1 - x0, y1 - y0) or 1
    ux, uy = (x1 - x0) / L, (y1 - y0) / L
    nx, ny = -uy, ux
    n = max(1, int(L / (w * every)))
    segs = []
    for k in range(1, n):
        s = L * k / n
        cx, cy = x0 + ux * s, y0 + uy * s
        # slant: the ply of a twisted thread runs at about 30 degrees to its length
        ax, ay = cx + nx * w * .26 - ux * w * .16, cy + ny * w * .26 - uy * w * .16
        bx, by = cx - nx * w * .26 + ux * w * .16, cy - ny * w * .26 + uy * w * .16
        segs.append(f"M{f(ax)} {f(ay)}L{f(bx)} {f(by)}")
    if not segs:
        return ""
    return (f'<path stroke="{hi}" stroke-width="{f(w * .14)}" stroke-linecap="round" opacity="{op}" '
            f'fill="none" d="{"".join(segs)}"/>')


def thread(pts, w, fill, taper=(0.04, 0.12), hi=None, step=None):
    """A loose thread along a centre line: a ribbon that thins where it ends."""
    step = step or max(0.5, w / 6)
    pts = resample(pts, step)
    out = f'<path fill="{fill}" d="{ribbon(pts, w, taper, 0.0, 1)}"/>'
    if hi:
        out += thread_twist(pts, w, hi)
    return out


def thread_twist(pts, w, hi, op=0.45):
    segs = []
    acc, every = 0.0, w * 1.05
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        L = math.hypot(x1 - x0, y1 - y0)
        acc += L
        if acc >= every and L > 0:
            acc = 0.0
            ux, uy = (x1 - x0) / L, (y1 - y0) / L
            nx, ny = -uy, ux
            ax, ay = x1 + nx * w * .24 - ux * w * .15, y1 + ny * w * .24 - uy * w * .15
            bx, by = x1 - nx * w * .24 + ux * w * .15, y1 - ny * w * .24 + uy * w * .15
            segs.append(f"M{f(ax)} {f(ay)}L{f(bx)} {f(by)}")
    if not segs:
        return ""
    return (f'<path stroke="{hi}" stroke-width="{f(w * .13)}" stroke-linecap="round" opacity="{op}" fill="none" '
            f'd="{"".join(segs)}"/>')


def running(pts, w, fill, hi=None, stitch_len=None, gap=None, seed=3, skip=0):
    """Running stitch along a centre line: thread on top, then under the cloth.
    Lengths follow stitch.rs (6.6 on, 3.7 off at 1.8 thick), scaled by `w`."""
    rnd = random.Random(seed)
    stitch_len = stitch_len or w * 3.7
    gap = gap or w * 2.05
    pts = resample(pts, max(0.25, w / 8))
    out, on, run, start = [], True, 0.0, pts[0]
    target = stitch_len * (1 + (rnd.random() - .5) * .2)
    count = 0
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        run += math.hypot(x1 - x0, y1 - y0)
        if run >= target:
            if on:
                if count >= skip:
                    j = (rnd.random() - .5) * w * .18
                    out.append(stitch((start[0], start[1] + j), (x1, y1 + j), w, fill, hi))
                count += 1
            on = not on
            run = 0.0
            start = (x1, y1)
            target = (stitch_len if on else gap) * (1 + (rnd.random() - .5) * .2)
    return "".join(out)


def cross(cx, cy, half, w, fill, hi=None):
    """A cross-stitch: the bottom leg first, the top leg over it. Every top leg on a
    piece lies the same way; that is how good cross-stitch is judged."""
    a = stitch((cx - half, cy + half), (cx + half, cy - half), w, fill, None)
    b = stitch((cx - half, cy - half), (cx + half, cy + half), w, fill, hi)
    return a + b


def knot(cx, cy, r, fill, hi, lo=None, turns=1.3, coil_w=0.22):
    """A French knot as the app draws it (stitch.rs `french_knot`): a bead of thread
    with one coil of lighter thread from the middle out. `lo` adds the shadow side,
    for sizes where the bead needs body."""
    parts = [f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}" fill="{fill}"/>']
    if lo:
        parts.append(f'<path fill="{lo}" opacity=".55" d="M{f(cx + r)} {f(cy)} A{f(r)} {f(r)} 0 0 1 {f(cx - r * .5)} {f(cy + r * .866)} '
                     f'A{f(r * .92)} {f(r * .92)} 0 0 0 {f(cx + r)} {f(cy)}Z"/>')
    pts = []
    n = 60
    for i in range(n + 1):
        t = i / n
        a = -2.0 + t * turns * 2 * math.pi
        rr = r * (0.16 + 0.6 * t)
        pts.append((cx + rr * math.cos(a), cy + rr * math.sin(a)))
    parts.append(f'<path fill="{hi}" d="{ribbon(pts, r * coil_w, (0.2, 0.25), 0.0, 2)}"/>')
    return "".join(parts)


# ---------------------------------------------------------------- the tile

BODY = superellipse(512, 512, 412, 5.0, 192)


def weave(pid, color, op, step=7.0, w=1.3):
    """Linen, seen close: a plain weave. Only ever on cloth, never under words."""
    s = step
    return (f'<pattern id="{pid}" width="{f(s * 2)}" height="{f(s * 2)}" patternUnits="userSpaceOnUse">'
            f'<path d="M0 {f(s * .5)}H{f(s * 2)}M0 {f(s * 1.5)}H{f(s * 2)}" stroke="{color}" stroke-width="{f(w)}" opacity="{op}"/>'
            f'<path d="M{f(s * .5)} 0V{f(s * 2)}M{f(s * 1.5)} 0V{f(s * 2)}" stroke="{color}" stroke-width="{f(w * .8)}" opacity="{f(op * .7)}"/>'
            f'</pattern>')


def icon_svg(ground, body, defs="", edge=None, cloth=None):
    """A macOS icon: one full rounded square (anything else lands in a grey tile on
    macOS 26). `cloth` lays a weave over the ground: (colour, opacity)."""
    clip = '<clipPath id="tile"><path d="' + BODY + '"/></clipPath>'
    weave_defs, weave_layer = "", ""
    if cloth:
        weave_defs = weave("wv", cloth[0], cloth[1])
        weave_layer = f'<path d="{BODY}" fill="url(#wv)"/>'
    edge = edge or ("#000", 0.1)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">'
            f'<defs>{clip}{weave_defs}{defs}</defs>'
            f'<path d="{BODY}" fill="{ground}"/>{weave_layer}'
            f'<g clip-path="url(#tile)">{body}</g>'
            f'<path d="{BODY}" fill="none" stroke="{edge[0]}" stroke-opacity="{edge[1]}" stroke-width="2"/></svg>')


# ---------------------------------------------------------------- flat marks (wordmark, mark)

def classes(pal):
    """Every flat mark colours itself by class, so one file follows light and dark."""
    c = {
        "ink": pal["ink"], "mad": pal["madder"], "mad-hi": shade(pal["madder"], .16), "mad-lo": shade(pal["madder"], -.12),
        "wal": pal["walnut"], "wal-hi": shade(pal["walnut"], .14), "steel": pal["steel"], "steel-hi": shade(pal["steel"], .2),
        "card": pal["card"], "paper": pal["paper"], "rule": pal["rule"], "faint": pal["faint"], "cloth": pal["cloth"],
        "ochre": pal["ochre"], "sage": pal["sage"], "indigo": pal["indigo"], "graphite": pal["graphite"],
    }
    return c


def style_for(pal):
    return " ".join(f".{k}{{fill:{v}}}" for k, v in classes(pal).items()) + \
        " .s-mad{stroke:" + pal["madder"] + "} .s-ink{stroke:" + pal["ink"] + "} .s-faint{stroke:" + pal["faint"] + "}" + \
        " .s-hi{stroke:" + shade(pal["madder"], .2) + "} .s-lo{stroke:" + shade(pal["madder"], -.12) + "}"


def flat_svg(view, body, label="Inkling"):
    """A wordmark or mark that follows the reader's light or dark setting."""
    x, y, w, h = view
    st = style_for(LIGHT) + " @media (prefers-color-scheme:dark){" + style_for(DARK) + "}"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{f(x)} {f(y)} {f(w)} {f(h)}" width="{f(w)}" height="{f(h)}" '
            f'role="img" aria-label="{label}"><style>{st}</style>{body}</svg>')


def fixed(svg, pal):
    """The same flat mark with its colours fixed to one palette (for the review page)."""
    import re
    return re.sub(r"<style>.*?</style>", "<style>" + style_for(pal) + "</style>", svg, flags=re.S)


def k_cls(cx, cy, r, coil=True, lo=False):
    """The app's knot as a class-coloured flat mark."""
    out = f'<circle class="mad" cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}"/>'
    if lo:
        out += (f'<path class="mad-lo" opacity=".5" d="M{f(cx + r)} {f(cy)} A{f(r)} {f(r)} 0 0 1 {f(cx - r * .5)} {f(cy + r * .866)} '
                f'A{f(r * .92)} {f(r * .92)} 0 0 0 {f(cx + r)} {f(cy)}Z"/>')
    if coil:
        pts = []
        for i in range(49):
            t = i / 48
            a = -2.0 + t * 1.3 * 2 * math.pi
            rr = r * (0.16 + 0.6 * t)
            pts.append((cx + rr * math.cos(a), cy + rr * math.sin(a)))
        out += f'<path class="mad-hi" d="{ribbon(pts, r * .22, (0.2, 0.25), 0.0, 2)}"/>'
    return out


def stitch_cls(a, b, w, cls="mad"):
    return stitch(a, b, w, "X").replace('fill="X"', f'class="{cls}"')


DIRECTIONS = []


def direction(fn):
    DIRECTIONS.append(fn)
    return fn


# ================================================================ shared geometry

def rot(pts, deg, cx, cy):
    a = math.radians(deg)
    c, s = math.cos(a), math.sin(a)
    return [(cx + (x - cx) * c - (y - cy) * s, cy + (x - cx) * s + (y - cy) * c) for x, y in pts]


def catmull(points, n=14):
    """A smooth line through every point (centripetal enough for hand-drawn curves)."""
    if len(points) < 3:
        return list(points)
    pts = [points[0]] + list(points) + [points[-1]]
    out = []
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        for k in range(n):
            t = k / n
            t2, t3 = t * t, t * t * t
            out.append(tuple(0.5 * ((2 * p1[j]) + (-p0[j] + p2[j]) * t + (2 * p0[j] - 5 * p1[j] + 4 * p2[j] - p3[j]) * t2
                                    + (-p0[j] + 3 * p1[j] - 3 * p2[j] + p3[j]) * t3) for j in (0, 1)))
    out.append(points[-1])
    return out


def sel_pts(cx, cy, rx, ry, n=5.0, steps=240):
    """Points round a superellipse (the tile's own shape, for borders inset in it)."""
    pts = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        c, s = math.cos(t), math.sin(t)
        pts.append((cx + rx * math.copysign(abs(c) ** (2 / n), c), cy + ry * math.copysign(abs(s) ** (2 / n), s)))
    return pts + [pts[0]]


def rounded_poly(pts, r):
    """A closed polygon with its corners rounded (for tags and labels)."""
    out = []
    n = len(pts)
    for i in range(n):
        p0, p1, p2 = pts[i - 1], pts[i], pts[(i + 1) % n]
        a = math.atan2(p0[1] - p1[1], p0[0] - p1[0])
        b = math.atan2(p2[1] - p1[1], p2[0] - p1[0])
        da, db = math.hypot(p0[0] - p1[0], p0[1] - p1[1]), math.hypot(p2[0] - p1[0], p2[1] - p1[1])
        rr = min(r, da / 2, db / 2)
        s = (p1[0] + math.cos(a) * rr, p1[1] + math.sin(a) * rr)
        e = (p1[0] + math.cos(b) * rr, p1[1] + math.sin(b) * rr)
        out.append((s, p1, e))
    d = f"M{f(out[0][2][0])} {f(out[0][2][1])}"
    for s, c, e in out[1:] + out[:1]:
        d += f" L{f(s[0])} {f(s[1])} Q{f(c[0])} {f(c[1])} {f(e[0])} {f(e[1])}"
    return d + "Z"


def poly_pts(pts, step=2.0):
    """Points along a closed polygon, for stitching a border inside a shape."""
    out = []
    for a, b in zip(pts, pts[1:] + pts[:1]):
        L = math.hypot(b[0] - a[0], b[1] - a[1])
        n = max(1, int(L / step))
        out += [(a[0] + (b[0] - a[0]) * k / n, a[1] + (b[1] - a[1]) * k / n) for k in range(n)]
    return out + [out[0]]


def wm_view(x0, x1, y0=-84.0, y1=30.0, pad=8.0):
    return (x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad)


def cloth_for(pal, dark):
    return (shade(pal["cloth"], .12 if dark else -.2), .3 if dark else .32)


def hi(c, d=.16):
    return shade(c, d)


def lo(c, d=-.13):
    return shade(c, d)


# The dotless i of Fraunces Display at 100 units: its stem's centre, and its dot
# (centre x, centre y, width), measured by outline.py.
I_STEM_CX = 14.725
I_DOT = (13.78, -61.25, 15.65)


# ================================================================ 01 Knot

@direction
def d01_knot():
    word, info = glyphs("inkling", dotless_first_i=True)
    d = info["dot"]
    wm = word + k_cls(d["cx"], d["cy"] + 0.4, d["w"] * 0.58)
    view = wm_view(0, info["width"])
    mark = flat_svg((0, 0, 100, 100), k_cls(50, 50, 38, lo=True), "Inkling")

    def icon(pal, dark):
        # the knot alone, as the app draws it, on the linen it was tied into
        m = pal["madder"]
        body = knot(512, 512, 178, m, hi(m, .08), lo(m, -.15), coil_w=.13)
        return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))

    return dict(slug="01-knot", name="Knot", wordmark=flat_svg(view, wm), mark=mark,
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 02 Tied (overhand knot)

def segs_cross(p, q, r, s):
    """Where segment pq meets segment rs, as (t, u) along each, or None."""
    d = (q[0] - p[0]) * (s[1] - r[1]) - (q[1] - p[1]) * (s[0] - r[0])
    if abs(d) < 1e-9:
        return None
    t = ((r[0] - p[0]) * (s[1] - r[1]) - (r[1] - p[1]) * (s[0] - r[0])) / d
    u = ((r[0] - p[0]) * (q[1] - p[1]) - (r[1] - p[1]) * (q[0] - p[0])) / d
    return (t, u) if 0 <= t <= 1 and 0 <= u <= 1 else None


OVERHAND = [(-3.4, 1.0), (-2.0, .7), (-.8, .3), (.3, -.5), (.6, -1.5), (0, -2.1), (-.8, -1.8), (-.9, -.9), (-.2, .1),
            (.9, .7), (1.5, .3), (1.1, -.5), (.2, -.6), (-.6, .2), (-.3, 1.0), (.8, 1.4), (2.2, 1.2), (3.4, .9)]


def overhand(cx, cy, scale, tails=1.0):
    """An overhand knot, pulled almost tight: the thread comes in from the left, loops,
    passes through its own loop and goes out to the right."""
    pts = [(x if abs(x) < 1.6 else math.copysign(1.6 + (abs(x) - 1.6) * tails, x), y) for x, y in OVERHAND]
    return catmull([(cx + x * scale, cy + y * scale) for x, y in pts], 14)


def tied(pts, w, fill, hi_=None, gap=2.4):
    """Draw a knotted thread so that it reads as a knot: walking along it, the crossings
    go over, under, over...; where it goes under, leave the thread out for a moment."""
    pts = resample(pts, max(.4, w / 5))
    cum = [0.0]
    for p, q in zip(pts, pts[1:]):
        cum.append(cum[-1] + math.hypot(q[0] - p[0], q[1] - p[1]))
    visits = []
    for i in range(len(pts) - 1):
        for j in range(i + 2, len(pts) - 1):
            hit = segs_cross(pts[i], pts[i + 1], pts[j], pts[j + 1])
            if hit:
                si = cum[i] + hit[0] * (cum[i + 1] - cum[i])
                sj = cum[j] + hit[1] * (cum[j + 1] - cum[j])
                visits += [si, sj]
    visits.sort()
    unders = [v for k, v in enumerate(visits) if k % 2 == 1]
    pieces, cur = [], []
    for p, c in zip(pts, cum):
        if any(abs(c - u) < w * gap / 2 for u in unders):
            if len(cur) > 1:
                pieces.append(cur)
            cur = []
        else:
            cur.append(p)
    if len(cur) > 1:
        pieces.append(cur)
    out = ""
    for k, pc in enumerate(pieces):
        first, last = k == 0, k == len(pieces) - 1
        out += thread(pc, w, fill, (0.05 if first else 0.0, 0.05 if last else 0.0), hi_)
    return out


@direction
def d02_tied():
    word, info = glyphs("inkling", 64, 0)
    wm = tied(overhand(26, -36, 12.5, tails=.42), 3.4, "X2", None).replace('fill="X2"', 'class="mad"') + word
    view = wm_view(-6, 64 + info["width"], y0=-84, y1=30)
    mk = tied(overhand(50, 56, 17, tails=.55), 5.4, "X2", None).replace('fill="X2"', 'class="mad"')

    def icon(pal, dark):
        m = pal["madder"]
        body = tied(overhand(500, 552, 124, tails=1.0), 40, m, hi(m, .2))
        return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))

    return dict(slug="02-tied", name="Tied off", wordmark=flat_svg(view, wm), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 03 Initial (satin stitch)

def satin(clip_id, d_path, transform, base, sheen, groove, spacing, angle=-24, extent=(0, -50, 30, 2)):
    """Satin stitch: straight stitches laid side by side across a shape, so close they
    read as one glossy band. The letter in thread colour, then the sheen and the groove
    of each stitch, clipped to it. Everything sits in the letter's own units."""
    x0, y0, x1, y1 = extent
    k = math.tan(math.radians(angle))
    lines_s, lines_g = [], []
    span = (x1 - x0) * abs(k) + spacing
    y = y0 - span
    while y < y1 + span:
        xa, xb = x0 - 2, x1 + 2
        ya, yb = y + k * (xa - x0), y + k * (xb - x0)
        lines_g.append(f"M{f(xa)} {f(ya)}L{f(xb)} {f(yb)}")
        lines_s.append(f"M{f(xa)} {f(ya - spacing * .4)}L{f(xb)} {f(yb - spacing * .4)}")
        y += spacing
    return (f'<g transform="{transform}"><clipPath id="{clip_id}"><path d="{d_path}"/></clipPath>'
            f'<path fill="{base}" d="{d_path}"/>'
            f'<g clip-path="url(#{clip_id})">'
            f'<path stroke="{sheen}" stroke-width="{f(spacing * .3)}" stroke-linecap="round" opacity=".42" fill="none" d="{"".join(lines_s)}"/>'
            f'<path stroke="{groove}" stroke-width="{f(spacing * .12)}" opacity=".55" fill="none" d="{"".join(lines_g)}"/></g></g>')


@direction
def d03_initial():
    i_d = outline("ı")["glyphs"][0]["d"]
    word, info = glyphs("nkling", 28.2, 0)
    sat = satin("sw", i_d, "translate(0 0)", "X1", "X2", "X3", 2.1, extent=(0, -50, 30, 2))
    sat = sat.replace('fill="X1"', 'class="mad"').replace('stroke="X2"', 'class="s-hi"').replace('stroke="X3"', 'class="s-lo"')
    dot = k_cls(I_DOT[0], I_DOT[1] + .4, I_DOT[2] * .58)
    wm = sat + word + dot
    view = wm_view(0, 28.2 + info["width"])

    mk = satin("sm", i_d, "translate(32 95) scale(1.3)", "X1", "X2", "X3", 2.4)
    mk = mk.replace('fill="X1"', 'class="mad"').replace('stroke="X2"', 'class="s-hi"').replace('stroke="X3"', 'class="s-lo"')
    mk += k_cls(32 + I_DOT[0] * 1.3, 95 + I_DOT[1] * 1.3 + .5, I_DOT[2] * 1.3 * .58)

    def icon(pal, dark):
        m = pal["madder"]
        s = 9.4
        base = 822
        gx = 512 - I_STEM_CX * s
        tr = f"translate({f(gx)} {base}) scale({s})"
        body = ""
        # the hemstitched hem: a drawn-thread band with its threads gathered into bundles
        band = sel_pts(512, 512, 352, 352, 5.0, 300)
        bundles = []
        L = 0.0
        for (xa, ya), (xb, yb) in zip(band, band[1:]):
            L += math.hypot(xb - xa, yb - ya)
            if L >= 13:
                L = 0.0
                ux, uy = (xb - xa), (yb - ya)
                ln = math.hypot(ux, uy) or 1
                nx, ny = -uy / ln, ux / ln
                bundles.append(stitch((xb - nx * 9, yb - ny * 9), (xb + nx * 9, yb + ny * 9), 5.2, pal["rule"] if not dark else shade(pal["rule"], .08)))
        body += "".join(bundles)
        body += f'<path fill="none" stroke="{shade(pal["paper"], -.1 if not dark else .1)}" stroke-width="2" d="{P(sel_pts(512, 512, 336, 336))}"/>'
        body += f'<path fill="none" stroke="{shade(pal["paper"], -.1 if not dark else .1)}" stroke-width="2" d="{P(sel_pts(512, 512, 368, 368))}"/>'
        # raised: padded satin throws a small shadow on the linen
        body += f'<path fill="{shade(pal["paper"], -.12 if not dark else -.06)}" opacity=".7" transform="translate({f(gx + 7)} {base + 10}) scale({s})" d="{i_d}"/>'
        body += satin("st", i_d, tr, m, hi(m, .2), lo(m, -.12), 1.75, extent=(0, -50, 30, 2))
        kx, ky = gx + I_DOT[0] * s, base + I_DOT[1] * s
        body += f'<circle cx="{f(kx + 6)}" cy="{f(ky + 9)}" r="{f(I_DOT[2] * s * .6)}" fill="{shade(pal["paper"], -.12 if not dark else -.06)}" opacity=".7"/>'
        body += knot(kx, ky, I_DOT[2] * s * .6, m, hi(m, .12), lo(m, -.15), coil_w=.17)
        return icon_svg(pal["paper"], body)

    return dict(slug="03-initial", name="Initial", wordmark=flat_svg(view, wm), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 04 Sampler (counted cross-stitch)

ALPHABET = {
    # x-height rows 0..5; ascender rows -3..-1; descender rows 6..7. "X" is one cross.
    "i": {0: "XX.", 1: ".X.", 2: ".X.", 3: ".X.", 4: ".X.", 5: "XXX"},
    "n": {0: "XX.XX.", 1: ".XX..X", 2: ".X...X", 3: ".X...X", 4: ".X...X", 5: "XXX.XX"},
    "k": {-3: "XX....", -2: ".X....", -1: ".X....", 0: ".X..XX", 1: ".X.X..", 2: ".XX...", 3: ".X.X..", 4: ".X..X.", 5: "XXX.XX"},
    "l": {-3: "XX.", -2: ".X.", -1: ".X.", 0: ".X.", 1: ".X.", 2: ".X.", 3: ".X.", 4: ".X.", 5: "XXX"},
    "g": {0: ".XXXXX", 1: "X...X.", 2: "X...X.", 3: "X...X.", 4: ".XXXX.", 5: "....X.", 6: "X...X.", 7: ".XXX.."},
}


def sampler_word(word, cell, ox, oy, emit, knot_first_i=True):
    """Lay out a word in the counted alphabet. emit(kind, cx, cy) draws each cell."""
    x = ox
    first = True
    for ch in word:
        rows = ALPHABET[ch]
        wcols = len(next(iter(rows.values())))
        for r, line in rows.items():
            for c, v in enumerate(line):
                if v == "X":
                    emit("x", x + (c + .5) * cell, oy + (r + .5) * cell)
        if ch == "i":
            # the dot: two rows above the stem; the first is a French knot
            emit("knot" if (first and knot_first_i) else "x", x + 1.5 * cell, oy + (-2 + .5) * cell)
            first = False
        x += (wcols + 1) * cell
    return x - cell - ox


def cross_def(pid, half, w, fill=None, hi_=None, cls=None):
    """One cross-stitch as a reusable symbol; every cell of a sampler is the same stitch."""
    body = cross(0, 0, half, w, "X", hi_)
    body = body.replace('fill="X"', f'class="{cls}"' if cls else f'fill="{fill}"')
    return f'<g id="{pid}">{body}</g>'


def use(pid, x, y):
    return f'<use href="#{pid}" x="{f(x)}" y="{f(y)}"/>'


@direction
def d04_sampler():
    cell = 8.0
    parts = [f'<defs>{cross_def("x8", cell * .44, cell * .4, cls="mad")}</defs>']

    def emit_wm(kind, cx, cy):
        parts.append(use("x8", cx, cy) if kind == "x" else k_cls(cx, cy, cell * .62))
    width = sampler_word("inkling", cell, 0, -6 * cell, emit_wm)
    view = wm_view(0, width, y0=-9.6 * cell, y1=2.2 * cell)
    mc = 9.0
    mparts = [f'<defs>{cross_def("x9", mc * .44, mc * .38, cls="mad")}</defs>']

    def emit_mk(kind, cx, cy):
        mparts.append(use("x9", cx, cy) if kind == "x" else k_cls(cx, cy, mc * .62))
    mw = sampler_word("ik", mc, 0, 0, emit_mk)
    mark = flat_svg((-mc * .6, -3.6 * mc, mw + mc * 1.2, 10.2 * mc), "".join(mparts))

    def icon(pal, dark):
        m = pal["madder"]
        c = 52.0
        ox = 512 - 10 * c / 2
        oy = 512 - 1.0 * c
        hole = shade(pal["cloth"], -.16 if not dark else .1)
        # Aida: the holes of the counted cloth, one at every cell corner, on the stitch grid
        defs = (f'<pattern id="aida" width="{f(c)}" height="{f(c)}" patternUnits="userSpaceOnUse" x="{f(ox % c)}" y="{f(oy % c)}">'
                + "".join(f'<circle cx="{x}" cy="{y}" r="{f(c * .09)}" fill="{hole}"/>' for x in (0, f(c)) for y in (0, f(c)))
                + '</pattern>' + cross_def("xs", c * .4, c * .3, m, hi(m, .2)))
        body = ['<rect x="0" y="0" width="1024" height="1024" fill="url(#aida)"/>']

        def emit(kind, cx, cy):
            body.append(use("xs", cx, cy) if kind == "x" else knot(cx, cy, c * .64, m, hi(m, .12), lo(m, -.15), coil_w=.18))
        sampler_word("ik", c, ox, oy, emit)
        # a band of the border Inkling stitches as threads are tied off, one colour per mind
        by = oy + 8.2 * c
        colors = [pal["ochre"], pal["sage"], m, pal["indigo"], pal["plum"]]
        for k, bx in enumerate([512 - 3.3 * c, 512 - 1.65 * c, 512, 512 + 1.65 * c, 512 + 3.3 * c]):
            for dx, dy in ((0, -1), (-1, 0), (1, 0), (0, 1), (0, 0)):
                body.append(cross(bx + dx * c * .34, by + dy * c * .34, c * .14, c * .1, colors[k]))
        return icon_svg(pal["cloth"], "".join(body), defs=defs)

    return dict(slug="04-sampler", name="Sampler", wordmark=flat_svg(view, "".join(parts)), mark=mark,
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 05 Needle

def needle_shape(x, y_eye_top, y_tip, w, eye_w, eye_h, eye_cy):
    """An upright needle: rounded head, the eye cut through it, a shaft that tapers to
    the point. One path, even-odd, so the eye is a real hole."""
    hw = w / 2
    taper_from = y_tip - (y_tip - y_eye_top) * 0.34
    d = (f"M{f(x - hw)} {f(y_eye_top + hw)} A{f(hw)} {f(hw)} 0 0 1 {f(x + hw)} {f(y_eye_top + hw)} "
         f"L{f(x + hw * .92)} {f(taper_from)} Q{f(x + hw * .6)} {f(y_tip - (y_tip - taper_from) * .2)} {f(x)} {f(y_tip)} "
         f"Q{f(x - hw * .6)} {f(y_tip - (y_tip - taper_from) * .2)} {f(x - hw * .92)} {f(taper_from)} Z ")
    ex = eye_w / 2
    d += (f"M{f(x - ex)} {f(eye_cy - eye_h / 2 + ex)} A{f(ex)} {f(ex)} 0 0 1 {f(x + ex)} {f(eye_cy - eye_h / 2 + ex)} "
          f"L{f(x + ex)} {f(eye_cy + eye_h / 2 - ex)} A{f(ex)} {f(ex)} 0 0 1 {f(x - ex)} {f(eye_cy + eye_h / 2 - ex)} Z")
    return d


def threaded_needle(x, top, tip, w, steel, steel_hi, steel_lo, mad, mad_hi, tw, scale, cls=False, twist_on=True):
    eye_h, eye_w = w * 2.3, w * .42
    eye_cy = top + w * 1.75
    nd = needle_shape(x, top, tip, w, eye_w, eye_h, eye_cy)
    # light from the upper left: the left half of the shaft is the bright side
    shine = (f'<path fill="{steel_hi}" opacity=".9" d="M{f(x - w * .3)} {f(top + w * .55)} L{f(x - w * .12)} {f(top + w * .55)} '
             f'L{f(x - w * .1)} {f(tip - (tip - top) * .3)} L{f(x - w * .24)} {f(tip - (tip - top) * .3)} Z"/>')
    s = scale
    # the thread: in through the back of the eye, out the front, both ends hanging left
    back = catmull([(x + 3 * s, eye_cy), (x + 14 * s, eye_cy + 10 * s), (x + 12 * s, eye_cy + 30 * s), (x - 2 * s, eye_cy + 52 * s),
                    (x - 26 * s, eye_cy + 78 * s), (x - 44 * s, eye_cy + 118 * s), (x - 40 * s, eye_cy + 150 * s)], 14)
    front = catmull([(x + 3 * s, eye_cy - 1 * s), (x - 12 * s, eye_cy + 6 * s), (x - 30 * s, eye_cy + 26 * s), (x - 40 * s, eye_cy + 60 * s),
                     (x - 36 * s, eye_cy + 96 * s), (x - 22 * s, eye_cy + 124 * s)], 14)
    t_hi = mad_hi if twist_on else None
    body = thread(back, tw, mad, (0.0, 0.1), t_hi)
    body += f'<path fill="{steel}" fill-rule="evenodd" d="{nd}"/>' + shine
    body += f'<path fill="{steel_lo}" opacity=".35" d="M{f(x + w * .18)} {f(top + w)} L{f(x + w * .46)} {f(top + w)} L{f(x + w * .44)} {f(tip - (tip - top) * .32)} L{f(x + w * .14)} {f(tip - (tip - top) * .32)} Z"/>'
    body += thread(front, tw, mad, (0.0, 0.1), t_hi)
    return body


@direction
def d05_needle():
    word, info = glyphs("nkling", 30, 0)
    # the first i is a needle: its eye is where the dot would be
    nb = threaded_needle(14.7, -69.5, 6, 9.2, "X1", "X4", "X5", "X2", "X3", 2.4, .2, twist_on=False)
    nb = (nb.replace('fill="X1"', 'class="ink"').replace('fill="X4"', 'class="faint"').replace('fill="X5"', 'class="ink"')
          .replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"'))
    view = wm_view(-10, 30 + info["width"], y0=-84, y1=30)
    mk = threaded_needle(58, 6, 96, 9, "X1", "X4", "X5", "X2", "X3", 3, .27, twist_on=False)
    mk = (mk.replace('fill="X1"', 'class="ink"').replace('fill="X4"', 'class="faint"').replace('fill="X5"', 'class="ink"')
          .replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"'))

    def icon(pal, dark):
        m = pal["madder"]
        steel = "#6E737B" if not dark else "#AEB4BC"
        body = threaded_needle(584, 196, 852, 66, steel, "#E9ECF0" if not dark else "#F1F3F6", "#2E3138", m, hi(m, .2), 26, 2.7)
        return icon_svg(pal["cloth"], f'<g transform="rotate(10 584 524)">{body}</g>', cloth=cloth_for(pal, dark))

    return dict(slug="05-needle", name="Needle", wordmark=flat_svg(view, nb + word),
                mark=flat_svg((0, 0, 100, 100), mk), icon=icon(LIGHT, False), icon_dark=icon(DARK, True))

# ================================================================ 06 Spool

def spool(cx, top, bottom, w, wood, wood_hi, wood_lo, mad, mad_hi, mad_lo, knot_y, kr, tw, lines=True, twist_on=True):
    """A wooden cotton reel side on, which is a serif capital I, wound with madder, its
    loose end climbing to a knot above it: a lowercase i made of a reel and a knot."""
    fl = (bottom - top) * .12          # flange thickness
    core_w = w * .78
    body = []
    # the thread on the barrel
    bx0, bx1 = cx - core_w / 2, cx + core_w / 2
    by0, by1 = top + fl, bottom - fl
    body.append(f'<rect x="{f(bx0)}" y="{f(by0)}" width="{f(core_w)}" height="{f(by1 - by0)}" fill="{mad}"/>')
    if lines:
        step = max(1.0, tw * .55)
        segs = []
        y = by0 + step * .6
        while y < by1:
            segs.append(f"M{f(bx0)} {f(y + step * .25)}L{f(bx1)} {f(y - step * .25)}")
            y += step
        body.append(f'<path stroke="{mad_hi}" stroke-width="{f(step * .28)}" opacity=".55" d="{"".join(segs)}"/>')
        body.append(f'<rect x="{f(bx1 - core_w * .2)}" y="{f(by0)}" width="{f(core_w * .2)}" height="{f(by1 - by0)}" fill="{mad_lo}" opacity=".35"/>')
    # flanges: the serifs of the I
    r = fl * .35
    for y in (top, bottom - fl):
        body.append(f'<rect x="{f(cx - w / 2)}" y="{f(y)}" width="{f(w)}" height="{f(fl)}" rx="{f(r)}" fill="{wood}"/>')
        body.append(f'<rect x="{f(cx - w / 2 + r)}" y="{f(y + fl * .16)}" width="{f(w - 2 * r)}" height="{f(fl * .2)}" rx="{f(fl * .1)}" fill="{wood_hi}" opacity=".7"/>')
    # the loose end: leaves the barrel at the top right and climbs to tie the dot
    sx, sy = bx1 - tw * .4, by0 + (by1 - by0) * .16
    t = catmull([(sx, sy), (cx + w * .62, sy - (bottom - top) * .1), (cx + w * .46, knot_y + kr * 1.6), (cx + kr * .3, knot_y)], 16)
    body.append(thread(t, tw, mad, (0.0, 0.0), mad_hi if twist_on else None))
    body.append(knot(cx, knot_y, kr, mad, hi(mad, .12) if mad.startswith("#") else mad_hi, None if not mad.startswith("#") else lo(mad, -.15), coil_w=.18))
    return "".join(body)


@direction
def d06_spool():
    word, info = glyphs("nkling", 28.2, 0)
    sp = spool(14.7, -47, 0, 21.5, "X4", "X6", "X7", "X2", "X3", "X5", -61, 8.2, 2.2, lines=False, twist_on=False)
    sp = (sp.replace('fill="X4"', 'class="wal"').replace('fill="X6"', 'class="wal-hi"').replace('fill="X7"', 'class="wal"')
          .replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"').replace('fill="X5"', 'class="mad-lo"')
          .replace('stroke="X3"', 'class="s-mad"'))
    view = wm_view(0, 28.2 + info["width"])
    mk = spool(50, 34, 96, 40, "X4", "X6", "X7", "X2", "X3", "X5", 15, 10, 3.2, lines=False, twist_on=False)
    mk = (mk.replace('fill="X4"', 'class="wal"').replace('fill="X6"', 'class="wal-hi"').replace('fill="X7"', 'class="wal"')
          .replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"').replace('fill="X5"', 'class="mad-lo"'))

    def icon(pal, dark):
        m = pal["madder"]
        wood = "#B88A5A" if not dark else "#C49A73"
        body = spool(512, 400, 850, 380, wood, shade(wood, .14), shade(wood, -.2), m, hi(m, .16), lo(m, -.14), 236, 62, 24)
        return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))

    return dict(slug="06-spool", name="Spool", wordmark=flat_svg(view, sp + word), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 07 Tag

def tag_outline(cx, cy, w, h, chamfer, angle):
    x0, y0, x1, y1 = cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2
    pts = [(x0 + chamfer, y0), (x1 - chamfer, y0), (x1, y0 + chamfer), (x1, y1), (x0, y1), (x0, y0 + chamfer)]
    return rot(pts, angle, cx, cy)


def hang_tag(cx, cy, w, h, angle, fill, edge, stitch_fill, thread_fill, thread_hi, knot_fill, knot_hi, sw, tw, up=260.0, twist_on=True):
    """Inkling's margin tag: a woven label with a stitched border and an eyelet, tied on
    with its thread."""
    ch = w * .26
    pts = tag_outline(cx, cy, w, h, ch, angle)
    body = f'<path fill="{fill}" d="{rounded_poly(pts, w * .06)}"/>'
    inset = tag_outline(cx, cy + 0, w - sw * 7, h - sw * 7, ch * .78, angle)
    body += running(poly_pts(inset, 1.0), sw, stitch_fill, None, sw * 3.2, sw * 2.0, seed=4)
    # eyelet near the top: a hole, ringed with buttonhole stitch
    ex, ey = rot([(cx, cy - h / 2 + w * .2)], angle, cx, cy)[0]
    er = w * .085
    body += f'<circle cx="{f(ex)}" cy="{f(ey)}" r="{f(er * 1.55)}" fill="{stitch_fill}"/>'
    body += f'<circle cx="{f(ex)}" cy="{f(ey)}" r="{f(er)}" fill="{edge}"/>'
    # the thread: through the eyelet, over the top edge and up out of the picture
    t_hi = thread_hi if twist_on else None
    loop = catmull([(ex - er * .2, ey + er * .2), (ex - er * 1.2, ey - er * 2.2), (ex - er * .4, ey - er * 3.6), (ex + er * .6, ey - er * 3.2)], 12)
    rise = catmull([(ex + er * .6, ey - er * 3.2), (ex + er * 1.6, ey - up * .35), (ex + er * 3.2, ey - up * .7), (ex + er * 2.2, ey - up)], 16)
    body += thread(loop, tw, thread_fill, (0.0, 0.0), t_hi) + thread(rise, tw, thread_fill, (0.0, 0.0), t_hi)
    body += knot(ex + er * .6, ey - er * 3.2, tw * 1.35, knot_fill, knot_hi, coil_w=.2)
    return body


@direction
def d07_tag():
    word, info = glyphs("inkling", 60, 0)
    tg = hang_tag(26, -26, 34, 46, -10, "X2", "X9", "X8", "X2", "X3", "X2", "X3", 1.5, 2.0, up=58, twist_on=False)
    tg = (tg.replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"').replace('fill="X8"', 'class="paper"')
          .replace('fill="X9"', 'class="paper"'))
    wm = tg + word.replace("<path ", "<path ")
    view = wm_view(2, 60 + info["width"], y0=-86)
    mk = hang_tag(52, 60, 50, 66, -10, "X2", "X9", "X8", "X2", "X3", "X2", "X3", 2.1, 2.6, up=60, twist_on=False)
    mk = (mk.replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"').replace('fill="X8"', 'class="paper"')
          .replace('fill="X9"', 'class="paper"'))

    def icon(pal, dark):
        m = pal["madder"]
        body = hang_tag(512, 590, 430, 560, -9, m, pal["cloth"], pal["card"] if not dark else "#F3E9DA", shade(m, -.08), hi(m, .14),
                        shade(m, -.02), hi(m, .14), 14, 20, up=420)
        return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))

    return dict(slug="07-tag", name="Tag", wordmark=flat_svg(view, wm), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 08 Woven label

def woven_label(x0, y0, x1, y1, fill, rib, fold, letters, letter_fill, rib_letter, edge_stitch, sw, folds=True):
    """A woven name label: a narrow ribbon with the name woven in, its ends folded under
    and sewn down. The weave shows as fine ribs across the whole label, letters included."""
    h = y1 - y0
    body = f'<rect x="{f(x0)}" y="{f(y0)}" width="{f(x1 - x0)}" height="{f(h)}" fill="{fill}"/>'
    if folds:
        fw = h * .16
        body += f'<path fill="{fold}" d="M{f(x0)} {f(y0)} L{f(x0 + fw)} {f(y0 + h * .08)} L{f(x0 + fw)} {f(y1 - h * .08)} L{f(x0)} {f(y1)}Z"/>'
        body += f'<path fill="{fold}" d="M{f(x1)} {f(y0)} L{f(x1 - fw)} {f(y0 + h * .08)} L{f(x1 - fw)} {f(y1 - h * .08)} L{f(x1)} {f(y1)}Z"/>'
    body += letters.replace('class="ink"', f'fill="{letter_fill}"')
    step = max(.8, h / 34)
    segs = "".join(f"M{f(x0)} {f(y)}H{f(x1)}" for y in [y0 + step * (k + .5) for k in range(int(h / step))])
    body += f'<path stroke="{rib}" stroke-width="{f(step * .3)}" opacity=".5" d="{segs}"/>'
    body += f'<path stroke="{rib_letter}" stroke-width="{f(step * .22)}" opacity=".18" d="{segs}"/>'
    if edge_stitch:
        body += running([(x0 + h * .3, y0 + h * .1), (x1 - h * .3, y0 + h * .1)], sw, edge_stitch, None, sw * 3.6, sw * 2.2, seed=8)
        body += running([(x0 + h * .3, y1 - h * .1), (x1 - h * .3, y1 - h * .1)], sw, edge_stitch, None, sw * 3.6, sw * 2.2, seed=9)
    return body


@direction
def d08_label():
    # a woven label is a made thing: the same madder ribbon and cream weft in light and dark
    word, info = glyphs("inkling", 26, 0)
    lab = woven_label(0, -88, info["width"] + 52, 36, "#A63F32", "#C25A4A", "#7E2B22", word, "#F6EEE2", "#A63F32", None, 0)
    view = wm_view(0, info["width"] + 52, y0=-88, y1=36, pad=8)
    mw, _ = glyphs("ik", 21, 73, .9)
    mk = woven_label(4, 14, 96, 92, "#A63F32", "#C25A4A", "#7E2B22", mw, "#F6EEE2", "#A63F32", None, 0)

    def icon(pal, dark):
        # the whole tile is the label: ribbed madder, the name woven across it in cream,
        # selvedges top and bottom, and the stitches that hold it down at each end
        m = "#A63F32" if not dark else "#8E3529"
        s = 2.2
        _, inf = glyphs("inkling", 0, 0, s)
        letters, _ = glyphs("inkling", 512 - inf["width"] / 2, 512 + 25 * s - 2, s)
        body = woven_label(0, 0, 1024, 1024, m, hi(m, .1), lo(m, -.1), letters, "#F6EEE2" if not dark else "#E9DCC8", m, None, 0, folds=False)
        for y in (196, 828):
            body += f'<rect x="0" y="{y - 14}" width="1024" height="28" fill="{lo(m, -.08)}"/>'
            body += running([(150, y), (874, y)], 11, "#F6EEE2" if not dark else "#E9DCC8", None, 46, 26, seed=y)
        return icon_svg(m, body)

    return dict(slug="08-label", name="Woven label", wordmark=flat_svg(view, lab), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))

# ================================================================ 09 Pamphlet stitch

def gutter_tie(x, y_top, y_bot, w, mad, mad_hi, kr, tails=1.0, holes=None, twist_on=True):
    """Inside a pamphlet-stitched booklet: one long stitch down the fold between the top
    and bottom holes, the two ends tied over it at the middle hole, tails left loose."""
    t_hi = mad_hi if twist_on else None
    ym = (y_top + y_bot) / 2
    body = ""
    if holes:
        for y in (y_top, ym, y_bot):
            body += f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(w * .75)}" fill="{holes}"/>'
    body += stitch((x, y_top), (x, y_bot), w, mad, t_hi)
    L = (y_bot - y_top) * .3 * tails
    t1 = catmull([(x, ym), (x - L * .3, ym - L * .35), (x - L * .55, ym - L * .5), (x - L * .9, ym - L * .45)], 12)
    t2 = catmull([(x, ym), (x + L * .3, ym + L * .3), (x + L * .6, ym + L * .52), (x + L * .95, ym + L * .5)], 12)
    body += thread(t1, w * .9, mad, (0.0, 0.25), t_hi) + thread(t2, w * .9, mad, (0.0, 0.25), t_hi)
    body += knot(x, ym, kr, mad, hi(mad, .12) if mad.startswith("#") else mad_hi, lo(mad, -.15) if mad.startswith("#") else None, coil_w=.2)
    return body


@direction
def d09_pamphlet():
    left, li = glyphs("ink", 0, 0)
    right, ri = glyphs("ling", li["width"] + 22, 0)
    gx = li["width"] + 11
    tie = gutter_tie(gx, -76, 24, 2.7, "X2", "X3", 5.2, tails=.9, twist_on=False)
    tie = tie.replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"')
    view = wm_view(0, li["width"] + 22 + ri["width"])
    # the mark: an open booklet from above
    mk = ('<path class="paper" d="M50 22 L12 16 Q9 16 9 19 L9 82 Q9 85 12 85 L50 90 Z"/>'
          '<path class="paper" d="M50 22 L88 16 Q91 16 91 19 L91 82 Q91 85 88 85 L50 90 Z"/>'
          '<path class="rule" d="M50 22 L50 90 L46 89.4 L46 21.4Z" opacity=".9"/>'
          '<path fill="none" class="s-faint" stroke-width="1.6" d="M50 22 L12 16 Q9 16 9 19 L9 82 Q9 85 12 85 L50 90 L88 85 Q91 85 91 82 L91 19 Q91 16 88 16 Z"/>')
    mk += gutter_tie(50, 30, 82, 3.4, "X2", "X3", 6, tails=1.0, twist_on=False).replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"')

    def icon(pal, dark):
        m = pal["madder"]
        page = pal["paper"] if not dark else "#2A272E"
        cover = "#6E4B2E" if not dark else "#3B2A1E"
        rule = pal["rule"] if not dark else "#3F3A45"
        edge = shade(page, -.1 if not dark else -.04)
        x0, x1, y0, y1 = 170, 854, 196, 846
        body = ""
        # the page block: two sheets below the top one show the notebook's thickness
        for k, dy in ((2, 14), (1, 7)):
            body += f'<path fill="{shade(edge, -.04 * k)}" d="M512 {y0 + dy} L{x0 - k * 3} {y0 - 6 + dy} L{x0 - k * 3} {y1 + dy} L512 {y1 + 10 + dy} L{x1 + k * 3} {y1 + dy} L{x1 + k * 3} {y0 - 6 + dy}Z"/>'
        body += f'<path fill="{page}" d="M512 {y0} L{x0 + 10} {y0 - 6} Q{x0} {y0 - 6} {x0} {y0 + 4} L{x0} {y1 - 4} Q{x0} {y1} {x0 + 10} {y1} L512 {y1 + 10}Z"/>'
        body += f'<path fill="{page}" d="M512 {y0} L{x1 - 10} {y0 - 6} Q{x1} {y0 - 6} {x1} {y0 + 4} L{x1} {y1 - 4} Q{x1} {y1} {x1 - 10} {y1} L512 {y1 + 10}Z"/>'
        # the fold, as two flat bands of shade either side of the gutter (no gradient)
        fold = shade(page, -.06 if not dark else -.025)
        body += f'<path fill="{fold}" d="M470 {y0 - 1} L512 {y0} L554 {y0 - 1} L554 {y1 + 9} L512 {y1 + 10} L470 {y1 + 9}Z"/>'
        lines = "".join(f"M{xa} {y}H{xb}" for y in range(282, 800, 52) for xa, xb in ((228, 458), (566, 796)))
        body += f'<path stroke="{rule}" stroke-width="5" stroke-linecap="round" d="{lines}"/>'
        body += f'<path stroke="{shade(page, -.16 if not dark else .06)}" stroke-width="3" d="M512 {y0}V{y1 + 10}"/>'
        body += gutter_tie(512, 262, 782, 26, m, hi(m, .2), 52, holes=shade(page, -.34 if not dark else -.12))
        return icon_svg(cover, body)

    return dict(slug="09-pamphlet", name="Pamphlet", wordmark=flat_svg(view, left + tie + right),
                mark=flat_svg((0, 0, 100, 100), mk), icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 10 Darn

def darn(cx, cy, n, pitch, tw, warp, warp_hi, weft, weft_hi, hole=None, seed=5, ends=True):
    """A darn: warp threads laid across a hole, then weft woven over and under them.
    The ends run on past the hole into sound cloth, unevenly, as they do by hand."""
    rnd = random.Random(seed)
    half = (n - 1) * pitch / 2
    body = ""
    if hole:
        hp = [(cx + math.cos(a) * half * (0.62 + rnd.random() * .16), cy + math.sin(a) * half * (0.6 + rnd.random() * .16))
              for a in [k * 2 * math.pi / 14 for k in range(14)]]
        body += f'<path fill="{hole}" d="{P(catmull(hp + hp[:1], 6))}"/>'
    wx = [cx - half + k * pitch for k in range(n)]
    wy = [cy - half + k * pitch for k in range(n)]
    ext = lambda: pitch * (0.55 + rnd.random() * .5) if ends else pitch * .5  # noqa: E731
    warps = [(x, cy - half - ext(), cy + half + ext()) for x in wx]
    wefts = [(y, cx - half - ext(), cx + half + ext()) for y in wy]
    for x, ya, yb in warps:
        body += stitch((x, ya), (x, yb), tw, warp, warp_hi)
    for j, (y, xa, xb) in enumerate(wefts):
        body += stitch((xa, y), (xb, y), tw, weft, weft_hi)
        # where the warp lies on top, draw a piece of it back over the weft
        for i, x in enumerate(wx):
            if (i + j) % 2 == 0:
                body += stitch((x, y - pitch * .5), (x, y + pitch * .5), tw, warp, warp_hi)
    return body


@direction
def d10_darn():
    word, info = glyphs("inkling", dotless_first_i=True)
    d = info["dot"]
    # the dot: a darn woven so close it reads as a square of cloth, not a grid
    dn = darn(d["cx"], d["cy"] + .6, 3, 4.6, 4.5, "X2", None, "X1", None, ends=False)
    dn = dn.replace('fill="X2"', 'class="mad"').replace('fill="X1"', 'class="ink"')
    view = wm_view(0, info["width"])
    mk = darn(50, 50, 5, 15, 11, "X2", None, "X1", None, hole=None, seed=3)
    mk = mk.replace('fill="X2"', 'class="mad"').replace('fill="X1"', 'class="ink"')

    def icon(pal, dark):
        m = pal["madder"]
        ink = pal["ink"] if not dark else "#EDE6DA"
        body = darn(512, 512, 7, 66, 46, m, hi(m, .2), ink, shade(ink, .3 if not dark else -.3), hole=shade(pal["cloth"], -.5 if not dark else -.06))
        return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))

    return dict(slug="10-darn", name="Darn", wordmark=flat_svg(view, word + dn), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))

# ================================================================ 11 Loose end (running stitch)

def loose_end(stitches, w, mad, mad_hi, rise, curl, twist_on=True):
    """A running stitch that stops, and the loose end of its thread lifting off the cloth
    and curling over: a question before it is answered. No knot: nothing is tied yet.
    `stitches` is a list of (x0, x1, y); the thread comes up at the last hole."""
    body = "".join(stitch((a, y), (b, y), w, mad, mad_hi if twist_on else None) for a, b, y in stitches)
    x_end, y = stitches[-1][1] + w * 2.2, stitches[-1][2]
    t = catmull([(x_end, y), (x_end + curl * .5, y - rise * .22), (x_end + curl * .82, y - rise * .64), (x_end + curl * .42, y - rise),
                 (x_end - curl * .2, y - rise * .86), (x_end - curl * .26, y - rise * .56)], 16)
    body += thread(t, w * .92, mad, (0.02, 0.3), mad_hi if twist_on else None)
    return body


@direction
def d11_loose_end():
    word, info = glyphs("inkling")
    y = 35
    st = []
    x = 2.0
    rnd = random.Random(4)
    while x < 300:
        L = 10.5 * (1 + (rnd.random() - .5) * .16)
        st.append((x, min(x + L, 310), y + (rnd.random() - .5) * .6))
        x += L + 6.2 * (1 + (rnd.random() - .5) * .2)
    le = loose_end(st, 2.9, "X2", "X3", 54, 28, twist_on=False)
    le = le.replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"')
    view = wm_view(0, info["width"] + 34, y1=40)
    mk = loose_end([(4, 16, 84), (22, 34, 84), (40, 52, 84)], 5.4, "X2", "X3", 70, 34, twist_on=False)
    mk = mk.replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"')

    def icon(pal, dark):
        m = pal["madder"]
        body = loose_end([(170, 258, 724), (302, 390, 724), (434, 522, 724)], 40, m, hi(m, .2), 476, 244)
        return icon_svg(pal["paper"], body)

    return dict(slug="11-loose-end", name="Loose end", wordmark=flat_svg(view, word + le), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))

# ================================================================ 12 One thread (couched script)

SLANT = 0.17
SCRIPT = [
    # the loose end, then i, drawn without lifting: runs join at cusps
    [(-44, 14), (-30, 10), (-14, 3), (-2, -16), (6, -46)],
    [(6, -46), (3, -20), (4, -5), (10, 0), (18, -7)],
    # n
    [(18, -7), (21, -28), (24, -46)],
    [(24, -46), (23, -22), (22, 0)],
    [(22, 0), (25, -28), (34, -45), (43, -41), (46, -22), (46, -5), (52, 0), (59, -7)],
    # k
    [(59, -7), (65, -40), (71, -72), (66, -82), (61, -74), (60, -40), (60, 0)],
    [(60, 0), (61, -22), (70, -40), (80, -42), (80, -33), (70, -26), (63, -25)],
    [(63, -25), (70, -22), (76, -10), (81, -1), (88, 0), (95, -7)],
    # l
    [(95, -7), (101, -42), (106, -74), (101, -84), (96, -76), (96, -40), (98, -5), (104, 0), (111, -7)],
    # i
    [(111, -7), (115, -28), (119, -46)],
    [(119, -46), (117, -20), (117, -5), (122, 0), (130, -7)],
    # n
    [(130, -7), (133, -28), (136, -46)],
    [(136, -46), (135, -22), (134, 0)],
    [(134, 0), (137, -28), (146, -45), (155, -41), (158, -22), (158, -5), (164, 0), (171, -7)],
    # g: the bowl, then down into the loop below the line, and away
    [(171, -7), (182, -36), (194, -46)],
    [(194, -46), (182, -48), (171, -38), (170, -18), (177, -4), (189, -7), (196, -26), (198, -46)],
    [(198, -46), (197, -12), (195, 16), (187, 31), (175, 33), (170, 24), (178, 12), (196, 3), (214, -6)],
]


def script_path(scale=1.0, ox=0.0, oy=0.0):
    runs = []
    for run in SCRIPT:
        pts = [(ox + (x - y * SLANT) * scale, oy + y * scale) for x, y in run]
        runs.append(catmull(pts, 10))
    allp = []
    for r in runs:
        allp += r if not allp else r[1:]
    return allp


def couched(pts, w, fill, hi_, tack, every, tack_w, twist_on=True):
    """Couching: the laid thread, held down by tiny stitches of a second thread across it."""
    body = thread(pts, w, fill, (0.03, 0.06), hi_ if twist_on else None, step=max(.5, w / 2.2))
    if tack:
        acc, segs = 0.0, []
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            L = math.hypot(x1 - x0, y1 - y0)
            acc += L
            if acc >= every and L > 0:
                acc = 0.0
                ux, uy = (x1 - x0) / L, (y1 - y0) / L
                nx, ny = -uy, ux
                segs.append(f"M{f(x1 - nx * w * .85)} {f(y1 - ny * w * .85)}L{f(x1 + nx * w * .85)} {f(y1 + ny * w * .85)}")
        body += f'<path stroke="{tack}" stroke-width="{f(tack_w)}" stroke-linecap="round" fill="none" d="{"".join(segs)}"/>'
    return body


@direction
def d12_one_thread():
    pts = script_path(1.0)
    th = couched(pts, 6.0, "X2", None, None, 0, 0, twist_on=False).replace('fill="X2"', 'class="mad"')
    kx, ky = 6 - (-64) * SLANT + 2, -64
    th += k_cls(kx, ky, 7.0)
    th += stitch_cls((119 + 64 * SLANT - 1.5, -63), (119 + 64 * SLANT + 2.5, -64.5), 3.8)
    th = f'<g transform="translate(47 0)">{th}</g>'
    view = wm_view(0, 216 + 4 + 47, y0=-90, y1=36)
    # the mark: the first stroke on its own, the loose end and the i, with its knot
    ms, mox, moy = 1.05, 60.5, 82
    first = []
    for run in SCRIPT[:2]:
        c = catmull([((x - y * SLANT) * ms + mox, y * ms + moy) for x, y in run], 10)
        first += c if not first else c[1:]
    mk = couched(first, 6.2, "X2", None, None, 0, 0, twist_on=False).replace('fill="X2"', 'class="mad"')
    mk += k_cls(mox + (6 + 64 * SLANT + 2) * ms, moy - 64 * ms, 7.0)

    def icon(pal, dark):
        m = pal["madder"]
        s = 6.6
        ox, oy = 236, 716
        p = script_path(s, ox, oy)
        body = couched(p, 30, m, hi(m, .2), shade(pal["ink"], .25 if not dark else -.25), 48, 5.6)
        body += knot(ox + (6 + 64 * SLANT + 2) * s, oy - 64 * s, 44, m, hi(m, .12), lo(m, -.15), coil_w=.2)
        return icon_svg(pal["cloth"], body, cloth=cloth_for(pal, dark))

    return dict(slug="12-one-thread", name="One thread", wordmark=flat_svg(view, th), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ 13 Hoop

def hoop(cx, cy, R, ring, wood, wood_hi, wood_lo, brass, fabric, mad, mad_hi, s, twist_on=True, weave_id=None, tilt=0.0):
    """A flat embroidery hoop: the outer ring with its screw, the inner ring, the cloth
    held taut, and one thread at work in it. Tilted so the screw sits off twelve
    o'clock, and the stitches run on a curve, so it never reads as a clock face."""
    body = f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R)}" fill="{wood}"/>'
    body += f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R - ring * .55)}" fill="none" stroke="{wood_lo}" stroke-width="{f(ring * .08)}" opacity=".6"/>'
    body += f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R - ring)}" fill="{fabric}"/>'
    if weave_id:
        body += f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R - ring)}" fill="url(#{weave_id})"/>'
    lw, lh = ring * 1.5, ring * 1.25
    clamp = (f'<rect x="{f(cx - lw / 2)}" y="{f(cy - R - lh * .62)}" width="{f(lw)}" height="{f(lh)}" rx="{f(ring * .22)}" fill="{wood}"/>'
             f'<rect x="{f(cx - lw * .08)}" y="{f(cy - R - lh * .62)}" width="{f(lw * .16)}" height="{f(lh)}" fill="{wood_lo}" opacity=".7"/>'
             f'<rect x="{f(cx - lw * .7)}" y="{f(cy - R - lh * .3)}" width="{f(lw * 1.4)}" height="{f(ring * .36)}" rx="{f(ring * .18)}" fill="{brass}"/>')
    body += f'<g transform="rotate({f(tilt)} {f(cx)} {f(cy)})">{clamp}</g>'
    # the work: one cross-stitched motif from the sampler border, centred, so nothing
    # in the hoop points from its middle the way a clock hand would
    u = s * 1.35
    for dx, dy in ((0, -1), (-1, 0), (1, 0), (0, 1), (0, 0)):
        body += cross(cx + dx * u * 1.6, cy + R * .04 + dy * u * 1.6, u * .62, s * .62, mad, mad_hi if twist_on else None)
    return body


@direction
def d13_hoop():
    word, info = glyphs("inkling", 74, 0)
    hp = hoop(32, -34, 30, 5.4, "X4", "X6", "X7", "X9", "X8", "X2", "X3", 5.2, twist_on=False, tilt=38)
    hp = (hp.replace('fill="X4"', 'class="wal"').replace('fill="X6"', 'class="wal-hi"').replace('stroke="X7"', 'class="s-ink"')
          .replace('fill="X7"', 'class="ink"').replace('fill="X9"', 'class="ochre"').replace('fill="X8"', 'class="paper"')
          .replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"'))
    view = wm_view(-4, 74 + info["width"], y0=-80)
    mk = hoop(50, 52, 38, 7, "X4", "X6", "X7", "X9", "X8", "X2", "X3", 7, twist_on=False, tilt=38)
    mk = (mk.replace('fill="X4"', 'class="wal"').replace('fill="X6"', 'class="wal-hi"').replace('stroke="X7"', 'class="s-ink"')
          .replace('fill="X7"', 'class="ink"').replace('fill="X9"', 'class="ochre"').replace('fill="X8"', 'class="paper"')
          .replace('fill="X2"', 'class="mad"').replace('fill="X3"', 'class="mad-hi"'))

    def icon(pal, dark):
        m = pal["madder"]
        wood = "#9A6D42" if not dark else "#B88E60"
        defs = weave("hw", shade(pal["paper"], -.18 if not dark else .1), .35, 7)
        fab = pal["paper"] if not dark else "#2B282E"
        body = hoop(512, 536, 336, 46, wood, shade(wood, .12), shade(wood, -.24), "#D9B467", fab, m, hi(m, .2), 60, weave_id="hw", tilt=38)
        return icon_svg(pal["cloth"], body, defs=defs, cloth=cloth_for(pal, dark))

    return dict(slug="13-hoop", name="Hoop", wordmark=flat_svg(view, hp + word), mark=flat_svg((0, 0, 100, 100), mk),
                icon=icon(LIGHT, False), icon_dark=icon(DARK, True))


# ================================================================ words

# One line on the object, one on the idea, one on the risk, for the review page.
META = {
    "01-knot": dict(
        object="A French knot: thread wound twice round the needle and taken back down beside where it came up. It is how embroiderers make a dot.",
        idea="Inkling lives in the dot of the i, so the whole mark is that dot: one French knot, drawn the way the app draws it.",
        risk="Large, the coil reads as a spiral sweet; under 32 px it is a red dot, close to a notification badge."),
    "02-tied": dict(
        object="An overhand knot, the first knot anyone learns.",
        idea="The end of every thread in the app: a loose end, tied. The mark is the moment a question is answered.",
        risk="Under 32 px it is a squiggle, and a knotted line is also the stock mark for sailing and climbing."),
    "03-initial": dict(
        object="A monogrammed handkerchief: one initial in padded satin stitch, inside a hemstitched border.",
        idea="Mark the app the way linen was always marked as someone's own: a satin-stitched i, dotted with a knot.",
        risk="A lone i on a tile is the info symbol at 16 and 32 px, and the satin only shows from 128 px up."),
    "04-sampler": dict(
        object="A band sampler: a counted cross-stitch alphabet, signed with the maker's initials.",
        idea="Inkling signs its work the way samplers were signed: ik in counted cross-stitch, with a French knot for the dot.",
        risk="Pixel letters can read as an 8-bit game rather than thread, and ik means nothing to a stranger."),
    "05-needle": dict(
        object="A sewing needle, threaded.",
        idea="Stood on end, a needle is an i: its eye sits where the dot would, and Inkling's thread runs through it.",
        risk="Needle and thread is the stock mark of tailors and alteration shops, and in the wordmark the needle reads as a capital I."),
    "06-spool": dict(
        object="A wooden cotton reel. Side on, it is a serif capital I.",
        idea="A reel of madder thread whose loose end climbs and ties a knot above it: an i made of a reel and a knot.",
        risk="It says sewing supplies before it says notebook, and at 16 px it is a red barrel."),
    "07-tag": dict(
        object="Inkling's margin tag: a fabric tag with a stitched border, tied on at its eyelet.",
        idea="Every question in the app hangs on one of these; the icon is that tag, on its thread.",
        risk="Before it reads as a question, a tag reads as a price, a sale or a shop."),
    "08-label": dict(
        object="A woven name label, the kind sewn into a seam: the name is woven in, not printed.",
        idea="The whole tile is the label: ribbed madder with the name woven across it, stitched down at both edges.",
        risk="The name is all there is, and it cannot be read below 64 px; a label also says clothing brand."),
    "09-pamphlet": dict(
        object="A pamphlet-stitched notebook: three holes in the fold, one thread, the ends tied in a knot over the middle.",
        idea="A notebook held together by one thread tied off in its middle: the app's whole story, in how a notebook is bound.",
        risk="An open book is the stock icon for reading apps, and the split wordmark can read as two words."),
    "10-darn": dict(
        object="A darn: threads laid across a hole and woven over and under, in a contrasting colour on purpose.",
        idea="Inkling finds where your words don't hold and you mend them, visibly: madder warp, your ink as the weft.",
        risk="A woven square reads as a grid or a waffle, and mending suggests your writing was broken."),
    "11-loose-end": dict(
        object="The running stitch the app sews under words, and the loose end it leaves.",
        idea="Three stitches and a loose end curling over: a question mark without its dot, before anyone answers it.",
        risk="At 19 px the stitched underline reads as a spell-check line, and the icon's dashes can read as loading."),
    "12-one-thread": dict(
        object="Couching: one thread laid along a line and held down by tiny stitches of a second thread.",
        idea="The name in one thread, never lifted: it starts as a loose end, and the knot on the i comes last, the way you dot an i.",
        risk="The only direction with drawn letters, which the language otherwise refuses; script marks look alike, and it is a scribble below 64 px."),
    "13-hoop": dict(
        object="An embroidery hoop: two wooden rings and a brass screw holding the cloth taut.",
        idea="Today's idea redrawn flat, in the app's own stitches, holding one motif from the border Inkling stitches as threads are tied.",
        risk="At 16 px a ring with a screw reads as a clock; if the hoop itself is what doesn't fit, this doesn't either."),
}


# ================================================================ output

def write_all(only=None):
    OUT.mkdir(parents=True, exist_ok=True)
    assets = []
    for fn in DIRECTIONS:
        if only and fn.__name__ not in only:
            continue
        d = fn()
        folder = OUT / d["slug"]
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "wordmark.svg").write_text(d["wordmark"])
        (folder / "mark.svg").write_text(d["mark"])
        (folder / "icon.svg").write_text(d["icon"])
        (folder / "icon-dark.svg").write_text(d["icon_dark"])
        d.update(META[d["slug"]])
        assets.append(d)
    return assets


if __name__ == "__main__":
    import sys
    from logo_page import build_page
    assets = write_all(sys.argv[1:] or None)
    if not sys.argv[1:]:
        build_page(assets)
    print("ok")

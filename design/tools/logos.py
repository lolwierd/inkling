# /// script
# dependencies = ["fonttools", "uharfbuzz"]
# ///
"""Build the three directions' wordmarks and app icons as SVG.

Every wordmark is the word "inkling" outlined from its direction's typeface, with
the dot of the first i replaced by that direction's mark: the place where Inkling
lives is the smallest piece of ink in its own name.
"""
import json
import math
import random
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent
FONTS = HERE / "fonts"  # fetch with ./fetch-fonts.sh
OUT = Path("/Users/ayaan/Projects/inkling/design/directions/assets")


def outline(font, text, size, track=0.0):
    args = ["uv", "run", "--quiet", str(HERE / "outline.py"), str(FONTS / font), text, str(size)]
    if track:
        args += ["--track", str(track)]
    return json.loads(subprocess.check_output(args))


def f(v):
    return f"{v:.1f}".rstrip("0").rstrip(".")


# ---------- strokes that look drawn ----------

def cubic(p0, p1, p2, p3, n=48):
    pts = []
    for i in range(n + 1):
        t = i / n
        a = (1 - t) ** 3
        b = 3 * (1 - t) ** 2 * t
        c = 3 * (1 - t) * t * t
        d = t ** 3
        pts.append((a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]))
    return pts


def poly(*segs):
    pts = []
    for s in segs:
        pts.extend(s if not pts else s[1:])
    return pts


def resample(pts, step):
    out = [pts[0]]
    acc = 0.0
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        seg = math.hypot(x1 - x0, y1 - y0)
        if seg == 0:
            continue
        t = step - acc
        while t <= seg:
            out.append((x0 + (x1 - x0) * t / seg, y0 + (y1 - y0) * t / seg))
            t += step
        acc = seg - (t - step)
    if out[-1] != pts[-1]:
        out.append(pts[-1])
    return out


def ribbon(pts, width, taper=(0.25, 0.25), wobble=0.0, seed=1):
    """A filled outline around a centre line: a stroke whose width swells and
    tapers like a pencil or a brush, instead of SVG's even stroke."""
    rnd = random.Random(seed)
    n = len(pts)
    left, right = [], []
    for i, (x, y) in enumerate(pts):
        a = pts[max(i - 1, 0)]
        b = pts[min(i + 1, n - 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        ln = math.hypot(dx, dy) or 1
        nx, ny = -dy / ln, dx / ln
        t = i / (n - 1)
        w = width(t) if callable(width) else width
        # ease the ends in and out
        if t < taper[0]:
            w *= 0.35 + 0.65 * math.sin(t / taper[0] * math.pi / 2)
        if t > 1 - taper[1]:
            w *= 0.35 + 0.65 * math.sin((1 - t) / taper[1] * math.pi / 2)
        j = (rnd.random() - 0.5) * wobble
        left.append((x + nx * (w / 2 + j), y + ny * (w / 2 + j)))
        right.append((x - nx * (w / 2 - j), y - ny * (w / 2 - j)))
    ring = left + right[::-1]
    d = "M" + " L".join(f"{f(x)} {f(y)}" for x, y in ring) + " Z"
    return d


def polyline_d(pts):
    return "M" + " L".join(f"{f(x)} {f(y)}" for x, y in pts)


def superellipse(cx, cy, r, n=5.0, steps=256):
    pts = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        c, s = math.cos(t), math.sin(t)
        x = cx + r * math.copysign(abs(c) ** (2 / n), c)
        y = cy + r * math.copysign(abs(s) ** (2 / n), s)
        pts.append((x, y))
    return "M" + " L".join(f"{f(x)} {f(y)}" for x, y in pts) + " Z"


GRAIN = """<filter id="{id}" x="-10%" y="-10%" width="120%" height="120%">
  <feTurbulence type="fractalNoise" baseFrequency="{freq}" numOctaves="2" seed="7" result="n"/>
  <feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 -2.2 {keep}" result="a"/>
  <feComposite in="SourceGraphic" in2="a" operator="in"/>
</filter>"""


def word_paths(data, skip_first_dot=True):
    """Paths for every glyph; the first i is drawn without its dot."""
    out = []
    first_i_done = False
    dot = None
    for g in data["glyphs"]:
        if g["glyph"] == "i" and not first_i_done and skip_first_dot:
            first_i_done = True
            dot = g["dot"]
            # redraw this i from its stem only
            out.append(("stem", g))
        else:
            out.append(("glyph", g))
    return out, dot


def stem_only(font, size, gx):
    """Outline the dotless i at the same place as the dotted one."""
    d = outline(font, "ı", size)
    g = d["glyphs"][0]
    return g["d"], g


def build_wordmark(font, size, mark_svg, pad, track=0.0, extra_below="", height_extra=0.0, ink_class="ink"):
    data = outline(font, "inkling", size, track)
    meta = data["meta"]
    parts = []
    dot = None
    first = True
    for g in data["glyphs"]:
        if g["glyph"] == "i" and first:
            first = False
            dot = g["dot"]
            sd, sg = stem_only(font, size, g["x"])
            parts.append(f'<path class="{ink_class}" transform="translate({f(g["x"])} 0)" d="{sd}"/>')
        else:
            parts.append(f'<path class="{ink_class}" d="{g["d"]}"/>')
    asc = meta["ascender"]
    desc = -meta["descender"]
    w = meta["width"]
    x0, y0 = -pad, -(asc * 0.78) - pad
    vw = w + pad * 2
    vh = asc * 0.78 + desc + pad * 2 + height_extra
    mark = mark_svg(dot, meta)
    body = "\n  ".join(parts)
    return (x0, y0, vw, vh), body, mark, extra_below, meta, dot


# ---------- Proof ----------
PF = {"ink": "#1B1A17", "pencil": "#2F6FB3", "mark": "#4F8FD0", "paper": "#FBFBF8", "desk": "#ECECE7",
      "ink_d": "#E9E6DE", "pencil_d": "#7CB2EC", "paper_d": "#121417"}


def pf_caret(dot, meta, scale=1.0, cls="pencil"):
    cx, cy = dot["cx"], dot["cy"] - 1.5 * scale
    hw, hh = 10.5 * scale, 8.2 * scale
    left = (cx - hw, cy + hh)
    apex = (cx + 0.6 * scale, cy - hh)
    right = (cx + hw + 0.8 * scale, cy + hh - 0.6 * scale)
    up = cubic(left, (left[0] + 3 * scale, left[1] - 5 * scale), (apex[0] - 3 * scale, apex[1] + 4 * scale), apex, 24)
    down = cubic(apex, (apex[0] + 3 * scale, apex[1] + 4 * scale), (right[0] - 3 * scale, right[1] - 6 * scale), right, 24)
    pts = resample(poly(up, down), 0.6 * scale)
    d = ribbon(pts, lambda t: 3.3 * scale * (1.0 + 0.12 * math.sin(t * math.pi)), (0.12, 0.2), 0.12 * scale, 4)
    return f'<path class="{cls}" filter="url(#pf-grain)" d="{d}"/>'


def pf_ring(dot, meta, scale=1.0, cls="pencil"):
    cx, cy = dot["cx"] + 0.4 * scale, dot["cy"] - 1.0 * scale
    rx, ry = 9.6 * scale, 8.0 * scale
    pts = []
    a0 = math.radians(205)
    for i in range(73):
        t = i / 72
        a = a0 + t * math.radians(395)
        r = 1 + 0.06 * t  # the pencil drifts outward and overshoots its start
        pts.append((cx + rx * r * math.cos(a), cy + ry * r * math.sin(a) - 1.2 * scale * t))
    pts = resample(pts, 0.6 * scale)
    d = ribbon(pts, 2.6 * scale, (0.15, 0.25), 0.1 * scale, 9)
    return f'<path class="{cls}" filter="url(#pf-grain)" d="{d}"/>'


# ---------- Chk'd ----------
CK = {"ink": "#22262A", "yellow": "#F2C94C", "red": "#C8372D", "blue": "#2D5DA8", "sheet": "#F2F7EE",
      "grid": "#D3E2D0", "ink_d": "#E8EEF6", "sheet_d": "#12233D"}


def ck_square(dot, meta, scale=1.0):
    cx, cy = dot["cx"], dot["cy"] - 0.5 * scale
    s = dot["w"] * 1.12
    return (f'<rect class="hi" x="{f(cx - s / 2)}" y="{f(cy - s / 2)}" width="{f(s)}" height="{f(s)}" '
            f'transform="rotate(-7 {f(cx)} {f(cy)})" rx="{f(1.2 * scale)}"/>')


def ck_tick(dot, meta, scale=1.0):
    cx, cy = dot["cx"], dot["cy"]
    a = (cx - 7 * scale, cy - 1 * scale)
    b = (cx - 1.5 * scale, cy + 6 * scale)
    c = (cx + 11 * scale, cy - 12 * scale)
    s1 = cubic(a, (a[0] + 2 * scale, a[1] + 2 * scale), (b[0] - 2 * scale, b[1] - 1 * scale), b, 16)
    s2 = cubic(b, (b[0] + 3 * scale, b[1] - 6 * scale), (c[0] - 6 * scale, c[1] + 7 * scale), c, 28)
    pts = resample(poly(s1, s2), 0.5 * scale)
    d = ribbon(pts, lambda t: 3.6 * scale * (0.9 + 0.3 * (1 - t)), (0.1, 0.45), 0.1 * scale, 3)
    return f'<path class="red" d="{d}"/>'


# ---------- Loose Threads ----------
TH = {"ink": "#26232B", "madder": "#A63F32", "madder_hi": "#D07A66", "madder_lo": "#7A2A21", "indigo": "#2E3F7A",
      "ochre": "#B07A1E", "sage": "#4F7350", "cloth": "#F3EDE2", "ink_d": "#EDE6DA", "cloth_d": "#1D1B1F"}


def th_knot(dot, meta, scale=1.0, tail=True):
    cx, cy = dot["cx"], dot["cy"] + 0.5 * scale
    r = dot["w"] * 0.56
    parts = []
    if tail:
        # a loose end leaving the knot: the question is still open
        t0 = (cx + r * 0.7, cy + r * 0.5)
        pts = cubic(t0, (cx + r * 2.6, cy + r * 1.6), (cx + r * 3.2, cy - r * 1.8), (cx + r * 4.6, cy - r * 1.1), 40)
        d = ribbon(resample(pts, 0.5 * scale), 2.1 * scale, (0.02, 0.5), 0.0, 2)
        parts.append(f'<path class="madder" d="{d}"/>')
    parts.append(f'<circle class="madder" cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}"/>')
    # the wraps of a French knot: three turns of thread around the centre
    for k, (a0, sweep, rr) in enumerate([(200, 150, 0.78), (20, 150, 0.78), (110, 160, 0.42)]):
        pts = []
        for i in range(25):
            a = math.radians(a0 + sweep * i / 24)
            pts.append((cx + r * rr * math.cos(a), cy + r * rr * 0.86 * math.sin(a)))
        d = ribbon(pts, 0.9 * scale * (1.1 if k < 2 else 0.9), (0.3, 0.3), 0.0, 5 + k)
        parts.append(f'<path class="madder-hi" d="{d}"/>')
    return "\n  ".join(parts)


def svg_doc(view, defs, style, body):
    x, y, w, h = view
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{f(x)} {f(y)} {f(w)} {f(h)}" width="{f(w)}" height="{f(h)}">\n'
            f'<style>{style}</style>\n<defs>{defs}</defs>\n  {body}\n</svg>\n')


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main():
    lab = []

    # ===== Proof wordmarks (caret and ring) =====
    for name, mark in (("caret", pf_caret), ("ring", pf_ring)):
        view, body, m, _, meta, dot = build_wordmark("literata-72-400.ttf", 100, mark, 12)
        style = (f".ink{{fill:{PF['ink']}}} .pencil{{fill:{PF['pencil']}}}"
                 f"@media (prefers-color-scheme:dark){{.ink{{fill:{PF['ink_d']}}} .pencil{{fill:{PF['pencil_d']}}}}}")
        defs = GRAIN.format(id="pf-grain", freq="1.4", keep="1.9")
        svg = svg_doc(view, defs, style, body + "\n  " + m)
        write(OUT / "proof" / f"wordmark-{name}.svg", svg)
        lab.append(("Proof wordmark: " + name, f"proof/wordmark-{name}.svg"))

    # ===== Chk'd wordmarks =====
    for name, mark in (("square", ck_square), ("tick", ck_tick)):
        view, body, m, _, meta, dot = build_wordmark("atkinson-next-700.ttf", 100, mark, 12)
        style = (f".ink{{fill:{CK['ink']}}} .hi{{fill:{CK['yellow']}}} .red{{fill:{CK['red']}}}"
                 f"@media (prefers-color-scheme:dark){{.ink{{fill:{CK['ink_d']}}}}}")
        svg = svg_doc(view, "", style, body + "\n  " + m)
        write(OUT / "check" / f"wordmark-{name}.svg", svg)
        lab.append(("Chk'd wordmark: " + name, f"check/wordmark-{name}.svg"))

    # ===== Threads wordmark =====
    view, body, m, _, meta, dot = build_wordmark("fraunces-144-400-soft.ttf", 100, th_knot, 14)
    style = (f".ink{{fill:{TH['ink']}}} .madder{{fill:{TH['madder']}}} .madder-hi{{fill:{TH['madder_hi']};opacity:.9}}"
             f"@media (prefers-color-scheme:dark){{.ink{{fill:{TH['ink_d']}}} .madder{{fill:#C9584A}}}}")
    svg = svg_doc(view, "", style, body + "\n  " + m)
    write(OUT / "threads" / "wordmark-knot.svg", svg)
    lab.append(("Threads wordmark: knot", "threads/wordmark-knot.svg"))

    write(HERE / "logolab.json", json.dumps(lab))
    print("ok", len(lab))


if __name__ == "__main__":
    main()

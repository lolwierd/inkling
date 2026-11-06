# /// script
# dependencies = ["fonttools", "uharfbuzz"]
# ///
"""Second pass on the marks, plus the app icons.

Proof   : a blue-pencil caret as the dot of the first i (the copy editor's
          "insert here"); icon = that i, big, on a proof sheet with crop marks.
Chk'd   : a yellow highlighter square as the dot; icon = the CHK'D cell of a
          title block with a red tick that overshoots it, on grid paper.
Threads : a French knot as the dot; icon = a hoop with a running-stitch "?"
          whose dot is the same knot.
"""
import json
import math
import random
from pathlib import Path

from logos import (CK, OUT, PF, TH, cubic, f, outline, poly, resample, ribbon, superellipse, write)

HERE = Path(__file__).parent


def glyph_run(font, text, size, x0, y0, cls, skip_dot_first_i=False, dot_out=None):
    data = outline(font, text, size)
    parts = []
    first = True
    for g in data["glyphs"]:
        if g["glyph"] == "i" and first and skip_dot_first_i:
            first = False
            if dot_out is not None:
                dot_out.update({k: v for k, v in g["dot"].items()})
                dot_out["cx"] += x0
                dot_out["cy"] += y0
                dot_out["stem"] = [g["stem"][0] + x0, g["stem"][1] + y0, g["stem"][2] + x0, g["stem"][3] + y0]
            sd = outline(font, "ı", size)["glyphs"][0]["d"]
            parts.append(f'<path class="{cls}" transform="translate({f(x0 + g["x"])} {f(y0)})" d="{sd}"/>')
        else:
            parts.append(f'<path class="{cls}" transform="translate({f(x0)} {f(y0)})" d="{g["d"]}"/>')
    return "\n  ".join(parts), data["meta"]


# ---------------------------------------------------------------- marks

def caret(cx, cy, s, cls="pencil", seed=4, weight=1.0):
    """A copy editor's caret, drawn in one motion: up to the point, back down.
    Slightly lopsided, with the right leg longer, the way a hand makes it."""
    hw, hh = 11.0 * s, 9.5 * s
    left = (cx - hw, cy + hh * 0.92)
    apex = (cx + 0.8 * s, cy - hh)
    right = (cx + hw + 1.4 * s, cy + hh * 1.08)
    up = cubic(left, (left[0] + 2.5 * s, left[1] - 6 * s), (apex[0] - 2.2 * s, apex[1] + 5 * s), apex, 30)
    down = cubic(apex, (apex[0] + 2.2 * s, apex[1] + 5 * s), (right[0] - 2.5 * s, right[1] - 7 * s), right, 30)
    pts = resample(poly(up, down), 0.5 * s)
    d = ribbon(pts, lambda t: 4.6 * s * weight * (0.92 + 0.16 * math.sin(t * math.pi)), (0.1, 0.16), 0.15 * s, seed)
    return f'<path class="{cls}" d="{d}"/>'


def knot(cx, cy, r, base="madder", hi="madder-hi", lo="madder-lo"):
    """A French knot: a bead of thread wound three times round the needle.
    Base bead, a shadow on the lower right, then three wraps of lighter thread."""
    parts = [f'<circle class="{base}" cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}"/>']
    # shadow crescent, light from the top left
    parts.append(f'<path class="{lo}" d="M{f(cx + r * 0.95)} {f(cy - r * 0.3)} A{f(r)} {f(r)} 0 0 1 {f(cx - r * 0.3)} {f(cy + r * 0.95)} '
                 f'A{f(r * 0.8)} {f(r * 0.8)} 0 0 0 {f(cx + r * 0.95)} {f(cy - r * 0.3)} Z"/>')
    for k, (a0, sweep, rr, w) in enumerate([(160, 120, 0.62, 0.30), (300, 115, 0.62, 0.30), (50, 105, 0.6, 0.26)]):
        pts = []
        for i in range(31):
            a = math.radians(a0 + sweep * i / 30)
            rad = r * rr * (1 + 0.18 * math.sin(i / 30 * math.pi))
            pts.append((cx + rad * math.cos(a), cy + rad * math.sin(a)))
        d = ribbon(pts, r * w, (0.35, 0.35), 0.0, 5 + k)
        parts.append(f'<path class="{hi}" d="{d}"/>')
    # the dimple where the needle went back through
    parts.append(f'<circle class="{lo}" cx="{f(cx + r * 0.08)}" cy="{f(cy + r * 0.06)}" r="{f(r * 0.17)}"/>')
    return "\n  ".join(parts)


def tick(a, b, c, s, cls="red", seed=3):
    s1 = cubic(a, (a[0] + 4 * s, a[1] + 3 * s), (b[0] - 4 * s, b[1] - 2 * s), b, 24)
    s2 = cubic(b, (b[0] + 8 * s, b[1] - 16 * s), (c[0] - 20 * s, c[1] + 22 * s), c, 48)
    pts = resample(poly(s1, s2), 0.6 * s)
    d = ribbon(pts, lambda t: 7.5 * s * (1.25 - 0.55 * t), (0.08, 0.35), 0.2 * s, seed)
    return f'<path class="{cls}" d="{d}"/>'


def running_stitch(pts, stitch, gap, width, cls, seed=1, skip=None):
    """Dashes along a path: thread on the surface, then under the cloth."""
    pts = resample(pts, 0.5)
    out = []
    dist = 0.0
    seg_start = None
    acc = [pts[0]]
    on = True
    run = 0.0
    rnd = random.Random(seed)
    cur = [pts[0]]
    target = stitch * (0.92 + rnd.random() * 0.16)
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        step = math.hypot(x1 - x0, y1 - y0)
        run += step
        cur.append((x1, y1))
        if run >= target:
            if on and len(cur) > 2:
                mid = cur[len(cur) // 2]
                if not (skip and skip(mid)):
                    out.append(ribbon(cur, width, (0.22, 0.22), 0.0, rnd.randint(1, 99)))
            on = not on
            run = 0.0
            cur = [(x1, y1)]
            target = (stitch if on else gap) * (0.92 + rnd.random() * 0.16)
    return "\n  ".join(f'<path class="{cls}" d="{d}"/>' for d in out)


# ---------------------------------------------------------------- wordmarks

def wordmark(font, size, pad, mark, style, name, folder, extra=""):
    dot = {}
    body, meta = glyph_run(font, "inkling", size, 0, 0, "ink", True, dot)
    asc = meta["ascender"]
    desc = -meta["descender"]
    top = -(asc * 0.80) - pad
    vh = asc * 0.80 + desc + pad * 2
    vw = meta["width"] + pad * 2
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{f(-pad)} {f(top)} {f(vw)} {f(vh)}" width="{f(vw)}" height="{f(vh)}" role="img" aria-label="Inkling">\n'
           f'<style>{style}</style>\n  {body}\n  {mark(dot, meta)}\n  {extra}\n</svg>\n')
    write(OUT / folder / f"{name}.svg", svg)
    return svg, dot, meta


PF_STYLE = (f".ink{{fill:{PF['ink']}}} .pencil{{fill:{PF['pencil']}}}"
            f"@media (prefers-color-scheme:dark){{.ink{{fill:{PF['ink_d']}}} .pencil{{fill:{PF['pencil_d']}}}}}")
CK_STYLE = (f".ink{{fill:{CK['ink']}}} .hi{{fill:{CK['yellow']}}} .red{{fill:{CK['red']}}}"
            f"@media (prefers-color-scheme:dark){{.ink{{fill:{CK['ink_d']}}}}}")
TH_STYLE = (f".ink{{fill:{TH['ink']}}} .madder{{fill:{TH['madder']}}} .madder-hi{{fill:#CF7462}} .madder-lo{{fill:#7E2B22}}"
            f"@media (prefers-color-scheme:dark){{.ink{{fill:{TH['ink_d']}}} .madder{{fill:#C2533F}} .madder-hi{{fill:#E58C78}} .madder-lo{{fill:#8E3427}}}}")


def build_wordmarks():
    pf, pf_dot, _ = wordmark("literata-72-500.ttf", 100, 13,
                             lambda d, m: caret(d["cx"], d["cy"] - 3.0, 0.92), PF_STYLE, "wordmark", "proof")
    ck, ck_dot, _ = wordmark("atkinson-next-700.ttf", 100, 12,
                             lambda d, m: (f'<rect class="hi" x="{f(d["cx"] - d["w"] * 0.6)}" y="{f(d["cy"] - d["w"] * 0.6)}" width="{f(d["w"] * 1.2)}" '
                                           f'height="{f(d["w"] * 1.2)}" rx="1.4" transform="rotate(-7 {f(d["cx"])} {f(d["cy"])})"/>'),
                             CK_STYLE, "wordmark", "check")
    th, th_dot, _ = wordmark("fraunces-144-500-soft.ttf", 100, 14,
                             lambda d, m: knot(d["cx"], d["cy"] + 0.4, d["w"] * 0.68), TH_STYLE, "wordmark", "threads")
    return pf_dot, ck_dot, th_dot


# ---------------------------------------------------------------- icons

ICON_BODY = superellipse(512, 512, 412, 5.0)


def icon_proof(dark=False):
    ink = PF["ink_d"] if dark else PF["ink"]
    paper_top, paper_bot = ("#23272C", "#16191D") if dark else ("#FFFFFF", "#EEEDE7")
    pencil = PF["pencil_d"] if dark else PF["pencil"]
    dot = {}
    size = 640
    # place the dotless i so its stem is centred
    probe = outline("literata-72-500.ttf", "i", size)["glyphs"][0]
    sx = 512 - probe["dot"]["cx"]
    base = 770
    body, _ = glyph_run("literata-72-500.ttf", "i", size, sx, base, "ink", True, dot)
    crop = []
    t0, t1, gap, ln = 236, 788, 22, 62
    for x, y, dx, dy in [(t0, t0, -1, -1), (t1, t0, 1, -1), (t0, t1, -1, 1), (t1, t1, 1, 1)]:
        crop.append(f'<line x1="{x + dx * gap}" y1="{y}" x2="{x + dx * (gap + ln)}" y2="{y}"/>')
        crop.append(f'<line x1="{x}" y1="{y + dy * gap}" x2="{x}" y2="{y + dy * (gap + ln)}"/>')
    reg = (f'<g class="reg"><circle cx="512" cy="166" r="20"/><line x1="472" y1="166" x2="552" y2="166"/>'
           f'<line x1="512" y1="126" x2="512" y2="206"/></g>')
    mark = caret(dot["cx"], dot["cy"] - 18, 6.3, "pencil", seed=4, weight=1.05)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<style>.ink{{fill:{ink}}} .pencil{{fill:{pencil}}} .crop line,.reg *{{stroke:{ink};stroke-width:6;fill:none;opacity:.42;stroke-linecap:butt}}</style>
<defs><linearGradient id="pfp" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{paper_top}"/><stop offset="1" stop-color="{paper_bot}"/></linearGradient>
<clipPath id="pfc"><path d="{ICON_BODY}"/></clipPath></defs>
<path d="{ICON_BODY}" fill="url(#pfp)"/>
<g clip-path="url(#pfc)">
  <g class="crop">{''.join(crop)}</g>
  {reg}
  {body}
  {mark}
</g>
<path d="{ICON_BODY}" fill="none" stroke="#000" stroke-opacity=".08" stroke-width="2"/>
</svg>
'''


def icon_check(dark=False):
    sheet_top, sheet_bot = ("#183056", "#102341") if dark else ("#F1F6EC", "#E2ECDB")
    grid = "#2B4A78" if dark else "#CBDDC6"
    ink = CK["ink_d"] if dark else CK["ink"]
    red = "#F0624F" if dark else CK["red"]
    lines = []
    step = 1024 / 24
    for i in range(1, 24):
        v = i * step
        lines.append(f'<line x1="{f(v)}" y1="0" x2="{f(v)}" y2="1024"/><line x1="0" y1="{f(v)}" x2="1024" y2="{f(v)}"/>')
    # the CHK'D cell, ruled on the grid
    x0, y0, x1, y1 = 4 * step + 2, 9 * step, 17 * step, 19 * step
    label, _ = glyph_run("b612-700.ttf", "CHK'D", 54, x0 + 30, y0 + 70, "lab")
    t = tick((x0 + 120, y0 + 250), (x0 + 250, y1 - 60), (x1 + 140, y0 - 170), 4.2, "red", seed=11)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<style>.lab{{fill:{ink};opacity:.78}} .red{{fill:{red}}} .grid line{{stroke:{grid};stroke-width:2.6}} .cell{{fill:none;stroke:{ink};stroke-width:9}}</style>
<defs><linearGradient id="ckp" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{sheet_top}"/><stop offset="1" stop-color="{sheet_bot}"/></linearGradient>
<clipPath id="ckc"><path d="{ICON_BODY}"/></clipPath></defs>
<path d="{ICON_BODY}" fill="url(#ckp)"/>
<g clip-path="url(#ckc)">
  <g class="grid">{''.join(lines)}</g>
  <rect class="cell" x="{f(x0)}" y="{f(y0)}" width="{f(x1 - x0)}" height="{f(y1 - y0)}"/>
  <line x1="{f(x0)}" y1="{f(y0 + 96)}" x2="{f(x1)}" y2="{f(y0 + 96)}" stroke="{ink}" stroke-width="4" opacity=".5"/>
  {label}
  {t}
</g>
<path d="{ICON_BODY}" fill="none" stroke="#000" stroke-opacity=".08" stroke-width="2"/>
</svg>
'''


def question_path(cx, cy, h):
    """Centre line of a hand-stitched question mark, top of the hook to the
    bottom of the stem (the dot is the knot, drawn separately)."""
    s = h / 300
    p0 = (cx - 92 * s, cy - 78 * s)
    hook = cubic(p0, (cx - 96 * s, cy - 170 * s), (cx + 104 * s, cy - 182 * s), (cx + 96 * s, cy - 82 * s), 60)
    bend = cubic((cx + 96 * s, cy - 82 * s), (cx + 90 * s, cy - 20 * s), (cx + 4 * s, cy - 12 * s), (cx + 2 * s, cy + 48 * s), 50)
    return poly(hook, bend), (cx + 2 * s, cy + 128 * s)


def icon_threads(dark=False):
    body_top, body_bot = ("#2A3566", "#1A2148") if dark else ("#3A4D8C", "#27356A")
    linen = "#E9E0CF" if dark else "#F3ECDF"
    weave = "#CDBFA6"
    cx, cy = 512, 540
    R = 352  # outer edge of the hoop
    fabric_r = 300
    weave_lines = []
    for i in range(-46, 47):
        v = i * 7
        weave_lines.append(f'<line x1="{cx + v}" y1="{cy - fabric_r}" x2="{cx + v}" y2="{cy + fabric_r}"/>'
                           f'<line x1="{cx - fabric_r}" y1="{cy + v}" x2="{cx + fabric_r}" y2="{cy + v}"/>')
    qpts, dotc = question_path(cx, cy - 6, 330)
    stitches = running_stitch(qpts, 38, 17, 20, "madder", seed=5)
    k = knot(dotc[0], dotc[1], 30)
    # a loose end where the stitching began: it hangs free until the question is answered
    q0 = qpts[0]
    tail = ribbon(resample(cubic(q0, (q0[0] - 60, q0[1] + 10), (q0[0] - 150, q0[1] + 120), (q0[0] - 205, q0[1] + 290), 60), 0.5),
                  7, (0.03, 0.5), 0.0, 7)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<style>.madder{{fill:{TH['madder']}}} .madder-hi{{fill:#D27B68}} .madder-lo{{fill:#7A2A21}} .weave line{{stroke:{weave};stroke-width:1.4;opacity:.45}}</style>
<defs>
<linearGradient id="thb" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{body_top}"/><stop offset="1" stop-color="{body_bot}"/></linearGradient>
<linearGradient id="wood" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#EDD3A4"/><stop offset=".55" stop-color="#D7B07A"/><stop offset="1" stop-color="#B98C55"/></linearGradient>
<radialGradient id="cloth" cx=".42" cy=".38" r=".75"><stop offset="0" stop-color="{linen}"/><stop offset="1" stop-color="#E2D6BF"/></radialGradient>
<clipPath id="thc"><path d="{ICON_BODY}"/></clipPath>
<clipPath id="fab"><circle cx="{cx}" cy="{cy}" r="{fabric_r}"/></clipPath>
</defs>
<path d="{ICON_BODY}" fill="url(#thb)"/>
<g clip-path="url(#thc)">
  <ellipse cx="{cx}" cy="{cy + 26}" rx="{R + 8}" ry="{R + 4}" fill="#000" opacity=".22"/>
  <circle cx="{cx}" cy="{cy}" r="{R}" fill="url(#wood)"/>
  <circle cx="{cx}" cy="{cy}" r="{R - 26}" fill="none" stroke="#9C7243" stroke-width="3" opacity=".55"/>
  <circle cx="{cx}" cy="{cy}" r="{fabric_r + 4}" fill="#A47A47"/>
  <circle cx="{cx}" cy="{cy}" r="{fabric_r}" fill="url(#cloth)"/>
  <g class="weave" clip-path="url(#fab)">{''.join(weave_lines)}</g>
  <rect x="{cx - 34}" y="{cy - R - 40}" width="68" height="64" rx="10" fill="#C9A66A" stroke="#8E6A35" stroke-width="4"/>
  <rect x="{cx - 14}" y="{cy - R - 74}" width="28" height="40" rx="6" fill="#B89155" stroke="#8E6A35" stroke-width="4"/>
  <path class="madder" d="{tail}"/>
  {stitches}
  {k}
</g>
<path d="{ICON_BODY}" fill="none" stroke="#000" stroke-opacity=".1" stroke-width="2"/>
</svg>
'''


def main():
    dots = build_wordmarks()
    for name, fn in (("proof", icon_proof), ("check", icon_check), ("threads", icon_threads)):
        write(OUT / name / "icon.svg", fn(False))
        write(OUT / name / "icon-dark.svg", fn(True))
    print("ok")


if __name__ == "__main__":
    main()

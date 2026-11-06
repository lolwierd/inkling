# /// script
# dependencies = ["fonttools", "uharfbuzz"]
# ///
"""Third pass: a coiled knot, a proof-strip icon for Proof, pencil grain, and a
yellow swipe on the Chk'd cell so it stops reading as a to-do checkbox."""
import math
from pathlib import Path
import brand
from brand import caret, glyph_run, ICON_BODY, question_path, running_stitch, tick
from logos import CK, OUT, PF, TH, cubic, f, outline, poly, resample, ribbon, write


def knot(cx, cy, r, base="madder", hi="madder-hi", lo="madder-lo"):
    """French knot as a coil: a bead with one spiral of lighter thread."""
    parts = [f'<circle class="{base}" cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}"/>']
    parts.append(f'<path class="{lo}" d="M{f(cx + r)} {f(cy)} A{f(r)} {f(r)} 0 0 1 {f(cx - r * 0.5)} {f(cy + r * 0.87)} '
                 f'A{f(r * 0.9)} {f(r * 0.9)} 0 0 0 {f(cx + r)} {f(cy)} Z" opacity=".8"/>')
    pts = []
    turns = 1.3
    for i in range(121):
        t = i / 120
        a = math.radians(-120) + t * turns * 2 * math.pi
        rad = r * (0.16 + 0.66 * t)
        pts.append((cx + rad * math.cos(a), cy + rad * math.sin(a) * 0.92))
    d = ribbon(pts, lambda t: r * (0.11 + 0.08 * t), (0.25, 0.3), 0.0, 3)
    parts.append(f'<path class="{hi}" d="{d}"/>')
    return "\n  ".join(parts)


brand.knot = knot

GRAIN = ('<filter id="{id}" x="-20%" y="-20%" width="140%" height="140%">'
         '<feTurbulence type="fractalNoise" baseFrequency="{freq}" numOctaves="2" seed="11" result="n"/>'
         '<feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 -2.4 {keep}" result="a"/>'
         '<feComposite in="SourceGraphic" in2="a" operator="in"/></filter>')


def with_grain(svg, fid, freq, keep, cls):
    """Run every shape of one class through a grain filter (pencil on paper)."""
    svg = svg.replace("<style>", "<defs>" + GRAIN.format(id=fid, freq=freq, keep=keep) + "</defs>\n<style>", 1)
    return svg.replace(f'class="{cls}" d=', f'class="{cls}" filter="url(#{fid})" d=')


def icon_proof_strip(dark=False):
    desk_top, desk_bot = ("#1C1F23", "#121417") if dark else ("#E9E8E2", "#D9D7CF")
    sheet = "#24282D" if dark else "#FFFFFF"
    ink = "#D9D5CB" if dark else "#2A2824"
    pencil = PF["pencil_d"] if dark else PF["pencil"]
    lines = []
    lens = [430, 452, 404, 446, 300, 438, 420, 458, 250]
    y = 268
    for i, L in enumerate(lens):
        if i == 4:
            lines.append(f'<rect x="270" y="{y}" width="{L}" height="22" rx="11"/>')
        else:
            lines.append(f'<rect x="270" y="{y}" width="{L}" height="22" rx="11"/>')
        y += 60
    cy5 = 268 + 4 * 60
    car = caret(270 + 300 + 4, cy5 + 52, 3.2, "pencil", seed=6, weight=1.1)
    # guide line from the caret out into the margin, then the circled query
    gl = ribbon(resample(cubic((592, cy5 + 34), (640, cy5 + 10), (680, cy5 - 40), (712, cy5 - 74), 40), 0.5),
                lambda t: 9 * (1 - 0.3 * t), (0.1, 0.2), 0.4, 8)
    ccx, ccy, cr = 760, cy5 - 132, 64
    ring = []
    for i in range(81):
        t = i / 80
        a = math.radians(115) + t * math.radians(385)
        rr = cr * (1 + 0.07 * t)
        ring.append((ccx + rr * math.cos(a), ccy + rr * 0.9 * math.sin(a)))
    ringd = ribbon(resample(ring, 0.6), 10, (0.12, 0.25), 0.5, 12)
    # a hand-written question mark inside the circle
    q1 = cubic((ccx - 20, ccy - 22), (ccx - 18, ccy - 50), (ccx + 26, ccy - 50), (ccx + 22, ccy - 20), 30)
    q2 = cubic((ccx + 22, ccy - 20), (ccx + 18, ccy - 2), (ccx + 1, ccy - 2), (ccx + 1, ccy + 16), 30)
    qd = ribbon(resample(poly(q1, q2), 0.5), 11, (0.1, 0.15), 0.3, 13)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<style>.pencil{{fill:{pencil}}} .type rect{{fill:{ink}}} .reg *{{stroke:{ink};stroke-width:5;fill:none;opacity:.5}}</style>
<defs><linearGradient id="pfd" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{desk_top}"/><stop offset="1" stop-color="{desk_bot}"/></linearGradient>
<clipPath id="pfc2"><path d="{ICON_BODY}"/></clipPath>
<filter id="pfsh" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="14" stdDeviation="18" flood-opacity="{.5 if dark else .16}"/></filter></defs>
<path d="{ICON_BODY}" fill="url(#pfd)"/>
<g clip-path="url(#pfc2)">
  <g transform="rotate(-4 512 540)">
    <rect x="214" y="168" width="640" height="760" rx="6" fill="{sheet}" filter="url(#pfsh)"/>
    <g class="reg"><circle cx="534" cy="214" r="15"/><line x1="504" y1="214" x2="564" y2="214"/><line x1="534" y1="184" x2="534" y2="244"/></g>
    <g class="type" opacity=".86">{''.join(lines)}</g>
    {car}
    <path class="pencil" d="{gl}"/>
    <path class="pencil" d="{ringd}"/>
    <path class="pencil" d="{qd}"/>
    <circle class="pencil" cx="{ccx + 1}" cy="{ccy + 36}" r="7.5"/>
  </g>
</g>
<path d="{ICON_BODY}" fill="none" stroke="#000" stroke-opacity=".08" stroke-width="2"/>
</svg>
'''


def icon_check(dark=False):
    sheet_top, sheet_bot = ("#183056", "#102341") if dark else ("#F1F6EC", "#E2ECDB")
    grid = "#2B4A78" if dark else "#CBDDC6"
    ink = CK["ink_d"] if dark else CK["ink"]
    red = "#F0624F" if dark else CK["red"]
    step = 1024 / 24
    lines = "".join(f'<line x1="{f(i * step)}" y1="0" x2="{f(i * step)}" y2="1024"/><line x1="0" y1="{f(i * step)}" x2="1024" y2="{f(i * step)}"/>' for i in range(1, 24))
    x0, y0, x1, y1 = 4 * step, 8 * step, 18 * step, 19 * step
    label, _ = glyph_run("b612-700.ttf", "CHK'D", 76, x0 + 34, y0 + 92, "labk")
    swipe = (f'<path class="hi" style="mix-blend-mode:multiply" d="M{f(x0 + 18)} {f(y0 + 34)} C{f(x0 + 120)} {f(y0 + 26)} {f(x0 + 240)} {f(y0 + 30)} {f(x0 + 336)} {f(y0 + 24)} '
             f'L{f(x0 + 330)} {f(y0 + 52)} L{f(x0 + 340)} {f(y0 + 80)} L{f(x0 + 334)} {f(y0 + 110)} C{f(x0 + 230)} {f(y0 + 116)} {f(x0 + 120)} {f(y0 + 112)} {f(x0 + 16)} {f(y0 + 118)} '
             f'L{f(x0 + 24)} {f(y0 + 92)} L{f(x0 + 12)} {f(y0 + 64)} Z" opacity=".9"/>')
    t = tick((x0 + 118, y0 + 300), (x0 + 250, y1 - 70), (x1 + 120, y0 - 150), 4.6, "red", seed=11)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<style>.labk{{fill:#22262A}} .red{{fill:{red}}} .hi{{fill:{CK['yellow']}}} .grid line{{stroke:{grid};stroke-width:2.6}} .cell{{fill:none;stroke:{ink};stroke-width:10}}</style>
<defs><linearGradient id="ckp" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{sheet_top}"/><stop offset="1" stop-color="{sheet_bot}"/></linearGradient>
<clipPath id="ckc"><path d="{ICON_BODY}"/></clipPath></defs>
<path d="{ICON_BODY}" fill="url(#ckp)"/>
<g clip-path="url(#ckc)">
  <g class="grid">{lines}</g>
  {swipe}
  <rect class="cell" x="{f(x0)}" y="{f(y0)}" width="{f(x1 - x0)}" height="{f(y1 - y0)}"/>
  <line x1="{f(x0)}" y1="{f(y0 + 136)}" x2="{f(x1)}" y2="{f(y0 + 136)}" stroke="{ink}" stroke-width="5" opacity=".55"/>
  {label}
  {t}
</g>
<path d="{ICON_BODY}" fill="none" stroke="#000" stroke-opacity=".08" stroke-width="2"/>
</svg>
'''


def icon_threads(dark=False):
    svg = brand.icon_threads(dark)
    return svg


def main():
    pf_dot, ck_dot, th_dot = brand.build_wordmarks()
    # pencil grain on the Proof caret
    p = OUT / "proof" / "wordmark.svg"
    write(p, with_grain(p.read_text(), "pfg", "1.3", "2.35", "pencil"))
    write(OUT / "proof" / "icon.svg", with_grain(brand.icon_proof(False), "pfg", "0.11", "2.4", "pencil"))
    write(OUT / "proof" / "icon-dark.svg", with_grain(brand.icon_proof(True), "pfg", "0.11", "2.4", "pencil"))
    write(OUT / "proof" / "icon-alt.svg", with_grain(icon_proof_strip(False), "pfg", "0.16", "2.45", "pencil"))
    write(OUT / "proof" / "icon-alt-dark.svg", with_grain(icon_proof_strip(True), "pfg", "0.16", "2.45", "pencil"))
    write(OUT / "check" / "icon.svg", with_grain(icon_check(False), "ckg", "0.12", "2.5", "red"))
    write(OUT / "check" / "icon-dark.svg", with_grain(icon_check(True), "ckg", "0.12", "2.5", "red"))
    write(OUT / "threads" / "icon.svg", icon_threads(False))
    write(OUT / "threads" / "icon-dark.svg", icon_threads(True))
    print("ok")


main()

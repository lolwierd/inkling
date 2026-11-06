"""Loose Threads app icon, second pass.

The hoop stays (it is what makes the icon recognisable at Dock size), but the
stitched "?" becomes a stitched lowercase i whose dot is a French knot: the same idea
as the wordmark, so the icon and the name say one thing. The knot's thread runs off to
a needle tucked into the linen: the work is in progress, the thread is loose.

usage: python3 icon_threads.py   (writes ../directions/assets/threads/icon*.svg)
"""
import math
from pathlib import Path

from logos import cubic, f, poly, resample, ribbon, superellipse

OUT = Path(__file__).resolve().parent.parent / "directions" / "assets" / "threads"
BODY = superellipse(512, 512, 412, 5.0)


def knot(cx, cy, r, base, hi, lo):
    """A French knot: a bead of thread with one coil of light on it."""
    parts = [f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(r)}" fill="{base}"/>']
    parts.append(f'<path fill="{lo}" opacity=".85" d="M{f(cx + r)} {f(cy)} A{f(r)} {f(r)} 0 0 1 {f(cx - r * .5)} {f(cy + r * .87)} '
                 f'A{f(r * .9)} {f(r * .9)} 0 0 0 {f(cx + r)} {f(cy)} Z"/>')
    pts = []
    for i in range(121):
        t = i / 120
        a = math.radians(-120) + t * 1.3 * 2 * math.pi
        rad = r * (0.16 + 0.66 * t)
        pts.append((cx + rad * math.cos(a), cy + rad * math.sin(a) * .92))
    parts.append(f'<path fill="{hi}" d="{ribbon(pts, lambda t: r * (0.12 + 0.08 * t), (0.25, 0.3), 0.0, 3)}"/>')
    return "\n  ".join(parts)


def capsule(x0, y0, x1, y1, w, fill, hi=None):
    """One stitch: thread on the surface between two holes, with the light on its twist."""
    pts = resample([(x0 + (x1 - x0) * t / 20, y0 + (y1 - y0) * t / 20) for t in range(21)], 1.0)
    out = f'<path fill="{fill}" d="{ribbon(pts, w, (0.18, 0.18), 0.0, 1)}"/>'
    if hi:
        # twist: short light strokes across the stitch at an angle
        n = max(2, int(math.hypot(x1 - x0, y1 - y0) / (w * 0.9)))
        for k in range(1, n):
            t = k / n
            cx, cy = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            out += (f'<path stroke="{hi}" stroke-width="{f(w * 0.16)}" stroke-linecap="round" opacity=".55" '
                    f'd="M{f(cx - w * 0.28)} {f(cy + w * 0.3)} L{f(cx + w * 0.28)} {f(cy - w * 0.3)}"/>')
    return out


def icon(dark=False):
    body_top, body_bot = ("#2A3566", "#1A2148") if dark else ("#3A4D8C", "#26346A")
    linen, linen_edge = ("#E9E0CF", "#D6C8AE") if dark else ("#F4ECDD", "#E1D4BC")
    weave = "#CDBFA6"
    ink, ink_hi = "#2B2730", "#6A6170"
    madder, madder_hi, madder_lo = "#A63F32", "#D27B68", "#7A2A21"
    cx, cy, R = 512, 548, 344
    fabric_r = R - 40

    weave_lines = "".join(
        f'<line x1="{cx + v}" y1="{cy - fabric_r}" x2="{cx + v}" y2="{cy + fabric_r}"/>'
        f'<line x1="{cx - fabric_r}" y1="{cy + v}" x2="{cx + fabric_r}" y2="{cy + v}"/>'
        for v in range(-fabric_r, fabric_r + 1, 6)
    )

    # a question mark in running stitch: the hook, then the stem
    hook = cubic((420, 468), (414, 352), (612, 336), (612, 452), 90)
    bend = cubic((612, 452), (612, 516), (522, 514), (522, 584), 70)
    stem = cubic((522, 584), (522, 600), (522, 616), (522, 632), 20)
    qline = resample(poly(hook, bend, stem), 1.0)
    stitches = []
    run, on, seg = 0.0, True, [qline[0]]
    lens = [40, 15]
    for (x0, y0), (x1, y1) in zip(qline, qline[1:]):
        run += math.hypot(x1 - x0, y1 - y0)
        seg.append((x1, y1))
        if run >= lens[0 if on else 1]:
            if on:
                d = ribbon(seg, 23, (0.2, 0.2), 0.0, 1)
                stitches.append(f'<path fill="{madder}" d="{d}"/>')
                # the light on the twist of each stitch
                (ax, ay), (bx, by) = seg[len(seg) // 3], seg[2 * len(seg) // 3]
                stitches.append(f'<path stroke="{madder_hi}" stroke-width="3.4" stroke-linecap="round" opacity=".55" d="M{f(ax)} {f(ay)} L{f(bx)} {f(by)}"/>')
            on, run, seg = not on, 0.0, [(x1, y1)]
    # the dot of the question mark is a French knot
    kx, ky, kr = 522, 712, 31
    # the loose end: from where the stitching began, a soft curl down to the parked needle
    needle_eye = (318, 610)
    needle_tip = (372, 742)
    thread = cubic((420, 470), (360, 520), (300, 520), needle_eye, 80)
    thread_d = ribbon(resample(thread, 1.0), 7.5, (0.04, 0.05), 0.0, 5)
    shadow_d = ribbon(resample([(x + 3, y + 6) for x, y in thread], 1.0), 8, (0.04, 0.05), 0.0, 5)
    nx, ny = needle_tip
    ex, ey = needle_eye
    ang = math.atan2(ey - ny, ex - nx)
    L = math.hypot(ex - nx, ey - ny) + 26
    needle = (f'<g transform="translate({nx} {ny}) rotate({f(math.degrees(ang))})">'
              f'<path d="M0 0 L{f(L - 20)} -6 Q{f(L)} -6 {f(L)} 0 Q{f(L)} 6 {f(L - 20)} 6 Z" fill="url(#steel)"/>'
              f'<ellipse cx="{f(L - 14)}" cy="0" rx="9" ry="2.6" fill="{linen}"/>'
              f'</g>')

    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>
<linearGradient id="thb" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{body_top}"/><stop offset="1" stop-color="{body_bot}"/></linearGradient>
<linearGradient id="wood" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#EFD6AA"/><stop offset=".55" stop-color="#D9B27C"/><stop offset="1" stop-color="#BB8E57"/></linearGradient>
<linearGradient id="wood2" x1="1" y1="1" x2="0" y2="0"><stop offset="0" stop-color="#E3C291"/><stop offset="1" stop-color="#C69A62"/></linearGradient>
<radialGradient id="cloth" cx=".44" cy=".38" r=".78"><stop offset="0" stop-color="{linen}"/><stop offset="1" stop-color="{linen_edge}"/></radialGradient>
<linearGradient id="steel" x1="0" y1="-1" x2="0" y2="1" gradientUnits="objectBoundingBox"><stop offset="0" stop-color="#F2F3F5"/><stop offset=".5" stop-color="#B9BCC4"/><stop offset="1" stop-color="#7E828C"/></linearGradient>
<linearGradient id="brass" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#9C7A3E"/><stop offset=".45" stop-color="#E6C77F"/><stop offset="1" stop-color="#8E6B33"/></linearGradient>
<clipPath id="thc"><path d="{BODY}"/></clipPath>
<clipPath id="fab"><circle cx="{cx}" cy="{cy}" r="{fabric_r}"/></clipPath>
</defs>
<path d="{BODY}" fill="url(#thb)"/>
<g clip-path="url(#thc)">
  <ellipse cx="{cx}" cy="{cy + 30}" rx="{R + 6}" ry="{R}" fill="#000" opacity=".24"/>
  <circle cx="{cx}" cy="{cy}" r="{R}" fill="url(#wood)"/>
  <circle cx="{cx}" cy="{cy}" r="{R - 14}" fill="none" stroke="#A47A47" stroke-width="2.5" opacity=".6"/>
  <circle cx="{cx}" cy="{cy}" r="{fabric_r + 10}" fill="url(#wood2)"/>
  <circle cx="{cx}" cy="{cy}" r="{fabric_r + 2}" fill="#9E7443" opacity=".75"/>
  <circle cx="{cx}" cy="{cy}" r="{fabric_r}" fill="url(#cloth)"/>
  <g clip-path="url(#fab)" stroke="{weave}" stroke-width="1.3" opacity=".42">{weave_lines}</g>
  <g clip-path="url(#fab)">
    <path d="{shadow_d}" fill="#000" opacity=".12"/>
    {''.join(stitches)}
    <path d="{thread_d}" fill="{madder}"/>
    {knot(kx, ky, kr, madder, madder_hi, madder_lo)}
    {needle}
  </g>
  <rect x="{cx - 26}" y="{cy - R - 30}" width="52" height="46" rx="9" fill="url(#brass)"/>
  <rect x="{cx - 11}" y="{cy - R - 58}" width="22" height="32" rx="5" fill="url(#brass)"/>
  <rect x="{cx - 26}" y="{cy - R - 16}" width="52" height="4" fill="#000" opacity=".18"/>
</g>
<path d="{BODY}" fill="none" stroke="#000" stroke-opacity=".1" stroke-width="2"/>
</svg>
'''


if __name__ == "__main__":
    (OUT / "icon.svg").write_text(icon(False))
    (OUT / "icon-dark.svg").write_text(icon(True))
    print("ok")

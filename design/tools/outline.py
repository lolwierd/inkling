# /// script
# dependencies = ["fonttools", "uharfbuzz"]
# ///
"""Outline a word in a font as SVG paths, one per glyph, and report where the
dot of each i sits, so a wordmark can swap that dot for its own mark.

usage: outline.py FONT TEXT SIZE [--track PX] [--json]
"""
import json
import sys

import uharfbuzz as hb
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont


def shape(path, text):
    blob = hb.Blob.from_file_path(path)
    face = hb.Face(blob)
    font = hb.Font(face)
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    hb.shape(font, buf, {"kern": True, "liga": True})
    return buf.glyph_infos, buf.glyph_positions, face.upem


def contours(glyphset, name):
    rec = DecomposingRecordingPen(glyphset)
    glyphset[name].draw(rec)
    out, cur = [], []
    for op, args in rec.value:
        cur.append((op, args))
        if op in ("closePath", "endPath"):
            out.append(cur)
            cur = []
    if cur:
        out.append(cur)
    return out


def bounds_of(ops):
    pts = [p for _, args in ops for p in args]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def main():
    path, text, size = sys.argv[1], sys.argv[2], float(sys.argv[3])
    track = 0.0
    if "--track" in sys.argv:
        track = float(sys.argv[sys.argv.index("--track") + 1])
    tt = TTFont(path)
    gs = tt.getGlyphSet()
    order = tt.getGlyphOrder()
    os2 = tt["OS/2"]
    xh = getattr(os2, "sxHeight", 0) or 0
    infos, poss, upem = shape(path, text)
    s = size / upem
    x = 0.0
    glyphs = []
    for info, pos in zip(infos, poss):
        name = order[info.codepoint]
        pen = SVGPathPen(gs, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
        gs[name].draw(TransformPen(pen, (s, 0, 0, -s, x + pos.x_offset * s, -pos.y_offset * s)))
        bp = BoundsPen(gs)
        gs[name].draw(bp)
        b = bp.bounds
        g = {"glyph": name, "x": round(x, 2), "adv": round(pos.x_advance * s, 2), "d": pen.getCommands()}
        if b:
            g["bounds"] = [round(x + b[0] * s, 2), round(-b[3] * s, 2), round(x + b[2] * s, 2), round(-b[1] * s, 2)]
        # The dot of an i: the contour that sits wholly above the x-height.
        if name in ("i",):
            for c in contours(gs, name):
                cb = bounds_of(c)
                if cb[1] > xh * 0.9:
                    g["dot"] = {
                        "cx": round(x + (cb[0] + cb[2]) / 2 * s, 2),
                        "cy": round(-(cb[1] + cb[3]) / 2 * s, 2),
                        "w": round((cb[2] - cb[0]) * s, 2),
                        "h": round((cb[3] - cb[1]) * s, 2),
                    }
                else:
                    g["stem"] = [round(x + cb[0] * s, 2), round(-cb[3] * s, 2), round(x + cb[2] * s, 2), round(-cb[1] * s, 2)]
        glyphs.append(g)
        x += pos.x_advance * s + track
    meta = {
        "upem": upem,
        "xHeight": round(xh * s, 2),
        "capHeight": round((getattr(os2, "sCapHeight", 0) or 0) * s, 2),
        "ascender": round(tt["hhea"].ascent * s, 2),
        "descender": round(tt["hhea"].descent * s, 2),
        "width": round(x - track, 2),
    }
    print(json.dumps({"meta": meta, "glyphs": glyphs}))


main()

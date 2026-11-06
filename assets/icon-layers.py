#!/usr/bin/env python3
"""Split the Initial logo into the layers of `assets/Inkling.icon`, the app icon that
macOS 26 draws in light, dark, clear and tinted.

    python3 assets/icon-layers.py

Reads `design/logo/03-initial/icon.svg` and `icon-dark.svg` and writes, for each
drawing, one SVG per layer into `assets/Inkling.icon/Assets/`, back to front:

    border        the hemstitched border: its two drawn threads and the stitches across
    i-shadow      the satin i's shadow on the linen
    i             the satin-stitched i
    knot-shadow   the French knot's shadow
    knot          the French knot

The elements are the logo's own, unchanged; only the frame moves. The logo's tile is
824 px inside a 1024 px canvas, and an icon layer is the whole 1024 px square, which
the system masks to the icon's shape, so every layer is scaled by 1024/824 about the
tile's corner. The tile itself (the linen, or the dark cloth) is not a layer: it is
the icon's background fill in `icon.json`, which this script checks against the
logo. The tile's hairline edge is dropped; the system draws the icon's edge.

`icon.json` is written by hand (or by Icon Composer): it stacks the layers into
groups, picks the dark files for the dark appearance, and sets the glass and the
mono (clear and tinted) treatment. This script never touches it.
"""

import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LOGO = REPO / "design/logo/03-initial"
ICON = REPO / "assets/Inkling.icon"
SVG = "http://www.w3.org/2000/svg"
ET.register_namespace("", SVG)
FRAME = f"scale({1024 / 824:.7f}) translate(-100 -100)"


def tag(el):
    return el.tag.rsplit("}", 1)[-1]


def layers(drawing):
    """The logo's elements, sorted into layers by where they sit in the drawing:
    the tile path, then a group clipped to the tile holding the border's stitches
    and drawn threads, the i's shadow (the first element with its own transform),
    the i (a group with its clip), the knot's shadow (a circle with an opacity),
    the knot and its shading."""
    root = ET.parse(drawing).getroot()
    kids = list(root)
    tile = next(k for k in kids if tag(k) == "path" and k.get("fill") not in (None, "none"))
    cloth = next(k for k in kids if tag(k) == "g" and k.get("clip-path") == "url(#tile)")
    parts = list(cloth)
    i_at = next(n for n, k in enumerate(parts) if tag(k) == "g")
    shadow_at = i_at - 1
    knot_at = next(n for n, k in enumerate(parts) if tag(k) == "circle" and k.get("opacity") is None)
    out = {
        "border": parts[:shadow_at],
        "i-shadow": [parts[shadow_at]],
        "i": [parts[i_at]],
        "knot-shadow": parts[i_at + 1 : knot_at],
        "knot": parts[knot_at:],
    }
    # What the sorting assumes, so a redrawn logo fails here instead of in the Dock.
    assert parts[shadow_at].get("transform") and parts[shadow_at].get("opacity"), "the i's shadow moved"
    assert parts[i_at].find(f"{{{SVG}}}clipPath") is not None, "the i lost its satin clip"
    assert [tag(k) for k in out["knot-shadow"]] == ["circle"], "the knot's shadow moved"
    assert all(tag(k) == "path" for k in out["border"]) and len(out["border"]) > 100, "the border changed"
    return tile.get("fill"), out


def write(name, elements):
    svg = ET.Element(f"{{{SVG}}}svg", {"viewBox": "0 0 1024 1024", "width": "1024", "height": "1024"})
    frame = ET.SubElement(svg, f"{{{SVG}}}g", {"transform": FRAME})
    frame.extend(elements)
    path = ICON / "Assets" / f"{name}.svg"
    ET.ElementTree(svg).write(path, encoding="unicode", short_empty_elements=True)
    return path


def srgb(hex_color):
    r, g, b = (int(hex_color[i : i + 2], 16) / 255 for i in (1, 3, 5))
    return f"srgb:{r:.5f},{g:.5f},{b:.5f},1.00000"


def main():
    (ICON / "Assets").mkdir(parents=True, exist_ok=True)
    fills = {}
    for drawing, suffix, appearance in ((LOGO / "icon.svg", "", None), (LOGO / "icon-dark.svg", "-dark", "dark")):
        fills[appearance], parts = layers(drawing)
        for name, elements in parts.items():
            print(f"wrote {write(name + suffix, elements).relative_to(REPO)}")
    # The background is icon.json's fill, per appearance; it must be the logo's tile.
    spec = json.loads((ICON / "icon.json").read_text())
    have = {s.get("appearance"): s["value"].get("solid") for s in spec.get("fill-specializations", [])}
    wrong = [f"{a or 'default'}: {have.get(a)} (the logo's tile is {srgb(c)})" for a, c in fills.items() if have.get(a) != srgb(c)]
    if wrong:
        sys.exit("icon.json's background fill is not the logo's tile:\n  " + "\n  ".join(wrong))
    print("icon.json's background fill matches the logo's tile")


if __name__ == "__main__":
    main()

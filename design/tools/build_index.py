"""Inline the three wordmarks into the overview so they follow the page's theme
toggle: strip each SVG's own <style>, prefix its classes, and drive fills from the
page's tokens."""
import re
from pathlib import Path
D = Path("/Users/ayaan/Projects/inkling/design/directions")
SRC = Path(__file__).parent / "index.src.html"  # edit this, then run build_index.py
PRE = {"proof": "pf", "check": "ck", "threads": "th"}

def inline(name):
    p = PRE[name]
    s = (D / "assets" / name / "wordmark.svg").read_text()
    s = re.sub(r"<style>.*?</style>\s*", "", s, flags=re.S)
    s = re.sub(r'class="([a-z-]+)"', lambda m: f'class="{p}-{m.group(1)}"', s)
    s = s.replace('id="pfg"', 'id="pf-wm-grain"').replace("url(#pfg)", "url(#pf-wm-grain)")
    s = re.sub(r'(<svg[^>]*?) width="[^"]*" height="[^"]*"', r"\1", s, count=1)
    s = re.sub(r' role="img" aria-label="Inkling"', ' focusable="false"', s, count=1)
    return s.strip()

html = SRC.read_text()
for name in PRE:
    html = html.replace(f"<!--WM:{name}-->", inline(name))
css = (".wm .pf-ink{fill:var(--pf-ink)} .wm .pf-pencil{fill:var(--pf-pencil)}"
       " .wm .ck-ink{fill:var(--ck-ink)} .wm .ck-hi{fill:var(--ck-yellow)}"
       " .wm .th-ink{fill:var(--th-ink)} .wm .th-madder{fill:var(--th-madder)}"
       " .wm .th-madder-hi{fill:var(--th-madder-hi)} .wm .th-madder-lo{fill:var(--th-madder-lo)}\n</style>")
html = html.replace("</style>", css, 1)
(D / "index.html").write_text(html)
print(len(html), "bytes")

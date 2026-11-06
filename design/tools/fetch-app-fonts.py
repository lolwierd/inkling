# /// script
# dependencies = ["fonttools"]
# ///
"""Fetch the Loose Threads faces as static instances and give each optical size its
own family name, because GPUI matches fonts by family + weight + style and cannot
set variable axes. Writes to assets/fonts/.

  Fraunces Text     opsz 14, SOFT 100, WONK 0   the writer's words
  Fraunces Display  opsz 72, SOFT 100, WONK 0   titles and headings
  Figtree                                        Inkling's voice and the interface
"""
import re
import subprocess
import urllib.request
from pathlib import Path

from fontTools.ttLib import TTFont

OUT = Path(__file__).resolve().parents[2] / "assets" / "fonts"
UA = "curl/8"  # a non-browser agent gets TTF links

def css(family_query):
    req = urllib.request.Request(f"https://fonts.googleapis.com/css2?family={family_query}", headers={"User-Agent": UA})
    return urllib.request.urlopen(req).read().decode()

def fetch(query, out_name, family, style_name, weight, italic):
    url = re.search(r"https://fonts\.gstatic\.com[^)]*", css(query)).group(0)
    data = urllib.request.urlopen(url).read()
    tmp = OUT / (out_name + ".tmp")
    tmp.write_bytes(data)
    f = TTFont(tmp)
    name = f["name"]
    full = f"{family} {style_name}".replace(" Regular", "") if style_name != "Regular" else f"{family}"
    ps = (family + "-" + style_name).replace(" ", "")
    for rec in list(name.names):
        if rec.nameID in (1, 2, 4, 6, 16, 17, 21, 22, 25):
            name.removeNames(nameID=rec.nameID)
    # Legacy family groups only Regular/Italic/Bold/Bold Italic; other weights get their own.
    rib = style_name in ("Regular", "Italic", "Bold", "Bold Italic")
    name.setName(family if rib else f"{family} {style_name.replace(' Italic', '')}", 1, 3, 1, 0x409)
    name.setName(style_name if rib else ("Italic" if italic else "Regular"), 2, 3, 1, 0x409)
    name.setName(f"{family} {style_name}", 4, 3, 1, 0x409)
    name.setName(ps, 6, 3, 1, 0x409)
    name.setName(family, 16, 3, 1, 0x409)
    name.setName(style_name, 17, 3, 1, 0x409)
    f["OS/2"].usWeightClass = weight
    sel = f["OS/2"].fsSelection & ~0b1100001  # clear italic, bold, regular
    if italic: sel |= 0b1
    if weight == 700: sel |= 0b100000
    if not italic and weight == 400: sel |= 0b1000000
    f["OS/2"].fsSelection = sel
    f["head"].macStyle = (0b10 if italic else 0) | (0b1 if weight == 700 else 0)
    dest = OUT / f"{out_name}.ttf"
    f.save(dest)
    tmp.unlink()
    print(dest.name, family, style_name, weight, "italic" if italic else "")

STYLE = {400: "Regular", 500: "Medium", 600: "SemiBold", 700: "Bold"}

for w in (400, 500, 600, 700):
    for it in (0, 1):
        if it and w in (500,):
            continue
        st = STYLE[w] if not it else ("Italic" if w == 400 else STYLE[w] + " Italic")
        fetch(f"Fraunces:ital,opsz,wght,SOFT,WONK@{it},14,{w},100,0", f"FrauncesText-{st.replace(' ', '')}", "Fraunces Text", st, w, bool(it))
for w in (500, 600):
    fetch(f"Fraunces:opsz,wght,SOFT,WONK@72,{w},100,0", f"FrauncesDisplay-{STYLE[w]}", "Fraunces Display", STYLE[w], w, False)
for w in (400, 500, 600, 700):
    fetch(f"Figtree:wght@{w}", f"Figtree-{STYLE[w]}", "Figtree", STYLE[w], w, False)
fetch("Figtree:ital,wght@1,400", "Figtree-Italic", "Figtree", "Italic", 400, True)

for lic, url in (("OFL-Fraunces.txt", "https://raw.githubusercontent.com/google/fonts/main/ofl/fraunces/OFL.txt"),
                 ("OFL-Figtree.txt", "https://raw.githubusercontent.com/google/fonts/main/ofl/figtree/OFL.txt")):
    (OUT / lic).write_bytes(urllib.request.urlopen(url).read())
    print(lic)

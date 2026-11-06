"""Round 2 of the review page: the darn, the dot on the i, Tied off, Sampler, and One
thread in the app. logo_page.py puts this at the top of ../logo/index.html and folds
round 1 below it.

Every entry here can be shortlisted and noted like round 1's, and its marks travel in
the same JSON as round 1's, keyed "r2-<slug>" (and "dot-<slug>" for the dots).
"""
import html
import re

import logo_round2 as R

DOT_NAMES = {k: name for k, name, _, _ in R.DOTS}


def esc(s):
    return html.escape(s, quote=True)


def vb(svg):
    return [float(v) for v in re.search(r'viewBox="([^"]+)"', svg).group(1).split()]


def wm(key, svg, size, extra=""):
    x, y, w, h = vb(svg)
    s = size / 100
    return f'<img data-a="{key}" style="width:{w * s:.1f}px;height:{h * s:.1f}px{extra}" alt="">'


def star(slug, name):
    return (f'<button class="star" type="button" data-slug="{slug}" aria-pressed="false" aria-label="Shortlist {esc(name)}">'
            f'<span class="k" aria-hidden="true"></span><span class="t">Shortlist</span></button>')


def note(slug, name):
    return (f'<label class="sr" for="n-{slug}">Pointers for {esc(name)}</label>'
            f'<textarea id="n-{slug}" data-note="{slug}" placeholder="Pointers: what to keep, change or combine"></textarea>')


def variant_row(d, key, code, group):
    """One variant as a specimen row: what it means and points at above; below, the icon
    light and dark at 128, 64, 32 and 16 pt, the true pixels at 32 and 16, the wordmark
    with its dot, and the mark."""
    name = d["name"]
    tiles = ""
    for cls, k, label in (("on-mac-l", "il", "light"), ("on-mac-d", "id", "dark")):
        sizes = "".join(f'<img data-a="{key}:{k}" width="{s}" height="{s}" alt="{esc(name)} icon at {s} pt, {label}">' for s in (128, 32, 16))
        loupes = "".join(f'<figure class="loupe"><canvas data-loupe="{key}:{k}:{px}" width="{px}" height="{px}" style="width:{px * m}px;height:{px * m}px" '
                         f'aria-label="{esc(name)} at {px} px, {label}, magnified"></canvas><figcaption>{px} px</figcaption></figure>'
                         for px, m in ((32, 2.5), (16, 3)))
        tiles += (f'<button class="v-tile {cls}" type="button" aria-expanded="false" aria-controls="big-{key}" title="Show at 512 pt">'
                  f'{sizes}<span class="v-div" aria-hidden="true"></span>{loupes}</button>')
    big = (f'<div class="v-big" id="big-{key}" hidden><figure class="on-mac-l"><img data-a="{key}:il" width="512" height="512" alt="{esc(name)} icon, light, 1024 px"></figure>'
           f'<figure class="on-mac-d"><img data-a="{key}:id" width="512" height="512" alt="{esc(name)} icon, dark, 1024 px"></figure>'
           f'<span class="lab">At 1024 px, shown at 512 pt: one to one on a Retina screen. Click either tile above to close.</span></div>')
    dot = DOT_NAMES.get(d.get("dot"), "")
    words = (f'<div class="v-words"><div class="v-wm">{wm(key + ":w?", d["wordmark"], 44)}</div>'
             f'<div class="v-wm small">{wm(key + ":w?", d["wordmark"], 19)}{wm(key + ":w?", d["wordmark"], 13)}'
             f'<img data-a="{key}:m?" width="56" height="56" alt="{esc(name)} mark"></div>'
             f'<span class="lab">Wordmark at 44, 19 and 13 px{"; dot: " + esc(dot.lower()) if dot else ""}. Mark at 56.</span></div>')
    return (f'<article class="v" id="v-{key}" data-slug="{key}">'
            f'<div class="v-text"><div class="v-head"><span class="n">{code}</span><h3 class="serif">{esc(name)}</h3>{star(key, group + ": " + name)}</div>'
            f'<div class="v-lines"><p><span class="lab">Means</span><span>{esc(d["means"])}</span></p>'
            f'<p><span class="lab">Points at</span><span>{esc(d["points"])}</span></p>'
            f'<p class="risk"><span class="lab">Risk</span><span>{esc(d["risk"])}</span></p></div>'
            f'<div class="v-note">{note(key, group + ": " + name)}</div></div>'
            f'<div class="v-art">{tiles}{words}</div>{big}</article>')


def dot_rows(pairs):
    rows = []
    for k, name, line, _ in R.DOTS:
        key = f"dot-{k}"
        svg = R.dot_wordmark(k)
        paired = "; ".join(f"{g}: {', '.join(n)}" for g, n in pairs.get(k, {}).items()) or "none yet"
        cells = ""
        for cls, v in (("on-linen", "wl"), ("on-lamp", "wd")):
            cells += f'<div class="dot-cell {cls}">{wm(key + ":" + v, svg, 72)}{wm(key + ":" + v, svg, 19)}{wm(key + ":" + v, svg, 13)}</div>'
        rows.append(f'<article class="dot-row" id="v-{key}" data-slug="{key}"><div class="dot-text"><h3 class="serif">{esc(name)}</h3>'
                    f'<p>{esc(line)}</p><p class="lab">Pairs with: {esc(paired)}</p>{star(key, "Dot: " + name)}'
                    f'<div class="dot-note">{note(key, "Dot: " + name)}</div></div>{cells}</article>')
    return "".join(rows)


def mock(theme):
    """A small note page, as the app would look if it took on the one-thread lettering:
    the name in the sidebar and Inkling's signature on each tag are couched thread;
    everything you write stays Fraunces, and every control stays Figtree."""
    k = "l" if theme == "l" else "d"
    mad = "#A63F32" if theme == "l" else "#D9705E"
    # the running stitch under the words, and the border motifs, drawn inline: no files, no requests
    stitch_svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 11 4"><rect x=".6" y="1.1" width="6.6" height="1.8" rx=".9" fill="{mad}"/></svg>')
    stitch_uri = "data:image/svg+xml," + stitch_svg.replace("<", "%3C").replace(">", "%3E").replace("#", "%23").replace('"', "'")
    motifs = "".join(R.motif(10 + 22 * k, 7, 3.4, 1.75, "currentColor") for k in range(5))
    return f'''<div class="mock {theme}" role="img" aria-label="The note page with one-thread lettering, {"light" if theme == "l" else "dark"}">
  <div class="mk-side">
    <span class="lights"><i></i><i></i><i></i></span>
    <img class="mk-wm" data-a="12-one-thread:w{k}" alt="">
    <span class="mk-cap">Running</span>
    <span class="mk-row on"><span>Sub-50 10k by March</span><b>2</b></span>
    <span class="mk-row"><span>Shoes: what wore out where</span></span>
    <span class="mk-cap">Home</span>
    <span class="mk-row"><span>Why the bread didn't rise</span></span>
    <span class="mk-row"><span>Kitchen shelf measurements</span></span>
  </div>
  <div class="mk-page">
    <svg class="mk-border" viewBox="0 0 120 14" aria-hidden="true">{motifs}</svg>
    <h4 class="mk-title">Sub-50 10k by March</h4>
    <p class="mk-p">Last year I ran the city 10k in 56:40. I went out too fast and walked twice after the 7 km mark.</p>
    <p class="mk-p">Sunday is the long run. I'll <span class="mk-st" style="background-image:url(&quot;{stitch_uri}&quot;)">add a kilometre every week until I reach 16</span>.</p>
    <p class="mk-p">Strength work twice a week, mostly calves and hips.</p>
    <div class="mk-tag">
      <div class="mk-tag-head"><img class="mk-sig" data-a="12-one-thread:w{k}" alt="inkling"><span>question</span></div>
      <p>Starting from how far? If you start at 8 km, 16 km is eight weeks out.</p>
    </div>
    <div class="mk-chip"><img class="mk-sig" data-a="12-one-thread:w{k}" alt="inkling"><span>Tied off</span></div>
    <svg class="mk-rope" aria-hidden="true"><path/></svg>
  </div>
</div>'''


def build(data, names):
    """Round 2's HTML, with its marks added to the page's data and its entries to the
    shortlist's names (in page order, ahead of round 1)."""
    darns = [fn() for fn in R.DARNS]
    tied = [fn() for fn in R.TIED]
    samplers = [fn() for fn in R.SAMPLERS]
    R.write_files(darns + tied + samplers)
    pairs = {}
    for group, items in (("Darn", darns), ("Tied off", tied), ("Sampler", samplers)):
        for d in items:
            key = f"r2-{d['slug']}"
            data[key] = {"w": d["wordmark"], "m": d["mark"], "il": d["icon"], "id": d["icon_dark"]}
            names[key] = f"{group}: {d['name']}"
            if d.get("dot"):
                pairs.setdefault(d["dot"], {}).setdefault(group, []).append(d["name"])
    for k, name, _, _ in R.DOTS:
        svg = R.dot_wordmark(k)
        data[f"dot-{k}"] = {"w": svg}
        names[f"dot-{k}"] = f"Dot: {name}"
    names["r2-thread-app"] = "One thread in the app"

    dock = ['<button class="dk" type="button" data-slug="now" aria-label="Today\'s icon"><img data-a="now:i?" alt="">'
            '<span class="tipname">Today</span><span class="run"></span></button>']
    for group, items in (("Darn", darns), ("Tied off", tied), ("Sampler", samplers)):
        dock.append('<span class="dk sep" aria-hidden="true"></span>')
        for d in items:
            key = f"r2-{d['slug']}"
            dock.append(f'<button class="dk" type="button" data-slug="{key}" aria-label="{esc(names[key])}">'
                        f'<img data-a="{key}:i?" alt=""><span class="tipname">{esc(names[key])}</span></button>')

    rows_darn = "".join(variant_row(d, f"r2-{d['slug']}", f"D{n + 1}", "Darn") for n, d in enumerate(darns))
    rows_tied = "".join(variant_row(d, f"r2-{d['slug']}", f"T{n + 1}", "Tied off") for n, d in enumerate(tied))
    rows_samp = "".join(variant_row(d, f"r2-{d['slug']}", f"S{n + 1}", "Sampler") for n, d in enumerate(samplers))

    return f'''
  <section class="lead wrap" id="round-2" aria-labelledby="r2-h">
    <p class="kicker">Loose Threads · logo directions · round 2 · 6 November 2026</p>
    <h1 class="serif" id="r2-h">Round 2: what the darn means, and where each mark points</h1>
    <div class="dock-sec" role="group" aria-label="Every round 2 icon in one Dock">
      <div class="desk">
        <div class="desk-note"><span>Round 2 in one Dock at 64 pt: today's, then the darns, Tied off and Sampler. Point along it to magnify; click one to jump to it.</span><span class="dock-theme"></span></div>
        <div class="dock mag" id="dock2">{"".join(dock)}</div>
      </div>
    </div>
    <div class="why r2-why">
      <div>
        <h2>The review asked for</h2>
        <ul>
          <li data-n="1"><span><b>The darn, hashed out.</b> What it means, and five darns that each point at something by where the darn sits.</span></li>
          <li data-n="2"><span><b>Another dot for the i.</b> The French knot is gone from every new wordmark; six answers side by side.</span></li>
          <li data-n="3"><span><b>A knot that means something.</b> Three Tied off versions: at the word, at the margin, at the i.</span></li>
          <li data-n="4"><span><b>The sampler in this aesthetic.</b> Madder on linen in the app's capsule stitches, two ways, no pixel grid.</span></li>
          <li data-n="5"><span><b>One thread's cost.</b> The note page with its lettering where it would appear, light and dark.</span></li>
        </ul>
      </div>
      <div>
        <h2>What a darn means in Inkling</h2>
        <p class="r2-mean">A darn is made of two threads. One is laid across the gap first, the warp; a second is woven through it, over and under, the weft. In Inkling the warp is madder, <b>Inkling's question laid across the place your words don't hold</b>, and the weft is ink, <b>your answer woven through it</b>. Neither holds alone; together they close the gap, and the mend stays visible on purpose, the way visible mending is meant to: a repair proud of itself, not hidden.</p>
      </div>
    </div>
  </section>

  <section class="sec wrap r2" id="r2-darn" aria-labelledby="r2-darn-h">
    <div class="sec-head"><h2 id="r2-darn-h" class="serif">The darn, five ways</h2>
      <p>Each darn points at something by where it sits, never by a caption: a word, the gap still open, the place a plan leans on, the margin, the record. Each is paired with the dot that suits it, and tested at 16 and 32 px: the magnified squares are the true pixels.</p></div>
    {rows_darn}
  </section>

  <section class="sec wrap r2" id="r2-dot" aria-labelledby="r2-dot-h">
    <div class="sec-head"><h2 id="r2-dot-h" class="serif">The dot on the i</h2>
      <p>Six answers in place of the French knot, each in the wordmark at 72, 19 and 13 px, on linen and on lamplight. The second i keeps the font's dot in every one, so the first reads as a choice.</p></div>
    {dot_rows(pairs)}
  </section>

  <section class="sec wrap r2" id="r2-tied" aria-labelledby="r2-tied-h">
    <div class="sec-head"><h2 id="r2-tied-h" class="serif">Tied off, pointing at something</h2>
      <p>The overhand knot from round 1, now tied where it means something: under the word it settled, at the margin where the question was, round the i like a reminder round a finger.</p></div>
    {rows_tied}
  </section>

  <section class="sec wrap r2" id="r2-sampler" aria-labelledby="r2-sampler-h">
    <div class="sec-head"><h2 id="r2-sampler-h" class="serif">Sampler, in the app's stitches</h2>
      <p>Flat madder on linen, the app's capsule stitch, letters drawn along a centre line the way a stitcher follows one. No pixel grid.</p></div>
    {rows_samp}
  </section>

  <section class="sec wrap r2" id="r2-thread" aria-labelledby="r2-thread-h" data-slug="r2-thread-app">
    <div class="sec-head"><h2 id="r2-thread-h" class="serif">One thread, in the app</h2>
      <p>If the name is written in one couched thread, the lettering has to live somewhere besides the icon, or the app and its mark disagree. These mocks put it in two places: the name in the sidebar, and Inkling's signature on every tag and tied-off chip. Everything you write stays Fraunces; every control stays Figtree.</p></div>
    <div class="mocks">{mock("l")}{mock("d")}</div>
    <div class="mock-foot">
      <ul class="cost">
        <li><b>What changes.</b> A second hand in the interface: the sidebar name, and a signature on each question and each tied-off chip, so Inkling's own voice is written in thread.</li>
        <li><b>What it costs.</b> The script is drawn, not typeset, so every new place it appears is a drawing to make; it cannot be used for running text, and it doubles the app's typefaces in feel if it spreads.</li>
        <li><b>What stays.</b> Your words, the questions themselves, labels and buttons: Fraunces and Figtree, as now.</li>
      </ul>
      <div class="pick">{star("r2-thread-app", "One thread in the app")}{note("r2-thread-app", "One thread in the app")}</div>
    </div>
  </section>

  <section class="sec wrap r2" id="r2-initial" aria-labelledby="r2-initial-h">
    <div class="sec-head"><h2 id="r2-initial-h" class="serif">Initial: retired</h2>
      <p>No deeper version turned up. A satin i can only get more ornate, not more meaningful, and every richer i (darned, tied, laced) is really the darn or Tied off wearing a letter, with the info-symbol problem still attached. Its best idea, a mark made the way linen was marked as someone's own, lives on in the darn.</p></div>
  </section>
'''

/* Proof: the marks Inkling makes, as code.
   One source for the page (proof.html) and for the standalone element files
   (assets/proof/elements/*.svg, written by gen-elements.js with Bun).
   Every hand-drawn mark is a tapered ribbon around a centre line, so it swells and
   thins like a pencil. The centre line is kept so a page can draw the mark on.
   Wobble is seeded (by thread id in the app), so a query's circle is the same circle
   every time the note opens; it never jitters between renders. */
(function (g) {
  "use strict";

  // ---------- numbers ----------
  function rng(seed) {
    let s = (Math.imul(seed | 0 || 1, 2654435761) >>> 0) || 1;
    return function () {
      s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0;
      return s / 4294967296;
    };
  }
  const f = (v) => String(Math.round(v * 10) / 10);

  // ---------- geometry ----------
  function cubic(p0, p1, p2, p3, n) {
    n = n || 32;
    const out = [];
    for (let i = 0; i <= n; i++) {
      const t = i / n, a = (1 - t) ** 3, b = 3 * (1 - t) ** 2 * t, c = 3 * (1 - t) * t * t, d = t ** 3;
      out.push([a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]]);
    }
    return out;
  }
  function join() {
    const out = [];
    for (const seg of arguments) for (let i = out.length ? 1 : 0; i < seg.length; i++) out.push(seg[i]);
    return out;
  }
  function resample(pts, step) {
    const out = [pts[0]];
    let carry = 0;
    for (let i = 1; i < pts.length; i++) {
      const [x0, y0] = pts[i - 1], [x1, y1] = pts[i];
      const len = Math.hypot(x1 - x0, y1 - y0);
      if (!len) continue;
      let t = step - carry;
      while (t <= len) { out.push([x0 + (x1 - x0) * t / len, y0 + (y1 - y0) * t / len]); t += step; }
      carry = len - (t - step);
    }
    const last = pts[pts.length - 1], tail = out[out.length - 1];
    if (tail[0] !== last[0] || tail[1] !== last[1]) out.push(last);
    return out;
  }

  /** A filled outline around a centre line: width(t) swells and the ends taper. */
  function ribbon(pts, width, taper, wobble, seed) {
    taper = taper || [0.2, 0.2]; wobble = wobble || 0;
    const r = rng(seed || 1), n = pts.length, L = [], R = [];
    for (let i = 0; i < n; i++) {
      const a = pts[Math.max(i - 1, 0)], b = pts[Math.min(i + 1, n - 1)];
      let dx = b[0] - a[0], dy = b[1] - a[1];
      const ln = Math.hypot(dx, dy) || 1; dx /= ln; dy /= ln;
      const nx = -dy, ny = dx, t = n > 1 ? i / (n - 1) : 0;
      let w = typeof width === "function" ? width(t) : width;
      if (t < taper[0]) w *= 0.35 + 0.65 * Math.sin((t / taper[0]) * Math.PI / 2);
      if (t > 1 - taper[1]) w *= 0.35 + 0.65 * Math.sin(((1 - t) / taper[1]) * Math.PI / 2);
      const j = (r() - 0.5) * wobble;
      L.push([pts[i][0] + nx * (w / 2 + j), pts[i][1] + ny * (w / 2 + j)]);
      R.push([pts[i][0] - nx * (w / 2 - j), pts[i][1] - ny * (w / 2 - j)]);
    }
    const ring = L.concat(R.reverse());
    return "M" + ring.map((p) => f(p[0]) + " " + f(p[1])).join("L") + "Z";
  }
  const line = (pts) => "M" + pts.map((p) => f(p[0]) + " " + f(p[1])).join("L");

  /** A mark: its fill outline (d), its centre line (c, for drawing on) and its widest width. */
  function mark(pts, width, taper, wobble, seed) {
    const w = typeof width === "function" ? Math.max(width(0), width(0.5), width(1)) : width;
    return { d: ribbon(pts, width, taper, wobble, seed), c: line(pts), w: w };
  }

  // ---------- the marks ----------
  /** The caret, below the baseline, pointing up at x. Drawn in one motion, the right leg
   *  a little longer, the way a hand makes it. Returns the mark plus its right foot. */
  function caret(x, baseline, s, seed) {
    s = s || 1;
    const r = rng(seed || 3);
    const left = [x - 4.6 * s, baseline + 8.2 * s + r() * 0.6 * s];
    const apex = [x + 0.3 * s, baseline + 1.6 * s];
    const right = [x + 5.1 * s, baseline + 8.8 * s];
    const up = cubic(left, [left[0] + 1.2 * s, left[1] - 3 * s], [apex[0] - 1 * s, apex[1] + 2.2 * s], apex, 14);
    const down = cubic(apex, [apex[0] + 1 * s, apex[1] + 2.2 * s], [right[0] - 1.2 * s, right[1] - 3.2 * s], right, 14);
    const m = mark(resample(join(up, down), 0.35 * s), (t) => 1.55 * s * (0.9 + 0.2 * Math.sin(t * Math.PI)), [0.12, 0.18], 0.08 * s, seed || 3);
    m.foot = right;
    return m;
  }

  /** The guide line: leaves the caret along the gap under the line of type (so it never
   *  crosses letters), then turns up into the margin to meet the query. */
  function guide(from, to, seed, runTo) {
    const r = rng(seed || 5);
    const dx = to[0] - from[0];
    // run along the gap to the end of the text column (runTo), then turn into the margin
    const rx = runTo != null ? Math.max(from[0] + 8, Math.min(runTo, to[0] - 14)) : from[0] + Math.max(10, dx * 0.62);
    const run = [rx, from[1] + 0.6];
    const span = Math.max(8, to[0] - run[0]);
    const pts = join(
      cubic(from, [from[0] + 6, from[1] + 1.2], [run[0] - Math.min(20, (run[0] - from[0]) / 2), run[1] + (r() - 0.5)], run, 18),
      cubic(run, [run[0] + span * 0.55, run[1]], [to[0] - span * 0.6, to[1] + (from[1] - to[1]) * 0.12], to, 22)
    );
    return mark(resample(pts, 1), (t) => 1.15 * (1 - 0.25 * t), [0.04, 0.22], 0.1, seed || 5);
  }

  /** A pencil loop around a box, like a copy editor circling a query: a rounded
   *  rectangle walked once, starting in the top-left corner, that wanders a little and
   *  drifts outward as it passes its start, so the ends overlap instead of meeting. */
  function loop(x, y, w, h, seed, opts) {
    opts = opts || {};
    const r = rng(seed || 7), pad = opts.pad == null ? 7 : opts.pad;
    const X0 = x - pad, Y0 = y - pad, W = w + pad * 2, H = h + pad * 2;
    const rc = Math.min(opts.radius || 14, W / 2, H / 2);
    const q = Math.PI * rc / 2, segs = [W - 2 * rc, q, H - 2 * rc, q, W - 2 * rc, q, H - 2 * rc, q];
    const P = segs.reduce((a, b) => a + b, 0);
    function at(s) {
      s = ((s % P) + P) % P;
      let a;
      if (s < segs[0]) return [X0 + rc + s, Y0, 0, -1]; s -= segs[0];
      if (s < segs[1]) { a = -Math.PI / 2 + s / rc; return [X0 + W - rc + rc * Math.cos(a), Y0 + rc + rc * Math.sin(a), Math.cos(a), Math.sin(a)]; } s -= segs[1];
      if (s < segs[2]) return [X0 + W, Y0 + rc + s, 1, 0]; s -= segs[2];
      if (s < segs[3]) { a = s / rc; return [X0 + W - rc + rc * Math.cos(a), Y0 + H - rc + rc * Math.sin(a), Math.cos(a), Math.sin(a)]; } s -= segs[3];
      if (s < segs[4]) return [X0 + W - rc - s, Y0 + H, 0, 1]; s -= segs[4];
      if (s < segs[5]) { a = Math.PI / 2 + s / rc; return [X0 + rc + rc * Math.cos(a), Y0 + H - rc + rc * Math.sin(a), Math.cos(a), Math.sin(a)]; } s -= segs[5];
      if (s < segs[6]) return [X0, Y0 + H - rc - s, -1, 0]; s -= segs[6];
      a = Math.PI + s / rc; return [X0 + rc + rc * Math.cos(a), Y0 + rc + rc * Math.sin(a), Math.cos(a), Math.sin(a)];
    }
    const start = P - q * (0.25 + r() * 0.35), turn = 1.05 + r() * 0.04;
    const p1 = r() * 6.28, p2 = r() * 6.28, steps = Math.max(80, Math.round(P * turn / 2.4)), pts = [];
    for (let i = 0; i <= steps; i++) {
      const t = i / steps, s = start + t * P * turn, pt = at(s), th = (s / P) * 2 * Math.PI;
      const off = 0.9 * Math.sin(3 * th + p1) + 0.55 * Math.sin(5 * th + p2) + (t > 0.78 ? (t - 0.78) * 13 : 0);
      pts.push([pt[0] + pt[2] * off, pt[1] + pt[3] * off - t * 0.6]);
    }
    return mark(resample(pts, 1.2), (t) => (opts.width || 1.35) * (0.85 + 0.3 * Math.sin(t * Math.PI)), [0.06, 0.16], 0.12, seed || 7);
  }

  /** A small loop round a paragraph number. */
  function ring(cx, cy, rad, seed) {
    const r = rng(seed || 11), pts = [], start = (200 + r() * 30) * Math.PI / 180;
    for (let i = 0; i <= 48; i++) {
      const t = i / 48, th = start + t * 2.12 * Math.PI, k = 1 + 0.06 * Math.sin(3 * th + r()) + (t > 0.8 ? (t - 0.8) * 0.5 : 0);
      pts.push([cx + rad * 1.12 * k * Math.cos(th), cy + rad * k * Math.sin(th)]);
    }
    return mark(resample(pts, 0.5), 1.15, [0.08, 0.2], 0.05, seed || 11);
  }

  /** A tick: short stroke down, long stroke up and away. s = 1 is about 10 px wide. */
  function tick(x, y, s, seed) {
    s = s || 1;
    const a = [x, y + 3.2 * s], b = [x + 3.2 * s, y + 6.6 * s], c = [x + 10.5 * s, y - 2.6 * s];
    const pts = join(cubic(a, [a[0] + 1, a[1] + 1], [b[0] - 1.2 * s, b[1] - 0.6 * s], b, 10),
      cubic(b, [b[0] + 1.8 * s, b[1] - 3.5 * s], [c[0] - 4 * s, c[1] + 4 * s], c, 18));
    return mark(resample(pts, 0.35 * s), (t) => 1.7 * s * (1.15 - 0.45 * t), [0.08, 0.3], 0.06 * s, seed || 13);
  }

  /** Stet dots: a row of dots under the words that should stand. */
  function stetDots(x0, x1, y, seed) {
    const r = rng(seed || 17), dots = [];
    for (let x = x0 + 2; x <= x1 - 1; x += 6.2) dots.push([x + (r() - 0.5) * 0.8, y + (r() - 0.5) * 0.6, 1.05 + r() * 0.25]);
    return dots;
  }

  /** A bracket over a span, for figures: ⌐ ¬ joined by a line. */
  function bracket(x0, x1, y, h, seed) {
    const pts = join(cubic([x0, y + h], [x0, y + h * 0.4], [x0 + 1, y], [x0 + 4, y], 8),
      cubic([x0 + 4, y], [x0 + (x1 - x0) * 0.4, y - 0.6], [x1 - (x1 - x0) * 0.4, y + 0.6], [x1 - 4, y], 24),
      cubic([x1 - 4, y], [x1 - 1, y], [x1, y + h * 0.4], [x1, y + h], 8));
    return mark(resample(pts, 0.8), 1.2, [0.05, 0.05], 0.08, seed || 19);
  }

  /** A short arrow, for figures. */
  function arrow(x0, y0, x1, y1, seed) {
    const shaft = mark(resample(cubic([x0, y0], [x0 + (x1 - x0) * 0.3, y0 + (y1 - y0) * 0.3 - 1], [x0 + (x1 - x0) * 0.7, y0 + (y1 - y0) * 0.7 + 1], [x1, y1], 16), 0.6), 1.1, [0.05, 0.1], 0.05, seed || 23);
    const ang = Math.atan2(y1 - y0, x1 - x0), k = 4.2;
    const h1 = [x1 - k * Math.cos(ang - 0.5), y1 - k * Math.sin(ang - 0.5)], h2 = [x1 - k * Math.cos(ang + 0.5), y1 - k * Math.sin(ang + 0.5)];
    const head = mark(join([h1, [x1, y1]], [[x1, y1], h2]), 1.1, [0.15, 0.15], 0, (seed || 23) + 1);
    return [shaft, head];
  }

  // ---------- the wordmark (outlined from Literata 500, caret from brand.py) ----------
  const WORDMARK = [{"c":"ink","d":"M3.8 0V-1.2L8.8 -1.7Q10.3 -1.9 10.8 -2.9Q11.3 -3.9 11.3 -6.5V-43.5Q11.3 -47 10.7 -48.25Q10.1 -49.5 7.9 -49.6L3.2 -50L3.3 -51.1L22 -52L22.7 -51.5V-6.1Q22.7 -3.8 23.2 -2.9Q23.7 -2 25.3 -1.8L30.5 -1.1V0Z"},{"c":"ink","d":"M37.3 0V-1.2L41.8 -1.7Q43.4 -1.9 43.9 -2.9Q44.4 -3.9 44.4 -6.5V-43.5Q44.4 -46.9 43.8 -48.2Q43.2 -49.5 40.9 -49.6L36.3 -50L36.4 -51.1L54.9 -52L55.5 -51.5L55.4 -41.5H55.5Q57.7 -47.2 62.3 -50.05Q66.9 -52.9 72.4 -52.9Q77.6 -52.9 81.05 -50.9Q84.5 -48.9 86.35 -45Q88.2 -41.1 88.2 -35.4V-5.8Q88.2 -3.5 88.8 -2.7Q89.4 -1.9 91.2 -1.7L95.6 -1.2V0H69.9V-1.1L73.9 -1.6Q75.7 -1.8 76.25 -2.7Q76.8 -3.6 76.8 -5.8V-36Q76.8 -40.8 76 -43.6Q75.2 -46.4 73.3 -47.55Q71.4 -48.7 67.9 -48.7Q64.5 -48.7 61.7 -47.05Q58.9 -45.4 57.35 -42.35Q55.8 -39.3 55.8 -35.3V-6.1Q55.8 -3.9 56.25 -2.95Q56.7 -2 58.2 -1.8L63 -1.1V0Z"},{"c":"ink","d":"M147.6 1Q145.2 1 143.3 0.2Q141.4 -0.6 139.65 -2.3Q137.9 -4 136 -6.8L121.5 -27.6V-28.3L140.2 -45.3Q141.7 -46.6 141.75 -47.8Q141.8 -49 139.7 -49.4L136.5 -50.2L136.7 -51.5H153.6V-50.2L150.3 -49.2Q148.9 -48.8 147.45 -48.15Q146 -47.5 143.5 -45.3L130.3 -33.6L146.8 -10.3Q149.3 -6.7 151 -5.35Q152.7 -4 154.5 -4Q155.6 -4 156.8 -4.3Q158 -4.6 159.1 -5.2L159.6 -4.3Q157.4 -2.1 154.15 -0.55Q150.9 1 147.6 1ZM101.9 0V-1.2L106 -1.7Q107.9 -1.8 108.45 -2.85Q109 -3.9 109 -6.6V-68.8Q109 -71.8 108.5 -73.1Q108 -74.4 105.5 -74.6L100.9 -75L101.1 -76L119.7 -76.6L120.4 -76.1V-6.3Q120.4 -4.3 120.8 -3.1Q121.2 -1.9 123.1 -1.7L126.4 -1.3V0Z"},{"c":"ink","d":"M161.8 0V-1.2L165.9 -1.7Q167.8 -1.8 168.35 -2.85Q168.9 -3.9 168.9 -6.6V-68.8Q168.9 -72.1 168.25 -73.3Q167.6 -74.5 165.4 -74.6L160.8 -75L160.9 -76L179.6 -76.6L180.3 -76.1V-6.3Q180.3 -3.9 180.85 -2.9Q181.4 -1.9 183.4 -1.7L188.1 -1.1V0Z"},{"c":"ink","d":"M194.6 0V-1.2L199.6 -1.7Q201.2 -1.9 201.65 -2.95Q202.1 -4 202.1 -6.5V-43.5Q202.1 -46.9 201.5 -48.2Q200.9 -49.5 198.7 -49.6L194 -50L194.1 -51.1L212.8 -52L213.5 -51.5V-6.1Q213.5 -3.8 214 -2.9Q214.5 -2 216.1 -1.8L221.3 -1.1V0ZM206.5 -61.8Q203.2 -61.8 201.1 -63.75Q199 -65.7 199 -68.8Q199 -71.9 201.1 -73.9Q203.2 -75.9 206.5 -75.9Q210.1 -75.9 212.1 -73.95Q214.1 -72 214.1 -68.9Q214.1 -65.7 212.05 -63.75Q210 -61.8 206.5 -61.8Z"},{"c":"ink","d":"M228.1 0V-1.2L232.6 -1.7Q234.2 -1.9 234.7 -2.9Q235.2 -3.9 235.2 -6.5V-43.5Q235.2 -46.9 234.6 -48.2Q234 -49.5 231.7 -49.6L227.1 -50L227.2 -51.1L245.7 -52L246.3 -51.5L246.2 -41.5H246.3Q248.5 -47.2 253.1 -50.05Q257.7 -52.9 263.2 -52.9Q268.4 -52.9 271.85 -50.9Q275.3 -48.9 277.15 -45Q279 -41.1 279 -35.4V-5.8Q279 -3.5 279.6 -2.7Q280.2 -1.9 282 -1.7L286.4 -1.2V0H260.7V-1.1L264.7 -1.6Q266.5 -1.8 267.05 -2.7Q267.6 -3.6 267.6 -5.8V-36Q267.6 -40.8 266.8 -43.6Q266 -46.4 264.1 -47.55Q262.2 -48.7 258.7 -48.7Q255.3 -48.7 252.5 -47.05Q249.7 -45.4 248.15 -42.35Q246.6 -39.3 246.6 -35.3V-6.1Q246.6 -3.9 247.05 -2.95Q247.5 -2 249 -1.8L253.8 -1.1V0Z"},{"c":"ink","d":"M315.2 23Q308.9 23 304.4 22.2Q299.9 21.4 297.05 19.85Q294.2 18.3 292.8 16.05Q291.4 13.8 291.4 10.9Q291.4 8.3 292.6 6.15Q293.8 4 296.3 2.55Q298.8 1.1 302.5 0.5V0Q298.2 -0.8 296.2 -3.15Q294.2 -5.5 294.2 -8.6Q294.2 -11.5 295.75 -13.95Q297.3 -16.4 301.2 -20.4V-20.8Q297.7 -22.6 295.7 -25.95Q293.7 -29.3 293.7 -34.6Q293.7 -40.3 296.4 -44.35Q299.1 -48.4 304.05 -50.65Q309 -52.9 315.5 -52.9Q318.2 -52.9 320.75 -52.4Q323.3 -51.9 325.7 -51Q328.1 -50.1 330 -48.9Q331.5 -53.9 333.65 -56.95Q335.8 -60 339.5 -60Q341.5 -60 343.1 -59.2Q344.7 -58.4 345.65 -57.15Q346.6 -55.9 346.6 -54.1Q346.6 -51.7 344.85 -49.9Q343.1 -48.1 340.5 -48.1Q339 -48.1 337.95 -48.9Q336.9 -49.7 336.1 -51.1L334.2 -54.1Q333.4 -52.7 332.8 -51.25Q332.2 -49.8 331.6 -47.8Q333.8 -45.5 335.35 -42.1Q336.9 -38.7 336.9 -35.2Q336.9 -29.8 334.3 -25.75Q331.7 -21.7 327 -19.5Q322.3 -17.3 315.7 -17.3Q311.7 -17.3 308.6 -17.95Q305.5 -18.6 303.1 -19.7Q301.4 -17.7 300.6 -16.4Q299.8 -15.1 299.8 -13.8Q299.8 -12.1 300.7 -11Q301.6 -9.9 303.5 -9.35Q305.4 -8.8 308.3 -8.8H323.2Q333.1 -8.8 338.15 -5.75Q343.2 -2.7 343.2 4.2Q343.2 7.4 342.1 10.25Q341 13.1 338.85 15.45Q336.7 17.8 333.4 19.5Q330.1 21.2 325.5 22.1Q320.9 23 315.2 23ZM315.9 21.1Q320.9 21.1 324.7 19.5Q328.5 17.9 330.65 15.15Q332.8 12.4 332.8 8.9Q332.8 4.5 330.1 2.55Q327.4 0.6 322.4 0.6H304.9Q303 2.1 301.95 4.5Q300.9 6.9 300.9 9.2Q300.9 15.3 304.65 18.2Q308.4 21.1 315.9 21.1ZM315.4 -19.2Q320.3 -19.2 322.85 -23.25Q325.4 -27.3 325.4 -34.7Q325.4 -42.7 322.65 -46.8Q319.9 -50.9 314.8 -50.9Q310 -50.9 307.6 -46.95Q305.2 -43 305.2 -35.2Q305.2 -27.6 307.95 -23.4Q310.7 -19.2 315.4 -19.2Z"},{"c":"pencil","d":"M6.2 -63.6 L6.6 -63.9 L7 -64.2 L7.4 -64.5 L7.7 -64.8 L8.1 -65 L8.5 -65.3 L8.8 -65.6 L9.1 -66 L9.3 -66.4 L9.6 -66.7 L9.8 -67.1 L10 -67.5 L10.3 -67.8 L10.6 -68.2 L10.9 -68.5 L11.2 -68.9 L11.4 -69.2 L11.7 -69.6 L11.9 -70 L12.2 -70.3 L12.5 -70.7 L12.8 -71 L13.1 -71.4 L13.4 -71.7 L13.6 -72.2 L13.9 -72.5 L14.2 -72.9 L14.5 -73.2 L14.7 -73.6 L15 -74 L15.2 -74.4 L15.6 -74.7 L15.9 -75.1 L16.1 -75.5 L16.4 -75.9 L16.7 -76.3 L17 -76.7 L17.3 -77.1 L17.5 -77.5 L17.7 -78 L18 -78.5 L18.2 -78.9 L18.1 -78.9 L15 -78.8 L14.7 -79 L14.9 -78.5 L15.3 -78.1 L15.5 -77.7 L15.7 -77.2 L16 -76.8 L16.2 -76.4 L16.5 -76 L16.7 -75.6 L16.9 -75.1 L17.3 -74.8 L17.5 -74.4 L17.8 -74 L18 -73.6 L18.3 -73.3 L18.6 -72.9 L18.8 -72.5 L19.1 -72.1 L19.3 -71.7 L19.6 -71.3 L19.9 -71 L20.2 -70.6 L20.4 -70.3 L20.6 -69.9 L20.9 -69.5 L21.2 -69.1 L21.4 -68.8 L21.7 -68.4 L21.9 -68 L22.2 -67.7 L22.4 -67.2 L22.6 -66.9 L22.8 -66.5 L23.1 -66.1 L23.3 -65.8 L23.7 -65.5 L24 -65.1 L24.2 -64.7 L24.5 -64.4 L24.8 -64.1 L25.1 -63.7 L25.4 -63.4 L25.7 -63 L26 -62.7 L26.4 -62.3 L26.5 -62.2 L27.8 -62.7 L27.9 -62.9 L27.8 -63.4 L27.7 -63.9 L27.7 -64.4 L27.6 -64.9 L27.5 -65.4 L27.4 -65.8 L27.3 -66.3 L27.2 -66.8 L27.1 -67.3 L26.8 -67.7 L26.6 -68.2 L26.4 -68.6 L26.2 -69 L26 -69.4 L25.8 -69.9 L25.5 -70.3 L25.3 -70.7 L25.1 -71.1 L24.8 -71.5 L24.5 -71.9 L24.3 -72.3 L24.1 -72.7 L23.8 -73.1 L23.6 -73.5 L23.2 -73.8 L23 -74.2 L22.8 -74.6 L22.5 -75 L22.3 -75.4 L22 -75.8 L21.7 -76.1 L21.5 -76.5 L21.3 -76.9 L21.1 -77.3 L20.7 -77.6 L20.6 -78 L20.3 -78.4 L20.1 -78.8 L19.9 -79.2 L19.6 -79.5 L19.5 -79.9 L19.3 -80.3 L19 -80.6 L18.8 -81 L18.2 -82.1 L14.7 -81.9 L14.2 -81 L13.9 -80.6 L13.8 -80.3 L13.6 -79.9 L13.4 -79.5 L13.1 -79.2 L12.9 -78.8 L12.6 -78.5 L12.4 -78.1 L12.2 -77.7 L12 -77.3 L11.6 -77 L11.4 -76.6 L11.1 -76.3 L10.9 -75.9 L10.6 -75.5 L10.3 -75.1 L10 -74.8 L9.8 -74.4 L9.6 -74 L9.3 -73.6 L9 -73.3 L8.7 -72.9 L8.4 -72.5 L8.2 -72.1 L8 -71.7 L7.7 -71.3 L7.4 -70.9 L7.1 -70.6 L6.8 -70.2 L6.6 -69.8 L6.3 -69.4 L6.1 -69 L5.8 -68.6 L5.7 -68.1 L5.5 -67.6 L5.3 -67.2 L5.1 -66.7 L5 -66.2 L5 -65.7 L5 -65.1 L4.9 -64.6 L5 -64.1 Z"}];
  function wordmark(opts) {
    opts = opts || {};
    const id = "pfwg" + Math.floor(Math.random() * 1e9).toString(36);
    const ink = opts.ink || "currentColor", pencil = opts.pencil || "#2F6FB3";
    const cls = opts.cls ? ' class="' + opts.cls + '"' : "";
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-13 -107.2 372.1 151"' + cls + ' role="img" aria-label="inkling">' +
      '<defs><filter id="' + id + '" x="-20%" y="-20%" width="140%" height="140%"><feTurbulence type="fractalNoise" baseFrequency="1.3" numOctaves="2" seed="11" result="n"/>' +
      '<feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 -2.4 2.35" result="a"/><feComposite in="SourceGraphic" in2="a" operator="in"/></filter></defs>' +
      WORDMARK.map((p) => p.c === "pencil"
        ? '<path style="fill:' + pencil + '" filter="url(#' + id + ')" d="' + p.d + '"/>'
        : '<path style="fill:' + ink + '" d="' + p.d + '"/>').join("") + "</svg>";
  }

  // ---------- standalone elements ----------
  // Classes, so a page can colour them from its tokens:
  //   pf-pencil (Inkling), pf-soft (guides), pf-wash, pf-graphite (the writer), pf-ink, pf-ink3, pf-paper, pf-wood
  function svg(w, h, body, label) {
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ' + w + " " + h + '" width="' + w + '" height="' + h + '" role="img" aria-label="' + label + '">' + body + "</svg>";
  }
  const P = (m, cls) => '<path class="' + cls + '" d="' + m.d + '"/>';

  const ELEMENTS = {
    caret: {
      name: "Caret", job: "Marks where Inkling's query applies: below the line, at the end of the words it is about.",
      where: "In the text, under the end of every anchored phrase.",
      w: 24, h: 24,
      draw: () => svg(24, 24, '<rect class="pf-ink3" x="2" y="9.5" width="20" height="1" opacity=".35"/>' + P(caret(12, 10, 1.15, 3), "pf-soft"), "Caret"),
    },
    guide: {
      name: "Guide line", job: "Ties the caret to its query, running along the gap under the line so it never crosses letters.",
      where: "Between the text column and the margin.",
      w: 120, h: 40,
      draw: () => svg(120, 40, P(caret(14, 22, 1, 3), "pf-soft") + P(guide([19.5, 31], [112, 8], 5), "pf-soft"), "Guide line"),
    },
    "query-circle": {
      name: "Query circle", job: "Circles Inkling's question so it reads as a note to you, never as part of the text.",
      where: "Round every query in the margin.",
      w: 160, h: 72,
      draw: () => svg(160, 72, P(loop(18, 16, 124, 40, 7), "pf-pencil") +
        '<rect class="pf-pencil" x="30" y="27" width="70" height="3" rx="1.5" opacity=".55"/><rect class="pf-pencil" x="30" y="37" width="96" height="3" rx="1.5" opacity=".55"/><rect class="pf-pencil" x="30" y="47" width="54" height="3" rx="1.5" opacity=".55"/>', "Query circle"),
    },
    "aq-tag": {
      name: "AQ tag", job: "Numbers each query and says whether it is a fix or a question. Numbers never change once given.",
      where: "First line of every query.",
      w: 120, h: 24,
      draw: () => svg(120, 24, '<text class="pf-pencil" x="2" y="16" font-family="IBM Plex Mono, ui-monospace, monospace" font-size="12" font-weight="500">AQ1</text>' +
        '<text class="pf-pencil" x="34" y="16" font-family="IBM Plex Sans Condensed, sans-serif" font-size="12">· Fix</text>' +
        '<text class="pf-pencil" x="68" y="16" font-family="IBM Plex Mono, ui-monospace, monospace" font-size="12" font-weight="500" opacity=".75">AQ2</text>', "AQ tag"),
    },
    "ok-tick": {
      name: "OK tick", job: "Your mark that a query is answered. Graphite, because it is yours, not Inkling's.",
      where: "Beside a resolved query. Blue when Inkling settled it itself.",
      w: 56, h: 24,
      draw: () => svg(56, 24, '<text class="pf-graphite" x="2" y="17" font-family="IBM Plex Sans Condensed, sans-serif" font-size="14" font-weight="600">OK</text>' + P(tick(26, 9, 1.2, 13), "pf-graphite"), "OK tick"),
    },
    "stet-dots": {
      name: "Stet dots", job: "Your mark for “leave it as it is”: dots under the words, stet in the margin.",
      where: "Under the anchored words when you dismiss a query.",
      w: 120, h: 32,
      draw: () => svg(120, 32, '<rect class="pf-ink3" x="4" y="9" width="64" height="7" rx="2" opacity=".28"/>' +
        stetDots(4, 68, 22, 17).map((d) => '<circle class="pf-graphite" cx="' + f(d[0]) + '" cy="' + f(d[1]) + '" r="' + f(d[2]) + '"/>').join("") +
        '<text class="pf-graphite" x="80" y="20" font-family="IBM Plex Sans Condensed, sans-serif" font-size="14" font-style="italic">stet</text>', "Stet dots"),
    },
    "para-unread": {
      name: "¶ not read", job: "A paragraph number with nothing beside it: Inkling has not read this yet.",
      where: "Left gutter.", w: 32, h: 24,
      draw: () => svg(32, 24, '<text class="pf-ink3" x="20" y="16" text-anchor="end" font-family="IBM Plex Mono, ui-monospace, monospace" font-size="12">2</text>', "Paragraph not read"),
    },
    "para-reading": {
      name: "¶ reading", job: "The number turns blue and pulses while Inkling reads the paragraph.",
      where: "Left gutter.", w: 32, h: 24,
      draw: () => svg(32, 24, '<text class="pf-pencil" x="20" y="16" text-anchor="end" font-family="IBM Plex Mono, ui-monospace, monospace" font-size="12" font-weight="500">2</text><circle class="pf-pencil" cx="25" cy="12" r="1.2"/><circle class="pf-pencil" cx="29" cy="12" r="1.2" opacity=".5"/>', "Paragraph being read"),
    },
    "para-read": {
      name: "¶ read, holds", job: "A small blue tick: read, and nothing to ask.",
      where: "Left gutter.", w: 32, h: 24,
      draw: () => svg(32, 24, P(tick(2, 10, 0.62, 29), "pf-pencil") + '<text class="pf-ink3" x="20" y="16" text-anchor="end" font-family="IBM Plex Mono, ui-monospace, monospace" font-size="12">2</text>', "Paragraph read"),
    },
    "para-query": {
      name: "¶ has a query", job: "The number circled in blue: this paragraph has a query waiting.",
      where: "Left gutter.", w: 32, h: 24,
      draw: () => svg(32, 24, P(ring(16.4, 11.8, 7.2, 31), "pf-pencil") + '<text class="pf-ink3" x="20" y="16" text-anchor="end" font-family="IBM Plex Mono, ui-monospace, monospace" font-size="12">2</text>', "Paragraph with a query"),
    },
    "crop-marks": {
      name: "Crop marks", job: "Where the sheet is trimmed. They frame a note as a proof, and they print.",
      where: "Corners of the sheet; the app icon.", w: 64, h: 64,
      draw: () => svg(64, 64, '<g class="pf-line">' +
        [[14, 14, -1, -1], [50, 14, 1, -1], [14, 50, -1, 1], [50, 50, 1, 1]].map(([x, y, dx, dy]) =>
          '<line x1="' + (x + dx * 3) + '" y1="' + y + '" x2="' + (x + dx * 12) + '" y2="' + y + '"/><line x1="' + x + '" y1="' + (y + dy * 3) + '" x2="' + x + '" y2="' + (y + dy * 12) + '"/>').join("") +
        '</g><rect class="pf-paper-line" x="14" y="14" width="36" height="36"/>', "Crop marks"),
    },
    "registration": {
      name: "Registration mark", job: "The printer's target for lining up plates. Here it marks the sheet's top and sits in the app icon.",
      where: "Top of the sheet; the app icon.", w: 32, h: 32,
      draw: () => svg(32, 32, '<g class="pf-line"><circle cx="16" cy="16" r="6.5"/><line x1="16" y1="3" x2="16" y2="29"/><line x1="3" y1="16" x2="29" y2="16"/></g>', "Registration mark"),
    },
    "blue-pencil": {
      name: "Blue pencil", job: "Inkling's only tool. Shown in the empty note and in About.",
      where: "Empty note, About.", w: 120, h: 40,
      draw: () => svg(120, 40,
        '<g transform="rotate(-14 60 20)">' +
        '<path class="pf-pencil" d="M22 14 L96 14 L96 26 L22 26 Z"/>' +
        '<path class="pf-pencil-dark" d="M22 22 L96 22 L96 26 L22 26 Z" opacity=".35"/>' +
        '<path class="pf-pencil-light" d="M22 14 L96 14 L96 17 L22 17 Z" opacity=".35"/>' +
        '<path class="pf-wood" d="M22 14 L8 20 L22 26 Z"/>' +
        '<path class="pf-pencil" d="M12.4 18 L8 20 L12.4 22 Z"/>' +
        '<rect class="pf-wood" x="96" y="14" width="3" height="12"/>' +
        '<circle class="pf-pencil" cx="97.5" cy="20" r="2"/>' +
        '<text class="pf-paper-text" transform="translate(31 22.2) scale(.5)" font-family="IBM Plex Sans Condensed, sans-serif" font-size="12" letter-spacing=".06em">NON-PHOTO BLUE</text>' +
        "</g>", "Blue pencil"),
    },
    "pencil-on": {
      name: "Pencil switch, on", job: "The pencil is down: Inkling's marks are showing. ⌘⇧. lifts it.",
      where: "Note header.", w: 24, h: 24,
      draw: () => svg(24, 24, '<line class="pf-line-ink" x1="3" y1="20" x2="21" y2="20"/><g transform="rotate(-45 12 12)"><rect class="pf-pencil" x="9.5" y="2" width="5" height="13" rx=".8"/><path class="pf-pencil" d="M9.5 15 L12 20 L14.5 15 Z"/></g>', "Pencil on"),
    },
    "pencil-off": {
      name: "Pencil switch, off", job: "The pencil is lifted: only your words show, as they will print.",
      where: "Note header.", w: 24, h: 24,
      draw: () => svg(24, 24, '<line class="pf-line-ink" x1="3" y1="20" x2="21" y2="20"/><g transform="translate(2 -4) rotate(-45 12 12)" opacity=".75"><rect class="pf-soft-fill" x="9.5" y="2" width="5" height="13" rx=".8"/><path class="pf-soft-fill" d="M9.5 15 L12 20 L14.5 15 Z"/></g>', "Pencil off"),
    },
    "figure-frame": {
      name: "Figure frame", job: "Frames a sketch you took into the note. In ink: once it is yours, it prints.",
      where: "Figure blocks in the note.", w: 160, h: 96,
      draw: () => svg(160, 96, '<g class="pf-line-ink">' +
        '<path d="M6 16 V6 H16"/><path d="M144 6 H154 V16"/><path d="M6 64 V74 H16"/><path d="M144 74 H154 V64"/></g>' +
        '<path class="pf-ink-fill" d="' + mark(resample(cubic([24, 58], [50, 30], [90, 52], [136, 22], 30), 1), 1.3, [0.1, 0.1], 0.05, 41).d + '"/>' +
        '<line class="pf-line-ink" x1="6" y1="84" x2="154" y2="84" stroke-width=".75"/>' +
        '<text class="pf-ink3" transform="translate(6 94) scale(.75)" font-family="IBM Plex Sans Condensed, sans-serif" font-size="12">Fig. 1  Pace and finish time</text>', "Figure frame"),
    },
  };

  // Light colours for standalone files; the page overrides these from its tokens.
  const FILE_STYLE =
    ".pf-pencil{fill:#2F6FB3}.pf-pencil-dark{fill:#1F4F86}.pf-pencil-light{fill:#9CC4EE}.pf-soft{fill:#5E95CF}.pf-soft-fill{fill:#8DBBE8}" +
    ".pf-graphite{fill:#55524B}.pf-ink{fill:#1B1A17}.pf-ink-fill{fill:#1B1A17}.pf-ink3{fill:#6E6A61}.pf-wood{fill:#E3C69B}" +
    ".pf-paper-text{fill:#FBFBF8}.pf-line{stroke:#1B1A17;stroke-width:1;fill:none;opacity:.55}.pf-line-ink{stroke:#1B1A17;stroke-width:1.25;fill:none;stroke-linecap:round}" +
    ".pf-paper-line{fill:none;stroke:#1B1A17;stroke-width:.5;stroke-dasharray:2 2;opacity:.25}" +
    "@media (prefers-color-scheme:dark){.pf-pencil{fill:#7CB2EC}.pf-soft{fill:#5E95CF}.pf-graphite{fill:#A8A395}.pf-ink,.pf-ink-fill{fill:#E9E6DE}" +
    ".pf-ink3{fill:#8E897F}.pf-line,.pf-line-ink,.pf-paper-line{stroke:#E9E6DE}.pf-paper-text{fill:#171A1E}}";

  g.ProofMarks = {
    rng, f, cubic, join, resample, ribbon, line, mark,
    caret, guide, loop, ring, tick, stetDots, bracket, arrow,
    wordmark, ELEMENTS, FILE_STYLE,
  };
})(typeof window !== "undefined" ? window : globalThis);

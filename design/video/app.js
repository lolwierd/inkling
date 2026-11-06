/* Inkling, rebuilt in HTML for the walkthrough film: the window, the page, threads, tags,
 * menus and panels, as src/ draws them on 6 Nov 2026 (checked against screenshots of the
 * real app). Nothing here keeps state between frames except caches of pure results, so
 * film.js can call render(spec) for any time in any order.
 *
 * Geometry is the app's, in points, for its default 1320 x 860 window.
 */
(function () {
  "use strict";
  const $ = (s, r) => (r || document).querySelector(s);
  const fx = (v) => Math.round(v * 100) / 100;
  const clamp = (v, a = 0, b = 1) => Math.max(a, Math.min(b, v));
  const lerp = (a, b, k) => a + (b - a) * k;
  const prog = (t, t0, t1) => (t1 <= t0 ? (t >= t1 ? 1 : 0) : clamp((t - t0) / (t1 - t0)));
  const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  const E = {
    out: (k) => 1 - Math.pow(1 - k, 3),
    in: (k) => k * k * k,
    io: (k) => (k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2),
    sine: (k) => 0.5 - 0.5 * Math.cos(Math.PI * k),
    quint: (k) => 1 - Math.pow(1 - k, 5),
  };
  /** A spring from 0 to 1 from rest after `s` seconds (stiffness k, damping c, mass 1). */
  function spring(s, k, c) {
    if (s <= 0) return 0;
    const w = Math.sqrt(k), z = c / (2 * w);
    if (z < 0.999) { const wd = w * Math.sqrt(1 - z * z); return 1 - Math.exp(-z * w * s) * (Math.cos(wd * s) + ((z * w) / wd) * Math.sin(wd * s)); }
    return 1 - Math.exp(-w * s) * (1 + w * s);
  }
  function rng(seed) {
    let s = (seed * 2654435761) >>> 0 || 1;
    return () => { s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; return (s % 100000) / 100000; };
  }

  // ================================================================ theme (src/theme.rs)
  const PAL = {
    light: { paper: "#fbf8f2", shelf: "#f3ede2", cloth: "#efe8db", ink: "#26232b", graphite: "#57505c", faint: "#6f6862", rule: "#e3dacb", card: "#fffdf8", line2: "#d6cbb8",
      wash: [166, 63, 50, 0.078], sel: [46, 63, 122, 0.17], shade: [60, 40, 20], menushade: 0.1 },
    dark: { paper: "#1f1d22", shelf: "#1a181d", cloth: "#18161a", ink: "#ede6da", graphite: "#bdb4a8", faint: "#a39a8f", rule: "#332f36", card: "#26232a", line2: "#45404b",
      wash: [217, 112, 94, 0.12], sel: [140, 155, 224, 0.28], shade: [0, 0, 0], menushade: 0.45 },
  };
  const THREADS = {
    madder: ["#a63f32", "#d9705e"], indigo: ["#2e3f7a", "#8c9be0"], sage: ["#4f7350", "#8db88e"], rose: ["#9b3550", "#e07a96"],
    plum: ["#6b3e62", "#c08ab4"], walnut: ["#6e4b2e", "#c49a73"], ochre: ["#8f6214", "#d9a548"],
  };
  const THREAD_ORDER = ["madder", "indigo", "sage", "rose", "plum", "walnut", "ochre"];
  const hexRGB = (h) => { const n = parseInt(h.slice(1), 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; };
  const mixHex = (a, b, k) => { const A = hexRGB(a), B = hexRGB(b); return `rgb(${A.map((v, i) => Math.round(lerp(v, B[i], k))).join(",")})`; };
  /** A thread's colour in the current light (k: 0 linen .. 1 lamplight). */
  const thread = (name, k) => (THREADS[name] ? mixHex(THREADS[name][0], THREADS[name][1], k) : name);
  /** The colour each mind wears (note_page.rs `mind_thread`): the note's own mind wears
   *  your thread; the others keep theirs, the same every time. */
  function mindThread(name, own, chosen) {
    if (!name || name === own) return chosen;
    const known = { Inkling: "madder", Skeptic: "indigo", "Socratic tutor": "ochre", Tutor: "ochre", Editor: "sage", "Staff engineer": "plum", Investor: "walnut" }[name];
    if (known && known !== chosen) return known;
    const others = ["indigo", "ochre", "sage", "plum", "walnut", "rose", "madder"].filter((c) => c !== chosen);
    return others[Number(seedOf(name) % BigInt(others.length))];
  }
  /** FNV-1a, as stitch.rs `seed_of`. */
  function seedOf(s) {
    let h = 0xcbf29ce484222325n;
    for (const b of new TextEncoder().encode(s)) { h ^= BigInt(b); h = (h * 0x100000001b3n) & 0xffffffffffffffffn; }
    return h;
  }
  function applyTheme(el, k, voice, opts) {
    const L = PAL.light, D = PAL.dark, st = el.style;
    for (const key of ["paper", "shelf", "cloth", "ink", "graphite", "faint", "rule", "card", "line2"]) st.setProperty("--" + key, mixHex(L[key], D[key], k));
    const rgba = (a, b) => `rgba(${[0, 1, 2].map((i) => Math.round(lerp(a[i], b[i], k))).join(",")},${fx(lerp(a[3], b[3], k))})`;
    st.setProperty("--wash", rgba(L.wash, D.wash));
    st.setProperty("--sel", rgba(L.sel, D.sel));
    st.setProperty("--shade", [0, 1, 2].map((i) => Math.round(lerp(L.shade[i], D.shade[i], k))).join(","));
    st.setProperty("--menushade", String(fx(lerp(L.menushade, D.menushade, k))));
    st.setProperty("--voice", thread(voice || "madder", k));
    for (const n of THREAD_ORDER) st.setProperty("--" + n, thread(n, k));
    st.setProperty("--weave", k < 0.5 ? `url("${ASSETS.WEAVE.light}")` : `url("${ASSETS.WEAVE.dark}")`);
    st.setProperty("--note", ((opts && opts.note) || 17) + "px");
    st.setProperty("--voicefont", opts && opts.lettering === "notes" ? '"Fraunces Text", serif' : '"Figtree", sans-serif');
    st.setProperty("--capink", mixHex(L.ink, D.ink, k));
  }

  // ================================================================ stitches (stitch.rs)
  /** A capsule of thread from (x0,y) to (x1,y): the running stitch's one stitch. */
  function capsule(x0, y0, x1, y1, h, c, op) {
    const L = Math.hypot(x1 - x0, y1 - y0);
    if (L < 0.2) return "";
    return `<line x1="${fx(x0)}" y1="${fx(y0)}" x2="${fx(x1)}" y2="${fx(y1)}" style="stroke:${c};stroke-width:${h};stroke-linecap:round"${op != null && op < 0.999 ? ` opacity="${fx(op)}"` : ""}/>`;
  }
  /** stitch.rs `jitter`: a repeatable wobble in -0.5..0.5. */
  function jitter(seed, i) {
    let x = (BigInt(seed) ^ ((BigInt(i) * 0x9e3779b97f4a7c15n) & 0xffffffffffffffffn)) & 0xffffffffffffffffn;
    x ^= x >> 33n; x = (x * 0xff51afd7ed558ccdn) & 0xffffffffffffffffn; x ^= x >> 33n;
    return Number(x & 0xffffn) / 65535 - 0.5;
  }
  const jitMemo = new Map();
  function jit(seed, i) { const k = seed + ":" + i; let v = jitMemo.get(k); if (v == null) { v = jitter(seed, i); jitMemo.set(k, v); } return v; }
  /** Running stitch along one line (stitch.rs `running_stitch`). Returns the stitch list. */
  function runningStitch(x0, x1, y, seed, skipFirst) {
    const out = [];
    let x = x0 + (skipFirst ? 11 : 0), i = 0;
    while (x < x1 - 2.5) {
      const len = 6.6 + jit(seed, i) * 1.4, end = Math.min(x + len, x1), dy = jit(seed, i + 101) * 0.7;
      out.push({ x0: x, x1: end, y: y + dy });
      x = end + 3.7 + jit(seed, i + 211) * 0.8;
      i++;
    }
    return out;
  }
  /** Cross-stitch: two strokes, `k1` and `k2` how far each is sewn. */
  function crossSVG(cx, cy, half, c, k1 = 1, k2 = 1, w = 1.75) {
    let s = "";
    if (k1 > 0) s += `<line x1="${fx(cx - half)}" y1="${fx(cy - half * 0.9)}" x2="${fx(cx - half + 2 * half * k1)}" y2="${fx(cy - half * 0.9 + 1.8 * half * k1)}" style="stroke:${c};stroke-width:${w};stroke-linecap:round"/>`;
    if (k2 > 0) s += `<line x1="${fx(cx + half)}" y1="${fx(cy - half * 0.9)}" x2="${fx(cx + half - 2 * half * k2)}" y2="${fx(cy - half * 0.9 + 1.8 * half * k2)}" style="stroke:${c};stroke-width:${w};stroke-linecap:round"/>`;
    return s;
  }
  /** French knot (stitch.rs `french_knot`): a bead with one coil of lighter thread. */
  function knotSVG(cx, cy, r, c, scale = 1) {
    r *= Math.max(0, scale);
    if (r < 0.3) return "";
    let d = "";
    for (let i = 0; i <= 22; i++) {
      const t = i / 22, a = -2.0 + t * 1.3 * 2 * Math.PI, rr = r * (0.16 + 0.6 * t);
      d += (i ? "L" : "M") + fx(cx + rr * Math.cos(a)) + " " + fx(cy + rr * Math.sin(a));
    }
    return `<circle cx="${fx(cx)}" cy="${fx(cy)}" r="${fx(r)}" style="fill:${c}"/><path d="${d}" style="fill:none;stroke:color-mix(in srgb,${c} 76%,#fff);stroke-width:${fx(Math.max(0.6, r * 0.22))};stroke-linecap:round"/>`;
  }
  /** The knot pops in just after the pull: a small overshoot, then rest (note_page.rs). */
  function knotScale(t) { if (t <= 0) return 0; const k = Math.min(t / 0.2, 1); return k < 0.6 ? (k / 0.6) * 1.3 : 1.3 - ((k - 0.6) / 0.4) * 0.3; }
  /** One motif of the sampler border: a small cross-stitched diamond (stitch.rs). */
  function motifSVG(cx, cy, c, scale) {
    const s = 4.4 * clamp(scale, 0, 1.25);
    if (s <= 0.05) return "";
    let out = "";
    [[0, -1], [-1, 0], [1, 0], [0, 1], [0, 0]].forEach(([dx, dy], i) => {
      const k = clamp(scale * 5 - i);
      out += crossSVG(cx + dx * s * 1.55, cy + dy * s * 1.55, s * 0.52, c, clamp(k * 2), clamp(k * 2 - 1), 1.75);
    });
    return out;
  }
  const NEEDLE = (x, y, phase, c) => {
    const lift = 3.5 * Math.sin(phase * 2 * Math.PI), tip = [x, y + lift + 7], eye = [x + 1.6, y + lift - 9];
    return `<path d="M${fx(tip[0])} ${fx(tip[1])}L${fx(eye[0])} ${fx(eye[1])}" style="stroke:${c};stroke-width:1.4;stroke-linecap:round"/>` +
      `<path d="M${fx(eye[0] - 0.9)} ${fx(eye[1] - 2.6)}C${fx(eye[0] - 0.9)} ${fx(eye[1] + 1.6)} ${fx(eye[0] + 1.4)} ${fx(eye[1] - 2.6)} ${fx(eye[0] + 1.4)} ${fx(eye[1] + 1.6)}" style="fill:none;stroke:${c};stroke-width:.9"/>`;
  };

  // ================================================================ the thread (stitch.rs `Rope`)
  const R = { POINTS: 24, GRAVITY: 760, DAMPING: 0.965, ITER: 16, BENDING: 0.04, MIN_SPAN: 1.955, SLACK: 0.08, MAX_SLACK: 30, DT: 1 / 120 };
  const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
  class Rope {
    constructor(a, b, opts) {
      this.a = a.slice(); this.b = b.slice(); this.slack = 1; this.slackTo = 1; this.pinB = true;
      this.gravity = (opts && opts.gravity) || R.GRAVITY; this.maxSlack = (opts && opts.maxSlack) || R.MAX_SLACK; this.slackK = (opts && opts.slackK) || R.SLACK;
      this.layOut();
      if (!(opts && opts.noSettle)) this.settle(360);
    }
    layOut() {
      const d = Math.max(0.001, dist(this.a, this.b)), ux = (this.b[0] - this.a[0]) / d, uy = (this.b[1] - this.a[1]) / d;
      let nx = -uy, ny = ux;
      if (ny < 0 || (Math.abs(ny) < 1e-3 && nx * (this.b[0] - this.a[0]) < 0)) { nx = -nx; ny = -ny; }
      const bow = Math.min(d * 0.16, this.maxSlack * 1.4);
      this.p = []; this.q = [];
      for (let i = 0; i < R.POINTS; i++) {
        const t = i / (R.POINTS - 1), k = bow * 4 * t * (1 - t);
        const pt = [this.a[0] + (this.b[0] - this.a[0]) * t + nx * k, this.a[1] + (this.b[1] - this.a[1]) * t + ny * k];
        this.p.push(pt); this.q.push(pt.slice());
      }
    }
    seg() { const d = dist(this.a, this.b); return (d * (1 - 0.005 * (1 - this.slack)) + Math.min(d * this.slackK, this.maxSlack) * this.slack) / (R.POINTS - 1); }
    settle(n) { for (let i = 0; i < n; i++) this.step(); this.q = this.p.map((p) => p.slice()); }
    nudge(s) { for (let i = 1; i < R.POINTS - 1; i++) { const t = i / (R.POINTS - 1); this.q[i][0] += s * 0.6 * Math.sin(Math.PI * t); this.q[i][1] += s * Math.sin(Math.PI * t); } }
    step() {
      if (this.slack !== this.slackTo) {
        const rate = R.DT / 0.32;
        this.slack = this.slack > this.slackTo ? Math.max(this.slack - rate, this.slackTo) : Math.min(this.slack + rate, this.slackTo);
      }
      const last = R.POINTS - 1, p = this.p, q = this.q;
      for (let i = 1; i < R.POINTS; i++) {
        if (i === last && this.pinB) continue;
        const vx = (p[i][0] - q[i][0]) * R.DAMPING, vy = (p[i][1] - q[i][1]) * R.DAMPING;
        q[i][0] = p[i][0]; q[i][1] = p[i][1];
        const g = this.gravity * (0.03 + 0.97 * this.slack);
        p[i][0] += vx; p[i][1] += vy + g * R.DT * R.DT;
      }
      const seg = this.seg();
      for (let it = 0; it < R.ITER; it++) {
        p[0][0] = this.a[0]; p[0][1] = this.a[1];
        if (this.pinB) { p[last][0] = this.b[0]; p[last][1] = this.b[1]; }
        for (let i = 0; i < last; i++) {
          const P = p[i], Q = p[i + 1], d = Math.max(1e-4, dist(P, Q)), diff = (d - seg) / d;
          const fp = i === 0, fq = i + 1 === last && this.pinB;
          const wp = fp && fq ? 0 : fp ? 0 : fq ? 1 : 0.5, wq = fp && fq ? 0 : fp ? 1 : fq ? 0 : 0.5;
          const dx = (Q[0] - P[0]) * diff, dy = (Q[1] - P[1]) * diff;
          P[0] += dx * wp; P[1] += dy * wp; Q[0] -= dx * wq; Q[1] -= dy * wq;
        }
        for (let i = 0; i < last - 1; i++) {
          const P = p[i], Q = p[i + 2], d = Math.max(1e-4, dist(P, Q)), min = seg * R.MIN_SPAN;
          if (d >= min) continue;
          const diff = ((d - min) / d) * 0.5, fp = i === 0, fq = i + 2 === last && this.pinB;
          const wp = fp && fq ? 0 : fp ? 0 : fq ? 1 : 0.5, wq = fp && fq ? 0 : fp ? 1 : fq ? 0 : 0.5;
          const dx = (Q[0] - P[0]) * diff, dy = (Q[1] - P[1]) * diff;
          P[0] += dx * wp; P[1] += dy * wp; Q[0] -= dx * wq; Q[1] -= dy * wq;
        }
        const mids = [];
        for (let i = 1; i < last; i++) mids.push([(p[i - 1][0] + p[i + 1][0]) / 2, (p[i - 1][1] + p[i + 1][1]) / 2]);
        for (let i = 1; i < last; i++) {
          if (i + 1 === last && !this.pinB) continue;
          const m = mids[i - 1];
          p[i][0] += (m[0] - p[i][0]) * R.BENDING; p[i][1] += (m[1] - p[i][1]) * R.BENDING;
        }
      }
      p[0][0] = this.a[0]; p[0][1] = this.a[1];
      if (this.pinB) { p[last][0] = this.b[0]; p[last][1] = this.b[1]; }
    }
    shape() {
      let pts = this.p.map((x) => x.slice());
      for (let k = 0; k < 2; k++) { const pr = pts.map((x) => x.slice()); for (let i = 1; i < pts.length - 1; i++) pts[i] = [(pr[i - 1][0] + 2 * pr[i][0] + pr[i + 1][0]) / 4, (pr[i - 1][1] + 2 * pr[i][1] + pr[i + 1][1]) / 4]; }
      return pts;
    }
    path() {
      const P = this.shape();
      let d = `M${fx(P[0][0])} ${fx(P[0][1])}`;
      for (let i = 1; i < P.length - 1; i++) { const m = [(P[i][0] + P[i + 1][0]) / 2, (P[i][1] + P[i + 1][1]) / 2]; d += `Q${fx(P[i][0])} ${fx(P[i][1])} ${fx(m[0])} ${fx(m[1])}`; }
      d += `L${fx(P[P.length - 1][0])} ${fx(P[P.length - 1][1])}`;
      return d;
    }
    end() { return this.p[R.POINTS - 1]; }
  }
  const r2 = (v) => Math.round(v * 4) / 4;
  const restMemo = new Map();
  function restRope(a, b, opts) {
    const key = [r2(a[0]), r2(a[1]), r2(b[0]), r2(b[1]), opts ? JSON.stringify(opts) : ""].join(",");
    let r = restMemo.get(key);
    if (!r) { r = new Rope([r2(a[0]), r2(a[1])], [r2(b[0]), r2(b[1])], opts); restMemo.set(key, r); if (restMemo.size > 3000) restMemo.clear(); }
    return r;
  }
  /** A thread in motion, simulated at the fixed step from `t0` (so any frame comes out the
   *  same whichever worker renders it). `track` says what happens: carry (the needle draws
   *  it out from the hole to the tag), tie (it pulls taut at tieAt), cut (it falls at cutAt),
   *  sway (a touch at t0), and `b(t)` may move the tag end. */
  const simMemo = new Map();
  function simRope(key, a, b, t0, t, track) {
    const speed = track.speed || 1;
    const steps = Math.max(0, Math.floor(((t - t0) * speed) / R.DT + 1e-6));
    const ck = `${key}|${r2(a[0])},${r2(a[1])},${r2(b[0])},${r2(b[1])}|${t0}|${JSON.stringify(track, (k, v) => (typeof v === "function" ? "fn" : v))}`;
    let st = simMemo.get(ck);
    if (!st || st.k > steps) {
      const opts = track.opts || null;
      let r;
      if (track.carry) { r = new Rope(a, [a[0] + 0.5, a[1] + 0.5], { ...(opts || {}), noSettle: true }); }
      else { r = new Rope(a, track.b ? track.b(t0) : b, opts); }
      if (track.sway) r.nudge(track.sway);
      st = { r, k: 0 };
      simMemo.set(ck, st);
      if (simMemo.size > 400) { const first = simMemo.keys().next().value; simMemo.delete(first); }
    }
    const r = st.r;
    for (; st.k < steps; st.k++) {
      const ts = t0 + ((st.k + 1) * R.DT) / speed;
      if (track.carry) { const e = E.out(clamp((ts - t0) / (track.carry || 0.5))); r.b = [lerp(a[0] + 0.5, b[0], e), lerp(a[1] + 0.5, b[1], e)]; }
      if (track.b) r.b = track.b(ts);
      if (track.tieAt != null && ts >= track.tieAt) r.slackTo = 0;
      if (track.cutAt != null && ts >= track.cutAt) r.pinB = false;
      if (track.swayAt != null && Math.abs(ts - track.swayAt) < R.DT / 2) r.nudge(track.swayAmt || 6);
      r.step();
    }
    return r;
  }
  function ropeSVG(r, c, op) {
    const d = r.path();
    return `<g opacity="${fx(op)}"><path d="${d}" style="fill:none;stroke:rgba(var(--shade),.24);stroke-width:2.2;stroke-linecap:round" transform="translate(.6 1.5)"/>` +
      `<path d="${d}" style="fill:none;stroke:${c};stroke-width:1.75;stroke-linecap:round"/>` +
      `<path d="${d}" style="fill:none;stroke:color-mix(in srgb,${c} 72%,#fff);stroke-width:.7;stroke-dasharray:1.3 2.9;opacity:.55" transform="translate(.15 -.35)"/></g>`;
  }

  // ================================================================ icons (lucide, as the app's icons/)
  const ICON = {
    menu: '<path d="M3 5.5h14M3 10h14M3 14.5h14"/>',
    chev: '<path d="M6 8l4 4 4-4"/>',
    pencil: '<path d="M13.5 3.5l3 3L7 16H4v-3z"/>',
    send: '<path d="M17 3L9 11M17 3l-5 14-3-6-6-3z"/>',
    copy: '<rect x="7" y="7" width="10" height="10" rx="2"/><path d="M13 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2"/>',
    search: '<circle cx="9" cy="9" r="5.5"/><path d="M13 13l4 4"/>',
    plus: '<path d="M10 4v12M4 10h12"/>',
    file: '<path d="M5 2.5h6.5L15.5 6.5v11h-10.5z"/><path d="M11 2.5v4.5h4.5M8 10.5h5M8 13.5h5"/>',
    folder: '<path d="M2.5 5.5a1.5 1.5 0 0 1 1.5-1.5h3.5l2 2H16a1.5 1.5 0 0 1 1.5 1.5v7.5A1.5 1.5 0 0 1 16 16.5H4A1.5 1.5 0 0 1 2.5 15z"/>',
    folderOpen: '<path d="M2.5 15V5.5A1.5 1.5 0 0 1 4 4h3.5l2 2H15a1.5 1.5 0 0 1 1.5 1.5V9"/><path d="M2.5 15l2.4-5.2A1.5 1.5 0 0 1 6.3 9h11.2l-2.6 6.2a1.5 1.5 0 0 1-1.4.8H2.5z"/>',
    folderPlus: '<path d="M2.5 5.5a1.5 1.5 0 0 1 1.5-1.5h3.5l2 2H16a1.5 1.5 0 0 1 1.5 1.5v7.5A1.5 1.5 0 0 1 16 16.5H4A1.5 1.5 0 0 1 2.5 15z"/><path d="M10 8.5v5M7.5 11h5"/>',
    x: '<path d="M5 5l10 10M15 5L5 15"/>',
    check: '<path d="M4.5 10.5l3.5 3.5 7.5-8"/>',
    bulb: '<path d="M7.5 15h5M8 17.5h4M10 2.5a5 5 0 0 0-3 9c.6.5 1 1.2 1 2h4c0-.8.4-1.5 1-2a5 5 0 0 0-3-9z"/>',
    msg: '<path d="M3.5 16.5l1.2-3.6A6.5 6.5 0 1 1 7.6 16z"/>',
    arrowUp: '<path d="M10 15V5M5.5 9.5L10 5l4.5 4.5"/>',
    left: '<path d="M12 5l-5 5 5 5"/>', right: '<path d="M8 5l5 5-5 5"/>', replay: '<path d="M4 10a6 6 0 1 0 2-4.5M4 3.5v3h3"/>',
  };
  const icon = (n, size, c, sw) => `<svg viewBox="0 0 20 20" width="${size}" height="${size}" style="fill:none;stroke:${c};stroke-width:${sw || 1.6};stroke-linecap:round;stroke-linejoin:round;flex:none">${ICON[n]}</svg>`;

  // ================================================================ layout (note_page.rs `Metrics`, measured)
  const GEO = {
    wide: { colLeft: 351, col: 614, margin: 248, gap: 40, lgap: 38, sheet: [38, 44, 1244], band: [66, 1254], header: 94, both: true },
    chat: { colLeft: 66, col: 560, margin: 248, gap: 50, lgap: 38, sheet: [38, 44, 933], band: [66, 943], header: 94, both: false },
  };
  const TAB_W = 54, TAB_H = 26;
  const KIND = { p: "p", h1: "h1", h2: "h2", h3: "h3", li: "li", ol: "ol", q: "q", bullet: "li", numbered: "ol", paragraph: "p", quote: "q" };

  /** Text with spans for anchors, the selection and a caret marker. */
  function markup(text, ranges, caret) {
    const cuts = new Set([0, text.length]);
    ranges.forEach((r) => { cuts.add(clamp(r.a, 0, text.length)); cuts.add(clamp(r.b, 0, text.length)); });
    if (caret != null && caret >= 0) cuts.add(Math.min(caret, text.length));
    const pts = [...cuts].sort((a, b) => a - b);
    let html = "";
    for (let k = 0; k < pts.length; k++) {
      const p = pts[k];
      if (caret === p) html += '<span class="cm"></span>';
      if (k === pts.length - 1) break;
      const q = pts[k + 1], seg = text.slice(p, q);
      if (!seg) continue;
      const on = ranges.filter((r) => r.a <= p && r.b >= q);
      if (!on.length) { html += esc(seg); continue; }
      const ids = on.filter((r) => r.id).map((r) => r.id).join(" ");
      const style = on.map((r) => r.style || "").join("");
      html += `<span class="${on.map((r) => r.cls || "").join(" ")}"${ids ? ` data-a="${ids}"` : ""}${style ? ` style="${style}"` : ""}>${esc(seg)}</span>`;
    }
    return html;
  }

  // ================================================================ the page
  /** Renders a note page into `host` (an element in window coordinates) and returns the
   *  measured geometry. `z` is the world scale on screen, for measuring. */
  /** A thread colour: a name from the thread box, or any CSS colour. */
  const cssCol = (c, d) => (!c ? d : c.startsWith("#") || c.startsWith("rgb") || c.startsWith("var") || c.startsWith("color") ? c : `var(--${c})`);
  function renderPage(host, P, t, z, K) {
    const G = P.chatOpen ? GEO.chat : GEO.wide;
    const colL = G.colLeft, colR = G.colLeft + G.col;
    const scroll = P.scroll || 0;
    const hideK = P.hideK || 0; // Hide threads: every mark lifts away together
    const markOp = 1 - hideK, lift = -10 * hideK;
    const voice = `var(--voice)`;
    let html = "";
    // the sheet and its sampler border
    html += `<div class="sheet" style="left:${G.sheet[0]}px;width:${G.sheet[2]}px;top:${fx(G.sheet[1] - scroll)}px"></div>`;
    // header row
    const H = P.header || {};
    const hy = G.header - scroll;
    const hov = (k) => (H.hov === k ? " hov" : "");
    const hideLabel = P.hidden ? "Show threads" : "Hide threads";
    html += `<div class="nh" style="left:${colL}px;width:${G.col + 4}px;top:${fx(hy)}px;opacity:${fx(H.op == null ? 1 : H.op)}">` +
      (P.hidden ? "" :
        `<span class="mind it${hov("mind")}"><span class="dot" style="width:7px;height:7px;background:${H.mindC || voice}"></span>${esc(H.mind || "Inkling")}${icon("chev", 12, "var(--faint)")}</span>` +
        (P.brief || P.briefIn ? "" : `<span class="it${hov("tell")}">${icon("pencil", 12, "var(--faint)")}Tell Inkling</span>`) +
        `<span class="it${hov("mode")}">${esc(H.mode || "Reads along")}${icon("chev", 12, "var(--faint)")}</span>`) +
      `<span class="grow"></span>` +
      (!P.hidden && H.loose ? `<span class="it${hov("loose")}"><span class="dot" style="width:7px;height:7px"></span>${H.loose === 1 ? "1 loose thread" : H.loose + " loose threads"}</span>` : "") +
      `<span class="it${hov("hide")}">${hideLabel}</span></div>`;
    let top = 136 - scroll;
    if (P.brief) {
      html += `<div class="briefline" style="left:${colL}px;top:${fx(hy + 30)}px;width:${G.col}px;opacity:${fx(H.op == null ? 1 : Math.max(0.5, H.op))}"><span class="lab">${icon("pencil", 10, "var(--faint)")}Tell Inkling</span><span class="tx">${esc(P.brief)}</span></div>`;
      top += 28;
    }
    if (P.briefIn) {
      const b = P.briefIn;
      html += `<div class="briefin" style="left:${colL}px;top:${fx(hy + 30)}px;width:${G.col}px"><div class="q">What’s this note for, or how should Inkling be here? It keeps this in mind.</div>` +
        `<div class="v">${b.text ? esc(b.text) : ""}${b.caret ? '<span class="icaret"></span>' : ""}</div><div class="row"><span>Cancel</span><span style="padding:5px 12px;border-radius:999px;background:var(--ink);color:var(--paper)">Save</span></div></div>`;
      top += 104;
    }
    // title
    const T = P.title || { text: "" };
    html += `<h1 class="n-title${T.text ? "" : " ph"}" style="left:${colL}px;top:${fx(top)}px;width:${G.col}px">${T.text ? markup(T.text, [], T.caret) : (T.caret != null ? '<span class="cm"></span>' : "") + "Untitled"}</h1>`;
    // blocks
    let bh = "";
    let olN = 0;
    for (const b of P.blocks) {
      const kind = KIND[b.kind] || "p";
      olN = kind === "ol" ? olN + 1 : 0;
      const ranges = (b.ranges || []).slice();
      if (b.sel && b.sel[1] > b.sel[0]) ranges.push({ a: b.sel[0], b: b.sel[1], cls: "sel" });
      const ph = !b.text && b.ph;
      const gap = b.gap != null ? b.gap : kind === "h2" || kind === "h1" || kind === "h3" ? 18 : 12;
      bh += `<div class="blk ${kind}${ph ? " ph" : ""}" data-b="${b.id}" data-n="${olN}" style="margin-top:${b.first ? 0 : gap}px${b.style ? ";" + b.style : ""}">${ph ? (b.caret != null ? '<span class="cm"></span>' : "") + esc(b.ph) : markup(b.text, ranges, b.caret)}</div>`;
    }
    html += `<div class="blocks" style="left:${colL}px;top:${fx(top + 54)}px;width:${G.col}px">${bh}</div>`;
    html += `<div class="caret" style="display:none"></div>`;
    html += `<svg class="ov" width="1320" height="860" style="opacity:1"></svg><div class="tags"></div><div class="menus"></div>`;
    host.innerHTML = html;

    // ---- measure
    const hr = host.getBoundingClientRect();
    const rel = (r) => ({ x: (r.left - hr.left) / z, y: (r.top - hr.top) / z, r: (r.right - hr.left) / z, b: (r.bottom - hr.top) / z, w: r.width / z, h: r.height / z });
    const blockEls = {};
    host.querySelectorAll(".blk").forEach((el) => (blockEls[el.dataset.b] = el));
    const lineRects = (bid, a, b) => {
      // the rects of a character range, one per line
      const el = blockEls[bid];
      if (!el) return [];
      const spans = el.querySelectorAll(`[data-a~="${a}"]`);
      const rs = [];
      spans.forEach((sp) => Array.from(sp.getClientRects()).forEach((r) => { if (r.width > 0.5) rs.push(rel(r)); }));
      const lines = [];
      rs.sort((p, q) => p.y - q.y || p.x - q.x).forEach((r) => {
        const L = lines.find((l) => Math.abs(l.y - r.y) < 5);
        if (L) { L.x = Math.min(L.x, r.x); L.r = Math.max(L.r, r.r); L.b = Math.max(L.b, r.b); }
        else lines.push({ ...r });
      });
      return lines;
    };
    const blockBox = (bid) => { const el = blockEls[bid]; return el ? rel(el.getBoundingClientRect()) : null; };
    const firstLine = (bid) => {
      const el = blockEls[bid];
      if (!el) return null;
      const lh = parseFloat(getComputedStyle(el).lineHeight) || 28;
      const bb = blockBox(bid);
      return { y: bb.y, h: lh, mid: bb.y + lh / 2 };
    };
    // caret
    const cm = host.querySelector(".cm"), caretEl = host.querySelector(".caret");
    if (cm && P.caretOn !== false) {
      const r = rel(cm.getBoundingClientRect());
      caretEl.style.display = "block";
      caretEl.style.left = fx(r.x - 0.8) + "px"; caretEl.style.top = fx(r.y - 1) + "px"; caretEl.style.height = fx(r.h + 2) + "px";
    }
    const geo = { G, colL, colR, lineRects, blockBox, firstLine, rel, host, caret: cm ? rel(cm.getBoundingClientRect()) : null };

    // ---- the sampler border, gutter marks, stitches, threads
    let svg = "";
    const B = P.border || {};
    const by = 68 - scroll + lift;
    {
      const x0 = G.band[0], x1 = G.band[1], motifs = B.motifs || [];
      const step = 22, mx0 = (x0 + x1) / 2 - ((motifs.length - 1) * step) / 2;
      // the dashed line, with a gap where the motifs sit
      const gapL = motifs.length ? mx0 - 16 : null, gapR = motifs.length ? mx0 + (motifs.length - 1) * step + 16 : null;
      const dash = (a, b) => `<line x1="${fx(a)}" y1="${fx(by)}" x2="${fx(b)}" y2="${fx(by)}" style="stroke:var(--faint);stroke-width:1.3;stroke-dasharray:7 4.4;opacity:.42"/>`;
      if (B.drawK != null && B.drawK < 1) svg += B.drawK > 0 ? dash(x0, lerp(x0, x1, B.drawK)) : "";
      else if (motifs.length) svg += dash(x0, gapL) + dash(gapR, x1);
      else svg += dash(x0, x1);
      motifs.forEach((m, i) => { svg += motifSVG(mx0 + i * step, by, m.c, m.k == null ? 1 : m.k); });
    }
    // gutter (editor.rs gutter marks): needle while reading, a tacking stitch once read,
    // a knot and its number beside a paragraph with an open question
    for (const b of P.blocks) {
      if (!b.gutter) continue;
      const fl = firstLine(b.id);
      if (!fl) continue;
      const gx = colL - 26, cy = fl.mid - 2;
      const g = b.gutter;
      const op = markOp * (g.op == null ? 1 : g.op);
      if (op <= 0.01) continue;
      svg += `<g opacity="${fx(op)}" transform="translate(0 ${fx(lift)})">`;
      if (g.type === "needle") svg += NEEDLE(gx, cy + 1, (t % 0.9) / 0.9, "var(--graphite)");
      else if (g.type === "tack") svg += capsule(gx - 4, cy + 1, gx + 4, cy + 1, 1.8, "color-mix(in srgb,var(--faint) 75%,transparent)");
      else if (g.type === "knot") svg += knotSVG(gx, cy - 1, 4.2, cssCol(g.c, voice), g.k == null ? 1 : g.k);
      svg += "</g>";
    }
    const ov = host.querySelector("svg.ov"), tagsEl = host.querySelector(".tags");
    let tagsHTML = "";
    const gutNums = [];
    for (const b of P.blocks) {
      if (b.gutter && b.gutter.type === "knot" && b.gutter.n) {
        const fl = firstLine(b.id);
        if (fl) gutNums.push(`<span class="gut-n" style="left:${colL - 26}px;top:${fx(fl.mid + 6 + lift)}px;opacity:${fx(markOp * (b.gutter.op == null ? 1 : b.gutter.op))}">${b.gutter.n}</span>`);
      }
    }
    // threads: tabs are stacked in reading order on each side, each pushed down only as far
    // as it needs to clear the one above (margin.rs `place`)
    geo.tags = {};
    const items = [];
    for (const th of P.threads || []) { const lines = lineRects(th.block, th.id); if (lines.length) items.push({ th, lines }); }
    const placedY = {}, bottoms = { left: -1e9, right: -1e9 };
    items.slice().sort((a, b) => a.lines[0].y - b.lines[0].y || (a.th.order || 0) - (b.th.order || 0)).forEach(({ th, lines }) => {
      if (th.noTag || th.tag === false && th.keepSlot !== true) return;
      const side = th.side || "right";
      const top = Math.max(lines[0].y - 3, bottoms[side]);
      placedY[th.id] = top;
      bottoms[side] = top + TAB_H + 14;
    });
    for (const { th, lines } of items) {
      const c = cssCol(th.c, voice);
      const op = (th.op == null ? 1 : th.op) * markOp;
      const seed = Number(seedOf(th.id) & 0xffffffn);
      // running stitch under the words (a correction starts with a cross-stitch)
      const sew = th.sew == null ? 1 : th.sew;
      const unpick = th.unpick || 0; // a snipped thread's stitches come out one by one
      let all = [];
      lines.forEach((L, li) => {
        const y = L.y + (L.h || 28) - 3.2;
        runningStitch(L.x, L.r, y, seed + li * 7, li === 0 && th.kind === "correction").forEach((s) => all.push(s));
      });
      const n = all.length;
      let headX = null, headY = null;
      if (!P.marksOnly || true) {
        all.forEach((s, i) => {
          const k = clamp(sew * (n + 1) - i);
          const gone = unpick > 0 && i / n >= 1 - unpick;
          if (k <= 0 || gone) return;
          const x1 = lerp(s.x0, s.x1, k);
          svg += `<g transform="translate(0 ${fx(lift)})">` + capsule(s.x0, s.y, x1, s.y, 1.8, c, op * (th.stitchOp == null ? 1 : th.stitchOp)) + "</g>";
          if (k < 1 || i === Math.floor(sew * (n + 1))) { headX = x1; headY = s.y; }
        });
        if (th.kind === "correction") {
          const L0 = lines[0], cx = L0.x + 3.5, cy = L0.y + (L0.h || 28) - 3.2;
          const k = clamp(sew * (n + 1) * 1.6);
          if (!(unpick > 0.98)) svg += `<g opacity="${fx(op * (th.stitchOp == null ? 1 : th.stitchOp))}" transform="translate(0 ${fx(lift)})">${crossSVG(cx, cy, 3.5, c, clamp(k * 2), clamp(k * 2 - 1))}</g>`;
        }
        if (th.wash) svg = `<g transform="translate(0 ${fx(lift)})">${lines.map((L) => `<rect x="${fx(L.x - 1)}" y="${fx(L.y + 1)}" width="${fx(L.r - L.x + 2)}" height="${fx((L.h || 28) - 2)}" rx="2" style="fill:var(--wash)"/>`).join("")}</g>` + svg;
        if (th.needle && headX != null && sew < 1) svg += `<g transform="translate(0 ${fx(lift)})">${NEEDLE(headX + 2, headY - 7, (t * 2.2) % 1, "var(--graphite)")}</g>`;
      }
      // marks only: a small ? after the passage instead of a tag
      if (P.marksOnly) {
        const Lz = lines[lines.length - 1];
        const k = th.markK == null ? 1 : th.markK;
        svg += knotSVG(Lz.r + 5, Lz.y + 6, 3.6, c, k * op);
        continue;
      }
      if (th.noTag) continue;
      // the tag: folded tab, beside the first line, in its margin
      const side = th.side || "right";
      const L0 = lines[0];
      const lineH = L0.h || 28;
      const off = th.offset || [0, 0];
      let tx = side === "right" ? colR + G.gap : colL - G.lgap - TAB_W;
      let ty = placedY[th.id] != null ? placedY[th.id] : L0.y - 3;
      tx += off[0]; ty += off[1];
      if (th.tabXY) { tx = th.tabXY[0]; ty = th.tabXY[1]; }
      const right = side === "right";
      // the hole at the column edge, level with the first stitched row
      const hole = right ? [colR + 12, L0.y + lineH - 3.2] : [colL - 12, L0.y + lineH - 3.2];
      const loop = right ? [tx + 13, ty + 13] : [tx + TAB_W - 13, ty + 13];
      th._hole = hole; th._loop = loop;
      geo.tags[th.id] = { x: tx, y: ty, w: TAB_W, h: TAB_H, hole, loop, lines, side };
      const tagOp = op * (th.tagOp == null ? 1 : th.tagOp);
      // the rope
      const ropeOp = (th.ropeOp == null ? 0.26 : th.ropeOp) * op;
      if (th.rope !== false && ropeOp > 0.005) {
        let rope;
        const R0 = th.ropeTrack;
        if (R0 && R0.offsetFn) {
          const base = [loop[0] - off[0], loop[1] - off[1]];
          rope = simRope(th.id, hole, loop, R0.t0, t, { ...R0, b: (s) => { const o = R0.offsetFn(s); return [base[0] + o[0], base[1] + o[1]]; } });
        } else if (R0) rope = simRope(th.id, hole, loop, R0.t0, t, R0);
        else rope = restRope(hole, loop);
        svg += `<g transform="translate(0 ${fx(lift)})">` + `<circle cx="${fx(hole[0])}" cy="${fx(hole[1])}" r="1.5" style="fill:rgba(var(--shade),.5)" opacity="${fx(Math.min(1, ropeOp * 3))}"/>` + ropeSVG(rope, c, ropeOp) + "</g>";
        geo.tags[th.id].ropeEnd = rope.end();
        if (th.knotAt != null) {
          const e = rope.end();
          svg += knotSVG(e[0], e[1], 3.2, c, knotScale(t - th.knotAt));
        }
      }
      if (th.tag === false || tagOp <= 0.005) continue;
      const tagY = ty + lift;
      tagsHTML += tagHTML(th, tx, tagY, c, tagOp, G, t);
    }
    ov.innerHTML = svg;
    tagsEl.innerHTML = tagsHTML + gutNums.join("");
    // panels: measure their natural height and grow them out of the tab
    tagsEl.querySelectorAll(".panel").forEach((pn) => {
      const inner = pn.querySelector(".in");
      const natural = inner.offsetHeight;
      const e = parseFloat(pn.dataset.e);
      const h = TAB_H + (Math.max(natural, TAB_H) - TAB_H) * e;
      pn.style.height = fx(h) + "px";
      const id = pn.dataset.id;
      if (geo.tags[id]) { geo.tags[id].panel = { h, natural, w: parseFloat(pn.style.width), x: parseFloat(pn.style.left) + parseFloat(pn.parentNode.style.left), y: parseFloat(pn.parentNode.style.top) }; }
      // a tag that would hang below the window slides up (tag_panel `lift`)
      const tg = pn.parentNode, y0 = parseFloat(tg.style.top);
      const below = 860 - 8 - y0;
      if (natural > below && pn.dataset.nolift !== "1") { const up = Math.min(natural - below, Math.max(0, y0 - 44)) * e; pn.style.top = fx(-up) + "px"; }
    });
    // menus anchored to words (selection toolbar, ask box, mind/mode menus)
    if (P.menus && P.menus.length) host.querySelector(".menus").innerHTML = P.menus.map((m) => menuHTML(m, geo, P)).join("");
    return geo;
  }

  /** A tag: the folded tab and, when it unfolds, the panel growing out of it. */
  function tagHTML(th, x, y, c, op, G, t) {
    const right = (th.side || "right") === "right";
    const T = th.tagState || {};
    const u = clamp(T.unfold || 0);
    const glyph = th.tied ? `<span class="k"></span>` : `<span class="g">${th.kind === "correction" ? "×" : "?"}</span>`;
    const num = th.num ? `<span class="n">${th.num}</span>` : "";
    let s = `<div class="tg" style="left:${fx(x)}px;top:${fx(y)}px;opacity:${fx(op)};--tc:${c};${T.z ? "z-index:" + T.z : ""}">` +
      `<div class="tabbox"></div><i class="ring" style="${right ? "left:9px" : "right:9px"}"></i>` +
      `<div class="lbl" style="${right ? "left:22px" : "right:22px"}">${right ? num + glyph : num + glyph}</div>`;
    if (u > 0.01) {
      const e = E.out(u);
      const width = T.width || G.margin;
      const w = TAB_W + (width - TAB_W) * e;
      let openRight = T.openRight != null ? T.openRight : right;
      const left = openRight ? 0 : TAB_W - w;
      const ringX = (right ? 13 : TAB_W - 13) - left;
      const words = clamp((u - 0.32) / 0.68);
      const shadow = `0 ${fx(1 + 3 * e)}px ${fx(4 + 10 * e)}px rgba(var(--shade),${fx(0.2 + 0.18 * e)})`;
      const pad = ringX < w / 2 ? `padding-left:${fx(ringX + 1)}px` : `padding-right:${fx(w - ringX + 1)}px`;
      s += `<div class="panel" data-id="${th.id}" data-e="${fx(e)}"${T.nolift ? ' data-nolift="1"' : ""} style="left:${fx(left)}px;width:${fx(w)}px;height:${TAB_H}px;border-radius:${fx(8 + 2 * e)}px;box-shadow:${shadow};z-index:5">` +
        `<i class="pr" style="left:${fx(ringX - 4)}px"></i>` +
        `<div class="in" style="width:${fx(width)}px;left:${fx(openRight ? 0 : w - width)}px;opacity:${fx(words)}">` +
        `<div class="hd" style="${pad}"><span class="who"><b>${esc(T.mind || "Inkling")}</b><span class="k">· ${esc(T.kindLabel || th.kind)}</span></span>${th.num ? `<span class="n">${th.num}</span>` : ""}</div>` +
        (T.body || "") + `</div></div>`;
    }
    s += `</div>`;
    return s;
  }

  // ---------------------------------------------------------------- tag bodies
  const paras = (s) => s.split(/\n\n+/).map((p) => `<p>${esc(p)}</p>`).join("");
  const B = {
    msg: (text) => `<div class="m">${paras(text)}</div>`,
    you: (text) => `<div class="you"><span>You</span><span>${esc(text)}</span></div>`,
    think: (label, t) => `<div class="think">${label ? `<span>${esc(label)}</span>` : ""}<span class="dot" style="width:5px;height:5px;opacity:${fx(0.3 + 0.6 * (0.5 + 0.5 * Math.sin((t / 1.1) * Math.PI * 2)))}"></span></div>`,
    reply: (text, focus, caretOn) => `<div class="reply"><span class="tx">${text ? esc(text) : '<span class="ph">Write back…</span>'}${focus && caretOn ? '<span class="icaret"></span>' : ""}</span>${icon("send", 13, "var(--graphite)")}</div>`,
    acts: (tied, hov) => `<div class="acts"><span class="${hov === "resolve" ? "hov" : ""}">${tied ? "Reopen" : "Resolve"}</span><span class="${hov === "dismiss" ? "hov" : ""}">Dismiss</span><span class="grow"></span><span class="${hov === "note" ? "hov" : ""}">Make a note</span><span class="${hov === "copy" ? "hov" : ""}">${icon("copy", 12, hov === "copy" ? "var(--ink)" : "var(--faint)")}</span></div>`,
    short: (text) => `<div class="short">${esc(text.length > 64 ? text.slice(0, 64) + "…" : text)}</div>`,
    replies: (n) => `<div class="faint">${n === 1 ? "1 reply" : n + " replies"}</div>`,
  };

  // ---------------------------------------------------------------- menus
  function menuHTML(m, geo, P) {
    const G = geo.G;
    if (m.type === "toolbar") {
      // editor.rs toolbar: block kinds, marks, text colours, then the asks; it sits in the
      // right margin beside the selection
      const lines = geo.lineRects(m.block, m.anchor);
      const y = lines.length ? lines[0].y - 4 : 200;
      const kinds = ["T", "H1", "H2", "H3", "❝", "•", "1."].map((k, i) => `<span class="${m.kind === i ? "on" : ""}${m.hovKind === i ? " hov" : ""}">${k}</span>`).join("");
      const marks = ["<b>B</b>", "<i>I</i>", "<u>U</u>", "<s>S</s>", "&lt;/&gt;"].map((k) => `<span>${k}</span>`).join("");
      const cols = ["var(--ink)", "#6f6862", "#6e4b2e", "#8f6214", "#a63f32", "#4f7350", "#2e3f7a", "#6b3e62"].map((c) => `<span style="color:${c};font-weight:500">A</span>`).join("");
      const act = (k, b, s) => `<div class="act${m.hov === k ? " hov" : ""}"><b>${b}</b><small>${s}</small></div>`;
      return `<div class="card tbar" style="left:${G.colLeft + G.col + G.gap}px;top:${fx(y)}px;opacity:${fx(m.op == null ? 1 : m.op)};transform:translateY(${fx((1 - (m.op == null ? 1 : m.op)) * 4)}px)">` +
        `<div class="row">${kinds}</div><div class="row">${marks}</div><div class="row">${cols}</div><hr>` +
        act("find", "Find a loose thread", "⌘↵") + act("ask", "Ask Inkling", "Your own question") + act("sketch", "Sketch it", "Stitched out step by s…") + `</div>`;
    }
    if (m.type === "ask") {
      const lines = geo.lineRects(m.block, m.anchor);
      const y = lines.length ? lines[0].y - 4 : 200;
      const chips = m.chips.map((ch) => {
        const c = `var(--${ch.c})`;
        return `<span class="chip${ch.on ? " on" : ""}" style="${ch.on ? `border-color:color-mix(in srgb,${c} 70%,transparent);background:color-mix(in srgb,${c} 8%,var(--card))` : ""}${ch.hov ? ";color:var(--ink)" : ""}"><span class="dot" style="width:6px;height:6px;background:${c}"></span>${esc(ch.name)}</span>`;
      }).join("");
      const n = m.chips.filter((c) => c.on).length;
      return `<div class="card askbox" style="left:${G.colLeft + G.col + G.gap}px;top:${fx(y)}px;width:${G.margin + 10}px;opacity:${fx(m.op == null ? 1 : m.op)}">` +
        `<div class="q"><span class="dot" style="width:6px;height:6px"></span><span style="flex:1">${m.text ? esc(m.text) : '<span class="ph">Ask about these words…</span>'}${m.caret ? '<span class="icaret"></span>' : ""}</span></div>` +
        `<div class="chips">${chips}</div><div class="hint">${n > 1 ? `${n} minds will answer side by side · Enter to ask · Esc to cancel` : "Pick up to three minds · Enter to ask · Esc to cancel"}</div></div>`;
    }
    if (m.type === "mind") {
      const items = m.minds.map((x, i) => (i === 1 ? `<div class="lab">Yours</div>` : "") +
        `<div class="mi${m.hov === i ? " hov" : ""}"><div class="r"><span class="dot" style="width:5px;height:5px;background:var(--ink)"></span><span class="grow">${esc(x.name)}</span>${x.on ? icon("check", 12, "var(--ink)") : ""}</div>${x.d ? `<small style="padding-left:11px">${esc(x.d)}</small>` : ""}</div>`).join("");
      return `<div class="card menu" style="left:${G.colLeft - 6}px;top:${fx(G.header + 28 - (P.scroll || 0))}px;width:232px;opacity:${fx(m.op == null ? 1 : m.op)}">${items}<div class="foot"><span>+ New mind</span><span>Manage</span></div></div>`;
    }
    if (m.type === "mode") {
      const it = (k, ic, title, body) => `<div class="mi${m.hov === k ? " hov" : ""}"><div class="r">${ic}<span class="grow">${title}</span>${m.cur === k ? icon("check", 12, "var(--ink)") : ""}</div><small style="padding-left:18px">${body}</small></div>`;
      return `<div class="card menu" style="left:${G.colLeft + 120}px;top:${fx(G.header + 28 - (P.scroll || 0))}px;width:220px;opacity:${fx(m.op == null ? 1 : m.op)}">` +
        it("read", icon("bulb", 12, "var(--graphite)"), "Reads along", "Reads along and speaks up when something’s worth a second look.") +
        it("ask", icon("msg", 11, "var(--faint)"), "Waits to be asked", "Quiet until you ask: say its name, select words, or open the chat.") + `</div>`;
    }
    return "";
  }

  // ================================================================ the window
  /** Renders the whole window into `host`. Returns the page geometry (or null). */
  function renderWindow(host, W, t, z) {
    const voice = W.voice || "madder";
    host.style.setProperty("--tc", `var(--${voice})`);
    let html = `<div class="desk" style="left:-40px;top:-40px;width:1400px;height:940px"></div><div class="pagewrap"><div class="page"></div></div>`;
    // chrome: traffic lights, sidebar toggle, Chat (app.rs `chrome_layer`)
    const cop = W.chromeOp == null ? 1 : W.chromeOp;
    html += `<div class="lights"${W.lights === false ? ' style="filter:grayscale(1) brightness(1.3);opacity:.6"' : ""}><i></i><i></i><i></i></div>`;
    html += `<div class="chrome" style="opacity:${fx(cop)}">${W.sideK > 0.5 ? "" : `<svg class="sidetoggle" viewBox="0 0 20 20" style="fill:none;stroke:var(--graphite);stroke-width:1.6;stroke-linecap:round">${ICON.menu}</svg>`}` +
      (W.chat && W.chat.k > 0.01 ? "" : `<div class="chat-pill${W.chatHov ? " hov" : ""}" style="${W.chatHov ? "border-color:var(--faint)" : ""}"><span class="dot" style="width:5px;height:5px"></span>Chat</div>`) + `</div>`;
    if (W.chat && W.chat.k > 0.01) html += chatHTML(W.chat, t);
    if (W.sideK > 0.005) html += sideHTML(W.side || {}, W.sideK);
    if (W.overlay) html += W.overlay;
    if (W.toast && W.toast.op > 0.01) html += `<div class="toast" style="opacity:${fx(W.toast.op)};transform:translate(-50%,${fx((1 - W.toast.op) * 8)}px)">${esc(W.toast.text)}</div>`;
    if (W.undo && W.undo.op > 0.01) html += `<div class="undo" style="opacity:${fx(W.undo.op)};transform:translateY(${fx((1 - W.undo.op) * 8)}px)"><span>Thread dismissed</span><b>Undo</b><span>⌥⌘Z</span></div>`;
    if (W.listen && W.listen.op > 0.01) html += `<div class="listen" style="opacity:${fx(W.listen.op)}"><span class="dot" style="width:7px;height:7px;opacity:${fx(0.35 + 0.65 * (0.5 + 0.5 * Math.sin((t / 0.9) * Math.PI * 2)))}"></span><span>${esc(W.listen.text || "Listening…")}</span></div>`;
    if (W.blots) html += `<svg class="ov" width="1320" height="860" style="z-index:45">${W.blots}</svg>`;
    host.innerHTML = html;
    let geo = null;
    if (W.page) geo = renderPage(host.querySelector(".page"), W.page, t, z);
    return geo;
  }

  function sideHTML(S, k) {
    const e = E.out(k);
    const row = (ic, label, o = {}) => `<div class="row${o.on ? " on" : ""}${o.hov ? " hov" : ""}${o.dim ? " dim" : ""}${o.fold ? " fold" : ""}" style="${o.indent ? `padding-left:${8 + o.indent}px` : ""}">${ic}<span class="grow">${esc(label)}</span>${o.dot ? '<span class="dot" style="width:4px;height:4px;opacity:.7"></span>' : ""}${o.keys ? `<small>${o.keys}</small>` : ""}</div>`;
    const F = (S.folders || []).map((f) => row(icon(f.open ? "folderOpen" : "folder", 12, "var(--faint)"), f.name, { fold: true, hov: f.hov }) +
      (f.open ? f.notes.map((n) => row(icon("file", 12, "var(--faint)"), n.title, { indent: 14, on: n.on, dot: n.dot, hov: n.hov })).join("") : "")).join("");
    const loose = (S.loose || []).map((n) => row(icon("file", 12, "var(--faint)"), n.title, { on: n.on, dot: n.dot, hov: n.hov })).join("");
    return `<div class="dim" style="background:color-mix(in srgb,var(--cloth) 50%,transparent);opacity:${fx(e)}"></div>` +
      `<div class="side" style="transform:translateX(${fx(-240 * (1 - e))}px)"><div class="hdr"><span class="wm">${WM_HTML(18)}</span>${icon("x", 13, "var(--graphite)")}</div>` +
      row(icon("search", 12, "var(--faint)"), "Find or start a note", { dim: true, keys: "⌘K" }) + row(icon("plus", 12, "var(--faint)"), "New note", { dim: true, hov: S.hovNew }) +
      `<div style="margin-top:10px">${F}${loose}${S.extra || ""}${row(icon("folderPlus", 12, "var(--faint)"), "New folder", { dim: true, hov: S.hovFolder })}</div>` +
      `<div class="foot"><div class="who">${esc(S.who || "you@example.com")}</div><div class="sv">All notes saved on this Mac</div>` +
      row("", "Copy this note", { keys: "⌘⇧C", hov: S.hovCopy }) + row("", "Export this note", { keys: "⌘⇧E" }) + row("", "Hide threads", { keys: "⌘⇧." }) + row("", "Customize Inkling", { keys: "⌘," }) + row("", "Minds") + row("", "Model") + `</div></div>`;
  }

  function chatHTML(C, t) {
    const e = E.out(C.k);
    let log = "";
    if (C.empty) {
      log += `<div class="hello">I’ve read the whole note and its threads. Ask me anything about it.</div><div class="pills">` +
        ["What am I still misunderstanding?", "What have we figured out so far?", "What should I work on next?"].map((q, i) => `<span class="pill${C.hovPill === i ? " hov" : ""}">${q}</span>`).join("") + `</div>`;
    }
    if (C.me) log += `<div class="me">${esc(C.me)}</div>`;
    if (C.think) log += `<div>${B.think("", t)}</div>`;
    if (C.ans) log += `<div class="ans" style="${C.ansShift ? `transform:translateY(${fx(-C.ansShift)}px)` : ""}">${paras(C.ans)}</div>`;
    return `<div class="chatp" style="transform:translateX(${fx(320 * (1 - e))}px)"><div class="hd"><div class="r"><span class="dot" style="width:6px;height:6px"></span><span class="grow">Inkling</span>` +
      `<span class="b">Copy</span><span class="b"${C.hovClear ? ' style="color:var(--ink)"' : ""}>Clear</span><span class="b">${icon("x", 12, "var(--graphite)")}</span></div>` +
      `<small>On “${esc(C.title || "")}”. Reading this note: ${esc(C.threads || "")}</small></div><div class="log">${log}</div>` +
      `<div class="ft"><span class="tx">${C.input ? esc(C.input) + (C.caret ? '<span class="icaret"></span>' : "") : '<span class="ph">Ask about this note…</span>'}</span><span class="send">${icon("arrowUp", 12, "var(--paper)", 2)}</span></div></div>`;
  }

  /** The wordmark (ui.rs `wordmark`): "ınkling" in Fraunces Display, a French knot for the dot. */
  let I_ADV = 0.255; // the advance of "ı" in Fraunces Display Medium, per pt of size (measured at load)
  function measureWordmark() {
    const c = document.createElement("canvas").getContext("2d");
    c.font = "500 100px 'Fraunces Display'";
    I_ADV = c.measureText("ı").width / 100;
  }
  function WM_HTML(size, opts) {
    const h = size * 1.3;
    const kx = I_ADV * size * 0.47, ky = h * 0.5 - size * 0.6;
    return `<span style="position:relative;display:inline-block;height:${fx(h)}px;font:500 ${size}px/${fx(h)}px 'Fraunces Display';color:var(--ink)">ınkling` +
      `<svg style="position:absolute;left:0;top:0;overflow:visible" width="1" height="1">${knotSVG(kx, ky, size * 0.13, "var(--voice)", opts && opts.knot != null ? opts.knot : 1)}</svg></span>`;
  }

  window.APP = { measureWordmark, iAdv: () => I_ADV, fx, clamp, lerp, prog, esc, E, spring, rng, applyTheme, thread, mindThread, seedOf, capsule, runningStitch, crossSVG, knotSVG, knotScale, motifSVG,
    NEEDLE, Rope, restRope, simRope, ropeSVG, icon, ICON, GEO, TAB_W, TAB_H, markup, renderPage, renderWindow, B, paras, WM_HTML, THREADS, THREAD_ORDER, jitter };
})();

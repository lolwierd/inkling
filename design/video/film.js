/* Inkling: a walkthrough film in the Loose Threads look.
 *
 * Every frame is renderAt(t), a pure function of time: no timers, no CSS animation, no
 * state carried from one frame to the next that could change what a frame looks like.
 * Rope physics is the one simulation, and it is stepped at a fixed rate from the start of
 * each phase, so frame 1234 is the same whether a worker starts there or arrives there.
 *
 * Inkling's words come from data/real.js (the brain's real output, see tools/real.ts).
 * The stitch, knot and rope code is the design board's (design/directions/threads.html).
 */
(function () {
  "use strict";
  const REAL = window.REAL, ASSETS = window.ASSETS;
  const C = REAL.calls, NOTE = REAL.note;
  const FPS = 30, DUR = 98;
  const $ = (s, r) => (r || document).querySelector(s);
  const $$ = (s, r) => Array.from((r || document).querySelectorAll(s));
  const fx = (v) => Math.round(v * 100) / 100;
  const clamp = (v, a = 0, b = 1) => Math.max(a, Math.min(b, v));
  const lerp = (a, b, k) => a + (b - a) * k;
  const prog = (t, t0, t1) => (t1 <= t0 ? (t >= t1 ? 1 : 0) : clamp((t - t0) / (t1 - t0)));
  const easeIO = (k) => (k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2);
  const easeOut = (k) => 1 - Math.pow(1 - k, 3);
  const easeIn = (k) => k * k * k;
  const sine = (k) => 0.5 - 0.5 * Math.cos(Math.PI * k);
  const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  const within = (t, a, b) => t >= a && t < b;

  // A fixed sequence of "random" numbers per seed, so a redraw never moves a stitch.
  function rng(seed) {
    let s = (seed * 2654435761) >>> 0 || 1;
    return () => { s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; return (s % 100000) / 100000; };
  }

  // ======================================================================== words
  const TXT = {
    title: NOTE.title,
    p1: NOTE.paras[0], p2: NOTE.paras[1], p3: NOTE.paras[2], p4: NOTE.paras[3],
    p2fixed: NOTE.p2Fixed,
    p5: NOTE.call,
    correction: C.read_p2.output.text,
    settle: C.settle.output.text,
    question: C.probe_p3.output.text,
    reply: NOTE.reply,
    answer: C.reply.output.text,
    roomTarget: NOTE.room.target, roomQ: NOTE.room.question,
    brief: NOTE.brief,
    chatQ: NOTE.chat, chatA: C.chat.output.text,
    call: C.call_by_name.output.text,
    svg: C.draw.output.svg,
  };
  const EDIT1 = { from: "5:30", to: "4:55" };
  const EDIT2 = { from: " and hold 5:30 per km on every run", to: ", two of them at 4:55 per km" };

  // ======================================================================== timeline (seconds)
  // 80 bpm: a bar is 3 s. Knots land on downbeats: bar 16 (45 s) and bar 21 (60 s).
  const T = {
    sew0: 0.35, glyphGap: 0.4, glyphDur: 0.62, fill0: 3.05, fill1: 3.85, knotPull: 4.28, knotPop: 4.5,
    line0: 5.1, line1: 5.7, lineOut0: 6.9, lineOut1: 7.3,
    fly0: 7.1, fly1: 8.55, win0: 7.75, win1: 8.95,
    title: [9.3, 10.6], p1: [11.0, 14.0], p2: [14.3, 17.7], p3: [18.0, 19.6], p4: [19.9, 21.9],
    enter: { p1: 10.75, p2: 14.15, p3: 17.85, p4: 19.75, p5: 80.85 },
    read: { p1: [14.4, 15.5], p2: [18.1, 19.4], p3: [19.95, 21.0], p4: [22.1, 22.9] },
    // thread 1: the correction
    t1Arrive: 19.4, t1Sew: [23.3, 24.6], t1Carry: [24.65, 25.15], t1Open: 24.9, t1Hover: 27.55,
    skClick: 28.9, skThink: [29.0, 29.85], skSteps: [[29.9, 31.15], [31.5, 32.75], [33.1, 34.35], [34.7, 35.95]],
    e1Sel: 38.1, e1Type: [38.45, 38.85], e2Drag: [39.5, 40.05], e2Type: [40.25, 41.5],
    t1Reread: [41.9, 42.7], t1Settle: [42.7, 43.85], t1Tie: 44.4, t1Knot: 45.0, t1Fold: 45.2, t1Motif: 45.25, t1Fade: [46.9, 47.35],
    // thread 2: find a loose thread
    t2Drag: [47.35, 48.1], t2Bar: 48.25, t2Click: 49.15, t2Think: [49.25, 50.1], t2Sew: [50.1, 50.9], t2Carry: [50.95, 51.45], t2Text: 51.2,
    t2ReplyClick: 54.3, t2Type: [54.6, 56.6], t2Send: 56.7, t2Read: [56.75, 57.5], t2Answer: [57.5, 58.8],
    t2Tie: 59.4, t2Knot: 60.0, t2Fold: 60.2, t2Motif: 60.25, t2Fade: [61.9, 62.35],
    // ask the room
    roomDrag: [61.3, 61.9], roomBar: 62.0, roomClick: 62.75, askBox: 62.85, chipSkeptic: 63.3, chipTutor: 63.7,
    roomType: [64.0, 65.3], roomSend: 65.45, roomSew: [65.55, 66.15], roomCarry: [[66.2, 66.7], [66.3, 66.8], [66.4, 66.9]],
    roomStream: { inkling: [66.85, 69.5], skeptic: [66.95, 69.1], tutor: [67.05, 69.85] }, roomFold: 72.3,
    // brief, chat, call by name
    briefClick: 72.5, briefOpen: [72.55, 72.8], briefType: [72.9, 74.5], briefSet: 74.65,
    chatKey: 75.0, chatIn: [75.0, 75.35], chatType: [75.6, 76.5], chatSend: 76.6, chatThink: [76.65, 77.0], chatStream: [77.0, 80.0], chatOut: [80.4, 80.7],
    p5: [81.0, 82.5], callArrive: 83.0, callSew: [83.1, 83.7], callCarry: [83.75, 84.25], callThink: [83.8, 84.2], callStream: [84.2, 86.4],
    // hide threads, lamplight, close
    hideClick: 87.9, hide: [87.9, 88.15], showClick: 89.7, show: [89.7, 89.95],
    lamp: [90.2, 91.1], winOut: [92.6, 93.4], fadeOut: [96.9, 98.0],
    icon: [93.4, 94.05], closeWm: [94.0, 94.45], closeKnot: 94.5, closeLine: [94.95, 95.6],
  };

  // ======================================================================== theme
  const TOK = {
    light: { bg: "#EFE8DB", surface: "#FBF8F2", shelf: "#F3EDE2", raised: "#FFFDF8", fg: "#26232B", fg2: "#57505C", fg3: "#6F6862", line: "#E3DACB", line2: "#D3C7B3",
      madder: "#A63F32", indigo: "#2E3F7A", ochre: "#A8741A", sage: "#4F7350", plum: "#6B3E62", tack: "#8C8276", steel: "#A7AEB7",
      sel: [46, 63, 122, 0.16], chip: [38, 35, 43, 0.06], shadow: [40, 26, 12] },
    dark: { bg: "#18161A", surface: "#1F1D22", shelf: "#1A181D", raised: "#26232A", fg: "#EDE6DA", fg2: "#BDB4A8", fg3: "#A39A8F", line: "#332F36", line2: "#47414D",
      madder: "#D9705E", indigo: "#8C9BE0", ochre: "#D9A548", sage: "#8DB88E", plum: "#C08AB4", tack: "#948A80", steel: "#8C939C",
      sel: [140, 155, 224, 0.24], chip: [237, 230, 218, 0.08], shadow: [0, 0, 0] },
  };
  const hexRGB = (h) => { const n = parseInt(h.slice(1), 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; };
  function applyTheme(k) {
    const st = $("#stage").style, L = TOK.light, D = TOK.dark;
    for (const key in L) {
      const a = L[key], b = D[key];
      if (typeof a === "string") {
        const A = hexRGB(a), B = hexRGB(b);
        st.setProperty("--" + key, `rgb(${A.map((v, i) => Math.round(lerp(v, B[i], k))).join(",")})`);
      } else if (a.length === 4) {
        st.setProperty("--" + key, `rgba(${[0, 1, 2].map((i) => Math.round(lerp(a[i], b[i], k))).join(",")},${fx(lerp(a[3], b[3], k))})`);
      } else {
        st.setProperty("--" + key, [0, 1, 2].map((i) => Math.round(lerp(a[i], b[i], k))).join(","));
      }
    }
    $("#desk-dark").style.opacity = String(fx(k));
    $("#desk-in-dark").style.opacity = String(fx(k));
  }
  function themeAt(t) {
    return easeIO(prog(t, T.lamp[0], T.lamp[1]));
  }

  // ======================================================================== stitches (from the board)
  function lens(x0, y0, x1, y1, w) {
    const dx = x1 - x0, dy = y1 - y0, L = Math.hypot(dx, dy) || 1, nx = -dy / L, ny = dx / L;
    const mx = (x0 + x1) / 2, my = (y0 + y1) / 2, h = w / 2, he = w * 0.2, k = 1.5;
    return `M${fx(x0 + nx * he)} ${fx(y0 + ny * he)}Q${fx(mx + nx * h * k)} ${fx(my + ny * h * k)} ${fx(x1 + nx * he)} ${fx(y1 + ny * he)}` +
      `L${fx(x1 - nx * he)} ${fx(y1 - ny * he)}Q${fx(mx - nx * h * k)} ${fx(my - ny * h * k)} ${fx(x0 - nx * he)} ${fx(y0 - ny * he)}Z`;
  }
  /** One stitch, `k` of the way sewn (0..1): it grows from its first hole. */
  function stitchSVG(s, k, opacity) {
    if (k == null) k = 1;
    if (k <= 0) return "";
    const g = clamp(k / 0.85), x1 = s.x0 + (s.x1 - s.x0) * g, y1 = s.y0 + (s.y1 - s.y0) * g;
    const dx = x1 - s.x0, dy = y1 - s.y0, L = Math.hypot(dx, dy);
    if (L < 0.3) return "";
    let nx = -dy / L, ny = dx / L;
    if (ny > 0 || (ny === 0 && nx > 0)) { nx = -nx; ny = -ny; }
    const o = s.w * 0.2;
    const ax = s.x0 + dx * 0.26 + nx * o, ay = s.y0 + dy * 0.26 + ny * o, bx = s.x0 + dx * 0.68 + nx * o, by = s.y0 + dy * 0.68 + ny * o;
    const op = Math.min(1, k / 0.35) * (opacity == null ? 1 : opacity);
    const c = s.c.startsWith("#") || s.c.startsWith("rgb") ? s.c : `var(--${s.c})`;
    return `<g${op < 0.999 ? ` opacity="${fx(op)}"` : ""}><path d="${lens(s.x0, s.y0, x1, y1, s.w)}" style="fill:${c}"/>` +
      `<path d="M${fx(ax)} ${fx(ay)}L${fx(bx)} ${fx(by)}" style="fill:none;stroke:color-mix(in srgb,${c} 46%,#fff);stroke-width:${fx(s.w * 0.24)};stroke-linecap:round;opacity:.72"/></g>`;
  }
  function crossSVG(cx, cy, s, w, c, k1, k2, op) {
    return stitchSVG({ x0: cx - s, y0: cy + s, x1: cx + s, y1: cy - s, w, c }, k1, op) + stitchSVG({ x0: cx - s, y0: cy - s, x1: cx + s, y1: cy + s, w, c }, k2, op);
  }
  function ribbon(pts, wf) {
    const n = pts.length, L = [], R = [];
    for (let i = 0; i < n; i++) {
      const a = pts[Math.max(i - 1, 0)], b = pts[Math.min(i + 1, n - 1)];
      const dx = b[0] - a[0], dy = b[1] - a[1], ln = Math.hypot(dx, dy) || 1, nx = -dy / ln, ny = dx / ln;
      const w = wf(i / (n - 1)) / 2;
      L.push([pts[i][0] + nx * w, pts[i][1] + ny * w]);
      R.push([pts[i][0] - nx * w, pts[i][1] - ny * w]);
    }
    return "M" + L.concat(R.reverse()).map((p) => fx(p[0]) + " " + fx(p[1])).join("L") + "Z";
  }
  /** French knot: a bead of thread with one coil showing, lit from the top left. */
  function knotSVG(cx, cy, r, c, scale) {
    if (scale == null) scale = 1;
    if (scale <= 0.01) return "";
    const pts = [];
    for (let i = 0; i <= 48; i++) {
      const t = i / 48, a = -2.09 + t * 1.3 * 2 * Math.PI, rad = r * (0.16 + 0.66 * t);
      pts.push([cx + rad * Math.cos(a), cy + rad * Math.sin(a) * 0.92]);
    }
    const hi = ribbon(pts, (t) => r * (0.12 + 0.09 * t) * Math.min(1, t * 5 + 0.2, (1 - t) * 4 + 0.35));
    const col = c.startsWith("#") ? c : `var(--${c})`;
    return `<g transform="translate(${fx(cx)} ${fx(cy)}) scale(${fx(scale)}) translate(${fx(-cx)} ${fx(-cy)})">` +
      `<circle cx="${fx(cx)}" cy="${fx(cy)}" r="${fx(r)}" style="fill:${col}"/>` +
      `<path d="M${fx(cx + r)} ${fx(cy)}A${fx(r)} ${fx(r)} 0 0 1 ${fx(cx - r * 0.5)} ${fx(cy + r * 0.87)}A${fx(r * 0.9)} ${fx(r * 0.9)} 0 0 0 ${fx(cx + r)} ${fx(cy)}Z" style="fill:color-mix(in srgb,${col} 68%,#000);opacity:.7"/>` +
      `<path d="${hi}" style="fill:color-mix(in srgb,${col} 46%,#fff)"/></g>`;
  }
  /** The knot pops in: a small overshoot, then rest (knot_scale in src/note_page.rs). */
  function knotScale(s) {
    if (s <= 0) return 0;
    const k = Math.min(s / 0.2, 1);
    return k < 0.6 ? (k / 0.6) * 1.3 : 1.3 - ((k - 0.6) / 0.4) * 0.3;
  }
  /** Stitches along a polyline. */
  function runAlong(pts, c, w, len, gap, seed) {
    const rand = rng(seed || 5), out = [];
    let on = true, acc = 0, seg = [pts[0]], target = len;
    for (let k = 1; k < pts.length; k++) {
      const a = pts[k - 1], b = pts[k], d = Math.hypot(b[0] - a[0], b[1] - a[1]);
      let used = 0;
      while (used < d - 1e-9) {
        const take = Math.min(d - used, target - acc);
        used += take; acc += take;
        const t = used / d, p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        seg.push(p);
        if (acc >= target - 0.01) {
          if (on) { const s0 = seg[0], s1 = seg[seg.length - 1]; out.push({ x0: s0[0], y0: s0[1], x1: s1[0], y1: s1[1], w, c }); }
          on = !on; acc = 0; seg = [p];
          target = (on ? len : gap) * (0.88 + rand() * 0.24);
        }
      }
    }
    if (on && seg.length > 1) { const s0 = seg[0], s1 = seg[seg.length - 1]; if (Math.hypot(s1[0] - s0[0], s1[1] - s0[1]) > len * 0.4) out.push({ x0: s0[0], y0: s0[1], x1: s1[0], y1: s1[1], w, c }); }
    return out;
  }
  /** The running stitch under a run of words, line by line (threadSVG on the board). */
  function stitchesUnder(lines, seed, colors, opts) {
    const rand = rng(seed || 11);
    const w = 2.3, len = opts && opts.len || 7.4, gap = opts && opts.gap || 4.2, out = [];
    lines.forEach((r, li) => {
      let x = r.x + 0.5 + (li === 0 && opts && opts.lead ? opts.lead : 0);
      const x1 = r.r - 0.5, y = r.b + 0.6;
      while (x < x1 - 1.5) {
        const l = Math.min(len * (0.88 + rand() * 0.24), x1 - x);
        if (l < len * 0.45) break;
        out.push({ x0: x, y0: y + (rand() - 0.5) * 0.9, x1: x + l, y1: y + (rand() - 0.5) * 0.9, w, c: colors[out.length % colors.length] });
        x += l + gap * (0.85 + rand() * 0.3);
      }
    });
    return out;
  }
  const NEEDLE = ASSETS.EL.needle.replace("<svg ", '<svg width="20" height="20" ');
  function needleAt(x, y, t, size) {
    const ph = (t % 0.9) / 0.9, b = Math.sin(ph * Math.PI * 2);
    const s = (size || 20) / 20;
    return `<g transform="translate(${fx(x - 2 * s + b * -1)} ${fx(y - 17 * s + (b + 1) * 1.5)}) scale(${fx(s)})">${NEEDLE}</g>`;
  }

  // ======================================================================== the rope (from the board)
  const ROPE = { POINTS: 26, SLACK: 1.09, SLACK_TIED: 0.995, GRAVITY: 900, GRAVITY_TIED: 20, DAMPING: 0.95, ITERATIONS: 24,
    BEND: 0.3, BEND_REACH: 0.94, STEP_MS: 1000 / 120, SETTLE_STEPS: 360, SWAY: 2, SWAY_MS: 600, TIE_MS: 320 };
  class Rope {
    constructor(a, b, slack, noSettle) {
      this.n = ROPE.POINTS; this.slack = slack == null ? ROPE.SLACK : slack; this.pinB = true;
      this.a = [a[0], a[1]]; this.b = [b[0], b[1]];
      this.len = this.straight() * this.slack;
      this.damping = ROPE.DAMPING;
      this.gravity = this.slack < 1.001 ? ROPE.GRAVITY_TIED : ROPE.GRAVITY;
      this.p = []; this.q = [];
      this.drape();
      if (!noSettle) this.settle(ROPE.SETTLE_STEPS);
    }
    straight() { return Math.hypot(this.b[0] - this.a[0], this.b[1] - this.a[1]); }
    drape() {
      const sag = this.straight() * Math.sqrt(Math.max(0, (3 * (this.slack - 1)) / 8));
      for (let i = 0; i < this.n; i++) {
        const t = i / (this.n - 1);
        const x = this.a[0] + (this.b[0] - this.a[0]) * t, y = this.a[1] + (this.b[1] - this.a[1]) * t + sag * 4 * t * (1 - t);
        this.p[i] = [x, y]; this.q[i] = [x, y];
      }
    }
    setB(b) { this.b = [b[0], b[1]]; this.len = this.straight() * this.slack; }
    setSlack(s) { this.slack = s; this.len = this.straight() * s; }
    free(i) { return i !== 0 && !(i === this.n - 1 && this.pinB); }
    keep(i, j, rest, stiffness, pushOnly) {
      const p = this.p, dx = p[j][0] - p[i][0], dy = p[j][1] - p[i][1], d = Math.hypot(dx, dy) || 1e-6;
      if (pushOnly && d >= rest) return;
      const f = ((d - rest) / d) * stiffness, fi = this.free(i), fj = this.free(j);
      const wi = fi && fj ? 0.5 : fi ? 1 : 0, wj = fi && fj ? 0.5 : fj ? 1 : 0;
      p[i][0] += dx * f * wi; p[i][1] += dy * f * wi; p[j][0] -= dx * f * wj; p[j][1] -= dy * f * wj;
    }
    step() {
      const n = this.n, p = this.p, q = this.q, dt = ROPE.STEP_MS / 1000, g = this.gravity * dt * dt;
      for (let i = 0; i < n; i++) {
        if (!this.free(i)) continue;
        const vx = (p[i][0] - q[i][0]) * this.damping, vy = (p[i][1] - q[i][1]) * this.damping;
        q[i][0] = p[i][0]; q[i][1] = p[i][1]; p[i][0] += vx; p[i][1] += vy + g;
      }
      const seg = this.len / (n - 1);
      for (let k = 0; k < ROPE.ITERATIONS; k++) {
        p[0][0] = this.a[0]; p[0][1] = this.a[1];
        if (this.pinB) { p[n - 1][0] = this.b[0]; p[n - 1][1] = this.b[1]; }
        for (let i = 0; i < n - 1; i++) this.keep(i, i + 1, seg, 1, false);
        for (let i = 0; i < n - 2; i++) this.keep(i, i + 2, 2 * seg * ROPE.BEND_REACH, ROPE.BEND, true);
      }
      p[0][0] = this.a[0]; p[0][1] = this.a[1];
      if (this.pinB) { p[n - 1][0] = this.b[0]; p[n - 1][1] = this.b[1]; }
    }
    settle(steps) { for (let i = 0; i < steps; i++) this.step(); }
    nudge(dir) {
      for (let i = 1; i < this.n - 1; i++) {
        const w = Math.sin((Math.PI * i) / (this.n - 1));
        this.q[i][0] -= dir[0] * ROPE.SWAY * w; this.q[i][1] -= dir[1] * ROPE.SWAY * w;
      }
    }
    path() {
      const P = this.p, n = P.length;
      let d = `M${fx(P[0][0])} ${fx(P[0][1])}`;
      for (let i = 0; i < n - 1; i++) {
        const p0 = P[Math.max(0, i - 1)], p1 = P[i], p2 = P[i + 1], p3 = P[Math.min(n - 1, i + 2)];
        const d1 = Math.sqrt(Math.hypot(p1[0] - p0[0], p1[1] - p0[1])), d2 = Math.sqrt(Math.hypot(p2[0] - p1[0], p2[1] - p1[1])), d3 = Math.sqrt(Math.hypot(p3[0] - p2[0], p3[1] - p2[1]));
        const c1 = [0, 0], c2 = [0, 0];
        for (let k = 0; k < 2; k++) {
          c1[k] = d1 > 1e-6 ? (d1 * d1 * p2[k] - d2 * d2 * p0[k] + (2 * d1 * d1 + 3 * d1 * d2 + d2 * d2) * p1[k]) / (3 * d1 * (d1 + d2)) : p1[k] + (p2[k] - p1[k]) / 3;
          c2[k] = d3 > 1e-6 ? (d3 * d3 * p1[k] - d2 * d2 * p3[k] + (2 * d3 * d3 + 3 * d3 * d2 + d2 * d2) * p2[k]) / (3 * d3 * (d3 + d2)) : p2[k] - (p2[k] - p1[k]) / 3;
        }
        d += `C${fx(c1[0])} ${fx(c1[1])} ${fx(c2[0])} ${fx(c2[1])} ${fx(p2[0])} ${fx(p2[1])}`;
      }
      return d;
    }
  }
  const ropeMemo = new Map();
  const r2 = (v) => Math.round(v * 20) / 20;
  /** A thread hanging at rest between two pins (memoised: a rope at rest is a pure function of its pins). */
  function restingRope(a, b, slack) {
    const key = [r2(a[0]), r2(a[1]), r2(b[0]), r2(b[1]), slack].join(",");
    let r = ropeMemo.get(key);
    if (!r) { r = new Rope([r2(a[0]), r2(a[1])], [r2(b[0]), r2(b[1])], slack); ropeMemo.set(key, r); if (ropeMemo.size > 4000) ropeMemo.clear(); }
    return r;
  }
  /** A thread in motion: simulated at the fixed step from the start of its phase.
   *  phase: carry (the needle takes the thread from the hole to the tag), tie (it pulls
   *  taut), sway (a touch on the tag). Pins are rounded so neighbouring frames agree. */
  const simCache = new Map();
  function movingRope(key, a, b, phase, since, slack) {
    a = [r2(a[0]), r2(a[1])]; b = [r2(b[0]), r2(b[1])];
    const steps = Math.max(0, Math.floor((since * 1000) / ROPE.STEP_MS));
    const ck = `${key}|${phase}|${a}|${b}`;
    let st = simCache.get(ck);
    if (!st || st.k > steps) {
      let r;
      if (phase === "carry") { r = new Rope(a, [a[0] + 0.5, a[1] + 0.5], slack || ROPE.SLACK, true); r.drape(); }
      else r = new Rope(a, b, slack || ROPE.SLACK);
      if (phase === "sway") r.nudge([1, -0.45]);
      st = { r, k: 0 }; simCache.set(ck, st);
    }
    const r = st.r, CARRY = 0.5 * 120, TIE = (ROPE.TIE_MS / ROPE.STEP_MS) | 0;
    for (; st.k < steps; st.k++) {
      const k = st.k + 1;
      if (phase === "carry") {
        const e = easeOut(Math.min(1, k / CARRY));
        r.setB([lerp(a[0] + 0.5, b[0], e), lerp(a[1] + 0.5, b[1], e)]);
      } else if (phase === "tie") {
        const e = easeOut(Math.min(1, k / TIE));
        r.setSlack(ROPE.SLACK + (ROPE.SLACK_TIED - ROPE.SLACK) * e);
        r.gravity = ROPE.GRAVITY + (ROPE.GRAVITY_TIED - ROPE.GRAVITY) * e;
      }
      r.step();
    }
    return r;
  }
  function ropeSVG(r, c, opacity, hole) {
    const d = r.path(), col = `var(--${c})`;
    let s = "";
    if (hole) s += `<circle cx="${fx(hole[0])}" cy="${fx(hole[1])}" r="1.5" style="fill:rgba(var(--shadow),.55)" opacity="${fx(Math.min(1, opacity * 2.5))}"/>`;
    s += `<g opacity="${fx(opacity)}"><path d="${d}" style="fill:none;stroke:rgba(var(--shadow),.17);stroke-width:2.3;stroke-linecap:round" transform="translate(.8 1.2)"/>` +
      `<path d="${d}" style="fill:none;stroke:${col};stroke-width:1.7;stroke-linecap:round"/>` +
      `<path d="${d}" style="fill:none;stroke:color-mix(in srgb,${col} 42%,#fff);stroke-width:.7;stroke-dasharray:2.4 2.1;opacity:.7;stroke-linecap:round" transform="translate(-.15 -.35)"/></g>`;
    return s;
  }

  // ======================================================================== the border band
  const MOTIFS = [[[1, 0], [0, 1], [2, 1], [1, 2]], [[0, 0], [2, 0], [1, 1], [0, 2], [2, 2]], [[1, 0], [0, 1], [2, 1], [1, 2]]];
  function motifSVG(cx, cy, k, c, sewK) {
    const cell = 5.4, s = 1.85, w = 1.4, cells = MOTIFS[k % 3];
    let out = "", i = 0;
    const n = cells.length * 2 + (k % 3 === 2 ? 1 : 0);
    const at = (j) => clamp(sewK * n - j);
    for (const [a, b] of cells) { out += crossSVG(cx + (a - 1) * cell, cy + (b - 1) * cell, s, w, c, at(i), at(i + 1)); i += 2; }
    if (k % 3 === 2) out += knotSVG(cx, cy, 2.2, "ochre", knotScale(at(i) * 0.2));
    return out;
  }

  // ======================================================================== typing
  /** When each character lands: a person's rhythm (a beat after a word, a breath after
   *  punctuation, the odd hesitation), fitted to the window [t0, t1]. */
  function sched(text, t0, t1, seed) {
    const r = rng(seed), w = [];
    for (let i = 0; i < text.length; i++) {
      const c = text[i], prev = i ? text[i - 1] : "";
      let d = 0.72 + r() * 0.56;
      if (prev === " ") d *= 1.25;
      if (/[.,?!:;]/.test(prev)) d *= 2.4;
      if (/[A-Z0-9’]/.test(c)) d *= 1.15;
      if (r() < 0.05) d *= 2.0;
      w.push(d);
    }
    const tot = w.reduce((a, b) => a + b, 0);
    let acc = 0;
    return w.map((d) => { acc += d; return t0 + (acc / tot) * (t1 - t0); });
  }
  const count = (times, t) => { let n = 0; while (n < times.length && times[n] <= t) n++; return n; };
  const S = {
    title: sched(TXT.title, ...T.title, 11), p1: sched(TXT.p1, ...T.p1, 12), p2: sched(TXT.p2, ...T.p2, 13),
    p3: sched(TXT.p3, ...T.p3, 14), p4: sched(TXT.p4, ...T.p4, 15), e1: sched(EDIT1.to, ...T.e1Type, 16), e2: sched(EDIT2.to, ...T.e2Type, 17),
    reply: sched(TXT.reply, ...T.t2Type, 18), roomQ: sched(TXT.roomQ, ...T.roomType, 19), brief: sched(TXT.brief, ...T.briefType, 20),
    chatQ: sched(TXT.chatQ, ...T.chatType, 21), p5: sched(TXT.p5, ...T.p5, 22),
  };
  const lastKey = (times, t) => { const n = count(times, t); return n ? times[n - 1] : -9; };

  /** Paragraph 2 over time: typed, then two edits (5:30 → 4:55 in the race sentence, then
   *  the training sentence). Returns the text and any selection. */
  function p2At(t) {
    let text = TXT.p2.slice(0, count(S.p2, t)), sel = null, caret = text.length;
    if (t < T.e1Sel) return { text, sel, caret };
    const i1 = TXT.p2.indexOf(EDIT1.from, TXT.p2.indexOf("If I hold"));
    if (t < T.e1Type[0]) return { text, sel: [i1, i1 + 4], caret: null };
    const n1 = count(S.e1, t);
    text = TXT.p2.slice(0, i1) + EDIT1.to.slice(0, n1) + TXT.p2.slice(i1 + 4);
    caret = i1 + n1;
    if (t < T.e2Drag[0]) return { text, sel, caret };
    const i2 = text.indexOf(EDIT2.from);
    if (t < T.e2Type[0]) {
      const k = sine(prog(t, T.e2Drag[0], T.e2Drag[1]));
      const end = Math.round(i2 + EDIT2.from.length * k);
      return { text, sel: end > i2 ? [i2, end] : null, caret: end > i2 ? null : i2, drag: true };
    }
    const n2 = count(S.e2, t);
    text = text.slice(0, i2) + EDIT2.to.slice(0, n2) + text.slice(i2 + EDIT2.from.length);
    return { text, sel, caret: i2 + n2 };
  }

  // ======================================================================== the drawing, stitched
  // The model's SVG (named colours, steps with captions) becomes running stitches, knots for
  // dots, washes for fills and Figtree for labels, precomputed once.
  let SKETCH = null;
  function buildSketch() {
    const doc = new DOMParser().parseFromString(TXT.svg, "image/svg+xml");
    const root = doc.documentElement;
    const vb = (root.getAttribute("viewBox") || "0 0 320 300").split(/[\s,]+/).map(Number);
    const meas = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    meas.setAttribute("style", "position:absolute;left:-9999px;top:0;width:10px;height:10px");
    document.body.appendChild(meas);
    const STROKE = { ink: "madder", graphite: "tack", faint: "line2", rule: "line2" };
    const steps = [];
    $$(":scope > g[data-step]", root).forEach((g, si) => {
      const items = [];
      let seed = 100 + si * 17;
      const els = Array.from(g.querySelectorAll("path,line,polyline,polygon,rect,circle,ellipse,text"));
      for (const el of els) {
        const tag = el.tagName.toLowerCase();
        const at = (k, d) => (el.getAttribute(k) ?? g.getAttribute(k) ?? d);
        if (tag === "text") {
          items.push({ kind: "text", x: +at("x", 0), y: +at("y", 0), text: el.textContent.trim(), size: +at("font-size", 13), cls: at("class", "sans"), anchor: at("text-anchor", "start"), fill: at("fill", "ink") });
          continue;
        }
        const fill = at("fill", "none"), stroke = at("stroke", "ink");
        if (tag === "circle" && +el.getAttribute("r") <= 5 && fill === "ink") {
          items.push({ kind: "knot", cx: +el.getAttribute("cx"), cy: +el.getAttribute("cy"), r: Math.max(2.6, +el.getAttribute("r") * 0.95) });
          continue;
        }
        const clone = document.createElementNS("http://www.w3.org/2000/svg", tag);
        for (const a of el.attributes) clone.setAttribute(a.name, a.value);
        meas.appendChild(clone);
        const L = clone.getTotalLength ? clone.getTotalLength() : 0;
        const pts = [];
        const N = Math.max(2, Math.ceil(L / 1.5));
        for (let i = 0; i <= N; i++) { const p = clone.getPointAtLength((L * i) / N); pts.push([p.x, p.y]); }
        if (fill === "wash" || fill === "paper") items.push({ kind: "fill", d: pathOf(tag, el), fill });
        if (stroke !== "none") {
          const st = runAlong(pts, STROKE[stroke] || "madder", 2.1, L < 26 ? 4.2 : 6.6, L < 26 ? 2.6 : 3.6, seed++);
          st.forEach((s) => items.push({ kind: "stitch", s }));
        }
      }
      steps.push({ caption: g.getAttribute("data-caption") || "", items });
    });
    meas.remove();
    SKETCH = { vb, steps };
  }
  function pathOf(tag, el) {
    const n = (k) => +el.getAttribute(k) || 0;
    if (tag === "rect") { const x = n("x"), y = n("y"), w = n("width"), h = n("height"), r = Math.min(n("rx"), w / 2, h / 2); return `M${x + r} ${y}H${x + w - r}Q${x + w} ${y} ${x + w} ${y + r}V${y + h - r}Q${x + w} ${y + h} ${x + w - r} ${y + h}H${x + r}Q${x} ${y + h} ${x} ${y + h - r}V${y + r}Q${x} ${y} ${x + r} ${y}Z`; }
    if (tag === "circle") { const cx = n("cx"), cy = n("cy"), r = n("r"); return `M${cx - r} ${cy}A${r} ${r} 0 1 0 ${cx + r} ${cy}A${r} ${r} 0 1 0 ${cx - r} ${cy}Z`; }
    if (tag === "ellipse") { const cx = n("cx"), cy = n("cy"), rx = n("rx"), ry = n("ry"); return `M${cx - rx} ${cy}A${rx} ${ry} 0 1 0 ${cx + rx} ${cy}A${rx} ${ry} 0 1 0 ${cx - rx} ${cy}Z`; }
    return el.getAttribute("d") || "";
  }
  /** The drawing as it stands at time t: earlier steps whole, the current one being sewn. */
  function sketchAt(t) {
    const { vb, steps } = SKETCH;
    let out = "", cur = 0;
    steps.forEach((st, i) => {
      const [a, b] = T.skSteps[i] || [999, 999];
      if (t < a) return;
      cur = i;
      const k = prog(t, a, b), n = st.items.length;
      st.items.forEach((it, j) => {
        const due = (j / n) * 0.86, kk = clamp((k - due) / 0.14);
        if (kk <= 0) return;
        if (it.kind === "stitch") out += stitchSVG(it.s, kk);
        else if (it.kind === "knot") out += knotSVG(it.cx, it.cy, it.r, "madder", knotScale(kk * 0.2));
        else if (it.kind === "fill") out += `<path d="${it.d}" style="fill:${it.fill === "wash" ? "color-mix(in srgb,var(--madder) 9%,transparent)" : "var(--surface)"}" opacity="${fx(kk)}"/>`;
        else if (it.kind === "text") {
          const col = it.fill === "graphite" ? "var(--fg2)" : it.fill === "faint" ? "var(--fg3)" : "var(--fg)";
          out += `<text x="${it.x}" y="${it.y}" text-anchor="${it.anchor}" style="font-size:${it.size}px;font-weight:${it.cls === "voice" ? 500 : 400};fill:${col}" opacity="${fx(kk)}">${esc(it.text)}</text>`;
        }
      });
    });
    const last = steps.findIndex((_, i) => t < (T.skSteps[i] || [999])[0]);
    const shown = last === -1 ? steps.length : last;
    return { svg: `<svg viewBox="${vb.join(" ")}">${out}</svg>`, step: cur + 1, of: steps.length, caption: steps[cur] ? steps[cur].caption : "", shown };
  }

  // ======================================================================== the wordmark, stitched
  let WMS = null;
  function buildWordmark() {
    const doc = new DOMParser().parseFromString(ASSETS.WM, "image/svg+xml");
    const root = doc.documentElement;
    const vb = root.getAttribute("viewBox");
    const meas = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    meas.setAttribute("style", "position:absolute;left:-9999px;top:0;width:10px;height:10px");
    document.body.appendChild(meas);
    const glyphs = [];
    $$("path.wm-ink", root).forEach((p, gi) => {
      const subs = p.getAttribute("d").split(/(?=M)/).filter((s) => s.trim());
      const stitches = [];
      subs.forEach((d, si) => {
        const el = document.createElementNS("http://www.w3.org/2000/svg", "path");
        el.setAttribute("d", d); meas.appendChild(el);
        const L = el.getTotalLength(), N = Math.max(4, Math.ceil(L / 0.5)), pts = [];
        for (let i = 0; i <= N; i++) { const q = el.getPointAtLength((L * i) / N); pts.push([q.x, q.y]); }
        // each contour is sewn from its own start; its stitches carry where along it they sit
        runAlong(pts, "fg", 1.15, 2.9, 1.7, 40 + gi * 7 + si).forEach((s, i, all) => stitches.push({ s, k: i / all.length }));
      });
      glyphs.push({ d: p.getAttribute("d"), stitches });
    });
    meas.remove();
    const knot = Array.from(root.children).filter((e) => /wm-k/.test(e.getAttribute("class") || "")).map((e) => e.outerHTML).join("");
    const kc = root.querySelector("circle.wm-k");
    WMS = { vb, glyphs, knot, kx: +kc.getAttribute("cx"), ky: +kc.getAttribute("cy") };
  }
  /** The wordmark at time t of its own making: `k` per glyph is how far it is sewn. */
  function wordmarkSVG(t, opts) {
    const { vb, glyphs, knot, kx, ky } = WMS;
    let fills = "", st = "", needle = "";
    const fillK = opts.fillK, sewn = opts.sewn;
    glyphs.forEach((g, gi) => {
      const k = sewn ? 1 : prog(t, T.sew0 + gi * T.glyphGap, T.sew0 + gi * T.glyphGap + T.glyphDur);
      fills += `<path class="wm-ink" d="${g.d}" opacity="${fx(fillK)}"/>`;
      if (opts.stitchOpacity <= 0.01) return;
      for (const { s, k: at } of g.stitches) {
        const kk = clamp((k - at * 0.9) / 0.1);
        if (kk > 0) st += stitchSVG({ ...s, c: "fg" }, kk);
      }
      if (!sewn && k > 0 && k < 1 && !needle) {
        const head = g.stitches.filter((x) => x.k * 0.9 <= k).pop();
        if (head) needle = needleAt(head.s.x1 + 2, head.s.y1 + 2, t, 16);
      }
    });
    const ks = opts.knotScale;
    const knotG = ks > 0.01 ? `<g transform="translate(${kx} ${ky}) scale(${fx(ks)}) translate(${-kx} ${-ky})">${knot}</g>` : "";
    return `<svg class="wmsvg" viewBox="${vb}">${fills}<g opacity="${fx(opts.stitchOpacity)}">${st}</g>${knotG}${needle}</svg>`;
  }
  const WM_STATIC = () => ASSETS.WM.replace("<svg ", '<svg class="wmsvg" ');

  // ======================================================================== threads
  // Each thread: its anchor, colour, mind, and when things happen to it. States are read
  // off the clock: closed (waiting for a pause), sewn, open, thinking, tied, faded.
  // A thread keeps its number for life, in the order the note's threads were made: the
  // brain numbers them the same way (the chat's "threads 3 to 5" are the room's three).
  const THREAD_NO = { t1: 1, t2: 2, r0: 3, r1: 4, r2: 5, call: 6 };
  const NUM = (t) => {
    // the threads still open
    const open = [];
    if (t >= T.t1Arrive && t < T.t1Knot) open.push("t1");
    if (t >= T.t2Think[0] && t < T.t2Knot) open.push("t2");
    if (t >= T.roomSend) open.push("r0", "r1", "r2");
    if (t >= T.callArrive) open.push("call");
    return open;
  };
  const ROOM = [
    { id: "r0", mind: "Inkling", c: "madder", key: "inkling" },
    { id: "r1", mind: "Skeptic", c: "indigo", key: "skeptic" },
    { id: "r2", mind: "Tutor", c: "ochre", key: "tutor" },
  ];
  /** Words revealed by `t` for a streamed answer: the brain's own chunks at their own pace,
   *  fitted into [t0, t1]. Structured answers (no chunks) arrive word by word. */
  function streamed(text, deltas, t0, t1, t) {
    if (t <= t0) return "";
    if (t >= t1) return text;
    if (deltas && deltas.length > 1) {
      const a = deltas[0].ms, b = deltas[deltas.length - 1].ms, k = prog(t, t0, t1) * (b - a) + a;
      let s = "";
      for (const d of deltas) { if (d.ms <= k) s += d.text; else break; }
      return s.replace(/^\s+/, "");
    }
    const words = text.split(" "), n = Math.ceil(prog(t, t0, t1) * words.length);
    return words.slice(0, n).join(" ");
  }
  const paras = (s) => s.split(/\n\n+/).map((p) => `<p>${esc(p)}</p>`).join("");

  // ======================================================================== camera
  // x: the window x at the centre of the frame; s: scale; the window's top edge sits at
  // TOP on screen, so captions above it never cover the app. scroll: the note's own scroll.
  const TOP = 130;
  const CAM0 = { x: 620, s: 1.18, scroll: 0 };
  const MOVES = [
    { t: [9.0, 21.8], to: { x: 660, s: 1.32 }, e: sine },
    { t: [22.5, 23.7], to: { x: 790, s: 1.5 } },
    { t: [29.2, 30.4], to: { x: 980, s: 1.72, scroll: 285 } },
    { t: [36.3, 37.4], to: { x: 720, s: 1.5, scroll: 0 } },
    { t: [46.7, 47.4], to: { x: 760, s: 1.4, scroll: 0 } },
    { t: [56.7, 57.6], to: { x: 790, s: 1.27, scroll: 0 } },
    { t: [60.6, 61.4], to: { x: 760, s: 1.42, scroll: 0 } },
    { t: [66.0, 67.6], to: { x: 800, s: 1.13, scroll: 0 } },
    { t: [67.6, 71.6], to: { x: 830, s: 1.22, scroll: 140 }, e: sine },
    { t: [71.8, 72.5], to: { x: 560, s: 1.42, scroll: 0 } },
    { t: [74.75, 75.45], to: { x: 990, s: 1.21, scroll: 0 } },
    { t: [76.6, 77.5], to: { x: 1010, s: 1.5, scroll: 0 } },
    { t: [80.3, 81.0], to: { x: 760, s: 1.42, scroll: 210 } },
    { t: [86.9, 87.75], to: { x: 620, s: 1.18, scroll: 0 } },
  ];
  function cam(t) {
    const c = { ...CAM0 };
    for (const m of MOVES) {
      if (t <= m.t[0]) break;
      const k = (m.e || easeIO)(prog(t, m.t[0], m.t[1]));
      for (const key in m.to) c[key] = lerp(c[key], m.to[key], k);
    }
    return c;
  }
  const toScreen = (c, x, y) => [960 + (x - c.x) * c.s, TOP + y * c.s];

  // ======================================================================== captions
  const CAPS = [
    { t: [9.0, 14.2], text: "You write.", u: "write" },
    { t: [14.6, 19.15], text: "Inkling reads along, and mostly stays quiet.", u: "reads along" },
    { t: [19.45, 22.95], text: "When it has a question, it waits for you to pause.", u: "waits" },
    { t: [23.3, 28.7], text: "When the numbers don’t add up, it pulls a thread.", u: "pulls a thread" },
    { t: [29.0, 36.35], text: "Ask for a sketch, and it stitches one out, step by step.", u: "stitches one out" },
    { t: [37.3, 41.7], text: "Fix the words yourself.", u: "Fix the words" },
    { t: [41.9, 44.75], text: "It reads them again.", u: "again" },
    { t: [45.0, 47.2], text: "Nobody pressed Resolve.", u: "Resolve" },
    { t: [47.45, 53.9], text: "Unsure of a line? Ask it to find a loose thread.", u: "find a loose thread" },
    { t: [54.2, 61.3], text: "Answer in the tag, and the thread ties itself off.", u: "ties itself off" },
    { t: [61.55, 65.9], text: "Ask the room: up to three minds at once.", u: "three minds" },
    { t: [66.15, 71.85], text: "Each mind answers on its own thread.", u: "its own thread" },
    { t: [72.2, 74.85], text: "Tell Inkling what the note is for.", u: "what the note is for" },
    { t: [75.05, 80.45], text: "Chat about the whole note.", u: "whole note" },
    { t: [80.75, 87.0], text: "Call any mind by name, mid-note.", u: "by name" },
    { t: [87.3, 89.9], text: "Hide threads. Only your words stay.", u: "Hide threads" },
    { t: [90.2, 92.4], text: "Lamplight, for late pages.", u: "Lamplight" },
  ];

  // ======================================================================== the pointer
  // Moves are eased with a slight arc, like a hand; presses dip it for a moment. Targets
  // are measured from the page at the time they are reached (resolveTargets, at load).
  const PTR = [
    { show: [26.95, 30.4] }, { show: [37.35, 40.2] }, { show: [46.95, 49.9] }, { show: [53.75, 54.55] },
    { show: [60.95, 63.95] }, { show: [71.75, 72.75] }, { show: [87.15, 90.6] },
  ];
  const MOVE = [
    // the correction tag, then Sketch it
    { t: [26.95, 27.55], from: "@t1tag:.55,.62+[60,70]", to: "@t1tag:.55,.62" },
    { t: [28.2, 28.85], to: "@sketchbtn" },
    { t: [29.3, 30.4], to: "@sketchbtn+[30,46]" },
    // fix the words
    { t: [37.35, 38.0], from: "@e1sel+[120,90]", to: "@e1sel", ibeam: true },
    { t: [38.95, 39.45], to: "@e2start", ibeam: true },
    { t: [39.5, 40.05], live: "selEnd", ibeam: true },
    // find a loose thread
    { t: [46.95, 47.3], from: "@t2start+[-60,80]", to: "@t2start", ibeam: true },
    { t: [47.35, 48.1], live: "selEnd", ibeam: true },
    { t: [48.35, 49.0], to: "@findbtn" },
    { t: [49.3, 49.9], to: "@findbtn+[40,60]" },
    // reply in the tag
    { t: [53.75, 54.25], from: "@t2reply+[90,80]", to: "@t2reply", ibeam: true },
    // ask the room
    { t: [60.95, 61.28], from: "@roomstart+[-40,70]", to: "@roomstart", ibeam: true },
    { t: [61.3, 61.9], live: "selEnd", ibeam: true },
    { t: [62.1, 62.7], to: "@askbtn" },
    { t: [62.9, 63.25], to: "@chip1" },
    { t: [63.35, 63.65], to: "@chip2" },
    { t: [63.72, 63.95], to: "@chip2+[30,30]" },
    // tell Inkling
    { t: [71.75, 72.45], from: "@tell+[60,120]", to: "@tell" },
    // hide threads, show threads
    { t: [87.15, 87.85], from: "@hide+[-160,160]", to: "@hide" },
    { t: [90.0, 90.6], to: "@hide+[-70,130]" },
  ];
  const PRESS = [28.9, 38.1, 38.2, 39.5, 47.35, 49.15, 54.3, 61.3, 62.75, 63.3, 63.7, 72.5, 87.9, 89.7];
  const RELEASE = { 39.5: 40.05, 47.35: 48.1, 61.3: 61.9 };
  const TARGETS = {}; // name -> [x, y] in window coordinates, filled at load

  // ======================================================================== DOM building
  const stage = () => $("#stage");
  let winRect0 = null;
  function sideHTML(title, open) {
    const knot = (c) => `<svg viewBox="0 0 12 12" width="10" height="10">${knotSVG(6, 6, 4.6, c)}</svg>`;
    return `<div class="find"><span>Find or start a note</span><kbd>⌘K</kbd></div>` +
      `<div class="fold">Running</div><div class="nt cur"><span>${esc(title || "Untitled")}</span>${open ? `<span class="st-n">${knot("madder")}${open}</span>` : ""}</div>` +
      `<div class="nt"><span>Shoes: what wore out where</span></div>` +
      `<div class="fold">Home</div><div class="nt"><span>Why the bread didn’t rise</span><span class="st-n">${knot("tack")}</span></div><div class="nt"><span>Kitchen shelf measurements</span></div>` +
      `<div class="fold">Notes</div><div class="nt"><span>Reading list for winter</span></div><div class="nt"><span>Questions for the landlord</span></div>`;
  }
  const PENCIL = `<svg viewBox="0 0 12 12"><path d="M2 10l.6-2.4L8.4 1.8l1.8 1.8-5.8 5.8z" style="fill:none;stroke:var(--fg3);stroke-width:1.1;stroke-linejoin:round"/></svg>`;
  const CHEV = `<svg viewBox="0 0 12 12"><path d="M3 4.5l3 3 3-3" style="fill:none;stroke:var(--fg3);stroke-width:1.3;stroke-linecap:round;stroke-linejoin:round"/></svg>`;
  const SEND = `<svg viewBox="0 0 14 14"><path d="M2 7h9M7.5 3.5L11 7l-3.5 3.5" style="fill:none;stroke:var(--fg2);stroke-width:1.4;stroke-linecap:round;stroke-linejoin:round"/></svg>`;

  /** Builds text with spans for anchors, the selection and a caret marker. */
  function markup(text, ranges, caret) {
    const cuts = new Set([0, text.length]);
    ranges.forEach((r) => { cuts.add(clamp(r.a, 0, text.length)); cuts.add(clamp(r.b, 0, text.length)); });
    if (caret != null) cuts.add(caret);
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
      html += `<span class="${on.map((r) => r.cls).join(" ")}"${ids ? ` data-a="${ids}"` : ""}>${esc(seg)}</span>`;
    }
    return html;
  }

  // ======================================================================== renderAt
  let lastT = -1;
  const st = { rects: {}, stitchCounts: {} };

  function renderAt(t) {
    lastT = t;
    const c = cam(t);
    applyTheme(themeAt(t));
    $("#world").style.transform = `translate(${fx(960 - c.x * c.s)}px, ${fx(TOP)}px) scale(${c.s})`;

    // ---- the window: rises onto the linen, sinks at the end
    const win = $("#win");
    const wk = easeOut(prog(t, T.win0, T.win1)), wo = easeIO(prog(t, T.winOut[0], T.winOut[1]));
    win.style.opacity = String(fx(wk * (1 - wo)));
    win.style.transform = `translateY(${fx((1 - wk) * 18 + wo * 26)}px) scale(${fx(0.985 + 0.015 * wk)})`;
    win.style.visibility = wk * (1 - wo) > 0.001 ? "visible" : "hidden";
    $("#tb-wm").style.opacity = t >= T.fly1 ? "1" : "0";

    renderNote(t, c);
    renderOpen(t, c);
    renderClose(t);
    renderCaption(t);
    renderPointer(t, c);
    renderKeys(t);
    $("#fade").style.opacity = String(fx(easeIn(prog(t, T.fadeOut[0], T.fadeOut[1]))));
  }

  // ---------------------------------------------------------------- the note page
  function renderNote(t, c) {
    const open = NUM(t), nOpen = open.length;
    const hideK = easeOut(prog(t, T.hide[0], T.hide[1])) * (1 - easeOut(prog(t, T.show[0], T.show[1])));
    const hidden = t >= T.hideClick && t < T.showClick;
    const markOp = 1 - hideK;

    // header row (the app's `header`): mind, Tell Inkling, mode, then what is loose and Hide threads
    const briefSet = t >= T.briefSet;
    const hov = (a, b) => within(t, a, b) ? " hov" : "";
    $("#nh").innerHTML =
      `<span class="mind" style="opacity:${fx(markOp)}"><span class="dot" style="width:7px;height:7px"></span>Inkling${CHEV}</span>` +
      (briefSet ? "" : `<span class="it${hov(72.2, 72.6)}" style="opacity:${fx(markOp)}">${PENCIL}Tell Inkling</span>`) +
      `<span class="it" style="opacity:${fx(markOp)}">Reads along${CHEV}</span><span class="grow"></span>` +
      (nOpen ? `<span class="it" style="opacity:${fx(markOp)}"><span class="dot" style="width:7px;height:7px"></span>${nOpen} loose thread${nOpen === 1 ? "" : "s"}</span>` : "") +
      `<span class="it${hov(87.6, 87.95) || hov(89.45, 89.75)}">${hidden ? "Show threads" : "Hide threads"}</span>`;

    // the brief line: an input while you type it, then the brief itself
    const bh = easeOut(prog(t, T.briefOpen[0], T.briefOpen[1])) * 40;
    const bl = $("#briefline");
    bl.style.height = fx(bh) + "px";
    if (t >= T.briefOpen[0] && t < T.briefSet) {
      const n = count(S.brief, t);
      const typing = t - lastKey(S.brief, t) < 0.5;
      bl.innerHTML = `<div class="bin">${PENCIL}<span style="position:relative">${n ? esc(TXT.brief.slice(0, n)) : '<span style="color:var(--fg3)">What is this note for?</span>'}${caretSpan(t, typing)}</span></div>`;
    } else if (briefSet) {
      bl.innerHTML = `<div class="bset">${PENCIL}<b>For this note:</b><span>${esc(TXT.brief)}</span></div>`;
    } else bl.innerHTML = "";
    const vpTop = 52 + bh;
    $("#vp").style.top = fx(vpTop) + "px";
    $("#content").style.transform = `translateY(${fx(-c.scroll)}px)`;

    // sidebar
    const titleN = count(S.title, t);
    $("#side").innerHTML = sideHTML(TXT.title.slice(0, titleN), nOpen);

    // ---- the words
    const title = $("#n-title");
    const focusTitle = t >= T.win0 && t < T.enter.p1;
    if (titleN === 0) { title.className = "n-title ph"; title.innerHTML = (focusTitle ? '<span class="cm"></span>' : "") + "Untitled"; }
    else { title.className = "n-title"; title.innerHTML = markup(TXT.title.slice(0, titleN), [], focusTitle ? titleN : null); }

    const blocks = blocksAt(t);
    const rows = $("#rows");
    let html = "";
    for (const b of blocks) html += `<div class="gut" data-p="${b.id}">${gutterHTML(b.id, t)}</div><p class="para" data-p="${b.id}">${markup(b.text, b.ranges, b.caret)}</p>`;
    rows.innerHTML = html;
    $$(".gut", rows).forEach((g) => (g.style.opacity = String(fx(markOp))));

    // the caret: wherever the writer's focus is
    placeCaret(t, c.s);

    // ---- measure what the threads hang from
    const content = $("#content"), cr = content.getBoundingClientRect(), s = c.s;
    const rel = (r) => ({ x: (r.left - cr.left) / s, y: (r.top - cr.top) / s, r: (r.right - cr.left) / s, b: (r.bottom - cr.top) / s });
    const lineRects = (id) => {
      const rs = [];
      $$(`[data-a~="${id}"]`, rows).forEach((sp) => Array.from(sp.getClientRects()).forEach((r) => { if (r.width > 1) rs.push(rel(r)); }));
      // one rect per line
      const lines = [];
      rs.sort((a, b) => a.y - b.y || a.x - b.x).forEach((r) => {
        const L = lines.find((l) => Math.abs(l.y - r.y) < 4);
        if (L) { L.x = Math.min(L.x, r.x); L.r = Math.max(L.r, r.r); L.b = Math.max(L.b, r.b); }
        else lines.push({ ...r });
      });
      return lines;
    };
    st.rel = rel; st.lineRects = lineRects; st.scale = s;

    // ---- the band: one motif per tied thread
    renderBand(t, markOp);

    // ---- tags, ropes, stitches
    renderThreads(t, c, open, markOp);
    renderCards(t);
    renderChat(t);
  }

  function caretSpan(t, typing) {
    const on = typing || Math.floor(t / 0.53) % 2 === 0;
    return on ? '<span class="icaret"></span>' : '<span class="icaret" style="opacity:0"></span>';
  }

  /** The note's paragraphs at time t, each with its anchors, selection and caret. */
  function blocksAt(t) {
    const out = [];
    const caretIn = focusAt(t);
    const add = (id, text, ranges, sel, caret) => {
      const rs = ranges.slice();
      if (sel && sel[1] > sel[0]) rs.push({ a: sel[0], b: sel[1], cls: "sel" });
      out.push({ id, text, ranges: rs, caret: caretIn === id ? caret : null });
    };
    if (t >= T.enter.p1) add("p1", TXT.p1.slice(0, count(S.p1, t)), [], null, count(S.p1, t));
    if (t >= T.enter.p2) {
      const p = p2At(t), rs = [];
      const i1 = p.text.indexOf("If I hold");
      if (t >= T.t1Arrive && i1 >= 0 && p.text.length === p.text.length) rs.push({ a: i1, b: p.text.length, cls: "anc" + (t >= T.t1Open && t < T.t1Knot ? " on" : ""), id: "t1" });
      const ir = p.text.indexOf(TXT.roomTarget);
      if (t >= T.roomSend && ir >= 0) rs.push({ a: ir, b: ir + TXT.roomTarget.length, cls: "anc", id: "room" });
      let sel = p.sel;
      if (t >= T.roomDrag[0] && t < T.roomSend && ir >= 0) {
        const k = sine(prog(t, T.roomDrag[0], T.roomDrag[1]));
        sel = [ir, Math.round(ir + TXT.roomTarget.length * k)];
      }
      let caret = p.caret;
      if (t >= T.roomSend && t < T.enter.p5) caret = ir + TXT.roomTarget.length;
      add("p2", p.text, rs, sel, caret == null ? -1 : caret);
    }
    if (t >= T.enter.p3) {
      const text = TXT.p3.slice(0, count(S.p3, t)), rs = [];
      const i2 = text.indexOf(TXT.p3.slice(TXT.p3.indexOf("I’ll")));
      if (t >= T.t2Think[0] && i2 >= 0) rs.push({ a: i2, b: text.length, cls: "anc" + (t >= T.t2Text && t < T.t2Knot ? " on" : ""), id: "t2" });
      let sel = null;
      const s0 = TXT.p3.indexOf("I’ll");
      if (t >= T.t2Drag[0] && t < T.t2Click) sel = [s0, Math.round(s0 + (TXT.p3.length - s0) * sine(prog(t, T.t2Drag[0], T.t2Drag[1])))];
      add("p3", text, rs, sel, text.length);
    }
    if (t >= T.enter.p4) add("p4", TXT.p4.slice(0, count(S.p4, t)), [], null, count(S.p4, t));
    if (t >= T.enter.p5) {
      const text = TXT.p5.slice(0, count(S.p5, t)), rs = [];
      if (t >= T.callArrive) rs.push({ a: 0, b: text.length, cls: "anc", id: "call" });
      add("p5", text, rs, null, text.length);
    }
    return out;
  }
  /** Which paragraph holds the caret (or null when the focus is elsewhere). */
  function focusAt(t) {
    if (t < T.enter.p1) return null;
    if (t < T.enter.p2) return "p1";
    if (t < T.enter.p3) return "p2";
    if (t < T.enter.p4) return "p3";
    if (t < T.e1Sel) return "p4";
    if (t < T.t2Drag[0]) return t >= T.e1Sel && t < T.e1Type[0] ? null : "p2";
    if (t < T.t2Click) return null;
    if (t < T.t2ReplyClick) return "p3";
    if (t < T.t2Send) return null;
    if (t < T.roomDrag[0]) return "p3";
    if (t < T.roomSend) return null;
    if (t < T.briefClick) return "p2";
    if (t < T.briefSet) return null;
    if (t < T.chatIn[0]) return "p2";
    if (t < T.chatOut[0]) return null;
    if (t < T.enter.p5) return "p4";
    return "p5";
  }
  function placeCaret(t, s) {
    const el = $("#caret"), m = $("#content .cm");
    if (!m || t >= T.winOut[0]) { el.style.display = "none"; return; }
    const cr = $("#content").getBoundingClientRect(), r = m.getBoundingClientRect();
    // solid while typing, blinking when idle
    const keys = [S.title, S.p1, S.p2, S.p3, S.p4, S.e1, S.e2, S.p5].map((x) => lastKey(x, t));
    const typing = t - Math.max(...keys) < 0.5;
    const on = typing || Math.floor(t / 0.53) % 2 === 0;
    el.style.display = on ? "block" : "none";
    el.style.left = fx((r.left - cr.left) / s - 0.8) + "px";
    el.style.top = fx((r.top - cr.top) / s - 1) + "px";
    el.style.height = fx(r.height / s + 2) + "px";
  }

  // ---------------------------------------------------------------- the gutter
  function gutterHTML(id, t) {
    const reading = (a, b) => t >= a && t < b;
    const needle = `<svg width="20" height="20" viewBox="0 0 64 64" style="left:18px;top:${fx(4 + 1.5 * (1 + Math.sin((t % 0.9) / 0.9 * Math.PI * 2)))}px">${NEEDLE.replace(/<svg[^>]*>/, "").replace("</svg>", "")}</svg>`;
    const held = `<svg width="18" height="14" viewBox="0 0 18 14" style="left:19px;top:7px">${stitchSVG({ x0: 2, y0: 7.4, x1: 16, y1: 6.6, w: 2.4, c: "tack" }, 1)}</svg>`;
    const knot = (n, c, k) => `<svg width="14" height="14" viewBox="0 0 14 14" style="left:21px;top:4px">${knotSVG(7, 7, 5.2, c, k)}</svg><span class="num">${n}</span>`;
    const R = T.read;
    if (id === "p1") return reading(...R.p1) ? needle : t >= R.p1[1] ? held : "";
    if (id === "p2") {
      if (reading(...R.p2) || reading(...T.t1Reread)) return needle;
      if (t >= T.t1Arrive && t < T.t1Knot) return knot(THREAD_NO.t1, "madder", knotScale(t - T.t1Arrive));
      if (t >= T.roomSend) return knot(THREAD_NO.r0, "madder", knotScale(t - T.roomSend));
      return t >= R.p2[1] ? held : "";
    }
    if (id === "p3") {
      if (reading(...R.p3)) return needle;
      if (t >= T.t2Think[0] && t < T.t2Knot) return knot(THREAD_NO.t2, "madder", knotScale(t - T.t2Think[0]));
      return t >= R.p3[1] ? held : "";
    }
    if (id === "p4") return reading(...R.p4) ? needle : t >= R.p4[1] ? held : "";
    if (id === "p5") {
      if (reading(T.p5[1] + 0.1, T.callArrive)) return needle;
      if (t >= T.callArrive) return knot(THREAD_NO.call, "indigo", knotScale(t - T.callArrive));
    }
    return "";
  }

  // ---------------------------------------------------------------- the band
  function renderBand(t, markOp) {
    const items = [];
    if (t >= T.t1Motif) items.push({ c: "madder", k: prog(t, T.t1Motif, T.t1Motif + 0.4) });
    if (t >= T.t2Motif) items.push({ c: "madder", k: prog(t, T.t2Motif, T.t2Motif + 0.4) });
    const Wd = 880 - 74, H = 20, n = 24, step = Wd / n;
    let out = "";
    for (let k = items.length; k < n; k++) {
      const x = k * step + step / 2;
      out += `<path d="M${fx(x - 3)} ${H / 2}L${fx(x + 3)} ${H / 2}" style="stroke:var(--tack);stroke-width:1.1;stroke-linecap:round;opacity:.55"/>`;
    }
    items.forEach((it, k) => { out += motifSVG(k * step + step / 2, H / 2, k, it.c, it.k); });
    const svg = $("#band-svg");
    svg.setAttribute("viewBox", `0 0 ${fx(Wd)} ${H}`);
    svg.innerHTML = out;
    svg.style.opacity = String(fx(markOp));
    const lab = $("#band-count");
    lab.textContent = items.length ? `${items.length} tied off` : "";
    lab.style.opacity = String(fx(markOp));
  }

  // ---------------------------------------------------------------- tags
  /** What a thread's tag shows at time t, or null when it has none yet. */
  function tagModel(id, t) {
    const head = (mind, kind, n) => `<div class="hd"><span class="who"><b>${esc(mind)}</b><span class="k">· ${esc(kind)}</span></span>${n ? `<span class="n">${n}</span>` : ""}</div>`;
    const think = (label) => `<div class="think">${label ? `<span>${esc(label)}</span>` : ""}<span class="dot" style="opacity:${fx(0.3 + 0.6 * (0.5 + 0.5 * Math.sin((t / 1.1) * Math.PI * 2)))}"></span></div>`;
    const num = THREAD_NO[id];
    const acts = (extra, hovers) => `<div class="acts">${extra || ""}<span class="${hovers && hovers.resolve ? "hov" : ""}">Resolve</span><span>Dismiss</span><span class="grow"></span><span>Make a note</span></div>`;
    const reply = (text, focus, caret) => `<div class="reply${focus ? " focus" : ""}"><span class="tx">${text ? esc(text) : '<span class="ph">Reply…</span>'}${focus ? caret : ""}</span>${SEND}</div>`;
    if (id === "t1") {
      if (t < T.t1Arrive) return null;
      const folded = t >= T.t1Fold;
      const op = 1 - prog(t, T.t1Fade[0], T.t1Fade[1]);
      if (op <= 0) return null;
      if (folded) return { c: "madder", op, html: head("Inkling", "tied off") + `<div class="short">${esc(TXT.correction.slice(0, 64))}…</div>`, anchor: "t1", folded: true };
      if (t < T.t1Open) return { c: "madder", op: easeOut(prog(t, T.t1Arrive, T.t1Arrive + 0.3)), html: head("Inkling", "correction", num) + `<div class="wait">Opens when you pause</div>`, anchor: "t1" };
      let body = `<div class="m">${paras(TXT.correction)}</div>`;
      if (t >= T.t1Reread[0] && t < T.t1Settle[0]) body += think("Reading your change");
      if (t >= T.t1Settle[0]) body += `<div class="m">${paras(streamed(TXT.settle, null, ...T.t1Settle, t))}</div>`;
      let reserve = null;
      if (t >= T.skThink[0] && t < T.skThink[1]) body += think("Stitching a sketch…");
      if (t >= T.skSteps[0][0]) {
        const sk = sketchAt(t);
        body += `<div class="sk">${sk.svg}</div><div class="capt"><span class="sn">${sk.step} / ${sk.of}</span><span class="gr">${esc(sk.caption)}</span><i>‹</i><i>›</i><i>↺</i></div>`;
      }
      const busy = t >= T.e1Sel; // while the writer edits, the tag waits without its buttons
      if (!busy) body += reply("", false, "") + acts(`<span class="pri${within(t, 28.5, 29.05) ? " hov" : ""}">Sketch it</span>`);
      return { c: "madder", op: 1, html: head("Inkling", "correction", num) + body, anchor: "t1", active: t < T.t1Knot, tying: t >= T.t1Tie };
    }
    if (id === "t2") {
      if (t < T.t2Think[0]) return null;
      const op = 1 - prog(t, T.t2Fade[0], T.t2Fade[1]);
      if (op <= 0) return null;
      if (t >= T.t2Fold) return { c: "madder", op, html: head("Inkling", "tied off") + `<div class="short">${esc(TXT.question.slice(0, 64))}…</div>`, anchor: "t2", folded: true };
      if (t < T.t2Text) return { c: "madder", op: easeOut(prog(t, T.t2Think[0], T.t2Think[0] + 0.25)), html: head("Inkling", "question", num) + think(""), anchor: "t2", active: true };
      const k = easeOut(prog(t, T.t2Text, T.t2Text + 0.3));
      let body = `<div class="m" style="opacity:${fx(k)}">${paras(TXT.question)}</div>`;
      if (t >= T.t2Send) body += `<div class="you"><span>You</span><span>${esc(TXT.reply)}</span></div>`;
      if (t >= T.t2Read[0] && t < T.t2Answer[0]) body += think("");
      if (t >= T.t2Answer[0]) body += `<div class="m">${paras(streamed(TXT.answer, null, ...T.t2Answer, t))}</div>`;
      if (t < T.t2Send) {
        const typing = t - lastKey(S.reply, t) < 0.5;
        const focus = t >= T.t2ReplyClick;
        body += reply(TXT.reply.slice(0, count(S.reply, t)), focus, caretSpan(t, typing)) + acts("");
      }
      return { c: "madder", op: 1, html: head("Inkling", "question", num) + body, anchor: "t2", active: t < T.t2Knot, tying: t >= T.t2Tie,
        reserve: t >= T.t2Send && t < T.t2Answer[1] ? head("Inkling", "question", num) + `<div class="m">${paras(TXT.question)}</div><div class="you"><span>You</span><span>${esc(TXT.reply)}</span></div><div class="m">${paras(TXT.answer)}</div>` : null };
    }
    const room = ROOM.find((r) => r.id === id);
    if (room) {
      const i = ROOM.indexOf(room);
      if (t < T.roomCarry[i][0]) return null;
      const call = C["room_" + room.key];
      const [a, b] = T.roomStream[room.key];
      const full = call.output.text;
      const kind = `on “${TXT.roomTarget}”`;
      if (t >= T.roomFold) return { c: room.c, op: 1, html: head(room.mind, kind, num) + `<div class="short">${esc(full.replace(/\s+/g, " ").slice(0, 60))}…</div>`, anchor: "room", folded: true, rope: i };
      const text = streamed(full, call.deltas, a, b, t);
      const body = text ? `<div class="m">${paras(text)}</div>` : think("");
      return { c: room.c, op: easeOut(prog(t, T.roomCarry[i][0], T.roomCarry[i][0] + 0.25)), html: head(room.mind, kind, num) + body, anchor: "room", active: true, rope: i,
        reserve: head(room.mind, kind, num) + `<div class="m">${paras(full)}</div>` };
    }
    if (id === "call") {
      if (t < T.callThink[0]) return null;
      const text = streamed(TXT.call, C.call_by_name.deltas, ...T.callStream, t);
      const body = text ? `<div class="m">${paras(text)}</div>` : think("");
      return { c: "indigo", op: easeOut(prog(t, T.callThink[0], T.callThink[0] + 0.25)), html: head("Skeptic", "answer", num) + body + (t >= T.callStream[1] ? acts("") : ""), anchor: "call", active: true,
        reserve: head("Skeptic", "answer", num) + `<div class="m">${paras(TXT.call)}</div>` + acts("") };
    }
    return null;
  }

  /** A tag folding down (tied off, or a room answer set aside) shrinks over a moment
   *  instead of jumping: the open tag's content, clipped to a height easing to the chip's. */
  const FOLDS = () => ({ t1: [T.t1Fold, 0.35], t2: [T.t2Fold, 0.35], r0: [T.roomFold, 0.4], r1: [T.roomFold, 0.4], r2: [T.roomFold, 0.4] });
  function tagModelAt(id, t) {
    const f = FOLDS()[id];
    if (f && t >= f[0] && t < f[0] + f[1]) {
      const open = tagModel(id, f[0] - 1e-4), shut = tagModel(id, t);
      if (open && shut) return { ...shut, html: open.html, reserve: null, fold: { k: easeIO(prog(t, f[0], f[0] + f[1])), html: shut.html } };
    }
    return tagModel(id, t);
  }

  function renderThreads(t, c, open, markOp) {
    const mg = $("#mg"), cards = $("#cards");
    const ids = ["t1", "t2", "r0", "r1", "r2", "call"];
    const models = ids.map((id) => ({ id, m: tagModelAt(id, t) })).filter((x) => x.m);
    // build tags, and an invisible twin for the height a streaming tag will need or the chip a folding tag becomes
    const twin = (id, m, attr, html) => `<div class="tag" ${attr}="${id}" style="--tc:var(--${m.c});visibility:hidden"><span class="ring"></span><div class="in">${html}</div></div>`;
    mg.innerHTML = models.map(({ id, m }) =>
      `<div class="tag" data-t="${id}" style="--tc:var(--${m.c});opacity:${fx(m.op * markOp)}"><span class="ring"></span><div class="in">${m.html}</div></div>` +
      (m.reserve ? twin(id, m, "data-r", m.reserve) : "") + (m.fold ? twin(id, m, "data-f", m.fold.html) : "")).join("");
    for (const { id, m } of models) {
      if (!m.fold) continue;
      const a = $(`.tag[data-t="${id}"] .in`, mg), b = $(`.tag[data-f="${id}"] .in`, mg);
      a.style.height = fx(lerp(a.offsetHeight, b.offsetHeight, m.fold.k)) + "px";
      a.style.overflow = "hidden";
    }
    // stack: each tag wants the top of its words' first line; none overlap (gap 14)
    const items = [];
    for (const { id, m } of models) {
      const lines = st.lineRects(m.anchor);
      if (!lines.length) continue;
      const el = $(`.tag[data-t="${id}"]`, mg), twin = $(`.tag[data-r="${id}"]`, mg);
      items.push({ id, m, el, want: lines[0].y - 3, h: Math.max(el.offsetHeight, twin ? twin.offsetHeight : 0), lines, order: ids.indexOf(id) });
    }
    items.sort((a, b) => a.want - b.want || a.order - b.order);
    let floor = -Infinity;
    for (const it of items) {
      const top = Math.max(it.want, floor + 14);
      it.top = top;
      it.el.style.top = fx(top) + "px";
      floor = top + it.h;
    }
    $$(".tag[data-r], .tag[data-f]", mg).forEach((x) => x.remove());
    st.tags = Object.fromEntries(items.map((it) => [it.id, it]));

    // ---- the overlay: stitches under the words, threads to the tags, knots, needles
    let svg = "";
    const T1 = st.tags.t1, T2 = st.tags.t2;
    const colRight = 616;
    const sewK = (t0, t1, i, n) => clamp((t - (t0 + ((t1 - t0 - 0.13) * i) / Math.max(1, n - 1))) / 0.13);
    // thread 1
    {
      const lines = st.lineRects("t1");
      if (lines.length && t >= T.t1Sew[0]) {
        const tied = t >= T.t1Knot, fade = 1 - prog(t, T.t1Fade[0], T.t1Fade[1]);
        const sOp = (tied ? lerp(1, 0.38, prog(t, T.t1Knot, T.t1Knot + 0.5)) : 1) * markOp;
        const lead = 12, sts = stitchesUnder(lines, 41, ["madder"], { lead });
        st.stitchCounts.t1 = sts.length + 2;
        const n = sts.length + 2;
        svg += `<g opacity="${fx(sOp)}">` + crossSVG(lines[0].x + 4.2, lines[0].b + 0.2, 3.7, 2.1, "madder", sewK(...T.t1Sew, 0, n), sewK(...T.t1Sew, 1, n));
        sts.forEach((s, i) => { svg += stitchSVG(s, sewK(...T.t1Sew, i + 2, n)); });
        svg += "</g>";
        const sewn = Math.floor(prog(t, ...T.t1Sew) * n);
        if (t < T.t1Sew[1] && sewn > 1) { const s = sts[Math.min(sts.length - 1, sewn - 2)]; svg += needleAt(s.x1, s.y1, t); }
        if (T1 && t >= T.t1Carry[0]) svg += threadTo("t1", "madder", sts, lines, T1, t, { carry: T.t1Carry[0], tie: T.t1Tie, knot: T.t1Knot, hover: T.t1Hover, fade, markOp, active: T1.m.active });
      }
    }
    // thread 2
    {
      const lines = st.lineRects("t2");
      if (lines.length && t >= T.t2Sew[0]) {
        const tied = t >= T.t2Knot, fade = 1 - prog(t, T.t2Fade[0], T.t2Fade[1]);
        const sOp = (tied ? lerp(1, 0.38, prog(t, T.t2Knot, T.t2Knot + 0.5)) : 1) * markOp;
        const sts = stitchesUnder(lines, 23, ["madder"]);
        st.stitchCounts.t2 = sts.length;
        svg += `<g opacity="${fx(sOp)}">`;
        sts.forEach((s, i) => { svg += stitchSVG(s, sewK(...T.t2Sew, i, sts.length)); });
        svg += "</g>";
        const sewn = Math.floor(prog(t, ...T.t2Sew) * sts.length);
        if (t < T.t2Sew[1] && sewn > 0) { const s = sts[Math.min(sts.length - 1, sewn - 1)]; svg += needleAt(s.x1, s.y1, t); }
        if (T2 && t >= T.t2Carry[0]) svg += threadTo("t2", "madder", sts, lines, T2, t, { carry: T.t2Carry[0], tie: T.t2Tie, knot: T.t2Knot, fade, markOp, active: T2.m.active });
      }
    }
    // the room: one run of stitches in three colours, three threads
    {
      const lines = st.lineRects("room");
      if (lines.length && t >= T.roomSew[0]) {
        const sts = stitchesUnder(lines, 61, ["madder", "indigo", "ochre"], { len: 6.6, gap: 3.6 });
        st.stitchCounts.room = sts.length;
        svg += `<g opacity="${fx(markOp)}">`;
        sts.forEach((s, i) => { svg += stitchSVG(s, sewK(...T.roomSew, i, sts.length)); });
        svg += "</g>";
        ROOM.forEach((r, i) => {
          const tg = st.tags[r.id];
          if (tg && t >= T.roomCarry[i][0]) svg += threadTo(r.id, r.c, sts, lines, tg, t, { carry: T.roomCarry[i][0], markOp, active: t < T.roomFold, slack: 1.006 + i * 0.003 });
        });
      }
    }
    // the mind called by name
    {
      const lines = st.lineRects("call");
      if (lines.length && t >= T.callSew[0]) {
        const sts = stitchesUnder(lines, 77, ["indigo"]);
        st.stitchCounts.call = sts.length;
        svg += `<g opacity="${fx(markOp)}">`;
        sts.forEach((s, i) => { svg += stitchSVG(s, sewK(...T.callSew, i, sts.length)); });
        svg += "</g>";
        const tg = st.tags.call;
        if (tg && t >= T.callCarry[0]) svg += threadTo("call", "indigo", sts, lines, tg, t, { carry: T.callCarry[0], markOp, active: true });
      }
    }
    $("#ov").innerHTML = svg;
  }

  /** The thread from the hole at the column edge to the ring on its tag. */
  function threadTo(key, color, sts, lines, tag, t, o) {
    const last = sts[sts.length - 1] || { x1: lines[0].r, y1: lines[0].b };
    const tr = 616;
    const hole = tr - last.x1 < 10 ? [last.x1 + 3.5, last.y1 - 0.3] : [tr + 7, last.y1 - 0.3];
    const ring = [640, tag.top + 15];
    let rope, phase = null, since = 0;
    if (o.knot != null && t >= o.tie) { phase = "tie"; since = t - o.tie; }
    else if (t - o.carry < 2.4) { phase = "carry"; since = t - o.carry; }
    else if (o.hover != null && t >= o.hover && t - o.hover < 1.2) { phase = "sway"; since = t - o.hover; }
    if (phase === "tie" && since > 1.0) rope = restingRope(hole, ring, ROPE.SLACK_TIED);
    else if (phase) rope = movingRope(key, hole, ring, phase, since, phase === "tie" ? null : o.slack);
    else rope = restingRope(hole, ring, o.slack || ROPE.SLACK);
    // quiet unless you look at it: 26% at rest, 60% under the pointer, full when open or being tied
    let op = 0.26;
    if (o.hoverOn) op = 0.6;
    if (o.active) op = 1;
    if (o.knot != null && t >= o.tie && t < o.knot + 0.9) op = 1;
    if (o.knot != null && t >= o.knot + 0.9) op = lerp(1, 0.26, prog(t, o.knot + 0.9, o.knot + 1.4));
    const carryK = phase === "carry" ? clamp(since / 0.12) : 1;
    op *= (o.fade == null ? 1 : o.fade) * o.markOp * carryK;
    let s = ropeSVG(rope, color, op, hole);
    if (o.knot != null && t >= o.knot) s += `<g opacity="${fx((o.fade == null ? 1 : o.fade) * o.markOp)}">${knotSVG(ring[0] + 0.6, ring[1], 3.7, color, knotScale(t - o.knot))}</g>`;
    return s;
  }

  // ---------------------------------------------------------------- toolbar and ask box
  function renderCards(t) {
    const cards = $("#cards");
    let html = "";
    const selTop = (id) => { const sp = $(`#rows [data-p="${id}"] .sel`); if (!sp) return null; const r = sp.getClientRects()[0]; return r ? st.rel(r).y : null; };
    const toolbar = (hovIdx) => `<div class="row"><span>T</span><span>H1</span><span>H2</span><span>H3</span><span>❝</span><span>•</span><span>1.</span></div><hr>` +
      `<div class="row"><span><b>B</b></span><span><i>I</i></span><span><u>U</u></span><span><s>S</s></span><span>&lt;/&gt;</span></div>` +
      `<div class="sw">${["fg", "madder", "indigo", "ochre", "sage", "plum", "tack", "steel"].map((c) => `<i style="background:var(--${c})"></i>`).join("")}</div><hr>` +
      `<div class="act${hovIdx === 0 ? " hov" : ""}"><b>Find a loose thread <kbd>⌘↵</kbd></b></div>` +
      `<div class="act${hovIdx === 1 ? " hov" : ""}"><b>Ask Inkling</b><small>Your own question</small></div>` +
      `<div class="act"><b>Sketch it</b><small>Stitched out step by step</small></div>`;
    if (t >= T.t2Bar && t < T.t2Click + 0.1) {
      const y = selTop("p3");
      if (y != null) html += `<div class="card tbar" style="left:640px;top:${fx(y - 6)}px;opacity:${fx(easeOut(prog(t, T.t2Bar, T.t2Bar + 0.18)))}">${toolbar(within(t, 48.95, 49.25) ? 0 : -1)}</div>`;
    }
    if (t >= T.roomBar && t < T.roomClick + 0.1) {
      const y = selTop("p2");
      if (y != null) html += `<div class="card tbar" style="left:640px;top:${fx(y - 6)}px;opacity:${fx(easeOut(prog(t, T.roomBar, T.roomBar + 0.18)))}">${toolbar(within(t, 62.55, 62.85) ? 1 : -1)}</div>`;
    }
    if (t >= T.askBox && t < T.roomSend) {
      const y = selTop("p2");
      const on = [true, t >= T.chipSkeptic, t >= T.chipTutor];
      const chips = [["Inkling", "madder"], ["Skeptic", "indigo"], ["Tutor", "ochre"], ["Editor", "sage"], ["Staff engineer", "plum"]].map(([n, c], i) =>
        `<span class="chip${on[i] ? " on" : ""}" style="${on[i] ? `border-color:color-mix(in srgb,var(--${c}) 70%,transparent);background:color-mix(in srgb,var(--${c}) 8%,var(--raised))` : ""}"><span class="dot" style="background:var(--${c})"></span>${n}</span>`).join("");
      const nOn = on.filter(Boolean).length;
      const n = count(S.roomQ, t), typing = t - lastKey(S.roomQ, t) < 0.5;
      const q = n ? `<span style="position:relative">${esc(TXT.roomQ.slice(0, n))}${caretSpan(t, typing)}</span>` : `<span class="ph">Ask Inkling about this…</span>${t >= T.chipTutor + 0.25 ? caretSpan(t, false) : ""}`;
      if (y != null) html += `<div class="card askbox" style="left:640px;top:${fx(y - 4)}px;opacity:${fx(easeOut(prog(t, T.askBox, T.askBox + 0.18)))}"><div class="q"><span class="dot"></span>${q}</div><div class="chips">${chips}</div>` +
        `<div class="hint">${nOn > 1 ? `${nOn} minds will answer side by side · Enter to ask · Esc to cancel` : "Pick up to three minds · Enter to ask · Esc to cancel"}</div></div>`;
    }
    cards.innerHTML = html;
  }

  // ---------------------------------------------------------------- chat
  function renderChat(t) {
    const k = easeOut(prog(t, T.chatIn[0], T.chatIn[1])) * (1 - easeIO(prog(t, T.chatOut[0], T.chatOut[1])));
    const ch = $("#chat");
    $("#chat-btn").classList.toggle("on", k > 0.5);
    if (k <= 0.001) { ch.style.display = "none"; return; }
    ch.style.display = "flex";
    ch.style.transform = `translateX(${fx((1 - k) * 102)}%)`;
    const open = NUM(t).length, visible = 5;
    let log = "";
    if (t < T.chatSend) {
      log += `<div class="hello">I’ve read the whole note and its threads. Ask me anything about it.</div><div class="pills"><span>What am I still misunderstanding?</span><span>What have we figured out so far?</span><span>What should I work on next?</span></div>`;
    } else {
      log += `<div class="me">${esc(TXT.chatQ)}</div>`;
      const text = streamed(TXT.chatA, C.chat.deltas, ...T.chatStream, t);
      log += text ? `<div class="ans">${paras(text)}</div>` : `<div><span class="dot" style="opacity:${fx(0.3 + 0.6 * (0.5 + 0.5 * Math.sin((t / 1.1) * Math.PI * 2)))}"></span></div>`;
    }
    const n = count(S.chatQ, t), typing = t - lastKey(S.chatQ, t) < 0.5;
    const input = t < T.chatSend && n ? `<span>${esc(TXT.chatQ.slice(0, n))}</span>${caretSpan(t, typing)}` : `${t >= T.chatIn[1] ? caretSpan(t, false) : ""}<span class="ph">Ask about this note…</span>`;
    ch.innerHTML = `<header><div class="ti"><div class="nm"><span class="dot"></span>Inkling</div><small>On “${esc(TXT.title)}”. Reading this note: ${visible} threads (${open} open)</small></div><span class="gb">Copy</span><span class="gb">Clear</span><span class="gb">✕</span></header>` +
      `<div class="log">${log}</div><footer><span class="tx">${input}</span><span class="send"><svg viewBox="0 0 12 12"><path d="M6 10V2.5M2.8 5.5L6 2.3l3.2 3.2" style="fill:none;stroke:var(--surface);stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round"/></svg></span></footer>`;
  }

  // ---------------------------------------------------------------- cold open
  function renderOpen(t, c) {
    const layer = $("#open");
    if (t >= T.fly1 + 0.05) { layer.style.display = "none"; return; }
    layer.style.display = "block";
    const wmEl = $("#open-wm");
    const W0 = 900, vb = WMS.vb.split(" ").map(Number), H0 = (W0 * vb[3]) / vb[2];
    const x0 = 960 - W0 / 2, y0 = 470 - H0 / 2;
    // the flight into the title bar: the target is the title bar's wordmark under the WIDE camera
    const cw = cam(T.fly1);
    const tbH = 26, tbW = (tbH * vb[2]) / vb[3];
    const [tx, ty] = toScreen(cw, 112, 9 + 18 * 0);
    const k = easeIO(prog(t, T.fly0, T.fly1));
    const sc = lerp(1, (tbW * cw.s) / W0, k);
    wmEl.style.transform = `translate(${fx(lerp(x0, tx, k))}px, ${fx(lerp(y0, ty, k))}px) scale(${fx(sc)})`;
    wmEl.style.width = W0 + "px"; wmEl.style.height = fx(H0) + "px";
    const fillK = easeIO(prog(t, T.fill0, T.fill1));
    wmEl.innerHTML = wordmarkSVG(t, { fillK, sewn: t >= T.fill1, stitchOpacity: lerp(1, 0, prog(t, T.fill0 + 0.3, T.fill1 + 0.6)), knotScale: knotScale(t - T.knotPop) });
    const line = $("#open-line");
    line.style.top = "720px";
    const lk = easeOut(prog(t, T.line0, T.line1)) * (1 - easeIO(prog(t, T.lineOut0, T.lineOut1)));
    line.style.opacity = String(fx(lk));
    line.style.transform = `translateY(${fx((1 - easeOut(prog(t, T.line0, T.line1))) * 10)}px)`;
  }

  // ---------------------------------------------------------------- close
  function renderClose(t) {
    const layer = $("#close");
    if (t < T.icon[0]) { layer.style.display = "none"; return; }
    layer.style.display = "block";
    const ik = prog(t, T.icon[0], T.icon[1]);
    const pop = ik < 0.7 ? lerp(0.86, 1.03, easeOut(ik / 0.7)) : lerp(1.03, 1, easeIO((ik - 0.7) / 0.3));
    const icon = $("#close-icon");
    icon.style.opacity = String(fx(easeOut(Math.min(1, ik * 1.6))));
    icon.style.transform = `scale(${fx(pop)})`;
    const wm = $("#close-wm");
    const wk = easeOut(prog(t, T.closeWm[0], T.closeWm[1]));
    wm.style.opacity = String(fx(wk));
    if (!wm._built) { wm._built = true; }
    wm.innerHTML = wordmarkSVG(t, { fillK: 1, sewn: true, stitchOpacity: 0, knotScale: knotScale(t - T.closeKnot) });
    const line = $("#close-line");
    const lk = easeOut(prog(t, T.closeLine[0], T.closeLine[1]));
    line.style.opacity = String(fx(lk));
    line.style.transform = `translateY(${fx((1 - lk) * 10)}px)`;
  }

  // ---------------------------------------------------------------- captions
  function renderCaption(t) {
    const caps = $("#caps");
    const cap = CAPS.find((x) => t >= x.t[0] - 0.01 && t < x.t[1] + 0.3);
    if (!cap) { caps.innerHTML = ""; return; }
    const kin = easeOut(prog(t, cap.t[0], cap.t[0] + 0.4)), kout = easeIn(prog(t, cap.t[1], cap.t[1] + 0.28));
    const i = cap.text.indexOf(cap.u);
    const html = i < 0 ? esc(cap.text) : esc(cap.text.slice(0, i)) + `<span class="u">${esc(cap.u)}</span>` + esc(cap.text.slice(i + cap.u.length));
    caps.innerHTML = `<div class="cap" style="opacity:${fx(kin * (1 - kout))};transform:translateY(${fx((1 - kin) * 8 - kout * 6)}px)">${html}<svg class="cu"></svg></div>`;
    const capEl = $(".cap", caps), u = $(".u", capEl);
    if (u) {
      const cr = capEl.getBoundingClientRect(), ur = u.getBoundingClientRect();
      const x0 = ur.left - cr.left, x1 = ur.right - cr.left, y = ur.bottom - cr.top - 2;
      const sts = stitchesUnder([{ x: x0, r: x1, b: y }], cap.t[0] * 100, ["madder"], { len: 11, gap: 6 }).map((s) => ({ ...s, w: 3.1 }));
      const svg = $(".cu", capEl);
      svg.setAttribute("width", "10"); svg.setAttribute("height", "10");
      const n = sts.length, s0 = cap.t[0] + 0.3, s1 = s0 + 0.45;
      svg.innerHTML = sts.map((s, j) => stitchSVG(s, clamp((t - (s0 + ((s1 - s0) * j) / Math.max(1, n - 1))) / 0.12))).join("");
    }
  }

  // ---------------------------------------------------------------- keys pressed
  // A shortcut shows as keycaps at the foot of the frame while it is pressed.
  const KEYS = [{ t: T.chatKey, keys: ["⌘", "J"], label: "Chat" }];
  function renderKeys(t) {
    const el = $("#keys");
    const k = KEYS.find((x) => t >= x.t - 0.3 && t < x.t + 0.9);
    if (!k) { el.style.display = "none"; return; }
    const op = easeOut(prog(t, k.t - 0.3, k.t - 0.1)) * (1 - easeIn(prog(t, k.t + 0.6, k.t + 0.9)));
    const down = t >= k.t && t < k.t + 0.16;
    el.style.display = "flex";
    el.style.opacity = String(fx(op));
    el.innerHTML = k.keys.map((x) => `<span class="key${down ? " down" : ""}">${esc(x)}</span>`).join("") + `<span class="kl">${esc(k.label)}</span>`;
  }

  // ---------------------------------------------------------------- the pointer
  const ARROW = `<svg width="28" height="36" viewBox="0 0 28 36"><path d="M3 2 L3 26 L9 20.5 L13.2 30.2 L17.4 28.4 L13.3 19.1 L21.5 19.1 Z" fill="#111" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>`;
  const IBEAM = `<svg width="28" height="36" viewBox="0 0 28 36"><path d="M9 4.5c2 0 3.5.6 4.5 1.8 1-1.2 2.5-1.8 4.5-1.8M9 31.5c2 0 3.5-.6 4.5-1.8 1 1.2 2.5 1.8 4.5 1.8M13.5 6.3v23.4" fill="none" stroke="#fff" stroke-width="3.6" stroke-linecap="round"/><path d="M9 4.5c2 0 3.5.6 4.5 1.8 1-1.2 2.5-1.8 4.5-1.8M9 31.5c2 0 3.5-.6 4.5-1.8 1 1.2 2.5 1.8 4.5 1.8M13.5 6.3v23.4" fill="none" stroke="#111" stroke-width="1.5" stroke-linecap="round"/></svg>`;
  function parseTarget(spec) {
    // "@name" | "@name+[dx,dy]" | "@name:fx,fy" (a point inside a measured box)
    const m = spec.match(/^@([a-z0-9]+)(?::([\d.]+),([\d.]+))?(?:\+\[(-?[\d.]+),(-?[\d.]+)\])?$/);
    const base = TARGETS[m[1] + (m[2] ? `:${m[2]},${m[3]}` : "")] || TARGETS[m[1]];
    if (!base) return [0, 0];
    return [base[0] + (m[4] ? +m[4] : 0), base[1] + (m[5] ? +m[5] : 0)];
  }
  function pointerAt(t) {
    let pos = null, ibeam = false;
    for (const mv of MOVE) {
      if (t < mv.t[0]) break;
      if (mv.live) {
        const p = liveSelEnd();
        if (p) pos = p;
        ibeam = !!mv.ibeam;
        continue;
      }
      const from = mv.from ? parseTarget(mv.from) : pos || parseTarget(mv.to);
      const to = parseTarget(mv.to);
      const k = easeIO(prog(t, mv.t[0], mv.t[1]));
      // a hand moves in a slight arc
      const dx = to[0] - from[0], dy = to[1] - from[1], bow = Math.sin(Math.PI * k) * 0.08;
      pos = [lerp(from[0], to[0], k) - dy * bow, lerp(from[1], to[1], k) + dx * bow];
      ibeam = !!mv.ibeam && k > 0.6;
    }
    return { pos, ibeam };
  }
  function liveSelEnd() {
    const sp = $$("#rows .sel");
    if (!sp.length) return null;
    const rs = sp.flatMap((s) => Array.from(s.getClientRects()));
    const r = rs[rs.length - 1], w = $("#win").getBoundingClientRect(), s = cam(lastT).s;
    return [(r.right - w.left) / s, (r.top - w.top) / s + r.height / s / 2];
  }
  function renderPointer(t, c) {
    const el = $("#cursor");
    const vis = PTR.find((p) => t >= p.show[0] && t < p.show[1]);
    if (!vis) { el.style.display = "none"; return; }
    const { pos, ibeam } = pointerAt(t);
    if (!pos) { el.style.display = "none"; return; }
    const fade = Math.min(prog(t, vis.show[0], vis.show[0] + 0.15), 1 - prog(t, vis.show[1] - 0.15, vis.show[1]));
    let press = 0;
    for (const p of PRESS) { const end = RELEASE[p] || p + 0.11; if (t >= p && t < end) press = 1; }
    const [sx, sy] = toScreen(c, pos[0], pos[1]);
    el.style.display = "block";
    el.style.opacity = String(fx(fade));
    el.innerHTML = ibeam ? IBEAM : ARROW;
    const ox = ibeam ? -13.5 : -3, oy = ibeam ? -18 : -2;
    el.style.transform = `translate(${fx(sx + ox)}px, ${fx(sy + oy)}px) scale(${press ? 0.9 : 1})`;
  }

  // ======================================================================== targets
  // The pointer heads for things on the page; where they are is read off the page itself,
  // at the moment the pointer gets there.
  function measureIn(t, fn) {
    renderAt(t);
    const w = $("#win").getBoundingClientRect(), s = cam(t).s;
    const r = fn();
    if (!r) return null;
    return [(r[0] - w.left) / s, (r[1] - w.top) / s];
  }
  const center = (el, fxk = 0.5, fyk = 0.5) => { if (!el) return null; const r = el.getBoundingClientRect(); return [r.left + r.width * fxk, r.top + r.height * fyk]; };
  function resolveTargets() {
    const spanOf = (sel) => { const sp = $$(sel); if (!sp.length) return null; return sp.flatMap((s) => Array.from(s.getClientRects())); };
    const set = (name, t, fn) => { const p = measureIn(t, fn); if (p) TARGETS[name] = p; };
    set("t1tag:.55,.62", 27.5, () => center($('.tag[data-t="t1"]'), 0.55, 0.62));
    set("t1tag", 27.5, () => center($('.tag[data-t="t1"]')));
    set("sketchbtn", 28.8, () => center($('.tag[data-t="t1"] .acts .pri')));
    set("e1sel", 38.3, () => { const r = spanOf("#rows .sel"); return r && [r[0].left + r[0].width * 0.5, r[0].top + r[0].height * 0.5]; });
    set("e2start", 39.6, () => { const r = spanOf("#rows .sel"); return r && [r[0].left, r[0].top + r[0].height * 0.5]; });
    set("t2start", 47.5, () => { const r = spanOf("#rows .sel"); return r && [r[0].left, r[0].top + r[0].height * 0.5]; });
    set("findbtn", 48.9, () => center($$(".tbar .act")[0], 0.32, 0.5));
    set("t2reply", 54.2, () => center($('.tag[data-t="t2"] .reply'), 0.3, 0.5));
    set("roomstart", 61.5, () => { const r = spanOf("#rows .sel"); return r && [r[0].left, r[0].top + r[0].height * 0.5]; });
    set("askbtn", 62.6, () => center($$(".tbar .act")[1], 0.25, 0.4));
    set("chip1", 63.2, () => center($$(".askbox .chip")[1]));
    set("chip2", 63.6, () => center($$(".askbox .chip")[2]));
    set("tell", 72.4, () => center($$("#nh .it")[0]));
    set("hide", 87.8, () => center($$("#nh .it")[$$("#nh .it").length - 1]));
  }

  // ======================================================================== sound events
  // What the audio renderer needs to put sound on the right frames.
  function collectEvents() {
    const ev = [];
    // The typing on screen is sped up; the sound of it is not. A key sounds at most every
    // 80 ms (about the pace of a quick typist), so fast passages read as typing, not a buzz.
    const keysOf = (times, text, gain) => {
      let last = -1;
      times.forEach((t, i) => {
        if (t - last < 0.08) return;
        last = t;
        ev.push({ t, kind: text[i] === " " ? "space" : "key", gain: gain || 1 });
      });
    };
    keysOf(S.title, TXT.title); keysOf(S.p1, TXT.p1); keysOf(S.p2, TXT.p2); keysOf(S.p3, TXT.p3); keysOf(S.p4, TXT.p4);
    keysOf(S.e1, EDIT1.to); keysOf(S.e2, EDIT2.to); keysOf(S.reply, TXT.reply, 0.8); keysOf(S.roomQ, TXT.roomQ, 0.8);
    keysOf(S.brief, TXT.brief, 0.8); keysOf(S.chatQ, TXT.chatQ, 0.8); keysOf(S.p5, TXT.p5);
    [T.enter.p1, T.enter.p2, T.enter.p3, T.enter.p4, T.enter.p5, T.roomSend, T.briefSet, T.chatSend, T.t2Send].forEach((t) => ev.push({ t, kind: "enter" }));
    ev.push({ t: T.chatKey, kind: "chord" });
    PRESS.forEach((t) => ev.push({ t, kind: "click" }));
    // stitches: one soft tick per stitch as it is sewn
    // stitches: a soft tick as they are sewn, no closer than 45 ms apart
    const sew = (t0, t1, n, gain) => {
      let last = -1;
      for (let i = 0; i < n; i++) {
        const t = t0 + ((t1 - t0 - 0.13) * i) / Math.max(1, n - 1);
        if (t - last < 0.045) continue;
        last = t;
        ev.push({ t, kind: "stitch", gain: gain || 1, i });
      }
    };
    WMS.glyphs.forEach((g, gi) => { const a = T.sew0 + gi * T.glyphGap; const n = Math.round(g.stitches.length / 6); for (let i = 0; i < n; i++) ev.push({ t: a + (T.glyphDur * i) / n, kind: "stitch", gain: 0.7, i }); });
    renderAt(T.t1Sew[1] + 0.2); sew(...T.t1Sew, st.stitchCounts.t1 || 40);
    renderAt(T.t2Sew[1] + 0.2); sew(...T.t2Sew, st.stitchCounts.t2 || 26);
    renderAt(T.roomSew[1] + 0.2); sew(...T.roomSew, st.stitchCounts.room || 14);
    renderAt(T.callSew[1] + 0.2); sew(...T.callSew, st.stitchCounts.call || 30);
    SKETCH.steps.forEach((s, i) => { const [a, b] = T.skSteps[i]; const n = s.items.filter((x) => x.kind === "stitch").length; for (let j = 0; j < n; j += 2) ev.push({ t: a + (b - a) * 0.86 * (j / n), kind: "stitch", gain: 0.6, i: j }); });
    [T.t1Motif, T.t2Motif].forEach((t) => { for (let i = 0; i < 9; i++) ev.push({ t: t + i * 0.04, kind: "stitch", gain: 0.8, i }); });
    // threads drawn through the cloth, knots snugging tight
    [T.t1Carry[0], T.t2Carry[0], ...T.roomCarry.map((x) => x[0]), T.callCarry[0]].forEach((t) => ev.push({ t, kind: "pull", gain: 0.6 }));
    [T.t1Tie, T.t2Tie, T.knotPull].forEach((t) => ev.push({ t, kind: "pull", gain: 1 }));
    [T.knotPop, T.t1Knot, T.t2Knot, T.closeKnot].forEach((t) => ev.push({ t, kind: "pop", gain: 1 }));
    [T.t1Arrive, T.t2Think[0], T.roomSend, T.callArrive].forEach((t) => ev.push({ t, kind: "pop", gain: 0.35 }));
    // paper and cloth: the window, the panels
    ev.push({ t: T.win0, kind: "paper", gain: 0.9 }, { t: T.chatIn[0], kind: "slide", gain: 0.7 }, { t: T.chatOut[0], kind: "slide", gain: 0.6 },
      { t: T.t2Bar, kind: "paper", gain: 0.25 }, { t: T.roomBar, kind: "paper", gain: 0.25 }, { t: T.askBox, kind: "paper", gain: 0.25 }, { t: T.briefOpen[0], kind: "paper", gain: 0.3 },
      { t: T.hide[0], kind: "lift", gain: 0.6 }, { t: T.show[0], kind: "lift", gain: 0.5 }, { t: T.lamp[0], kind: "lamp", gain: 0.8 }, { t: T.winOut[0], kind: "paper", gain: 0.7 }, { t: T.icon[0], kind: "settle", gain: 0.8 });
    ev.sort((a, b) => a.t - b.t);
    // overlapping runs of stitches (the wordmark's letters, a sketch's lines) thin out too
    let lastStitch = -1;
    return ev.filter((e) => {
      if (e.kind !== "stitch") return true;
      if (e.t - lastStitch < 0.045) return false;
      lastStitch = e.t;
      return true;
    });
  }

  // ======================================================================== boot
  async function boot() {
    const fonts = ['400 17px "Fraunces"', '500 30px "Fraunces"', '500 17px "Fraunces"', '400 14px "Figtree"', '500 14px "Figtree"', '600 14px "Figtree"', '700 14px "Figtree"'];
    await Promise.all(fonts.map((f) => document.fonts.load(f)));
    await document.fonts.ready;
    $("#desk").style.setProperty("--weave-l", `url("${ASSETS.WEAVE.light}")`);
    $("#stage").style.setProperty("--weave-l", `url("${ASSETS.WEAVE.light}")`);
    $("#stage").style.setProperty("--weave-d", `url("${ASSETS.WEAVE.dark}")`);
    ["#desk-in"].forEach((s) => { $(s).style.backgroundImage = "var(--weave-l)"; $(s).style.backgroundColor = "var(--bg)"; $(s).style.backgroundSize = "48px 48px"; });
    ["#desk-in-dark"].forEach((s) => { $(s).style.backgroundImage = "var(--weave-d)"; $(s).style.backgroundColor = "var(--bg)"; $(s).style.backgroundSize = "48px 48px"; $(s).style.opacity = "0"; });
    $("#tb-wm").innerHTML = WM_STATIC();
    buildWordmark();
    buildSketch();
    resolveTargets();
    window.FILM.events = collectEvents();
    // wait for the icon image too
    const img = $("#close-icon img");
    if (!img.complete) await new Promise((r) => { img.onload = r; img.onerror = r; });
    window.FILM.ready = true;
    const q = new URLSearchParams(location.search);
    if (q.has("t")) renderAt(+q.get("t"));
    else if (q.has("play")) {
      const t0 = performance.now() - (+q.get("play") || 0) * 1000;
      const tick = () => { const t = (performance.now() - t0) / 1000; renderAt(Math.min(t, DUR)); if (t < DUR) requestAnimationFrame(tick); };
      requestAnimationFrame(tick);
    } else renderAt(0);
  }
  window.FILM = { FPS, DUR, T, renderAt, ready: false, events: null };
  boot().catch((e) => { window.FILM.error = String(e && e.stack || e); console.error(e); });
})();

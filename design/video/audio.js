/* The walkthrough's sound, made from the material: needle, cloth and thread.
 *
 * Synthesised sample by sample from FILM.events (film.js), so every sound sits exactly on
 * the frame it belongs to and moves across the stereo field with the thing on screen.
 *   - The needle is the rhythm section: pierce clicks and cloth taps, and a sewing machine
 *     whose speed follows the stitching on screen and stops when it stops.
 *   - Tension is a pulled thread: a bowed string that rises as a thread goes taut and lets
 *     go into the knot.
 *   - A short motif for a loose thread (E G A F#, in E dorian) stays on the open 2nd while a
 *     thread is open, and comes home to E only when one is tied off.
 *   - Each mind has its own instrument, and "speaks" in it while its words appear:
 *     Inkling a plucked thread, the Skeptic a low muted reed, the Socratic tutor a breathy
 *     flute, the Coach a blunt wooden mallet.
 *   - Silence is used on purpose: a held breath before each tie-off, a hard stop on a snip,
 *     and almost nothing while the threads are hidden.
 * No pad, no four-on-the-floor, no chord progression: the music is threads and knots.
 *
 * SCORE.render(events) makes two stems, "music" and "fx"; mix.sh balances and levels them.
 */
(function () {
  "use strict";
  const SR = 48000;
  const out = {};
  let N = 0;
  const BUS = {};
  const midi = (m) => 440 * Math.pow(2, (m - 69) / 12);
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  // ---- a repeatable noise source
  let seed = 12345;
  const rnd = () => { seed ^= seed << 13; seed >>>= 0; seed ^= seed >>> 17; seed ^= seed << 5; seed >>>= 0; return (seed / 4294967296) * 2 - 1; };
  // ---- RBJ biquads, per-sample
  function biquad(type, f, q) {
    let B0, B1, B2, A1, A2;
    const set = (f) => {
      const w = (2 * Math.PI * clamp(f, 10, SR * 0.45)) / SR, c = Math.cos(w), s = Math.sin(w), a = s / (2 * q);
      let b0, b1, b2, a0, a1, a2;
      if (type === "lp") { b0 = (1 - c) / 2; b1 = 1 - c; b2 = (1 - c) / 2; a0 = 1 + a; a1 = -2 * c; a2 = 1 - a; }
      else if (type === "hp") { b0 = (1 + c) / 2; b1 = -(1 + c); b2 = (1 + c) / 2; a0 = 1 + a; a1 = -2 * c; a2 = 1 - a; }
      else { b0 = a; b1 = 0; b2 = -a; a0 = 1 + a; a1 = -2 * c; a2 = 1 - a; } // band-pass, 0 dB peak
      B0 = b0 / a0; B1 = b1 / a0; B2 = b2 / a0; A1 = a1 / a0; A2 = a2 / a0;
    };
    set(f);
    let x1 = 0, x2 = 0, y1 = 0, y2 = 0;
    const fn = (x) => { const y = B0 * x + B1 * x1 + B2 * x2 - A1 * y1 - A2 * y2; x2 = x1; x1 = x; y2 = y1; y1 = y; return y; };
    fn.set = set;
    return fn;
  }
  // ---- writing into a bus, with equal-power panning
  function bus(name) { if (!BUS[name]) BUS[name] = [new Float32Array(N), new Float32Array(N)]; return BUS[name]; }
  function put(name, t, mono, gain, pan) {
    const [L, R] = bus(name);
    const i0 = Math.round(t * SR);
    const panAt = typeof pan === "function" ? pan : () => pan || 0;
    for (let i = 0; i < mono.length; i++) {
      const j = i0 + i;
      if (j < 0 || j >= N) continue;
      const p = clamp(panAt(i / SR), -1, 1), a = ((p + 1) * Math.PI) / 4;
      const v = mono[i] * gain;
      L[j] += v * Math.cos(a); R[j] += v * Math.sin(a);
    }
  }
  const env = (n, att, dec, curve) => { const e = new Float32Array(n); const A = Math.max(1, Math.round(att * SR)); for (let i = 0; i < n; i++) e[i] = i < A ? i / A : Math.pow(Math.max(0, 1 - (i - A) / Math.max(1, n - A)), curve || 2); return e; };

  // ================================================================ instruments
  /** Karplus-Strong: a plucked thread. Fractional delay keeps it in tune. */
  const ksCache = new Map();
  function pluck(f, dur, bright, decay) {
    const key = [f.toFixed(2), dur, bright, decay].join(":");
    if (ksCache.has(key)) return ksCache.get(key);
    const n = Math.round(dur * SR), y = new Float32Array(n);
    const L = SR / f, Li = Math.floor(L), fr = L - Li;
    const buf = new Float32Array(Li + 2);
    let lp = 0;
    for (let i = 0; i < buf.length; i++) { lp = lp + (rnd() - lp) * (0.25 + 0.7 * bright); buf[i] = lp; }
    let idx = 0, prev = 0;
    const damp = decay || 0.996;
    let dc = 0;
    for (let i = 0; i < n; i++) {
      const a = buf[idx % buf.length], b = buf[(idx + 1) % buf.length];
      const s = a * (1 - fr) + b * fr;
      const nv = damp * (0.5 * (s + prev) * (0.5 + 0.5 * bright) + s * (0.5 - 0.5 * bright));
      prev = s;
      buf[idx % buf.length] = nv;
      idx++;
      dc += (nv - dc) * 0.002;
      y[i] = (nv - dc) * Math.min(1, i / 24);
    }
    // a little body: the thread over a wooden hoop
    const body = biquad("bp", 220, 1.2), body2 = biquad("bp", 520, 2.0);
    for (let i = 0; i < n; i++) y[i] = y[i] * 0.85 + body(y[i]) * 0.6 + body2(y[i]) * 0.3;
    ksCache.set(key, y);
    return y;
  }
  /** A bowed thread, gliding from f0 to f1: the sound of a thread going taut. */
  function bowed(f0, f1, dur, opts = {}) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    let ph1 = 0, ph2 = 0;
    const lp = biquad("lp", 900, 0.7), body = biquad("bp", 300, 1.4), bowN = biquad("bp", 2400, 1.5);
    const att = opts.att || Math.min(0.12, dur * 0.4), rel = opts.rel || 0.05;
    for (let i = 0; i < n; i++) {
      const t = i / SR, k = t / dur;
      const glide = opts.curve === "lin" ? k : k * k * (3 - 2 * k);
      const f = f0 * Math.pow(f1 / f0, glide) * (1 + 0.0035 * Math.sin(2 * Math.PI * 5.2 * t) * Math.min(1, t * 4));
      ph1 += f / SR; ph2 += (f * 1.0035) / SR;
      const saw = (ph1 % 1) * 2 - 1 + ((ph2 % 1) * 2 - 1) * 0.7;
      const e = Math.min(1, t / att) * Math.min(1, (dur - t) / rel) * (opts.cresc ? 0.35 + 0.65 * k : 1);
      const bright = 0.4 + 0.6 * (opts.cresc ? k : 0.6);
      y[i] = (lp(saw) * (0.6 + bright * 0.5) + body(saw) * 0.4 + bowN(rnd()) * 0.06 * e) * e;
    }
    return y;
  }
  /** The Socratic tutor: a breathy flute. */
  function flute(f, dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    const br = biquad("bp", f * 2, 3);
    let ph = 0;
    for (let i = 0; i < n; i++) {
      const t = i / SR, e = Math.min(1, t / 0.05) * Math.min(1, (dur - t) / 0.12);
      ph += (f * (1 + 0.005 * Math.sin(2 * Math.PI * 5 * t) * Math.min(1, t * 3))) / SR;
      y[i] = (Math.sin(2 * Math.PI * ph) + 0.22 * Math.sin(4 * Math.PI * ph) + br(rnd()) * 0.5) * e;
    }
    return y;
  }
  /** The Skeptic: a low reed, muted, with a doubting little bend down. */
  function reed(f, dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    const lp = biquad("lp", 750, 0.9), lp2 = biquad("lp", 1400, 0.6);
    let ph = 0;
    for (let i = 0; i < n; i++) {
      const t = i / SR, e = Math.min(1, t / 0.015) * Math.exp(-t * 3.2) * Math.min(1, (dur - t) / 0.04);
      ph += (f * (1 - 0.03 * Math.min(1, t / 0.12))) / SR;
      const sq = (ph % 1) < 0.5 ? 1 : -1;
      y[i] = lp2(lp(sq)) * e * 1.4;
    }
    return y;
  }
  /** The Coach: a blunt wooden mallet. */
  function mallet(f, dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    let p1 = 0, p2 = 0;
    for (let i = 0; i < n; i++) {
      const t = i / SR;
      const fp = f * (1 + 0.6 * Math.exp(-t * 90));
      p1 += fp / SR; p2 += (f * 2.76) / SR;
      y[i] = Math.sin(2 * Math.PI * p1) * Math.exp(-t * 7) + 0.35 * Math.sin(2 * Math.PI * p2) * Math.exp(-t * 26) + (i < 120 ? rnd() * 0.3 * (1 - i / 120) : 0);
    }
    return y;
  }
  /** Small bells (the Editor) and a dry block (Staff engineer), a brass breath (Investor): chips only. */
  function bell(f, dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    for (let i = 0; i < n; i++) { const t = i / SR; const m = Math.sin(2 * Math.PI * f * 3.5 * t) * 2.2 * Math.exp(-t * 6); y[i] = Math.sin(2 * Math.PI * f * t + m) * Math.exp(-t * 4.5); }
    return y;
  }
  function block(f, dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    for (let i = 0; i < n; i++) { const t = i / SR; y[i] = (Math.sin(2 * Math.PI * f * t) * 0.8 + Math.sin(2 * Math.PI * f * 1.58 * t) * 0.5) * Math.exp(-t * 38); }
    return y;
  }
  function brass(f, dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    const lp = biquad("lp", 900, 1.2);
    let ph = 0;
    for (let i = 0; i < n; i++) { const t = i / SR; ph += f / SR; const e = Math.min(1, t / 0.06) * Math.exp(-t * 3); y[i] = lp((ph % 1) * 2 - 1) * e; }
    return y;
  }
  const VOICE = {
    inkling: { make: (f, d) => pluck(f, d, 0.55, 0.997), base: 64, span: [0, 7], gain: 0.5 },
    skeptic: { make: (f, d) => reed(f, d), base: 40, span: [0, 7], gain: 0.42 },
    tutor: { make: (f, d) => flute(f, d), base: 76, span: [0, 7], gain: 0.16 },
    coach: { make: (f, d) => mallet(f, d), base: 52, span: [0, 5], gain: 0.5 },
    staff: { make: (f, d) => block(f, d), base: 71, span: [0, 4], gain: 0.4 },
    editor: { make: (f, d) => bell(f, d), base: 83, span: [0, 4], gain: 0.16 },
    investor: { make: (f, d) => brass(f, d), base: 47, span: [0, 4], gain: 0.22 },
  };
  // E dorian
  const SCALE = [0, 2, 3, 5, 7, 9, 10];
  const deg = (base, d) => base + SCALE[((d % 7) + 7) % 7] + 12 * Math.floor(d / 7);

  // ================================================================ foley
  function noiseBurst(dur, filters, curve) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    const fs = filters.map(([t, f, q]) => biquad(t, f, q));
    for (let i = 0; i < n; i++) { let v = rnd(); for (const F of fs) v = F(v); y[i] = v * Math.pow(1 - i / n, curve || 3); }
    return y;
  }
  const FX = {
    pierce: () => mix([noiseBurst(0.004, [["hp", 2500, 0.7]], 2), 1.2], [noiseBurst(0.09, [["bp", 5200, 12]], 4), 0.9], [noiseBurst(0.03, [["lp", 900, 0.7]], 3), 0.5]),
    click: () => mix([noiseBurst(0.003, [["hp", 1800, 0.7]], 2), 1], [noiseBurst(0.035, [["bp", 4300, 9]], 4), 0.6]),
    tick: () => mix([noiseBurst(0.0025, [["bp", 3000, 1.5]], 2), 1], [noiseBurst(0.02, [["bp", 1800, 6]], 4), 0.3]),
    tap: () => mix([noiseBurst(0.03, [["lp", 1300, 0.7]], 3), 0.9], [sine(160, 0.03, 60), 0.35], [noiseBurst(0.002, [["hp", 3000, 0.7]], 2), 0.25]),
    key: () => mix([noiseBurst(0.05, [["lp", 800, 0.8]], 3), 1], [sine(110, 0.06, 45), 0.7], [noiseBurst(0.004, [["bp", 2600, 3]], 2), 0.6]),
    thump: () => mix([sine(120, 0.07, 40, 70), 0.9], [noiseBurst(0.04, [["lp", 500, 0.7]], 3), 0.6]),
    stitch: () => mix([noiseBurst(0.002, [["hp", 3500, 0.7]], 2), 0.8], [noiseBurst(0.018, [["hp", 4200, 0.8]], 3), 0.5], [noiseBurst(0.012, [["lp", 700, 0.7]], 3), 0.35]),
    tack: () => mix([noiseBurst(0.012, [["hp", 3800, 0.7]], 3), 0.5], [noiseBurst(0.02, [["lp", 600, 0.7]], 3), 0.4]),
    pop: () => mix([sine(520, 0.05, 30, 900), 0.6], [noiseBurst(0.01, [["bp", 1500, 2]], 3), 0.4]),
    drop: () => mix([sine(900, 0.035, 20, 1600), 0.7], [noiseBurst(0.07, [["lp", 1100, 0.9]], 3), 0.45], [sine(220, 0.05, 40), 0.25]),
  };
  function sine(f, dur, decay, f2) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    let ph = 0;
    for (let i = 0; i < n; i++) { const k = i / n; const ff = f2 ? f2 + (f - f2) * Math.exp(-k * 6) : f; ph += ff / SR; y[i] = Math.sin(2 * Math.PI * ph) * Math.exp(-(i / SR) * decay); }
    return y;
  }
  function mix(...parts) {
    const n = Math.max(...parts.map(([p]) => p.length)), y = new Float32Array(n);
    for (const [p, g] of parts) for (let i = 0; i < p.length; i++) y[i] += p[i] * g;
    return y;
  }
  /** A cloth swish: noise through a band that sweeps. */
  function swish(dur, f0, f1, q) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    const F = biquad("bp", f0, q || 0.8);
    for (let i = 0; i < n; i++) {
      const k = i / n;
      if ((i & 31) === 0) F.set(f0 * Math.pow(f1 / f0, k));
      y[i] = F(rnd()) * Math.sin(Math.PI * k) * Math.sin(Math.PI * k);
    }
    return y;
  }
  /** The sewing machine: needle-down clicks at its rate, the take-up tick between, a felt
   *  thump on every third stitch (it falls in threes, never four-square), and the motor. */
  function machine(t0, t1, r0, r1, pan0, pan1, gain, opts = {}) {
    const dur = t1 - t0;
    let t = 0, k = 0;
    const panAt = (u) => pan0 + (pan1 - pan0) * clamp(u / dur, 0, 1);
    while (t < dur) {
      const u = t / dur, r = r0 + (r1 - r0) * (u * u * (3 - 2 * u));
      const acc = k % 3 === 0;
      put("fx", t0 + t, FX.click(), gain * (acc ? 0.5 : 0.32) * (opts.soft ? 0.6 : 1), panAt(t));
      put("fx", t0 + t + 0.45 / r, FX.tick(), gain * 0.12, panAt(t) + 0.08);
      if (acc) put("fx", t0 + t, FX.thump(), gain * 0.18 * (opts.soft ? 0.5 : 1), panAt(t));
      if (opts.triple) { put("fx", t0 + t + 0.012, FX.click(), gain * 0.18, panAt(t) - 0.2); put("fx", t0 + t + 0.021, FX.click(), gain * 0.14, panAt(t) + 0.2); }
      t += 1 / r; k++;
    }
    // the motor, its pitch following the speed
    const n = Math.round((dur + 0.08) * SR), y = new Float32Array(n);
    const lp = biquad("lp", 420, 0.8);
    let ph = 0;
    for (let i = 0; i < n; i++) {
      const tt = i / SR, u = clamp(tt / dur, 0, 1), r = r0 + (r1 - r0) * (u * u * (3 - 2 * u));
      ph += (r * 4.2) / SR;
      const e = Math.min(1, tt / 0.05) * Math.min(1, Math.max(0, (dur + 0.06 - tt) / 0.07));
      y[i] = lp((ph % 1) * 2 - 1) * e;
    }
    put("fx", t0, y, gain * 0.05 * (opts.soft ? 0.6 : 1), (tt) => panAt(tt));
  }

  // ================================================================ the motif
  const MOTIF = [64, 67, 69, 66]; // E G A ... F#: left open
  function loose(t, pan, mind, step) {
    const V = VOICE[mind] || VOICE.inkling;
    const shift = mind === "skeptic" ? -24 : mind === "tutor" ? 12 : mind === "coach" ? -12 : 0;
    const notes = step != null ? MOTIF.slice(0, 2 + step) : MOTIF;
    notes.forEach((m, i) => {
      const last = i === notes.length - 1;
      put("mus", t + i * 0.085, V.make(midi(m + shift), last ? 1.6 : 0.6), V.gain * (last ? 0.9 : 0.7), pan);
    });
  }
  /** The knot: 0 a plain knot; 1 a half close (on B); 2 the motif comes home (F# to E);
   *  3 the last knot of the film. */
  function knot(t, pan, resolve, v) {
    v = v == null ? 1 : v;
    put("fx", t, mix([sine(180, 0.09, 30, 90), 0.8], [noiseBurst(0.03, [["lp", 900, 0.7]], 3), 0.6], [noiseBurst(0.004, [["hp", 2500, 0.7]], 2), 0.4]), 0.45 * v, pan);
    if (resolve === 0) return;
    if (resolve === 1) { put("mus", t, pluck(midi(71), 1.4, 0.5, 0.997), 0.42 * v, pan); put("mus", t, pluck(midi(59), 1.6, 0.4, 0.997), 0.3 * v, pan); return; }
    // home: the F# from the motif falls to E, over E and B below
    put("mus", t - 0.07, pluck(midi(66), 0.25, 0.6, 0.994), 0.38 * v, pan);
    put("mus", t, pluck(midi(64), 2.2, 0.55, 0.998), 0.6 * v, pan);
    put("mus", t, pluck(midi(40), 2.6, 0.35, 0.998), 0.55 * v, 0);
    put("mus", t + 0.02, bowed(midi(47), midi(47), resolve === 3 ? 4.5 : 1.3, { att: 0.08, rel: resolve === 3 ? 2.4 : 0.6 }), 0.16 * v, 0);
    if (resolve === 3) { put("mus", t + 0.03, pluck(midi(52), 3.2, 0.45, 0.999), 0.4, -0.2); put("mus", t + 0.05, pluck(midi(71), 3.2, 0.5, 0.999), 0.3, 0.25); put("mus", t + 0.07, pluck(midi(78), 3.2, 0.6, 0.999), 0.18, 0.4); }
  }
  /** A mind speaks in its own instrument while its words appear: a note a word, the line
   *  rising at a question and settling at a full stop. */
  function voice(e) {
    const V = VOICE[e.mind] || VOICE.inkling;
    const words = e.text.replace(/\s+/g, " ").trim().split(" ");
    const take = Math.min(words.length, e.words || 9);
    const dur = e.t1 - e.t0, step = dur / Math.max(1, take + 1);
    let d = 2;
    const r = mulberry(e.text.length * 7 + e.mind.length);
    for (let i = 0; i < take; i++) {
      const w = words[i];
      const end = /[?]$/.test(w) ? "q" : /[.]$/.test(w) ? "s" : /[,:;]$/.test(w) ? "c" : "";
      d += r() < 0.5 ? -1 : 1;
      if (r() < 0.25) d += r() < 0.5 ? -1 : 1;
      d = clamp(d, V.span[0], V.span[1] + 2);
      if (end === "q") d = Math.min(V.span[1] + 3, d + 3);
      if (end === "s") d = 0;
      const t = e.t0 + (i + 0.5) * step + (i % 2 ? step * 0.12 : 0);
      const len = end ? 0.55 : Math.min(0.35, step * 1.6);
      let g = V.gain * (e.gain == null ? 1 : e.gain) * (0.75 + 0.25 * Math.min(1, w.length / 6));
      if (e.gainAt) for (const [tg, gv] of e.gainAt) if (t >= tg) g = V.gain * gv;
      put("mus", t, V.make(midi(deg(V.base, d)), len + 0.4), g * 0.55, e.pan);
    }
  }
  function mulberry(a) { return () => { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }

  // ================================================================ the score
  const ducks = [];
  function render(events, dur) {
    N = Math.ceil((dur + 1.5) * SR);
    for (const k in BUS) delete BUS[k];
    bus("fx"); bus("mus");
    seed = 12345;
    for (const e of events) {
      switch (e.type) {
        case "pierce": put("fx", e.t, FX.pierce(), 0.8, e.pan); break;
        case "tap": put("fx", e.t, FX.tap(), 0.17 * (e.v || 1), e.pan); break;
        case "key": put("fx", e.t, FX.key(), 0.42 * (e.v || 1), e.pan); if (e.hold) put("fx", e.t + e.hold, FX.tap(), 0.22, e.pan); break;
        case "tick": put("fx", e.t, FX.tick(), 0.3 * (e.v || 1), e.pan); break;
        case "thump": put("fx", e.t, FX.thump(), 0.32 * (e.v || 1), e.pan); break;
        case "stitch": put("fx", e.t, FX.stitch(), 0.45 * (e.v || 1), e.pan); break;
        case "tack": put("fx", e.t, FX.tack(), 0.35, e.pan); break;
        case "pop": put("fx", e.t, FX.pop(), 0.3, e.pan); break;
        case "drop": put("fx", e.t, FX.drop(), 0.42, e.pan); break;
        case "stitchrun": { const n = Math.max(3, Math.round((e.t1 - e.t0) * 16)); for (let i = 0; i < n; i++) { const u = i / (n - 1); put("fx", e.t0 + u * (e.t1 - e.t0), FX.stitch(), 0.16 * (e.v || 1) * (0.7 + 0.3 * Math.sin(i * 2.3)), e.pan0 + (e.pan1 - e.pan0) * u); } break; }
        case "needle": { for (let t = e.t0; t < e.t1; t += 0.45) { put("fx", t, FX.stitch(), 0.12 * (e.v || 1), e.pan); put("fx", t + 0.22, FX.tack(), 0.08 * (e.v || 1), e.pan); } break; }
        case "sew": machine(e.t0, e.t1, e.r0, e.r1, e.pan0, e.pan1, e.gain || 1, { triple: e.triple, soft: e.soft }); break;
        case "taut": { const d = e.t1 - e.t0; const y = bowed(e.f0, e.f1, d + 0.02, { cresc: true, att: Math.min(0.2, d * 0.5), rel: 0.015 }); put("mus", e.t0, y, 0.2 * (e.v || 1), (tt) => e.pan0 + (e.pan1 - e.pan0) * clamp(tt / d, 0, 1)); break; }
        case "knot": knot(e.t, e.pan, e.resolve, e.v); break;
        case "loose": loose(e.t, e.pan, e.mind || "inkling", e.step); break;
        case "voice": voice(e); break;
        case "chip": { const V = VOICE[e.mind] || VOICE.inkling; put("mus", e.t, V.make(midi(deg(V.base, 4)), 0.9), V.gain * 0.8, e.pan); break; }
        case "unfold": put("fx", e.t, swish(0.26, 500, 1700, 0.9), 0.1, e.pan); break;
        case "fold": put("fx", e.t, swish(0.18, 1400, 500, 0.9), 0.07, e.pan); break;
        case "whoosh": { const y = e.low ? swish(e.dur, 160, 700, 0.7) : swish(e.dur, 380, 1900, 0.8); put("fx", e.t, y, 0.16 * (e.v || 1), (tt) => e.pan0 + (e.pan1 - e.pan0) * clamp(tt / e.dur, 0, 1)); break; }
        case "lift": put("fx", e.t, e.back ? swish(0.35, 2400, 500, 0.9) : swish(0.35, 500, 2600, 0.9), 0.14, e.pan); break;
        case "marks": [0, 0.07, 0.14].forEach((d, i) => put("mus", e.t + d, pluck(midi(76 + i * 3), 0.5, 0.7, 0.995), 0.18, -0.3 + i * 0.3)); break;
        case "snip": {
          // two blades closing, then nothing at all
          put("fx", e.t, mix([noiseBurst(0.012, [["bp", 3900, 7]], 2), 1], [noiseBurst(0.006, [["hp", 6000, 0.7]], 2), 0.6]), 0.75, e.pan);
          put("fx", e.t + 0.065, mix([noiseBurst(0.014, [["bp", 6800, 8]], 2), 1], [noiseBurst(0.02, [["hp", 4500, 0.7]], 3), 0.5]), 0.85, e.pan);
          ducks.push({ t0: e.t + 0.1, t1: e.t + 0.1 + e.stop, level: 0, hard: true });
          break;
        }
        case "unsnip": put("fx", e.t, swish(0.3, 2200, 500, 0.9), 0.12, e.pan); put("mus", e.t + 0.25, pluck(midi(69), 0.8, 0.5, 0.996), 0.3, e.pan); break;
        case "duck": ducks.push({ t0: e.t0, t1: e.t1, level: e.level, hard: e.hard }); break;
        case "groove": groove(e.t0, e.t1, e.bpm); break;
        case "bass": put("mus", e.t, pluck(midi(e.note), 2.2, 0.35, 0.998), 0.5, -0.05); put("mus", e.t, sine(midi(e.note), 0.6, 4), 0.12, 0); break;
        case "pluck": put("mus", e.t, pluck(midi(e.note), 0.9, 0.6, 0.996), 0.32 * (e.v || 1), e.pan); break;
        case "swell": put("mus", e.t0, bowed(midi(e.note), midi(e.note), e.t1 - e.t0, { att: (e.t1 - e.t0) * 0.45, rel: (e.t1 - e.t0) * 0.5 }), 0.14, 0); put("mus", e.t0 + 0.05, bowed(midi(e.note + 7), midi(e.note + 7), e.t1 - e.t0 - 0.05, { att: (e.t1 - e.t0) * 0.5, rel: (e.t1 - e.t0) * 0.45 }), 0.08, 0.1); break;
        case "breath": { const y = swish(e.t1 - e.t0, 700, 900, 1.2); for (let i = 0; i < y.length; i++) y[i] *= 0.5 + 0.5 * Math.sin((2 * Math.PI * i) / (SR * 0.45)); put("fx", e.t0, y, 0.07, e.pan); break; }
        case "weave": weave(e.t0, e.t1, e.blocks); break;
      }
    }
    roomTone(dur);
    applyDucks();
  }
  /** The montage's groove: the machine running in 16ths at the tempo, accents in 3+3+2. */
  function groove(t0, t1, bpm) {
    const s16 = 60 / bpm / 4;
    let i = 0;
    for (let t = t0; t < t1 - 0.01; t += s16, i++) {
      const p = i % 8, acc = p === 0 || p === 3 || p === 6;
      put("fx", t, FX.click(), acc ? 0.2 : 0.09, acc ? -0.12 : 0.12);
      if (acc) put("fx", t, FX.thump(), p === 0 ? 0.09 : 0.05, 0);
      if (p === 2 || p === 5) put("fx", t, FX.tack(), 0.06, 0.3);
    }
    // the motor under it, thin and quiet: a hum you would hear in the room, not a bed
    const n = Math.round((t1 - t0) * SR), y = new Float32Array(n);
    const lp = biquad("lp", 380, 0.8), hp = biquad("hp", 110, 0.7);
    let ph = 0;
    for (let k = 0; k < n; k++) { ph += 26 / SR; const e = Math.min(1, k / (SR * 0.4)) * Math.min(1, (n - k) / (SR * 0.05)); y[k] = hp(lp((ph % 1) * 2 - 1)) * e; }
    put("fx", t0, y, 0.012, 0);
  }
  /** Darning the border: per block, the warp laid down in a run of plucks, then the weft woven across. */
  function weave(t0, t1, blocks) {
    const per = (t1 - t0) / blocks;
    const runs = [[64, 66, 67, 69, 71], [67, 69, 71, 73, 74], [69, 71, 73, 74, 76], [71, 74, 76, 78, 79]];
    for (let b = 0; b < blocks; b++) {
      const s = t0 + b * per, pan = -0.45 + b * 0.3;
      runs[b % 4].forEach((m, i) => put("mus", s + i * per * 0.1, pluck(midi(m), 0.8, 0.6, 0.996), 0.2, pan));
      for (let w = 0; w < 3; w++) put("fx", s + per * 0.55 + w * per * 0.13, swish(0.16, 600, 2400, 1), 0.08, pan - 0.2 + w * 0.2);
      put("fx", s + per * 0.98, FX.stitch(), 0.3, pan);
    }
  }
  /** The air of the room: cloth and a little lamp hum, very low. */
  function roomTone(dur) {
    const [L, R] = bus("fx");
    const lpL = biquad("lp", 700, 0.6), lpR = biquad("lp", 650, 0.6);
    for (let i = 0; i < Math.min(N, Math.round(dur * SR)); i++) { L[i] += lpL(rnd()) * 0.006; R[i] += lpR(rnd()) * 0.006; }
  }
  /** Held breaths, hard stops, and the hush while threads are hidden: one gain over everything. */
  function applyDucks() {
    const g = new Float32Array(N).fill(1);
    for (const d of ducks) {
      const i0 = Math.round(d.t0 * SR), i1 = Math.round(d.t1 * SR);
      const fadeIn = Math.round((d.hard ? 0.004 : 0.18) * SR), fadeOut = Math.round(0.22 * SR);
      for (let i = Math.max(0, i0); i < Math.min(N, i1 + fadeOut); i++) {
        let k;
        if (i < i0 + fadeIn) k = 1 - (1 - d.level) * ((i - i0) / fadeIn);
        else if (i < i1) k = d.level;
        else k = d.level + (1 - d.level) * ((i - i1) / fadeOut);
        g[i] = Math.min(g[i], k);
      }
    }
    for (const name of ["fx", "mus"]) { const [L, R] = BUS[name]; for (let i = 0; i < N; i++) { L[i] *= g[i]; R[i] *= g[i]; } }
  }

  // ================================================================ the room, and the files
  async function master(name, wet, irLen) {
    const [L, R] = BUS[name];
    const ctx = new OfflineAudioContext(2, N, SR);
    const src = ctx.createBufferSource();
    const b = ctx.createBuffer(2, N, SR);
    b.copyToChannel(L, 0); b.copyToChannel(R, 1);
    src.buffer = b;
    // a small room for the foley, a warmer one for the music: decaying noise, a little dark
    const ir = ctx.createBuffer(2, Math.round(irLen * SR), SR);
    for (let c = 0; c < 2; c++) {
      const d = ir.getChannelData(c);
      let lp = 0;
      for (let i = 0; i < d.length; i++) { lp += (rnd() - lp) * 0.35; d[i] = lp * Math.pow(1 - i / d.length, 3.2); }
    }
    const conv = ctx.createConvolver(); conv.normalize = true; conv.buffer = ir;
    const wetG = ctx.createGain(); wetG.gain.value = wet;
    const dry = ctx.createGain(); dry.gain.value = 1;
    src.connect(dry); src.connect(conv); conv.connect(wetG);
    dry.connect(ctx.destination); wetG.connect(ctx.destination);
    src.start(0);
    return ctx.startRendering();
  }
  function wav(abuf, gain) {
    const ch = 2, n = abuf.length, bytes = 44 + n * ch * 2, buf = new ArrayBuffer(bytes), v = new DataView(buf);
    const W = (o, s) => { for (let i = 0; i < s.length; i++) v.setUint8(o + i, s.charCodeAt(i)); };
    W(0, "RIFF"); v.setUint32(4, bytes - 8, true); W(8, "WAVE"); W(12, "fmt "); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, ch, true);
    v.setUint32(24, SR, true); v.setUint32(28, SR * ch * 2, true); v.setUint16(32, ch * 2, true); v.setUint16(34, 16, true); W(36, "data"); v.setUint32(40, n * ch * 2, true);
    const L = abuf.getChannelData(0), R = abuf.getChannelData(1);
    let o = 44;
    for (let i = 0; i < n; i++) { v.setInt16(o, clamp(Math.round(L[i] * gain * 32767), -32768, 32767), true); v.setInt16(o + 2, clamp(Math.round(R[i] * gain * 32767), -32768, 32767), true); o += 4; }
    let s = "";
    const u8 = new Uint8Array(buf);
    for (let i = 0; i < u8.length; i += 0x8000) s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    return btoa(s);
  }
  const peak = (ab) => { let p = 0; for (let c = 0; c < 2; c++) { const d = ab.getChannelData(c); for (let i = 0; i < d.length; i++) { const a = Math.abs(d[i]); if (a > p) p = a; } } return p; };
  window.SCORE = {
    async render(events) {
      ducks.length = 0;
      const t0 = performance.now();
      render(events, window.FILM.DUR);
      out.music = await master("mus", 0.24, 2.2);
      out.fx = await master("fx", 0.07, 0.5);
      // one gain for both stems, so the balance set here survives into mix.sh
      const p = Math.max(peak(out.music), peak(out.fx));
      out.gain = 0.95 / p;
      return { seconds: out.music.length / SR, events: events.length, peak: p, gain: out.gain, ms: Math.round(performance.now() - t0) };
    },
    wav(stem) { out[stem + "64"] = wav(out[stem], out.gain); return out[stem + "64"].length; },
    chunk(stem, i, size) { return out[stem + "64"].slice(i * size, (i + 1) * size); },
  };
})();

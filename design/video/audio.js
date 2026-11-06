/* The walkthrough's score: one quiet piece under the whole film.
 *
 * It is sewn, not played at the picture. A felt piano carries one figure, the running
 * stitch: four notes that go over and under a middle note, the way a needle goes over and
 * under the cloth, held and lifted in the stitch's own proportion (stitch.rs lays 6.6 px of
 * thread for every 3.7 px under the cloth), each a little uneven, as a hand-sewn row is.
 * Every phrase ends on its middle note: a knot. Under it, a bowed pad and a soft bass hold
 * one chord at a time, in F; the harmony moves where the chapters and their moments begin,
 * and turns to D flat when the desk turns to Lamplight. When a thread is tied off on screen,
 * a phrase lands on its knot right there, with a low note under it: that, and the last
 * knot becoming the dot of the i, are the only moments the music marks. When the threads
 * are hidden, the melody rests.
 *
 * Synthesised sample by sample from FILM (its plan and its few cues, film.js), so it
 * follows the cut when moments are added. SCORE.render() makes two stems: "music", and
 * "fx" (only the breath of thread drawn tight at each tie-off); mix.sh levels them.
 * Everything starts and stops on a smooth envelope: no clicks, no hard edges.
 */
(function () {
  "use strict";
  const SR = 48000;
  const out = {};
  let N = 0;
  const BUS = {};
  const midi = (m) => 440 * Math.pow(2, (m - 69) / 12);
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  // ---- a repeatable noise and jitter
  let seed = 7;
  const rnd = () => { seed ^= seed << 13; seed >>>= 0; seed ^= seed >>> 17; seed ^= seed << 5; seed >>>= 0; return (seed / 4294967296) * 2 - 1; };
  function mulberry(a) { return () => { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
  /** A raised-cosine ramp 0..1 over k in 0..1: no corner at either end. */
  const ramp = (k) => (k <= 0 ? 0 : k >= 1 ? 1 : 0.5 - 0.5 * Math.cos(Math.PI * k));

  function bus(name) { if (!BUS[name]) BUS[name] = [new Float32Array(N), new Float32Array(N)]; return BUS[name]; }
  /** Adds a mono signal at time t, panned (equal power, -1..1). */
  function put(name, t, mono, gain, pan) {
    const [L, R] = bus(name);
    const i0 = Math.round(t * SR), a = ((clamp(pan || 0, -1, 1) + 1) * Math.PI) / 4, gl = Math.cos(a) * gain, gr = Math.sin(a) * gain;
    for (let i = 0; i < mono.length; i++) {
      const j = i0 + i;
      if (j < 0) continue;
      if (j >= N) break;
      L[j] += mono[i] * gl; R[j] += mono[i] * gr;
    }
  }

  // ================================================================ instruments
  /** A felt piano note: a few partials, slightly stretched, two strings a breath apart, a
   *  soft hammer (an 18 ms raised-cosine attack and little above the fifth partial), a decay
   *  that is slower for low notes, and a damper that lifts the sound away over `rel` after
   *  `hold` seconds. Ends at exactly zero. */
  function felt(m, vel, hold, rel) {
    const f = midi(m), B = 0.00022;
    const ring = Math.min(hold + rel, 9);
    const n = Math.round((ring + 0.05) * SR), y = new Float32Array(n);
    const t60 = clamp(7.5 - (m - 48) * 0.09, 2.6, 8); // seconds to fall 60 dB, for the fundamental
    const bright = 0.35 + 0.5 * vel;
    const att = Math.round(0.018 * SR), relN = Math.round(rel * SR), holdN = Math.round(hold * SR);
    const parts = [];
    for (let p = 1; p <= 8; p++) {
      const fp = f * p * Math.sqrt(1 + B * p * p);
      if (fp > 5200) break;
      const amp = Math.pow(p, -1.3) * Math.exp(-(p - 1) * (0.45 - 0.35 * bright)) * (p === 2 ? 1.2 : 1);
      const decay = (6.91 / t60) * (1 + 0.55 * (p - 1));
      parts.push({ w1: (2 * Math.PI * fp * (1 + 0.00035)) / SR, w2: (2 * Math.PI * fp * (1 - 0.00035)) / SR, amp, decay });
    }
    for (const pt of parts) {
      let ph1 = rnd() * Math.PI, ph2 = rnd() * Math.PI;
      for (let i = 0; i < n; i++) {
        const tt = i / SR;
        y[i] += pt.amp * Math.exp(-pt.decay * tt) * (Math.sin(ph1) + 0.8 * Math.sin(ph2));
        ph1 += pt.w1; ph2 += pt.w2;
      }
    }
    for (let i = 0; i < n; i++) {
      let e = i < att ? ramp(i / att) : 1;
      if (i >= holdN) e *= 1 - ramp((i - holdN) / Math.max(1, relN));
      y[i] *= e * vel * 0.5;
    }
    return y;
  }
  /** One voice of the pad: a bowed, breathy tone, three partials and a slow vibrato, that
   *  swells in over `att` and away over `rel`. */
  function bowed(m, dur, att, rel) {
    const f = midi(m), n = Math.round((dur + rel) * SR), y = new Float32Array(n);
    const attN = Math.round(att * SR), endN = Math.round(dur * SR), relN = Math.round(rel * SR);
    let ph = rnd() * Math.PI, lp = 0;
    const vib = 4.6 + rnd() * 0.6, vdepth = 0.0011;
    for (let i = 0; i < n; i++) {
      const tt = i / SR;
      ph += (2 * Math.PI * f * (1 + vdepth * Math.sin(2 * Math.PI * vib * tt + 1.3) * ramp(tt / 1.5))) / SR;
      const s = Math.sin(ph) + 0.28 * Math.sin(2 * ph + 0.4) + 0.1 * Math.sin(3 * ph + 1.1);
      lp += (s - lp) * 0.35; // a little darker
      let e = ramp(i / Math.max(1, attN));
      if (i >= endN) e *= 1 - ramp((i - endN) / Math.max(1, relN));
      y[i] = lp * e;
    }
    return y;
  }
  /** The bass: a soft sine with a little second partial, in and out on ramps. */
  function bass(m, dur, rel) {
    const f = midi(m), n = Math.round((dur + rel) * SR), y = new Float32Array(n);
    const attN = Math.round(0.35 * SR), endN = Math.round(dur * SR), relN = Math.round(rel * SR);
    let ph = 0;
    for (let i = 0; i < n; i++) {
      ph += (2 * Math.PI * f) / SR;
      let e = ramp(i / attN) * Math.exp(-(i / SR) * 0.12);
      if (i >= endN) e *= 1 - ramp((i - endN) / relN);
      y[i] = (Math.sin(ph) + 0.16 * Math.sin(2 * ph)) * e;
    }
    return y;
  }
  /** Thread drawn tight: a breath of low, band-limited noise that swells into the knot and
   *  is gone, on raised-cosine edges (the only sound in the film that is not a note). */
  function breath(dur) {
    const n = Math.round(dur * SR), y = new Float32Array(n);
    let a = 0, b = 0, c = 0;
    for (let i = 0; i < n; i++) {
      const k = i / n;
      // two one-pole low-passes and a one-pole high-pass: a soft band around 300-700 Hz
      a += (rnd() - a) * 0.09; b += (a - b) * 0.09; c += (b - c) * 0.012;
      const env = Math.pow(Math.sin(Math.PI * Math.pow(k, 0.75)), 2);
      y[i] = (b - c) * env * 6;
    }
    return y;
  }

  // ================================================================ harmony
  // Chords in F: a bass note and a pad voicing (MIDI), and the tones the melody may use.
  const CHORD = {
    Fmaj9: { bass: 41, pad: [57, 60, 64, 67], tones: [65, 69, 72, 76, 67, 74] },
    "F/A": { bass: 45, pad: [53, 60, 65, 69], tones: [65, 69, 72, 77] },
    Fadd9: { bass: 41, pad: [53, 60, 67, 69], tones: [65, 67, 69, 72] },
    Am7: { bass: 45, pad: [55, 60, 64, 67], tones: [64, 67, 69, 72, 76] },
    Bbmaj7: { bass: 46, pad: [57, 62, 65, 69], tones: [65, 69, 70, 74, 77] },
    Dm9: { bass: 38, pad: [53, 57, 60, 64], tones: [62, 64, 65, 69, 72] },
    Gm9: { bass: 43, pad: [53, 58, 62, 69], tones: [62, 65, 67, 69, 70, 74] },
    C9sus: { bass: 36, pad: [53, 58, 62, 67], tones: [65, 67, 70, 72, 74] },
    Dbmaj7: { bass: 37, pad: [53, 56, 60, 65], tones: [65, 68, 72, 73, 77] },
  };
  /** Each chapter's progression, spread over its moments (film.js PLAN): a chord a moment,
   *  or a few moments, so new moments take the next chord along. */
  const PROG = {
    0: ["Fmaj9"],
    1: ["Am7", "Bbmaj7"],
    2: ["Dm9", "Bbmaj7", "Gm9", "C9sus", "Fmaj9"],
    3: ["Dm9", "Bbmaj7", "Gm9", "Am7", "Bbmaj7", "Gm9", "C9sus"],
    4: ["F/A", "Bbmaj7", "Bbmaj7", "Dm9", "C9sus", "Am7", "Bbmaj7", "Gm9", "C9sus"],
    5: ["Bbmaj7", "Dm9"],
    6: ["Bbmaj7", "Dbmaj7", "Bbmaj7", "C9sus"],
    7: ["Fmaj9", "Dm9", "Bbmaj7", "Gm9", "C9sus"],
    8: ["C9sus", "Fadd9"],
  };

  /** The score's plan from the film: chord changes, phrases and the soft moments. */
  function plan(F) {
    const changes = []; // [time, chord]
    for (const ch of F.PLAN) {
      const t0 = F.CHS[ch.n], ids = ch.moments.map((m) => F.MOMENT[`${ch.n}.${m.id}`]);
      const prog = ch.chords || PROG[ch.n] || ["Fmaj9"];
      if (ch.moments.length <= 2 || ch.n === 6 || ch.n === 7) {
        // few moments, or a montage of beats: spread the chords evenly, on the nearest beat
        const t1 = ids[ids.length - 1].t1;
        prog.forEach((c, i) => {
          let t = t0 + ((t1 - t0) * i) / prog.length;
          if (i) { const near = ids.map((m) => m.t0).reduce((a, b) => (Math.abs(b - t) < Math.abs(a - t) ? b : a), t); if (Math.abs(near - t) < 1.6) t = near; }
          changes.push([t, c]);
        });
      } else {
        // a chord a moment, as far as the progression goes; the last chord holds
        ids.forEach((m, i) => { if (i < prog.length) changes.push([m.t0, prog[i]]); });
      }
    }
    // cadences: each tie-off resolves to F on its knot; Lamplight turns the harmony to D flat.
    // These are pinned: a change that falls too near one gives way to it.
    const ties = F.events.filter((e) => e.type === "tie").map((e) => e.t);
    const pins = ties.map((t) => [t, "Fmaj9", true]);
    if (F.M6 && F.M6.dark) pins.push([F.M6.dark + 0.85, "Dbmaj7", true]);
    const settle = F.events.find((e) => e.type === "settle");
    if (settle) pins.push([settle.t, "Fadd9", true]);
    // the chord before each tie-off is a suspended dominant, so the knot lands as a cadence
    for (const t of ties) pins.push([t - 3.0, "C9sus", false]);
    const all = changes.map(([t, c]) => [t, c, false]).concat(pins).sort((a, b) => a[0] - b[0] || (a[2] ? 1 : -1));
    const clean = [];
    for (const c of all) {
      const last = clean[clean.length - 1];
      if (last && c[0] - last[0] < 1.5) {
        if (last[2] && !c[2]) continue; // a pinned change holds its place
        if (last[1] === c[1]) { if (c[2]) last[2] = true; continue; } // the same chord: keep the earlier
        clean.pop();
      }
      const prev = clean[clean.length - 1];
      if (prev && prev[1] === c[1]) { if (c[2]) prev[2] = true; continue; }
      clean.push(c);
    }
    return { changes: clean.map(([t, c]) => [t, c]), ties, settle: settle && settle.t, hush: F.events.filter((e) => e.type === "hush") };
  }

  // ================================================================ the running stitch
  /** One phrase: four stitches over and under a middle note, ending on it (the knot).
   *  `end` puts the knot on that time; otherwise the phrase starts at `start`. */
  function phrase(chord, at, opts, r) {
    const C = CHORD[chord];
    const tones = C.tones.slice().sort((a, b) => a - b);
    // the middle: the chord tone nearest A4, a little higher as the film goes on
    const mid = tones.reduce((a, b) => (Math.abs(b - (69 + (opts.lift || 0))) < Math.abs(a - (69 + (opts.lift || 0))) ? b : a));
    const above = tones.filter((x) => x > mid), below = tones.filter((x) => x < mid);
    const over = above[0] ?? mid + 3, over2 = above[Math.min(1, above.length - 1)] ?? over, under = below[below.length - 1] ?? mid - 4;
    const notes = opts.short ? [over, mid] : r() < 0.5 ? [over, under, over2, mid] : [under, over, under, mid];
    // the stitch's proportion: on for 0.64 of a period, under the cloth for 0.36
    const period = opts.period || 1.55;
    const lens = notes.map(() => period * (0.9 + 0.2 * r()));
    const total = lens.slice(0, -1).reduce((a, b) => a + b, 0);
    let t = opts.end != null ? opts.end - total : at;
    return notes.map((m, i) => {
      const last = i === notes.length - 1;
      const n = { t, m, vel: (last ? 0.34 : 0.4 + 0.06 * r()) * (opts.vel || 1), hold: last ? 2.2 : lens[i] * 0.64, rel: last ? 1.6 : 0.42 };
      t += lens[i];
      return n;
    });
  }

  // ================================================================ the score
  function render(F) {
    const DUR = F.DUR;
    N = Math.ceil((DUR + 2) * SR);
    for (const k in BUS) delete BUS[k];
    bus("music"); bus("fx");
    seed = 7;
    const P = plan(F);
    const r = mulberry(1109);
    const chordAt = (t) => { let c = P.changes[0][1]; for (const [tc, nm] of P.changes) if (tc <= t + 1e-6) c = nm; return c; };
    const hushed = (t) => P.hush.some((h) => t >= h.t0 - 0.4 && t < h.t1);
    // ---- the pad and the bass: each chord swells in as the last one swells out
    P.changes.forEach(([t, nm], i) => {
      const C = CHORD[nm], next = P.changes[i + 1] ? P.changes[i + 1][0] : DUR;
      const dur = Math.max(0.5, next - t), rel = Math.min(2.8, 1.2 + dur * 0.2);
      const first = i === 0;
      C.pad.forEach((m, k) => {
        const g = (0.03 - k * 0.003) * (first ? 0.9 : 1);
        put("music", t - (first ? 0 : 0.9), bowed(m, dur + (first ? 0 : 0.9), first ? 4.5 : 2.2, rel), g, -0.4 + (0.8 * k) / Math.max(1, C.pad.length - 1));
      });
      put("music", t, bass(C.bass, dur, Math.min(1.8, rel)), 0.05, -0.05);
    });
    // ---- the melody: a phrase after each change, and one landing on each knot
    const notes = [];
    const ends = P.ties.slice();
    if (P.settle) ends.push(P.settle);
    // the cold open: the title is sewn and tied; the first phrase is that stitch
    const tie0 = F.T.openTie;
    notes.push(...phrase("Fmaj9", 0, { end: tie0, period: 1.2 }, r));
    // a phrase after a change of chord, with room around it: at least three seconds of rest
    // after the last phrase, and none where a knot's own phrase is coming
    let lastEnd = tie0;
    P.changes.forEach(([t, nm], i) => {
      const next = P.changes[i + 1] ? P.changes[i + 1][0] : F.DUR;
      const start = Math.max(t + 1.1 + 0.4 * r(), lastEnd + 3.0 + 1.5 * r());
      if (start > next - 2.5) return;
      if (ends.some((e) => e > start - 2 && e < start + 9)) return;
      if (hushed(start) || hushed(start + 5)) return;
      const lift = F.CHS && t > F.CHS[3] && t < F.CHS[6] ? 2 : 0;
      const ph = phrase(nm, start, { lift, period: 1.55 + 0.2 * r(), short: r() < 0.3 }, r);
      notes.push(...ph);
      lastEnd = ph[ph.length - 1].t + 1.5;
    });
    // a phrase that ties off exactly on each knot, in the chord that resolves there
    for (const e of ends) {
      const chord = chordAt(e - 0.5);
      const ph = phrase(chord === "C9sus" ? "C9sus" : chord, 0, { end: e, period: 1.25 }, r);
      // the knot itself is the F
      ph[ph.length - 1].m = e === P.settle ? 65 : 69;
      notes.push(...ph);
    }
    // the wordmark is sewn on: two last notes, rising, and the long F under them
    if (P.settle) {
      notes.push({ t: P.settle + 1.6, m: 72, vel: 0.3, hold: 1.4, rel: 1.4 }, { t: P.settle + 2.45, m: 77, vel: 0.27, hold: 3.2, rel: 2.4 });
    }
    // keep phrases apart, out of the hush, and inside the film
    notes.sort((a, b) => a.t - b.t);
    const kept = [];
    for (const n of notes) {
      if (n.t < 0.3 || n.t > F.DUR - 2.5 || hushed(n.t)) continue;
      if (kept.length && n.t - kept[kept.length - 1].t < 0.5) continue;
      kept.push(n);
    }
    for (const n of kept) put("music", n.t, felt(n.m, n.vel, n.hold, n.rel), 0.55, clamp((n.m - 69) / 14, -0.35, 0.35));
    // ---- the soft moments: a low note under each knot, and the breath of the thread
    for (const t of ends) {
      const root = t === P.settle ? 41 : 53;
      put("music", t, felt(root, 0.3, 3.0, 2.2), 0.6, 0);
      put("fx", t - 0.42, breath(0.5), 0.05, 0.1);
    }
    out.plan = { changes: P.changes.length, notes: kept.length, ties: ends.length };
    // ---- the end: everything fades over the last seconds
    const fade0 = Math.round((F.DUR - 3.2) * SR), fade1 = Math.round((F.DUR - 0.2) * SR);
    for (const name of ["music", "fx"]) {
      const [L, R] = BUS[name];
      for (let i = fade0; i < N; i++) { const g = i >= fade1 ? 0 : 1 - ramp((i - fade0) / (fade1 - fade0)); L[i] *= g; R[i] *= g; }
      // and nothing at the very start but a ramp from silence
      const a = Math.round(0.25 * SR);
      for (let i = 0; i < a; i++) { const g = ramp(i / a); L[i] *= g; R[i] *= g; }
    }
  }

  // ================================================================ the room, and the files
  /** A warm room: a convolution with decaying, darkened, decorrelated noise. */
  async function room(name, wet, len) {
    const [L, R] = BUS[name];
    const ctx = new OfflineAudioContext(2, N, SR);
    const src = ctx.createBufferSource();
    const b = ctx.createBuffer(2, N, SR);
    b.copyToChannel(L, 0); b.copyToChannel(R, 1);
    src.buffer = b;
    const ir = ctx.createBuffer(2, Math.round(len * SR), SR);
    for (let c = 0; c < 2; c++) {
      const d = ir.getChannelData(c);
      let lp = 0, lp2 = 0;
      for (let i = 0; i < d.length; i++) {
        lp += (rnd() - lp) * 0.22; lp2 += (lp - lp2) * 0.5;
        const k = i / d.length;
        d[i] = lp2 * Math.pow(1 - k, 2.4) * ramp(i / (0.006 * SR)) * (1 - 0.6 * Math.min(1, k * 3)) ;
      }
    }
    const conv = ctx.createConvolver(); conv.normalize = true; conv.buffer = ir;
    const pre = ctx.createDelay(0.1); pre.delayTime.value = 0.022;
    const wetG = ctx.createGain(); wetG.gain.value = wet;
    const dry = ctx.createGain(); dry.gain.value = 1;
    const tone = ctx.createBiquadFilter(); tone.type = "lowpass"; tone.frequency.value = 5200; tone.Q.value = 0.5;
    const hp = ctx.createBiquadFilter(); hp.type = "highpass"; hp.frequency.value = 28; hp.Q.value = 0.5;
    src.connect(dry); src.connect(pre); pre.connect(conv); conv.connect(wetG);
    dry.connect(tone); wetG.connect(tone); tone.connect(hp); hp.connect(ctx.destination);
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
    // a whisper of triangular dither, so the quiet tails never step
    for (let i = 0; i < n; i++) {
      const d1 = (Math.random() - Math.random()) / 32768, d2 = (Math.random() - Math.random()) / 32768;
      v.setInt16(o, clamp(Math.round((L[i] * gain + d1) * 32767), -32768, 32767), true); v.setInt16(o + 2, clamp(Math.round((R[i] * gain + d2) * 32767), -32768, 32767), true); o += 4;
    }
    let s = "";
    const u8 = new Uint8Array(buf);
    for (let i = 0; i < u8.length; i += 0x8000) s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    return btoa(s);
  }
  const peak = (ab) => { let p = 0; for (let c = 0; c < 2; c++) { const d = ab.getChannelData(c); for (let i = 0; i < d.length; i++) { const a = Math.abs(d[i]); if (a > p) p = a; } } return p; };
  window.SCORE = {
    async render() {
      const t0 = performance.now();
      render(window.FILM);
      out.music = await room("music", 0.3, 3.4);
      out.fx = await room("fx", 0.2, 1.2);
      // one gain for both stems, so the balance set here survives into mix.sh
      const p = Math.max(peak(out.music), peak(out.fx));
      out.gain = 0.5 / p;
      return { seconds: out.music.length / SR, ...out.plan, peak: p, gain: out.gain, ms: Math.round(performance.now() - t0) };
    },
    wav(stem) { out[stem + "64"] = wav(out[stem], out.gain); return out[stem + "64"].length; },
    chunk(stem, i, size) { return out[stem + "64"].slice(i * size, (i + 1) * size); },
  };
})();

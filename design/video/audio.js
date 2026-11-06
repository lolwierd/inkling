/* Inkling film: the score and the foley, synthesised (no samples), on the film's clock.
 *
 * Score: 80 bpm in D major, 3 s a bar. The two knots that tie a thread off land on
 * downbeats (bar 15 at 45 s, bar 20 at 60 s) and resolve home to D. Instruments are made
 * here: a felt piano (additive partials), a plucked thread (Karplus-Strong), a soft pad,
 * a round bass, a small bell for knots, and a wooden "flute" for the third mind.
 * Foley: keys, clicks, stitches, a thread drawn through cloth, a knot snugging tight,
 * paper, a lamp's pull chain. Its events come from the film itself (FILM.events).
 *
 * SCORE.render() makes two stems, music and fx; SCORE.wav(stem) returns one as base64 WAV.
 * The mix (ducking, loudness) is done by mix.sh with ffmpeg.
 */
(function () {
  "use strict";
  const SR = 48000, DUR = 98;
  const N = SR * DUR;
  const hz = (m) => 440 * Math.pow(2, (m - 69) / 12);
  const PC = { C: 0, "C#": 1, D: 2, "D#": 3, E: 4, F: 5, "F#": 6, G: 7, "G#": 8, A: 9, "A#": 10, B: 11 };
  const m = (s) => { const x = s.match(/^([A-G]#?)(-?\d)$/); return 12 * (+x[2] + 1) + PC[x[1]]; };
  function rng(seed) { let s = (seed * 2654435761) >>> 0 || 1; return () => { s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; return (s % 100000) / 100000; }; }
  const R = rng(7);
  const bar = (b, beat = 0) => b * 3 + beat * 0.75;

  // ---------------------------------------------------------------- buses
  const bus = () => ({ L: new Float32Array(N), R: new Float32Array(N) });
  const B = { piano: bus(), pluck: bus(), pad: bus(), bass: bus(), bell: bus(), flute: bus(), perc: bus(), fx: bus() };
  function mixIn(b, buf, t, gain, pan) {
    const i0 = Math.round(t * SR);
    const gl = gain * Math.cos(((pan + 1) * Math.PI) / 4), gr = gain * Math.sin(((pan + 1) * Math.PI) / 4);
    for (let i = 0; i < buf.length; i++) {
      const j = i0 + i;
      if (j < 0) continue;
      if (j >= N) break;
      b.L[j] += buf[i] * gl; b.R[j] += buf[i] * gr;
    }
  }

  // ---------------------------------------------------------------- instruments
  const cache = new Map();
  const memo = (k, f) => { if (!cache.has(k)) cache.set(k, f()); return cache.get(k); };
  /** Felt piano: a few partials, the upper ones dying first; a soft hammer; felt keeps it dark. */
  function piano(midi, dur) {
    return memo(`p${midi}:${dur}`, () => {
      const f = hz(midi), len = Math.floor(SR * (dur + 1.6)), b = new Float32Array(len);
      const ring = 1.1 * Math.pow(261 / f, 0.45) + 0.5;
      const parts = [[1, 1, 1], [2, 0.36, 1.9], [3, 0.12, 2.8], [4, 0.05, 3.9], [5.01, 0.02, 5]];
      for (const [k, a, dk] of parts) {
        if (f * k > 9000) continue;
        const w = (2 * Math.PI * f * k) / SR;
        for (let i = 0; i < len; i++) b[i] += a * Math.sin(w * i + k) * Math.exp(-(i / SR) * dk / ring);
      }
      let lp = 0;
      const r = rng(midi);
      for (let i = 0; i < len; i++) {
        const t = i / SR;
        const att = Math.min(1, t / 0.006), rel = t < dur ? 1 : Math.exp(-(t - dur) / 0.35);
        const ham = t < 0.02 ? (r() * 2 - 1) * 0.25 * (1 - t / 0.02) : 0;
        lp += 0.5 * (b[i] * att * rel + ham - lp); // felt: a gentle low-pass
        b[i] = lp * 0.55;
      }
      return b;
    });
  }
  /** A plucked thread (Karplus-Strong): a burst of noise circling a delay line, an averaging
   *  filter taking the brightness out a little each time round, and an all-pass for tuning. */
  function pluck(midi, dur, bright) {
    return memo(`k${midi}:${dur}:${bright}`, () => {
      const P = SR / hz(midi), total = P - 0.5;
      const n0 = Math.floor(total - 0.1), d = total - n0, C = (1 - d) / (1 + d);
      const len = Math.floor(SR * (dur + 1.2)), out = new Float32Array(len), line = new Float32Array(n0), r = rng(midi * 3 + 1);
      let lp = 0;
      for (let i = 0; i < n0; i++) { lp += (bright || 0.5) * ((r() * 2 - 1) - lp); line[i] = lp; }
      // no DC in the burst: the loop keeps whatever offset it starts with, as a thump
      const mean = line.reduce((a, b) => a + b, 0) / n0;
      for (let i = 0; i < n0; i++) line[i] -= mean;
      const damp = 0.9965 - Math.max(0, (midi - 72) * 0.0007);
      let idx = 0, prev = 0, apx = 0, apy = 0, dcx = 0, dcy = 0;
      for (let i = 0; i < len; i++) {
        const y = line[idx];
        const avg = damp * 0.5 * (y + prev);
        prev = y;
        const ap = C * avg + apx - C * apy;
        apx = avg; apy = ap;
        line[idx] = ap;
        idx = idx + 1 === n0 ? 0 : idx + 1;
        const t = i / SR, rel = t < dur ? 1 : Math.exp(-(t - dur) / 0.2);
        // and a gentle DC blocker on the way out
        dcy = y - dcx + 0.995 * dcy; dcx = y;
        out[i] = dcy * rel * Math.min(1, t / 0.0015) * 0.9;
      }
      return out;
    });
  }
  /** A small bell for knots: inharmonic partials, bright and short. */
  function bell(midi, dur) {
    return memo(`b${midi}:${dur}`, () => {
      const f = hz(midi), len = Math.floor(SR * (dur + 2)), b = new Float32Array(len);
      for (const [k, a, d] of [[1, 1, 1.4], [2.76, 0.32, 3.2], [5.4, 0.12, 6], [8.93, 0.05, 9]]) {
        const w = (2 * Math.PI * f * k) / SR;
        for (let i = 0; i < len; i++) b[i] += a * Math.sin(w * i) * Math.exp(-(i / SR) * d) * Math.min(1, i / (SR * 0.002));
      }
      for (let i = 0; i < len; i++) b[i] *= 0.4;
      return b;
    });
  }
  /** Pad: soft sines a few cents apart, slow in and slow out. */
  function pad(midi, dur) {
    return memo(`d${midi}:${dur}`, () => {
      const f = hz(midi), len = Math.floor(SR * (dur + 2.2)), b = new Float32Array(len);
      const att = 1.1, rel = 1.8;
      for (const [k, a, cents] of [[1, 1, -5], [1, 1, 6], [2, 0.22, 3], [3, 0.06, -4]]) {
        const w = (2 * Math.PI * f * k * Math.pow(2, cents / 1200)) / SR;
        for (let i = 0; i < len; i++) b[i] += a * Math.sin(w * i + k * 1.7);
      }
      for (let i = 0; i < len; i++) {
        const t = i / SR;
        const e = t < att ? Math.sin((Math.PI / 2) * (t / att)) ** 2 : t < dur ? 1 : Math.exp(-(t - dur) / (rel / 3));
        b[i] *= e * 0.16 * (1 + 0.06 * Math.sin(2 * Math.PI * 0.21 * t));
      }
      return b;
    });
  }
  /** Bass: a round sine with a touch of second harmonic, plucked. */
  function bass(midi, dur) {
    return memo(`a${midi}:${dur}`, () => {
      const f = hz(midi), len = Math.floor(SR * (dur + 0.8)), b = new Float32Array(len);
      for (let i = 0; i < len; i++) {
        const t = i / SR, e = Math.sin(Math.PI / 2 * Math.min(1, t / 0.03)) * (t < dur ? Math.exp(-t * 0.9) : Math.exp(-dur * 0.9) * Math.exp(-(t - dur) / 0.15));
        b[i] = (Math.sin(2 * Math.PI * f * t) + 0.25 * Math.sin(4 * Math.PI * f * t)) * e * 0.5;
      }
      return b;
    });
  }
  /** A wooden flute for the third mind: a breathy sine with a slow vibrato. */
  function flute(midi, dur) {
    return memo(`f${midi}:${dur}`, () => {
      const f = hz(midi), len = Math.floor(SR * (dur + 0.6)), b = new Float32Array(len), r = rng(midi + 99);
      let ph = 0, lp = 0;
      for (let i = 0; i < len; i++) {
        const t = i / SR, vib = 1 + 0.004 * Math.sin(2 * Math.PI * 5.1 * t) * Math.min(1, t / 0.5);
        ph += (2 * Math.PI * f * vib) / SR;
        const e = Math.min(1, t / 0.09) * (t < dur ? 1 : Math.exp(-(t - dur) / 0.12));
        lp += 0.08 * ((r() * 2 - 1) - lp);
        b[i] = (Math.sin(ph) + 0.18 * Math.sin(2 * ph) + lp * 0.06) * e * 0.3;
      }
      return b;
    });
  }

  // ---------------------------------------------------------------- the score
  const CH = {
    Dadd9: { pad: ["D3", "A3", "E4", "F#4"], bass: "D2", arp: ["D4", "E4", "F#4", "A4"] },
    D: { pad: ["D3", "A3", "D4", "F#4"], bass: "D2", arp: ["A4", "D5", "F#5", "A5"] },
    Bm7: { pad: ["F#3", "A3", "B3", "D4"], bass: "B1", arp: ["F#4", "B4", "D5", "F#5"] },
    Gmaj7: { pad: ["G3", "B3", "D4", "F#4"], bass: "G1", arp: ["G4", "B4", "D5", "F#5"] },
    A7sus4: { pad: ["G3", "A3", "D4", "E4"], bass: "A1", arp: ["E4", "A4", "D5", "E5"] },
    A: { pad: ["E3", "A3", "C#4", "E4"], bass: "A1", arp: ["E4", "A4", "C#5", "E5"] },
    Asus4: { pad: ["E3", "A3", "D4", "E4"], bass: "A1", arp: ["E4", "A4", "D5", "E5"] },
    Em7: { pad: ["E3", "G3", "B3", "D4"], bass: "E2", arp: ["E4", "G4", "B4", "D5"] },
    "F#m7": { pad: ["E3", "F#3", "A3", "C#4"], bass: "F#1", arp: ["F#4", "A4", "C#5", "E5"] },
    "D/F#": { pad: ["F#3", "A3", "D4", "E4"], bass: "F#1", arp: ["F#4", "A4", "D5", "E5"] },
    Gmaj9: { pad: ["F#3", "A3", "B3", "D4"], bass: "G1", arp: ["B4", "D5", "F#5", "A5"] },
    Bm9: { pad: ["F#3", "A3", "C#4", "D4"], bass: "B1", arp: ["F#4", "A4", "C#5", "D5"] },
  };
  // one chord per bar (bar = 3 s); the arc follows the film
  const PROG = ["Dadd9", "Dadd9", "Gmaj7", // cold open, the wordmark, into the window
    "D", "Bm7", "Gmaj7", "A7sus4", // you write
    "Em7", // a pause, and a question
    "Gmaj7", "F#m7", "Em7", "Asus4", // the sketch
    "Bm7", "Gmaj7", "A7sus4", // fix the words... (A7 at the end of bar 14)
    "D", "Bm7", "Gmaj7", "Em7", "A7sus4", // ...tied off on the downbeat of bar 15; the second thread
    "Dadd9", "Gmaj7", "F#m7", "Bm7", // tied off on bar 20; the room
    "Gmaj7", "D/F#", "Em7", "A", "Bm7", // the brief, the chat, a mind by name
    "Gmaj9", // hide threads
    "Bm9", // lamplight
    "Gmaj7", "Dadd9", "Dadd9"]; // the icon, the name, the end
  const notes = [];
  const add = (inst, t, note, dur, vel, pan) => notes.push({ inst, t, midi: typeof note === "number" ? note : m(note), dur, vel, pan: pan == null ? 0 : pan });
  const hum = (t, amt = 0.008) => t + (R() - 0.5) * 2 * amt;

  PROG.forEach((name, b) => {
    const c = CH[name], t0 = bar(b);
    const quiet = b === 29 ? 0.55 : 1;
    // the pad is under everything, a little louder at the ends
    c.pad.forEach((p, i) => add("pad", t0, p, 3.05, (b < 3 || b >= 30 ? 1.1 : 0.85) * quiet, (i - 1.5) * 0.35));
    // bass from the writing on, out for hide threads
    if (b >= 3 && b !== 29) {
      add("bass", t0, c.bass, b >= 24 && b <= 28 ? 1.4 : 2.6, 0.9);
      if (b >= 24 && b <= 28) { add("bass", t0 + 1.5, c.bass, 0.6, 0.6); add("bass", t0 + 2.25, m(c.bass) + 12, 0.5, 0.4); }
    }
    if (b === 30 || b >= 31) add("bass", t0, c.bass, 2.8, 0.75);
    // the felt piano: broken chords in eighths while you write, then through most of the film
    const arpBars = (b >= 3 && b <= 28 && b !== 7 && !(b >= 8 && b <= 11)) || b === 30;
    if (arpBars) {
      const pat = b === 30 ? [0, 2, 3, 1] : [0, 1, 2, 3, 2, 1, 2, 3];
      const step = b === 30 ? 0.75 : 0.375;
      pat.forEach((k, i) => {
        const v = (i % 4 === 0 ? 0.62 : 0.42) * (0.9 + R() * 0.2) * (b >= 20 && b <= 23 ? 0.75 : 1);
        add("piano", hum(t0 + i * step), c.arp[k], step * 1.6, v, -0.25);
      });
    }
  });
  // the cold open: a needle's motif as the name is sewn, a bell for the knot
  [["A4", 0.4], ["D5", 0.9], ["E5", 1.35], ["F#5", 1.8], ["E5", 2.25], ["A5", 2.7], ["F#5", 3.15], ["E5", 3.6]].forEach(([n, t]) => add("pluck", hum(t, 0.01), n, 0.5, 0.55, 0.2));
  add("bell", 4.5, "D6", 1.5, 0.8, 0.1); add("bell", 4.52, "A5", 1.5, 0.45, -0.1);
  add("piano", 5.1, "F#4", 2, 0.4, -0.2); add("piano", 5.85, "A4", 2, 0.34, -0.2); add("piano", 6.6, "E5", 2.2, 0.3, -0.2);
  add("pluck", 7.15, "D5", 0.6, 0.4, 0.3); add("pluck", 7.5, "A5", 0.8, 0.35, 0.3);
  // a pause, and a question (bar 7): unresolved, rising
  add("bell", 23.3, "E5", 1, 0.32, 0.2); add("bell", 23.75, "A5", 1.2, 0.3, 0.25);
  add("piano", 21.0, "B4", 1.5, 0.4, -0.2); add("piano", 21.75, "D5", 1.5, 0.36, -0.2); add("piano", 22.5, "E5", 1.8, 0.32, -0.2);
  // the sketch (bars 8-11): plucked sixteenths, in and out like a needle
  for (let b = 8; b <= 11; b++) {
    const c = CH[PROG[b]], t0 = bar(b);
    const hi = c.arp.map((x) => m(x) + 12);
    const pat = [3, 1, 2, 1, 3, 1, 2, 0, 3, 1, 2, 1, 3, 2, 1, 0];
    pat.forEach((k, i) => add("pluck", hum(t0 + i * 0.1875, 0.005), hi[k], 0.25, (i % 4 === 0 ? 0.42 : 0.26) * (0.9 + R() * 0.2), 0.35));
    add("piano", t0, c.arp[0], 2.8, 0.45, -0.2);
  }
  // each sketch step gets a small bell
  [29.9, 31.5, 33.1, 34.7].forEach((t, i) => add("bell", t, ["A5", "B5", "C#6", "D6"][i], 0.8, 0.22, 0.3));
  // fix the words: A7 at the end of bar 14 pulls to D
  add("piano", bar(14, 2), "G4", 1.4, 0.45, -0.2); add("piano", bar(14, 2.5), "C#5", 1.2, 0.45, -0.2); add("piano", bar(14, 3), "E5", 1, 0.5, -0.2);
  // tied off: home, on the downbeat, with a little strum of thread
  const tieOff = (t) => {
    add("bell", t, "D6", 2, 0.7, 0.1); add("bell", t + 0.01, "F#5", 2, 0.35, -0.1);
    ["D5", "F#5", "A5", "D6"].forEach((n, i) => add("pluck", t + 0.05 + i * 0.06, n, 1.2, 0.4 - i * 0.05, 0.3));
  };
  tieOff(45.0); tieOff(60.0);
  add("piano", bar(19, 2), "G4", 1.4, 0.42, -0.2); add("piano", bar(19, 2.5), "C#5", 1.2, 0.42, -0.2); add("piano", bar(19, 3), "E5", 1, 0.46, -0.2);
  // the room (bars 20-23): three minds, three voices — thread (pluck), piano, flute
  [["F#5", 0], ["E5", 1], ["D5", 1.5], ["E5", 2], ["A4", 3]].forEach(([n, bt]) => add("pluck", hum(bar(20, bt)), n, 0.9, 0.42, 0.45));
  [["D5", 0], ["B4", 1], ["D5", 2], ["F#5", 2.5], ["E5", 3]].forEach(([n, bt]) => add("pluck", hum(bar(21, bt)), n, 0.9, 0.4, 0.45));
  [["C#5", 0], ["E5", 1], ["A5", 2], ["G#5", 3]].forEach(([n, bt]) => add("piano", hum(bar(22, bt)), n, 0.8, 0.42, -0.45));
  [["F#5", 0], ["E5", 1.5], ["D5", 2]].forEach(([n, bt]) => add("flute", bar(23, bt), n, bt === 2 ? 1.2 : 1.3, 0.55, 0.05));
  [["A5", 0], ["F#5", 2]].forEach(([n, bt]) => add("flute", bar(22, bt), n, 1.8, 0.5, 0.05));
  [66.2, 66.3, 66.4].forEach((t, i) => add("bell", t, ["F#5", "A5", "D6"][i], 1.2, 0.3, [-0.4, 0, 0.4][i]));
  // a soft brush through the brief and the chat (bars 24-28)
  for (let b = 24; b <= 28; b++) for (let i = 0; i < 8; i++) add("perc", bar(b) + i * 0.375 + (i % 2 ? 0.02 : 0), 0, 0.08, i % 2 ? 0.35 : 0.55, (i % 2 ? 0.3 : -0.3));
  // hide threads: one note held (bar 29), then the threads come back
  add("piano", 87.9, "D5", 2.4, 0.45, 0); add("piano", 88.65, "A4", 2, 0.3, 0);
  ["F#5", "A5", "D6"].forEach((n, i) => add("pluck", 89.7 + i * 0.07, n, 0.8, 0.35, 0.3));
  // the close: the needle's motif on the bell, the knot, home
  [["A4", 93.0], ["D5", 93.375], ["E5", 93.75], ["F#5", 94.125]].forEach(([n, t]) => add("pluck", t, n, 0.6, 0.4, 0.2));
  add("bell", 94.5, "D6", 2.5, 0.7, 0.05); add("bell", 94.52, "A5", 2.5, 0.35, -0.05);
  add("bell", 96.0, "F#5", 2.4, 0.3, 0.2); add("bell", 96.02, "A5", 2.4, 0.25, -0.2);
  add("piano", 96.0, "D4", 2.5, 0.4, -0.2); add("piano", 96.0, "A4", 2.5, 0.32, -0.2);

  function renderNotes() {
    for (const n of notes) {
      const v = n.vel;
      if (n.inst === "piano") mixIn(B.piano, piano(n.midi, n.dur), n.t, v * 0.55, n.pan);
      else if (n.inst === "pluck") mixIn(B.pluck, pluck(n.midi, n.dur, 0.72), n.t, v * 0.5, n.pan);
      else if (n.inst === "bell") mixIn(B.bell, bell(n.midi, n.dur), n.t, v * 0.42, n.pan);
      else if (n.inst === "pad") mixIn(B.pad, pad(n.midi, n.dur), n.t, v * 0.5, n.pan);
      else if (n.inst === "bass") mixIn(B.bass, bass(n.midi, n.dur), n.t, v * 0.42, 0);
      else if (n.inst === "flute") mixIn(B.flute, flute(n.midi, n.dur), n.t, v * 0.6, n.pan);
      else if (n.inst === "perc") mixIn(B.perc, brush(n.dur), n.t, v * 0.05, n.pan);
    }
  }

  // ---------------------------------------------------------------- foley
  function noise(len, seed, color) {
    const r = rng(seed), b = new Float32Array(len);
    let lp = 0;
    for (let i = 0; i < len; i++) { const x = r() * 2 - 1; lp += (color || 1) * (x - lp); b[i] = lp; }
    return b;
  }
  /** A biquad band-pass, applied in place (RBJ cookbook). */
  function bandpass(b, f0, q, f1) {
    let x1 = 0, x2 = 0, y1 = 0, y2 = 0;
    for (let i = 0; i < b.length; i++) {
      const f = f1 ? f0 * Math.pow(f1 / f0, i / b.length) : f0;
      const w = (2 * Math.PI * f) / SR, al = Math.sin(w) / (2 * q), a0 = 1 + al;
      const b0 = al / a0, b2 = -al / a0, a1 = (-2 * Math.cos(w)) / a0, a2 = (1 - al) / a0;
      const x = b[i], y = b0 * x + b2 * x2 - a1 * y1 - a2 * y2;
      x2 = x1; x1 = x; y2 = y1; y1 = y; b[i] = y;
    }
    return b;
  }
  const env = (b, att, dec) => { for (let i = 0; i < b.length; i++) { const t = i / SR; b[i] *= Math.min(1, t / att) * Math.exp(-Math.max(0, t - att) / dec); } return b; };
  function brush(dur) { return memo(`br${dur}`, () => env(bandpass(noise(Math.floor(SR * (dur + 0.1)), 5, 1), 7000, 0.7), 0.004, dur / 2.5)); }
  let seed = 1000;
  const FX = {
    key(gain) {
      const s = seed++, len = Math.floor(SR * 0.05);
      const click = env(bandpass(noise(len, s, 1), 2400 + (s % 7) * 220, 1.4), 0.0008, 0.006);
      const thock = new Float32Array(len), f = 150 + (s % 5) * 14;
      for (let i = 0; i < len; i++) { const t = i / SR; thock[i] = Math.sin(2 * Math.PI * f * t) * Math.exp(-t / 0.012) * 0.6; }
      for (let i = 0; i < len; i++) click[i] = click[i] * 1.6 + thock[i] * 0.5;
      return { b: click, g: 0.16 * gain };
    },
    space(gain) { const k = FX.key(gain); const len = k.b.length; for (let i = 0; i < len; i++) k.b[i] += Math.sin(2 * Math.PI * 110 * i / SR) * Math.exp(-i / SR / 0.02) * 0.4; return k; },
    enter(gain) { const k = FX.key(1.2); return k; },
    chord() { const a = FX.key(1), b2 = FX.key(1), out = new Float32Array(a.b.length + Math.floor(SR * 0.09)); out.set(a.b, 0); for (let i = 0; i < b2.b.length; i++) out[i + Math.floor(SR * 0.09)] += b2.b[i]; return { b: out, g: 0.17 }; },
    click() {
      const len = Math.floor(SR * 0.04), s = seed++;
      const b = env(bandpass(noise(len, s, 1), 3800, 2.5), 0.0005, 0.004);
      for (let i = 0; i < len; i++) b[i] += Math.sin(2 * Math.PI * 95 * i / SR) * Math.exp(-i / SR / 0.008) * 0.25;
      return { b, g: 0.2 };
    },
    stitch(gain, i) {
      // the needle through the cloth: a tiny tick, then the thread following it
      const len = Math.floor(SR * 0.06), s = seed++;
      const tick = env(bandpass(noise(len, s, 1), (i % 2 ? 5200 : 4300), 2), 0.0004, 0.003);
      const draw = env(bandpass(noise(len, s + 1, 1), 1800, 1.2), 0.008, 0.018);
      for (let j = 0; j < len; j++) tick[j] = tick[j] * 1.0 + draw[j] * 0.35;
      return { b: tick, g: 0.1 * gain };
    },
    pull(gain) { const len = Math.floor(SR * 0.34); return { b: env(bandpass(noise(len, seed++, 1), 700, 6, 3200), 0.03, 0.12), g: 0.5 * gain }; },
    pop(gain) {
      const len = Math.floor(SR * 0.14), b = new Float32Array(len);
      let ph = 0;
      for (let i = 0; i < len; i++) { const t = i / SR, f = 560 * Math.pow(260 / 560, Math.min(1, t / 0.09)); ph += 2 * Math.PI * f / SR; b[i] = Math.sin(ph) * Math.min(1, t / 0.003) * Math.exp(-t / 0.035); }
      const c = env(bandpass(noise(len, seed++, 1), 2600, 1.5), 0.0005, 0.004);
      for (let i = 0; i < len; i++) b[i] += c[i] * 0.4;
      return { b, g: 0.42 * gain };
    },
    paper(gain) { const len = Math.floor(SR * 0.45); return { b: env(bandpass(noise(len, seed++, 0.9), 900, 0.6, 2600), 0.07, 0.14), g: 0.26 * gain }; },
    slide(gain) { const len = Math.floor(SR * 0.4); return { b: env(bandpass(noise(len, seed++, 0.9), 1400, 0.8, 600), 0.05, 0.12), g: 0.22 * gain }; },
    lift(gain) { const len = Math.floor(SR * 0.4); return { b: env(bandpass(noise(len, seed++, 1), 5000, 0.9, 2500), 0.06, 0.1), g: 0.12 * gain }; },
    lamp(gain) {
      // a lamp's pull chain: two small metal clicks, and the warmth coming up
      const len = Math.floor(SR * 0.3), b = new Float32Array(len);
      for (const [at, f] of [[0, 3400], [0.055, 2900]]) {
        const c = env(bandpass(noise(Math.floor(SR * 0.05), seed++, 1), f, 6), 0.0004, 0.006);
        const o = Math.floor(at * SR);
        for (let i = 0; i < c.length && o + i < len; i++) b[o + i] += c[i] * 1.2;
      }
      return { b, g: 0.3 * gain };
    },
    settle(gain) { return FX.paper(gain * 0.8); },
  };
  function renderFx(events) {
    for (const e of events) {
      const f = FX[e.kind];
      if (!f) continue;
      const { b, g } = f(e.gain == null ? 1 : e.gain, e.i || 0);
      const pan = e.kind === "stitch" ? 0.15 : e.kind === "key" || e.kind === "space" ? -0.1 : 0;
      mixIn(B.fx, b, e.t, g, pan);
    }
  }

  // ---------------------------------------------------------------- the room
  /** An impulse response: a short early cluster and a warm, decaying tail. */
  function ir(ctx, secs, damp, seed0) {
    const len = Math.floor(SR * secs), buf = ctx.createBuffer(2, len, SR);
    for (let ch = 0; ch < 2; ch++) {
      const d = buf.getChannelData(ch), r = rng(seed0 + ch);
      let lp = 0;
      for (let i = 0; i < len; i++) {
        const t = i / SR;
        lp += damp * ((r() * 2 - 1) - lp);
        d[i] = lp * Math.exp(-t * 3.2 / secs) * (t < 0.012 ? t / 0.012 : 1);
      }
    }
    return buf;
  }
  async function master(stems, wetSend, irSecs) {
    const ctx = new OfflineAudioContext(2, N, SR);
    const out = ctx.createGain(); out.gain.value = 1;
    const comp = ctx.createDynamicsCompressor();
    comp.threshold.value = -16; comp.ratio.value = 2.2; comp.attack.value = 0.01; comp.release.value = 0.25; comp.knee.value = 8;
    const verb = ctx.createConvolver(); verb.normalize = true; verb.buffer = ir(ctx, irSecs, 0.35, 11);
    const vg = ctx.createGain(); vg.gain.value = 1;
    verb.connect(vg).connect(out);
    out.connect(comp).connect(ctx.destination);
    for (const [b, gain, send] of stems) {
      const buf = ctx.createBuffer(2, N, SR);
      buf.copyToChannel(b.L, 0); buf.copyToChannel(b.R, 1);
      const src = ctx.createBufferSource(); src.buffer = buf;
      const g = ctx.createGain(); g.gain.value = gain;
      const s = ctx.createGain(); s.gain.value = send * wetSend;
      src.connect(g).connect(out);
      src.connect(s).connect(verb);
      src.start(0);
    }
    return ctx.startRendering();
  }
  function wav(buf) {
    const ch = [buf.getChannelData(0), buf.getChannelData(1)], len = ch[0].length;
    const bytes = new DataView(new ArrayBuffer(44 + len * 2 * 3));
    const str = (o, s) => { for (let i = 0; i < s.length; i++) bytes.setUint8(o + i, s.charCodeAt(i)); };
    str(0, "RIFF"); bytes.setUint32(4, 36 + len * 6, true); str(8, "WAVE"); str(12, "fmt ");
    bytes.setUint32(16, 16, true); bytes.setUint16(20, 1, true); bytes.setUint16(22, 2, true); bytes.setUint32(24, SR, true);
    bytes.setUint32(28, SR * 6, true); bytes.setUint16(32, 6, true); bytes.setUint16(34, 24, true); str(36, "data"); bytes.setUint32(40, len * 6, true);
    let o = 44;
    for (let i = 0; i < len; i++) for (let c = 0; c < 2; c++) {
      const v = Math.max(-1, Math.min(1, ch[c][i])) * 8388607;
      const x = Math.round(v) & 0xffffff;
      bytes.setUint8(o, x & 255); bytes.setUint8(o + 1, (x >> 8) & 255); bytes.setUint8(o + 2, (x >> 16) & 255); o += 3;
    }
    const u8 = new Uint8Array(bytes.buffer);
    let s = "";
    for (let i = 0; i < u8.length; i += 0x8000) s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    return btoa(s);
  }

  const out = {};
  window.SCORE = {
    ready: false, error: null,
    async render(events) {
      renderNotes();
      renderFx(events);
      // the score: dry instruments with a warm room; the foley: close, with a little air
      out.music = await master([[B.pad, 0.3, 0.55], [B.piano, 1.25, 0.36], [B.pluck, 1.1, 0.42], [B.bell, 1.0, 0.6], [B.bass, 0.62, 0.06], [B.flute, 0.9, 0.45], [B.perc, 1, 0.2]], 0.55, 2.8);
      out.fx = await master([[B.fx, 1, 0.12]], 1, 0.7);
      return { music: out.music.length / SR, fx: out.fx.length / SR, notes: notes.length, events: events.length };
    },
    /** The WAV as base64, handed over in pieces (a 98 s stereo stem is about 37 MB of text). */
    wav(stem) { out[stem + "64"] = wav(out[stem]); return out[stem + "64"].length; },
    chunk(stem, i, size) { return out[stem + "64"].slice(i * size, (i + 1) * size); },
  };
})();

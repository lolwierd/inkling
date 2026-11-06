// What a mix sounds like, in numbers, for when nobody can listen: loudness and true peak,
// how often something starts (onsets a minute), where the energy sits (spectral centroid,
// the share above 2 and 4 kHz), and clicks (short bursts of high frequencies that stand out
// of what is around them, and single-sample steps). Give it several files to compare them.
//
//   bun run design/video/tools/listen.ts <file.wav|file.mp4> [more files...] [--json out.json]
import { writeFileSync } from "node:fs";

const FFMPEG = "/opt/homebrew/bin/ffmpeg";
const SR = 48000;
const args = process.argv.slice(2);
const jsonAt = args.indexOf("--json");
const jsonOut = jsonAt >= 0 ? args.splice(jsonAt, 2)[1] : null;

async function ff(a: string[]) {
  const p = Bun.spawn([FFMPEG, "-hide_banner", "-nostats", ...a], { stdout: "pipe", stderr: "pipe" });
  const [o, e] = await Promise.all([new Response(p.stdout).arrayBuffer(), new Response(p.stderr).text()]);
  await p.exited;
  return { o, e };
}

/** In-place radix-2 FFT on (re, im). */
function fft(re: Float64Array, im: Float64Array) {
  const n = re.length;
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) { [re[i], re[j]] = [re[j], re[i]]; [im[i], im[j]] = [im[j], im[i]]; }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const ang = (-2 * Math.PI) / len, wr = Math.cos(ang), wi = Math.sin(ang);
    for (let i = 0; i < n; i += len) {
      let cr = 1, ci = 0;
      for (let k = 0; k < len / 2; k++) {
        const a = i + k, b = a + len / 2;
        const tr = re[b] * cr - im[b] * ci, ti = re[b] * ci + im[b] * cr;
        re[b] = re[a] - tr; im[b] = im[a] - ti; re[a] += tr; im[a] += ti;
        const nr = cr * wr - ci * wi; ci = cr * wi + ci * wr; cr = nr;
      }
    }
  }
}

async function analyse(file: string) {
  const r: any = { file };
  const eb = (await ff(["-i", file, "-map", "0:a", "-af", "ebur128=peak=true", "-f", "null", "-"])).e;
  const sum = eb.slice(eb.lastIndexOf("Summary:"));
  r.integratedLUFS = +(/I:\s+(-?[\d.]+) LUFS/.exec(sum)?.[1] ?? NaN);
  r.LRA = +(/LRA:\s+([\d.]+) LU/.exec(sum)?.[1] ?? NaN);
  r.truePeakDBTP = +(/Peak:\s+(-?[\d.]+) dBFS/.exec(sum)?.[1] ?? NaN);
  const x = new Float32Array((await ff(["-i", file, "-map", "0:a", "-ac", "1", "-ar", String(SR), "-f", "f32le", "-"])).o);
  const minutes = x.length / SR / 60;
  r.seconds = +(x.length / SR).toFixed(1);

  // ---- the spectrum, frame by frame
  const n = 2048, hop = 512, win = new Float64Array(n).map((_, i) => 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (n - 1)));
  const bins = n / 2, hz = SR / n, top = Math.floor(8000 / hz);
  let prev: Float64Array | null = null;
  const flux: number[] = [], rmsDb: number[] = [];
  let cSum = 0, eSum = 0, e2k = 0, e4k = 0, eAll = 0;
  const cents: number[] = [];
  for (let s = 0; s + n <= x.length; s += hop) {
    const re = new Float64Array(n), im = new Float64Array(n);
    let ms = 0;
    for (let i = 0; i < n; i++) { re[i] = x[s + i] * win[i]; ms += x[s + i] * x[s + i]; }
    const db = 10 * Math.log10(ms / n + 1e-12);
    rmsDb.push(db);
    fft(re, im);
    const mag = new Float64Array(bins), lg = new Float64Array(bins);
    let num = 0, den = 0, en = 0, en2 = 0, en4 = 0;
    for (let k = 1; k < bins; k++) {
      const m = Math.hypot(re[k], im[k]);
      mag[k] = m; lg[k] = Math.log1p(1000 * m);
      num += k * hz * m; den += m;
      const p = m * m; en += p; if (k * hz >= 2000) en2 += p; if (k * hz >= 4000) en4 += p;
    }
    let f = 0;
    if (prev) for (let k = 1; k < top; k++) { const d = lg[k] - prev[k]; if (d > 0) f += d; }
    flux.push(f);
    prev = lg;
    if (db > -50 && den > 0) { const c = num / den; cents.push(c); cSum += c * en; eSum += en; }
    eAll += en; e2k += en2; e4k += en4;
  }
  cents.sort((a, b) => a - b);
  r.centroidMedianHz = Math.round(cents[cents.length >> 1] || 0);
  r.centroidEnergyWeightedHz = Math.round(cSum / (eSum || 1));
  r.energyAbove2kHzPct = +((100 * e2k) / eAll).toFixed(2);
  r.energyAbove4kHzPct = +((100 * e4k) / eAll).toFixed(3);

  // ---- onsets: spectral flux peaks over a moving median, at least 60 ms apart, in frames
  // that are not near-silent
  const fps = SR / hop, half = Math.round(0.4 * fps), gap = Math.round(0.06 * fps);
  const meanF = flux.reduce((a, b) => a + b, 0) / flux.length;
  let onsets = 0, last = -1e9;
  for (let i = 1; i < flux.length - 1; i++) {
    if (flux[i] < flux[i - 1] || flux[i] < flux[i + 1]) continue;
    if (rmsDb[i] < -55) continue;
    const w = flux.slice(Math.max(0, i - half), Math.min(flux.length, i + half + 1)).sort((a, b) => a - b);
    const thr = w[w.length >> 1] * 1.5 + 0.5 * meanF;
    if (flux[i] > thr && i - last >= gap) { onsets++; last = i; }
  }
  r.onsetsPerMinute = +(onsets / minutes).toFixed(1);

  // ---- clicks: (1) bursts above 6 kHz, 2 ms long, standing 13 dB out of the 200 ms around
  // them and louder than -70 dBFS; (2) single-sample steps, 12x the local second difference
  const b = biquadHP(6000);
  const hp = new Float32Array(x.length);
  for (let i = 0; i < x.length; i++) hp[i] = b(x[i]);
  const w2 = 96, blocks = Math.floor(hp.length / w2), be = new Float64Array(blocks);
  for (let k = 0; k < blocks; k++) { let s = 0; for (let j = 0; j < w2; j++) { const v = hp[k * w2 + j]; s += v * v; } be[k] = s / w2; }
  let bursts = 0;
  const around = 50; // blocks either side (200 ms)
  for (let k = around; k < blocks - around; k++) {
    if (be[k] < 1e-7) continue; // -70 dBFS
    let s = 0, c = 0;
    for (let j = k - around; j <= k + around; j++) { if (Math.abs(j - k) <= 2) continue; s += be[j]; c++; }
    const ctx = s / c;
    if (be[k] > 20 * ctx && be[k] >= be[k - 1] && be[k] >= be[k + 1]) bursts++;
  }
  r.hfClicksPerMinute = +(bursts / minutes).toFixed(2);
  let steps = 0;
  const d2 = new Float32Array(x.length);
  for (let i = 2; i < x.length; i++) d2[i] = Math.abs(x[i] - 2 * x[i - 1] + x[i - 2]);
  const L = 960;
  for (let s = 0; s + L <= x.length; s += L) {
    const seg = Array.from(d2.subarray(s, s + L)).sort((a, b) => a - b);
    const med = seg[L >> 1], mx = seg[L - 1];
    if (mx > 1e-3 && mx > 40 * Math.max(med, 1e-7)) steps++;
  }
  r.sampleStepsPerMinute = +(steps / minutes).toFixed(2);
  return r;
}
function biquadHP(f: number) {
  const w = (2 * Math.PI * f) / SR, c = Math.cos(w), s = Math.sin(w), q = Math.SQRT1_2, a = s / (2 * q);
  const b0 = (1 + c) / 2, b1 = -(1 + c), b2 = (1 + c) / 2, a0 = 1 + a, a1 = -2 * c, a2 = 1 - a;
  let x1 = 0, x2 = 0, y1 = 0, y2 = 0;
  return (x: number) => { const y = (b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0; x2 = x1; x1 = x; y2 = y1; y1 = y; return y; };
}

const results = [];
for (const f of args) { const r = await analyse(f); results.push(r); console.log(JSON.stringify(r)); }
if (jsonOut) writeFileSync(jsonOut, JSON.stringify(results, null, 1));

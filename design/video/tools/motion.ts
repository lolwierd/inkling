// How smoothly a film moves, frame by frame. For every pair of consecutive frames it
// measures how far the picture moved (phase correlation on a central crop, sub-pixel) and,
// given the camera's own path from the timeline (render.ts camera), compares the two:
//   - a frame that barely moves while the camera should have moved, followed by one that
//     moves twice as far, is a stale capture (a repeated frame and a skipped one);
//   - the timeline's path itself: the largest step per frame, and how often the camera
//     bounces (its velocity turns back mid-move, as an overshooting spring does).
// Also counts frames whose timestamps are uneven and frames identical to the one before.
//
//   bun run tools/motion.ts <film.mp4> [camera.json] [--json out.json]
// camera.json: [[x, y, z], ...] per frame at the film's rate, world units (render.ts camera).
import { readFileSync, writeFileSync } from "node:fs";

const FFMPEG = "/opt/homebrew/bin/ffmpeg", FFPROBE = "/opt/homebrew/bin/ffprobe";
const args = process.argv.slice(2);
const jsonAt = args.indexOf("--json");
const jsonOut = jsonAt >= 0 ? args.splice(jsonAt, 2)[1] : null;
const [file, camFile] = args;
const W = 480, H = 270, N = 256, X0 = (W - N) >> 1, Y0 = (H - N) >> 1;
const SCALE = 1920 / W; // report in 1080p pixels

async function run(cmd: string[]) {
  const p = Bun.spawn(cmd, { stdout: "pipe", stderr: "pipe" });
  const [o, e] = await Promise.all([new Response(p.stdout).arrayBuffer(), new Response(p.stderr).text()]);
  await p.exited;
  return { o: new Uint8Array(o), e };
}

function fft(re: Float64Array, im: Float64Array, inv = false) {
  const n = re.length;
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) { let t = re[i]; re[i] = re[j]; re[j] = t; t = im[i]; im[i] = im[j]; im[j] = t; }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const ang = ((inv ? 2 : -2) * Math.PI) / len, wr = Math.cos(ang), wi = Math.sin(ang);
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
/** 2D FFT of an N x N block, rows then columns. */
function fft2(re: Float64Array, im: Float64Array, inv = false) {
  const r = new Float64Array(N), i = new Float64Array(N);
  for (let y = 0; y < N; y++) { for (let x = 0; x < N; x++) { r[x] = re[y * N + x]; i[x] = im[y * N + x]; } fft(r, i, inv); for (let x = 0; x < N; x++) { re[y * N + x] = r[x]; im[y * N + x] = i[x]; } }
  for (let x = 0; x < N; x++) { for (let y = 0; y < N; y++) { r[y] = re[y * N + x]; i[y] = im[y * N + x]; } fft(r, i, inv); for (let y = 0; y < N; y++) { re[y * N + x] = r[y]; im[y * N + x] = i[y]; } }
}
const hann = new Float64Array(N).map((_, k) => 0.5 - 0.5 * Math.cos((2 * Math.PI * k) / (N - 1)));
function spectrum(frame: Uint8Array) {
  const re = new Float64Array(N * N), im = new Float64Array(N * N);
  let mean = 0;
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) mean += frame[(Y0 + y) * W + X0 + x];
  mean /= N * N;
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) re[y * N + x] = (frame[(Y0 + y) * W + X0 + x] - mean) * hann[x] * hann[y];
  fft2(re, im);
  return { re, im };
}
/** The shift that takes frame a onto frame b, in crop pixels, and the peak's strength. */
function shift(a: { re: Float64Array; im: Float64Array }, b: { re: Float64Array; im: Float64Array }) {
  const re = new Float64Array(N * N), im = new Float64Array(N * N);
  for (let k = 0; k < N * N; k++) {
    const pr = b.re[k] * a.re[k] + b.im[k] * a.im[k], pi = b.im[k] * a.re[k] - b.re[k] * a.im[k];
    const m = Math.hypot(pr, pi) + 1e-9;
    re[k] = pr / m; im[k] = pi / m;
  }
  fft2(re, im, true);
  let best = 0, bv = -Infinity;
  for (let k = 0; k < N * N; k++) if (re[k] > bv) { bv = re[k]; best = k; }
  const py = Math.floor(best / N), px = best % N;
  const at = (x: number, y: number) => re[((y + N) % N) * N + ((x + N) % N)];
  const sub = (l: number, c: number, r: number) => { const d = l - 2 * c + r; return d === 0 ? 0 : (0.5 * (l - r)) / d; };
  let dx = px + sub(at(px - 1, py), bv, at(px + 1, py)), dy = py + sub(at(px, py - 1), bv, at(px, py + 1));
  if (dx > N / 2) dx -= N; if (dy > N / 2) dy -= N;
  return { dx, dy, peak: bv / (N * N) };
}

const report: any = { file };
// timestamps
const pts = new TextDecoder().decode((await run([FFPROBE, "-v", "error", "-select_streams", "v", "-show_entries", "frame=pts_time", "-of", "csv=p=0", file])).o).trim().split("\n").map(Number);
const deltas = pts.slice(1).map((t, i) => t - pts[i]);
const step = deltas.slice().sort((a, b) => a - b)[deltas.length >> 1];
report.frames = pts.length;
report.fps = +(1 / step).toFixed(3);
report.unevenTimestamps = deltas.filter((d) => Math.abs(d - step) > step * 0.05).length;
// the picture
const raw = (await run([FFMPEG, "-hide_banner", "-loglevel", "error", "-i", file, "-map", "0:v", "-vf", `scale=${W}:${H}:flags=area,format=gray`, "-f", "rawvideo", "-"])).o;
const frames = Math.floor(raw.length / (W * H));
let identical = 0;
const moves: { dx: number; dy: number; peak: number }[] = [];
let prev = spectrum(raw.subarray(0, W * H)), prevRaw = raw.subarray(0, W * H);
for (let f = 1; f < frames; f++) {
  const cur = raw.subarray(f * W * H, (f + 1) * W * H);
  let same = true;
  for (let k = 0; k < cur.length; k += 7) if (cur[k] !== prevRaw[k]) { same = false; break; }
  if (same) identical++;
  const s = spectrum(cur);
  const m = shift(prev, s);
  moves.push({ dx: m.dx * SCALE, dy: m.dy * SCALE, peak: m.peak });
  prev = s; prevRaw = cur;
}
report.identicalToPrevious = identical;
const speed = moves.map((m) => Math.hypot(m.dx, m.dy));
report.measuredMaxStepPx = +Math.max(...speed).toFixed(1);
report.measuredStepsOver20Px = speed.filter((v) => v > 20).length;
// against the timeline: stale captures and skipped frames during moves
if (camFile) {
  const cam: number[][] = JSON.parse(readFileSync(camFile, "utf8"));
  // the screen motion of the world point under the frame's centre, in 1080p pixels
  const pred = cam.slice(1).map((c, i) => { const p = cam[i]; return { dx: (p[0] - c[0]) * c[2], dy: (p[1] - c[1]) * c[2], dz: c[2] / p[2] - 1 }; });
  let stale = 0, skipped = 0, considered = 0;
  const examples: number[] = [];
  for (let i = 1; i < Math.min(pred.length, moves.length) - 1; i++) {
    const want = Math.hypot(pred[i].dx, pred[i].dy);
    if (want < 3 || Math.abs(pred[i].dz) > 0.004 || want > 60) continue; // pans the crop can follow
    if (Math.hypot(pred[i - 1].dx - pred[i].dx, pred[i - 1].dy - pred[i].dy) > want * 0.5) continue; // a cut or a jolt in the path itself
    considered++;
    const got = Math.hypot(moves[i].dx, moves[i].dy);
    if (got < 0.25 * want) { stale++; if (examples.length < 12) examples.push(i + 1); }
    else if (got > 1.75 * want) skipped++;
  }
  report.panFramesChecked = considered;
  report.staleFrames = stale;
  report.doubleStepFrames = skipped;
  report.staleExamples = examples;
  // the path itself
  const v = pred.map((p) => Math.hypot(p.dx, p.dy));
  report.timelineMaxStepPx = +Math.max(...v.filter((x) => x < 400)).toFixed(1);
  report.timelineCuts = v.filter((x) => x >= 400).length;
  let bounces = 0;
  for (let i = 2; i < pred.length; i++) {
    const a = pred[i - 1], b = pred[i];
    const dot = a.dx * b.dx + a.dy * b.dy;
    if (Math.hypot(a.dx, a.dy) > 0.6 && Math.hypot(b.dx, b.dy) > 0.6 && dot < 0) bounces++;
    if (Math.abs(a.dz) > 0.0015 && Math.abs(b.dz) > 0.0015 && Math.sign(a.dz) !== Math.sign(b.dz)) bounces++;
  }
  report.timelineDirectionReversals = bounces;
  let jerky = 0;
  for (let i = 2; i < pred.length; i++) { const a = Math.hypot(pred[i].dx - pred[i - 1].dx, pred[i].dy - pred[i - 1].dy); if (a > 6 && v[i] < 400 && v[i - 1] < 400) jerky++; }
  report.timelineAccelOver6PxPerFrame2 = jerky;
}
console.log(JSON.stringify(report));
if (jsonOut) writeFileSync(jsonOut, JSON.stringify({ report, moves }, null, 1));

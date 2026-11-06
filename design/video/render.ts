// Renders the film with headless Chrome, driven over the DevTools protocol (no packages).
//
//   bun run render.ts stills <scratch> 0.5,12,30.2 [--res=720]    one PNG per time, into <scratch>/stills
//   bun run render.ts audio <scratch>                               the score's stems, into audio/music.wav and fx.wav
//   bun run render.ts frames <scratch> [--workers=8] [--only=3,4]   the picture, chapter by chapter (see below)
//   bun run render.ts page <scratch> [--kbps=N]                     segments + audio/mix.wav -> page/film.mp4, under 20 MiB
//   bun run render.ts moments <scratch>                             coverage.md and page/moments.json from the cut
//   bun run render.ts paths <scratch>                               how smoothly the camera, threads, knots and pointer move
//   bun run render.ts eval <scratch> '<expression>'                 a value from the page, for tuning
//
// The picture is rendered at 1080p and the film's own rate (60 fps), in segments that follow
// the film's plan (film.js PLAN): one per moment, cut into chunks of at most 360 frames so eight
// Chromes can share the work. segments/manifest.json remembers each chunk's chapter, frames
// and signature (the chapter's moments and their lengths, and the look of the rebuild:
// app.js, settings.js, film.html). `frames` renders only the chunks whose signature or
// length changed, plus every chapter named in --only (use it when a chapter's content
// changed but its moments did not); the rest are kept, even when they now start later.
// `page` joins the chunks in the plan's order with ffmpeg (stream copy, no re-encode) and
// encodes the page's film from them in two passes. So a second cut re-renders only what it
// touched:   bun run render.ts frames <scratch> --only=3 && bun run render.ts page <scratch>
// No transition crosses a moment's edge in the core cut, so a moment re-renders on its own.
//
// Every Chrome gets its own --user-data-dir under <scratch>, so nothing touches a real profile.
import { mkdirSync, readFileSync, writeFileSync, existsSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";

const HERE = import.meta.dir;
const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const FFMPEG = "/opt/homebrew/bin/ffmpeg";
const [mode, scratch, ...rest] = process.argv.slice(2);
if (!mode || !scratch) {
  console.error("usage: bun run render.ts <stills|audio|frames|page|moments|paths|eval|worker> <scratch> ...");
  process.exit(2);
}
mkdirSync(scratch, { recursive: true });
const opt = (k: string, d?: string) => rest.find((a) => a.startsWith(`--${k}=`))?.split("=")[1] ?? d;
const RES = +(opt("res", "1080") as string); // the full frame; 720 draws a third fewer pixels
const SCALE = RES / 1080;
const W = Math.round(1920 * SCALE), H = Math.round(1080 * SCALE);

// ----- a tiny DevTools client ------------------------------------------------------
class Tab {
  ws!: WebSocket;
  id = 0;
  waits = new Map<number, (m: any) => void>();
  proc!: ReturnType<typeof Bun.spawn>;
  static async open(profile: string, page: string) {
    const t = new Tab();
    rmSync(join(profile, "DevToolsActivePort"), { force: true });
    t.proc = Bun.spawn([CHROME, "--headless=new", `--user-data-dir=${profile}`, "--remote-debugging-port=0", "--no-first-run", "--no-default-browser-check",
      "--hide-scrollbars", "--window-size=1920,1080", "--force-color-profile=srgb", "--allow-file-access-from-files",
      "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--disable-backgrounding-occluded-windows", "--font-render-hinting=none", "about:blank"],
      { stdout: "ignore", stderr: "ignore" });
    let port = "";
    for (let i = 0; i < 200 && !port; i++) {
      await Bun.sleep(50);
      const f = join(profile, "DevToolsActivePort");
      if (existsSync(f)) port = readFileSync(f, "utf8").split("\n")[0].trim();
    }
    if (!port) throw new Error("Chrome did not start");
    let list: any[] = [];
    for (let i = 0; i < 100 && !list.some((x) => x.type === "page"); i++) { list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json(); if (!list.some((x) => x.type === "page")) await Bun.sleep(50); }
    const target = list.find((x) => x.type === "page");
    t.ws = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((r, j) => { t.ws.onopen = r; t.ws.onerror = j; });
    t.ws.onmessage = (e) => { const m = JSON.parse(String(e.data)); if (m.id && t.waits.has(m.id)) { t.waits.get(m.id)!(m); t.waits.delete(m.id); } };
    // the page is laid out at 1920 x 1080 and drawn at the output's size
    await t.send("Emulation.setDeviceMetricsOverride", { width: 1920, height: 1080, deviceScaleFactor: SCALE, mobile: false });
    await t.send("Page.enable");
    await t.send("Page.navigate", { url: page });
    return t;
  }
  send(method: string, params: object = {}): Promise<any> {
    const id = ++this.id;
    return new Promise((res, rej) => {
      this.waits.set(id, (m) => (m.error ? rej(new Error(`${method}: ${m.error.message}`)) : res(m.result)));
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
  async eval(expr: string) {
    const r = await this.send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
    return r.result.value;
  }
  async ready(flag = "window.FILM && (window.FILM.ready || window.FILM.error)") {
    for (let i = 0; i < 600; i++) {
      const v = await this.eval(`!!(${flag})`).catch(() => false);
      if (v) break;
      await Bun.sleep(100);
    }
    const err = await this.eval("window.FILM && window.FILM.error");
    if (err) throw new Error(err);
  }
  async png(): Promise<Buffer> {
    const r = await this.send("Page.captureScreenshot", { format: "png", clip: { x: 0, y: 0, width: 1920, height: 1080, scale: 1 }, optimizeForSpeed: true });
    return Buffer.from(r.data, "base64");
  }
  close() { try { this.ws.close(); } catch {} this.proc.kill(); }
}

const PAGE = `file://${join(HERE, "film.html")}`;
const run = async (args: string[]) => { const p = Bun.spawn([FFMPEG, "-hide_banner", ...args], { stdout: "inherit", stderr: "inherit" }); if (await p.exited) throw new Error(`ffmpeg ${args.join(" ")} failed`); };

async function stills(times: number[]) {
  const dir = join(scratch, "stills");
  mkdirSync(dir, { recursive: true });
  const tab = await Tab.open(join(scratch, "chrome-stills"), PAGE);
  try {
    await tab.ready();
    for (const t of times) {
      await tab.eval(`FILM.renderAt(${t})`);
      writeFileSync(join(dir, `s_${t.toFixed(2).padStart(6, "0")}.png`), await tab.png());
    }
    console.log(`stills: ${times.length} in ${dir}`);
  } finally { tab.close(); }
}

/** One worker: frames [f0, f1) into an H.264 segment at the output's size. Every segment is
 *  encoded alike (one keyframe at its start), so they join with a stream copy. */
async function worker(f0: number, f1: number, out: string, profile: string) {
  const tab = await Tab.open(profile, PAGE);
  try {
    await tab.ready();
    const fps = await tab.eval("FILM.FPS");
    // frame n is drawn at exactly t = n / fps; every segment is encoded alike (one keyframe
    // a second, the first at its start), so they join with a stream copy
    const ff = Bun.spawn([FFMPEG, "-y", "-loglevel", "error", "-f", "image2pipe", "-framerate", String(fps), "-c:v", "png", "-i", "-",
      "-vf", `scale=${W}:${H}:flags=lanczos,scale=out_color_matrix=bt709:out_range=tv,format=yuv420p`, "-c:v", "libx264", "-preset", "faster", "-crf", "12", "-profile:v", "high",
      "-g", String(fps), "-bf", "2", "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv", "-r", String(fps), out], { stdin: "pipe", stderr: "inherit" });
    const t0 = Date.now();
    for (let f = f0; f < f1; f++) {
      await tab.eval(`FILM.renderAt(${f} / ${fps})`);
      ff.stdin.write(await tab.png());
      if ((f - f0) % 120 === 119) console.log(`[${out.split("/").pop()}] ${f + 1 - f0}/${f1 - f0} · ${((Date.now() - t0) / (f - f0 + 1)).toFixed(0)} ms/frame`);
    }
    ff.stdin.end();
    await ff.exited;
  } finally { tab.close(); }
}

/** The film's chunks under its current plan: one or more per chapter, at most CHUNK frames. */
const CHUNK = 360;
async function chunks() {
  const probe = await Tab.open(join(scratch, "chrome-probe"), PAGE);
  let info: any;
  try {
    await probe.ready();
    info = await probe.eval("({ fps: FILM.FPS, dur: FILM.DUR, plan: FILM.PLAN.map((ch) => ({ n: ch.n, t0: FILM.CHS[ch.n], moments: ch.moments.map((m) => [m.id, m.dur]) })) })");
  } finally { probe.close(); }
  const look = createHash("sha1");
  for (const f of ["app.js", "settings.js", "film.html"]) look.update(readFileSync(join(HERE, f)));
  const lookSig = look.digest("hex").slice(0, 10);
  const out: { id: string; ch: number; f0: number; f1: number; sig: string }[] = [];
  const fr = (t: number) => Math.round(t * info.fps);
  info.plan.forEach((ch: any, i: number) => {
    const F0 = fr(ch.t0), F1 = i + 1 < info.plan.length ? fr(info.plan[i + 1].t0) : fr(info.dur);
    const sig = createHash("sha1").update(JSON.stringify(ch.moments) + lookSig + RES).digest("hex").slice(0, 12);
    for (let k = 0, a = F0; a < F1; k++, a += CHUNK) out.push({ id: `ch${ch.n}-${k}`, ch: ch.n, f0: a, f1: Math.min(F1, a + CHUNK), sig });
  });
  return out;
}

async function frames() {
  const workers = +(opt("workers", "8") as string);
  const only = new Set((opt("only", "") as string).split(",").filter(Boolean).map(Number));
  const segDir = join(scratch, "segments");
  mkdirSync(segDir, { recursive: true });
  const manPath = join(segDir, "manifest.json");
  const old: Record<string, any> = existsSync(manPath) ? JSON.parse(readFileSync(manPath, "utf8")) : {};
  const all = await chunks();
  const todo = all.filter((c) => {
    const o = old[c.id], file = join(segDir, `${c.id}.mp4`);
    return only.has(c.ch) || !o || o.sig !== c.sig || o.f1 - o.f0 !== c.f1 - c.f0 || !existsSync(file) || statSync(file).size < 1000;
  });
  console.log(`frames: ${all.length} chunks, ${todo.length} to render (${todo.map((c) => c.id).join(" ") || "none"})`);
  const t0 = Date.now();
  // a pool of workers, each its own process, so a slow frame in one never holds up the others
  let next = 0, failed = "";
  const lane = async (k: number) => {
    while (next < todo.length) {
      const c = todo[next++];
      const p = Bun.spawn([process.execPath, "run", join(HERE, "render.ts"), "worker", scratch, `--f0=${c.f0}`, `--f1=${c.f1}`, `--out=${join(segDir, `${c.id}.mp4`)}`, `--profile=${join(scratch, `chrome-w${k}`)}`, `--res=${RES}`], { stdout: "inherit", stderr: "inherit" });
      if (await p.exited) failed += ` ${c.id}`;
    }
  };
  await Promise.all(Array.from({ length: Math.min(workers, todo.length) }, (_, k) => lane(k)));
  if (failed) throw new Error(`chunks failed:${failed}`);
  const man: Record<string, any> = {};
  for (const c of all) man[c.id] = c;
  writeFileSync(manPath, JSON.stringify(man, null, 1));
  writeFileSync(join(segDir, "list.txt"), all.map((c) => `file '${join(segDir, `${c.id}.mp4`)}'`).join("\n") + "\n");
  console.log(`frames: ${todo.length} chunks in ${((Date.now() - t0) / 1000).toFixed(1)} s with ${workers} workers`);
}

/** The page's film: the chunks joined (stream copy), with the mix, two-pass x264 at about
 *  620 kbit/s and AAC at 96 kbit/s, under 20 MiB. */
async function page() {
  const segDir = join(scratch, "segments");
  const joined = join(scratch, "joined.mp4");
  await run(["-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i", join(segDir, "list.txt"), "-c", "copy", joined]);
  const outFile = join(HERE, "page", "film.mp4"), log = join(scratch, "x264-2pass");
  // the bitrate that fills about 19.4 MiB with the video, after 96 kbit/s of audio
  const probe = Bun.spawn(["/opt/homebrew/bin/ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", joined], { stdout: "pipe" });
  const dur = +(await new Response(probe.stdout).text()).trim();
  const kbps = opt("kbps") ?? String(Math.floor((19.4 * 8 * 1048.576) / dur - 96));
  console.log(`page: ${dur.toFixed(1)} s at ${kbps} kbit/s video + 96 kbit/s audio`);
  const v = ["-c:v", "libx264", "-preset", "slow", "-b:v", kbps + "k", "-maxrate", String(Math.round(+kbps * 2)) + "k", "-bufsize", String(Math.round(+kbps * 4)) + "k", "-profile:v", "high", "-pix_fmt", "yuv420p",
    "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv", "-passlogfile", log];
  await run(["-loglevel", "error", "-y", "-i", joined, ...v, "-pass", "1", "-an", "-f", "mp4", "/dev/null"]);
  // the mix's reverb rings past the picture: fade it with the picture's last 0.6 s, so the
  // film ends in silence rather than on a cut tail
  await run(["-loglevel", "error", "-y", "-i", joined, "-i", join(HERE, "audio", "mix.wav"), "-map", "0:v", "-map", "1:a", ...v, "-pass", "2",
    "-af", `afade=t=out:st=${(dur - 0.6).toFixed(3)}:d=0.6`, "-c:a", "aac", "-b:a", "96k", "-ar", "48000", "-shortest", "-movflags", "+faststart", outFile]);
  const size = statSync(outFile).size;
  console.log(`→ ${outFile}: ${(size / 1048576).toFixed(2)} MiB${size >= 20 * 1048576 ? " (over 20 MiB: lower --kbps)" : ""}`);
}

async function audio() {
  const dir = join(HERE, "audio");
  mkdirSync(dir, { recursive: true });
  const tab = await Tab.open(join(scratch, "chrome-audio"), PAGE);
  try {
    await tab.ready();
    const info = await tab.eval("SCORE.render()");
    console.log("score:", JSON.stringify(info));
    for (const stem of ["music", "fx"]) {
      const total = await tab.eval(`SCORE.wav(${JSON.stringify(stem)})`);
      const size = 4 << 20;
      let b64 = "";
      for (let i = 0; i * size < total; i++) b64 += await tab.eval(`SCORE.chunk(${JSON.stringify(stem)}, ${i}, ${size})`);
      writeFileSync(join(dir, `${stem}.wav`), Buffer.from(b64, "base64"));
    }
    console.log(`audio: ${dir}/music.wav, fx.wav`);
  } finally { tab.close(); }
}

if (mode === "stills") await stills((rest[0] || "0").split(",").map(Number));
else if (mode === "frames") await frames();
else if (mode === "worker") await worker(+opt("f0")!, +opt("f1")!, opt("out")!, opt("profile")!);
else if (mode === "audio") await audio();
else if (mode === "page") await page();
else if (mode === "eval") {
  const tab = await Tab.open(join(scratch, "chrome-eval"), PAGE);
  try { await tab.ready(); console.log(JSON.stringify(await tab.eval(rest[0]), null, 1)); } finally { tab.close(); }
}
else if (mode === "moments") {
  // writes coverage.md (what the film shows, moment by moment) and page/moments.json from the cut
  const tab = await Tab.open(join(scratch, "chrome-mom"), PAGE);
  try {
    await tab.ready();
    const { moments, dur, fps } = await tab.eval("({ moments: FILM.moments, dur: FILM.DUR, fps: FILM.FPS })");
    const mmss = (t: number) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`;
    let md = `# The walkthrough film: what it shows\n\nThe core cut: one note, the notes only. Generated from \`film.js\` by \`bun run render.ts moments <scratch>\`, so the times are the cut's own. ` +
      `Length ${mmss(dur)}, ${fps} fps.\n\n| At | Moment | Caption |\n|---|---|---|\n`;
    for (const m of moments) md += `| ${mmss(m.t)} | ${m.name} | ${m.caption ?? ""} |\n`;
    writeFileSync(join(HERE, "coverage.md"), md);
    writeFileSync(join(HERE, "page", "moments.json"), JSON.stringify({ length: mmss(dur), fps, moments: moments.map((m: any) => ({ ...m, at: mmss(m.t) })) }, null, 1) + "\n");
    console.log(`coverage.md and page/moments.json: ${moments.length} moments, ${mmss(dur)}`);
  } finally { tab.close(); }
}
else if (mode === "paths") {
  // How smoothly everything that moves on its own moves, from the timeline itself: the camera
  // (the world point under the frame's centre), and on screen the threads (every point of a
  // rope), the knots (from the tie through the flight to the border) and the pointer. For
  // each, the largest step, acceleration and jerk in 1080p px per frame: a jump, a snap or a
  // bounce shows as a spike in the last two. tools/motion.ts checks the encoded film.
  const tab = await Tab.open(join(scratch, "chrome-paths"), PAGE);
  try {
    await tab.ready();
    const out = await tab.eval(`(() => {
      const F = FILM, T = F.T, fps = F.FPS, N = Math.round(F.DUR * fps), S = { camera: [], threads: [], knots: [], pointer: [] };
      const flights = [[T.knot5, T.fly5], [T.knot7, T.fly7]];
      for (let n = 0; n < N; n++) {
        const t = n / fps, c = F.camAt(t), scr = (x, y) => [960 + (x - c.x) * c.z, 540 + (y - c.y) * c.z];
        F.renderAt(t);
        S.camera.push([c.x, c.y, c.z]);
        const p = document.querySelector('#sceneA svg.ov path[style*="stroke-width:1.75"]');
        let rope = null;
        if (p) {
          const g = p.parentNode.parentNode, m = (g.getAttribute("transform") || "").match(/translate\\(([-\\d.]+)[ ,]+([-\\d.]+)\\)/), ly = m ? +m[2] : 0;
          const v = (p.getAttribute("d").match(/-?\\d+(?:\\.\\d+)?/g) || []).map(Number);
          rope = []; for (let i = 0; i + 1 < v.length; i += 2) rope.push(scr(v[i], v[i + 1] + ly));
        }
        S.threads.push(rope);
        const fly = document.querySelector("#film svg.fly circle"), end = document.querySelector("#film g.endknot circle");
        const on = [...document.querySelectorAll("#sceneA svg.ov circle")].find((e) => +e.getAttribute("r") > 2);
        const xy = (e) => [+e.getAttribute("cx"), +e.getAttribute("cy")];
        S.knots.push(fly ? [xy(fly)] : end ? [xy(end)] : on && flights.some(([k, f]) => t >= k && t < f[0]) ? [scr(...xy(on))] : flights.some(([k, f]) => t >= f[1] && t < f[1] + 0.5) ? [scr(660, 68)] : null);
        const el = document.getElementById("cursor"), tm = el.style.display !== "none" && el.style.transform.match(/translate\\(([-\\d.]+)px,\\s*([-\\d.]+)px\\)/);
        S.pointer.push(tm ? [[+tm[1], +tm[2]]] : null);
      }
      return S;
    })()`);
    // the camera as the motion of the world point under the centre; the rest as positions
    const cam: number[][] = out.camera;
    const vel = cam.map((c, i) => (i ? [(cam[i - 1][0] - c[0]) * c[2], (cam[i - 1][1] - c[1]) * c[2]] : [0, 0]));
    const stats = (pos: (number[][] | null)[], v?: number[][]) => {
      let step = 0, acc = 0, jerk = 0, at = 0;
      const d = (a: number[][], b: number[][]) => a.map((p, k) => [p[0] - b[k][0], p[1] - b[k][1]]);
      const big = (a: number[][]) => Math.max(...a.map((p) => Math.hypot(p[0], p[1])));
      for (let i = 3; i < pos.length; i++) {
        let v0, v1, v2;
        if (v) [v0, v1, v2] = [[v[i - 2]], [v[i - 1]], [v[i]]];
        else {
          const w = pos.slice(i - 3, i + 1);
          if (w.some((x) => !x || x.length !== w[0]!.length)) continue;
          [v0, v1, v2] = [d(w[1]!, w[0]!), d(w[2]!, w[1]!), d(w[3]!, w[2]!)];
        }
        const a1 = d(v2, v1), a0 = d(v1, v0), j = big(d(a1, a0));
        step = Math.max(step, big(v2)); acc = Math.max(acc, big(a1));
        if (j > jerk) { jerk = j; at = i / 60; }
      }
      return { maxStep: +step.toFixed(1), maxAccel: +acc.toFixed(2), maxJerk: +jerk.toFixed(2), maxJerkAt: +at.toFixed(2) };
    };
    console.log(JSON.stringify({ camera: stats(cam.map(() => [[0, 0]]), vel), threads: stats(out.threads), knots: stats(out.knots), pointer: stats(out.pointer) }));
  } finally { tab.close(); }
}
else { console.error(`unknown mode ${mode}`); process.exit(2); }
process.exit(0);

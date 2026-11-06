// Renders the film with headless Chrome, driven over the DevTools protocol (no packages).
//
//   bun run render.ts stills <scratch> 0.5,12,30.2         one PNG per time, into <scratch>/stills
//   bun run render.ts frames <scratch> --workers=8          the whole film, split across 8 Chromes,
//                                                           each writing an H.264 segment; joined
//   bun run render.ts audio <scratch>                       music.wav and fx.wav (audio.html), into audio/
//   bun run render.ts mux <scratch>                         segments + audio/mix.wav -> inkling-walkthrough.mp4
//
// Every Chrome gets its own --user-data-dir under <scratch>, so nothing touches a real profile.
import { mkdirSync, readFileSync, writeFileSync, existsSync, rmSync } from "node:fs";
import { join } from "node:path";

const HERE = import.meta.dir;
const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const FFMPEG = "/opt/homebrew/bin/ffmpeg";
const [mode, scratch, ...rest] = process.argv.slice(2);
if (!mode || !scratch) {
  console.error("usage: bun run render.ts <stills|frames|audio|mux|worker> <scratch> ...");
  process.exit(2);
}
mkdirSync(scratch, { recursive: true });
const opt = (k: string, d?: string) => rest.find((a) => a.startsWith(`--${k}=`))?.split("=")[1] ?? d;

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
      "--hide-scrollbars", "--force-device-scale-factor=1", "--window-size=1920,1080", "--force-color-profile=srgb", "--allow-file-access-from-files",
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
    await t.send("Emulation.setDeviceMetricsOverride", { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });
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

/** One worker: frames [f0, f1) into an H.264 segment. */
async function worker(f0: number, f1: number, out: string, profile: string) {
  const tab = await Tab.open(profile, PAGE);
  const ff = Bun.spawn([FFMPEG, "-y", "-loglevel", "error", "-f", "image2pipe", "-framerate", "30", "-c:v", "png", "-i", "-",
    "-vf", "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p", "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-profile:v", "high",
    "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv", "-r", "30", out], { stdin: "pipe", stderr: "inherit" });
  try {
    await tab.ready();
    const fps = await tab.eval("FILM.FPS");
    const t0 = Date.now();
    for (let f = f0; f < f1; f++) {
      await tab.eval(`FILM.renderAt(${(f / fps).toFixed(6)})`);
      ff.stdin.write(await tab.png());
      if ((f - f0) % 60 === 59) console.log(`[${out.split("/").pop()}] ${f + 1 - f0}/${f1 - f0} · ${((Date.now() - t0) / (f - f0 + 1)).toFixed(0)} ms/frame`);
    }
    ff.stdin.end();
    await ff.exited;
  } finally { tab.close(); }
}

async function frames() {
  const workers = +(opt("workers", "8") as string);
  const from = +(opt("from", "0") as string), to = opt("to");
  const segDir = join(scratch, "segments");
  mkdirSync(segDir, { recursive: true });
  const probe = await Tab.open(join(scratch, "chrome-probe"), PAGE);
  await probe.ready();
  const fps = await probe.eval("FILM.FPS"), dur = await probe.eval("FILM.DUR");
  probe.close();
  const F0 = Math.round(from * fps), F1 = to ? Math.round(+to * fps) : Math.round(dur * fps);
  const per = Math.ceil((F1 - F0) / workers);
  const t0 = Date.now();
  const jobs: Promise<unknown>[] = [];
  const names: string[] = [];
  for (let k = 0; k < workers; k++) {
    const a = F0 + k * per, b = Math.min(F1, a + per);
    if (a >= b) break;
    const name = join(segDir, `seg${String(k).padStart(2, "0")}.mp4`);
    names.push(name);
    // each worker is its own process, so a slow frame in one never holds up the others
    const p = Bun.spawn([process.execPath, "run", join(HERE, "render.ts"), "worker", scratch, `--f0=${a}`, `--f1=${b}`, `--out=${name}`, `--profile=${join(scratch, `chrome-w${k}`)}`], { stdout: "inherit", stderr: "inherit" });
    jobs.push(p.exited.then((code) => { if (code) throw new Error(`worker ${k} exited ${code}`); }));
  }
  await Promise.all(jobs);
  writeFileSync(join(segDir, "list.txt"), names.map((n) => `file '${n}'`).join("\n") + "\n");
  console.log(`frames ${F0}..${F1} in ${((Date.now() - t0) / 1000).toFixed(1)} s with ${names.length} workers`);
}

async function audio() {
  const dir = join(HERE, "audio");
  mkdirSync(dir, { recursive: true });
  const tab = await Tab.open(join(scratch, "chrome-audio"), PAGE);
  try {
    await tab.ready();
    const info = await tab.eval("SCORE.render(FILM.events)");
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

async function mux() {
  const segDir = join(scratch, "segments");
  const out = join(HERE, "inkling-walkthrough.mp4");
  const p = Bun.spawn([FFMPEG, "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", join(segDir, "list.txt"), "-i", join(HERE, "audio", "mix.wav"),
    "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "256k", "-shortest", "-movflags", "+faststart", out], { stdout: "inherit", stderr: "inherit" });
  if (await p.exited) throw new Error("mux failed");
  console.log(`→ ${out}`);
}

if (mode === "stills") await stills((rest[0] || "0").split(",").map(Number));
else if (mode === "frames") await frames();
else if (mode === "worker") await worker(+opt("f0")!, +opt("f1")!, opt("out")!, opt("profile")!);
else if (mode === "audio") await audio();
else if (mode === "eval") {
  // bun run render.ts eval <scratch> '<expression>': for measuring the page while tuning
  const tab = await Tab.open(join(scratch, "chrome-eval"), PAGE);
  try { await tab.ready(); console.log(JSON.stringify(await tab.eval(rest[0]), null, 1)); } finally { tab.close(); }
}
else if (mode === "mux") await mux();
else { console.error(`unknown mode ${mode}`); process.exit(2); }
process.exit(0);

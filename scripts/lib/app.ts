// Launching Inkling for a tool: a scratch notebook, the fake brain, no focus, a window
// that keeps drawing, JSON logs. Also reading what a run leaves behind (the app log,
// the brain log, the notebook files) and taking screenshots of the window.
//
// Every launch here is safe by construction: the notebook is a fresh folder under
// outRoot() (never ~/Library/Application Support/Inkling), the brain is the fake one
// unless `real` is asked for, and the window opens in the background.

import { closeSync, cpSync, existsSync, mkdirSync, openSync, readdirSync, readFileSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

export const REPO = resolve(import.meta.dir, "../..");
export const E2E = join(REPO, "brain/test/e2e");
export const SCENARIOS = join(E2E, "scenarios");
export const FIXTURES = join(E2E, "fixtures");
export const FAKE_BRAIN = join(E2E, "fake-brain.ts");
export const WINDOW_TITLE = process.env.INKLING_WINDOW_TITLE || "Inkling Agents";

/** Where tool runs keep their notebooks, logs and screenshots. Never inside the repo:
 *  the brain's working folder lives in the notebook, and the Claude runtime must not
 *  find this repo's files from there. */
export function outRoot(): string {
  return process.env.INKLING_AGENT_OUT || join(process.env.TMPDIR || tmpdir(), "inkling-agent");
}

export function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
}

export function binary(): string {
  if (process.env.INKLING_BIN) return process.env.INKLING_BIN;
  return join(process.env.CARGO_TARGET_DIR || join(REPO, "target"), "debug/inkling");
}

export const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Run a command to completion, capturing its output. */
export async function sh(cmd: string[], opts: { cwd?: string; env?: Record<string, string | undefined>; timeoutMs?: number } = {}) {
  const started = Date.now();
  const proc = Bun.spawn(cmd, { cwd: opts.cwd ?? REPO, env: opts.env ?? process.env, stdout: "pipe", stderr: "pipe" });
  let timedOut = false;
  const timer = opts.timeoutMs ? setTimeout(() => ((timedOut = true), proc.kill()), opts.timeoutMs) : undefined;
  const [out, err] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text()]);
  const code = await proc.exited;
  if (timer) clearTimeout(timer);
  return { code: timedOut ? 124 : code, out, err, ms: Date.now() - started, timedOut };
}

/** `cargo build` (debug), so a run never tests a stale binary. A no-op build takes
 *  under a second. Returns the compiler's output when it fails. */
export async function build(): Promise<{ ok: boolean; ms: number; output: string }> {
  if (process.env.INKLING_BIN) return { ok: true, ms: 0, output: "" };
  const r = await sh(["cargo", "build", "--color", "never"]);
  return { ok: r.code === 0, ms: r.ms, output: r.err + r.out };
}

// ----- notebooks ---------------------------------------------------------------

export function fixtureNames(): string[] {
  return existsSync(FIXTURES) ? readdirSync(FIXTURES).filter((f) => statSync(join(FIXTURES, f)).isDirectory()).sort() : [];
}

/** A fresh notebook folder: empty, a named fixture, or a copy of another notebook's
 *  store files (notes, settings, minds, folders; not its logs). */
export function prepareNotebook(dir: string, from?: { fixture?: string; notebook?: string; settings?: Record<string, unknown> }) {
  mkdirSync(join(dir, "notes"), { recursive: true });
  if (from?.fixture) {
    const src = join(FIXTURES, from.fixture);
    if (!existsSync(src)) throw new Error(`no fixture ${from.fixture} (have: ${fixtureNames().join(", ")})`);
    cpSync(src, dir, { recursive: true });
  }
  if (from?.notebook) {
    for (const f of ["notes", "settings.json", "minds.json", "folders.json"]) {
      const src = join(from.notebook, f);
      if (existsSync(src)) cpSync(src, join(dir, f), { recursive: true });
    }
  }
  if (from?.settings) {
    const path = join(dir, "settings.json");
    const now = existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) : {};
    Bun.write(path, JSON.stringify({ ...now, ...from.settings }, null, 1));
  }
}

export interface Store {
  notes: any[];
  settings: any;
  minds: any[];
  folders: any[];
}

/** The notebook's files as the app left them. */
export function readStore(dir: string): Store {
  const json = (p: string, d: any) => {
    try {
      return JSON.parse(readFileSync(p, "utf8"));
    } catch {
      return d;
    }
  };
  const notesDir = join(dir, "notes");
  const notes = existsSync(notesDir)
    ? readdirSync(notesDir)
        .filter((f) => f.endsWith(".json"))
        .map((f) => json(join(notesDir, f), null))
        .filter(Boolean)
    : [];
  notes.sort((a, b) => String(b.updated_at).localeCompare(String(a.updated_at)));
  return { notes, settings: json(join(dir, "settings.json"), {}), minds: json(join(dir, "minds.json"), []), folders: json(join(dir, "folders.json"), []) };
}

// ----- logs ----------------------------------------------------------------------

export interface LogRecord {
  ms: number | null;
  level: string; // error | warn | info | debug | trace | raw (not from the logger: a panic, say)
  target: string;
  msg: string;
}

/** The app's log, written with INKLING_LOG_JSON=1 (one JSON object per line). Plain
 *  env_logger lines are read too, so logs from brain/test/e2e/run.sh parse as well. */
export function readLog(path: string): LogRecord[] {
  if (!existsSync(path)) return [];
  const out: LogRecord[] = [];
  for (const line of readFileSync(path, "utf8").split("\n")) {
    if (!line.trim()) continue;
    if (line.startsWith("{")) {
      try {
        const r = JSON.parse(line);
        out.push({ ms: r.ms ?? null, level: r.level ?? "info", target: r.target ?? "", msg: r.msg ?? "" });
        continue;
      } catch {
        // not one of ours: fall through
      }
    }
    const m = /^\[(\S+) (ERROR|WARN|INFO|DEBUG|TRACE)\s+(\S+)\] (.*)$/.exec(line);
    if (m) out.push({ ms: null, level: m[2].toLowerCase(), target: m[3], msg: m[4] });
    else if (out.length && out[out.length - 1].level !== "raw" && !line.startsWith("thread ")) out[out.length - 1].msg += "\n" + line;
    else out.push({ ms: null, level: "raw", target: "", msg: line });
  }
  return out;
}

export interface Dump {
  label: string;
  ms: number | null;
  state: any;
}

/** `dump:` steps, in order. */
export function dumpsOf(log: LogRecord[]): Dump[] {
  const out: Dump[] = [];
  for (const r of log) {
    const m = /^script dump (.*?): (\{[\s\S]*\})$/.exec(r.msg);
    if (!m) continue;
    try {
      out.push({ label: m[1], ms: r.ms, state: JSON.parse(m[2]) });
    } catch {
      out.push({ label: m[1], ms: r.ms, state: { unparsed: m[2] } });
    }
  }
  return out;
}

/** What went wrong in a run, as far as the log can tell: script steps that did nothing
 *  (they only warn), errors, and panics. */
export function problems(log: LogRecord[]) {
  return {
    scriptWarnings: log.filter((r) => r.level === "warn" && r.msg.startsWith("script")).map((r) => r.msg),
    errors: log.filter((r) => r.level === "error").map((r) => r.msg),
    panics: log.filter((r) => r.level === "raw" && /panicked at/.test(r.msg)).map((r) => r.msg),
  };
}

export interface BrainLine {
  t: number; // seconds since the brain started
  dir: "recv" | "send";
  data: any;
}

/** Requests and events the fake brain logged (`[fake t] recv {...}` / `send {...}`).
 *  The real brain logs neither, so a real run gives []. */
export function readBrainLog(path: string): BrainLine[] {
  if (!existsSync(path)) return [];
  const out: BrainLine[] = [];
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const m = /^\[fake\s+([\d.]+)\] (recv|send) (\{.*\})$/.exec(line);
    if (!m) continue;
    try {
      out.push({ t: Number(m[1]), dir: m[2] as "recv" | "send", data: JSON.parse(m[3]) });
    } catch {
      // a line cut short by a crash
    }
  }
  return out;
}

// ----- the window ----------------------------------------------------------------

// CGWindowListCopyWindowInfo through JavaScript for Automation: no Xcode, no extra
// dependency, about 0.2 s. Prints the windows of one process as JSON.
const WINDOWS_JXA = `
ObjC.import('CoreGraphics');
function run(argv) {
  const pid = Number(argv[0]);
  const list = ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionAll, $.kCGNullWindowID));
  const out = [];
  for (let i = 0; i < list.count; i++) {
    const d = ObjC.deepUnwrap(list.objectAtIndex(i));
    if (d.kCGWindowOwnerPID !== pid) continue;
    out.push({ id: d.kCGWindowNumber, name: d.kCGWindowName ?? null, layer: d.kCGWindowLayer, w: d.kCGWindowBounds.Width, h: d.kCGWindowBounds.Height });
  }
  return JSON.stringify(out);
}`;

/** The main window of process `pid`: the largest normal-layer window, or the one whose
 *  title contains `named` (case aside), such as "settings". */
export async function windowOf(pid: number, named?: string): Promise<number | null> {
  const r = await sh(["osascript", "-l", "JavaScript", "-e", WINDOWS_JXA, String(pid)], { timeoutMs: 10000 });
  if (r.code !== 0) return null;
  try {
    let wins = (JSON.parse(r.out) as { id: number; name: string | null; layer: number; w: number; h: number }[]).filter((w) => w.layer === 0);
    if (named) wins = wins.filter((w) => (w.name ?? "").toLowerCase().includes(named.toLowerCase()));
    wins.sort((a, b) => b.w * b.h - a.w * a.h);
    return wins[0]?.id ?? null;
  } catch {
    return null;
  }
}

/** Screenshot one window into `out` (PNG). It needs Screen Recording permission for
 *  whatever runs this tool; a covered window only shows a fresh frame while it is
 *  jiggled (INKLING_JIGGLE, which launch() turns on). */
export async function captureWindow(windowId: number, out: string): Promise<string | null> {
  mkdirSync(join(out, ".."), { recursive: true });
  const r = await sh(["screencapture", "-l", String(windowId), "-o", "-x", out], { timeoutMs: 15000 });
  return r.code === 0 && existsSync(out) ? null : (r.err.trim() || `screencapture exited ${r.code}`);
}

// ----- one launch ------------------------------------------------------------------

export interface LaunchOptions {
  /** Script file to play (INKLING_SCRIPT). */
  script: string;
  /** The notebook folder; app.log, brain.log and shots/ land in it too. */
  dir: string;
  env?: Record<string, string>;
  real?: boolean;
  timeoutMs: number;
  /** Window nudge interval in ms, so a covered window keeps drawing; 0 turns it off. */
  jiggle?: number;
  open?: string;
  /** Shots capture the window whose title contains this (e.g. "settings"), not the note's. */
  window?: string;
}

export interface LaunchResult {
  exit: number | null;
  timedOut: boolean;
  ms: number;
  pid: number;
  shots: { name: string; path: string | null; error?: string }[];
}

/** Play a script in the app and wait for it to quit (or time out). `shot: name` steps
 *  are captured here, while the app holds still for them. */
export async function launch(o: LaunchOptions): Promise<LaunchResult> {
  mkdirSync(join(o.dir, "notes"), { recursive: true });
  const logPath = join(o.dir, "app.log");
  const fd = openSync(logPath, "w");
  const env: Record<string, string | undefined> = { ...process.env };
  // Nothing from the caller's shell may point the app at another notebook, script or brain.
  for (const k of Object.keys(env)) if (k.startsWith("INKLING_") && k !== "INKLING_BUN" && k !== "INKLING_CLAUDE_PATH") delete env[k];
  Object.assign(env, {
    INKLING_DATA_DIR: o.dir,
    INKLING_SCRIPT: o.script,
    INKLING_BACKGROUND: "1",
    INKLING_WINDOW_TITLE: WINDOW_TITLE,
    INKLING_LOG_JSON: "1",
  });
  if ((o.jiggle ?? 120) > 0) env.INKLING_JIGGLE = String(o.jiggle ?? 120);
  if (!o.real) env.INKLING_BRAIN_ENTRY = FAKE_BRAIN;
  if (o.open) env.INKLING_OPEN = o.open;
  Object.assign(env, o.env ?? {});
  // Dumps are part of the test protocol, not optional diagnostics. An inherited
  // filter (including a module-specific one) must never hide them.
  env.RUST_LOG = "warn,inkling=info";

  const started = Date.now();
  const proc = Bun.spawn([binary()], { cwd: o.dir, env, stdin: "ignore", stdout: fd, stderr: fd });
  closeSync(fd);
  const result: LaunchResult = { exit: null, timedOut: false, ms: 0, pid: proc.pid, shots: [] };

  // Follow the log for `shot:` steps while the app runs.
  let offset = 0;
  let windowId: number | null = null;
  let done = false;
  const follow = (async () => {
    while (!done) {
      await sleep(50);
      let text = "";
      try {
        const size = statSync(logPath).size;
        if (size <= offset) continue;
        text = await Bun.file(logPath).slice(offset, size).text();
      } catch {
        continue;
      }
      const end = text.lastIndexOf("\n");
      if (end < 0) continue;
      offset += Buffer.byteLength(text.slice(0, end + 1));
      for (const line of text.slice(0, end).split("\n")) {
        let msg = line;
        try {
          msg = JSON.parse(line).msg ?? line;
        } catch {
          // a plain line
        }
        const m = /^script: Shot\("(.*)"\)$/.exec(msg);
        if (!m) continue;
        const name = m[1].replace(/[^\w.-]+/g, "-") || "shot";
        windowId ??= await windowOf(proc.pid, o.window);
        if (windowId === null) {
          result.shots.push({ name, path: null, error: o.window ? `no window titled like "${o.window}"` : "could not find the app's window" });
          continue;
        }
        const path = join(o.dir, "shots", `${name}.png`);
        const error = await captureWindow(windowId, path);
        result.shots.push(error ? { name, path: null, error } : { name, path });
      }
    }
  })();

  const timer = setTimeout(() => {
    result.timedOut = true;
    proc.kill("SIGTERM");
    setTimeout(() => proc.kill("SIGKILL"), 1500);
  }, o.timeoutMs);
  result.exit = await proc.exited;
  clearTimeout(timer);
  done = true;
  await follow;
  result.ms = Date.now() - started;
  return result;
}

/** A short name for a path inside outRoot(), for messages. */
export function short(path: string): string {
  return path.startsWith(outRoot()) ? join("$OUT", path.slice(outRoot().length)) : path;
}

export { basename };

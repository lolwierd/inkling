// End-to-end scenarios: brain/test/e2e/scenarios/*.txt played against the real app with
// the fake brain, then checked. A scenario is an INKLING_SCRIPT (steps: src/script.rs)
// plus lines only this runner reads:
//
//   # fixture: beta              start from brain/test/e2e/fixtures/beta/
//   # after: r04a-silent         start from the notebook that scenario left (a relaunch)
//   # env: FAKE_DELAY_READ_MS=2500 OTHER=1
//   # timeout: 150               seconds before the run is stopped (default 60)
//   # brain: real                needs the real model; skipped unless --real
//   # allow-warn: no thread      a script warning containing this text is expected
//   # tags: smoke                `scripts/dev e2e smoke` runs the tagged ones
//   dump: after                  (a script step) the page state as JSON, named `after`
//   expect: d.threads.length === 1   a JS expression that must be true after the run
//   shot: after                  (a script step) a screenshot, saved as shots/after.png
//
// An `expect:` sees `d`, the state from the nearest `dump:` above it, and the run's
// results: dumps (by label), notes / settings / minds / folders (the notebook files after
// the app quit), note(title), requests (what the app sent the brain), tasks (their task
// names), events (what the brain sent back), log (app log records), warnings, exit; and
// helpers: textOf(note), rangeText(note, thread), blockOf(note, thread or quote),
// overlaps(d). `requests` and the files are the final state, not the state at the dump.
//
// A run fails when an expectation is false or throws, a dump or shot is missing, the app
// panics, logs an error, exits non-zero or times out, or a script step only warned
// (unknown step, phrase not found, no such thread): a typo must not pass silently.

import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, rmdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  SCENARIOS,
  build,
  dumpsOf,
  launch,
  outRoot,
  prepareNotebook,
  problems,
  readBrainLog,
  readLog,
  readStore,
  sleep,
  stamp,
  type Dump,
} from "./app.ts";
import { lintScenarios } from "./lint.ts";

export interface Expect {
  line: number;
  expr: string;
  /** Index into the scenario's dumps of the nearest `dump:` above, or -1. */
  dump: number;
}

export interface Scenario {
  name: string;
  path: string;
  about: string;
  fixture?: string;
  after?: string;
  env: Record<string, string>;
  timeoutS: number;
  real: boolean;
  allowWarn: string[];
  tags: string[];
  dumps: string[];
  shots: string[];
  expects: Expect[];
  quits: boolean;
}

const HEADER = /^#\s*(fixture|after|env|timeout|brain|allow-warn|tags):\s*(.*)$/;

export function parseScenario(path: string): Scenario {
  const name = path.split("/").pop()!.replace(/\.txt$/, "");
  const s: Scenario = { name, path, about: "", env: {}, timeoutS: 60, real: false, allowWarn: [], tags: [], dumps: [], shots: [], expects: [], quits: false };
  const about: string[] = [];
  readFileSync(path, "utf8")
    .split("\n")
    .forEach((raw, i) => {
      const l = raw.trim();
      if (!l) return;
      if (l.startsWith("#")) {
        const h = HEADER.exec(l);
        if (!h) {
          if (s.dumps.length === 0 && s.expects.length === 0) about.push(l.replace(/^#\s?/, ""));
          return;
        }
        const v = h[2].trim();
        switch (h[1]) {
          case "fixture":
            s.fixture = v;
            break;
          case "after":
            s.after = v;
            break;
          case "env":
            for (const kv of v.split(/\s+/).filter(Boolean)) {
              const [k, ...rest] = kv.split("=");
              s.env[k] = rest.join("=");
            }
            break;
          case "timeout":
            s.timeoutS = Number(v) || s.timeoutS;
            break;
          case "brain":
            s.real = v === "real";
            break;
          case "allow-warn":
            s.allowWarn.push(v);
            break;
          case "tags":
            s.tags.push(...v.split(/[\s,]+/).filter(Boolean));
            break;
        }
        return;
      }
      // The app reads `cmd: arg`, splitting at the first colon (src/script.rs).
      const colon = l.indexOf(":");
      const cmd = (colon < 0 ? l : l.slice(0, colon)).trim();
      const arg = colon < 0 ? "" : l.slice(colon + 1).trim();
      if (cmd === "dump") s.dumps.push(arg);
      else if (cmd === "shot") s.shots.push(arg);
      else if (cmd === "quit") s.quits = true;
      else if (cmd === "expect") s.expects.push({ line: i + 1, expr: arg, dump: s.dumps.length - 1 });
    });
  s.about = about.join(" ");
  return s;
}

export function allScenarios(): Scenario[] {
  return readdirSync(SCENARIOS)
    .filter((f) => f.endsWith(".txt"))
    .sort()
    .map((f) => parseScenario(join(SCENARIOS, f)));
}

// ----- checking ---------------------------------------------------------------------

export interface ExpectResult {
  line: number;
  expr: string;
  dump: string | null;
  ok: boolean;
  error?: string;
}

export interface Result {
  name: string;
  status: "pass" | "fail" | "skip";
  why: string[];
  ms: number;
  dir: string | null;
  exit: number | null;
  expects: ExpectResult[];
  shots: { name: string; path: string | null; error?: string }[];
  dumps: number;
}

/** One line per thread and the fields that usually explain a failure. */
export function compactDump(state: any): string {
  if (!state || typeof state !== "object") return String(state);
  const parts: string[] = [];
  const keys = ["reading", "waiting", "pending", "chat_pending", "active", "error", "typing", "rechecking", "undo"];
  for (const k of keys) if (state[k] !== undefined && state[k] !== null && !(Array.isArray(state[k]) && state[k].length === 0) && state[k] !== false) parts.push(`${k}=${JSON.stringify(state[k])}`);
  const lines = [`note=${JSON.stringify(state.note?.title)} mode=${state.note?.mode} mind=${state.note?.mind} ${parts.join(" ")}`];
  for (const t of state.threads ?? []) lines.push(`  thread ${t.id.slice(0, 4)} ${t.kind}/${t.status}/${t.side} msgs=${t.msgs}${t.drawing ? " drawing" : ""} quote=${JSON.stringify(t.quote.slice(0, 48))}`);
  return lines.join("\n");
}

/** Helpers an `expect:` can call. Offsets in notes are UTF-8 byte offsets into the
 *  blocks joined by "\n" (title included), as the app stores them. */
const helpers = {
  /** A note's text as the app indexes it. */
  textOf: (n: any): string => (n?.blocks ?? []).map((b: any) => b.text).join("\n"),
  /** The text a thread's stored byte range covers. */
  rangeText: (n: any, t: any): string => Buffer.from(helpers.textOf(n)).subarray(t.start, t.end).toString(),
  /** The block (0 = title) a thread starts in; `t` may be a thread or its quote. */
  blockOf: (n: any, t: any): number => {
    const th = typeof t === "string" ? n.threads.find((x: any) => x.quote === t) : t;
    return Buffer.from(helpers.textOf(n)).subarray(0, th.start).toString().split("\n").length - 1;
  },
  /** Margin tags in a dump that overlap a neighbour on the same side. */
  overlaps: (d: any): number => {
    let n = 0;
    for (const side of ["Left", "Right"]) {
      const s = (d.slots ?? []).filter((x: any) => x.side === side).sort((a: any, b: any) => a.top - b.top);
      for (let i = 1; i < s.length; i++) if (s[i].top < s[i - 1].top + s[i - 1].h - 0.5) n++;
    }
    return n;
  },
};

function evaluate(expr: string, scope: Record<string, unknown>): { ok: boolean; error?: string } {
  try {
    const f = new Function(...Object.keys(scope), `"use strict"; return (\n${expr}\n);`);
    const v = f(...Object.values(scope));
    return v === true ? { ok: true } : { ok: false, error: `was ${JSON.stringify(v)}` };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? `${e.name}: ${e.message}` : String(e) };
  }
}

/** Check a finished run of `s` whose notebook is `dir`. */
export function check(s: Scenario, dir: string, run: { exit: number | null; timedOut: boolean; ms: number; shots: Result["shots"] }): Result {
  const log = readLog(join(dir, "app.log"));
  const dumps: Dump[] = dumpsOf(log);
  writeFileSync(join(dir, "dumps.json"), JSON.stringify(dumps, null, 1));
  const store = readStore(dir);
  const brain = readBrainLog(join(dir, "brain.log"));
  const requests = brain.filter((b) => b.dir === "recv" && b.data.op === "run").map((b) => b.data);
  const p = problems(log);
  const why: string[] = [];

  if (run.timedOut) why.push(`timed out after ${s.timeoutS}s (${s.quits ? "a step hung or a wait is too long" : "the script has no quit step"})`);
  else if (run.exit !== 0) why.push(`the app exited with ${run.exit}`);
  for (const panic of p.panics) why.push(`panic: ${panic.slice(0, 300)}`);
  for (const e of p.errors) why.push(`error logged: ${e.slice(0, 300)}`);
  for (const w of p.scriptWarnings) if (!s.allowWarn.some((a) => w.includes(a))) why.push(`script step did nothing: ${w} (add "# allow-warn: ..." if that is the point)`);
  if (dumps.length < s.dumps.length) why.push(`${s.dumps.length - dumps.length} of ${s.dumps.length} dumps never happened (first missing: "${s.dumps[dumps.length]}")`);
  for (const shot of run.shots) if (!shot.path) why.push(`shot ${shot.name}: ${shot.error}`);
  const taken = new Set(run.shots.map((x) => x.name));
  for (const name of s.shots) if (!taken.has(name.replace(/[^\w.-]+/g, "-") || "shot")) why.push(`shot ${name} was never taken`);

  const byLabel: Record<string, any> = {};
  for (const d of dumps) byLabel[d.label] = d.state;
  const scope = {
    ...helpers,
    dumps: byLabel,
    notes: store.notes,
    settings: store.settings,
    minds: store.minds,
    folders: store.folders,
    note: (title: string) => store.notes.find((n) => n.title === title),
    requests,
    tasks: requests.map((r) => r.input?.task),
    events: brain.filter((b) => b.dir === "send").map((b) => b.data),
    log,
    warnings: p.scriptWarnings,
    exit: run.exit,
  };
  const expects: ExpectResult[] = s.expects.map((e) => {
    const label = e.dump >= 0 ? s.dumps[e.dump] : null;
    if (e.dump >= 0 && !dumps[e.dump]) return { line: e.line, expr: e.expr, dump: label, ok: false, error: `dump "${label}" never happened` };
    const r = evaluate(e.expr, { d: e.dump >= 0 ? dumps[e.dump].state : undefined, ...scope });
    return { line: e.line, expr: e.expr, dump: label, ...r };
  });
  for (const e of expects) if (!e.ok) why.push(`line ${e.line}: expect ${e.expr}  ->  ${e.error}`);
  return { name: s.name, status: why.length ? "fail" : "pass", why, ms: run.ms, dir, exit: run.exit, expects, shots: run.shots, dumps: dumps.length };
}

// ----- running ------------------------------------------------------------------------

function unlock(lock: string) {
  rmSync(join(lock, "owner.json"), { force: true });
  try {
    rmdirSync(lock);
  } catch {
    // already gone
  }
}

/** One GUI suite at a time on this machine: the scenarios wait on wall-clock time,
 *  and two suites (or two agents) sharing the CPU make each other flaky. */
async function withLock<T>(enabled: boolean, say: (s: string) => void, f: () => Promise<T>): Promise<T> {
  if (!enabled) return f();
  const lock = join(outRoot(), "e2e.lock");
  mkdirSync(outRoot(), { recursive: true });
  const waitUntil = Date.now() + 15 * 60_000;
  let told = false;
  for (;;) {
    try {
      mkdirSync(lock);
      writeFileSync(join(lock, "owner.json"), JSON.stringify({ pid: process.pid, cwd: process.cwd(), since: new Date().toISOString() }));
      break;
    } catch {
      let owner: any = null;
      try {
        owner = JSON.parse(readFileSync(join(lock, "owner.json"), "utf8"));
      } catch {
        // being written right now
      }
      // A dead owner, or a lock left half-made (no owner file after 10 s), is stale.
      const age = (() => { try { return Date.now() - statSync(lock).mtimeMs; } catch { return 0; } })();
      const alive = owner?.pid ? (() => { try { process.kill(owner.pid, 0); return true; } catch { return false; } })() : age < 10_000;
      if (!alive) {
        unlock(lock);
        continue;
      }
      if (Date.now() > waitUntil) throw new Error(`gave up waiting for ${lock} (held by ${JSON.stringify(owner)}); pass --no-lock to run anyway`);
      if (!told) say(`waiting for another e2e run to finish (${JSON.stringify(owner)}); --no-lock skips this`);
      told = true;
      await sleep(1000);
    }
  }
  const release = () => unlock(lock);
  process.on("exit", release);
  for (const sig of ["SIGINT", "SIGTERM"] as const) process.on(sig, () => process.exit(130));
  try {
    return await f();
  } finally {
    release();
  }
}

export interface RunOptions {
  names: string[];
  jobs: number;
  real: boolean;
  lock: boolean;
  json: boolean;
}

/** Pick scenarios by name, prefix or tag ("all", "smoke"), plus what they start from. */
export function select(names: string[], all: Scenario[]): Scenario[] {
  const want = names.length === 0 ? ["all"] : names;
  const picked = new Map<string, Scenario>();
  for (const w of want) {
    const hits = w === "all" ? all : all.filter((s) => s.name === w || s.name.startsWith(w) || s.tags.includes(w) || s.path === w || s.path.endsWith("/" + w));
    if (hits.length === 0) throw new Error(`no scenario matches "${w}" (see: scripts/dev e2e --list)`);
    for (const s of hits) picked.set(s.name, s);
  }
  // Pull in what each one starts from.
  for (const s of [...picked.values()]) {
    let a = s.after;
    while (a && !picked.has(a)) {
      const dep = all.find((x) => x.name === a);
      if (!dep) throw new Error(`${s.name} starts after ${a}, which does not exist`);
      picked.set(dep.name, dep);
      a = dep.after;
    }
  }
  return [...picked.values()];
}

export async function runSuite(o: RunOptions): Promise<number> {
  const say = (msg: string) => (o.json ? process.stderr.write(msg + "\n") : console.log(msg));
  const all = allScenarios();
  const chosen = select(o.names, all);
  // Unknown steps and broken expects fail here, before any window opens.
  const bad = lintScenarios(chosen.map((s) => s.path));
  if (bad.length) {
    for (const b of bad) say(b);
    return 2;
  }
  const b = await build();
  if (!b.ok) {
    say("cargo build failed:\n" + b.output.split("\n").slice(-60).join("\n"));
    return 2;
  }
  const root = join(outRoot(), `e2e-${stamp()}-${process.pid}`);
  mkdirSync(root, { recursive: true });

  // Chains: a scenario and the ones that start from it run in order, one worker each.
  const byName = new Map(chosen.map((s) => [s.name, s]));
  const chains: Scenario[][] = [];
  for (const s of chosen) {
    if (s.after && byName.has(s.after)) continue;
    const chain = [s];
    for (let i = 0; i < chain.length; i++) for (const t of chosen) if (t.after === chain[i].name) chain.push(t);
    chains.push(chain);
  }

  const results: Result[] = [];
  const started = Date.now();
  say(`e2e: ${chosen.length} scenario(s), ${Math.min(o.jobs, chains.length)} at a time -> ${root}`);
  await withLock(o.lock, say, async () => {
    const queue = [...chains];
    const worker = async () => {
      for (let chain = queue.shift(); chain; chain = queue.shift()) {
        let previous: Result | null = null;
        for (const s of chain) {
          const r = await runOne(s, root, o.real, previous);
          results.push(r);
          previous = r;
          say(line(r));
          if (r.status === "fail") for (const w of r.why) say(`        ${w}`);
          if (r.status === "fail" && r.dir) say(`        dir: ${r.dir}`);
        }
      }
    };
    await Promise.all(Array.from({ length: Math.max(1, Math.min(o.jobs, chains.length)) }, worker));
  });

  const order = new Map(chosen.map((s, i) => [s.name, i]));
  results.sort((a, b) => order.get(a.name)! - order.get(b.name)!);
  const count = (st: string) => results.filter((r) => r.status === st).length;
  const summary = { root, ms: Date.now() - started, passed: count("pass"), failed: count("fail"), skipped: count("skip"), results };
  writeFileSync(join(root, "results.json"), JSON.stringify(summary, null, 1));
  if (o.json) console.log(JSON.stringify(summary, null, 1));
  else say(`${summary.passed} passed, ${summary.failed} failed, ${summary.skipped} skipped in ${(summary.ms / 1000).toFixed(0)}s. Results: ${join(root, "results.json")}`);
  return summary.failed ? 1 : 0;
}

function line(r: Result): string {
  const tag = r.status === "pass" ? "PASS" : r.status === "fail" ? "FAIL" : "SKIP";
  const extra = r.status === "skip" ? r.why.join("; ") : `${r.expects.length} expects${r.shots.length ? `, ${r.shots.length} shots` : ""}`;
  return `  ${tag}  ${r.name.padEnd(26)} ${(r.ms / 1000).toFixed(1).padStart(5)}s  ${extra}`;
}

async function runOne(s: Scenario, root: string, real: boolean, previous: Result | null): Promise<Result> {
  const skip = (why: string): Result => ({ name: s.name, status: "skip", why: [why], ms: 0, dir: null, exit: null, expects: [], shots: [], dumps: 0 });
  if (s.real && !real) return skip("needs the real brain (model calls on the Claude login); pass --real");
  if (s.after && (!previous || previous.name !== s.after || !previous.dir)) return skip(`starts after ${s.after}, which did not run`);
  const dir = join(root, s.name);
  try {
    prepareNotebook(dir, { fixture: s.fixture, notebook: s.after ? previous!.dir! : undefined });
  } catch (e) {
    return { ...skip(""), status: "fail", why: [String(e instanceof Error ? e.message : e)], dir };
  }
  cpSync(s.path, join(dir, "script.txt"));
  const run = await launch({ script: s.path, dir, env: s.env, real: s.real && real, timeoutMs: s.timeoutS * 1000 });
  return check(s, dir, run);
}

export function listScenarios(json: boolean) {
  const all = allScenarios();
  if (json) {
    console.log(JSON.stringify(all.map(({ path, ...s }) => ({ ...s, path })), null, 1));
    return;
  }
  for (const s of all) {
    const bits = [s.fixture && `fixture=${s.fixture}`, s.after && `after=${s.after}`, Object.keys(s.env).length && `env=${Object.entries(s.env).map(([k, v]) => `${k}=${v}`).join(",")}`, s.real && "real", s.tags.length && `tags=${s.tags.join(",")}`, `${s.expects.length} expects`]
      .filter(Boolean)
      .join(" ");
    console.log(`${s.name.padEnd(26)} ${bits}${s.about ? `  # ${s.about.slice(0, 90)}` : ""}`);
  }
}

export { existsSync };

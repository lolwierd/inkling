// A fake brain for deterministic end-to-end runs: it speaks the same JSON-lines
// protocol as src/main.ts (ready; run -> delta/done/error per task; cancel;
// status; shutdown) but answers from canned templates, so every path of the app
// can be driven without a model call.
//
//   INKLING_BRAIN_ENTRY=$PWD/brain/test/e2e/fake-brain.ts
//
// Behaviour comes from directives written into the request's own text fields
// (the paragraph, the selection, the question, the reply, the chat message), so
// a script picks the path by what it types:
//
//   [[silent]]            read: speak=false            [[speak]] force speak
//   [[quote:TEXT]]        quote TEXT, snapped onto the paragraph like the real brain
//   [[rawquote:TEXT]]     quote TEXT verbatim (may be missing from the paragraph)
//   [[correction]]        kind=correction              [[sketch]] sketch=true
//   [[resolved]]          reply: resolved=true         [[redraw]] reply: redraw=true
//   [[error:KIND]]        fail with KIND (auth, rate_limit, overloaded, cancelled,
//                         bad_output, other); [[errmsg:TEXT]] sets the message
//   [[hang]]              never answer (a cancel still ends it, as "cancelled")
//   [[exit]]              exit the process as soon as the request arrives
//   [[exitmid]]           stream two deltas (or wait the delay), then exit
//   [[exitafter]]         answer, then exit right away (the next request finds it gone)
//   [[delay:MS]]          answer after MS instead of the default
//   [[badsvg]]            draw: an <svg> that does not parse
//   [[nosvg]]             draw: what the real brain sends when no <svg> came back
//   [[tallsvg]]           draw: a valid but absurdly tall viewBox (10 x 2000)
//   [[empty]]             empty text                   [[null]] done with data null
//   [[learn:TEXT]]        learn: update=true, aboutMe=TEXT;  [[nolearn]] update=false
//   [[again]]             read/probe: speak even on words the writer let stand (below)
//
// Like the real brain, a read or probe never raises a point the writer let stand: when
// the quote it would use (or a probe's whole selection) is exactly the quote of one of
// `ctx.letStand`, it answers {speak: false, repeats: n}, n counting from 1.
//
//   With [[correction]], a check for it, which goes through the real brain's checks
//   (src/working.ts): a sum that does not add up, or words not in the note, turn the
//   correction into its [[question:...]] (or silence, or bad_output for a probe):
//   [[sum:5:30 × 10 = 55:00]]   a line of the working (repeat for more lines)
//   [[given:5:30@5:30 per km]]  a number the sum takes from the writer, and their words
//   [[words:PASSAGE]]           a passage from elsewhere in the note (repeatable)
//   [[question:TEXT]]           the question to ask instead;  [[nocheck]] none of these
//   A bare [[correction]] is passed through unchecked, as before.
//
// Prefix a directive with a task to scope it: [[draw>badsvg]], [[reply>resolved]],
// [[chat>error:overloaded]]. Directives are only read from the request's own
// fields (never the whole note), so they cannot leak between requests.
//
// Environment:
//   FAKE_DELAY_MS=200            default delay before answering
//   FAKE_DELAY_<TASK>_MS         per task, e.g. FAKE_DELAY_DRAW_MS=1500
//   FAKE_DELTA_MS=60             gap between streamed deltas
//   FAKE_READY_DELAY_MS=0        delay before the ready event; FAKE_NO_READY=1 never sends it
//   FAKE_EXIT_AFTER=N            exit after the Nth run request arrives
//   FAKE_READ_DEFAULT=speak      read answers speak unless [[silent]]; "silent" flips it
//   FAKE_MODELS=none             status reports no model list (default: the list the real
//                                SDK gave on 6 Nov 2026, test/fixtures/supported-models.json)
//
// Every request and every non-delta event is logged to stderr (the app sends it
// to <data dir>/brain.log) as `[fake] recv {...}` / `[fake] send {...}`.

import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";
import { withWorking } from "../../src/working.ts";

type Any = Record<string, any>;

const env = process.env;
const num = (v: string | undefined, d: number) => (v !== undefined && v !== "" && !Number.isNaN(Number(v)) ? Number(v) : d);
const DEFAULT_DELAY = num(env.FAKE_DELAY_MS, 200);
const DELTA_MS = num(env.FAKE_DELTA_MS, 60);
const EXIT_AFTER = num(env.FAKE_EXIT_AFTER, 0);
const READ_DEFAULT = env.FAKE_READ_DEFAULT === "silent" ? "silent" : "speak";

let runs = 0;
const live = new Map<string, { timers: ReturnType<typeof setTimeout>[]; done: boolean }>();

const T0 = Date.now();
function log(...args: unknown[]) {
  const t = ((Date.now() - T0) / 1000).toFixed(3).padStart(8);
  process.stderr.write(`[fake ${t}] ${args.map((a) => (typeof a === "string" ? a : JSON.stringify(a))).join(" ")}\n`);
}

function emit(e: Any) {
  if (e.ev !== "delta") log("send", e);
  process.stdout.write(JSON.stringify(e) + "\n");
}

interface Directive {
  task: string | null;
  name: string;
  arg: string | null;
}

function directives(text: string): Directive[] {
  const out: Directive[] = [];
  const re = /\[\[(?:([a-z_]+)>)?([a-z_]+)(?::([^\]]*))?\]\]/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) out.push({ task: m[1] ?? null, name: m[2], arg: m[3] ?? null });
  return out;
}

/** The request's own text, where its directives live. */
function ownText(input: Any): string {
  const parts: unknown[] = [];
  switch (input.task) {
    case "read":
    case "probe":
    case "answer":
    case "sketch":
      parts.push(input.target);
      break;
    case "ask":
      parts.push(input.target, input.question);
      break;
    case "draw":
      parts.push(input.quote, input.thought, input.request);
      break;
    case "reply":
      parts.push(input.message, input.quote);
      break;
    case "chat":
    case "learn":
      parts.push(input.message);
      break;
  }
  return parts.filter((p) => typeof p === "string").join("\n");
}

/** Strip directives so canned answers read cleanly. */
function clean(text: string): string {
  return text.replace(/\[\[[^\]]*\]\]/g, "").replace(/\s+/g, " ").trim();
}

// Same snapping as src/main.ts, so [[quote:...]] reaches the app the way a real
// answer would. Directives in the paragraph are masked first so a quote never
// matches the directive that asked for it.
function snapQuote(quote: string, source: string): string {
  const masked = source.replace(/\[\[[^\]]*\]\]/g, (m) => "\u0001".repeat(m.length));
  const q = quote.trim().replace(/^[«"“']+|[»"”']+$/g, "");
  if (!q) return source;
  const i = masked.indexOf(q);
  if (i >= 0) return source.slice(i, i + q.length);
  const idx = masked.toLowerCase().indexOf(q.toLowerCase());
  if (idx >= 0) return source.slice(idx, idx + q.length);
  const norm = (s: string) => s.replace(/\s+/g, " ");
  const j = norm(masked).toLowerCase().indexOf(norm(q).toLowerCase());
  if (j >= 0) return norm(source).slice(j, j + norm(q).length);
  return source;
}

const GOOD_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 200">
<g data-step="1" data-caption="First, the box."><path d="M40 40 L280 40 L280 160 L40 160 Z" stroke="ink" stroke-width="1.2" fill="none"/></g>
<g data-step="2" data-caption="Then the arrow."><path d="M60 100 Q160 60 260 100" stroke="graphite" stroke-width="1.1" fill="none"/><text x="160" y="185" class="voice" font-size="14" fill="ink" text-anchor="middle">done</text></g>
</svg>`;
const TALL_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 2000"><g data-step="1" data-caption="tall"><path d="M1 1 L9 1999" stroke="ink"/></g></svg>`;
const BAD_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 200"><g data-step="1" data-caption="broken"><path d="M0 0 L10 10" stroke="ink"></g></svg>`;

/** The first `n` words of `s`, as an exact substring of it. */
function firstWords(s: string, n: number): string {
  const m = s.match(new RegExp(`^\\s*\\S+(?:\\s+\\S+){0,${n - 1}}`));
  return m ? m[0].trim() : s;
}

function plan(id: string, input: Any) {
  const task: string = input.task;
  const ds = directives(ownText(input)).filter((d) => d.task === null || d.task === task);
  const has = (name: string) => ds.some((d) => d.name === name);
  const arg = (name: string) => ds.find((d) => d.name === name)?.arg ?? null;
  const delayDirective = arg("delay");
  const delay = delayDirective !== null ? num(delayDirective, DEFAULT_DELAY) : num(env[`FAKE_DELAY_${task.toUpperCase()}_MS`], DEFAULT_DELAY);
  return { task, ds, has, arg, delay };
}

function answerFor(id: string, input: Any, p: ReturnType<typeof plan>): { data?: unknown; stream?: string; error?: [string, string] } {
  const { task, has, arg } = p;
  const errKind = arg("error");
  if (errKind !== null) return { error: [errKind || "other", arg("errmsg") ?? `fake ${errKind || "other"} error`] };
  if (has("null")) return { data: null };
  const target: string = typeof input.target === "string" ? input.target : "";
  const shown = clean(target);
  switch (task) {
    case "read":
    case "probe": {
      const silent = has("silent") || (READ_DEFAULT === "silent" && !has("speak"));
      if (task === "read" && silent) return { data: { speak: false } };
      const raw = arg("rawquote");
      const q = arg("quote");
      const quote = raw !== null ? raw : snapQuote(q ?? firstWords(target, 3), target);
      const stood: Any[] = Array.isArray(input.ctx?.letStand) ? input.ctx.letStand : [];
      const n = stood.findIndex((s) => s.quote === quote || (task === "probe" && s.quote === shown));
      if (n >= 0 && !has("again")) return { data: { speak: false, repeats: n + 1 } };
      const text = has("empty") ? "" : `[${id}] ${task === "read" ? "Read" : "Probe"}: what about “${clean(quote)}”?`;
      if (task === "read" && !text.trim()) return { data: { speak: false } };
      const thought = { speak: true as const, kind: has("correction") ? ("correction" as const) : ("question" as const), quote, text, sketch: has("sketch") };
      const checked = ["sum", "given", "words", "question", "nocheck"].some(has);
      if (thought.kind !== "correction" || !checked) return { data: thought };
      // The check goes through the real brain's rules, against the text without directives.
      const all = (name: string) => p.ds.filter((d) => d.name === name).map((d) => d.arg ?? "");
      const check = {
        sum: all("sum").map((l) => {
          const at = l.lastIndexOf("=");
          return { sum: at < 0 ? l : l.slice(0, at).trim(), is: at < 0 ? "" : l.slice(at + 1).trim() };
        }),
        givens: all("given").map((g) => ({ value: g.split("@")[0].trim(), words: g.split("@").slice(1).join("@").trim() })),
        words: all("words"),
      };
      const bare = (v: unknown) => (typeof v === "string" ? v.replace(/\[\[[^\]]*\]\]/g, "") : "");
      const ctx = input.ctx ?? {};
      const writer = { title: bare(ctx.noteTitle), note: bare(ctx.noteText), brief: ctx.brief ? bare(ctx.brief) : null, target: bare(target), quote: bare(quote) };
      const settled = withWorking(thought, { check: has("nocheck") ? undefined : check, question: arg("question") ?? "" }, writer, (why) => log(why));
      if (settled) return { data: settled };
      return task === "read" ? { data: { speak: false } } : { error: ["bad_output", "a correction came without a check or a question"] };
    }
    case "sketch": {
      const raw = arg("rawquote");
      const quote = raw !== null ? raw : snapQuote(arg("quote") ?? "", target);
      return { data: { quote, text: has("empty") ? "" : `[${id}] Picture it: ${shown.slice(0, 60)}` } };
    }
    case "draw":
      if (has("nosvg")) return { error: ["bad_output", "the drawing did not come back as SVG"] };
      return { data: { svg: has("badsvg") ? BAD_SVG : has("tallsvg") ? TALL_SVG : GOOD_SVG } };
    case "reply":
      return {
        data: {
          text: has("empty") ? "" : `[${id}] Reply to “${clean(input.message ?? "").slice(0, 50)}” (history ${Array.isArray(input.messages) ? input.messages.length : "?"})`,
          resolved: has("resolved"),
          redraw: has("redraw"),
        },
      };
    case "ask":
      return { stream: has("empty") ? "" : `[${id}] Answer to “${clean(input.question ?? "")}” about “${shown.slice(0, 40)}”.` };
    case "answer":
      return { stream: has("empty") ? "" : `[${id}] You called me: “${shown.slice(0, 50)}”.` };
    case "chat": {
      const near = input.near === null || input.near === undefined ? "null" : `“${clean(String(input.near)).slice(0, 30)}”`;
      const hist = Array.isArray(input.history) ? input.history.length : "?";
      return { stream: has("empty") ? "" : `[${id}] Chat reply to “${clean(input.message ?? "")}” (history ${hist}, near ${near}).` };
    }
    case "learn": {
      if (has("nolearn")) return { data: { update: false, aboutMe: input.aboutMe ?? "" } };
      const learned = arg("learn") ?? clean(input.message ?? "").slice(0, 80);
      const about = [input.aboutMe ?? "", learned].filter((s: string) => s && s.trim()).join(" ");
      return { data: { update: true, aboutMe: about.slice(0, 700) } };
    }
  }
  return { error: ["other", `fake brain does not know task ${task}`] };
}

function chunks(text: string, n: number): string[] {
  if (!text) return [];
  const size = Math.max(1, Math.ceil(text.length / n));
  const out: string[] = [];
  for (let i = 0; i < text.length; i += size) out.push(text.slice(i, i + size));
  return out;
}

function run(id: string, input: Any) {
  runs += 1;
  const p = plan(id, input);
  const state = { timers: [] as ReturnType<typeof setTimeout>[], done: false };
  live.set(id, state);
  const later = (ms: number, f: () => void) => state.timers.push(setTimeout(f, ms));
  const finish = (e: Any) => {
    if (state.done) return;
    state.done = true;
    live.delete(id);
    emit(e);
    if (p.has("exitafter")) {
      log(`exiting after answering ${id}`);
      process.exit(5);
    }
  };

  if (p.has("exit") || (EXIT_AFTER > 0 && runs >= EXIT_AFTER)) {
    log(`exiting on request ${id} (${p.task})`);
    process.exit(3);
  }
  if (p.has("hang")) {
    log(`hanging on ${id} (${p.task})`);
    return;
  }
  const answer = answerFor(id, input, p);
  if (answer.error) {
    later(p.delay, () => finish({ ev: "error", id, kind: answer.error![0], message: answer.error![1] }));
    return;
  }
  const usage = { ms: p.delay, model: "fake" };
  if (answer.stream !== undefined) {
    const parts = chunks(answer.stream, 5);
    let t = p.delay;
    parts.forEach((part, i) => {
      later(t, () => {
        if (!state.done) emit({ ev: "delta", id, text: part });
        if (p.has("exitmid") && i === 1) {
          log(`exiting mid-stream on ${id}`);
          process.exit(4);
        }
      });
      t += DELTA_MS;
    });
    later(t, () => finish({ ev: "done", id, data: { text: answer.stream!.trim() }, usage }));
    return;
  }
  if (p.has("exitmid")) {
    later(p.delay, () => {
      log(`exiting mid-request on ${id}`);
      process.exit(4);
    });
    return;
  }
  later(p.delay, () => finish({ ev: "done", id, data: answer.data, usage }));
}

function cancel(id: string) {
  const state = live.get(id);
  if (!state) return;
  for (const t of state.timers) clearTimeout(t);
  // Like the real brain: an aborted request still ends with an error event.
  if (!state.done) {
    state.done = true;
    live.delete(id);
    emit({ ev: "error", id, kind: "cancelled", message: "cancelled" });
  }
}

function start() {
  if (env.FAKE_NO_READY !== "1") {
    setTimeout(() => emit({ ev: "ready", provider: "fake", version: "test" }), num(env.FAKE_READY_DELAY_MS, 0));
  }
  const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
  rl.on("line", (line) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    let req: Any;
    try {
      req = JSON.parse(trimmed);
    } catch {
      log("ignoring a line that is not JSON");
      return;
    }
    log("recv", req);
    switch (req.op) {
      case "run":
        run(req.id, req.input ?? {});
        break;
      case "cancel":
        cancel(req.id);
        break;
      case "status": {
        const models = env.FAKE_MODELS === "none" ? null : JSON.parse(readFileSync(new URL("../fixtures/supported-models.json", import.meta.url), "utf8"));
        emit({ ev: "status", id: req.id, data: { account: { email: "fake@example.test", subscriptionType: "fake" }, models } });
        break;
      }
      case "shutdown":
        process.exit(0);
    }
  });
  rl.on("close", () => process.exit(0));
  log(`started pid ${process.pid}`);
}

start();

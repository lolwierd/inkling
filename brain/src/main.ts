// Inkling brain: a sidecar the app spawns once. It reads requests as JSON lines on
// stdin and writes events as JSON lines on stdout. Each request runs concurrently
// and can be cancelled by id. Every run request with an id ends in exactly one
// `done` or `error` event; after a cancel it is never `done`.

import { createInterface } from "node:readline";
import { accountStatus, ProviderError, runStructured, runText, type LimitInfo } from "./claude.ts";
import { cleanSvg, clip, snapQuote } from "./output.ts";
import {
  answerPrompt,
  askPrompt,
  chatPrompt,
  drawPrompt,
  readPrompt,
  replyPrompt,
  probePrompt,
  systemPrompt,
  sketchPrompt,
  REPLY_SCHEMA,
  PROBE_SCHEMA,
  LEARN_SCHEMA,
  LEARN_SYSTEM,
  learnPrompt,
  THOUGHT_SCHEMA,
  SKETCH_SCHEMA,
} from "./prompts.ts";
import type { Context, Event, Frequency, LetStandPoint, RunSettings, Task, ThreadMessage, Usage, Voice } from "./protocol.ts";
import { withWorking } from "./working.ts";

const VERSION = "0.1.0";
const running = new Map<string, AbortController>();

function emit(e: Event) {
  process.stdout.write(JSON.stringify(e) + "\n");
}

function log(...args: unknown[]) {
  process.stderr.write(`[brain] ${args.map(String).join(" ")}\n`);
}

const onLimit = (l: LimitInfo) => emit({ ev: "limit", ...l });

// ----- reading requests -------------------------------------------------------

type Json = Record<string, unknown>;

function badRequest(message: string): never {
  throw new ProviderError("other", `bad request: ${message}`);
}

function isObject(v: unknown): v is Json {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function text(v: unknown): string {
  return typeof v === "string" ? v : "";
}

function optionalText(v: unknown): string | null {
  return typeof v === "string" && v.trim() ? v : null;
}

function required(v: unknown, field: string): string {
  if (typeof v !== "string" || !v.trim()) badRequest(`${field} is missing`);
  return v;
}

function messages(v: unknown): ThreadMessage[] {
  if (!Array.isArray(v)) return [];
  return v.filter(isObject).map((m) => ({ role: m.role === "you" ? "you" : "mind", text: text(m.text) }));
}

/** At most this many points the writer let stand reach a prompt (the app sends 12). */
const LET_STAND_MAX = 20;

/** The points the writer let stand, as the app sends them; an entry without its words or
 * its point is left out, and long fields are cut. */
function letStand(v: unknown): LetStandPoint[] {
  if (!Array.isArray(v)) return [];
  return v
    .filter(isObject)
    .map((p): LetStandPoint => {
      // each on one line, as the prompt lists them
      const line = (v: unknown, max: number) => clip(text(v).replace(/\s+/g, " ").trim(), max);
      return {
        quote: line(p.quote, 160),
        point: line(p.point, 240),
        kind: p.kind === "correction" ? "correction" : "question",
        mind: line(p.mind, 60) || "Inkling",
        reason: line(p.reason, 200) || null,
      };
    })
    .filter((p) => p.quote && p.point)
    .slice(0, LET_STAND_MAX);
}

/** The point a read or probe says it makes again, if it names one of the `count` the
 * writer let stand; anything else is 0. */
function repeated(v: unknown, count: number): number {
  return typeof v === "number" && Number.isInteger(v) && v >= 1 && v <= count ? v : 0;
}

const VOICES = new Set<Voice>(["plain", "warm", "blunt", "riddle"]);
const FREQUENCIES = new Set<Frequency>(["rarely", "when_it_matters", "often"]);
const EFFORTS = new Set(["low", "medium", "high", "xhigh", "max"]);

/** The note context, with what an older or sloppier client left out filled in. */
function context(v: unknown): Context {
  if (!isObject(v)) badRequest("ctx is missing");
  const mind = isObject(v.mind) ? v.mind : {};
  return {
    noteTitle: text(v.noteTitle),
    noteText: text(v.noteText),
    brief: optionalText(v.brief),
    aboutMe: optionalText(v.aboutMe),
    threads: Array.isArray(v.threads)
      ? v.threads.filter(isObject).map((t) => ({ quote: text(t.quote), messages: messages(t.messages), resolved: t.resolved === true }))
      : [],
    now: text(v.now) || new Date().toDateString(),
    letStand: letStand(v.letStand),
    mind: {
      name: text(mind.name).trim() || "Inkling",
      description: text(mind.description),
      howItThinks: text(mind.howItThinks),
      voice: VOICES.has(mind.voice as Voice) ? (mind.voice as Voice) : "plain",
      frequency: FREQUENCIES.has(mind.frequency as Frequency) ? (mind.frequency as Frequency) : "when_it_matters",
    },
  };
}

function task(v: unknown): Task {
  if (!isObject(v)) badRequest("input is missing");
  switch (v.task) {
    case "read":
    case "probe":
    case "answer":
    case "sketch":
      return { task: v.task, ctx: context(v.ctx), target: required(v.target, "target") };
    case "ask":
      return { task: "ask", ctx: context(v.ctx), target: required(v.target, "target"), question: required(v.question, "question") };
    case "draw":
      return { task: "draw", ctx: context(v.ctx), quote: text(v.quote), thought: text(v.thought), request: optionalText(v.request) ?? undefined };
    case "reply":
      return { task: "reply", ctx: context(v.ctx), quote: text(v.quote), messages: messages(v.messages), message: required(v.message, "message") };
    case "chat":
      return { task: "chat", ctx: context(v.ctx), history: messages(v.history), message: required(v.message, "message"), near: optionalText(v.near) };
    case "learn":
      return { task: "learn", aboutMe: optionalText(v.aboutMe), message: required(v.message, "message") };
    default:
      return badRequest(`unknown task ${JSON.stringify(v.task ?? null)}`);
  }
}

function settings(v: unknown): RunSettings {
  const s = isObject(v) ? v : {};
  return {
    model: typeof s.model === "string" && s.model.trim() ? s.model : undefined,
    effort: EFFORTS.has(s.effort as string) ? (s.effort as RunSettings["effort"]) : undefined,
  };
}

// ----- checking what comes back -------------------------------------------------

function structured(data: unknown): Json {
  if (!isObject(data)) throw new ProviderError("bad_output", "the model's answer was not a JSON object");
  return data;
}

function nonEmpty(v: unknown, what: string): string {
  const s = text(v).trim();
  if (!s) throw new ProviderError("bad_output", `the model returned ${what}`);
  return s;
}

// ----- running tasks ------------------------------------------------------------

async function run(id: string, settings: RunSettings, input: Task, signal: AbortController): Promise<{ data: unknown; usage: Usage }> {
  if (input.task === "learn") {
    const { data, usage } = await runStructured<unknown>(LEARN_SYSTEM, learnPrompt(input.aboutMe, input.message), LEARN_SCHEMA, settings, signal, onLimit);
    const d = structured(data);
    const aboutMe = d.update === true ? clip(text(d.aboutMe).trim(), 700) : "";
    return { data: { update: aboutMe.length > 0, aboutMe }, usage };
  }
  const sys = systemPrompt(input.ctx);
  // Nothing streams to the app once a request is cancelled.
  const delta = (t: string) => {
    if (t && !signal.signal.aborted) emit({ ev: "delta", id, text: t });
  };

  switch (input.task) {
    case "read":
    case "probe": {
      const [prompt, schema] =
        input.task === "read" ? [readPrompt(input.ctx, input.target), THOUGHT_SCHEMA] : [probePrompt(input.ctx, input.target), PROBE_SCHEMA];
      const { data, usage } = await runStructured<unknown>(sys, prompt, schema, settings, signal, onLimit);
      const d = structured(data);
      // A point the writer let stand is not raised again, in any words: the model says
      // which one its note repeats, and then the note is held back.
      const repeats = repeated(d.repeats, input.ctx.letStand.length);
      if (repeats > 0) {
        log(`${input.task} ${id} held back: it makes let-stand point ${repeats} again`);
        return { data: { speak: false, repeats }, usage };
      }
      // Read may stay silent; a probe was asked for, so it always says something.
      if (input.task === "read" && !(d.speak === true && text(d.text).trim())) return { data: { speak: false }, usage };
      const quote = snapQuote(text(d.quote), input.target);
      const thought = withWorking(
        { speak: true, kind: d.kind === "correction" ? "correction" : "question", quote, text: nonEmpty(d.text, "an empty note"), sketch: d.sketch === true },
        d,
        { title: input.ctx.noteTitle, note: input.ctx.noteText, brief: input.ctx.brief, target: input.target, quote },
        log,
      );
      // A correction with neither a check nor a question to fall back on is never shown.
      if (!thought && input.task === "read") return { data: { speak: false }, usage };
      if (!thought) throw new ProviderError("bad_output", "a correction came without a check or a question");
      return { data: thought, usage };
    }
    case "sketch": {
      const { data, usage } = await runStructured<unknown>(sys, sketchPrompt(input.ctx, input.target), SKETCH_SCHEMA, settings, signal, onLimit);
      const d = structured(data);
      return { data: { quote: snapQuote(text(d.quote), input.target), text: nonEmpty(d.text, "no idea to draw") }, usage };
    }
    case "draw": {
      const { text: out, usage } = await runText(sys, drawPrompt(input.ctx, input.quote, input.thought, input.request), settings, signal, undefined, onLimit);
      return { data: { svg: cleanSvg(out) }, usage };
    }
    case "reply": {
      const { data, usage } = await runStructured<unknown>(sys, replyPrompt(input.ctx, input.quote, input.messages, input.message), REPLY_SCHEMA, settings, signal, onLimit);
      const d = structured(data);
      return { data: { text: nonEmpty(d.text, "an empty reply"), resolved: d.resolved === true, redraw: d.redraw === true }, usage };
    }
    case "ask": {
      const { text: out, usage } = await runText(sys, askPrompt(input.ctx, input.target, input.question), settings, signal, delta, onLimit);
      return { data: { text: out }, usage };
    }
    case "answer": {
      const { text: out, usage } = await runText(sys, answerPrompt(input.ctx, input.target), settings, signal, delta, onLimit);
      return { data: { text: out }, usage };
    }
    case "chat": {
      const { text: out, usage } = await runText(sys, chatPrompt(input.ctx, input.history, input.message, input.near), settings, signal, delta, onLimit);
      return { data: { text: out }, usage };
    }
  }
}

async function handleRun(req: Json) {
  const id = req.id;
  if (typeof id !== "string" || !id) {
    log("ignoring a run request without an id");
    return;
  }
  if (running.has(id)) {
    log(`ignoring a second run request for ${id}, which is still running`);
    return;
  }
  const signal = new AbortController();
  running.set(id, signal);
  let name = "?";
  try {
    const input = task(req.input);
    name = input.task;
    const { data, usage } = await run(id, settings(req.settings), input, signal);
    if (signal.signal.aborted) throw new ProviderError("cancelled", "cancelled");
    emit({ ev: "done", id, data, usage });
  } catch (e) {
    const err = e instanceof ProviderError ? e : new ProviderError("other", e instanceof Error ? e.message : String(e));
    if (signal.signal.aborted) {
      err.kind = "cancelled";
      err.message = "cancelled";
    }
    log(`task ${name} ${id} failed (${err.kind}): ${err.message}`);
    emit({ ev: "error", id, kind: err.kind, message: err.message });
  } finally {
    if (running.get(id) === signal) running.delete(id);
  }
}

async function handleStatus(id: string) {
  try {
    emit({ ev: "status", id, data: await accountStatus() });
  } catch (e) {
    const err = e instanceof ProviderError ? e : new ProviderError("other", e instanceof Error ? e.message : String(e));
    emit({ ev: "error", id, kind: err.kind, message: err.message });
  }
}

function quit() {
  for (const s of running.values()) s.abort();
  process.exit(0);
}

function handle(req: unknown) {
  if (!isObject(req)) {
    log("ignoring a line that is not a request object");
    return;
  }
  switch (req.op) {
    case "run":
      void handleRun(req);
      return;
    case "cancel":
      if (typeof req.id === "string") running.get(req.id)?.abort();
      return;
    case "status":
      if (typeof req.id === "string" && req.id) void handleStatus(req.id);
      else log("ignoring a status request without an id");
      return;
    case "shutdown":
      quit();
      return;
    default:
      log(`ignoring unknown op ${JSON.stringify(req.op ?? null)}`);
  }
}

// One bad request must never take the other requests down with it.
process.on("unhandledRejection", (e) => log("unhandled rejection:", e instanceof Error ? e.stack : e));
process.on("uncaughtException", (e) => log("uncaught exception:", e.stack ?? e));
// The app went away: nobody is left to answer.
process.stdout.on("error", () => process.exit(0));

emit({ ev: "ready", provider: "claude", version: VERSION });

const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let req: unknown;
  try {
    req = JSON.parse(trimmed);
  } catch {
    log("ignoring a line that is not JSON");
    return;
  }
  handle(req);
});
rl.on("close", quit);

// Inkling brain: a sidecar the app spawns once. It reads requests as JSON lines on
// stdin and writes events as JSON lines on stdout. Each request runs concurrently
// and can be cancelled by id. Every run request with an id ends in exactly one
// `done` or `error` event; after a cancel it is never `done`.

import { createInterface } from "node:readline";
import { accountStatus, ProviderError, runStructured, runText, WebSpent, type LimitInfo } from "./claude.ts";
import { cleanSvg, clip, findQuote, pickAnswers, snapQuote } from "./output.ts";
import {
  answerPrompt,
  askPrompt,
  chatPrompt,
  drawPrompt,
  readPrompt,
  replyPrompt,
  probePrompt,
  relatePrompt,
  systemPrompt,
  sketchPrompt,
  throughSystemPrompt,
  REPLY_SCHEMA,
  PROBE_SCHEMA,
  LEARN_SCHEMA,
  LEARN_SYSTEM,
  learnPrompt,
  THOUGHT_SCHEMA,
  SKETCH_SCHEMA,
  RELATE_SCHEMA,
  answersBlock,
  answersOnlyPrompt,
  openThreadNumbers,
  settlePrompt,
  SETTLE_SCHEMA,
  THOUGHT_WITH_ANSWERS_SCHEMA,
} from "./prompts.ts";
import type { Context, ElsewhereEnd, Event, Frequency, LetStandPoint, Passage, Relation, RunSettings, Task, ThreadMessage, Usage, Voice } from "./protocol.ts";
import { withWorking } from "./working.ts";
import { keepFoundLinks, verifySource, WEB_TASKS, type Found } from "./web.ts";

const VERSION = "0.1.2";
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

function strings(v: unknown): string[] {
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string" && x.trim() !== "") : [];
}

function messages(v: unknown): ThreadMessage[] {
  if (!Array.isArray(v)) return [];
  return v.filter(isObject).map((m) => {
    const out: ThreadMessage = { role: m.role === "you" ? "you" : "mind", text: text(m.text) };
    const mind = optionalText(m.mind);
    if (mind && out.role === "mind") out.mind = mind.trim();
    return out;
  });
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
      ? v.threads.filter(isObject).map((t) => ({
          quote: text(t.quote),
          messages: messages(t.messages),
          resolved: t.resolved === true,
          elsewhere: isObject(t.elsewhere) && text(t.elsewhere.quote).trim() ? { noteTitle: text(t.elsewhere.noteTitle), quote: text(t.elsewhere.quote) } : null,
          own: t.own === true,
          id: optionalText(t.id) ?? undefined,
          open: typeof t.open === "boolean" ? t.open : undefined,
          notAnswers: strings(t.notAnswers),
        }))
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

const RELATIONS = new Set<Relation>(["contradicts", "depends"]);

/** The passages of a relate request; ones without words are left out. */
function passages(v: unknown): Passage[] {
  if (!Array.isArray(v)) return [];
  return v
    .filter(isObject)
    .map((p) => ({
      note: text(p.note),
      noteTitle: text(p.noteTitle),
      written: text(p.written),
      edited: text(p.edited),
      heading: optionalText(p.heading),
      text: text(p.text),
    }))
    .filter((p) => p.text.trim());
}

/** The other end of a thread between notes on a reply, or undefined. */
function elsewhereEnd(v: unknown): ElsewhereEnd | undefined {
  if (!isObject(v) || !text(v.quote).trim()) return undefined;
  const gone = v.gone === true || typeof v.text !== "string";
  return {
    noteTitle: text(v.noteTitle),
    edited: text(v.edited),
    quote: text(v.quote),
    text: gone ? null : text(v.text),
    relation: RELATIONS.has(v.relation as Relation) ? (v.relation as Relation) : "contradicts",
    gone,
  };
}

function task(v: unknown): Task {
  if (!isObject(v)) badRequest("input is missing");
  switch (v.task) {
    case "read":
      return { task: "read", ctx: context(v.ctx), target: required(v.target, "target"), through: v.through === true, answersOnly: v.answersOnly === true };
    case "probe":
    case "answer":
    case "sketch":
      return { task: v.task, ctx: context(v.ctx), target: required(v.target, "target") };
    case "ask":
      return { task: "ask", ctx: context(v.ctx), target: required(v.target, "target"), question: required(v.question, "question"), history: messages(v.history) };
    case "draw":
      return { task: "draw", ctx: context(v.ctx), quote: text(v.quote), thought: text(v.thought), request: optionalText(v.request) ?? undefined };
    case "reply":
      return {
        task: "reply",
        ctx: context(v.ctx),
        quote: text(v.quote),
        messages: messages(v.messages),
        message: required(v.message, "message"),
        elsewhere: elsewhereEnd(v.elsewhere),
      };
    case "chat":
      return { task: "chat", ctx: context(v.ctx), history: messages(v.history), message: required(v.message, "message"), near: optionalText(v.near) };
    case "learn":
      return { task: "learn", aboutMe: optionalText(v.aboutMe), message: required(v.message, "message") };
    case "relate":
      return {
        task: "relate",
        ctx: context(v.ctx),
        target: required(v.target, "target"),
        before: Array.isArray(v.before) ? v.before.filter((b): b is string => typeof b === "string" && b.trim() !== "").slice(-2) : [],
        passages: passages(v.passages),
      };
    case "settle":
      return {
        task: "settle",
        ctx: context(v.ctx),
        quote: text(v.quote),
        messages: messages(v.messages),
        target: required(v.target, "target"),
        answer: optionalText(v.answer) ?? undefined,
        notAnswers: strings(v.notAnswers),
      };
    default:
      return badRequest(`unknown task ${JSON.stringify(v.task ?? null)}`);
  }
}

function settings(v: unknown): RunSettings {
  const s = isObject(v) ? v : {};
  return {
    model: typeof s.model === "string" && s.model.trim() ? s.model : undefined,
    effort: EFFORTS.has(s.effort as string) ? (s.effort as RunSettings["effort"]) : undefined,
    web: s.web === true,
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

/**
 * A task that may look things up runs with the web; if that runs out of turns,
 * it runs again without it (one turn, no tools), so it still ends as an answer: a question,
 * for a correction it could not back. The usage then counts both.
 */
async function run(id: string, settings: RunSettings, input: Task, signal: AbortController): Promise<{ data: unknown; usage: Usage }> {
  const web = settings.web === true && WEB_TASKS.has(input.task);
  try {
    return await runOnce(id, settings, input, signal, web);
  } catch (e) {
    if (!web || !(e instanceof WebSpent) || signal.signal.aborted) throw e;
    log(`${input.task} ${id}: ${e.message}; answering without the web`);
    const again = await runOnce(id, settings, input, signal, false);
    return { data: again.data, usage: { ...again.usage, costUsd: (again.usage.costUsd ?? 0) + e.spentUsd } };
  }
}

async function runOnce(id: string, settings: RunSettings, input: Task, signal: AbortController, web: boolean): Promise<{ data: unknown; usage: Usage }> {
  if (input.task === "relate" && input.passages.length === 0) {
    // Nothing to compare with: no model call.
    return { data: { speak: false }, usage: { ms: 0 } };
  }
  if (input.task === "learn") {
    const { data, usage } = await runStructured<unknown>(LEARN_SYSTEM, learnPrompt(input.aboutMe, input.message), LEARN_SCHEMA, settings, signal, onLimit);
    const d = structured(data);
    const aboutMe = d.update === true ? clip(text(d.aboutMe).trim(), 700) : "";
    return { data: { update: aboutMe.length > 0, aboutMe }, usage };
  }
  const sys = systemPrompt(input.ctx);
  // With the web on, a link no search returned is taken out; off, the words are as before.
  const links = (t: string, found: Found[]) => (web ? keepFoundLinks(t, found) : t);
  // Nothing streams to the app once a request is cancelled.
  const delta = (t: string) => {
    if (t && !signal.signal.aborted) emit({ ev: "delta", id, text: t });
  };

  switch (input.task) {
    case "read":
    case "probe": {
      // Reading it through, the note rides in the system prompt (`throughSystemPrompt`).
      const through = input.task === "read" && input.through === true;
      // A read in a note with open threads also says which of them the paragraph answers
      // (Answer in the note); a note that waits to be asked only asks that.
      const answering = input.task === "read" && openThreadNumbers(input.ctx).length > 0;
      if (input.task === "read" && input.answersOnly && !answering) return { data: { speak: false }, usage: { ms: 0 } };
      const [system, prompt, schema] =
        input.task === "probe"
          ? [sys, probePrompt(input.ctx, input.target, web), PROBE_SCHEMA]
          : input.answersOnly
            ? [sys, answersOnlyPrompt(input.ctx, input.target), THOUGHT_WITH_ANSWERS_SCHEMA]
            : [through ? throughSystemPrompt(input.ctx) : sys, readPrompt(input.ctx, input.target, through) + answersBlock(input.ctx), answering ? THOUGHT_WITH_ANSWERS_SCHEMA : THOUGHT_SCHEMA];
      const { data, usage, found } = await runStructured<unknown>(system, prompt, schema, settings, signal, onLimit, web);
      const d = structured(data);
      // Answer in the note: the open threads this paragraph answers, whatever else the read says.
      const answers = answering ? pickAnswers(d.answers, input.ctx.threads, input.target) : [];
      const answered = answers.length > 0 ? { answers } : {};
      // A note that waits to be asked raises nothing new.
      if (input.task === "read" && input.answersOnly) return { data: { speak: false, ...answered }, usage };
      // A point the writer let stand is not raised again, in any words: the model says
      // which one its note repeats, and then the note is held back.
      const repeats = repeated(d.repeats, input.ctx.letStand.length);
      if (repeats > 0) {
        log(`${input.task} ${id} held back: it makes let-stand point ${repeats} again`);
        return { data: { speak: false, repeats, ...answered }, usage };
      }
      // Read may stay silent; a probe was asked for, so it always says something.
      if (input.task === "read" && !(d.speak === true && text(d.text).trim())) return { data: { speak: false, ...answered }, usage };
      const quote = snapQuote(text(d.quote), input.target);
      // A page a correction cites counts only if this call's searches returned it and the
      // passage is on it.
      const cites = web && d.kind === "correction" && isObject(d.check) && isObject(d.check.source);
      const cited = cites ? await verifySource((d.check as Json).source, found) : null;
      const thought = withWorking(
        { speak: true, kind: d.kind === "correction" ? "correction" : "question", quote, text: links(nonEmpty(d.text, "an empty note"), found), sketch: d.sketch === true },
        d,
        { title: input.ctx.noteTitle, note: input.ctx.noteText, brief: input.ctx.brief, target: input.target, quote },
        log,
        cited,
      );
      // A correction with neither a check nor a question to fall back on is never shown.
      if (!thought && input.task === "read") return { data: { speak: false, ...answered }, usage };
      if (!thought) throw new ProviderError("bad_output", "a correction came without a check or a question");
      return { data: { ...thought, ...answered }, usage };
    }
    case "settle": {
      const prompt = settlePrompt(input.ctx, input.quote, input.messages, input.target, input.answer, input.notAnswers);
      const { data, usage } = await runStructured<unknown>(sys, prompt, SETTLE_SCHEMA, settings, signal, onLimit);
      const d = structured(data);
      // An answer with no words has nothing for the knot to point at: it is not one.
      const words = text(d.quote).trim();
      if (d.settled !== true || !words) return { data: { settled: false, quote: "", text: "" }, usage };
      return { data: { settled: true, quote: snapQuote(words, input.target), text: text(d.text).trim() }, usage };
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
    case "relate":
      return relate(input, sys, settings, signal);
    case "reply": {
      const { data, usage, found } = await runStructured<unknown>(sys, replyPrompt(input.ctx, input.quote, input.messages, input.message, input.elsewhere, web), REPLY_SCHEMA, settings, signal, onLimit, web);
      const d = structured(data);
      return { data: { text: links(nonEmpty(d.text, "an empty reply"), found), resolved: d.resolved === true, redraw: d.redraw === true }, usage };
    }
    case "ask": {
      const { text: out, usage, found } = await runText(sys, askPrompt(input.ctx, input.target, input.question, web, input.history), settings, signal, delta, onLimit, web);
      return { data: { text: links(out, found) }, usage };
    }
    case "answer": {
      const { text: out, usage, found } = await runText(sys, answerPrompt(input.ctx, input.target, web), settings, signal, delta, onLimit, web);
      return { data: { text: links(out, found) }, usage };
    }
    case "chat": {
      const { text: out, usage, found } = await runText(sys, chatPrompt(input.ctx, input.history, input.message, input.near, web), settings, signal, delta, onLimit, web);
      return { data: { text: links(out, found) }, usage };
    }
  }
}

/**
 * Threads between notes: the paragraph just read against passages from other notes. It
 * speaks only when the model says both are about the same thing and disagree, names a
 * passage that was sent, and quotes words that are really in it; the far quote never falls
 * back to the whole passage, so a quote the model made up is silence, not a thread.
 */
async function relate(
  input: Extract<Task, { task: "relate" }>,
  sys: string,
  settings: RunSettings,
  signal: AbortController,
): Promise<{ data: unknown; usage: Usage }> {
  const prompt = relatePrompt(input.ctx, input.target, input.before, input.passages);
  const { data, usage } = await runStructured<unknown>(sys, prompt, RELATE_SCHEMA, settings, signal, onLimit);
  const d = structured(data);
  // A point the writer let stand is not raised again here either.
  const repeats = repeated(d.repeats, input.ctx.letStand.length);
  if (repeats > 0) {
    log(`relate held back: it makes let-stand point ${repeats} again`);
    return { data: { speak: false, repeats }, usage };
  }
  const n = typeof d.passage === "number" ? d.passage : Number.NaN;
  const relation = d.relation === "contradicts" || d.relation === "depends" ? d.relation : null;
  const say = text(d.text).trim();
  if (d.same_subject !== true || !relation || !Number.isInteger(n) || n < 1 || n > input.passages.length || !say) {
    return { data: { speak: false }, usage };
  }
  const passage = input.passages[n - 1];
  const other = findQuote(text(d.other), passage.text);
  if (other === null) {
    log(`relate: the other words ${JSON.stringify(text(d.other).slice(0, 80))} are not in passage ${n}; staying silent`);
    return { data: { speak: false }, usage };
  }
  return {
    data: { speak: true, passage: n - 1, note: passage.note, quote: snapQuote(text(d.quote), input.target), other, relation, text: say },
    usage,
  };
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

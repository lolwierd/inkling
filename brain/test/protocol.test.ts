// The wire protocol end to end: the real src/main.ts in a child process, with the
// SDK replaced by test/fake-sdk.ts (see test/fake-brain.ts). No model calls.
// Run: bun test test/protocol.test.ts
import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const DATA = mkdtempSync(join(tmpdir(), "inkling-brain-protocol-"));

type Ev = Record<string, any>;

class FakeBrain {
  proc: ReturnType<typeof Bun.spawn>;
  events: Ev[] = [];
  stderr = "";
  private waiters: { pred: (e: Ev) => boolean; resolve: (e: Ev) => void }[] = [];

  constructor(env: Record<string, string> = {}) {
    this.proc = Bun.spawn(["bun", "run", "test/fake-brain.ts"], {
      cwd: ROOT,
      env: { PATH: process.env.PATH ?? "", HOME: process.env.HOME ?? "", INKLING_DATA_DIR: DATA, ...env },
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
    });
    void this.pump(this.proc.stdout as ReadableStream<Uint8Array>, (line) => {
      const ev = JSON.parse(line);
      this.events.push(ev);
      this.waiters = this.waiters.filter((w) => (w.pred(ev) ? (w.resolve(ev), false) : true));
    });
    void this.pump(this.proc.stderr as ReadableStream<Uint8Array>, (line) => (this.stderr += line + "\n"));
  }

  private async pump(stream: ReadableStream<Uint8Array>, onLine: (l: string) => void) {
    const dec = new TextDecoder();
    let buf = "";
    for await (const chunk of stream) {
      buf += dec.decode(chunk, { stream: true });
      let i: number;
      while ((i = buf.indexOf("\n")) >= 0) {
        const line = buf.slice(0, i);
        buf = buf.slice(i + 1);
        if (line.trim()) onLine(line);
      }
    }
  }

  send(req: unknown) {
    const stdin = this.proc.stdin as import("bun").FileSink;
    stdin.write(typeof req === "string" ? req : JSON.stringify(req) + "\n");
    stdin.flush();
  }

  waitFor(pred: (e: Ev) => boolean, ms = 5000): Promise<Ev> {
    const have = this.events.find(pred);
    if (have) return Promise.resolve(have);
    return new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`timed out; events so far: ${JSON.stringify(this.events)}\nstderr: ${this.stderr}`)), ms);
      this.waiters.push({ pred, resolve: (e) => (clearTimeout(t), resolve(e)) });
    });
  }

  /** The done or error that ends request `id`. */
  end(id: string, ms = 5000) {
    return this.waitFor((e) => e.id === id && (e.ev === "done" || e.ev === "error"), ms);
  }

  of(id: string) {
    return this.events.filter((e) => e.id === id);
  }

  closeStdin() {
    (this.proc.stdin as import("bun").FileSink).end();
  }

  kill() {
    this.proc.kill();
  }
}

const settings = { model: "claude-opus-5-5", effort: "low" };
const fake = (d: object) => `[[fake ${JSON.stringify(d)}]]`;
const ctx = (noteText = "", extra: object = {}) => ({
  noteTitle: "Rate limiter",
  noteText,
  brief: null,
  mind: { name: "Inkling", description: "", howItThinks: "", voice: "plain", frequency: "when_it_matters" },
  aboutMe: null,
  threads: [],
  now: "Thursday 6 November 2026",
  ...extra,
});
const run = (id: string, input: object, s: object = settings) => ({ op: "run", id, settings: s, input });
const ask = (id: string, d: object) => run(id, { task: "ask", ctx: ctx(fake(d)), target: "the words", question: "why?" });

let brains: FakeBrain[] = [];
function brain(env?: Record<string, string>) {
  const b = new FakeBrain(env);
  brains.push(b);
  return b;
}
afterEach(() => {
  for (const b of brains) b.kill();
  brains = [];
});

describe("startup and lifecycle", () => {
  test("the first line is ready", async () => {
    const b = brain();
    const ev = await b.waitFor(() => true);
    expect(ev).toEqual({ ev: "ready", provider: "claude", version: "0.1.0" });
  });

  test("closing stdin exits, even with a request in flight", async () => {
    const b = brain();
    await b.waitFor((e) => e.ev === "ready");
    b.send(ask("r1", { hang: true }));
    await Bun.sleep(100);
    b.closeStdin();
    const code = await Promise.race([b.proc.exited, Bun.sleep(3000).then(() => "still running")]);
    expect(code).toBe(0);
  });

  test("shutdown exits", async () => {
    const b = brain();
    await b.waitFor((e) => e.ev === "ready");
    b.send(ask("r1", { hang: true }));
    b.send({ op: "shutdown" });
    const code = await Promise.race([b.proc.exited, Bun.sleep(3000).then(() => "still running")]);
    expect(code).toBe(0);
  });
});

describe("requests", () => {
  test("a streamed ask: deltas in order, then done with the full text", async () => {
    const b = brain();
    b.send(ask("r1", { deltas: ["The ", "reset ", "runs late."], delayMs: 5 }));
    const done = await b.end("r1");
    expect(done.ev).toBe("done");
    expect(done.data).toEqual({ text: "The reset runs late." });
    expect(b.of("r1").filter((e) => e.ev === "delta").map((e) => e.text)).toEqual(["The ", "reset ", "runs late."]);
    expect(done.usage.ms).toBeGreaterThanOrEqual(0);
  });

  test("concurrent requests interleave and each ends once", async () => {
    const b = brain();
    b.send(ask("a", { deltas: ["a1", "a2", "a3", "a4"], delayMs: 25 }));
    b.send(ask("b", { deltas: ["b1", "b2", "b3", "b4"], delayMs: 15 }));
    b.send(run("c", { task: "chat", ctx: ctx(fake({ deltas: ["c1", "c2"], delayMs: 10 })), history: [], message: "hi", near: null }));
    await Promise.all([b.end("a"), b.end("b"), b.end("c")]);
    const order = b.events.filter((e) => e.ev === "delta").map((e) => e.id);
    expect(new Set(order.slice(0, 4)).size).toBeGreaterThan(1); // really interleaved
    for (const id of ["a", "b", "c"]) {
      const evs = b.of(id);
      expect(evs.filter((e) => e.ev === "done").length).toBe(1);
      expect(evs.filter((e) => e.ev === "error").length).toBe(0);
      const text = evs.filter((e) => e.ev === "delta").map((e) => e.text).join("");
      expect(evs.at(-1)!.data.text).toBe(text);
    }
  });

  test("cancel mid-stream: no done, no later deltas, one cancelled error", async () => {
    const b = brain();
    b.send(ask("r1", { deltas: Array.from({ length: 50 }, (_, i) => `w${i} `), delayMs: 20 }));
    await b.waitFor((e) => e.id === "r1" && e.ev === "delta");
    b.send({ op: "cancel", id: "r1" });
    const end = await b.end("r1");
    expect(end).toMatchObject({ ev: "error", kind: "cancelled" });
    await Bun.sleep(150);
    const evs = b.of("r1");
    expect(evs.filter((e) => e.ev === "done").length).toBe(0);
    expect(evs.filter((e) => e.ev === "error").length).toBe(1);
    expect(evs.at(-1)!.ev).toBe("error");
  });

  test("cancel racing a finished result still never sends done", async () => {
    const b = brain();
    // The fake ignores the abort and returns a result anyway.
    b.send(ask("r1", { deltas: ["a", "b", "c"], delayMs: 60, ignoreAbort: true }));
    await b.waitFor((e) => e.id === "r1" && e.ev === "delta");
    b.send({ op: "cancel", id: "r1" });
    const end = await b.end("r1");
    expect(end).toMatchObject({ ev: "error", kind: "cancelled" });
    await Bun.sleep(100);
    const deltas = b.of("r1").filter((e) => e.ev === "delta");
    expect(deltas.length).toBe(1); // nothing streams after the cancel
  });

  test("cancel of a request that has not produced anything yet", async () => {
    const b = brain();
    b.send(ask("r1", { hang: true }));
    b.send({ op: "cancel", id: "r1" });
    expect(await b.end("r1")).toMatchObject({ ev: "error", kind: "cancelled" });
  });

  test("cancel of an unknown id is a no-op", async () => {
    const b = brain();
    b.send({ op: "cancel", id: "nope" });
    b.send(ask("r1", { result: "fine" }));
    expect((await b.end("r1")).data).toEqual({ text: "fine" });
  });

  test("a second run with the id of a running request is ignored", async () => {
    const b = brain();
    b.send(ask("r1", { deltas: ["x"], delayMs: 50 }));
    b.send(ask("r1", { result: "imposter" }));
    const end = await b.end("r1");
    await Bun.sleep(150);
    expect(end.data).toEqual({ text: "x" });
    expect(b.of("r1").filter((e) => e.ev === "done" || e.ev === "error").length).toBe(1);
  });

  test("a very large note goes through intact", async () => {
    const b = brain();
    const big = "lorem ipsum dolor sit amet ".repeat(200_000) + "ünïcødé 漢字 🚀"; // ~5.4 MB
    b.send(run("big", { task: "ask", ctx: ctx(big + fake({ echo: "options" })), target: "lorem", question: "q" }));
    const end = await b.end("big", 20000);
    expect(end.ev).toBe("done");
    const echoed = JSON.parse(end.data.text);
    expect(echoed.prompt).toContain("ünïcødé 漢字 🚀");
    expect(echoed.prompt.length).toBeGreaterThan(big.length);
  });
});

describe("malformed input never kills the brain", () => {
  const junk = [
    "this is not json\n",
    "null\n",
    "42\n",
    '"a string"\n',
    "[]\n",
    "{}\n",
    '{"op":"bogus","id":"x1"}\n',
    '{"op":"run"}\n',
    '{"op":"status"}\n',
    "\n",
    "   \n",
    '{"op":"run","id":"half"', // a line split across two writes
  ];
  test("junk lines are ignored and later requests still work", async () => {
    const b = brain();
    for (const j of junk) b.send(j);
    b.send("}\n"); // completes the split line: a run without input
    b.send(ask("after", { result: "still here" }));
    expect((await b.end("after")).data).toEqual({ text: "still here" });
    const ends = b.events.filter((e) => e.ev === "done" || e.ev === "error");
    expect(ends.map((e) => [e.id, e.ev])).toEqual([
      ["half", "error"],
      ["after", "done"],
    ]);
    expect(b.proc.exitCode).toBeNull();
  });

  const bad: [string, object, string][] = [
    ["no input", { op: "run", id: "x", settings }, "input is missing"],
    ["unknown task", run("x", { task: "summarise", ctx: ctx() }), 'unknown task "summarise"'],
    ["no task", run("x", { ctx: ctx() }), "unknown task null"],
    ["no ctx", run("x", { task: "read", target: "t" }), "ctx is missing"],
    ["no target", run("x", { task: "read", ctx: ctx() }), "target is missing"],
    ["blank question", run("x", { task: "ask", ctx: ctx(), target: "t", question: "  " }), "question is missing"],
    ["no message", run("x", { task: "reply", ctx: ctx(), quote: "q", messages: [] }), "message is missing"],
    ["learn without message", run("x", { task: "learn", aboutMe: null }), "message is missing"],
  ];
  for (const [name, req, msg] of bad) {
    test(`${name} gets an error for its id`, async () => {
      const b = brain();
      b.send(req);
      const end = await b.end("x");
      expect(end).toMatchObject({ ev: "error", kind: "other" });
      expect(end.message).toContain(msg);
    });
  }

  test("missing optional context fields and settings are filled in", async () => {
    const b = brain();
    b.send({ op: "run", id: "x", input: { task: "ask", ctx: { noteText: fake({ echo: "options" }) }, target: "t", question: "q" } });
    const end = await b.end("x");
    expect(end.ev).toBe("done");
    const echoed = JSON.parse(end.data.text);
    expect(echoed.model).toBeNull();
    expect(echoed.effort).toBeNull();
    expect(echoed.systemPrompt).toContain("You are Inkling");
    expect(echoed.systemPrompt).not.toContain("undefined");
  });

  test("the writer's own threads and the mind each line came from survive the context", async () => {
    const b = brain();
    const threads = [
      { quote: "four days a week", resolved: false, own: true, messages: [{ role: "you", text: "Too many?" }, { role: "mind", text: "Which two are hard?", mind: "Skeptic" }] },
      { quote: "5:30", resolved: false, own: "yes", messages: [{ role: "mind", text: "Check the sum.", mind: "  " }] },
    ];
    const history = [{ role: "you", text: "Two easy, two hard." }, { role: "you", text: "Inkling, enough?", mind: "ignored on your lines" }];
    b.send(run("x", { task: "ask", ctx: ctx(fake({ echo: "options" }), { threads }), target: "four days a week", question: "Too many?", history }));
    const echoed = JSON.parse((await b.end("x")).data.text);
    expect(echoed.prompt).toContain("Thread 1 (the writer's own) on «four days a week»:\n  Writer: Too many?\n  Skeptic: Which two are hard?");
    // only `true` marks a thread as theirs, and a blank name is no name
    expect(echoed.prompt).toContain("Thread 2 on «5:30»:\n  You: Check the sum.");
    expect(echoed.prompt).toContain("Unless they ask you about one directly");
    expect(echoed.prompt).toContain("On «four days a week» the writer hung a question of their own:");
    expect(echoed.prompt).toContain("Writer: Two easy, two hard.\nWriter: Inkling, enough?");
  });

  test("unknown voice and frequency fall back instead of printing undefined", async () => {
    const b = brain();
    const c = ctx(fake({ echo: "options" }), { mind: { name: "Ada", voice: "shouty", frequency: "always" } });
    b.send(run("x", { task: "ask", ctx: c, target: "t", question: "q" }, { model: "m", effort: "extreme" }));
    const echoed = JSON.parse((await b.end("x")).data.text);
    expect(echoed.systemPrompt).toContain("You are Ada");
    expect(echoed.systemPrompt).not.toContain("undefined");
    expect(echoed.effort).toBeNull();
  });
});

describe("what each task sends back", () => {
  const thought = (speak: boolean, quote: string, t: string, extra: object = {}) => ({
    structured: { speak, kind: "question", quote, text: t, sketch: false, ...extra },
  });

  test("read that speaks: quote snapped onto the paragraph", async () => {
    const b = brain();
    const target = "We’ll keep one Redis counter per user and reset it every minute.";
    b.send(run("r", { task: "read", ctx: ctx(fake(thought(true, "ONE redis counter", "What happens at the boundary?", { sketch: true }))), target }));
    expect((await b.end("r")).data).toEqual({ speak: true, kind: "question", quote: "one Redis counter", text: "What happens at the boundary?", sketch: true });
  });

  test("read that stays silent", async () => {
    const b = brain();
    b.send(run("r", { task: "read", ctx: ctx(fake(thought(false, "", ""))), target: "Fine." }));
    expect((await b.end("r")).data).toEqual({ speak: false });
  });

  test("read with speak but no text is silence", async () => {
    const b = brain();
    b.send(run("r", { task: "read", ctx: ctx(fake(thought(true, "x", "   "))), target: "Fine." }));
    expect((await b.end("r")).data).toEqual({ speak: false });
  });

  test("a probe always speaks; an empty one is bad_output", async () => {
    const b = brain();
    b.send(run("r", { task: "probe", ctx: ctx(fake(thought(false, "a", "Why a?"))), target: "a b" }));
    expect((await b.end("r")).data).toEqual({ speak: true, kind: "question", quote: "a", text: "Why a?", sketch: false });
    b.send(run("r2", { task: "probe", ctx: ctx(fake(thought(false, "", ""))), target: "a b" }));
    expect(await b.end("r2")).toMatchObject({ ev: "error", kind: "bad_output" });
  });

  describe("a correction shows its working", () => {
    const target = "My pace is 5:30 per km, so the 10k takes 50 minutes.";
    const sum = { sum: [{ sum: "5:30 × 10", is: "55:00" }], givens: [{ value: "5:30", words: "5:30 per km" }, { value: "10", words: "the 10k" }] };
    const correction = (check: object, question = "How long is 10 km at 5:30?") =>
      thought(true, "so the 10k takes 50 minutes", "At 5:30 a km the 10k takes 55:00.", { kind: "correction", check, question });

    test("a backed correction carries its check", async () => {
      const b = brain();
      b.send(run("r", { task: "read", ctx: ctx(fake(correction(sum)) + "\n" + target), target }));
      expect((await b.end("r")).data).toEqual({
        speak: true,
        kind: "correction",
        quote: "so the 10k takes 50 minutes",
        text: "At 5:30 a km the 10k takes 55:00.",
        sketch: false,
        check: { sum: [{ sum: "5:30 × 10", is: "55:00", rounded: false }], givens: sum.givens, words: [] },
      });
    });

    test("one that does not add up arrives as its question", async () => {
      const b = brain();
      const wrong = { ...sum, sum: [{ sum: "5:30 × 10", is: "50:00" }] };
      b.send(run("r", { task: "read", ctx: ctx(fake(correction(wrong)) + "\n" + target), target }));
      expect((await b.end("r")).data).toEqual({ speak: true, kind: "question", quote: "so the 10k takes 50 minutes", text: "How long is 10 km at 5:30?", sketch: false });
      expect(b.stderr).toContain("correction became a question: line 1: 5:30 × 10 is 55:00, not 50:00");
    });

    test("with no question to fall back on, a read is silent and a probe is bad_output", async () => {
      const b = brain();
      b.send(run("r", { task: "read", ctx: ctx(fake(correction({}, "")) + "\n" + target), target }));
      expect((await b.end("r")).data).toEqual({ speak: false });
      b.send(run("p", { task: "probe", ctx: ctx(fake(correction({}, "")) + "\n" + target), target }));
      expect(await b.end("p")).toMatchObject({ ev: "error", kind: "bad_output" });
    });
  });

  test("a read made by Read it through answers like a read, with the note out of its prompt", async () => {
    const b = brain();
    const target = "We keep one Redis counter per user and reset it every minute.";
    // The fake SDK takes its directive from the prompt. In the paragraph, it is found.
    b.send(run("t", { task: "read", ctx: ctx("the note"), target: `${target} ${fake(thought(true, "reset it every minute", "When does it reset?"))}`, through: true }));
    expect((await b.end("t")).data).toEqual({ speak: true, kind: "question", quote: "reset it every minute", text: "When does it reset?", sketch: false });
    b.send(run("s", { task: "read", ctx: ctx("the note"), target: `Fine. ${fake(thought(false, "", ""))}`, through: true }));
    expect((await b.end("s")).data).toEqual({ speak: false });
    // In the note, it is found by a read along but not by the pass's read, whose note
    // rides in the system prompt: the fake SDK then answers "ok", which is no thought.
    const inNote = ctx(fake(thought(false, "", "")));
    b.send(run("a", { task: "read", ctx: inNote, target }));
    expect((await b.end("a")).data).toEqual({ speak: false });
    b.send(run("p", { task: "read", ctx: inNote, target, through: true }));
    expect(await b.end("p")).toMatchObject({ ev: "error", kind: "bad_output" });
  });

  test("reading it through still holds back a point the writer let stand, and a correction still shows its working", async () => {
    const b = brain();
    // the directives sit in the paragraph: a pass's note rides in the system prompt
    const point = { quote: "four days a week", point: "Is pace the problem?", kind: "question", mind: "Skeptic", reason: "Pace is the problem." };
    b.send(run("l", { task: "read", ctx: ctx("Four days a week.", { letStand: [point] }), target: `Four days a week. ${fake(thought(true, "Four days", "Why four?", { repeats: 1 }))}`, through: true }));
    expect((await b.end("l")).data).toEqual({ speak: false, repeats: 1 });
    const pace = "My pace is 5:30 per km, so the 10k takes 50 minutes.";
    const sum = { sum: [{ sum: "5:30 × 10", is: "55:00" }], givens: [{ value: "5:30", words: "5:30 per km" }, { value: "10", words: "the 10k" }] };
    const correction = thought(true, "so the 10k takes 50 minutes", "At 5:30 a km the 10k takes 55:00.", { kind: "correction", check: sum, question: "How long is 10 km at 5:30?" });
    b.send(run("w", { task: "read", ctx: ctx(pace), target: `${pace} ${fake(correction)}`, through: true }));
    expect((await b.end("w")).data).toMatchObject({ kind: "correction", quote: "so the 10k takes 50 minutes", check: { sum: [{ sum: "5:30 × 10", is: "55:00", rounded: false }] } });
  });

  describe("sources from the web, only when real", () => {
    const TOWER = "https://en.wikipedia.org/wiki/Eiffel_Tower";
    const target = "The Eiffel Tower is 450 metres tall.";
    const web = { ...settings, web: true };
    const hit = { query: "eiffel tower height", results: [{ title: "Eiffel Tower - Wikipedia", url: TOWER }] };
    const page = "<p>The tower is 330 metres (1,083 ft) tall, about the same height as an 81-storey building.</p>";
    const cite = (url: string, extra: object = {}) =>
      thought(true, "450 metres tall", "It is 330 metres, not 450.", { kind: "correction", check: { source: { url, passage: "The tower is 330 metres (1,083 ft) tall" } }, question: "Where did 450 come from?", ...extra });

    test("a page a search returned, with the passage on it, backs the correction", async () => {
      const b = brain();
      b.send(run("r", { task: "probe", ctx: ctx(fake({ ...cite(TOWER), searches: [hit], pages: { [TOWER]: page } }) + "\n" + target), target }, web));
      const done = await b.end("r");
      expect(done.data.kind).toBe("correction");
      expect(done.data.check.source).toEqual({ url: TOWER, title: "Eiffel Tower - Wikipedia", domain: "en.wikipedia.org", passage: "The tower is 330 metres (1,083 ft) tall" });
      expect(done.usage.searches).toBe(1);
    });

    test("a made-up URL, or a passage the page does not have, falls back to the question", async () => {
      const b = brain();
      const madeUp = "https://eiffel-facts.example/height";
      b.send(run("r1", { task: "probe", ctx: ctx(fake({ ...cite(madeUp), searches: [hit], pages: { [madeUp]: page } }) + "\n" + target), target }, web));
      expect((await b.end("r1")).data).toMatchObject({ kind: "question", text: "Where did 450 come from?" });
      b.send(run("r2", { task: "probe", ctx: ctx(fake({ ...cite(TOWER), searches: [hit], pages: { [TOWER]: "<p>Nothing about its height.</p>" } }) + "\n" + target), target }, web));
      expect((await b.end("r2")).data).toMatchObject({ kind: "question", text: "Where did 450 come from?" });
      expect(b.stderr).toContain(`a source was dropped: ${madeUp} did not come back from a search in this answer`);
      expect(b.stderr).toContain(`a source was dropped: the passage is not on ${TOWER}`);
    });

    test("with the web off, nothing was searched, so no page counts", async () => {
      const b = brain();
      b.send(run("r", { task: "probe", ctx: ctx(fake({ ...cite(TOWER), pages: { [TOWER]: page } }) + "\n" + target), target }));
      expect((await b.end("r")).data).toMatchObject({ kind: "question" });
    });

    test("reading along never looks things up, even with the web on", async () => {
      const b = brain();
      b.send(run("r", { task: "read", ctx: ctx(fake({ ...cite(TOWER), searches: [hit], pages: { [TOWER]: page } }) + "\n" + target), target }, web));
      expect((await b.end("r")).data).toMatchObject({ kind: "question", text: "Where did 450 come from?" });
    });

    test("a call that runs out of turns answers again without the web, and counts both", async () => {
      const b = brain();
      // the first call runs out of turns; the second (no tools) answers as a question
      b.send(run("p", { task: "probe", ctx: ctx(fake({ resultError: { subtype: "error_max_turns", isError: true }, webOnce: true, structured: { kind: "question", quote: "450 metres tall", text: "Where did 450 come from?", sketch: false } }) + "\n" + target), target }, web));
      const done = await b.end("p");
      expect(done.data).toMatchObject({ kind: "question", text: "Where did 450 come from?" });
      expect(b.stderr).toContain("answering without the web");
    });

    test("in prose, a link no search returned is taken out", async () => {
      const b = brain();
      b.send(run("a", { task: "ask", ctx: ctx(fake({ deltas: [`It is 330 m (${TOWER}); `, "see https://made.up/x too."], searches: [hit] })), target: "the words", question: "why?" }, web));
      expect((await b.end("a")).data.text).toBe(`It is 330 m (${TOWER}); see too.`);
    });

    test("the tools: file tools always, web tools when enabled; no spending cap", async () => {
      const b = brain();
      const echo = (id: string, input: object, s: object) => {
        b.send(run(id, input, s));
        return b.end(id).then((e) => JSON.parse(e.data.text ?? "{}"));
      };
      const chat = await echo("c", { task: "chat", ctx: ctx(fake({ echo: "options" })), history: [], message: "hi", near: null }, web);
      expect(chat).toMatchObject({ tools: ["Read", "Glob", "Grep", "WebSearch", "WebFetch"], allowedTools: ["Read", "Glob", "Grep", "WebSearch", "WebFetch"], maxTurns: 8, maxBudgetUsd: null });
      expect(chat.prompt).toContain("Link only to a page your search returned");
      const off = await echo("o", { task: "chat", ctx: ctx(fake({ echo: "options" })), history: [], message: "hi", near: null }, settings);
      expect(off).toMatchObject({ tools: ["Read", "Glob", "Grep"], maxTurns: 8, maxBudgetUsd: null });
      expect(off.prompt).not.toContain("WebSearch");
    });
  });

  test("a non-object structured answer is bad_output", async () => {
    const b = brain();
    b.send(run("r", { task: "read", ctx: ctx(fake({ structured: ["speak"] })), target: "a b" }));
    expect(await b.end("r")).toMatchObject({ ev: "error", kind: "bad_output" });
  });

  test("sketch", async () => {
    const b = brain();
    b.send(run("v", { task: "sketch", ctx: ctx(fake({ structured: { quote: "reset", text: " The window resets all at once. " } })), target: "reset it every minute" }));
    expect((await b.end("v")).data).toEqual({ quote: "reset", text: "The window resets all at once." });
  });

  test("draw cleans the SVG; prose without SVG is bad_output", async () => {
    const b = brain();
    const svg = '<svg viewBox="0 0 320 200"><g data-step="1" data-caption="R&D"><text>a</text></g></svg>';
    b.send(run("d", { task: "draw", ctx: ctx(fake({ result: "```svg\n" + svg + "\n```" })), quote: "q", thought: "t" }));
    expect((await b.end("d")).data).toEqual({ svg: svg.replace("R&D", "R&amp;D") });
    b.send(run("d2", { task: "draw", ctx: ctx(fake({ result: "Sorry, I can only describe it." })), quote: "q", thought: "t" }));
    expect(await b.end("d2")).toMatchObject({ ev: "error", kind: "bad_output" });
  });

  test("read with open threads: the threads its paragraph answers, by id, only when there are some", async () => {
    const b = brain();
    const target = "The long run starts at 8 km on 6 December.";
    const threads = [
      { id: "t1", quote: "q1", messages: [], resolved: true, open: false },
      { id: "t2", quote: "q2", messages: [], resolved: false, open: true },
    ];
    const answers = [{ thread: 2, quote: "STARTS at 8 km" }, { thread: 1, quote: "x" }];
    b.send(run("a", { task: "read", ctx: ctx(fake(thought(false, "", "", { answers })), { threads }), target }));
    expect((await b.end("a")).data).toEqual({ speak: false, answers: [{ thread: "t2", quote: "starts at 8 km" }] });
    b.send(run("s", { task: "read", ctx: ctx(fake(thought(true, "8 km", "From when?", { answers })), { threads }), target }));
    expect((await b.end("s")).data).toEqual({ speak: true, kind: "question", quote: "8 km", text: "From when?", sketch: false, answers: [{ thread: "t2", quote: "starts at 8 km" }] });
    // no open thread: no answers, whatever comes back
    b.send(run("n", { task: "read", ctx: ctx(fake(thought(false, "", "", { answers })), { threads: [threads[0]] }), target }));
    expect((await b.end("n")).data).toEqual({ speak: false });
  });

  test("a read for answers only never speaks, and costs nothing with no open thread", async () => {
    const b = brain();
    const threads = [{ id: "t2", quote: "q2", messages: [], resolved: false, open: true }];
    b.send(run("q", { task: "read", answersOnly: true, ctx: ctx(fake(thought(true, "8 km", "From when?", { answers: [{ thread: 1, quote: "8 km" }] })), { threads }), target: "From 8 km." }));
    expect((await b.end("q")).data).toEqual({ speak: false, answers: [{ thread: "t2", quote: "8 km" }] });
    b.send(run("z", { task: "read", answersOnly: true, ctx: ctx(fake({ hang: true })), target: "From 8 km." }));
    expect(await b.end("z", 2000)).toMatchObject({ ev: "done", data: { speak: false } });
  });

  test("settle: settled with the words, snapped; settled with no words is not settled", async () => {
    const b = brain();
    const target = "The long run starts at 8 km on 6 December.";
    const settle = (id: string, structured: object) =>
      run(id, { task: "settle", ctx: ctx(fake({ structured })), quote: "add a kilometre", messages: [{ role: "mind", text: "From how far?" }], target, answer: "starts at 8 km", notAnswers: [] });
    b.send(settle("y", { settled: true, quote: "STARTS at 8 km", text: " From 8 km, 16 comes in week nine. " }));
    expect((await b.end("y")).data).toEqual({ settled: true, quote: "starts at 8 km", text: "From 8 km, 16 comes in week nine." });
    b.send(settle("w", { settled: true, quote: "  ", text: "Settled." }));
    expect((await b.end("w")).data).toEqual({ settled: false, quote: "", text: "" });
    b.send(settle("x", { settled: false, quote: "", text: "" }));
    expect((await b.end("x")).data).toEqual({ settled: false, quote: "", text: "" });
    b.send(run("m", { task: "settle", ctx: ctx(), quote: "q", messages: [] }));
    expect(await b.end("m")).toMatchObject({ ev: "error", kind: "other" });
  });

  test("reply: text, resolved, redraw", async () => {
    const b = brain();
    b.send(run("p", { task: "reply", ctx: ctx(fake({ structured: { text: "Yes.", resolved: true, redraw: false } })), quote: "q", messages: [{ role: "mind", text: "Why?" }], message: "Because." }));
    expect((await b.end("p")).data).toEqual({ text: "Yes.", resolved: true, redraw: false });
  });

  describe("relate", () => {
    const passages = [
      { note: "e1", noteTitle: "Race calendar", written: "Thursday 3 September 2026", edited: "Thursday 3 September 2026", heading: null, text: "…Pick up the race bib on Friday 13 March at the harbour office." },
      { note: "e3", noteTitle: "March budget", written: "Thursday 1 October 2026", edited: "Thursday 1 October 2026", heading: null, text: "The March budget for running gear is £120." },
    ];
    const target = "I'll pick up my bib on Saturday 14 March.";
    // the fake SDK's answer rides in `before`, so the quotes cannot match its own text
    const relate = (id: string, answer: object, extra: object = {}) =>
      run(id, { task: "relate", ctx: ctx(), target, before: [fake({ structured: answer })], passages, ...extra });
    const clash = { same_subject: true, relation: "contradicts", passage: 1, quote: "SATURDAY 14 March", other: "“Friday 13 March”", text: " Race calendar says Friday 13 March. Which day? " };

    test("that speaks: the passage by index, its note, both quotes exact, the relation", async () => {
      const b = brain();
      b.send(relate("r", clash));
      expect((await b.end("r")).data).toEqual({ speak: true, passage: 0, note: "e1", quote: "Saturday 14 March", other: "Friday 13 March", relation: "contradicts", text: "Race calendar says Friday 13 March. Which day?" });
      b.send(relate("r2", { ...clash, passage: 2, relation: "depends", other: "£120" }));
      expect((await b.end("r2")).data).toMatchObject({ speak: true, passage: 1, note: "e3", other: "£120", relation: "depends" });
    });

    test("is silent unless both are the same thing, they disagree, the passage was sent, its words are real and there is a note", async () => {
      const b = brain();
      const silent = [
        { ...clash, same_subject: false },
        { ...clash, relation: "none" },
        { ...clash, relation: "maybe" },
        { ...clash, passage: 0 },
        { ...clash, passage: 3 },
        { ...clash, passage: 1.5 },
        { ...clash, other: "Saturday 20 March" },
        { ...clash, text: "  " },
      ];
      silent.forEach((a, i) => b.send(relate(`s${i}`, a)));
      for (let i = 0; i < silent.length; i++) expect((await b.end(`s${i}`)).data).toEqual({ speak: false });
    });

    test("a clash that makes a point the writer let stand again is held back", async () => {
      const b = brain();
      const point = { quote: "Saturday 14 March", point: "Is the office open on Saturday?", kind: "question", mind: "Inkling", reason: "It is." };
      b.send(run("l", { task: "relate", ctx: ctx("", { letStand: [point] }), target, before: [fake({ structured: { ...clash, repeats: 1 } })], passages }));
      expect((await b.end("l")).data).toEqual({ speak: false, repeats: 1 });
      // a number that names no point is ignored
      b.send(run("m", { task: "relate", ctx: ctx("", { letStand: [point] }), target, before: [fake({ structured: { ...clash, repeats: 4 } })], passages }));
      expect((await b.end("m")).data).toMatchObject({ speak: true, other: "Friday 13 March" });
    });

    test("with no passages there is no model call", async () => {
      const b = brain();
      b.send(run("n", { task: "relate", ctx: ctx(), target: target + " " + fake({ hang: true }), before: [], passages: [{ note: "x", noteTitle: "Empty", text: "  " }] }));
      expect(await b.end("n", 2000)).toMatchObject({ ev: "done", data: { speak: false } });
    });

    test("a target is required; before and passages may be left out", async () => {
      const b = brain();
      b.send(run("t", { task: "relate", ctx: ctx(), passages }));
      expect(await b.end("t")).toMatchObject({ ev: "error", kind: "other" });
      b.send(run("u", { task: "relate", ctx: ctx(), target }));
      expect((await b.end("u")).data).toEqual({ speak: false });
    });
  });

  test("a reply may carry the other end of a thread between notes, well formed or not", async () => {
    const b = brain();
    const answer = fake({ structured: { text: "Then fix the calendar.", resolved: false, redraw: false } });
    const elsewhere = { noteTitle: "Race calendar", edited: "Thursday 3 September 2026", quote: "Friday 13 March", text: "Pick up the race bib on Friday 13 March.", relation: "contradicts" };
    for (const [id, e] of [["e1", elsewhere], ["e2", { ...elsewhere, text: null, gone: true }], ["e3", "junk"], ["e4", { quote: "" }]] as const) {
      b.send(run(id, { task: "reply", ctx: ctx(answer), quote: "Saturday 14 March", messages: [], message: "Saturday.", elsewhere: e }));
      expect((await b.end(id)).data).toEqual({ text: "Then fix the calendar.", resolved: false, redraw: false });
    }
  });

  test("learn: update and no update; the profile is clipped on a character boundary", async () => {
    const b = brain();
    const long = "You " + "🚀".repeat(800);
    b.send(run("l1", { task: "learn", aboutMe: null, message: fake({ structured: { update: true, aboutMe: long } }) }));
    const l1 = (await b.end("l1")).data;
    expect(l1.update).toBe(true);
    expect(Array.from(l1.aboutMe as string).length).toBe(700);
    expect((l1.aboutMe as string).isWellFormed()).toBe(true);
    b.send(run("l2", { task: "learn", aboutMe: "You write Rust.", message: fake({ structured: { update: false, aboutMe: "" } }) }));
    expect((await b.end("l2")).data).toEqual({ update: false, aboutMe: "" });
    b.send(run("l3", { task: "learn", aboutMe: null, message: fake({ structured: { update: true, aboutMe: "  " } }) }));
    expect((await b.end("l3")).data).toEqual({ update: false, aboutMe: "" });
  });

  test("limit events are forwarded", async () => {
    const b = brain();
    b.send(ask("r", { limit: { status: "allowed_warning", resetsAt: 1762430400, utilization: 0.91, rateLimitType: "five_hour" }, result: "ok" }));
    await b.end("r");
    expect(b.events.find((e) => e.ev === "limit")).toEqual({ ev: "limit", status: "allowed_warning", resetsAt: 1762430400, utilization: 0.91, kind: "five_hour" });
  });
});

describe("error kinds on the wire", () => {
  const cases: [string, object, string][] = [
    ["auth tag", { assistantError: "authentication_failed", resultError: { subtype: "success", result: "Invalid API key · Please run /login" } }, "auth"],
    ["rate limit rejected", { limit: { status: "rejected" }, resultError: { subtype: "success", result: "You've hit your limit" } }, "rate_limit"],
    ["overloaded", { throwMessage: "API Error: 529 Overloaded" }, "overloaded"],
    ["structured retries", { resultError: { subtype: "error_max_structured_output_retries", errors: ["gave up"] } }, "bad_output"],
    ["no result", { endEarly: true }, "other"],
  ];
  for (const [name, d, kind] of cases) {
    test(name, async () => {
      const b = brain();
      b.send(run("e", { task: "read", ctx: ctx(fake(d)), target: "a b c d" }));
      const end = await b.end("e");
      expect(end).toMatchObject({ ev: "error", kind });
      expect(typeof end.message).toBe("string");
      expect(end.message.length).toBeGreaterThan(0);
    });
  }
});

describe("status", () => {
  test("answers with the account", async () => {
    const b = brain();
    b.send({ op: "status", id: "s" });
    const ev = await b.waitFor((e) => e.id === "s");
    expect(ev).toMatchObject({ ev: "status", data: { account: { email: "writer@example.com", subscriptionType: "max" } } });
  });

  test("a hung runtime times out with an error and does not hold up other requests", async () => {
    const b = brain({ FAKE_STATUS: "hang", INKLING_STATUS_TIMEOUT_MS: "800" });
    b.send({ op: "status", id: "s" });
    b.send(ask("r", { result: "meanwhile" }));
    const r = await b.end("r");
    expect(b.of("s").length).toBe(0); // the run finished first
    expect(r.data).toEqual({ text: "meanwhile" });
    const s = await b.waitFor((e) => e.id === "s", 4000);
    expect(s).toMatchObject({ ev: "error", kind: "other" });
    expect(s.message).toContain("timed out");
  });

  test("not logged in is an auth error", async () => {
    const b = brain({ FAKE_STATUS: "fail" });
    b.send({ op: "status", id: "s" });
    expect(await b.waitFor((e) => e.id === "s")).toMatchObject({ ev: "error", kind: "auth" });
  });
});

describe("what reaches the runtime", () => {
  test("options: file tools, user context, no session, settings passed through, cwd in the data dir", async () => {
    const b = brain();
    b.send(run("o", { task: "chat", ctx: ctx(fake({ echo: "options" })), history: [], message: "hi", near: "the reset" }, { model: "claude-sonnet-5-5", effort: "medium" }));
    const o = JSON.parse((await b.end("o")).data.text);
    expect(o).toMatchObject({ model: "claude-sonnet-5-5", effort: "medium", tools: ["Read", "Glob", "Grep"], maxTurns: 8, persistSession: false, settingSources: ["user"], includePartialMessages: true, outputFormat: null });
    expect(o.cwd).toBe(join(DATA, "brain-cwd"));
    expect(o.systemPrompt).toContain("projects/*/memory/MEMORY.md");
    expect(o).toMatchObject({ settings: { disableAllHooks: true }, permissionMode: "dontAsk", strictMcpConfig: true });
    expect(o.prompt).toContain("«the reset»");
  });

  test("note directories reach the model for inherited or overridden context", async () => {
    const b = brain();
    b.send(run("paths", { task: "chat", ctx: { ...ctx(fake({ echo: "options" })), localDirectories: ["/tmp/project-a", "/tmp/project-b"] }, history: [], message: "check the implementation", near: null }, settings));
    const o = JSON.parse((await b.end("paths")).data.text);
    expect(o.systemPrompt).toContain('["/tmp/project-a","/tmp/project-b"]');
    expect(o.tools).toEqual(["Read", "Glob", "Grep"]);
  });

  test("ANTHROPIC_API_KEY is dropped unless INKLING_USE_API_KEY=1", async () => {
    const b = brain({ ANTHROPIC_API_KEY: "sk-ant-test", ANTHROPIC_AUTH_TOKEN: "tok" });
    b.send(ask("e", { echo: "env" }));
    expect(JSON.parse((await b.end("e")).data.text)).toEqual({ ANTHROPIC_API_KEY: false, ANTHROPIC_AUTH_TOKEN: false, CLAUDE_AGENT_SDK_CLIENT_APP: "inkling/0.1.0" });
    const k = brain({ ANTHROPIC_API_KEY: "sk-ant-test", INKLING_USE_API_KEY: "1" });
    k.send(ask("e", { echo: "env" }));
    expect(JSON.parse((await k.end("e")).data.text).ANTHROPIC_API_KEY).toBe(true);
  });
});

describe("points the writer let stand", () => {
  const point = { quote: "four days a week", point: "Is pace the problem?", kind: "question", mind: "Skeptic", reason: "Pace is the problem." };
  const thought = (extra: object) => ({ structured: { speak: true, kind: "question", quote: "four days", text: "Why four?", sketch: false, ...extra } });

  test("reach the system prompt one line each; bad entries are dropped and long ones cut", async () => {
    const b = brain();
    const letStand = [
      { ...point, point: "Is pace\n\nthe   problem?" },
      { quote: "", point: "no words" },
      { quote: "no point" },
      "junk",
      { quote: "x".repeat(400), point: "y".repeat(400), kind: "bogus", reason: "   " },
      ...Array.from({ length: 30 }, (_, i) => ({ quote: `q${i}`, point: `p${i}` })),
    ];
    b.send(run("x", { task: "ask", ctx: ctx(fake({ echo: "options" }), { letStand }), target: "t", question: "q" }));
    const sys: string = JSON.parse((await b.end("x")).data.text).systemPrompt;
    expect(sys).toContain("1. On «four days a week», Skeptic's question: Is pace the problem? Their reason: Pace is the problem.");
    expect(sys).toContain(`2. On «${"x".repeat(160)}», Inkling's question: ${"y".repeat(240)} (No reason given.)`);
    expect(sys).toContain("20. On «q17»");
    expect(sys).not.toContain("21. On");
    expect(sys).not.toContain("no words");
  });

  test("a missing list is none", async () => {
    const b = brain();
    b.send(run("x", { task: "ask", ctx: ctx(fake({ echo: "options" })), target: "t", question: "q" }));
    expect(JSON.parse((await b.end("x")).data.text).systemPrompt).not.toContain("let stand");
  });

  test("a read whose note repeats one is held back; a number that names none is ignored", async () => {
    const b = brain();
    b.send(run("r", { task: "read", ctx: ctx(fake(thought({ repeats: 1 })), { letStand: [point] }), target: "Four days a week." }));
    expect((await b.end("r")).data).toEqual({ speak: false, repeats: 1 });
    b.send(run("r2", { task: "read", ctx: ctx(fake(thought({ repeats: 3 })), { letStand: [point] }), target: "Four days a week." }));
    expect((await b.end("r2")).data).toMatchObject({ speak: true, quote: "Four days", text: "Why four?" });
    b.send(run("r3", { task: "read", ctx: ctx(fake(thought({ repeats: 1 }))), target: "Four days a week." }));
    expect((await b.end("r3")).data).toMatchObject({ speak: true });
    expect(b.stderr).toContain("held back");
  });

  test("a probe that only finds one says so instead of speaking", async () => {
    const b = brain();
    b.send(run("p", { task: "probe", ctx: ctx(fake(thought({ repeats: 1 })), { letStand: [point] }), target: "four days a week" }));
    expect((await b.end("p")).data).toEqual({ speak: false, repeats: 1 });
  });
});

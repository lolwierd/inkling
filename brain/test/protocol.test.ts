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

  test("reply: text, resolved, redraw", async () => {
    const b = brain();
    b.send(run("p", { task: "reply", ctx: ctx(fake({ structured: { text: "Yes.", resolved: true, redraw: false } })), quote: "q", messages: [{ role: "mind", text: "Why?" }], message: "Because." }));
    expect((await b.end("p")).data).toEqual({ text: "Yes.", resolved: true, redraw: false });
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
  test("options: no tools, one turn, no session, settings passed through, cwd in the data dir", async () => {
    const b = brain();
    b.send(run("o", { task: "chat", ctx: ctx(fake({ echo: "options" })), history: [], message: "hi", near: "the reset" }, { model: "claude-sonnet-5-5", effort: "medium" }));
    const o = JSON.parse((await b.end("o")).data.text);
    expect(o).toMatchObject({ model: "claude-sonnet-5-5", effort: "medium", tools: [], maxTurns: 1, persistSession: false, settingSources: [], includePartialMessages: true, outputFormat: null });
    expect(o.cwd).toBe(join(DATA, "brain-cwd"));
    expect(o.prompt).toContain("«the reset»");
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

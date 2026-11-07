// Pure pieces of the brain: quote snapping, SVG cleaning, clipping, error
// classification, the child environment, and the SDK drain against a fake SDK.
// Run: bun test test/unit.test.ts
import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

process.env.INKLING_DATA_DIR = join(tmpdir(), "inkling-brain-unit-test");

const { accountStatus, addUsage, childEnv, classify, installedClaude, runStructured, runText, setQueryForTests, usageLine, ProviderError } = await import("../src/claude.ts");
const { cleanSvg, clip, findQuote, snapQuote } = await import("../src/output.ts");
const { relatePrompt, replyPrompt, readPrompt } = await import("../src/prompts.ts");
const { fakeQuery } = await import("./fake-sdk.ts");
setQueryForTests(fakeQuery as any);

const settings = { model: "claude-opus-5-5", effort: "low" as const };
const fake = (d: object) => `[[fake ${JSON.stringify(d)}]]`;

test("finds native, Homebrew, and custom Claude installs and reports missing executables", () => {
  const home = mkdtempSync(join(tmpdir(), "inkling-claude-"));
  try {
    const shellBin = join(home, "bin");
    const nativeBin = join(home, ".local", "bin");
    const brewBin = join(home, "homebrew", "bin");
    mkdirSync(shellBin);
    mkdirSync(nativeBin, { recursive: true });
    mkdirSync(brewBin, { recursive: true });
    const shellClaude = join(shellBin, "claude");
    const nativeClaude = join(nativeBin, "claude");
    const brewClaude = join(brewBin, "claude");
    writeFileSync(shellClaude, "#!/bin/sh\n", { mode: 0o755 });
    symlinkSync(shellClaude, brewClaude);
    symlinkSync(shellClaude, nativeClaude);
    expect(installedClaude({}, home, [brewBin])).toBe(nativeClaude);
    expect(installedClaude({ INKLING_CLAUDE_PATH: shellClaude }, home, [brewBin])).toBe(shellClaude);

    rmSync(nativeClaude);
    symlinkSync(join(home, "missing"), nativeClaude);
    expect(installedClaude({}, home, [brewBin])).toBe(brewClaude);
    expect(installedClaude({ PATH: shellBin }, home, [brewBin])).toBe(shellClaude);
    expect(() => installedClaude({}, home, [])).toThrow("Claude Code was not found");
    expect(() => installedClaude({ INKLING_CLAUDE_PATH: nativeClaude }, home, [brewBin]))
      .toThrow("INKLING_CLAUDE_PATH must point to an executable");
  } finally {
    rmSync(home, { recursive: true });
  }
});

describe("snapQuote", () => {
  const src = "We’ll keep one Redis counter per user and  reset it every minute.";
  test("exact", () => expect(snapQuote("one Redis counter", src)).toBe("one Redis counter"));
  test("strips wrapping quote marks", () => expect(snapQuote("“one Redis counter”", src)).toBe("one Redis counter"));
  test("case differences map back to the source's casing", () => expect(snapQuote("ONE redis COUNTER", src)).toBe("one Redis counter"));
  test("straight apostrophe matches a curly one", () => expect(snapQuote("We'll keep", src)).toBe("We’ll keep"));
  test("collapsed whitespace returns the source's own spacing", () => {
    const got = snapQuote("and reset it every minute", src);
    expect(got).toBe("and  reset it every minute");
    expect(src.includes(got)).toBe(true);
  });
  test("trailing punctuation the source does not have", () => expect(snapQuote("per user;", src)).toBe("per user"));
  test("ellipsis spans from the first piece to the last", () => {
    expect(snapQuote("one Redis counter … every minute", src)).toBe("one Redis counter per user and  reset it every minute");
  });
  test("no match falls back to the whole source", () => expect(snapQuote("a Postgres table", src)).toBe(src));
  test("empty quote falls back to the whole source", () => expect(snapQuote("  “” ", src)).toBe(src));
  test("Japanese", () => {
    const ja = "キャッシュは毎分リセットされるので、古いデータは残らない。";
    expect(snapQuote("毎分リセットされる", ja)).toBe("毎分リセットされる");
    expect(snapQuote("「古いデータは残らない」", ja)).toBe("古いデータは残らない");
  });
  test("full-width letters fold onto ASCII", () => expect(snapQuote("redis", "Ｒｅｄｉｓ を使う")).toBe("Ｒｅｄｉｓ"));
  test("case folding that changes length (Turkish İ) keeps indices right", () => {
    const tr = "İstanbul'da önbellek her dakika sıfırlanır.";
    expect(snapQuote("ÖNBELLEK her", tr)).toBe("önbellek her");
  });
  test("astral characters are never split", () => {
    const s = "Ship it 🚀 on Friday, then 🎉 Monday";
    const got = snapQuote("🚀 ON FRIDAY", s);
    expect(got).toBe("🚀 on Friday");
  });
  test("result is always a substring", () => {
    const cases = ["UNIQUE  words", "keep one", "we'll KEEP", "zzz", "", "minute.", "…"];
    for (const c of cases) expect(src.includes(snapQuote(c, src))).toBe(true);
  });
});

describe("findQuote", () => {
  const passage = "…Pick up the race bib on Friday 13 March at the harbour office.";
  test("exact and loose matches are the passage's own words", () => {
    expect(findQuote("Friday 13 March", passage)).toBe("Friday 13 March");
    expect(findQuote("“friday 13 march”", passage)).toBe("Friday 13 March");
    expect(findQuote("…Pick up the race bib", passage)).toBe("Pick up the race bib");
  });
  test("nothing that matches is null, never the whole passage", () => {
    expect(findQuote("Saturday 14 March", passage)).toBeNull();
    expect(findQuote("", passage)).toBeNull();
    expect(snapQuote("Saturday 14 March", passage)).toBe(passage);
  });
});

describe("prompts for threads between notes", () => {
  const mind = { name: "Inkling", description: "", howItThinks: "", voice: "plain" as const, frequency: "when_it_matters" as const };
  const ctx = (threads: any[] = []) => ({ noteTitle: "Race-week plan", noteText: "Race week is about sleep.\n\nI'll pick up my bib on Saturday 14 March.", brief: null, mind, aboutMe: null, threads, now: "Saturday 7 November 2026" });
  const passages = [
    { note: "e1", noteTitle: "Race calendar", written: "Thursday 3 September 2026", edited: "Thursday 3 September 2026", heading: "Spring", text: "Pick up the race bib on Friday 13 March at the harbour office." },
    { note: "e3", noteTitle: "March \"budget\"", written: "Thursday 1 October 2026", edited: "Thursday 1 October 2026", heading: null, text: "The March budget for running gear is £120." },
  ];
  test("relate shows the paragraph, what comes before it and each passage with its note and dates, but not the rest of the note", () => {
    const p = relatePrompt(ctx(), "I'll pick up my bib on Saturday 14 March.", ["Race week is about sleep."], passages);
    expect(p).toContain("<paragraph>I'll pick up my bib on Saturday 14 March.</paragraph>");
    expect(p).toContain("<before>Race week is about sleep.</before>");
    expect(p).toContain('<passage n="1" note="Race calendar" written="Thursday 3 September 2026" edited="Thursday 3 September 2026" heading="Spring">Pick up the race bib');
    expect(p).toContain(`<passage n="2" note="March 'budget'"`);
    expect(p).toContain("The search only matched words");
    expect(p).toContain("same_subject");
    expect(p).not.toContain("<note");
    expect(p).not.toContain("Already in the margin");
  });
  test("relate names the open threads already on the paragraph", () => {
    const threads = [
      { quote: "Saturday 14 March", messages: [{ role: "mind", text: "Why Saturday?" }], resolved: false },
      { quote: "elsewhere entirely", messages: [{ role: "mind", text: "Not this one." }], resolved: false },
    ];
    const p = relatePrompt(ctx(threads), "I'll pick up my bib on Saturday 14 March.", [], passages);
    expect(p).toContain("Already in the margin about this paragraph:\n- «Saturday 14 March»: Why Saturday?");
    expect(p).not.toContain("Not this one.");
    expect(p).not.toContain("<before>");
  });
  test("a reply on a thread between notes sees the other end, or that its note is gone", () => {
    const end = { noteTitle: "Race calendar", edited: "Thursday 3 September 2026", quote: "Friday 13 March", text: "Pick up the race bib on Friday 13 March at the harbour office.", relation: "contradicts" as const };
    const p = replyPrompt(ctx(), "Saturday 14 March", [{ role: "mind", text: "Which day?" }], "Saturday.", end);
    expect(p).toContain("This thread also quotes the writer's other note «Race calendar» (last edited Thursday 3 September 2026): «Friday 13 March», in this passage:\n<passage>Pick up the race bib");
    expect(p).toContain("For a thread between two notes, resolved is true when the two no longer disagree");
    const gone = replyPrompt(ctx(), "Saturday 14 March", [], "Saturday.", { ...end, text: null, gone: true });
    expect(gone).toContain("which has since been deleted");
    const plain = replyPrompt(ctx(), "Saturday 14 March", [], "Saturday.");
    expect(plain).not.toContain("other note");
    expect(plain).not.toContain("\n\n\n");
  });
  test("every prompt's margin names a thread's other end", () => {
    const p = readPrompt(ctx([{ quote: "Saturday 14 March", messages: [], resolved: false, elsewhere: { noteTitle: "Race calendar", quote: "Friday 13 March" } }]), "x");
    expect(p).toContain("Thread 1 on «Saturday 14 March», which also quotes «Friday 13 March» in the note «Race calendar»:");
  });
});

describe("cleanSvg", () => {
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 200"><g data-step="1" data-caption="A"><text x="10" y="20">a</text></g></svg>';
  test("plain", () => expect(cleanSvg(svg)).toBe(svg));
  test("code fences and prose around it", () => expect(cleanSvg("Here you go:\n```svg\n" + svg + "\n```\nHope it helps")).toBe(svg));
  test("two SVGs: keeps the first whole one", () => expect(cleanSvg(svg + "\n" + svg.replace("A", "B"))).toBe(svg));
  test("no SVG is bad_output", () => {
    expect(() => cleanSvg("I can't draw that.")).toThrow(ProviderError);
    try {
      cleanSvg("nothing");
    } catch (e: any) {
      expect(e.kind).toBe("bad_output");
    }
  });
  test("cut off before </svg> is bad_output", () => {
    try {
      cleanSvg(svg.slice(0, -10));
      throw new Error("expected a throw");
    } catch (e: any) {
      expect(e.kind).toBe("bad_output");
    }
  });
  test("HTML entities, bare ampersands and bare < become XML", () => {
    const messy =
      '<svg viewBox="0 0 320 200"><g data-step="1" data-caption="R&D &rarr; ship"><text>a &lt; b &amp; c</text><text>x < y &nbsp; &mdash; &#8594; &#x2192; &bogus;</text></g></svg>';
    const out = cleanSvg(messy);
    expect(out).toContain('data-caption="R&amp;D → ship"');
    expect(out).toContain("a &lt; b &amp; c");
    expect(out).toContain("x &lt; y   — &#8594; &#x2192; &amp;bogus;");
  });
  test("drops scripts, styles, foreignObject, comments and handlers", () => {
    const evil =
      '<svg viewBox="0 0 320 200" onload="alert(1)"><!-- note --><style>.a{}</style><script>alert(1)</script><foreignObject><div>x</div></foreignObject><image href="x.png"/><g data-step="1" data-caption="c" onclick=\'x()\'><path d="M0 0 L10 10"/></g></svg>';
    const out = cleanSvg(evil);
    expect(out).toBe('<svg viewBox="0 0 320 200"><g data-step="1" data-caption="c"><path d="M0 0 L10 10"/></g></svg>');
  });
});

describe("clip", () => {
  test("short strings unchanged", () => expect(clip("abc", 5)).toBe("abc"));
  test("counts characters, not UTF-16 units, and never leaves half a pair", () => {
    const s = "a" + "🚀".repeat(10);
    const out = clip(s, 5);
    expect(Array.from(out).length).toBe(5);
    expect(out.isWellFormed()).toBe(true);
    expect(JSON.stringify(out)).not.toContain("\\ud");
  });
});

describe("classify", () => {
  const cases: [string, string][] = [
    ["Invalid API key · Please run /login", "auth"],
    ["Not logged in · Please run /login", "auth"],
    ["OAuth token has expired. Please obtain a new token or refresh your existing token.", "auth"],
    ['API Error: 401 {"type":"error","error":{"type":"authentication_error"}}', "auth"],
    ["API Error: 403 Forbidden", "auth"],
    ["Claude AI usage limit reached|1762430400", "rate_limit"],
    ["You've hit your limit · resets 5pm (Asia/Kolkata)", "rate_limit"],
    ['API Error: 429 {"type":"error","error":{"type":"rate_limit_error"}}', "rate_limit"],
    ['API Error: 529 {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}', "overloaded"],
    ["Repeated 529 Overloaded errors", "overloaded"],
    // Request ids and other digits must not look like status codes.
    ["API Error: 500 Internal server error (request_id req_test123)", "other"],
    ["The author of this note is not here", "other"],
    ["Claude Code process exited with code 1", "other"],
    ["Claude Code process aborted by user", "other"],
  ];
  for (const [msg, kind] of cases) test(`${kind}: ${msg.slice(0, 50)}`, () => expect(classify(msg)).toBe(kind as any));
});

describe("childEnv", () => {
  test("drops an exported API key and auth token by default", () => {
    const env = childEnv({ ANTHROPIC_API_KEY: "sk-test", ANTHROPIC_AUTH_TOKEN: "t", HOME: "/h" });
    expect(env.ANTHROPIC_API_KEY).toBeUndefined();
    expect(env.ANTHROPIC_AUTH_TOKEN).toBeUndefined();
    expect(env.HOME).toBe("/h");
    expect(env.CLAUDE_AGENT_SDK_CLIENT_APP).toBe("inkling/0.1.4");
  });
  test("keeps them with INKLING_USE_API_KEY=1", () => {
    const env = childEnv({ ANTHROPIC_API_KEY: "sk-test", INKLING_USE_API_KEY: "1" });
    expect(env.ANTHROPIC_API_KEY).toBe("sk-test");
  });
});

describe("drain against the fake SDK", () => {
  const ac = () => new AbortController();
  const kindOf = async (p: Promise<unknown>) => {
    try {
      await p;
      return "ok";
    } catch (e: any) {
      return e.kind;
    }
  };

  test("streams deltas and returns the result text", async () => {
    const seen: string[] = [];
    const out = await runText("sys", fake({ deltas: ["a", "b", "c"] }), settings, ac(), (t) => seen.push(t));
    expect(seen).toEqual(["a", "b", "c"]);
    expect(out.text).toBe("abc");
    expect(out.usage.model).toBe("claude-opus-5-5");
  });
  test("structured output", async () => {
    const out = await runStructured<any>("sys", fake({ structured: { a: 1 } }), {}, settings, ac());
    expect(out.data).toEqual({ a: 1 });
  });
  test("structured calls leave room for the runtime's schema retry; text calls allow file lookups", async () => {
    const s = await runStructured<any>("sys", fake({ echo: "options" }), {}, settings, ac());
    expect(s.data.maxTurns).toBe(11);
    expect(s.data.outputFormat).toBe("json_schema");
    expect(s.data.tools).toEqual(["Read", "Glob", "Grep"]);
    const t = await runText("sys", fake({ echo: "options" }), settings, ac());
    expect(JSON.parse(t.text).maxTurns).toBe(8);
  });
  test("structured output missing falls back to JSON in the text", async () => {
    const out = await runStructured<any>("sys", fake({ result: 'Sure: {"a": 2} done' }), {}, settings, ac());
    expect(out.data).toEqual({ a: 2 });
  });
  test("no JSON anywhere is bad_output", async () => {
    expect(await kindOf(runStructured("sys", fake({ result: "no json here" }), {}, settings, ac()))).toBe("bad_output");
  });
  test("structured-output retries exhausted is bad_output", async () => {
    const d = { resultError: { subtype: "error_max_structured_output_retries", errors: ["Failed to provide valid structured output after 5 attempts"] } };
    expect(await kindOf(runStructured("sys", fake(d), {}, settings, ac()))).toBe("bad_output");
  });
  test("max turns on a structured task is bad_output", async () => {
    expect(await kindOf(runStructured("sys", fake({ resultError: { subtype: "error_max_turns" } }), {}, settings, ac()))).toBe("bad_output");
  });
  test("the runtime's assistant error tag wins over the text", async () => {
    const d = { assistantError: "authentication_failed", resultError: { subtype: "success", result: "Something went wrong" } };
    expect(await kindOf(runText("sys", fake(d), settings, ac()))).toBe("auth");
    const r = { assistantError: "rate_limit", resultError: { subtype: "success", result: "Request failed" } };
    expect(await kindOf(runText("sys", fake(r), settings, ac()))).toBe("rate_limit");
  });
  test("a rejected rate-limit event marks the failure as rate_limit", async () => {
    const seen: any[] = [];
    const d = { limit: { status: "rejected", resetsAt: 1762430400, rateLimitType: "five_hour" }, resultError: { subtype: "success", result: "Request failed" } };
    expect(await kindOf(runText("sys", fake(d), settings, ac(), undefined, (l) => seen.push(l)))).toBe("rate_limit");
    expect(seen).toEqual([{ status: "rejected", resetsAt: 1762430400, utilization: undefined, kind: "five_hour" }]);
  });
  test("a thrown SDK error is classified from its message", async () => {
    expect(await kindOf(runText("sys", fake({ throwMessage: "API Error: 529 Overloaded" }), settings, ac()))).toBe("overloaded");
  });
  test("ending without a result is other", async () => {
    expect(await kindOf(runText("sys", fake({ endEarly: true }), settings, ac()))).toBe("other");
  });
  test("aborting a hung call settles as cancelled", async () => {
    const signal = ac();
    const p = kindOf(runText("sys", fake({ hang: true }), settings, signal));
    setTimeout(() => signal.abort(), 30);
    expect(await p).toBe("cancelled");
  });
});

describe("accountStatus", () => {
  afterEach(() => delete process.env.FAKE_STATUS);
  test("answers with the account and models", async () => {
    const s: any = await accountStatus(500);
    expect(s.account.email).toBe("writer@example.com");
    expect(s.models).toEqual([{ value: "claude-opus-5-5", displayName: "Opus 5.5" }]);
  });
  test("asks without naming a model or an effort", async () => {
    let seen: any;
    setQueryForTests(((args: any) => {
      seen = args.options;
      return fakeQuery(args);
    }) as any);
    try {
      await accountStatus(500);
    } finally {
      setQueryForTests(fakeQuery as any);
    }
    expect(seen.model).toBeUndefined();
    expect(seen.effort).toBeUndefined();
  });
  test("a hung runtime times out with an error instead of hanging", async () => {
    process.env.FAKE_STATUS = "hang";
    const t0 = Date.now();
    let err: any;
    try {
      await accountStatus(300);
    } catch (e) {
      err = e;
    }
    expect(err?.kind).toBe("other");
    expect(String(err?.message)).toContain("timed out");
    expect(Date.now() - t0).toBeLessThan(2000);
  });
  test("a hung model list still answers with the account", async () => {
    process.env.FAKE_STATUS = "slow-models";
    const s: any = await accountStatus(300);
    expect(s.account.email).toBe("writer@example.com");
    expect(s.models).toBeNull();
  });
  test("not logged in is an auth error", async () => {
    process.env.FAKE_STATUS = "fail";
    let err: any;
    try {
      await accountStatus(500);
    } catch (e) {
      err = e;
    }
    expect(err?.kind).toBe("auth");
  });
});

describe("points the writer let stand", async () => {
  const { letStandBlock, systemPrompt, readPrompt, probePrompt, THOUGHT_SCHEMA, PROBE_SCHEMA } = await import("../src/prompts.ts");
  const mind = { name: "Editor", description: "", howItThinks: "", voice: "plain" as const, frequency: "when_it_matters" as const };
  const base = { noteTitle: "Plan", noteText: "Four days a week.", brief: null, mind, aboutMe: null, threads: [], now: "Saturday 7 November 2026" };
  const points = [
    { quote: "four days a week", point: "Is pace the problem, or the last 3 km?", kind: "question" as const, mind: "Skeptic", reason: "Pace is the problem." },
    { quote: "always", point: "Always? What is the evidence?", kind: "correction" as const, mind: "Inkling", reason: null },
  ];

  test("numbered, each with its mind, kind and reason", () => {
    const block = letStandBlock(points);
    expect(block).toContain("Never raise any of them again, in any words");
    expect(block).toContain("1. On «four days a week», Skeptic's question: Is pace the problem, or the last 3 km? Their reason: Pace is the problem.");
    expect(block).toContain("2. On «always», Inkling's correction: Always? What is the evidence? (No reason given.)");
  });

  test("every mind's system prompt carries them, and none when there are none", () => {
    const told = systemPrompt({ ...base, letStand: points });
    expect(told).toContain("You are Editor");
    expect(told).toContain("Points the writer let stand.");
    expect(told.indexOf("Points the writer let stand.")).toBeLessThan(told.indexOf("Today is"));
    expect(systemPrompt({ ...base, letStand: [] })).not.toContain("let stand");
  });

  test("read and probe ask the model which point its note repeats, last", () => {
    expect(readPrompt({ ...base, letStand: points }, "Four days a week.")).toContain("or one the writer let stand");
    expect(probePrompt({ ...base, letStand: points }, "four days")).toContain("find the next most useful one");
    for (const schema of [THOUGHT_SCHEMA, PROBE_SCHEMA]) {
      expect(schema.required.at(-1)).toBe("repeats");
      expect(Object.keys(schema.properties).at(-1)).toBe("repeats");
      expect(schema.properties.repeats).toEqual({ type: "integer" });
    }
  });
});

describe("the writer's own threads in prompts", async () => {
  const { askPrompt, readPrompt, OWN_THREADS_RULE } = await import("../src/prompts.ts");
  const mind = { name: "Inkling", description: "", howItThinks: "", voice: "plain" as const, frequency: "when_it_matters" as const };
  const ctx = (threads: any[]) => ({ noteTitle: "Plan", noteText: "Run four days a week.", brief: null, mind, aboutMe: null, threads, letStand: [], now: "Saturday 7 November 2026" });
  const own = { quote: "four days a week", resolved: false, own: true, messages: [{ role: "you" as const, text: "Too many?" }] };
  const theirs = { quote: "Run", resolved: true, messages: [{ role: "mind" as const, text: "How far?" }] };

  test("a read marks them and says once not to answer or ask them again", () => {
    const p = readPrompt(ctx([theirs, own]), "Run four days a week.");
    expect(p).toContain("Thread 1 (resolved) on «Run»:\n  You: How far?");
    expect(p).toContain("Thread 2 (the writer's own) on «four days a week»:\n  Writer: Too many?");
    expect(p.split(OWN_THREADS_RULE).length).toBe(2);
  });

  test("without them the margin reads as before", () => {
    const p = readPrompt(ctx([theirs]), "Run four days a week.");
    expect(p).not.toContain(OWN_THREADS_RULE);
    expect(p).not.toContain("the writer's own");
  });

  test("another mind's line is under its name; the mind's own lines stay 'You'", () => {
    const visited = { ...own, messages: [...own.messages, { role: "mind" as const, text: "Which two?", mind: "Skeptic" }, { role: "mind" as const, text: "Mine.", mind: "inkling" }] };
    const p = readPrompt(ctx([visited]), "Run four days a week.");
    expect(p).toContain("  Skeptic: Which two?");
    expect(p).toContain("  You: Mine.");
  });

  test("Ask without a history is today's prompt; a call into the writer's thread brings it", () => {
    const plain = askPrompt(ctx([]), "four days a week", "Too many?");
    expect(plain).toContain("The writer selected «four days a week» and asked you:\n<question>Too many?</question>");
    expect(askPrompt(ctx([]), "four days a week", "Too many?", false, [])).toBe(plain);
    const called = askPrompt(ctx([own]), "four days a week", "Too many?", false, [
      { role: "you", text: "Two easy, two hard." },
      { role: "mind", text: "Which two?", mind: "Skeptic" },
      { role: "you", text: "Inkling, is that enough?" },
    ]);
    expect(called).toContain("On «four days a week» the writer hung a question of their own:\n<question>Too many?</question>");
    expect(called).toContain("Writer: Two easy, two hard.\nSkeptic: Which two?\nWriter: Inkling, is that enough?");
    expect(called).toContain("They have now called you into the thread.");
    expect(called).not.toContain("selected «");
  });
});

const { pickAnswers } = await import("../src/output.ts");
const { answersBlock, answersOnlyPrompt, settlePrompt, THOUGHT_SCHEMA, THOUGHT_WITH_ANSWERS_SCHEMA } = await import("../src/prompts.ts");

describe("answers in the note", () => {
  const mind = { name: "Inkling", description: "", howItThinks: "", voice: "plain" as const, frequency: "when_it_matters" as const };
  const thread = (id: string, open: boolean, notAnswers: string[] = []) => ({ id, quote: `q ${id}`, messages: [], resolved: !open, open, notAnswers });
  const ctx = (threads: ReturnType<typeof thread>[]) => ({ noteTitle: "Plan", noteText: "The long run starts at 8 km.", brief: null, mind, aboutMe: null, threads, now: "Friday 7 November 2026", letStand: [] });
  const target = "The long run starts at 8 km on 6 December, so it reaches 16 km by late January.";

  test("a read names threads by their number; only open ones, each once, at most three, words snapped", () => {
    const threads = [thread("t1", false), thread("t2", true), thread("t3", true), thread("t4", true), thread("t5", true)];
    const raw = [
      { thread: 2, quote: "STARTS AT 8 km" },
      { thread: 1, quote: "tied off already" },
      { thread: 9, quote: "no such thread" },
      { thread: 2, quote: "twice" },
      { thread: 3, quote: "words that are not there" },
      { thread: 4, quote: "16 km" },
      { thread: 5, quote: "one too many" },
    ];
    expect(pickAnswers(raw, threads, target)).toEqual([
      { thread: "t2", quote: "starts at 8 km" },
      { thread: "t3", quote: target },
      { thread: "t4", quote: "16 km" },
    ]);
    expect(pickAnswers("nonsense", threads, target)).toEqual([]);
    expect(pickAnswers([{ thread: 2 }], [{ resolved: false }], target)).toEqual([]); // no id, nothing to name
  });

  test("a thread of the writer's own is never answered, whatever `open` says", () => {
    expect(pickAnswers([{ thread: 1, quote: "16 km" }], [{ id: "a", resolved: false, open: true, own: true }], target)).toEqual([]);
    expect(answersBlock({ ...ctx([]), threads: [{ ...thread("o", true), own: true }] } as any)).toBe("");
  });

  test("an older client's thread without `open` is open while not resolved", () => {
    expect(pickAnswers([{ thread: 1, quote: "16 km" }], [{ id: "a", resolved: false }], target)).toEqual([{ thread: "a", quote: "16 km" }]);
  });

  test("the read's answers block lists open threads and the words turned down, and nothing when none are open", () => {
    expect(answersBlock(ctx([thread("t1", false)]))).toBe("");
    const block = answersBlock(ctx([thread("t1", false), thread("t2", true, ["Sunday is the long run."]), thread("t3", true)]));
    expect(block).toContain("Open margin threads: 2, 3. Check whether this paragraph answers any of them.");
    expect(block).toContain("Thread 2: the writer has said these words do not answer it: «Sunday is the long run.».");
    expect(block).not.toContain("Thread 3:");
    const quiet = answersOnlyPrompt(ctx([thread("t2", true)]), target);
    expect(quiet).toContain("raise nothing new");
    expect(quiet).toContain(`<paragraph>${target}</paragraph>`);
    expect(quiet).toContain("Open margin threads: 1.");
  });

  test("the schema with answers is the read's schema plus answers, before repeats, which stays last", () => {
    const without = (l: string[]) => l.filter((k) => k !== "repeats");
    expect(THOUGHT_WITH_ANSWERS_SCHEMA.required).toEqual([...without([...THOUGHT_SCHEMA.required]), "answers", "repeats"]);
    expect(Object.keys(THOUGHT_WITH_ANSWERS_SCHEMA.properties)).toEqual([...without(Object.keys(THOUGHT_SCHEMA.properties)), "answers", "repeats"]);
  });

  test("settle and a read are background checks: they never look things up on the web", async () => {
    const { WEB_TASKS } = await import("../src/web.ts");
    expect(WEB_TASKS.has("settle") || WEB_TASKS.has("read")).toBe(false);
  });

  test("settle shows the thread, the paragraph, the words and what was turned down, and asks for strictness", () => {
    const p = settlePrompt(ctx([]), "add a kilometre every week", [{ role: "mind", text: "Starting from how far?" }, { role: "you", text: "Good point." }], target, "starts at 8 km", ["Sunday is the long run."]);
    expect(p).toContain("This margin thread is about «add a kilometre every week»:\nYou: Starting from how far?\nWriter: Good point.");
    expect(p).toContain(`<paragraph>${target}</paragraph>\nThe words that may settle it: «starts at 8 km»`);
    expect(p).toContain("do not answer it: «Sunday is the long run.».");
    expect(p).toContain("Be strict: when in doubt, it does not.");
    expect(p).not.toMatch(/\n\n\n/);
    const bare = settlePrompt(ctx([]), "q", [], target, undefined, []);
    expect(bare).toContain("(nothing said in it yet)");
    expect(bare).not.toContain("The words that may settle it");
  });
});

describe("usage", () => {
  test("two calls for one request add up", () => {
    const a = { ms: 900, model: "haiku", inputTokens: 300, outputTokens: 10, costUsd: 0.001, turns: 1 };
    const b = { ms: 2100, model: "sonnet", inputTokens: 4000, cacheReadTokens: 3000, outputTokens: 60, costUsd: 0.02, turns: 2, tools: { Grep: 1 } };
    const u = addUsage(a, b);
    expect(u).toEqual({ ms: 3000, model: "haiku + sonnet", inputTokens: 4300, cacheReadTokens: 3000, outputTokens: 70, costUsd: 0.021, turns: 3, tools: { Grep: 1 } });
    expect(usageLine("read", "r7", u)).toBe("usage read r7: 3.0 s, haiku + sonnet, in 4300 (cache read 3000, write 0), out 70, $0.0210, 3 turns, tools Grep×1");
  });
});

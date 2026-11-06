// Pure pieces of the brain: quote snapping, SVG cleaning, clipping, error
// classification, the child environment, and the SDK drain against a fake SDK.
// Run: bun test test/unit.test.ts
import { afterEach, describe, expect, test } from "bun:test";
import { tmpdir } from "node:os";
import { join } from "node:path";

process.env.INKLING_DATA_DIR = join(tmpdir(), "inkling-brain-unit-test");

const { accountStatus, childEnv, classify, runStructured, runText, setQueryForTests, ProviderError } = await import("../src/claude.ts");
const { cleanSvg, clip, snapQuote } = await import("../src/output.ts");
const { fakeQuery } = await import("./fake-sdk.ts");
setQueryForTests(fakeQuery as any);

const settings = { model: "claude-opus-5-5", effort: "low" as const };
const fake = (d: object) => `[[fake ${JSON.stringify(d)}]]`;

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
    ["API Error: 500 Internal server error (request_id req_0114013b4290)", "other"],
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
    expect(env.CLAUDE_AGENT_SDK_CLIENT_APP).toBe("inkling/0.1.0");
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
  test("structured calls leave room for the runtime's schema retry; text calls take one turn", async () => {
    const s = await runStructured<any>("sys", fake({ echo: "options" }), {}, settings, ac());
    expect(s.data.maxTurns).toBe(3);
    expect(s.data.outputFormat).toBe("json_schema");
    expect(s.data.tools).toEqual([]);
    const t = await runText("sys", fake({ echo: "options" }), settings, ac());
    expect(JSON.parse(t.text).maxTurns).toBe(1);
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
    expect(s.models.length).toBe(1);
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

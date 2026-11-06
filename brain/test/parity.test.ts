// Three copies of the task list must agree: the brain's protocol (src/protocol.ts),
// the tasks the app sends (src/*.rs), and the fake brain the end-to-end scenarios run
// on (test/e2e/fake-brain.ts). Adding a task to one and not the others is the usual
// way a new feature passes its unit tests and then fails, or cannot be tested, in the app.
// Run: bun test test/parity.test.ts
import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const BRAIN = join(import.meta.dir, "..");
const APP_SRC = join(BRAIN, "..", "src");

/** Task names in the protocol's `Task` union: `| { task: "read"; ... }`. */
function protocolTasks(): string[] {
  const src = readFileSync(join(BRAIN, "src/protocol.ts"), "utf8");
  const union = src.slice(src.indexOf("export type Task"), src.indexOf("export type Request"));
  return [...union.matchAll(/\{\s*task:\s*"(\w+)"/g)].map((m) => m[1]);
}

/** Task names the app puts on the wire: `"task": "reply"`, or `let task = if … { "probe" } else { "read" }`. */
function appTasks(): string[] {
  const names = new Set<string>();
  for (const f of readdirSync(APP_SRC).filter((f) => f.endsWith(".rs"))) {
    const src = readFileSync(join(APP_SRC, f), "utf8");
    // Tests may send anything; only the app's own requests count.
    const body = src.split(/#\[cfg\(test\)\]/)[0];
    for (const m of body.matchAll(/"task":\s*"(\w+)"/g)) names.add(m[1]);
    for (const m of body.matchAll(/let task = (.*);/g)) for (const q of m[1].matchAll(/"(\w+)"/g)) names.add(q[1]);
  }
  return [...names].sort();
}

describe("task lists agree", () => {
  const protocol = protocolTasks();

  test("the protocol lists the tasks main.ts handles", () => {
    expect(protocol.length).toBeGreaterThan(5);
    const main = readFileSync(join(BRAIN, "src/main.ts"), "utf8");
    for (const t of protocol) expect(main.includes(`case "${t}"`)).toBe(true);
  });

  test("every task the app sends is in the protocol", () => {
    const missing = appTasks().filter((t) => !protocol.includes(t));
    expect(missing).toEqual([]);
  });

  test("the fake brain answers every task in the protocol", async () => {
    const proc = Bun.spawn(["bun", "run", "test/e2e/fake-brain.ts"], {
      cwd: BRAIN,
      env: { PATH: process.env.PATH ?? "", HOME: process.env.HOME ?? "", FAKE_DELAY_MS: "0", FAKE_DELTA_MS: "0" },
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
    });
    const ctx = {
      noteTitle: "Parity",
      noteText: "Some words here.",
      brief: null,
      mind: { name: "Inkling", description: "", howItThinks: "", voice: "plain", frequency: "when_it_matters" },
      aboutMe: null,
      threads: [],
      now: "Thursday 6 November 2026",
    };
    const fields = { ctx, target: "Some words here.", question: "Why?", message: "I am a tester.", quote: "Some words", thought: "A thought.", messages: [], history: [], near: null, aboutMe: null };
    const stdin = proc.stdin as import("bun").FileSink;
    for (const t of protocol) stdin.write(JSON.stringify({ op: "run", id: t, settings: {}, input: { task: t, ...fields } }) + "\n");
    stdin.flush();

    const ends = new Map<string, any>();
    const dec = new TextDecoder();
    let buf = "";
    const deadline = Date.now() + 8000;
    for await (const chunk of proc.stdout as ReadableStream<Uint8Array>) {
      buf += dec.decode(chunk, { stream: true });
      let i: number;
      while ((i = buf.indexOf("\n")) >= 0) {
        const ev = JSON.parse(buf.slice(0, i));
        buf = buf.slice(i + 1);
        if (ev.ev === "done" || ev.ev === "error") ends.set(ev.id, ev);
      }
      if (ends.size === protocol.length || Date.now() > deadline) break;
    }
    proc.kill();
    const failed = protocol.filter((t) => ends.get(t)?.ev !== "done").map((t) => `${t}: ${JSON.stringify(ends.get(t) ?? "no answer")}`);
    expect(failed).toEqual([]);
  });
});

// Read it through's prompts: a read made by a pass is today's read with the note moved to
// the end of the system prompt (so the runtime can serve it from its cache for every read
// after the first) and two lines changed.
// Run: bun test test/read-through.test.ts
import { describe, expect, test } from "bun:test";
import { readPrompt, systemPrompt, throughSystemPrompt } from "../src/prompts.ts";
import type { Context } from "../src/protocol.ts";

const ctx: Context = {
  noteTitle: "Build cache",
  noteText: "The share fills up every two weeks.\n\nRollback triggers when the 5xx rate passes 2% for five minutes.",
  brief: null,
  mind: { name: "Skeptic", description: "", howItThinks: "", voice: "blunt", frequency: "rarely" },
  aboutMe: null,
  threads: [{ quote: "every two weeks", messages: [{ role: "mind", text: "Who clears it?" }], resolved: false }],
  now: "Saturday 7 November 2026",
  letStand: [{ quote: "five working days", point: "Why five days and not three?", kind: "question", mind: "Skeptic", reason: "The on-call week is five days." }],
};
const target = "If error rates rise we roll back that region.";

describe("a read made by Read it through", () => {
  const along = readPrompt(ctx, target);
  const through = readPrompt(ctx, target, true);

  test("reading along is unchanged: the note and its margin, then the paragraph just finished", () => {
    expect(along).toContain("<note title=\"Build cache\">");
    expect(along).toContain("Rollback triggers when the 5xx rate");
    expect(along).toContain("Thread 1 on «every two weeks»");
    expect(along).toContain("The writer just finished this paragraph:");
    expect(along).not.toContain("read it through");
    expect(readPrompt(ctx, target, false)).toBe(along);
  });

  test("the pass's prompt leaves the note to the system prompt and says the draft is finished", () => {
    expect(through).not.toContain("<note");
    expect(through).not.toContain("Rollback triggers");
    expect(through).toContain("The writer has finished this draft and asked you to read it through, one paragraph at a time.");
    expect(through).toContain(`<paragraph>${target}</paragraph>`);
    expect(through).toContain("when another paragraph of the note already answers what you would ask, stay silent.");
  });

  test("every rule of a read is kept, in the same order", () => {
    const rules = (p: string) => p.slice(p.indexOf("Decide whether")).split("\n").filter((l) => !l.startsWith("The whole draft is in front of you"));
    expect(rules(through)).toEqual(rules(along));
  });

  test("its system prompt is the usual one with the note and its margin at the end", () => {
    const sys = throughSystemPrompt(ctx);
    expect(sys.startsWith(systemPrompt(ctx))).toBe(true);
    const tail = sys.slice(systemPrompt(ctx).length);
    expect(tail).toContain("The note you are reading through, with its margin:");
    expect(tail).toContain("Rollback triggers when the 5xx rate passes 2% for five minutes.");
    expect(tail).toContain("Thread 1 on «every two weeks»");
    // the paragraph changes with every read, so it stays out of the cached part
    expect(sys).not.toContain(target);
  });

  test("a point the writer let stand reaches every read of a pass, and so do the working rules", () => {
    // Let it stand: the points ride in the system prompt, which the pass's prompt keeps
    expect(throughSystemPrompt(ctx)).toContain("Point 1. On «five working days», Skeptic's question: Why five days and not three? Their reason: The on-call week is five days.");
    // the read's rules (Show its working's check and question, repeats, no sources) are kept
    for (const rule of ["- check (corrections only)", "- question (corrections only)", "- repeats:", "Web lookup is disabled for this call"]) {
      expect(through).toContain(rule);
    }
  });

  test("the same note gives the same system prompt for every paragraph of the pass", () => {
    expect(throughSystemPrompt({ ...ctx })).toBe(throughSystemPrompt(ctx));
  });
});

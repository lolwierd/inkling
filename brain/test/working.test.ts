// Show its working: the sums a correction shows are worked out again here, every number
// in them must be the writer's, and a correction that cannot show its check becomes its
// fallback question. Run: bun test test/working.test.ts
import { describe, expect, test } from "bun:test";
import { backCorrection, evaluate, mentions, parseQuantity, withWorking, type Thought, type Writer } from "../src/working.ts";

const NOTE = [
  "Last year I ran the city 10k in 56:40. I went out too fast and walked twice after the 7 km mark. This year I want to finish under 50 minutes.",
  "",
  "The plan is to run four days a week and hold 5:30 per km on every run, so my legs learn race pace. If I hold 5:30 for the whole race, I’ll come in just under 50.",
  "",
  "Sunday is the long run. I’ll add a kilometre every week until I reach 16.",
].join("\n");
const TARGET = "The plan is to run four days a week and hold 5:30 per km on every run, so my legs learn race pace. If I hold 5:30 for the whole race, I’ll come in just under 50.";
const QUOTE = "If I hold 5:30 for the whole race, I’ll come in just under 50.";
const writer: Writer = { title: "Sub-50 10k by March", note: NOTE, brief: null, target: TARGET, quote: QUOTE };

const TEXT = "At 5:30 a km, 10 km takes 55:00. Under 50 needs 5:00 a km or faster. Which do you want to change: the pace or the goal?";
const GOOD = {
  sum: [
    { sum: "5:30 × 10", is: "55:00" },
    { sum: "50:00 ÷ 10", is: "5:00" },
  ],
  givens: [
    { value: "5:30", words: "hold 5:30 for the whole race" },
    { value: "10", words: "10k" },
    { value: "50:00", words: "under 50 minutes" },
  ],
};
const QUESTION = "What does 5:30 a km come to over 10 km?";
const correction = (text = TEXT): Thought => ({ speak: true, kind: "correction", quote: QUOTE, text, sketch: false });

function why(check: unknown, text = TEXT, w = writer): string {
  const r = backCorrection(check, text, w);
  return "why" in r ? r.why : "backed";
}

describe("quantities", () => {
  test("durations, numbers, percentages, money", () => {
    expect(parseQuantity("5:30")).toMatchObject({ kind: "duration", value: 330, half: 0.5 });
    expect(parseQuantity("1:05:00")?.value).toBe(3900);
    expect(parseQuantity("50:00")).toMatchObject({ lead: 50, whole: true });
    expect(parseQuantity("1,200")?.value).toBe(1200);
    expect(parseQuantity("3.33")).toMatchObject({ kind: "number", value: 3.33, half: 0.005 });
    expect(parseQuantity("15%")).toMatchObject({ kind: "percent", value: 0.15 });
    expect(parseQuantity("$40")?.value).toBe(40);
    expect(parseQuantity("≈ 49:10")?.value).toBe(2950);
    expect(parseQuantity("5:75")).toBeNull();
    expect(parseQuantity("five")).toBeNull();
  });

  test("numbers in the writer's words, in digits and in words", () => {
    const of = (s: string) => mentions(s).map((m) => m.values);
    expect(of("the city 10k in 56:40")).toEqual([[10, 10000], [3400]]);
    expect(of("four days a week")).toEqual([[4]]);
    expect(of("1,200 people, 15% of them")).toEqual([[1200], [0.15]]);
    expect(of("[r13] asked about v2.5")).toEqual([]);
    expect(of("the 7 km mark")).toEqual([[7]]);
  });
});

describe("sums", () => {
  const value = (s: string) => {
    const r = evaluate(s);
    return "error" in r ? r.error : r.value;
  };
  test("are worked out with the usual precedence", () => {
    expect(value("5:30 × 10")).toBe(3300);
    expect(value("50:00 ÷ 10")).toBe(300);
    expect(value("(5:30 − 5:00) × 10")).toBe(300);
    expect(value("1,200 × 15%")).toBeCloseTo(180);
    expect(value("1:05:00 − 49:10")).toBe(950);
    expect(value("2 + 3 × 4")).toBe(14);
    expect(value("5:30 * 10 / 2 - 1")).toBe(1649);
    expect(value("5:30 x 10")).toBe(3300);
    expect(value("−5 + 10")).toBe(5);
  });
  test("refuse words, a division by zero and broken brackets", () => {
    expect(value("five × 10")).toContain("not a number");
    expect(value("10 ÷ 0")).toContain("divides by zero");
    expect(value("(5:30 × 10")).toContain("not closed");
    expect(value("5:30 ×")).toContain("ends too soon");
  });
  test("are set with proper signs", () => {
    const r = evaluate("(5:30-5:00)*10");
    expect("shown" in r && r.shown).toBe("(5:30 − 5:00) × 10");
  });
});

describe("a correction shows its working", () => {
  test("the running plan's sum is backed, line by line", () => {
    const r = backCorrection(GOOD, TEXT, writer);
    expect(r).toEqual({
      check: {
        sum: [
          { sum: "5:30 × 10", is: "55:00", rounded: false },
          { sum: "50:00 ÷ 10", is: "5:00", rounded: false },
        ],
        givens: [
          { value: "5:30", words: "hold 5:30 for the whole race" },
          { value: "10", words: "10k" },
          { value: "50:00", words: "under 50 minutes" },
        ],
        words: [],
      },
    });
  });

  test("a rounded result is kept and marked", () => {
    const check = { sum: [{ sum: "50:00 ÷ 7", is: "7:09" }], givens: [{ value: "50", words: "just under 50" }] };
    const w = { ...writer };
    const r = backCorrection({ ...check, givens: [{ value: "50:00", words: "just under 50" }] }, "Spread over 7 days that is 7:09 a day.", w);
    expect("check" in r && r.check.sum[0]).toEqual({ sum: "50:00 ÷ 7", is: "7:09", rounded: true });
  });

  test("a line that does not add up", () => {
    expect(why({ ...GOOD, sum: [{ sum: "5:30 × 10", is: "50:00" }] }, "At 5:30 a km, 10 km takes 50:00.")).toBe(
      "line 1: 5:30 × 10 is 55:00, not 50:00",
    );
  });
  test("a number from nowhere", () => {
    expect(why({ ...GOOD, sum: [{ sum: "5:30 × 10.2", is: "56:06" }] }, "That is 56:06.")).toBe(
      "line 1: 10.2 is not in the note, not an earlier result and not a unit",
    );
  });
  test("a unit needs no given, an earlier result can be used again", () => {
    const check = { sum: [{ sum: "5:30 × 10", is: "55:00" }, { sum: "55:00 ÷ 60", is: "55" }], givens: GOOD.givens };
    expect(why(check, "At 5:30 a km, 10 km takes 55:00: 55 minutes, not 50.")).toBe("backed");
  });
  test("a given that is not in the note", () => {
    expect(why({ ...GOOD, givens: [...GOOD.givens, { value: "5:20", words: "hold 5:20" }] })).toBe("“hold 5:20” is not in the note");
  });
  test("a given whose words do not say it", () => {
    expect(why({ ...GOOD, givens: [{ value: "5:30", words: "hold 5:30 for the whole race" }, { value: "12", words: "10k" }] })).toBe(
      "“10k” does not say 12",
    );
  });
  test("no number from the corrected paragraph", () => {
    const check = { sum: [{ sum: "56:40 − 50:00", is: "6:40" }], givens: [{ value: "56:40", words: "10k in 56:40" }, { value: "50:00", words: "under 50 minutes" }] };
    expect(why(check, "You need to find 6:40.")).toBe("none of its numbers come from the paragraph it corrects");
  });
  test("a number in the text that the check does not show", () => {
    expect(why(GOOD, TEXT + " 4:55 leaves a little room.")).toBe("it says 4:55, which neither the note nor the check shows");
  });
  test("an empty or missing check", () => {
    expect(why(undefined)).toBe("it came without a check");
    expect(why({ sum: [], words: [] })).toBe("its check was empty");
  });
  test("the writer's numbers the model did not list are found in their words", () => {
    const r = backCorrection({ sum: GOOD.sum }, TEXT, writer);
    expect("check" in r && r.check.givens).toEqual([
      { value: "5:30", words: "5:30" },
      { value: "10", words: "10k" },
      { value: "50:00", words: "50" },
    ]);
    const minutes = backCorrection({ sum: [{ sum: "56:40 − 50:00", is: "6:40" }] }, "That is 6:40 faster than last year, a lot to find.", { ...writer, target: NOTE.split("\n")[0] });
    expect("check" in minutes && minutes.check.givens).toEqual([
      { value: "56:40", words: "56:40" },
      { value: "50:00", words: "50 minutes" },
    ]);
  });
});

describe("a correction shows the writer's own words", () => {
  const note = "I run four days a week, Monday to Thursday.\n\nWith three runs a week, Sunday stays free for the long run.";
  const w: Writer = { title: "Plan", note, brief: null, target: "With three runs a week, Sunday stays free for the long run.", quote: "three runs a week" };
  const text = "Earlier you planned four days a week. Which is it?";
  test("a passage from elsewhere, snapped onto the note's own characters", () => {
    const r = backCorrection({ words: ["“i RUN four days a week”"] }, text, w);
    expect(r).toEqual({ check: { sum: [], givens: [], words: ["I run four days a week"] } });
  });
  test("a list's dash is not part of the words", () => {
    expect(why({ words: ["- I run four days a week"] }, text, w)).toBe("backed");
  });
  test("the corrected words themselves are no check", () => {
    expect(why({ words: ["three runs a week"] }, text, w)).toBe("“three runs a week” is the corrected words themselves");
  });
  test("a passage that is not in the note", () => {
    expect(why({ words: ["I run five days a week"] }, text, w)).toBe("“I run five days a week” is not in the note");
  });
  test("one word is too short to check", () => {
    expect(why({ words: ["four"] }, text, w)).toBe("“four” is too short to check");
  });
});

describe("what leaves the brain", () => {
  test("a backed correction carries its check", () => {
    const t = withWorking(correction(), { check: GOOD, question: QUESTION }, writer);
    expect(t?.kind).toBe("correction");
    expect(t?.check?.sum.length).toBe(2);
  });
  test("a correction that cannot show its check becomes its question", () => {
    const logged: string[] = [];
    const t = withWorking(correction(), { check: { ...GOOD, sum: [{ sum: "5:30 × 10", is: "50:00" }] }, question: QUESTION }, writer, (w) => logged.push(w));
    expect(t).toEqual({ speak: true, kind: "question", quote: QUOTE, text: QUESTION, sketch: false });
    expect(logged).toEqual(["correction became a question: line 1: 5:30 × 10 is 55:00, not 50:00"]);
  });
  test("with no question to fall back on, nothing", () => {
    expect(withWorking(correction(), { question: "  " }, writer)).toBeNull();
  });
  test("a question passes through and drops any check the model sent", () => {
    const q: Thought = { speak: true, kind: "question", quote: QUOTE, text: "Why 50?", sketch: false, check: { sum: [], givens: [], words: [] } };
    expect(withWorking(q, { check: GOOD }, writer)).toEqual({ speak: true, kind: "question", quote: QUOTE, text: "Why 50?", sketch: false });
  });
});

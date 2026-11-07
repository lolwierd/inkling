// Show its working: a correction reaches the page only with a check the writer can do
// themselves, either a sum made from their own numbers or their own words from elsewhere
// in the note, a verified web source, or a verified local file. Unsupported remembered
// facts use questions. Everything here is pure: the brain, the fake brain and the tests
// all go through `withWorking`.

import { findQuote } from "./output.ts";
import type { Source } from "./web.ts";
import type { FileSource } from "./files.ts";

export interface SumLine {
  sum: string;
  is: string;
  rounded: boolean;
}

export interface Given {
  value: string;
  words: string;
}

export interface Check {
  sum: SumLine[];
  givens: Given[];
  words: string[];
  /** A page that says so, made sure of (web.ts `verifySource`); only with the web on. */
  source?: Source;
  /** A local file that says so, made sure of (files.ts `verifyFile`). */
  file?: FileSource;
}

/** What became of the source a correction cited: made sure of, or why not. */
export type Cited = { source: Source } | { why: string } | null;
/** What became of the file a correction cited: made sure of, or why not. */
export type Filed = { file: FileSource } | { why: string } | null;

/** The writer's own text a check may draw on, and what the correction is about. */
export interface Writer {
  title: string;
  note: string;
  brief: string | null;
  /** The paragraph (or selection) that was read. */
  target: string;
  /** The corrected words, already snapped onto the paragraph. */
  quote: string;
}

export interface Thought {
  speak: true;
  kind: "question" | "correction";
  quote: string;
  text: string;
  sketch: boolean;
  check?: Check;
}

/** Numbers a sum may use without taking them from the writer: units everyone knows. */
export const UNITS = [1, 2, 7, 12, 24, 52, 60, 100, 365, 1000];

const MAX_LINES = 4;
const MAX_GIVENS = 6;
const MAX_PASSAGES = 2;

// ----- quantities -------------------------------------------------------------

export type Kind = "number" | "duration" | "percent";

export interface Quantity {
  kind: Kind;
  /** Durations in their smallest written unit (5:30 is 330), percentages as fractions. */
  value: number;
  /** Half the last written unit: how far a rounded result may be from the true one. */
  half: number;
  /** A duration's first part (50 in 50:00), and whether every later part is zero. */
  lead: number;
  whole: boolean;
}

const CURRENCY = /^[$€£¥₹]/u;

/** One quantity as written: 5:30, 1:05:00, 1,200, 3.5, 15%, $40. `null` if it is not one. */
export function parseQuantity(raw: string): Quantity | null {
  let s = raw.trim().replace(/^(?:≈|~|about\s+)/iu, "").trim();
  let sign = 1;
  if (/^[-−–]/u.test(s)) {
    sign = -1;
    s = s.slice(1).trim();
  }
  s = s.replace(CURRENCY, "");
  let m = /^(\d+)((?::\d{2}){1,2})$/u.exec(s);
  if (m) {
    const parts = [m[1], ...m[2].slice(1).split(":")].map(Number);
    if (parts.slice(1).some((p) => p >= 60)) return null;
    const value = parts.reduce((acc, p) => acc * 60 + p, 0);
    return { kind: "duration", value: sign * value, half: 0.5, lead: parts[0], whole: parts.slice(1).every((p) => p === 0) };
  }
  m = /^(\d{1,3}(?:,\d{3})+|\d+)(?:\.(\d+))?(%)?$/u.exec(s);
  if (m) {
    const decimals = m[2]?.length ?? 0;
    const n = Number(m[1].replace(/,/g, "") + (m[2] ? "." + m[2] : ""));
    const half = 0.5 * 10 ** -decimals;
    if (m[3]) return { kind: "percent", value: (sign * n) / 100, half: half / 100, lead: n, whole: true };
    return { kind: "number", value: sign * n, half, lead: n, whole: true };
  }
  return null;
}

function near(a: number, b: number): boolean {
  return Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(a), Math.abs(b));
}

// ----- sums ---------------------------------------------------------------------

interface Token {
  op?: "+" | "-" | "×" | "÷" | "(" | ")";
  q?: Quantity;
  text?: string;
}

const QUANTITY = /^[$€£¥₹]?(?:\d+(?::\d{2}){1,2}|(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?%?)/u;
const OPS: Record<string, Token["op"]> = { "+": "+", "-": "-", "−": "-", "–": "-", "×": "×", "*": "×", "·": "×", "÷": "÷", "/": "÷", "(": "(", ")": ")" };

function tokens(sum: string): Token[] | string {
  const out: Token[] = [];
  let i = 0;
  while (i < sum.length) {
    const ch = sum[i];
    if (/\s/u.test(ch)) {
      i++;
      continue;
    }
    // "x" between two quantities is a times sign: 5:30 x 10
    if ((ch === "x" || ch === "X") && out.length && (out[out.length - 1].q || out[out.length - 1].op === ")")) {
      out.push({ op: "×" });
      i++;
      continue;
    }
    if (OPS[ch]) {
      out.push({ op: OPS[ch] });
      i++;
      continue;
    }
    const m = QUANTITY.exec(sum.slice(i));
    const q = m && parseQuantity(m[0]);
    if (!m || !q) return `“${sum}” has something that is not a number or a sign`;
    out.push({ q, text: m[0] });
    i += m[0].length;
  }
  return out.length ? out : "an empty sum";
}

/** The value of a sum and the quantities it was made from, or why it cannot be worked out. */
export function evaluate(sum: string): { value: number; operands: { q: Quantity; text: string }[]; shown: string } | { error: string } {
  const ts = tokens(sum);
  if (typeof ts === "string") return { error: ts };
  let at = 0;
  const operands: { q: Quantity; text: string }[] = [];
  const fail = (why: string): never => {
    throw new Error(why);
  };
  const expr = (): number => {
    let v = term();
    while (ts[at]?.op === "+" || ts[at]?.op === "-") {
      const op = ts[at++].op;
      const r = term();
      v = op === "+" ? v + r : v - r;
    }
    return v;
  };
  const term = (): number => {
    let v = factor();
    while (ts[at]?.op === "×" || ts[at]?.op === "÷") {
      const op = ts[at++].op;
      const r = factor();
      if (op === "÷" && r === 0) fail("it divides by zero");
      v = op === "×" ? v * r : v / r;
    }
    return v;
  };
  const factor = (): number => {
    const t = ts[at++];
    if (!t) return fail(`“${sum}” ends too soon`);
    if (t.op === "-") return -factor();
    if (t.op === "(") {
      const v = expr();
      if (ts[at++]?.op !== ")") fail(`“${sum}” has a bracket that is not closed`);
      return v;
    }
    if (t.q) {
      operands.push({ q: t.q, text: t.text! });
      return t.q.value;
    }
    return fail(`“${sum}” has a sign where a number should be`);
  };
  try {
    const value = expr();
    if (at < ts.length) return { error: `“${sum}” has more after the sum ends` };
    if (!Number.isFinite(value)) return { error: `“${sum}” does not come to a number` };
    return { value, operands, shown: shown(ts) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

/** The sum as the tag sets it: proper signs, one space around each. */
function shown(ts: Token[]): string {
  let out = "";
  ts.forEach((t, i) => {
    const prev = ts[i - 1];
    const unary = t.op === "-" && (!prev || (prev.op && prev.op !== ")"));
    if (t.q) out += t.text;
    else if (t.op === "(" || t.op === ")") out += t.op;
    else if (unary) out += "−";
    else out += ` ${t.op === "-" ? "−" : t.op} `;
  });
  return out.replace(/\s+/gu, " ").trim();
}

// ----- numbers in the writer's words ------------------------------------------------

interface Mention {
  kind: Kind;
  /** Every value the words could mean: "10k" is 10 (a 10k race) or 10,000. */
  values: number[];
  /** Where it is in the text it was found in, and how long it is. */
  at: number;
  len: number;
}

const WORDS: Record<string, number> = {
  zero: 0, one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7, eight: 8, nine: 9, ten: 10,
  eleven: 11, twelve: 12, thirteen: 13, fourteen: 14, fifteen: 15, sixteen: 16, seventeen: 17, eighteen: 18,
  nineteen: 19, twenty: 20, thirty: 30, forty: 40, fifty: 50, sixty: 60, seventy: 70, eighty: 80, ninety: 90,
  hundred: 100, thousand: 1000, dozen: 12, half: 0.5, quarter: 0.25, once: 1, twice: 2, double: 2, triple: 3,
};

const MENTION =
  /(?<![\p{L}\d:.,])(\d+(?::\d{2}){1,2})(?![\d:])|(?<![\p{L}\d:.,])(\d{1,3}(?:,\d{3})+|\d+)(?:\.(\d+))?(?:\s*(%|percent\b|per cent\b)|([kK])\b)?(?![\d:])|\b([A-Za-z]+)\b/gu;

/** Every number mentioned in `text`, in digits or in words. */
export function mentions(text: string, digitsOnly = false): Mention[] {
  const out: Mention[] = [];
  for (const m of text.matchAll(MENTION)) {
    const at = m.index ?? 0;
    const len = m[0].length;
    if (m[1]) {
      const q = parseQuantity(m[1]);
      if (q) out.push({ kind: "duration", values: [q.value], at, len });
    } else if (m[2]) {
      const n = Number(m[2].replace(/,/g, "") + (m[3] ? "." + m[3] : ""));
      if (m[4]) out.push({ kind: "percent", values: [n / 100], at, len });
      else if (m[5]) out.push({ kind: "number", values: [n, n * 1000], at, len });
      else out.push({ kind: "number", values: [n], at, len });
    } else if (m[6] && !digitsOnly) {
      const w = WORDS[m[6].toLowerCase()];
      if (w !== undefined) out.push({ kind: "number", values: [w], at, len });
    }
  }
  return out;
}

/** Do the writer's words contain this given's value? */
function fits(q: Quantity, ms: Mention[]): boolean {
  return ms.some((m) => {
    if (q.kind === "duration") {
      if (m.kind === "duration") return m.values.some((v) => near(v, q.value));
      // 50:00 is the "50" in "under 50 minutes"
      return m.kind === "number" && q.whole && m.values.some((v) => near(v, q.lead));
    }
    if (q.kind === "percent") return m.kind === "percent" ? m.values.some((v) => near(v, q.value)) : false;
    return m.kind === "number" && m.values.some((v) => near(v, q.value));
  });
}

const STOP = new Set(["a", "an", "the", "per", "of", "in", "on", "at", "to", "for", "and", "or", "by", "is", "it"]);

/**
 * The writer's own words for a number the sum uses but the model did not list as a given:
 * the first mention of it (in the corrected paragraph first), with the word after it when
 * that word says what it counts ("50 minutes"). `null` when the writer never wrote it.
 */
export function findGiven(q: Quantity, w: Writer): { words: string; inTarget: boolean } | null {
  for (const [source, inTarget] of [[w.target, true], [w.note, false], [w.title, false], [w.brief ?? "", false]] as const) {
    for (const m of mentions(source)) {
      if (!fits(q, [m])) continue;
      let end = m.at + m.len;
      const next = /^\s+([\p{L}]+)/u.exec(source.slice(end));
      if (next && /^\d/u.test(source.slice(m.at)) && !/[kK%]$/u.test(source.slice(m.at, end)) && !STOP.has(next[1].toLowerCase()) && next[1].length <= 10) {
        end += next[0].length;
      }
      return { words: source.slice(m.at, end), inTarget };
    }
  }
  return null;
}

// ----- the check -------------------------------------------------------------------

type Json = Record<string, unknown>;

const isObject = (v: unknown): v is Json => typeof v === "object" && v !== null && !Array.isArray(v);
const str = (v: unknown): string => (typeof v === "string" ? v.trim() : "");
const list = (v: unknown): unknown[] => (Array.isArray(v) ? v : []);

/** A passage as the model may wrap it: quote marks, a list's dash, a heading's #. */
function bare(passage: string): string {
  return passage.trim().replace(/^(?:[#>]+\s*|[-*]\s+(?:\[[ xX]\]\s+)?|\d+\.\s+)/u, "").trim();
}

function inWriter(words: string, w: Writer): string | null {
  for (const source of [w.note, w.title, w.brief ?? ""]) {
    const found = source ? findQuote(words, source) : null;
    if (found) return found;
  }
  return null;
}

/**
 * The check a correction carries, made sure of: every line of its sum adds up, every
 * number in it is the writer's, an earlier result or a unit, and every passage is the
 * writer's own, from elsewhere in the note. Returns why not, otherwise.
 */
export function backCorrection(raw: unknown, text: string, w: Writer, cited: Cited = null, filed: Filed = null): { check: Check } | { why: string } {
  if (!isObject(raw)) return { why: "it came without a check" };
  const lines = list(raw.sum).filter(isObject);
  const passages = list(raw.words).map(str).filter(Boolean);
  const source = cited && "source" in cited ? cited.source : null;
  const file = filed && "file" in filed ? filed.file : null;
  if (!lines.length && !passages.length && !source && !file) {
    const why = (cited && "why" in cited ? cited.why : null) ?? (filed && "why" in filed ? filed.why : null);
    return { why: why ?? "its check was empty" };
  }
  const check: Check = { sum: [], givens: [], words: [] };
  // numbers the correction's text may use: the writer's, the check's, the source's and the file's
  const shownValues: number[] = mentions([w.title, w.note, w.brief ?? "", w.target, source?.passage ?? "", file?.passage ?? ""].join("\n")).flatMap((m) => m.values);
  if (source) check.source = source;
  if (file) check.file = file;

  if (lines.length) {
    if (lines.length > MAX_LINES) return { why: `its sum has ${lines.length} lines` };
    const givens = list(raw.givens).filter(isObject).slice(0, MAX_GIVENS);
    const known: number[] = [];
    let fromParagraph = false;
    for (const g of givens) {
      const value = str(g.value);
      const q = parseQuantity(value);
      if (!q) return { why: `the given “${value}” is not a number` };
      const words = inWriter(str(g.words), w);
      if (!words) return { why: `“${str(g.words)}” is not in the note` };
      if (!fits(q, mentions(words))) return { why: `“${words}” does not say ${value}` };
      if (findQuote(words, w.target)) fromParagraph = true;
      known.push(q.value);
      shownValues.push(q.value);
      check.givens.push({ value, words });
    }
    for (const [i, line] of lines.entries()) {
      const n = i + 1;
      const got = evaluate(str(line.sum));
      if ("error" in got) return { why: `line ${n}: ${got.error}` };
      const is = parseQuantity(str(line.is));
      if (!is) return { why: `line ${n}: “${str(line.is)}” is not a number` };
      for (const o of got.operands) {
        if (known.some((k) => near(k, o.q.value)) || (o.q.kind === "number" && UNITS.includes(o.q.value))) continue;
        // one of the writer's numbers the model did not list: find their words for it
        const found = check.givens.length < MAX_GIVENS ? findGiven(o.q, w) : null;
        if (!found) return { why: `line ${n}: ${o.text} is not in the note, not an earlier result and not a unit` };
        if (found.inTarget) fromParagraph = true;
        known.push(o.q.value);
        shownValues.push(o.q.value);
        check.givens.push({ value: o.text, words: found.words });
      }
      const off = Math.abs(got.value - is.value);
      if (off > is.half + 1e-9 * Math.max(1, Math.abs(is.value))) {
        return { why: `line ${n}: ${got.shown} is ${describe(got.value, is)}, not ${str(line.is)}` };
      }
      known.push(is.value, got.value);
      shownValues.push(is.value, got.value, ...got.operands.map((o) => o.q.value));
      check.sum.push({ sum: got.shown, is: str(line.is).replace(/^(?:≈|~|about\s+)/iu, "").trim(), rounded: !near(got.value, is.value) });
    }
    if (!fromParagraph) return { why: "none of its numbers come from the paragraph it corrects" };
  }

  if (passages.length) {
    if (passages.length > MAX_PASSAGES) return { why: `it quotes ${passages.length} passages` };
    for (const p of passages) {
      const words = bare(p);
      if (words.includes("\n")) return { why: "a passage runs over more than one line" };
      if (words.split(/\s+/u).filter(Boolean).length < 2) return { why: `“${words}” is too short to check` };
      const found = findQuote(words, w.note) ?? findQuote(words, w.title);
      if (!found) return { why: `“${words}” is not in the note` };
      if (w.quote.includes(found) || found.includes(w.quote)) return { why: `“${found}” is the corrected words themselves` };
      check.words.push(found);
    }
  }

  // The correction says nothing its check does not show.
  for (const m of mentions(text, true)) {
    if (m.kind === "number" && m.values[0] <= 12 && Number.isInteger(m.values[0])) continue;
    if (!m.values.some((v) => shownValues.some((s) => near(s, v)))) {
      return { why: `it says ${formatMention(m)}, which neither the note nor the check shows` };
    }
  }
  return { check };
}

function describe(value: number, like: Quantity): string {
  if (like.kind === "duration") {
    const sign = value < 0 ? "−" : "";
    let v = Math.round(Math.abs(value));
    const parts: number[] = [];
    while (v >= 60 && parts.length < 2) {
      parts.unshift(v % 60);
      v = Math.floor(v / 60);
    }
    parts.unshift(v);
    if (parts.length === 1) parts.push(0);
    return sign + parts.map((p, i) => (i === 0 ? String(p) : String(p).padStart(2, "0"))).join(":");
  }
  if (like.kind === "percent") return `${+(value * 100).toFixed(4)}%`;
  return String(+value.toFixed(6));
}

function formatMention(m: Mention): string {
  const v = m.values[0];
  if (m.kind === "percent") return `${+(v * 100).toFixed(4)}%`;
  if (m.kind === "duration") return describe(v, { kind: "duration", value: v, half: 0.5, lead: 0, whole: false });
  return String(v);
}

/**
 * What a read or probe sends the app. A question passes through (without any check the
 * model sent). A correction goes out with its check, or comes back as its fallback
 * question, or `null` when it has neither: a read then stays silent and a probe fails.
 */
export function withWorking(thought: Thought, answer: Json, w: Writer, log: (why: string) => void = () => {}, cited: Cited = null, filed: Filed = null): Thought | null {
  const { check: _ignored, ...plain } = thought;
  if (thought.kind !== "correction") return plain;
  if (cited && "why" in cited) log(`a source was dropped: ${cited.why}`);
  if (filed && "why" in filed) log(`a file was dropped: ${filed.why}`);
  const backed = backCorrection(answer.check, thought.text, w, cited, filed);
  if ("check" in backed) return { ...plain, check: backed.check };
  log(`correction became a question: ${backed.why}`);
  const question = str(answer.question);
  if (!question) return null;
  return { ...plain, kind: "question", text: question };
}

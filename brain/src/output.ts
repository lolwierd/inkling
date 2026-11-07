// Shaping what the model returns into what the app can use: quotes that are
// exact substrings of the writer's text, SVG the app's XML parser accepts, and
// strings cut on character boundaries.

import { ProviderError } from "./claude.ts";

const EDGE_MARKS = /^[\s«»"“”„'‘’‹›`「」『』…]+|[\s«»"“”„'‘’‹›`「」『』…]+$/gu;

/**
 * Snap a model-provided quote onto the exact text it should come from. The
 * result is always a substring of `source` (the whole of it when nothing
 * matches), because the app anchors the note by searching for it verbatim.
 */
export function snapQuote(quote: string, source: string): string {
  return findQuote(quote, source) ?? source;
}

/**
 * Like `snapQuote`, but `null` when the quote is not in `source` at all: for a quote that
 * must not fall back to the whole text (a check's words, the far end of a thread between
 * notes), which the model could otherwise invent.
 */
export function findQuote(quote: string, source: string): string | null {
  const q = quote.replace(EDGE_MARKS, "");
  if (!q) return null;
  if (source.includes(q)) return q;
  // Looser matches (case, whitespace, curly quotes, dashes), mapped back onto
  // the source's own characters; then without trailing punctuation.
  for (const candidate of [q, q.replace(/[\s.,;:!?…]+$/u, "")]) {
    const span = looseFind(source, candidate);
    if (span) return source.slice(span[0], span[1]);
  }
  // "first words … last words": from the first piece to the last.
  const pieces = q.split(/\s*(?:\.\.\.|…)\s*/u).filter((p) => p.trim());
  if (pieces.length > 1) {
    const a = looseFind(source, pieces[0]);
    const b = a && looseFind(source, pieces[pieces.length - 1], a[1]);
    if (a && b) return source.slice(a[0], b[1]);
  }
  return null;
}

function foldChar(ch: string): string {
  if (/\s/u.test(ch)) return " ";
  switch (ch) {
    case "‘":
    case "’":
    case "‚":
    case "‛":
      return "'";
    case "“":
    case "”":
    case "„":
    case "«":
    case "»":
      return '"';
    case "‐":
    case "‑":
    case "‒":
    case "–":
    case "—":
    case "−":
      return "-";
    case "…":
      return "...";
  }
  return ch.normalize("NFKC").toLowerCase();
}

/** `text` is `s` folded for loose matching; `map[i]` is the index in `s` that produced `text[i]`. */
function fold(s: string, from = 0): { text: string; map: number[] } {
  let text = "";
  const map: number[] = [];
  let i = 0;
  for (const ch of s) {
    const at = i;
    i += ch.length;
    if (at < from) continue;
    const f = foldChar(ch);
    if (f === " " && (text === "" || text.endsWith(" "))) continue;
    text += f;
    for (let k = 0; k < f.length; k++) map.push(at);
  }
  return { text, map };
}

function looseFind(source: string, q: string, from = 0): [number, number] | null {
  const needle = fold(q).text.trim();
  if (!needle) return null;
  const hay = fold(source, from);
  const j = hay.text.indexOf(needle);
  if (j < 0) return null;
  const start = hay.map[j];
  const last = hay.map[j + needle.length - 1];
  const end = last + ((source.codePointAt(last) ?? 0) > 0xffff ? 2 : 1);
  return [start, end];
}

/** Cut `s` to at most `max` characters without splitting a surrogate pair. */
export function clip(s: string, max: number): string {
  const chars = Array.from(s);
  return chars.length <= max ? s : chars.slice(0, max).join("").trimEnd();
}

// HTML names the model sometimes writes; XML only knows amp, lt, gt, quot and apos.
const HTML_ENTITIES: Record<string, string> = {
  nbsp: " ",
  rarr: "→",
  larr: "←",
  harr: "↔",
  uarr: "↑",
  darr: "↓",
  rArr: "⇒",
  lArr: "⇐",
  hArr: "⇔",
  mdash: "—",
  ndash: "–",
  hellip: "…",
  times: "×",
  divide: "÷",
  middot: "·",
  bull: "•",
  deg: "°",
  plusmn: "±",
  le: "≤",
  ge: "≥",
  ne: "≠",
  asymp: "≈",
  infin: "∞",
  lsquo: "‘",
  rsquo: "’",
  ldquo: "“",
  rdquo: "”",
  laquo: "«",
  raquo: "»",
  copy: "©",
  euro: "€",
  pound: "£",
  micro: "µ",
  alpha: "α",
  beta: "β",
  gamma: "γ",
  delta: "δ",
  Delta: "Δ",
  lambda: "λ",
  mu: "μ",
  pi: "π",
  sigma: "σ",
  Sigma: "Σ",
  theta: "θ",
  check: "✓",
};

/**
 * Pull the drawing out of the model's text and make it well-formed XML: the
 * first complete <svg>…</svg>, without comments, scripts, styles or event
 * handlers, with HTML entities turned into characters and stray `&` and `<`
 * escaped. The app parses it with a strict XML parser.
 */
export function cleanSvg(raw: string): string {
  const open = /<svg[\s>]/i.exec(raw);
  if (!open) throw new ProviderError("bad_output", "the drawing did not come back as SVG");
  const tags = /<svg[\s>]|<\/svg\s*>/gi;
  tags.lastIndex = open.index;
  let depth = 0;
  let end = -1;
  for (let m = tags.exec(raw); m; m = tags.exec(raw)) {
    if (m[0].startsWith("</")) {
      depth -= 1;
      if (depth === 0) {
        end = m.index + m[0].length;
        break;
      }
    } else {
      depth += 1;
    }
  }
  if (end < 0) throw new ProviderError("bad_output", "the drawing was cut off before </svg>");
  return raw
    .slice(open.index, end)
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/<(script|style|foreignObject)\b[\s\S]*?<\/\1\s*>/gi, "")
    .replace(/<(script|style|foreignObject|image)\b[^>]*\/>/gi, "")
    .replace(/\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*')/gi, "")
    .replace(/&(?!(?:amp|lt|gt|quot|apos|#\d+|#x[0-9a-fA-F]+);)(?:([A-Za-z][A-Za-z0-9]*);)?/g, (_m, name?: string) => {
      if (!name) return "&amp;";
      return HTML_ENTITIES[name] ?? HTML_ENTITIES[name.toLowerCase()] ?? `&amp;${name};`;
    })
    .replace(/<(?![A-Za-z/!?])/g, "&lt;");
}

/**
 * The threads a read says its paragraph answers, as the app can use them: each thread's id
 * (from its number in the context's margin), only threads still open, each once, the
 * words snapped onto the paragraph, at most three.
 */
export function pickAnswers(
  raw: unknown,
  threads: { id?: string; resolved: boolean; open?: boolean; own?: boolean }[],
  target: string,
): { thread: string; quote: string }[] {
  if (!Array.isArray(raw)) return [];
  const out: { thread: string; quote: string }[] = [];
  for (const a of raw) {
    if (typeof a !== "object" || a === null) continue;
    const n = Number((a as Record<string, unknown>).thread);
    const t = Number.isInteger(n) && n >= 1 ? threads[n - 1] : undefined;
    if (!t || !t.id || !(t.open ?? !t.resolved) || t.own || out.some((o) => o.thread === t.id)) continue;
    const q = (a as Record<string, unknown>).quote;
    out.push({ thread: t.id, quote: snapQuote(typeof q === "string" ? q : "", target) });
    if (out.length === 3) break;
  }
  return out;
}

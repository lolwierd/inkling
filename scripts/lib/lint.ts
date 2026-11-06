// The owner's rule (design/directions/DESIGN.md, "Words"): none of Riddled's verbs or
// names anywhere in the app or around it. This checks every string literal in the app
// (src/) and the brain (brain/src/), which is where words reach the screen or the
// model, plus README.md. Identifiers and comments are not checked; old stored values
// that must keep their spelling are listed in ALLOWED with the reason.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { REPO, SCENARIOS } from "./app.ts";

export const BANNED: { re: RegExp; use: string }[] = [
  { re: /riddl/i, use: "Find a loose thread (the action), Socratic (the voice), mind (the reader)" },
  { re: /visuali[sz]/i, use: "Sketch it" },
  { re: /thinks with you/i, use: "Reads along" },
  // The old mode's label. Lower case it is ordinary English ("it answers only when you
  // ask"), so only the label form counts.
  { re: /\bWhen you ask\b/, use: "Waits to be asked" },
  { re: /just my words/i, use: "Hide threads" },
];

/** Literals that keep an old spelling on purpose: `file` relative to the repo. */
const ALLOWED: { file: string; literal: string; why: string }[] = [
  { file: "src/store.rs", literal: "riddle", why: "the stored voice value in old minds.json files (shown as Socratic)" },
  { file: "brain/src/protocol.ts", literal: "riddle", why: "the voice value on the wire" },
  { file: "brain/src/main.ts", literal: "riddle", why: "the voice value on the wire" },
  { file: "src/drawing.rs", literal: "riddle", why: "a colour name older drawings use" },
  { file: "src/script.rs", literal: "riddle", why: "old name of the `probe` script step, still read" },
  { file: "src/script.rs", literal: "visualize", why: "old name of the `sketch` script step, still read" },
  { file: "src/note_page.rs", literal: "tb-riddle", why: "an element id, never shown" },
];

interface Literal {
  line: number;
  text: string;
}

/** String literals in Rust or TypeScript source, with the line each starts on. Skips
 *  comments and char literals; good enough for finding words, not a parser. */
export function literals(src: string, lang: "rs" | "ts"): Literal[] {
  const out: Literal[] = [];
  let i = 0;
  let line = 1;
  const n = src.length;
  const adv = (k = 1) => {
    for (let j = 0; j < k && i < n; j++, i++) if (src[i] === "\n") line++;
  };
  while (i < n) {
    const c = src[i];
    const next = src[i + 1];
    if (c === "/" && next === "/") {
      while (i < n && src[i] !== "\n") adv();
    } else if (c === "/" && next === "*") {
      adv(2);
      while (i < n && !(src[i] === "*" && src[i + 1] === "/")) adv();
      adv(2);
    } else if (lang === "rs" && c === "r" && (next === '"' || next === "#") && !/[A-Za-z0-9_]/.test(src[i - 1] ?? "")) {
      // raw string r"..." or r#"..."#
      let j = i + 1;
      let hashes = 0;
      while (src[j] === "#") (hashes++, j++);
      if (src[j] !== '"') {
        adv();
        continue;
      }
      const start = line;
      adv(j - i + 1);
      const end = '"' + "#".repeat(hashes);
      let text = "";
      while (i < n && src.slice(i, i + end.length) !== end) (text += src[i], adv());
      adv(end.length);
      out.push({ line: start, text });
    } else if (lang === "rs" && c === "'") {
      // a char literal ('x', '\n', '\u{2019}') or a lifetime ('a): skip either
      const m = /^'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'/u.exec(src.slice(i, i + 12));
      adv(m ? m[0].length : 1);
    } else if (c === '"' || (lang === "ts" && (c === "'" || c === "`"))) {
      const quote = c;
      const start = line;
      adv();
      let text = "";
      while (i < n && src[i] !== quote) {
        if (src[i] === "\\") {
          text += src[i + 1] ?? "";
          adv(2);
        } else {
          text += src[i];
          adv();
        }
      }
      adv();
      out.push({ line: start, text });
    } else {
      adv();
    }
  }
  return out;
}

export interface Finding {
  file: string;
  line: number;
  text: string;
  word: string;
  use: string;
}

function files(dir: string, ext: string): string[] {
  return readdirSync(join(REPO, dir), { recursive: true })
    .map(String)
    .filter((f) => f.endsWith(ext))
    .map((f) => join(dir, f));
}

export function lintWords(): Finding[] {
  const found: Finding[] = [];
  const check = (file: string, line: number, text: string) => {
    for (const b of BANNED) {
      const m = b.re.exec(text);
      if (!m) continue;
      if (ALLOWED.some((a) => a.file === file && a.literal === text)) continue;
      found.push({ file, line, text: text.length > 120 ? text.slice(0, 117) + "..." : text, word: m[0], use: b.use });
    }
  };
  for (const file of files("src", ".rs")) for (const l of literals(readFileSync(join(REPO, file), "utf8"), "rs")) check(file, l.line, l.text);
  for (const file of files("brain/src", ".ts")) for (const l of literals(readFileSync(join(REPO, file), "utf8"), "ts")) check(file, l.line, l.text);
  readFileSync(join(REPO, "README.md"), "utf8")
    .split("\n")
    .forEach((text, i) => check("README.md", i + 1, text));
  return found;
}

// ----- the map and the scenarios -----------------------------------------------------

/** AGENTS.md is the map agents start from: every path it names must exist, and every
 *  module must be on it, or the next agent starts from a wrong picture. */
export function lintMap(): string[] {
  const text = readFileSync(join(REPO, "AGENTS.md"), "utf8");
  const out: string[] = [];
  for (const m of text.matchAll(/`((?:src|brain|scripts|docs|design)\/[\w./-]+)`/g)) {
    const p = m[1].replace(/[.,:]+$/, "");
    if (!existsSync(join(REPO, p))) out.push(`AGENTS.md names ${p}, which does not exist`);
  }
  for (const f of readdirSync(join(REPO, "src")).filter((f) => f.endsWith(".rs"))) if (!text.includes(`src/${f}`)) out.push(`src/${f} is not on the map in AGENTS.md`);
  for (const f of readdirSync(join(REPO, "brain/src")).filter((f) => f.endsWith(".ts"))) if (!text.includes(f)) out.push(`brain/src/${f} is not on the map in AGENTS.md`);
  return out;
}

/** Step names the app's script parser accepts (the match arms of `parse` in src/script.rs). */
export function stepNames(): Set<string> {
  const src = readFileSync(join(REPO, "src/script.rs"), "utf8");
  const body = src.slice(src.indexOf("pub fn parse"), src.indexOf("#[cfg(test)]"));
  const names = new Set<string>();
  for (const arm of body.matchAll(/^\s*((?:"[\w-]+"\s*\|\s*)*"[\w-]+")\s*=>/gm)) for (const n of arm[1].matchAll(/"([\w-]+)"/g)) names.add(n[1]);
  return names;
}

/** Scenario lines the app would skip with a warning, and `expect:` lines that do not
 *  parse as JavaScript: found before any window opens. */
export function lintScenarios(files?: string[]): string[] {
  const known = stepNames();
  const out: string[] = [];
  const list = files ?? readdirSync(SCENARIOS).filter((f) => f.endsWith(".txt")).map((f) => join(SCENARIOS, f));
  for (const file of list) {
    readFileSync(file, "utf8")
      .split("\n")
      .forEach((raw, i) => {
        const l = raw.trim();
        if (!l || l.startsWith("#")) return;
        const colon = l.indexOf(":");
        const cmd = (colon < 0 ? l : l.slice(0, colon)).trim();
        const where = `${relative(REPO, file)}:${i + 1}`;
        if (!known.has(cmd)) out.push(`${where}: unknown step "${cmd}" (steps: src/script.rs)`);
        if (cmd === "expect") {
          try {
            new Function("d", `return (\n${l.slice(colon + 1).trim()}\n);`);
          } catch (e) {
            out.push(`${where}: expect does not parse: ${e instanceof Error ? e.message : e}`);
          }
        }
      });
  }
  return out;
}

export function runLint(json: boolean): number {
  const found = lintWords();
  const map = lintMap();
  const scenarios = lintScenarios();
  const ok = found.length + map.length + scenarios.length === 0;
  if (json) {
    console.log(JSON.stringify({ ok, words: found, map, scenarios }, null, 1));
    return ok ? 0 : 1;
  }
  for (const f of found) console.log(`${relative(REPO, join(REPO, f.file))}:${f.line}: "${f.word}" in ${JSON.stringify(f.text)} -> say: ${f.use}`);
  if (found.length) console.log(`  The rule: design/directions/DESIGN.md, "Words". A spelling that must stay goes in ALLOWED in scripts/lib/lint.ts, with the reason.`);
  for (const m of [...map, ...scenarios]) console.log(m);
  console.log(ok ? "lint: words, map and scenarios ok" : `lint: ${found.length} banned word(s), ${map.length} map problem(s), ${scenarios.length} scenario problem(s)`);
  return ok ? 0 : 1;
}

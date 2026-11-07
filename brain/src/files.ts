// Corrections backed by a file: a correction may cite a local file it opened, as it may
// cite a web page, and the brain opens the file itself to make sure the quoted words are
// there before the citation reaches the page. Like web.ts for pages.

import { readFileSync, statSync } from "node:fs";
import { isAbsolute, normalize } from "node:path";
import { findQuote } from "./output.ts";

/** A local file a correction cites, made sure of. */
export interface FileSource {
  /** The file, as an absolute path. */
  path: string;
  /** The line the quoted words start on, counted from 1. */
  line: number;
  /** The words quoted from it, as they are in the file. */
  passage: string;
}

/** The largest file the brain opens to find a passage. */
const MAX_BYTES = 2 * 1024 * 1024;

/** A file's text, or null when it cannot be read or is too large. */
export type ReadFile = (path: string) => string | null;

function readSmall(path: string): string | null {
  try {
    if (statSync(path).size > MAX_BYTES) return null;
    return readFileSync(path, "utf8");
  } catch {
    return null;
  }
}

/**
 * The file a correction cites, made sure of: one this call opened with Read, and its
 * passage (5 to 40 words) is in it. The line is counted here, not taken from the model.
 * Returns why not, otherwise.
 */
export function verifyFile(raw: unknown, opened: string[], read: ReadFile = readSmall): { file: FileSource } | { why: string } {
  if (typeof raw !== "object" || raw === null) return { why: "it cites no file" };
  const path = typeof (raw as any).path === "string" ? (raw as any).path.trim() : "";
  const passage = typeof (raw as any).passage === "string" ? (raw as any).passage.trim() : "";
  if (!path || !isAbsolute(path)) return { why: "its file has no absolute path" };
  const at = normalize(path);
  if (!opened.some((p) => normalize(p) === at)) return { why: `${at} was not opened in this answer` };
  const words = passage.split(/\s+/u).filter(Boolean).length;
  if (words < 5 || words > 40) return { why: `the passage it quotes from ${at} has ${words} words` };
  const text = read(at);
  if (text === null) return { why: `${at} could not be read` };
  const inFile = findQuote(passage, text);
  if (!inFile) return { why: `the passage is not in ${at}` };
  const line = text.slice(0, Math.max(0, text.indexOf(inFile))).split("\n").length;
  return { file: { path: at, line, passage: inFile.replace(/\s+/gu, " ").trim() } };
}

/** What a call looked at, for the line under its answer: the files it opened, then the
 * pages it fetched, each once, at most `max`. */
export function lookedAt(files: string[], pages: string[], max = 8): string[] {
  return [...new Set([...files, ...pages])].slice(0, max);
}

// Sources from the web, only when real. With "Look things up on the web" on, the model
// may search (the SDK's WebSearch) and read pages (WebFetch). A correction may then cite a
// page, and the citation reaches the page only if its URL came back from a search in that
// same call and the quoted passage is on the page, which the brain opens itself to look.
// URLs in free text that no search returned are taken out. Tests swap the page fetcher;
// nothing here touches the network in a test.

import { findQuote } from "./output.ts";

/** The tasks that may look things up when the web is on; sketches, drawings and learning never do. */
export const WEB_TASKS: ReadonlySet<string> = new Set(["read", "probe", "ask", "answer", "reply", "chat"]);

/** A search result the model saw in this call. */
export interface Found {
  url: string;
  title: string;
}

/** A page a correction cites, made sure of. */
export interface Source {
  url: string;
  /** The page's title, as the search gave it. */
  title: string;
  /** Where it is: the host, without "www.". */
  domain: string;
  /** The words quoted from it, as they are on the page. */
  passage: string;
}

/** Search results in one tool output, if it is a WebSearch output. */
export function foundIn(output: unknown): Found[] {
  if (typeof output !== "object" || output === null || !Array.isArray((output as any).results)) return [];
  const out: Found[] = [];
  for (const r of (output as any).results) {
    if (typeof r !== "object" || r === null || !Array.isArray(r.content)) continue;
    for (const c of r.content) {
      if (c && typeof c.url === "string" && /^https?:\/\//i.test(c.url)) out.push({ url: c.url, title: typeof c.title === "string" ? c.title.trim() : "" });
    }
  }
  return out;
}

/** A URL compared loosely: no fragment, no trailing slash, the host in lower case. */
export function sameUrl(a: string, b: string): boolean {
  const norm = (u: string) => {
    try {
      const x = new URL(u.trim());
      x.hash = "";
      return `${x.protocol}//${x.host.toLowerCase()}${x.pathname.replace(/\/+$/, "")}${x.search}`;
    } catch {
      return u.trim();
    }
  };
  return norm(a) === norm(b);
}

export function domainOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./i, "");
  } catch {
    return "";
  }
}

const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " ", rsquo: "’", lsquo: "‘", rdquo: "”", ldquo: "“", ndash: "–", mdash: "—", hellip: "…", deg: "°", times: "×", minus: "−" };

/** The words of an HTML page, as a reader sees them: no scripts, styles or tags. */
export function pageText(html: string): string {
  return html
    .replace(/<(script|style|noscript|template|svg)\b[\s\S]*?<\/\1\s*>/gi, " ")
    .replace(/<!--[\s\S]*?-->/g, " ")
    .replace(/<(br|p|div|li|tr|h[1-6]|td|th|section|article)\b[^>]*>/gi, "\n")
    .replace(/<[^>]+>/g, " ")
    .replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (m, e: string) => {
      if (e[0] === "#") {
        const code = e[1] === "x" || e[1] === "X" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
        return Number.isFinite(code) && code > 0 && code < 0x110000 ? String.fromCodePoint(code) : " ";
      }
      return ENTITIES[e.toLowerCase()] ?? m;
    })
    .replace(/[ \t\r\f\v ]+/g, " ")
    .replace(/\s*\n\s*/g, "\n")
    .trim();
}

/** A page as it came: HTML or text. `null` when it cannot be opened. */
export type FetchPage = (url: string) => Promise<string | null>;

/** The page's own title (og:title, else <title>), tidied; "" when it has none. */
export function titleOf(page: string): string {
  const og = /<meta[^>]+property=["']og:title["'][^>]*content=["']([^"']+)["']/i.exec(page) ?? /<meta[^>]+content=["']([^"']+)["'][^>]*property=["']og:title["']/i.exec(page);
  const t = og?.[1] ?? /<title[^>]*>([\s\S]*?)<\/title>/i.exec(page)?.[1] ?? "";
  return pageText(t).replace(/\s+/g, " ").trim().slice(0, 160);
}

const PAGE_TIMEOUT_MS = 10_000;
const PAGE_MAX_BYTES = 4 * 1024 * 1024;

/** The page as it came, or `null` when it cannot be opened (a timeout, an error status, not text). */
async function fetchPageForReal(url: string): Promise<string | null> {
  const signal = AbortSignal.timeout(PAGE_TIMEOUT_MS);
  try {
    const res = await fetch(url, { redirect: "follow", signal, headers: { "user-agent": "Mozilla/5.0 (Macintosh) Inkling/0.1 (checking a quote)", accept: "text/html,text/plain;q=0.9,*/*;q=0.5" } });
    if (!res.ok) return null;
    const type = res.headers.get("content-type") ?? "";
    if (!/text\/|html|xml|json/i.test(type)) return null;
    return (await res.text()).slice(0, PAGE_MAX_BYTES);
  } catch {
    return null;
  }
}

let fetchPage: FetchPage = fetchPageForReal;
export function setPageFetchForTests(f: FetchPage) {
  fetchPage = f;
}

/**
 * The source a correction cites, made sure of: its URL is one a search returned in this
 * call, and its passage (5 to 40 words) is on the page. Returns why not, otherwise.
 */
export async function verifySource(raw: unknown, found: Found[], open: FetchPage = fetchPage): Promise<{ source: Source } | { why: string }> {
  if (typeof raw !== "object" || raw === null) return { why: "it cites no page" };
  const url = typeof (raw as any).url === "string" ? (raw as any).url.trim() : "";
  const passage = typeof (raw as any).passage === "string" ? (raw as any).passage.trim() : "";
  if (!url) return { why: "its source has no URL" };
  const hit = found.find((f) => sameUrl(f.url, url));
  if (!hit) return { why: `${url} did not come back from a search in this answer` };
  const words = passage.split(/\s+/u).filter(Boolean).length;
  if (words < 5 || words > 40) return { why: `the passage it quotes from ${domainOf(hit.url)} has ${words} words` };
  const page = await open(hit.url);
  if (page === null) return { why: `${hit.url} could not be opened` };
  const onPage = findQuote(passage, pageText(page));
  if (!onPage) return { why: `the passage is not on ${hit.url}` };
  // the page's own title reads better than a search's (which can be a section's name)
  const title = titleOf(page) || hit.title || domainOf(hit.url);
  return { source: { url: hit.url, title, domain: domainOf(hit.url), passage: onPage.replace(/\s+/gu, " ").trim() } };
}

const URL_RE = /\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)|<?(https?:\/\/[^\s<>()"]+[^\s<>()".,;:!?'’”])>?/giu;

/** Free text with every link no search returned taken out (a Markdown link keeps its words). */
export function keepFoundLinks(text: string, found: Found[]): string {
  return text
    .replace(URL_RE, (m, label: string | undefined, linked: string | undefined, bare: string | undefined) => {
      const url = linked ?? bare ?? "";
      if (found.some((f) => sameUrl(f.url, url))) return m;
      return label ?? "";
    })
    .replace(/\(\s*\)/g, "")
    .replace(/[ \t]{2,}/g, " ")
    .replace(/ +([.,;:!?])/g, "$1")
    .trim();
}

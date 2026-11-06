// Sources from the web, only when real: a cited page counts only if a search in the same
// call returned its URL and the quoted passage is on the page. No network: pages come from
// a fake fetch. Run: bun test test/web.test.ts
import { describe, expect, test } from "bun:test";
import { domainOf, foundIn, keepFoundLinks, pageText, sameUrl, titleOf, verifySource, WEB_TASKS, type Found } from "../src/web.ts";
import { backCorrection, withWorking, type Thought, type Writer } from "../src/working.ts";

const TOWER = "https://en.wikipedia.org/wiki/Eiffel_Tower";
const found: Found[] = [{ url: TOWER, title: "Eiffel Tower - Wikipedia" }, { url: "https://www.toureiffel.paris/en/the-monument/key-figures", title: "Key figures" }];
const PAGE =
  "<html><head><title>Eiffel Tower</title><script>var x = 'The tower is 450 metres tall';</script></head><body><p>The tower is 330&nbsp;metres (1,083&nbsp;ft) tall, about the same height as an 81-storey building, and the tallest structure in Paris.</p></body></html>";
const open = async (url: string) => (url === TOWER ? pageText(PAGE) : null);
const PASSAGE = "The tower is 330 metres (1,083 ft) tall";

describe("search results", () => {
  test("are read off a WebSearch output, and nothing else", () => {
    const out = { query: "eiffel tower height", results: [{ tool_use_id: "a", content: [{ title: "Eiffel Tower - Wikipedia", url: TOWER }] }, "some commentary"], durationSeconds: 1 };
    expect(foundIn(out)).toEqual([{ url: TOWER, title: "Eiffel Tower - Wikipedia" }]);
    expect(foundIn({ bytes: 10, code: 200, result: "a summary", url: TOWER })).toEqual([]);
    expect(foundIn(null)).toEqual([]);
  });
  test("URLs match without a fragment or a trailing slash, the host in any case", () => {
    expect(sameUrl("https://EN.wikipedia.org/wiki/Eiffel_Tower/#Height", TOWER)).toBe(true);
    expect(sameUrl("https://en.wikipedia.org/wiki/Eiffel_Towers", TOWER)).toBe(false);
    expect(domainOf("https://www.toureiffel.paris/en")).toBe("toureiffel.paris");
  });
  test("a page's words leave out scripts, tags and entities", () => {
    const words = pageText(PAGE);
    expect(words).toContain("The tower is 330 metres (1,083 ft) tall");
    expect(words).not.toContain("450");
  });
});

describe("a page's own title", () => {
  test("og:title first, then <title>, tidied; a search's title when it has none", async () => {
    expect(titleOf('<head><meta property="og:title" content="The Eiffel Tower just got 20 feet taller"><title>NewsNational News</title></head>')).toBe("The Eiffel Tower just got 20 feet taller");
    expect(titleOf("<title>\n  Eiffel Tower &amp; its lifts\n</title>")).toBe("Eiffel Tower & its lifts");
    expect(titleOf("plain text")).toBe("");
    const r = await verifySource({ url: TOWER, passage: PASSAGE }, found, async () => '<title>Eiffel Tower – the official site</title><p>The tower is 330 metres (1,083 ft) tall.</p>');
    expect("source" in r && r.source.title).toBe("Eiffel Tower – the official site");
  });
});

describe("a cited page counts only when real", () => {
  test("a search hit with the passage on the page", async () => {
    expect(await verifySource({ url: TOWER, passage: PASSAGE }, found, open)).toEqual({
      source: { url: TOWER, title: "Eiffel Tower - Wikipedia", domain: "en.wikipedia.org", passage: PASSAGE },
    });
  });
  test("a URL no search returned is made up, whatever the page says", async () => {
    expect(await verifySource({ url: "https://www.eiffel-facts.example/height", passage: PASSAGE }, found, open)).toEqual({
      why: "https://www.eiffel-facts.example/height did not come back from a search in this answer",
    });
  });
  test("a search hit whose page does not say it", async () => {
    expect(await verifySource({ url: TOWER, passage: "The tower is 450 metres tall in total" }, found, open)).toEqual({
      why: `the passage is not on ${TOWER}`,
    });
  });
  test("a page that cannot be opened, and a passage too short to check", async () => {
    const other = found[1].url;
    expect(await verifySource({ url: other, passage: PASSAGE }, found, open)).toEqual({ why: `${other} could not be opened` });
    expect(await verifySource({ url: TOWER, passage: "330 metres" }, found, open)).toEqual({ why: "the passage it quotes from en.wikipedia.org has 2 words" });
  });
});

describe("a correction backed by a page", () => {
  const w: Writer = { title: "Paris", note: "The Eiffel Tower is 450 metres tall.", brief: null, target: "The Eiffel Tower is 450 metres tall.", quote: "450 metres tall" };
  const text = "It is 330 metres, not 450. Where did 450 come from?";
  const thought: Thought = { speak: true, kind: "correction", quote: "450 metres tall", text, sketch: false };
  const source = { url: TOWER, title: "Eiffel Tower - Wikipedia", domain: "en.wikipedia.org", passage: PASSAGE };

  test("stands on the source alone, and may use the source's numbers", () => {
    const r = backCorrection({ source: { url: TOWER, passage: PASSAGE } }, text, w, { source });
    expect(r).toEqual({ check: { sum: [], givens: [], words: [], source } });
  });
  test("one whose source fell through becomes its question, saying why", () => {
    const logged: string[] = [];
    const t = withWorking(thought, { check: { source: { url: "https://made.up/x", passage: PASSAGE } }, question: "How tall is it?" }, w, (l) => logged.push(l), {
      why: "https://made.up/x did not come back from a search in this answer",
    });
    expect(t?.kind).toBe("question");
    expect(t?.text).toBe("How tall is it?");
    expect(logged).toEqual([
      "a source was dropped: https://made.up/x did not come back from a search in this answer",
      "correction became a question: https://made.up/x did not come back from a search in this answer",
    ]);
  });
  test("a number only the dropped page said cannot stand on the writer's words", () => {
    const note = "My guidebook says the tower is 450 metres.\n\nThe Eiffel Tower is 450 metres tall.";
    expect(backCorrection({ words: ["My guidebook says the tower is 450 metres"] }, text, { ...w, note }, { why: "dropped" })).toEqual({
      why: "it says 330, which neither the note nor the check shows",
    });
  });
});

describe("which tasks may look things up", () => {
  test("only what the writer asked for: probes, asks, answers, replies and chat", () => {
    expect(["probe", "ask", "answer", "reply", "chat"].every((t) => WEB_TASKS.has(t))).toBe(true);
    // reading along at a pause never searches, nor do sketches, drawings, learning or relating notes
    expect(["read", "sketch", "draw", "learn", "relate"].some((t) => WEB_TASKS.has(t))).toBe(false);
  });
});

describe("links in prose", () => {
  test("a link no search returned is taken out; a Markdown link keeps its words", () => {
    const t = `It is 330 m ([Wikipedia](${TOWER})). See https://eiffel-facts.example/height or [this page](https://made.up/x) for more.`;
    expect(keepFoundLinks(t, found)).toBe(`It is 330 m ([Wikipedia](${TOWER})). See or this page for more.`);
    expect(keepFoundLinks("No links here.", [])).toBe("No links here.");
  });
});

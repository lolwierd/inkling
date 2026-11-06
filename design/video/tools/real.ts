// Gets Inkling's real words for the walkthrough film from the brain sidecar.
// It runs brain/src/main.ts over its JSON-lines protocol (as brain/test/live.ts does),
// with a clean environment and a throwaway data folder, so it never touches the
// owner's notebook. Each call is saved to data/real.json (and data/real.js for the
// page) as it returns; later stages read what the model said earlier, the way the app
// would. Every call is sent with the context the app would send at that moment of the
// film: the note as it stands, its brief, the thread summaries, the mind.
//
//   bun run design/video/tools/real.ts <scratch-dir> <stage> [<stage>...]
//   stages, in the film's order:
//     first   the guide's first sentence (a probe, as `first_run` sends it)
//     reads   ¶1, ¶2 (the correction), ¶3, the fix (settle), ¶4, as the writer types;
//             only ¶2 may speak (the film shows the others read without a word)
//     probe   Find a loose thread on ¶3
//     ask     Ask Inkling on ¶1
//     room    Ask the room on ¶2: Inkling, Staff engineer, Socratic tutor
//     call    ¶5 calls the Skeptic by name
//     sketch  Sketch it on ¶1 (sketch, then draw)
//     chat    the brief, then the chat
//     reply   the writer answers the probe's question (FILM_REPLY, or REPLY below)
//     mind    a mind of your own reads ¶6
//     learn   Inkling learns about you from a reply
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const VIDEO = join(import.meta.dir, "..");
const BRAIN = join(VIDEO, "..", "..", "brain");
const OUT = join(VIDEO, "data", "real.json");
const [scratch, ...stages] = process.argv.slice(2);
if (!scratch || stages.length === 0) {
  console.error("usage: bun run real.ts <scratch-dir> <stage...>");
  process.exit(2);
}
mkdirSync(join(scratch, "brain-data"), { recursive: true });

const MODEL = "claude-opus-5-5"; // the app's default
const LIGHT = { model: MODEL, effort: "low" }; // the app's read-along
const HEAVY = { model: MODEL, effort: "medium" }; // everything the writer asks for
const NOW = "Saturday 7 November 2026";

// ----- the note, as the film shows it ------------------------------------------
export const TITLE = "Cold starts under two seconds";
const P1 = "Today it takes 9 s at p99 to go from the API call to a shell in a new VM.";
const P2 = "So we skip boot and restore each VM from a memory snapshot. A 4 GiB snapshot copies over a 25 Gbit/s link in about 160 ms, so the copy is not on the critical path.";
// The writer's fix, after the correction: the number, and what follows from it.
const EDIT1 = { from: "160 ms", to: "1.4 s" };
const EDIT2 = { from: "so the copy is not on the critical path", to: "so we load pages on demand" };
const P2_FIXED = P2.replace(EDIT1.from, EDIT1.to).replace(EDIT2.from, EDIT2.to);
const P3 = "We try it on the staging hosts first, from Monday.";
// snapshot_restore is inline code in the film (⌘E); the brain gets the words as they are
const P4 = "The flag is snapshot_restore, and it stays off by default.";
const P5 = "Skeptic, will restores hold up when a hundred VMs start at once?";
const P6 = "We rebuild the snapshot once a week, along with the image.";
const P7 = "Measure p99 from the API call, not from boot.";
const BRIEF = "For a design review on Thursday. What matters is p99, not the median.";
const FIRST = "We can get VM cold starts under two seconds by March.";
const PROBE_TARGET = "We try it on the staging hosts first";
const ASK_TARGET = "9 s at p99";
const ASK_QUESTION = "Does a snapshot skip all of it?";
const ROOM_TARGET = "restore each VM from a memory snapshot";
const ROOM_QUESTION = "What worries you most here? One sentence.";
const SKETCH_TARGET = "from the API call to a shell";
const CHAT_MESSAGE = "What’s the weakest part of this plan now?";
const LEARN_MESSAGE = "I look after the VM fleet, so p99 matters more to me than the average.";

const INKLING = { name: "Inkling", description: "Thinks alongside you, and speaks up when something is worth a second look.", howItThinks: "", voice: "plain", frequency: "when_it_matters" };
// The app's own templates (src/store.rs `mind_templates`), seated in the room or called by name.
const STAFF = { name: "Staff engineer", description: "Reads designs as if it will be on call for them.", howItThinks: "Read like a senior engineer reviewing a design doc. Probe scale, failure modes, consistency, operational cost and the edge cases. Push on hand-waving such as \"we'll just cache it\" or \"it should be fast\". Leave naming and style alone.", voice: "blunt", frequency: "when_it_matters" };
const SKEPTIC = { name: "Skeptic", description: "Asks for the evidence.", howItThinks: "Question claims that are not supported. Ask what the evidence is, what would prove it wrong, and what else could explain it. Test confident words like always, never and obviously.", voice: "plain", frequency: "when_it_matters" };
const SOCRATIC = { name: "Socratic tutor", description: "Never gives the answer; asks the question that leads to it.", howItThinks: "The writer is learning. Find the misunderstanding underneath what they wrote and ask the smallest question that would let them discover it. Do not state the answer. Say so when they get it right.", voice: "warm", frequency: "when_it_matters" };
// The mind the writer makes in chapter 5 (Settings › Minds › New mind, from Blank).
const SECURITY = { name: "Security", description: "Looks for what an attacker would use.", howItThinks: "Read like a security reviewer. Ask what is exposed, what is shared between machines, and what goes stale. Never suggest a product.", voice: "blunt", frequency: "often" };

type Json = Record<string, any>;
const real: Json = existsSync(OUT) ? JSON.parse(readFileSync(OUT, "utf8")) : { calls: {} };
real.note = {
  title: TITLE, paras: [P1, P2, P3, P4, P5, P6, P7], p2Fixed: P2_FIXED, edits: [EDIT1, EDIT2], brief: BRIEF, first: FIRST,
  probe: PROBE_TARGET, ask: { target: ASK_TARGET, question: ASK_QUESTION }, room: { target: ROOM_TARGET, question: ROOM_QUESTION, minds: ["Inkling", "Staff engineer", "Socratic tutor"] },
  sketch: SKETCH_TARGET, chat: CHAT_MESSAGE, mind: SECURITY, learn: LEARN_MESSAGE, reply: real.note?.reply,
};
// real.js is the same data for the film page, which loads it with a script tag.
const save = () => {
  writeFileSync(OUT, JSON.stringify(real, null, 2) + "\n");
  writeFileSync(OUT.replace(/\.json$/, ".js"), `// Generated by tools/real.ts: Inkling's words, as the brain returned them.\nwindow.REAL = ${JSON.stringify(real)};\n`);
};

function ctx(paras: string[], extra: Json = {}) {
  return { noteTitle: TITLE, noteText: paras.join("\n\n"), brief: null, mind: INKLING, aboutMe: null, threads: [], now: NOW, ...extra };
}

// ----- the brain ------------------------------------------------------------------
const env: Record<string, string> = {
  HOME: process.env.HOME!,
  USER: process.env.USER ?? "",
  LOGNAME: process.env.USER ?? "",
  PATH: "/usr/bin:/bin:/usr/sbin:/sbin",
  LANG: "en_US.UTF-8",
  TMPDIR: process.env.TMPDIR ?? "/tmp",
  SHELL: "/bin/zsh",
  INKLING_DATA_DIR: join(scratch, "brain-data"),
};
const proc = Bun.spawn([process.execPath, "run", "src/main.ts"], { cwd: BRAIN, env, stdin: "pipe", stdout: "pipe", stderr: "pipe" });
const events: Json[] = [];
let waiters: { pred: (e: Json) => boolean; resolve: (e: Json) => void }[] = [];
let stderr = "";
(async () => {
  const dec = new TextDecoder();
  let buf = "";
  for await (const chunk of proc.stdout as ReadableStream<Uint8Array>) {
    buf += dec.decode(chunk, { stream: true });
    for (let i = buf.indexOf("\n"); i >= 0; i = buf.indexOf("\n")) {
      const line = buf.slice(0, i);
      buf = buf.slice(i + 1);
      if (!line.trim()) continue;
      const ev = { ...JSON.parse(line), at: Date.now() };
      events.push(ev);
      waiters = waiters.filter((w) => (w.pred(ev) ? (w.resolve(ev), false) : true));
    }
  }
})();
(async () => {
  for await (const chunk of proc.stderr as ReadableStream<Uint8Array>) stderr += new TextDecoder().decode(chunk);
})();
function send(req: unknown) {
  const s = proc.stdin as import("bun").FileSink;
  s.write(JSON.stringify(req) + "\n");
  s.flush();
}
function waitFor(pred: (e: Json) => boolean, ms = 240_000): Promise<Json> {
  const have = events.find(pred);
  if (have) return Promise.resolve(have);
  return new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error("timed out")), ms);
    waiters.push({ pred, resolve: (e) => (clearTimeout(t), resolve(e)) });
  });
}

let calls = 0;
async function call(name: string, settings: Json, input: Json) {
  const id = `film-${name}`;
  const t0 = Date.now();
  send({ op: "run", id, settings, input });
  const e = await waitFor((x) => x.id === id && (x.ev === "done" || x.ev === "error"));
  calls++;
  // Streamed tasks keep their chunks with their arrival times, so the film can stream
  // the words at the pace they came.
  const deltas = events.filter((x) => x.id === id && x.ev === "delta").map((x) => ({ ms: x.at - t0, text: x.text }));
  if (e.ev !== "done") throw new Error(`${name}: ${e.kind}: ${e.message}`);
  if (real.calls[name]) real.calls[`${name}_take${Object.keys(real.calls).filter((k) => k.startsWith(`${name}_take`)).length + 1}`] = real.calls[name];
  real.calls[name] = { at: new Date(t0).toISOString(), ms: Date.now() - t0, settings, input: { ...input, ctx: input.ctx ? { ...input.ctx, noteText: `${input.ctx.noteText.length} chars` } : undefined }, output: e.data, usage: e.usage, deltas: deltas.length ? deltas : undefined };
  save();
  console.log(`${name} (${((Date.now() - t0) / 1000).toFixed(1)} s): ${JSON.stringify(e.data).slice(0, 900)}`);
  return e.data as Json;
}
const out = (name: string) => {
  const c = real.calls[name];
  if (!c) throw new Error(`run the stage that makes ${name} first`);
  return c.output as Json;
};
const reply = () => process.env.FILM_REPLY || real.note.reply || REPLY;

/** The note's threads at a point in the film, as the app sends them (a dismissed thread
 *  is put back with ⌥⌘Z, so none is missing). */
function threads(which: string[]): Json[] {
  const all: Record<string, () => Json[]> = {
    t1: () => { const r = out("read_p2"), s = out("settle"); return [{ quote: r.quote, messages: [{ role: "mind", text: r.text }, { role: "mind", text: s.text }], resolved: s.resolved === true }]; },
    t1open: () => { const r = out("read_p2"); return [{ quote: r.quote, messages: [{ role: "mind", text: r.text }], resolved: false }]; },
    t2: () => { const q = out("probe_p3"); return [{ quote: q.quote, messages: [{ role: "mind", text: q.text }], resolved: false }]; },
    t2r: () => { const q = out("probe_p3"), r = out("reply"); return [{ quote: q.quote, messages: [{ role: "mind", text: q.text }, { role: "you", text: reply() }, { role: "mind", text: r.text }], resolved: true }]; },
    ask: () => [{ quote: ASK_TARGET, messages: [{ role: "you", text: ASK_QUESTION }, { role: "mind", text: out("ask").text }], resolved: false }],
    room: () => ["inkling", "staff", "tutor"].map((m) => ({ quote: ROOM_TARGET, messages: [{ role: "you", text: ROOM_QUESTION }, { role: "mind", text: out(`room_${m}`).text }], resolved: false })),
    call: () => [{ quote: P5, messages: [{ role: "mind", text: out("call").text }], resolved: false }],
    sketch: () => { const s = out("sketch"); return [{ quote: s.quote, messages: [{ role: "mind", text: s.text }], resolved: false }]; },
  };
  return which.flatMap((w) => all[w]());
}
const BODY = [P1, P2_FIXED, P3, P4];

// ----- stages ---------------------------------------------------------------------
const STAGES: Record<string, () => Promise<unknown>> = {
  // A new notebook: the writer's first sentence under the guide, probed as `first_run`
  // does, with the guide's own Markdown and threads (data/start-here.json, from the app).
  async first() {
    const guide = JSON.parse(readFileSync(join(VIDEO, "data", "start-here.json"), "utf8"));
    const blocks = guide.blocks.slice();
    blocks[blocks.length - 1] = { kind: "paragraph", text: FIRST };
    const noteText = markdown(blocks);
    if (process.env.FIRST_REQUEST) {
      // the app's own request, captured from a run with the fake brain: the text must match
      const app = JSON.parse(readFileSync(process.env.FIRST_REQUEST, "utf8")).input.ctx.noteText;
      if (app !== noteText) throw new Error(`the guide's Markdown differs from the app's:\n${app}\n---\n${noteText}`);
    }
    await call("first", LIGHT, { task: "probe", ctx: { ...ctx([]), noteTitle: guide.title, noteText, threads: guide.threads.map((t: Json) => ({ quote: t.quote, messages: t.messages, resolved: false })) }, target: FIRST });
  },
  // Reading along: each paragraph is read when the writer pauses after it.
  async reads() {
    await call("read_p1", LIGHT, { task: "read", ctx: ctx([P1]), target: P1 });
    const r2 = await call("read_p2", LIGHT, { task: "read", ctx: ctx([P1, P2]), target: P2 });
    if (!r2.speak) throw new Error("¶2 was read without a word; the film needs its correction");
    await call("read_p3", LIGHT, { task: "read", ctx: ctx([P1, P2, P3], { threads: threads(["t1open"]) }), target: P3 });
    // Fix the words: the thread keeps the words it was opened on; the edit is the writer's turn.
    await call("settle", HEAVY, { task: "reply", ctx: ctx([P1, P2_FIXED, P3], { threads: threads(["t1open"]) }), quote: r2.quote, messages: [{ role: "mind", text: r2.text }], message: `(I edited the paragraph. It now reads: «${P2_FIXED}»)` });
    await call("read_p4", LIGHT, { task: "read", ctx: ctx(BODY, { threads: threads(["t1"]) }), target: P4 });
  },
  // Find a loose thread (⌘↵) on words of ¶3, which was read without a word.
  async probe() {
    await call("probe_p3", HEAVY, { task: "probe", ctx: ctx(BODY, { threads: threads(["t1"]) }), target: PROBE_TARGET });
  },
  async ask() {
    await call("ask", HEAVY, { task: "ask", ctx: ctx(BODY, { threads: threads(["t1", "t2"]) }), target: ASK_TARGET, question: ASK_QUESTION });
  },
  // Ask the room: one question, three minds, answered side by side.
  async room() {
    const th = threads(["t1", "t2", "ask"]);
    await Promise.all(([["inkling", INKLING], ["staff", STAFF], ["tutor", SOCRATIC]] as const).map(([key, mind]) =>
      call(`room_${key}`, HEAVY, { task: "ask", ctx: ctx(BODY, { threads: th, mind }), target: ROOM_TARGET, question: ROOM_QUESTION })));
  },
  async call() {
    await call("call", HEAVY, { task: "answer", ctx: ctx([...BODY, P5], { threads: threads(["t1", "t2", "ask", "room"]), mind: SKEPTIC }), target: P5 });
  },
  async sketch() {
    const base = ctx([...BODY, P5], { threads: threads(["t1", "t2", "ask", "room", "call"]) });
    const s = await call("sketch", HEAVY, { task: "sketch", ctx: base, target: SKETCH_TARGET });
    await call("draw", HEAVY, { task: "draw", ctx: { ...base, threads: [...base.threads, { quote: s.quote, messages: [{ role: "mind", text: s.text }], resolved: false }] }, quote: s.quote, thought: s.text, request: null });
  },
  async chat() {
    await call("chat", HEAVY, { task: "chat", ctx: ctx([...BODY, P5], { threads: threads(["t1", "t2", "ask", "room", "call", "sketch"]), brief: BRIEF }), history: [], message: CHAT_MESSAGE, near: null });
  },
  async reply() {
    const q = out("probe_p3");
    real.note.reply = reply();
    await call("reply", HEAVY, { task: "reply", ctx: ctx([...BODY, P5], { threads: threads(["t1", "t2", "ask", "room", "call", "sketch"]), brief: BRIEF }), quote: q.quote, messages: [{ role: "mind", text: q.text }], message: reply() });
  },
  // The note's mind is now the writer's own; it reads the next paragraph.
  async mind() {
    await call("mind", LIGHT, { task: "read", ctx: ctx([...BODY, P5, P6], { threads: threads(["t1", "t2r", "ask", "room", "call", "sketch"]), brief: BRIEF, mind: SECURITY }), target: P6 });
  },
  async learn() {
    await call("learn", LIGHT, { task: "learn", aboutMe: null, message: LEARN_MESSAGE });
  },
};

// The writer's answer to the probe's question, written after reading it.
const REPLY = "Our three worst-burst days, replayed at production density with the background traffic. Pass: p99 under 2 s inside the bursts alone.";

/** The note's text as the app sends it (store.rs `markdown_blocks`, title left out). */
function markdown(blocks: Json[]): string {
  const family = (k: string) => (k === "bullet" || k === "check" || k === "check-done" ? 0 : k === "numbered" ? 1 : null);
  const run = (k: string) => k.startsWith("callout") || k.startsWith("code");
  let s = "", n = 0, list: number | null = null;
  for (let i = 1; i < blocks.length; ) {
    const b = blocks[i];
    if (run(b.kind)) {
      let end = i + 1;
      while (end < blocks.length && blocks[end].kind === b.kind) end++;
      const shown = blocks.slice(i, end).filter((x: Json) => x.text.trim());
      if (shown.length) {
        if (list != null) { s += "\n"; list = null; }
        n = 0;
        if (b.kind.startsWith("callout")) {
          s += `> [!${b.kind.split("-")[1]}]\n` + shown.map((x: Json) => `> ${x.text}\n`).join(">\n");
        } else {
          const lines = blocks.slice(i, end);
          let a = 0; while (!lines[a].text.trim()) a++;
          let z = lines.length - 1; while (!lines[z].text.trim()) z--;
          const longest = Math.max(0, ...lines.map((x: Json) => Math.max(0, ...(x.text.match(/`+/g) || [""]).map((r: string) => r.length))));
          const fence = "`".repeat(Math.max(longest, 2) + 1);
          s += `${fence}${b.kind.slice(5)}\n` + lines.slice(a, z + 1).map((x: Json) => x.text + "\n").join("") + fence + "\n";
        }
        s += "\n";
      }
      i = end;
      continue;
    }
    i++;
    if (!b.text.trim()) continue;
    if (list != null && list !== family(b.kind)) s += "\n";
    if (b.kind !== "numbered") n = 0;
    if (b.kind === "h1") s += `# ${b.text}\n\n`;
    else if (b.kind === "h2") s += `## ${b.text}\n\n`;
    else if (b.kind === "h3") s += `### ${b.text}\n\n`;
    else if (b.kind === "bullet") s += `- ${b.text}\n`;
    else if (b.kind === "check" || b.kind === "check-done") s += `- [${b.kind === "check-done" ? "x" : " "}] ${b.text}\n`;
    else if (b.kind === "numbered") s += `${++n}. ${b.text}\n`;
    else if (b.kind === "quote") s += `> ${b.text}\n\n`;
    else s += `${b.text}\n\n`;
    list = family(b.kind);
  }
  return s.trimEnd();
}

try {
  if (stages.includes("markdown")) {
    // check the guide's Markdown against a captured app request without calling the model
    const guide = JSON.parse(readFileSync(join(VIDEO, "data", "start-here.json"), "utf8"));
    const blocks = guide.blocks.slice();
    blocks[blocks.length - 1] = { kind: "paragraph", text: FIRST };
    const app = JSON.parse(readFileSync(process.env.FIRST_REQUEST!, "utf8")).input.ctx.noteText;
    console.log(app === markdown(blocks) ? "the guide's Markdown matches the app's" : `differs:\n${app}\n---\n${markdown(blocks)}`);
  } else {
    await waitFor((e) => e.ev === "ready", 15_000);
    for (const s of stages) {
      if (!STAGES[s]) throw new Error(`unknown stage ${s}`);
      await STAGES[s]();
    }
  }
} catch (e) {
  console.error(String(e));
  console.error(stderr.slice(-2000));
  process.exitCode = 1;
} finally {
  console.log(`model calls this run: ${calls}`);
  (proc.stdin as import("bun").FileSink).end();
  await Promise.race([proc.exited, Bun.sleep(5000)]);
}

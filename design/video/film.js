/* Inkling: the walkthrough film, the core cut (7 Nov 2026). One note, the notes only:
 * you write, Inkling reads along, a thread comes out of your exact words, a correction shows
 * its working, you fix the words or write back, and the thread ties itself off.
 *
 * Every frame is renderAt(t), a pure function of time: the camera, every stitch, tag, rope
 * and caption is computed from t alone (the rope is stepped at a fixed rate from the start of
 * each motion), nothing reads a clock, and CSS transitions and animations are off. So frame
 * n of a 60 fps render is exactly t = n / 60, whichever worker draws it, and motion is as
 * even as the curves it follows. The camera moves on a smootherstep: it starts and stops with
 * zero velocity and zero acceleration, never overshoots and never cuts.
 *
 * Every word Inkling says is the brain's real output (data/real.js, from tools/real.ts).
 * PLAN lays the film out as moments end to end, so a moment can be added and the rest moves
 * with it; render.ts renders each moment as its own chunk. FILM.events is the score's short
 * cue list (audio.js); FILM.moments the page's.
 */
(function () {
  "use strict";
  const A = window.APP, { fx, clamp, lerp, prog, esc, E, spring, knotSVG, knotScale, capsule, runningStitch } = A;
  const REAL = window.REAL, C = REAL.calls, N = REAL.note;
  const FPS = 60;
  const $ = (s, r) => (r || document).querySelector(s);
  const within = (t, a, b) => t >= a && t < b;
  const rng = A.rng;
  const out = (name) => C[name].output;
  /** smootherstep: zero velocity and acceleration at both ends, no overshoot */
  const SMOOTH = (k) => (k <= 0 ? 0 : k >= 1 ? 1 : k * k * k * (k * (k * 6 - 15) + 10));

  // ======================================================================= words (all real)
  const TITLE = N.title;
  const [P1, P2, P3] = N.paras;
  const EDIT = N.edit, P2F = N.p2Fixed;
  const TXT = {
    correction: out("read_p2").text, corrQuote: out("read_p2").quote, check: out("read_p2").check, settle: out("settle").text,
    probe: out("probe_p3").text, probeQuote: out("probe_p3").quote, reply: N.reply, answer: out("reply").text,
  };

  // ======================================================================= the timeline
  // PLAN: the film as moments, end to end. Each is [id, length, cues(start)] with its cues
  // measured from its own start, a caption, and the chords the score holds under it.
  const T = {}, ED = {}, MOMENT = {};
  const PLAN = [
    { n: 0, id: "open", name: "A notebook that asks", dur: 4.2, chords: ["Fmaj9"],
      cues: (s) => ({ sew0: s + 0.4, sew1: s + 1.9, openTie: s + 2.35, lineUp: s + 2.55, pull0: s + 2.6, pull1: s + 4.2 }) },
    { n: 1, id: "write", name: "You write", dur: 9.0, chords: ["Dm9"], caption: [0.7, 8.4, "You write. It reads along."],
      cues: (s) => ({ title: [s + 0.2, s + 1.3], p1: [s + 1.6, s + 4.6], read1: [s + 4.9, s + 5.8] }) },
    { n: 2, id: "plan", name: "It reads at your pauses", dur: 9.0, chords: ["Bbmaj7"], caption: [0.3, 8.6, "It reads each paragraph when you pause."],
      cues: (s) => ({ p2: [s, s + 5.2], read2: [s + 5.3, s + 6.2], p3: [s + 5.5, s + 7.3], t1arrive: s + 6.2, pause3: s + 7.3 }) },
    { n: 3, id: "thread", name: "A thread from your words", dur: 7.0, chords: ["Gm9"], caption: [0.2, 6.6, "When a line doesn’t add up, it pulls a thread from those words."],
      cues: (s) => ({ stitch1: [s + 0.4, s + 1.4], carry1: s + 1.4 }) },
    { n: 4, id: "working", name: "It shows its working", dur: 9.0, chords: ["Am7", "C9sus"], caption: [0.4, 8.6, "It shows its working, from your own numbers."],
      cues: (s) => ({ hover1: s + 0.4, link: s + 7.2 }) },
    { n: 5, id: "fix", name: "Fix the words", dur: 10.0, chords: ["C9sus"], caption: [0.2, 9.6, "Fix the words, and the thread ties itself off."],
      cues: (s) => ({ click: s + 0.15, type: [s + 0.6, s + 1.6], reread: [s + 1.9, s + 2.9], settle: [s + 3.0, s + 4.6], breath5: s + 5.3, tie5: s + 5.9, knot5: s + 6.5, fly5: [s + 6.6, s + 7.7], fade5: [s + 8.0, s + 8.5] }) },
    { n: 6, id: "find", name: "Select words, and it asks", dur: 8.0, chords: ["Dm9", "Bbmaj7"], caption: [0.3, 7.8, "Select any words, and it finds the question in them."],
      cues: (s) => ({ sel: [s + 0.5, s + 1.1], bar: s + 1.2, find: s + 2.1, sew2: [s + 2.3, s + 3.0], carry2: s + 3.0, open2: s + 3.9 }) },
    { n: 7, id: "reply", name: "Write back, and tie it off", dur: 10.0, chords: ["Gm9"], caption: [0.3, 9.6, "Write back. When it’s settled, tie it off."],
      cues: (s) => ({ openTag: s + 0.2, focus: s + 0.6, typeReply: [s + 0.8, s + 3.8], send: s + 3.9, answer: [s + 4.5, s + 5.9], resolveHov: s + 6.6, breath7: s + 7.0, tie7: s + 7.5, knot7: s + 8.1, fly7: [s + 8.2, s + 9.3], fade7: [s + 9.4, s + 9.9] }) },
    { n: 8, id: "end", name: "The thinking stays yours", dur: 7.2, chords: ["Fmaj9"], caption: [0.2, 2.4, "It never writes your notes. The thinking stays yours.", "title"],
      cues: (s) => ({ __into: "ED", back: [s, s + 1.5], lift: [s + 2.4, s + 3.1], plate: [s + 2.5, s + 3.1], tile: [s + 3.1, s + 3.45], border: [s + 3.2, s + 3.9], satin: [s + 3.45, s + 4.2],
        settle: s + 4.3, word: [s + 4.4, s + 5.2], wdot: s + 5.3, line: [s + 5.35, s + 5.9] }) },
  ];
  const CHS = {};
  let DUR = 0;
  {
    let t = 0;
    for (const mo of PLAN) {
      CHS[mo.n] = t;
      const cues = mo.cues(t);
      const into = cues.__into === "ED" ? ED : T;
      delete cues.__into;
      Object.assign(into, cues);
      MOMENT[`${mo.n}.${mo.id}`] = { t0: t, t1: t + mo.dur };
      mo.moments = [{ id: mo.id, dur: mo.dur }]; // render.ts and audio.js read plans as chapters of moments
      t += mo.dur;
    }
    DUR = t;
  }
  const cEnd = CHS[8];

  // ======================================================================= typing
  function sched(text, t0, t1, seed) {
    const r = rng(seed), w = [];
    for (let i = 0; i < text.length; i++) {
      const c = text[i], prev = i ? text[i - 1] : "";
      let d = 0.72 + r() * 0.56;
      if (prev === " ") d *= 1.25;
      if (/[.,?!:;]/.test(prev)) d *= 2.3;
      if (/[A-Z0-9’]/.test(c)) d *= 1.15;
      if (r() < 0.05) d *= 1.9;
      w.push(d);
    }
    const tot = w.reduce((a, b) => a + b, 0);
    let acc = 0;
    return w.map((d) => { acc += d; return t0 + (acc / tot) * (t1 - t0); });
  }
  const count = (times, t) => { let n = 0; while (n < times.length && times[n] <= t) n++; return n; };
  const lastKey = (times, t) => { const n = count(times, t); return n ? times[n - 1] : -9; };
  const S = {
    title: sched(TITLE, ...T.title, 11), p1: sched(P1, ...T.p1, 12), p2: sched(P2, ...T.p2, 13), p3: sched(P3, ...T.p3, 14),
    edit: sched(EDIT.to, ...T.type, 15), reply: sched(TXT.reply, ...T.typeReply, 23),
  };
  const ALLKEYS = [S.title, S.p1, S.p2, S.p3, S.edit];
  const typingAt = (t) => ALLKEYS.some((k) => t - lastKey(k, t) < 0.55 && t >= k[0] - 0.05);
  const caretOn = (t) => typingAt(t) || Math.floor(t / 0.53) % 2 === 0;
  /** Streamed words: word by word, at the pace the window allows. */
  function streamed(text, t0, t1, t) {
    if (t <= t0) return "";
    if (t >= t1) return text;
    const words = text.split(" "), n = Math.ceil(prog(t, t0, t1) * words.length);
    return words.slice(0, n).join(" ");
  }
  /** How far a tag is unfolded at t, from a list of [time, target] (springs, as the app). */
  function unfoldAt(t, keys) {
    let from = 0, to = 0, at = null;
    const seg = (x) => lerp(from, to, to >= from ? spring(x - at, 420, 41) : spring(x - at, 520, 45.6));
    for (const [tk, target] of keys) {
      if (tk > t) break;
      const vk = at == null ? 0 : seg(tk);
      from = vk; to = target; at = tk;
    }
    return at == null ? 0 : seg(t);
  }

  // ======================================================================= measurements
  const MEAS = {};
  function measureNote(name, page) {
    const host = $("#measure .page");
    const geo = A.renderPage(host, page, 0, 1);
    const res = { blocks: {}, anchors: {} };
    for (const b of page.blocks) { const bb = geo.blockBox(b.id); if (bb) res.blocks[b.id] = bb; }
    for (const th of page.threads || []) { const L = geo.lineRects(th.block, th.id); if (L.length) res.anchors[th.id] = L; }
    MEAS[name] = res;
  }

  // ======================================================================= the note
  /** Paragraph 2 over time: typed, then its number fixed by typing over the words the
   *  working's link selected. */
  function p2At(t) {
    let text = P2.slice(0, count(S.p2, t)), sel = null, caret = text.length;
    const i = P2.indexOf(EDIT.from);
    if (t < T.click) return { text, sel, caret };
    if (t < T.type[0]) return { text, sel: [i, i + EDIT.from.length], caret: null };
    const n = count(S.edit, t);
    if (n === 0) return { text, sel: [i, i + EDIT.from.length], caret: null };
    text = P2.slice(0, i) + EDIT.to.slice(0, n) + P2.slice(i + EDIT.from.length);
    return { text, sel: null, caret: i + n };
  }
  /** The chrome steps back while you type and comes back when the pointer moves. */
  function calmAt(t) {
    const spans = [[T.title[0], T.p1[1] + 0.2], [T.p2[0], T.p3[1] + 0.2]];
    let v = 0;
    for (const [a, b] of spans) {
      if (t < a) continue;
      const k = 1 - Math.exp(-5 * Math.max(0, Math.min(t, b) - a));
      v = t <= b ? k : k * Math.exp(-14 * (t - b));
      if (t <= b + 0.5) break;
    }
    return clamp(v);
  }

  function mainNote(t) {
    const blocks = [];
    const caretIn = (() => {
      if (t < T.p1[0]) return "title";
      if (t < T.p2[0]) return "p1";
      if (t < T.p3[0]) return "p2";
      if (t < T.click) return "p3";
      if (within(t, T.type[0], T.reread[0])) return "p2";
      return null;
    })();
    const p1 = P1.slice(0, count(S.p1, t)), p2 = p2At(t), p3 = P3.slice(0, count(S.p3, t));
    const sel = {};
    if (p2.sel) sel.p2 = p2.sel;
    if (within(t, T.sel[0], T.sew2[0])) { const i = p3.indexOf(TXT.probeQuote); if (i >= 0) sel.p3 = [i, Math.round(i + TXT.probeQuote.length * SMOOTH(prog(t, ...T.sel)))]; }
    const gut = {};
    gut.p1 = within(t, ...T.read1) ? { type: "needle" } : t >= T.read1[1] ? { type: "tack" } : null;
    gut.p2 = within(t, ...T.read2) || within(t, ...T.reread) ? { type: "needle" } : t >= T.read2[1] ? { type: "tack" } : null;
    gut.p3 = t >= T.p3[1] + 0.9 ? { type: "tack" } : t >= T.p3[1] ? { type: "needle" } : null;
    const add = (id, text, caret) => blocks.push({ id, kind: "p", text, caret: caretIn === id ? caret : null, sel: sel[id], ranges: [], gutter: gut[id] });
    if (t >= T.p1[0]) add("p1", p1, p1.length);
    if (t >= T.p2[0]) add("p2", p2.text, p2.caret);
    if (t >= T.p3[0]) add("p3", p3, p3.length);
    if (t >= T.title[1] && t < T.p1[0]) blocks.push({ id: "p0", kind: "p", text: "", ph: "Just start writing.", caret: null });
    const block = (id) => blocks.find((x) => x.id === id);

    const threads = [];
    // t1: the correction on the budget's arithmetic, with its working; fixed, it ties itself off
    if (t >= T.stitch1[0]) {
      const b = block("p2"), i = p2.text.indexOf(TXT.corrQuote);
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.corrQuote.length, id: "t1", cls: "anc" });
        const tied = t >= T.knot5;
        const u = unfoldAt(t, [[T.hover1, 1], [T.settle[1] + 0.4, 1], [T.fade5[0] - 0.2, 0]]);
        // the check's words: the one the writer retyped is no longer in the note
        const changed = t >= T.type[1];
        let body = A.B.msg(TXT.correction) + A.B.working(TXT.check, { changed: changed ? [EDIT.from] : [], hov: within(t, T.link - 0.1, T.click + 0.3) ? EDIT.from : null, open: false });
        if (t >= T.reread[0] && t < T.settle[0]) body += A.B.think("Reading your change", t);
        if (t >= T.settle[0]) body += A.B.msg(streamed(TXT.settle, ...T.settle, t));
        const gone = t >= T.fade5[1];
        threads.push({ id: "t1", block: "p2", a: i, kind: "correction", c: "madder", side: "right", sew: prog(t, ...T.stitch1), needle: t < T.stitch1[1], tied, tag: !gone && t >= T.carry1 + 0.45, rope: !gone,
          stitchOp: tied ? 0.42 : 1, ropeOp: t < T.carry1 ? 0 : tied ? 0.8 * (1 - prog(t, ...T.fade5)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry1 + 1.4 ? { t0: T.carry1, carry: 0.5 } : t >= T.tie5 ? { t0: T.tie5, tieAt: T.tie5, speed: 0.5 } : null, knotAt: t >= T.knot5 && t < T.fly5[0] ? T.knot5 : null,
          tagOp: 1 - prog(t, ...T.fade5), wash: u > 0.4 && !tied,
          tagState: { unfold: u, mind: "Inkling", kindLabel: tied ? "tied off" : "correction", body } });
      }
    }
    // t2: Find a loose thread on the staging line; written back to, settled, tied off
    if (t >= T.sew2[0]) {
      const b = block("p3"), i = p3.indexOf(TXT.probeQuote);
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.probeQuote.length, id: "t2", cls: "anc" });
        const tied = t >= T.knot7;
        const u = unfoldAt(t, [[T.open2, 1], [T.fade7[0] - 0.2, 0]]);
        let body = A.B.msg(TXT.probe);
        if (t >= T.send) body += A.B.you(TXT.reply);
        if (t >= T.send && t < T.answer[0]) body += A.B.think("", t);
        if (t >= T.answer[0]) body += A.B.msg(streamed(TXT.answer, ...T.answer, t));
        const active = t >= T.openTag && t < T.fade7[0];
        if (active && t < T.send) body += A.B.reply(TXT.reply.slice(0, count(S.reply, t)), t >= T.focus, t - lastKey(S.reply, t) < 0.5 || Math.floor(t / 0.53) % 2 === 0);
        if (active) body += A.B.acts(tied, within(t, T.resolveHov, T.tie7) ? "resolve" : null);
        const gone = t >= T.fade7[1];
        threads.push({ id: "t2", block: "p3", a: i, kind: "question", c: "madder", side: "right", sew: prog(t, ...T.sew2), needle: t < T.sew2[1], tied, tag: !gone && t >= T.carry2 + 0.45, rope: !gone,
          stitchOp: tied ? 0.42 : 1, ropeOp: t < T.carry2 ? 0 : tied ? 0.8 * (1 - prog(t, ...T.fade7)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry2 + 1.4 ? { t0: T.carry2, carry: 0.5 } : t >= T.tie7 ? { t0: T.tie7, tieAt: T.tie7, speed: 0.5 } : null, knotAt: t >= T.knot7 && t < T.fly7[0] ? T.knot7 : null,
          tagOp: 1 - prog(t, ...T.fade7), wash: u > 0.4 && !tied,
          tagState: { unfold: u, mind: "Inkling", kindLabel: tied ? "tied off" : "question", body } });
      }
    }
    // open questions are numbered in reading order, with a knot in the gutter
    const order = ["p1", "p2", "p3"];
    const open = threads.filter((th) => !th.tied && th.tag !== false);
    open.sort((a, b) => order.indexOf(a.block) - order.indexOf(b.block) || a.a - b.a);
    open.forEach((th, i) => (th.num = i + 1));
    for (const id of order) {
      const first = open.find((th) => th.block === id);
      const bl = block(id);
      if (first && bl && !(bl.gutter && bl.gutter.type === "needle")) bl.gutter = { type: "knot", n: first.num, c: first.c };
    }
    // the border at the top of the page: a motif for each thread tied off; the last one
    // leaves it at the end, to become the dot of the i
    const motifs = [];
    if (t >= T.fly5[1]) motifs.push({ c: "madder", k: prog(t, T.fly5[1], T.fly5[1] + 0.4) });
    if (t >= T.fly7[1] && t < ED.lift[0]) motifs.push({ c: "madder", k: prog(t, T.fly7[1], T.fly7[1] + 0.4) });
    const header = { op: 1 - calmAt(t), mind: "Inkling", mindC: "var(--voice)", mode: "Reads along", loose: open.length };
    const page = { scroll: 0, header, blocks, threads, border: { motifs }, title: { text: TITLE.slice(0, count(S.title, t)), caret: caretIn === "title" ? count(S.title, t) : null }, caretOn: caretOn(t), menus: [] };
    if (within(t, T.bar, T.find + 0.1)) page.menus.push({ type: "toolbar", block: "p3", anchor: "t2s", op: spring(t - T.bar, 400, 40), hov: t >= T.find - 0.15 ? "find" : null });
    if (within(t, T.bar, T.find + 0.1)) { const b = block("p3"); const i = p3.indexOf(TXT.probeQuote); if (b && i >= 0) b.ranges.push({ a: i, b: i + TXT.probeQuote.length, id: "t2s", cls: "" }); }
    return page;
  }

  // ======================================================================= the camera
  // (x, y): the world point at the centre of the frame; z: screen px per point. Every move
  // is a smootherstep from where the camera is to where it goes: no overshoot, no cut, and
  // nothing moves between moves.
  let CAM = [];
  function camAt(t) {
    let c = { x: 660, y: -318, z: 1.56 };
    for (const mv of CAM) {
      if (t <= mv.t[0]) break;
      const k = SMOOTH(prog(t, mv.t[0], mv.t[1]));
      c = { x: lerp(c.x, mv.to.x, k), y: lerp(c.y, mv.to.y, k), z: lerp(c.z, mv.to.z, k) };
    }
    return c;
  }
  const toScreen = (cam, x, y) => [960 + (x - cam.x) * cam.z, 540 + (y - cam.y) * cam.z];

  function buildCamera() {
    const M = MEAS.main, an = (id) => (M.anchors[id] || [{ x: 351, y: 300, r: 900, b: 328 }])[0], blk = (id) => M.blocks[id] || { y: 300, h: 28, x: 351, w: 614 };
    const a1 = an("t1"), a2 = an("t2"), b1 = blk("p1"), b2 = blk("p2"), b3 = blk("p3");
    const tagX = 351 + 614 + 40 + 124; // the middle of a tag in the right margin
    CAM = [
      // the title is sewn on the cloth, then the camera comes down to the window
      { t: [T.pull0, T.pull1 + 0.4], to: { x: 660, y: 300, z: 1.32 } },
      // writing: in a little on the page as it fills, following the words down
      { t: [T.title[0] - 0.2, T.p1[0] + 0.8], to: { x: 640, y: 250, z: 1.62 } },
      { t: [T.p2[0] - 0.4, T.p2[0] + 2.2], to: { x: 640, y: (b1.y + b2.y) / 2 + 40, z: 1.62 } },
      // the thread: in on the words as it is sewn, then along it to its tag
      { t: [T.pause3 - 0.4, T.stitch1[1]], to: { x: 700, y: a1.y + 27, z: 2.1 } },
      { t: [T.carry1 - 0.1, T.carry1 + 1.6], to: { x: 805, y: a1.y + 27, z: 1.95 } },
      // the working: the tag open beside the whole paragraph, so every number in the sum shows
      // in the note too; clear of the caption
      { t: [T.hover1, T.hover1 + 1.8], to: { x: 805, y: a1.y + 135, z: 1.9 } },
      // the fix: the paragraph and the tag together as the tag grows; in on the knot as it is tied
      { t: [T.click - 0.6, T.click + 0.9], to: { x: 810, y: a1.y + 227, z: 1.72 } },
      { t: [T.breath5 - 0.3, T.tie5 + 0.2], to: { x: 950, y: a1.y + 27, z: 2.4 } },
      { t: [T.fly5[0] - 0.1, T.fly5[1] + 0.6], to: { x: 700, y: 260, z: 1.45 } },
      // the staging line, its menu, its question
      { t: [T.sel[0] - 1.0, T.sel[1] + 0.3], to: { x: 800, y: b3.y + 110, z: 1.75 } },
      { t: [T.carry2 - 0.1, T.open2 + 0.8], to: { x: 805, y: a2.y + 170, z: 1.9 } },
      // writing back: the open tag as it grows
      { t: [T.openTag - 0.1, T.focus + 1.0], to: { x: 810, y: a2.y + 250, z: 1.7 } },
      { t: [T.breath7 - 0.3, T.tie7 + 0.2], to: { x: 950, y: a2.y + 30, z: 2.4 } },
      { t: [T.fly7[0] - 0.1, T.fly7[1] + 0.6], to: { x: 700, y: 270, z: 1.45 } },
      // the end: back to the whole page
      { t: [ED.back[0], ED.back[1] + 0.4], to: { x: 660, y: 330, z: 1.2 } },
    ];
  }

  // ======================================================================= captions
  const CAPS = [];
  for (const mo of PLAN) if (mo.caption) { const [a, b, text, kind] = mo.caption; CAPS.push({ t0: CHS[mo.n] + a, t1: CHS[mo.n] + b, text, kind: kind || "title" }); }
  // ======================================================================= the end: Initial
  // The last knot leaves the page's border: the cloth comes up behind it, the icon's tile is
  // laid down and stitched round, the i is satin-stitched under the knot, and the knot
  // settles in as its dot. Then the wordmark is sewn on beside it.
  // Drawn from design/logo/03-initial (data/logo.js), the dark icon: the film ends in Lamplight.
  let LG = null;
  const ICON = { x: 470, y: 330, size: 400 };
  function buildLogo() {
    const P = new DOMParser();
    const doc = P.parseFromString(window.LOGO.iconDark, "image/svg+xml").documentElement;
    doc.querySelectorAll("style").forEach((nd) => nd.remove());
    const html = (el) => el.outerHTML.replace(/ xmlns="[^"]*"/g, "");
    const parts = { tile: "", stitches: [], outlines: [], shadow: "", stem: "", dot: [], edge: "", tileClip: "" };
    const clip = doc.querySelector("defs clipPath#tile path");
    parts.tileClip = clip ? clip.getAttribute("d") : "";
    for (const el of doc.children) {
      if (el.tagName === "defs") continue;
      if (el.tagName === "path" && !el.getAttribute("stroke")) parts.tile = html(el);
      else if (el.tagName === "path") parts.edge = html(el);
      else if (el.tagName === "g") {
        for (const c of el.children) {
          const fill = (c.getAttribute("fill") || "").toUpperCase(), stroke = (c.getAttribute("stroke") || "").toUpperCase();
          if (c.tagName === "path" && fill === "#48424C") {
            const nums = (c.getAttribute("d").match(/-?\d+(\.\d+)?/g) || []).map(Number);
            const ang = Math.atan2(nums[1] - 512, nums[0] - 512);
            parts.stitches.push({ html: html(c), ang: (ang + Math.PI / 2 + 2 * Math.PI) % (2 * Math.PI) });
          } else if (c.tagName === "path" && stroke === "#38343E") parts.outlines.push(html(c).replace("<path", '<path pathLength="1"'));
          else if (c.tagName === "path" && fill === "#100F11") parts.shadow = html(c);
          else if (c.tagName === "g") parts.stem = html(c).replace(/id="st"/g, 'id="end-st"').replace(/url\(#st\)/g, "url(#end-st)");
          else parts.dot.push(html(c));
        }
      }
    }
    parts.stitches.sort((a, b) => a.ang - b.ang);
    // the wordmark: letters in ink; its i in madder, satin-stitched, dotted with a knot
    const wm = P.parseFromString(window.LOGO.wordmark, "image/svg+xml").documentElement;
    wm.querySelectorAll("style").forEach((nd) => nd.remove());
    const vb = wm.getAttribute("viewBox").split(/\s+/).map(Number);
    const fill = { ink: "var(--ink)", mad: "var(--madder)", "mad-hi": "color-mix(in srgb,var(--madder) 62%,#fff)", "mad-lo": "color-mix(in srgb,var(--madder) 70%,#000)" };
    const recolour = (s) => s.replace(/class="(ink|mad|mad-hi|mad-lo)"/g, (_, c) => `style="fill:${fill[c]}"`).replace(/class="s-hi"/g, 'style="stroke:color-mix(in srgb,var(--madder) 55%,#fff)"').replace(/class="s-lo"/g, 'style="stroke:color-mix(in srgb,var(--madder) 70%,#000)"');
    const word = [], dot = [];
    for (const el of wm.children) {
      if (el.tagName === "circle" || (el.tagName === "path" && el.getAttribute("class") === "mad-hi")) dot.push(recolour(html(el)));
      else word.push(recolour(html(el)).replace(/id="sw"/g, 'id="end-sw"').replace(/url\(#sw\)/g, "url(#end-sw)"));
    }
    LG = { parts, wm: { vb, word: word.join(""), dot: dot.join("") } };
  }
  /** The dot of the icon's i, on screen: where the last knot settles. */
  const dotAt = () => { const s = ICON.size / 1024; return { x: ICON.x + 503.1 * s, y: ICON.y + 246.2 * s, r: 88.3 * s }; };
  function endHTML(t, cam, geo) {
    if (t < ED.lift[0] - 0.05 || !LG) return "";
    const s = ICON.size / 1024, P = LG.parts, D = dotAt();
    let html = "";
    // the cloth comes up behind the knot
    const plate = E.io(prog(t, ...ED.plate));
    if (plate > 0) html += `<div style="position:absolute;inset:0;background-color:var(--cloth);background-image:var(--weave);background-size:48px 48px;opacity:${fx(plate)}"></div>`;
    html += `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible"><defs><clipPath id="end-tile"><path d="${P.tileClip}"/></clipPath>`;
    const satin = prog(t, ...ED.satin);
    html += `<clipPath id="end-sew"><rect x="0" y="0" width="1024" height="${fx(150 + 760 * E.io(satin))}"/></clipPath></defs>`;
    // the tile, laid down
    const tk = E.out(prog(t, ...ED.tile));
    if (tk > 0) {
      const sc = 0.94 + 0.06 * tk;
      html += `<g transform="translate(${fx(ICON.x + 512 * s * (1 - sc))} ${fx(ICON.y + 512 * s * (1 - sc))}) scale(${fx(s * sc)})" opacity="${fx(tk)}">${P.tile}`;
      // its edge, stitched round from the top
      const bk = prog(t, ...ED.border), n = P.stitches.length;
      html += `<g clip-path="url(#end-tile)">`;
      P.stitches.forEach((st, i) => { const k = clamp(bk * (n + 6) - i * 1.0, 0, 1); if (k > 0) html += `<g opacity="${fx(k)}">${st.html}</g>`; });
      P.outlines.forEach((o) => { html += o.replace("<path", `<path style="stroke-dasharray:1 1;stroke-dashoffset:${fx(1 - E.io(bk))}"`); });
      // the i, satin-stitched from the top down
      if (satin > 0) html += `<g clip-path="url(#end-sew)">${P.shadow}${P.stem}</g>`;
      // the dot, once the knot has settled into it
      const dk = prog(t, ED.settle, ED.settle + 0.25);
      if (dk > 0) html += `<g opacity="${fx(dk)}">${P.dot.join("")}</g>`;
      html += `</g>${P.edge}</g>`;
      if (satin > 0 && satin < 1) {
        const ny = ICON.y + (150 + 760 * E.io(satin)) * s;
        html += `<g transform="translate(${fx(ICON.x + 560 * s)} ${fx(ny)}) scale(2.6)">${A.NEEDLE(0, -7, (t * 2.4) % 1, "var(--graphite)")}</g>`;
      }
    }
    // the knot: off the thread's end, up to where the dot will be, and in
    // from the last motif in the page's border (two motifs, 22 apart, centred on the band)
    const from = toScreen(camAt(ED.lift[0]), 671, 68);
    const lk = SMOOTH(prog(t, ...ED.lift));
    const kx = lerp(from[0], D.x, lk), ky = lerp(from[1], D.y, lk) - 120 * Math.sin(Math.PI * lk);
    const kr = lerp(3.2 * camAt(ED.lift[0]).z, D.r, SMOOTH(lk));
    const pop = t >= ED.settle ? knotScale(t - ED.settle + 0.12) : 1;
    const ko = 1 - prog(t, ED.settle + 0.05, ED.settle + 0.3);
    if (ko > 0) html += `<g class="endknot" opacity="${fx(ko)}">${knotSVG(kx, ky, kr, "var(--voice)", pop)}</g>`;
    // the wordmark, sewn on beside the icon, and its knot
    const W = LG.wm, ws = 600 / W.vb[2], wx = 940, wy = 578; // wy: the baseline
    const rk = prog(t, ...ED.word);
    if (rk > 0) {
      const reveal = W.vb[0] + E.io(rk) * (W.vb[2] + 10);
      html += `<defs><clipPath id="end-wm"><rect x="${fx(W.vb[0] - 5)}" y="${fx(W.vb[1] - 5)}" width="${fx(Math.max(0, reveal - W.vb[0] + 5))}" height="${fx(W.vb[3] + 10)}"/></clipPath></defs>`;
      html += `<g transform="translate(${fx(wx - W.vb[0] * ws)} ${fx(wy)}) scale(${fx(ws)})"><g clip-path="url(#end-wm)">${W.word}</g>`;
      const dk = knotScale(t - ED.wdot);
      if (dk > 0) html += `<g transform="translate(13.8 -60.9) scale(${fx(dk)}) translate(-13.8 60.9)">${W.dot}</g>`;
      html += `</g>`;
      if (rk < 1) { const nx = wx + (reveal - W.vb[0]) * ws; html += `<g transform="translate(${fx(nx)} ${fx(wy + 40)}) scale(2.4)">${A.NEEDLE(0, -7, (t * 2.4) % 1, "var(--graphite)")}</g>`; }
    }
    html += `</svg>`;
    // one line under the mark: the film's first words, again
    const lk2 = prog(t, ...ED.line);
    if (lk2 > 0) {
      const line = "A notebook that asks.";
      const reveal = E.io(lk2) * 1300;
      html += `<div style="position:absolute;left:0;right:0;top:${fx(800 + (1 - E.out(lk2)) * 8)}px;text-align:center;font:400 38px/1.3 Figtree;color:var(--graphite);-webkit-mask-image:linear-gradient(90deg,#000 ${fx(310 + reveal - 40)}px,transparent ${fx(310 + reveal)}px)">${line}</div>`;
    }
    return html;
  }


  // ======================================================================= the cold open
  function coldOpenFX(t, cam) {
    if (t > T.pull1 + 0.1) return "";
    // one running stitch across the cloth, under the words; when the camera tilts down to
    // the window it settles onto the sheet's border, which is where every tie-off will go
    const sewK = E.io(prog(t, T.sew0, T.sew1));
    const morph = SMOOTH(prog(t, T.pull0, T.pull1 - 0.15));
    const b0 = toScreen(cam, 66, 68), b1 = toScreen(cam, 1254, 68);
    const X0 = lerp(330, b0[0], morph), X1 = lerp(1590, b1[0], morph), Y = lerp(548, b0[1], morph);
    const n = 104;
    let s = "";
    const head = sewK * n;
    for (let i = 0; i < n; i++) {
      if (i > head) break;
      const k = Math.min(1, head - i);
      const u0 = i / n, u1 = (i + 0.64) / n;
      const xa = lerp(X0, X1, u0), xb = lerp(X0, X1, lerp(u0, u1, k));
      const yy = Y + A.jitter(5, i) * lerp(1.6, 0.4, morph);
      s += capsule(xa, yy, xb, yy, lerp(3.2, 1.6 * cam.z, morph), morph > 0.5 ? "color-mix(in srgb,var(--faint) 70%,transparent)" : "var(--voice)", 1 - prog(t, T.pull1 - 0.35, T.pull1));
    }
    if (sewK > 0 && sewK < 1) s += `<g transform="translate(${fx(lerp(X0, X1, sewK))} ${fx(Y)}) scale(2.2)">${A.NEEDLE(1, -7, (t * 2.6) % 1, "var(--graphite)")}</g>`;
    // the loose end, until the tie
    if (t >= T.sew1 && t < T.openTie + 0.25) {
      const e = [X1, Y];
      const r = A.simRope("tail", e, [e[0] + 70, e[1] + 12], T.sew1, Math.min(t, T.openTie - 0.25), { cutAt: T.sew1 + 0.05, opts: { maxSlack: 40 } });
      s += A.ropeSVG(r, "var(--voice)", 0.95 * (1 - prog(t, T.openTie - 0.2, T.openTie)));
    }
    if (t >= T.openTie) s += knotSVG(X1 + 4, Y, lerp(6.5, 3.2 * cam.z, morph), "var(--voice)", knotScale(t - T.openTie));
    return `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible">${s}</svg>` + openTitleHTML(t);
  }
  function openTitleHTML(t) {
    const k = prog(t, T.sew0 + 0.2, T.sew1 + 0.2);
    if (t > T.lineUp + 0.4) return "";
    const words = "A notebook that asks.".split(" ");
    const out = 1 - E.in(prog(t, T.lineUp, T.lineUp + 0.32));
    const lift = -60 * E.in(prog(t, T.lineUp, T.lineUp + 0.32)) + 4 * Math.sin(SMOOTH(prog(t, T.openTie - 0.12, T.openTie + 0.1)) * Math.PI);
    let html = `<div style="position:absolute;left:0;right:0;top:${fx(400 + lift)}px;text-align:center;font:500 108px/1 'Fraunces Display';letter-spacing:-.02em;color:var(--ink);opacity:${fx(out)}">`;
    words.forEach((w, i) => {
      const wk = clamp((k * words.length - i) * 1.4);
      const rise = (1 - spring(Math.max(0, (k * words.length - i)) * 0.5, 160, 25.3)) * 26;
      html += `<span style="display:inline-block;opacity:${fx(wk)};transform:translateY(${fx(rise)}px)">${esc(w)}</span> `;
    });
    return html + `</div>`;
  }

  // ======================================================================= the film's layer
  function patch(x, y, w, h, op, rot) {
    return `<div style="position:absolute;left:${fx(x)}px;top:${fx(y)}px;width:${fx(w)}px;height:${fx(h)}px;border-radius:14px;background:color-mix(in srgb,var(--card) 94%,transparent);box-shadow:0 2px 0 rgba(var(--shade),.05),0 22px 40px -18px rgba(var(--shade),.55);opacity:${fx(op)};transform:rotate(${fx(rot || 0)}deg)"><div style="position:absolute;inset:6px;border-radius:10px;border:1.5px dashed color-mix(in srgb,var(--voice) 46%,transparent)"></div></div>`;
  }
  const measCtx = document.createElement("canvas").getContext("2d");
  function textW(text, font) { measCtx.font = font; return measCtx.measureText(text).width; }
  function captionsHTML(t) {
    let html = "";
    for (const c of CAPS) {
      if (t < c.t0 || t > c.t1 + 0.05) continue;
      const big = c.kind === "title";
      const size = c.size || (big ? 50 : 32);
      const font = big ? `500 ${size}px 'Fraunces Display'` : `500 ${size}px Figtree`;
      const tw = textW(c.text, font);
      const keyW = c.keys ? c.keys.k.length * 62 + 26 : 0;
      const padX = big ? 38 : 30, h = big ? 104 : 80;
      const w = tw + padX * 2 + keyW;
      const x = 96, y = 1080 - 92 - h;
      // in: the patch is sewn on (it unfolds from its top edge), the words stitch in behind a needle
      const inK = spring(t - c.t0, 300, 34.7);
      const sew = prog(t, c.t0 + 0.08, c.t0 + 0.08 + clamp(0.3 + c.text.length * 0.012, 0.45, 0.95));
      // out: tied off. The thread under the words pulls taut, a knot pops, the patch lifts away
      const tie0 = c.t1 - 0.36, taut = prog(t, tie0, tie0 + 0.14), pop = t - (tie0 + 0.14), yank = prog(t, c.t1 - 0.17, c.t1);
      const dy = -4 * Math.sin(Math.PI * SMOOTH(prog(t, tie0 + 0.1, c.t1 - 0.17))) - 70 * E.in(yank);
      const op = (1 - E.in(yank));
      const sy = clamp(inK, 0, 1.2);
      html += `<div style="position:absolute;left:0;top:0;transform:translate(${fx(x)}px,${fx(y + dy)}px)">` + `<div style="transform-origin:0 0;transform:scaleY(${fx(sy)})">` + patch(0, 0, w, h, op) + `</div>`;
      // words, revealed behind the needle
      const reveal = E.io(sew) * (tw + 40);
      html += `<div style="position:absolute;left:${padX}px;top:${big ? 22 : 20}px;white-space:nowrap;font:${font};letter-spacing:${big ? "-.012em" : "-.004em"};color:var(--ink);opacity:${fx(op)};` +
        `-webkit-mask-image:linear-gradient(90deg,#000 ${fx(reveal - 28)}px,transparent ${fx(reveal)}px)">${esc(c.text)}</div>`;
      // the running stitch under the words, sewn behind the needle, pulled straight at the tie
      let st = "";
      const ly = h - (big ? 22 : 19);
      const stitches = runningStitch(padX, padX + tw, ly, (c.t0 * 1000) | 0, false);
      stitches.forEach((s, i) => {
        const sx1 = s.x1 + (taut > 0 ? (stitches[i + 1] ? stitches[i + 1].x0 - s.x1 : 0) * E.out(taut) : 0);
        if (s.x0 > reveal) return;
        st += capsule(s.x0, ly, Math.min(sx1, reveal), ly, big ? 2.4 : 2, "var(--voice)", op);
      });
      if (sew > 0 && sew < 1) st += `<g transform="translate(${fx(padX + reveal - 30)} ${fx(ly)}) scale(1.5)">${A.NEEDLE(0, -7, (t * 2.4) % 1, "var(--graphite)")}</g>`;
      if (pop > 0) st += knotSVG(padX + tw + 6, ly, big ? 5 : 4.2, "var(--voice)", knotScale(pop) * op);
      html += `<svg width="${fx(w + 40)}" height="${h + 20}" style="position:absolute;left:0;top:0;overflow:visible">${st}</svg>`;
      if (c.keys) html += keysInline(t, c.keys, padX + tw + 26, (h - 52) / 2, op);
      html += `</div>`;
    }
    return html;
  }
  function keysInline(t, K, x, y, op) {
    const pressed = (at) => within(t, at, at + 0.22) || (K.hold && within(t, at, at + K.hold));
    const ks = K.again && t >= K.again - 0.05 && K.k2 ? K.k2 : K.k;
    const at = K.again && t >= K.again - 0.05 ? K.again : K.at;
    const down = pressed(at);
    return `<div style="position:absolute;left:${fx(x)}px;top:${fx(y)}px;display:flex;gap:8px;opacity:${fx(op * clamp((t - K.at + 0.6) * 4))}">` +
      ks.map((k) => `<span style="min-width:52px;height:52px;padding:0 12px;display:grid;place-items:center;border-radius:11px;font:500 25px/1 Figtree;color:var(--ink);background:var(--paper);border:1px solid var(--line2);box-shadow:${down ? "0 1px 0" : "0 4px 0"} var(--line2);transform:translateY(${down ? 3 : 0}px)">${esc(k)}</span>`).join("") + `</div>`;
  }

  // ======================================================================= frames
  /** Renders one shot of the desk into a scene layer. */
  function renderShot(layer, t, cam) {
    A.applyTheme(layer, 0, "madder", { note: 17, lettering: "own" });
    const world = layer.querySelector(".world");
    world.style.transform = `translate(${960 - cam.x * cam.z}px, ${540 - cam.y * cam.z}px) scale(${cam.z})`;
    world.innerHTML = `<div class="desk"></div><div class="win"></div>`;
    const page = mainNote(t);
    return A.renderWindow(world.querySelector(".win"), { voice: "madder", page, chromeOp: page.header.op }, t, cam.z);
  }

  let lastGeo = null;
  function renderAt(t) {
    const cam = camAt(t);
    const A_ = $("#sceneA");
    $("#sceneB").style.display = "none";
    const geo = renderShot(A_, t, cam);
    lastGeo = geo;
    // the film ends in Lamplight, on the mark
    const dark = prog(t, ED.plate[0], ED.plate[1]);
    A.applyTheme($("#stage"), dark, "madder", {});
    $("#fx").innerHTML = coldOpenFX(t, cam);
    $("#film").innerHTML = captionsHTML(t) + flyHTML(t, cam, geo) + endHTML(t, cam, geo);
    renderPointer(t, cam);
    $("#fade").style.opacity = String(fx(Math.max(1 - prog(t, 0, 0.6), prog(t, DUR - 0.6, DUR))));
  }

  /** A tied thread's knot flies up to the page's border. */
  function flyHTML(t, cam, geo) {
    let s = "";
    for (const [id, span] of [["t1", T.fly5], ["t2", T.fly7]]) {
      if (!within(t, ...span) || !geo) continue;
      const g = geo.tags[id];
      const from = g && g.ropeEnd ? g.ropeEnd : [1000, 300];
      const k = SMOOTH(prog(t, ...span));
      const wx = lerp(from[0], 660, k), wy = lerp(from[1], 68, k) - 60 * Math.sin(Math.PI * k);
      const p = toScreen(cam, wx, wy);
      s += knotSVG(p[0], p[1], 3.2 * cam.z, "var(--voice)", 1);
    }
    return s ? `<svg class="fly" width="1920" height="1080" style="position:absolute;inset:0;overflow:visible">${s}</svg>` : "";
  }

  // ======================================================================= the pointer
  // A hand on a mouse. It shows when the hand reaches for it, moves from point to point (a
  // smootherstep on a slight arc), dips where it clicks, and fades when typing starts or the
  // hand lets go. Between moves it stays put on the page, as a real pointer does, so every
  // target is taken once, where its move ends (freezePointer, at boot). A move with no target
  // is a click in place; the cursor takes its target's shape past mid-move.
  function pointerSessions() {
    const tag = (id, dx = 26, dy = 13) => (G) => { const g = G.tags[id]; return g ? [g.x + dx, g.y + dy] : null; };
    const word = (id, f, line = 0) => () => { const Ls = MEAS.main.anchors[id] || []; const L = Ls[line < 0 ? Ls.length + line : line]; return L ? [lerp(L.x, L.r, f), L.y + 15] : null; };
    const panel = (id, dx, dy) => (G) => { const g = G.tags[id]; return g && g.panel ? [g.panel.x + (dx >= 0 ? dx : g.panel.w + dx), g.panel.y + g.panel.h + dy] : null; };
    const link = (G) => { const r = G.links && G.links[EDIT.from]; return r ? [r.x + r.w / 2, r.y + r.h / 2] : null; };
    const act = (k) => (G) => { const r = G.acts && G.acts[k]; return r ? [r.x + r.w * 0.35, r.y + r.h / 2] : null; };
    // out of the tag's way while you read it: in the gap between the words and the tag, under the thread
    const aside = (id) => tag(id, -22, 64);
    return [
      // open the correction, read it, point at its words in the working, click to select them
      { show: T.hover1 - 1.5, hide: T.type[0], moves: [
        [T.hover1 - 0.9, T.hover1, tag("t1"), { press: true }], [T.hover1 + 0.35, T.hover1 + 1.25, aside("t1")],
        [T.link - 0.9, T.link, link], [T.click, T.click, null, { press: true }]] },
      // select the staging line, find its question, open it; going back in shows the reply
      // field, and a click there starts the reply
      { show: T.sel[0] - 1.5, hide: T.typeReply[0], moves: [
        [T.sel[0] - 0.9, T.sel[0], word("t2", 0), { ibeam: true }], [T.sel[0], T.sel[1], word("t2", 1), { ibeam: true }],
        [T.sel[1], T.find, act("find"), { press: true }], [T.open2 - 0.8, T.open2, tag("t2"), { press: true }],
        [T.open2 + 0.35, T.open2 + 1.25, aside("t2")], [T.openTag - 0.3, T.focus, panel("t2", 60, -60), { press: true, ibeam: true }]] },
      // the hand comes back to the mouse, where it left it in the reply, to resolve the thread
      { show: T.resolveHov - 1.3, hide: T.knot7 + 0.6, from: [T.focus, panel("t2", 60, -60)], ibeam: true, moves: [
        [T.resolveHov - 0.6, T.resolveHov, panel("t2", 30, -20), { press: true }]] },
    ];
  }
  let POINTER = [];
  /** Takes every target where its move ends, by laying the page out at that moment. */
  function freezePointer() {
    const at = (t, fn) => fn(renderShot($("#sceneA"), t, camAt(t)) || { tags: {} });
    POINTER = pointerSessions().map((S) => ({
      ...S,
      from: S.from ? at(S.from[0], S.from[1]) : null,
      moves: S.moves.map(([a, b, fn, o = {}]) => ({ a, b, p: fn ? at(b, fn) : null, o })),
    }));
  }
  function pointerPath(t) {
    for (const S of POINTER) {
      if (t < S.show || t > S.hide + 0.2) continue;
      const q0 = S.moves[0].p;
      let p = S.from || (q0 ? [q0[0] + 90, q0[1] + 120] : null), ibeam = !!S.ibeam, press = 0;
      for (const { a, b, p: q, o } of S.moves) {
        if (t < a) break;
        if (o.press && within(t, b, b + 0.12)) press = 1;
        if (!q) continue;
        if (!p) { p = q; ibeam = !!o.ibeam; continue; }
        const k = SMOOTH(prog(t, a, b)), arc = Math.sin(Math.PI * k) * 14;
        p = [lerp(p[0], q[0], k) + arc * 0.3, lerp(p[1], q[1], k) - arc];
        if (k > 0.6) ibeam = !!o.ibeam;
        if (k < 1) break;
      }
      const op = Math.min(prog(t, S.show, S.show + 0.25), 1 - prog(t, S.hide, S.hide + 0.2));
      return p && op > 0 ? { p, ibeam, press, op } : null;
    }
    return null;
  }
  function renderPointer(t, cam) {
    const el = $("#cursor");
    const P = pointerPath(t);
    if (!P) { el.style.display = "none"; return; }
    const s = toScreen(cam, P.p[0], P.p[1]);
    el.style.display = "block";
    el.style.opacity = fx(P.op);
    el.style.transform = `translate(${s[0]}px, ${s[1]}px) scale(${1.25 * (P.press ? 0.9 : 1)})`;
    el.innerHTML = P.ibeam
      ? `<svg width="14" height="26" viewBox="-7 -13 14 26"><path d="M-4 -11h8M-4 11h8M0 -11v22" style="stroke:#fff;stroke-width:3.4;stroke-linecap:round"/><path d="M-4 -11h8M-4 11h8M0 -11v22" style="stroke:#111;stroke-width:1.4;stroke-linecap:round"/></svg>`
      : `<svg width="22" height="30" viewBox="0 0 22 30"><path d="M1.5 1.5v22.5l6-5.4 3.8 8.6 3.6-1.6-3.8-8.4 8-.6z" style="fill:#111;stroke:#fff;stroke-width:1.5;stroke-linejoin:round"/></svg>`;
  }

  // ======================================================================= the score's cues
  // One quiet piece under the whole film (audio.js): its harmony moves where each moment
  // begins (PLAN's chords), and it marks only the two tie-offs and the last knot.
  const EVENTS = [];
  function buildEvents() {
    for (const mo of PLAN) EVENTS.push({ type: "chapter", t: CHS[mo.n], n: mo.n, name: mo.name });
    EVENTS.push({ type: "tie", t: T.knot5 }, { type: "tie", t: T.knot7 }, { type: "settle", t: ED.settle }, { type: "fin", t: DUR });
    EVENTS.sort((a, b) => a.t - b.t);
  }

  // ======================================================================= boot
  const MOMENTS = PLAN.map((mo) => ({ n: mo.n, id: mo.id, name: mo.name, t: CHS[mo.n], dur: mo.dur, caption: mo.caption ? mo.caption[2] : null }));
  const FILM = { FPS, DUR, T, ED, PLAN, CHS, MOMENT, ready: false, error: null, renderAt, events: EVENTS, moments: MOMENTS, chapters: [], camAt, real: { note: N } };
  window.FILM = FILM;
  async function boot() {
    try {
      const mm = document.createElement("div");
      mm.id = "measure";
      mm.style.cssText = "position:absolute;left:0;top:0;width:1320px;height:860px;visibility:hidden;overflow:hidden";
      mm.innerHTML = '<div class="page"></div>';
      $("#stage").appendChild(mm);
      A.applyTheme(mm, 0, "madder", {});
      await document.fonts.ready;
      await Promise.all(["400 17px 'Fraunces Text'", "500 17px 'Fraunces Text'", "600 30px 'Fraunces Display'", "500 30px 'Fraunces Display'", "400 14px Figtree", "500 14px Figtree", "600 14px Figtree", "400 13px Figtree"].map((f) => document.fonts.load(f)));
      A.measureWordmark();
      buildLogo();
      // the note, finished, for the camera's targets
      const full = { blocks: [["p1", P1], ["p2", P2], ["p3", P3]].map(([id, text]) => ({ id, kind: "p", text, ranges: [] })), threads: [], header: {}, title: { text: TITLE }, border: {} };
      const rng2 = (bid, phrase, id) => { const b = full.blocks.find((x) => x.id === bid); const i = b.text.indexOf(phrase); if (i < 0) throw new Error(`no «${phrase}» in ${bid}`); b.ranges.push({ a: i, b: i + phrase.length, id }); full.threads.push({ id, block: bid, kind: "question", noTag: true, sew: 0 }); };
      rng2("p2", TXT.corrQuote, "t1"); rng2("p3", TXT.probeQuote, "t2");
      measureNote("main", full);
      buildCamera();
      buildEvents();
      freezePointer();
      FILM.ready = true;
      const q = new URLSearchParams(location.search);
      if (q.has("t")) renderAt(+q.get("t"));
      else if (q.has("play")) { const t0 = performance.now() - (+q.get("play") || 0) * 1000; const loop = () => { renderAt(Math.min(DUR, (performance.now() - t0) / 1000)); requestAnimationFrame(loop); }; loop(); }
      else renderAt(0);
    } catch (e) {
      FILM.error = String((e && e.stack) || e);
      console.error(e);
    }
  }
  boot();
})();

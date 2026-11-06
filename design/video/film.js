/* Inkling: the walkthrough film, re-cut on 7 Nov 2026 around one technical note, a plan to
 * get VM cold starts under two seconds.
 *
 * Every frame is renderAt(t), a pure function of time. The rope is the one simulation, and
 * it is stepped at a fixed rate from the start of each motion, so frame 4321 comes out the
 * same whichever worker renders it.
 *
 * The film is a desk of linen with the app's windows on it and a virtual camera moving over
 * them: pushing in to the words as a thread is pulled, tracking the thread to its tag,
 * pulling back to the sheet. Captions are stitched in and tied off; chapters hang in on
 * woven labels whose knots fly down to the sampler border at the foot of the frame, which
 * is the film's progress bar. Every word Inkling or another mind says is the brain's real
 * output (data/real.js, from tools/real.ts). The timeline is laid out from PLAN, chapters of
 * moments end to end, so a moment can be added anywhere and the rest moves with it.
 * FILM.events is the short cue list the score (audio.js) reads; FILM.coverage maps
 * design/capabilities.md.
 */
(function () {
  "use strict";
  const A = window.APP, { fx, clamp, lerp, prog, esc, E, spring, knotSVG, knotScale, motifSVG, capsule, runningStitch } = A;
  const REAL = window.REAL, C = REAL.calls, N = REAL.note, START = window.START;
  const FPS = 30, BEAT = 0.625;
  const $ = (s, r) => (r || document).querySelector(s);
  const within = (t, a, b) => t >= a && t < b;
  const rng = A.rng;
  const out = (name) => C[name].output;

  // ======================================================================= words (all real)
  // The writer's words are the note (tools/real.ts holds them, with the context of every
  // call); Inkling's and the other minds' are the brain's, unedited.
  const TITLE = N.title;
  const [P1, P2, P3, P4, P5, P6, P7] = N.paras;
  const FIRST = N.first;
  const EDIT1 = N.edits[0], EDIT2 = N.edits[1];
  const MIND = N.mind; // the writer's own mind, made in chapter 5
  const TXT = {
    first: out("first").text, correction: out("read_p2").text, corrQuote: out("read_p2").quote, settle: out("settle").text,
    probe: out("probe_p3").text, probeQuote: out("probe_p3").quote,
    ask: out("ask").text, askQ: N.ask.question, askTarget: N.ask.target,
    room: { inkling: out("room_inkling").text, staff: out("room_staff").text, tutor: out("room_tutor").text },
    roomQ: N.room.question, roomTarget: N.room.target,
    call: out("call").text, chat: out("chat").text, chatQ: N.chat,
    reply: N.reply, answer: out("reply").text,
    mind: out("mind").text, mindQuote: out("mind").quote, brief: N.brief,
    idea: out("sketch").text, ideaQuote: out("sketch").quote, svg: out("draw").svg,
    learned: out("learn").aboutMe,
  };
  // the rest of the notebook: other notes are only seen by title, in the sidebar and ⌘K.
  // "Boot traces, week 44" is the note this one leans on (and contradicts: it says 6 GiB).
  const NOTEBOOK = {
    folders: [
      ["Platform", [TITLE, "Boot traces, week 44", "Kernel upgrade notes"]],
      ["Home", ["Kitchen shelf measurements", "Why the bread didn’t rise"]],
      ["Reading", ["Reading list for winter"]],
    ],
    loose: ["Questions for the landlord", "Start here"],
    related: { title: "Boot traces, week 44", paras: [
      "Traced 2,000 cold starts on Tuesday. At p99, boot is 7.4 s of the 9 s; placement and disk attach take 0.9 s, and the shell handshake 0.7 s.",
      "Since the October kernel update, the default image’s memory snapshot is 6 GiB.",
    ] },
    vault: [["Runbooks", ["Disk full", "Rotate the certs", "Node won’t boot"]], ["Postmortems", ["Slow deploys", "The noisy neighbour"]]],
  };
  // what the writer adds at the end of the note before the review (chapter 7)
  const SEWN = [
    { id: "s1", kind: "check-done", text: "Trace the 9 s, stage by stage" },
    { id: "s2", kind: "check", text: "Restore on one staging host" },
    { id: "s3", kind: "check", text: "Replay the worst-burst days" },
    { id: "s4", kind: "callout-warning", text: "A snapshot only restores on a host with the same CPU model and kernel." },
    { id: "s5", kind: "code-sh", text: "# where the 9 s goes, one launch at a time" },
    { id: "s6", kind: "code-sh", text: "systemd-analyze critical-chain" },
  ];

  // ======================================================================= the timeline
  // PLAN is the film: chapters of moments, laid end to end. A moment is [id, length,
  // cues(start)], its cues measured from its own start; a montage is a list of beats.
  // Chapter starts (c1..c7) and the end (cEnd) fall out of the layout.
  const T = {}, M6 = {}, M7 = {}, ED = {}, MOMENT = {};
  const m = (id, dur, cues) => ({ id, dur, cues: cues || (() => ({})) });
  const beats = (into, list) => list.map(([id, dur]) => m(id, dur, (s) => ({ [id]: s, __into: into })));
  const PLAN = [
    { n: 0, moments: [m("open", 8.75, (s) => ({ pierce: s + 0.9, sew0: s + 1.0, sew1: s + 3.2, tail: s + 3.25, openTie: s + 5.625, lineUp: s + 5.9, pull0: s + 6.2, pull1: s + 8.9 }))] },
    { n: 1, name: "Start here", c: "madder", x: 1430, tag: -0.6, moments: [
      m("guide", 6.1, (s) => ({ glide0: s + 0.2, glide1: s + 5.6 })),
      m("first", 14.9, (s) => ({ type1: [s, s + 3.3], read1: [s + 3.8, s + 4.9], sew1a: s + 4.9, sew1b: s + 6.0, carry1: s + 6.05, hover1: s + 7.1 })),
    ] },
    { n: 2, name: "Reading along", c: "madder", x: 1420, tag: 0.35, moments: [
      m("write", 14.25, (s) => ({ title2: [s + 0.45, s + 1.85], p1: [s + 2.15, s + 5.35], read2a: [s + 5.55, s + 6.35], p2: [s + 6.55, s + 10.35], read2b: [s + 10.45, s + 11.45],
        p3: [s + 10.65, s + 12.25], t1arrive: s + 11.45, pause2: s + 12.25 })),
      m("correction", 4.8, (s) => ({ t1hover: s + 0.0, t1fold: s + 4.4 })),
      m("fix", 6.65, (s) => ({ e1sel: s, e1type: [s + 0.3, s + 0.9], e2drag: [s + 1.1, s + 1.7], e2type: [s + 1.9, s + 3.3], reread: [s + 3.45, s + 4.45], settleOpen: s + 4.45, settle: [s + 4.65, s + 6.1] })),
      m("tie", 2.5, (s) => ({ breath2: s, tie2: s + 0.85, knot2: s + 1.475, fly2: [s + 1.55, s + 2.35], fade2: [s + 3.05, s + 3.5] })),
      m("next", 2.8, (s) => ({ p4: [s, s + 1.9], read2d: [s + 2.05, s + 2.55] })),
    ] },
    { n: 3, name: "Asking", c: "indigo", x: 470, tag: 0, moments: [
      m("find", 7.1, (s) => ({ sel3: [s + 0.75, s + 1.35], bar3: s + 1.45, find3: s + 1.75, sew3: [s + 2.05, s + 2.85], carry3: s + 2.85, open3: s + 3.55, fold3: s + 6.8 })),
      m("ask", 7.1, (s) => ({ sel4: [s, s + 0.5], bar4: s + 0.7, askClick4: s + 1.2, type4: [s + 1.5, s + 2.7], send4: s + 2.8, sew4: [s + 2.9, s + 3.5], carry4: s + 3.5, stream4: [s + 4.2, s + 6.7], fold4: s + 6.9 })),
      m("room", 10.2, (s) => ({ sel5: [s, s + 0.5], bar5: s + 0.7, askClick5: s + 1.15, chip5a: s + 1.5, chip5b: s + 1.9, type5: [s + 2.2, s + 3.5], send5: s + 3.6,
        sew5: [s + 3.7, s + 4.3], carry5: s + 4.3, open5: s + 5.0, stream5: { inkling: [s + 5.1, s + 7.0], staff: [s + 5.3, s + 7.9], tutor: [s + 5.5, s + 8.4] }, hov5b: s + 6.7, hov5c: s + 8.3, fold5: s + 10.0 })),
      m("call", 7.2, (s) => ({ caret6: s + 0.2, p5: [s + 0.6, s + 2.4], read6: [s + 2.7, s + 3.3], sew6: [s + 3.3, s + 3.9], carry6: s + 3.9, stream6: [s + 4.6, s + 6.8] })),
      m("sketch", 14.0, (s) => ({ move7: [s, s + 0.8], sel7: [s + 0.9, s + 1.4], bar7: s + 1.5, sketch7: s + 2.0, sew7: [s + 2.1, s + 2.6], carry7: s + 2.6, think7: [s + 3.1, s + 3.8], idea7: s + 3.8,
        steps7: [s + 4.3, s + 5.25, s + 6.2, s + 7.15, s + 8.1], larger7: s + 9.3, big7: [s + 9.4, s + 13.0], bigSteps: [s + 9.65, s + 10.25, s + 10.85, s + 11.45, s + 12.05], fold7: s + 13.2 })),
      m("brief", 2.6, (s) => ({ tell8: s + 0.3, brief8: [s + 0.6, s + 2.0], save8: s + 2.2 })),
      m("chat", 5.8, (s) => ({ chatKey8: s, chatIn8: [s + 0.1, s + 0.5], pills8: s + 0.6, chatType8: [s + 1.2, s + 2.4], chatSend8: s + 2.5, chatStream8: [s + 2.9, s + 5.4], clear8: s + 5.3 })),
    ] },
    { n: 4, name: "Threads", c: "ochre", x: 1450, tag: 0.6, moments: [
      m("numbers", 3.2, (s) => ({ chatOut9: [s + 0.05, s + 0.45], numbers9: s + 1.1 })),
      m("fold", 2.8, (s) => ({ hov9: [s + 0.1, s + 0.9, s + 1.0, s + 1.8, s + 1.9, s + 2.6] })),
      m("jump", 2.2, (s) => ({ jump9: [s, s + 1.0] })),
      m("reply", 8.7, (s) => ({ drag9: [s, s + 1.0], dragOpen9: s + 1.35, replyFocus9: s + 1.9, reply9: [s + 2.05, s + 5.25], send9: s + 5.35, answer9: [s + 5.95, s + 7.65], resolveHov9: s + 8.15 })),
      m("tie", 3.55, (s) => ({ breath9: s, tie9: s + 0.65, knot9: s + 1.275, fly9: [s + 1.35, s + 2.15], fade9: [s + 2.95, s + 3.4] })),
      m("snip", 3.85, (s) => ({ open10: s, dis10: s + 0.6, snip10: s + 0.725, undo10: s + 2.1 })),
      m("reopen", 2.6, (s) => ({ reopen11: s })),
      m("hide", 2.3, (s) => ({ hide12: s, show12: s + 1.4 })),
      m("where", 2.6, (s) => ({ right13: s, marks13: s + 1.0, both13: s + 2.0, snip14: s + 2.48 })),
    ] },
    { n: 5, name: "Minds", c: "sage", x: 1460, tag: 0.1, moments: [
      m("make", 7.15, (s) => ({ chips15: s + 0.7, blank15: s + 2.3, name15: [s + 2.5, s + 3.0], desc15: [s + 3.1, s + 4.1], how15: [s + 4.2, s + 5.8], voice15: s + 6.0, often15: s + 6.4, create15: s + 6.9 })),
      m("pick", 9.725, (s) => ({ pan16: [s, s + 1.0], menu16: s + 1.2, pick16: s + 2.0, p6: [s + 3.05, s + 4.65], read16: [s + 4.85, s + 5.45], sew16: [s + 5.45, s + 5.95], carry16: s + 5.95, open16: s + 6.45 })),
    ] },
    // the montages cut on the beat: each beat is a cut, held for its length
    { n: 6, name: "Make it yours", c: "plum", x: 1420, tag: 0, moments: [m("in6", 0.625), ...beats("M6", [["dark", 2.5], ["ink", 2.5], ["letter", 1.25], ["size", 1.25], ["mode", 1.25], ["pace", 1.25],
      ["drop", 2.5], ["speak", 2.5], ["learn", 1.875], ["keys", 2.5], ["ring", 1.875]])] },
    { n: 7, name: "Your notebook", c: "walnut", x: 1430, tag: 0, moments: [m("in7", 0.625), ...beats("M7", [["side", 2.5], ["find", 1.875], ["copy", 1.25], ["exp", 2.5], ["obs", 2.5], ["help", 1.25],
      ["model", 2.5], ["claude", 1.25], ["blocks", 1.875], ["code", 1.875], ["check", 2.5], ["sewn", 2.5]])] },
    // the end: the writer's own mind's thread is tied off, and its knot becomes the dot of the i
    { n: 8, moments: [m("end", 10.5, (s) => ({ __into: "ED", act: s + 0.3, hov: s + 0.8, breath: s + 0.95, tie: s + 1.25, knot: s + 1.875, lift: [s + 1.95, s + 2.85], plate: [s + 2.0, s + 2.8],
      tile: [s + 2.9, s + 3.5], border: [s + 3.1, s + 4.5], satin: [s + 3.9, s + 5.3], settle: s + 5.45, word: [s + 5.9, s + 7.2], wdot: s + 7.35, line: [s + 7.6, s + 8.4] }))] },
  ];
  const CHS = {}; // chapter starts by number
  let DUR = 0;
  {
    let t = 0;
    for (const ch of PLAN) {
      CHS[ch.n] = t;
      for (const mo of ch.moments) {
        const cues = mo.cues(t);
        const into = cues.__into === "M6" ? M6 : cues.__into === "M7" ? M7 : cues.__into === "ED" ? ED : T;
        delete cues.__into;
        Object.assign(into, cues);
        MOMENT[`${ch.n}.${mo.id}`] = { t0: t, t1: t + mo.dur };
        t += mo.dur;
      }
    }
    DUR = t;
  }
  const c1 = CHS[1], c2 = CHS[2], c3 = CHS[3], c4 = CHS[4], c5 = CHS[5], c6 = CHS[6], c7 = CHS[7], cEnd = CHS[8];
  M6.end = c7; M7.end = cEnd;
  // cues that straddle a chapter boundary
  T.wipe1 = [c2 - 1.0, c2 + 0.35];
  T.wipe17 = [c6 - 0.5, c6 + 0.55];

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
    first: sched(FIRST, ...T.type1, 3), title: sched(TITLE, ...T.title2, 11), p1: sched(P1, ...T.p1, 12), p2: sched(P2, ...T.p2, 13), p3: sched(P3, ...T.p3, 14),
    e1: sched(EDIT1.to, ...T.e1type, 15), e2: sched(EDIT2.to, ...T.e2type, 16), p4: sched(P4, ...T.p4, 17), q4: sched(TXT.askQ, ...T.type4, 18), q5: sched(TXT.roomQ, ...T.type5, 19),
    p5: sched(P5, ...T.p5, 20), brief: sched(TXT.brief, ...T.brief8, 21), chat: sched(TXT.chatQ, ...T.chatType8, 22), reply: sched(TXT.reply, ...T.reply9, 23),
    name: sched(MIND.name, ...T.name15, 24), desc: sched(MIND.description, ...T.desc15, 25), how: sched(MIND.howItThinks, ...T.how15, 26), p6: sched(P6, ...T.p6, 27),
  };
  const ALLKEYS = [S.first, S.title, S.p1, S.p2, S.p3, S.e1, S.e2, S.p4, S.p5, S.p6];
  const typingAt = (t) => ALLKEYS.some((k) => t - lastKey(k, t) < 0.55 && t >= k[0] - 0.05);
  /** Streamed words: word by word, at the pace the window allows. */
  function streamed(text, t0, t1, t) {
    if (t <= t0) return "";
    if (t >= t1) return text;
    const words = text.split(" "), n = Math.ceil(prog(t, t0, t1) * words.length);
    return words.slice(0, n).join(" ");
  }

  // ======================================================================= tags over time
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
  // Laid out once at load (scale 1) so the camera knows where the words will be.
  const MEAS = {};
  function measureNote(name, page) {
    const host = $("#measure .page");
    const geo = A.renderPage(host, page, 0, 1);
    const res = { blocks: {}, anchors: {} };
    for (const b of page.blocks) { const bb = geo.blockBox(b.id); if (bb) res.blocks[b.id] = bb; }
    for (const th of page.threads || []) { const L = geo.lineRects(th.block, th.id); if (L.length) res.anchors[th.id] = L; }
    MEAS[name] = res;
  }

  // ======================================================================= the main note
  /** Paragraph 2 over time: typed, then fixed (the number, then what follows from it).
   *  `num` is where the corrected number ends, which is where the correction's words end. */
  function p2At(t) {
    let text = P2.slice(0, count(S.p2, t)), sel = null, caret = text.length;
    const i1 = P2.indexOf(EDIT1.from);
    const numEnd = (txt) => (txt.indexOf(EDIT1.from) >= 0 ? txt.indexOf(EDIT1.from) + EDIT1.from.length : -1);
    if (t < T.e1sel) return { text, sel, caret, num: numEnd(text) };
    if (t < T.e1type[0]) return { text, sel: [i1, i1 + EDIT1.from.length], caret: null, num: i1 + EDIT1.from.length };
    const n1 = count(S.e1, t);
    text = P2.slice(0, i1) + EDIT1.to.slice(0, n1) + P2.slice(i1 + EDIT1.from.length);
    caret = i1 + n1;
    const num = i1 + Math.max(1, n1);
    if (t < T.e2drag[0]) return { text, sel, caret, num };
    const i2 = text.indexOf(EDIT2.from);
    if (t < T.e2type[0]) {
      const k = E.sine(prog(t, T.e2drag[0], T.e2drag[1]));
      const end = Math.round(i2 + EDIT2.from.length * k);
      return { text, sel: end > i2 ? [i2, end] : null, caret: end > i2 ? null : i2, num };
    }
    const n2 = count(S.e2, t);
    return { text: text.slice(0, i2) + EDIT2.to.slice(0, n2) + text.slice(i2 + EDIT2.from.length), sel: null, caret: i2 + n2, num };
  }
  /** Settings that change during the film (Ch4 placement, Ch5 the note's mind, Ch6 your look). */
  function looks(t) {
    let ink = "madder";
    if (within(t, M6.ink, M6.letter)) { const seq = ["madder", "indigo", "sage", "rose", "plum", "walnut", "ochre", "madder"]; ink = seq[clamp(Math.floor((t - M6.ink) / (BEAT / 2)), 0, 7)]; }
    const dark = t < M6.dark + 0.85 ? 0 : t > cEnd + 1.5 ? 1 : E.io(prog(t, M6.dark + 0.85, M6.dark + 1.6));
    let note = 17;
    if (within(t, M6.size, M6.mode)) note = t < M6.size + 0.45 ? 15 : t < M6.size + 0.95 ? 19 : 17;
    const lettering = within(t, M6.letter + 0.35, M6.size) ? "notes" : "own";
    return { ink, dark, note, lettering };
  }
  const mindOf = (t) => (t >= T.pick16 + 0.1 ? MIND.name : "Inkling");
  /** A mind's thread colour at t (note_page.rs `mind_thread`). */
  const colourOf = (mind, t) => A.mindThread(mind, mindOf(t), looks(t).ink);
  /** How far the page is scrolled: only for the blocks at the foot of the note, in chapter 7. */
  const scrollAt = (t) => (within(t, M7.check, M7.end) ? MEAS.sewnScroll || 0 : 0);
  /** The checklist item ticked in chapter 7: [cross leg 1, leg 2, the tacking through]. */
  function tickAt(t) {
    const s = t - (M7.check + 1.0);
    if (s < 0) return null;
    const leg = (x) => E.out(clamp(x / 0.1));
    return { cross: [leg(s), leg(s - 0.08)], tack: clamp((s - 0.18) / 0.5) };
  }

  function mainNote(t) {
    const L = looks(t);
    const blocks = [];
    const hidden = within(t, T.hide12 + 0.05, T.show12 + 0.05);
    const hideK = E.out(prog(t, T.hide12, T.hide12 + 0.4)) * (1 - E.out(prog(t, T.show12, T.show12 + 0.4)));
    // ---- text
    const caretIn = (() => {
      if (t < T.p1[0]) return "title";
      if (t < T.p2[0]) return "p1";
      if (t < T.p3[0]) return "p2";
      if (t < T.e1sel) return "p3";
      if (t < T.e1type[0]) return null;
      if (t < T.reread[0]) return "p2";
      if (t < T.p4[0]) return null;
      if (t < T.sel3[0]) return "p4";
      if (within(t, T.caret6, T.move7[0])) return "p5";
      if (within(t, T.p6[0] - 0.3, T.open16)) return "p6";
      if (within(t, M6.speak, M6.learn)) return "p7";
      return null;
    })();
    const sel = {}; // selections, by block
    const selPhrase = (bid, text, phrase, t0, t1, until) => {
      if (!within(t, t0, until)) return;
      const i = text.indexOf(phrase);
      if (i < 0) return;
      sel[bid] = [i, Math.round(i + phrase.length * E.sine(prog(t, t0, t1)))];
    };
    const p1 = P1.slice(0, count(S.p1, t)), p2 = p2At(t), p3 = P3.slice(0, count(S.p3, t)), p4 = P4.slice(0, count(S.p4, t));
    selPhrase("p3", p3, TXT.probeQuote, ...T.sel3, T.sew3[0]);
    selPhrase("p1", p1, TXT.askTarget, ...T.sel4, T.send4);
    selPhrase("p2", p2.text, TXT.roomTarget, ...T.sel5, T.send5);
    selPhrase("p1", p1, TXT.ideaQuote, ...T.sel7, T.sew7[0]);
    const flag = "snapshot_restore", fi = P4.indexOf(flag);
    selPhrase("p4", P4, flag, M7.code + 0.2, M7.code + 0.55, M7.code + 1.0);
    if (p2.sel) sel.p2 = p2.sel;
    const gut = {};
    const add = (id, kind, text, caret, extra) => blocks.push({ id, kind, text, caret: caretIn === id ? caret : null, sel: sel[id], ranges: [], gutter: gut[id], ...(extra || {}) });
    // gutter marks: the needle while reading, a tacking stitch once read and quiet
    const needle = (a, b) => within(t, a, b);
    gut.p1 = needle(...T.read2a) ? { type: "needle" } : t >= T.read2a[1] ? { type: "tack" } : null;
    gut.p2 = needle(...T.read2b) || needle(...T.reread) ? { type: "needle" } : t >= T.read2b[1] ? { type: "tack" } : null;
    gut.p3 = t >= T.p3[1] + 0.9 ? { type: "tack" } : t >= T.p3[1] ? { type: "needle" } : null;
    gut.p4 = needle(...T.read2d) ? { type: "needle" } : t >= T.read2d[1] ? { type: "tack" } : null;
    gut.p5 = needle(...T.read6) ? { type: "needle" } : null;
    gut.p6 = needle(...T.read16) ? { type: "needle" } : null;
    if (t >= T.p1[0]) add("p1", "p", p1, p1.length);
    if (t >= T.p2[0]) add("p2", "p", p2.text, p2.caret);
    if (t >= T.p3[0]) add("p3", "p", p3, p3.length);
    // the flag is set as code (⌘E) in chapter 7; until then it is plain words
    if (t >= T.p4[0]) add("p4", "p", p4, p4.length, { code: t >= M7.code + 0.75 && p4.length >= fi + flag.length ? [[fi, fi + flag.length]] : [] });
    if (t >= T.p5[0]) { const s = P5.slice(0, count(S.p5, t)); add("p5", "p", s, s.length); }
    if (t >= T.p6[0]) { const s = P6.slice(0, count(S.p6, t)); add("p6", "p", s, s.length); }
    const spoken = within(t, M6.speak + 1.95, cEnd + 30);
    if (spoken) add("p7", "p", P7, P7.length);
    // the foot of the note, written before the review: a checklist, a warning, the command
    if (t >= c7) {
      const tick = tickAt(t);
      SEWN.forEach((b) => {
        const x = { id: b.id, kind: b.kind, text: b.text, ranges: [] };
        if (b.id === "s2" && tick) { x.kind = "check-done"; x.cross = tick.cross; x.tack = tick.tack; }
        if (b.id === "s5" || b.id === "s6") x.copy = within(t, M7.sewn + 0.9, M7.sewn + 1.5) ? "hover" : within(t, M7.sewn + 1.5, M7.end) ? "done" : null;
        blocks.push(x);
      });
    }
    if (t < T.p1[0] && t >= c2) blocks.push({ id: "p0", kind: "p", text: "", ph: "Just start writing.", caret: null });

    // ---- threads
    const threads = [];
    const placement = t >= T.right13 && t < T.marks13 ? "right" : t >= T.marks13 && t < T.both13 ? "marks" : "both";
    const sideOf = (s) => (placement === "right" ? "right" : s);
    const thread = (o) => { threads.push(o); return o; };
    const block = (id) => blocks.find((x) => x.id === id);
    // t1: the correction on the copy's arithmetic (read along), fixed and tied off
    if (t >= T.t1arrive) {
      const b = block("p2"), i = p2.text.indexOf(TXT.corrQuote.split(" ").slice(0, 3).join(" "));
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: Math.max(i + 1, p2.num), id: "t1", cls: "anc" });
        const tied = t >= T.knot2;
        const u = unfoldAt(t, [[T.t1hover, 1], [T.t1fold, 0], [T.settleOpen, 1], [T.fade2[0] + 0.3, 0]]);
        let body = A.B.msg(TXT.correction);
        if (t >= T.reread[0] && t < T.settle[0]) body += A.B.think("Reading your change", t);
        if (t >= T.settle[0]) body += A.B.msg(streamed(TXT.settle, T.settle[0], T.settle[1], t));
        const gone = t >= T.fade2[1];
        thread({ id: "t1", block: "p2", a: i, kind: "correction", c: colourOf("Inkling", t), side: sideOf("right"), sew: 1, tied, tag: !gone, rope: !gone,
          stitchOp: tied ? 0.42 : 1, ropeOp: tied ? 0.8 * (1 - prog(t, ...T.fade2)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t >= T.tie2 ? { t0: T.tie2, tieAt: T.tie2, speed: 0.5 } : null, knotAt: t >= T.knot2 ? T.knot2 : null,
          tagOp: 1 - prog(t, ...T.fade2), wash: u > 0.4 && !tied,
          tagState: { unfold: u, mind: "Inkling", kindLabel: tied ? "tied off" : "correction", body } });
      }
    }
    // t2: Find a loose thread on the staging line; answered and resolved in ch4; reopened after
    if (t >= T.sew3[0]) {
      const b = block("p3"), i = p3.indexOf(TXT.probeQuote);
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.probeQuote.length, id: "t2", cls: "anc" });
        const tied = t >= T.knot9;
        const dragK = E.out(prog(t, ...T.drag9));
        const off = [lerp(0, 92, dragK), lerp(0, 132, dragK)];
        const u = unfoldAt(t, [[T.open3, 1], [T.fold3, 0], [T.jump9[1], 0], [T.dragOpen9, 1], [T.fade9[0] - 0.1, 0], [T.reopen11, 1], [T.hide12 - 0.2, 0]]);
        let body = A.B.msg(TXT.probe);
        if (t >= T.send9) body += A.B.you(TXT.reply);
        if (t >= T.send9 && t < T.answer9[0]) body += A.B.think("", t);
        if (t >= T.answer9[0]) body += A.B.msg(streamed(TXT.answer, ...T.answer9, t));
        const active = t >= T.dragOpen9 && t < T.fade9[0];
        if (active && t < T.send9) body += A.B.reply(TXT.reply.slice(0, count(S.reply, t)), t >= T.replyFocus9, t - lastKey(S.reply, t) < 0.5 || Math.floor(t / 0.53) % 2 === 0);
        if (active || within(t, T.reopen11, T.hide12)) {
          const hov = within(t, T.resolveHov9, T.tie9) ? "resolve" : within(t, T.reopen11 + 0.5, T.reopen11 + 0.95) ? "resolve" : within(t, T.reopen11 + 0.95, T.reopen11 + 1.4) ? "copy" : within(t, T.reopen11 + 1.4, T.reopen11 + 2.2) ? "note" : null;
          body += A.B.acts(tied, hov);
        }
        const reopened = t >= T.reopen11;
        thread({ id: "t2", block: "p3", a: i, kind: "question", c: colourOf("Inkling", t), side: sideOf("right"), offset: off,
          sew: prog(t, ...T.sew3), needle: t < T.sew3[1], tied,
          stitchOp: tied ? 0.42 : 1, ropeOp: t < T.carry3 ? 0 : tied && !reopened ? 0.8 * (1 - prog(t, ...T.fade9)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry3 + 1.4 ? { t0: T.carry3, carry: 0.5 } : within(t, T.drag9[0], T.tie9) ? { t0: T.drag9[0], offsetFn: (x) => { const k = E.out(prog(x, ...T.drag9)); return [lerp(0, 92, k), lerp(0, 132, k)]; } } : within(t, T.tie9, T.fade9[1]) ? { t0: T.tie9, tieAt: T.tie9, speed: 0.5 } : null,
          knotAt: within(t, T.knot9, T.fade9[1]) ? T.knot9 : null, rope: !(tied && t >= T.fade9[1] && !within(t, T.reopen11, T.hide12)),
          tag: t >= T.carry3 + 0.45 && (!tied || t < T.fade9[1] || within(t, T.reopen11, T.hide12)), tagOp: tied && t < T.reopen11 ? 1 - prog(t, ...T.fade9) : 1, wash: u > 0.4,
          tagState: { unfold: u, mind: "Inkling", kindLabel: tied ? "tied off" : "question", body, openRight: t >= T.drag9[0] ? false : undefined } });
      }
    }
    // t3: Ask Inkling, your own question (left margin); snipped and brought back in ch4
    if (t >= T.sew4[0]) {
      const b = block("p1"), i = p1.indexOf(TXT.askTarget);
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.askTarget.length, id: "t3", cls: "anc" });
        const snipped = within(t, T.snip10, T.undo10 + 0.1);
        const u = unfoldAt(t, [[T.carry4 + 0.6, 1], [T.fold4, 0], [T.jump9[0] + 0.1, 1], [T.jump9[1], 0], [T.open10, 1], [T.snip10, 0]]);
        let body = A.B.you(TXT.askQ) + A.B.msg(streamed(TXT.ask, ...T.stream4, t));
        if (t < T.stream4[0]) body = A.B.you(TXT.askQ) + A.B.think("", t);
        const back = prog(t, T.undo10, T.undo10 + 0.45);
        thread({ id: "t3", block: "p1", a: i, kind: "ask", c: colourOf("Inkling", t), side: sideOf("left"),
          sew: snipped ? 1 : t < T.undo10 ? prog(t, ...T.sew4) : back, needle: t < T.sew4[1], unpick: snipped ? prog(t, T.snip10 + 0.15, T.snip10 + 0.95) : 0,
          ropeOp: t < T.carry4 ? 0 : snipped ? 0.7 * (1 - prog(t, T.snip10 + 0.5, T.snip10 + 0.95)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry4 + 1.4 ? { t0: T.carry4, carry: 0.5 } : snipped ? { t0: T.snip10, cutAt: T.snip10 } : within(t, T.undo10, T.undo10 + 1.2) ? { t0: T.undo10, carry: 0.45 } : null,
          tag: t >= T.carry4 + 0.45, tagOp: snipped ? 1 - prog(t, T.snip10, T.snip10 + 0.4) : t >= T.undo10 ? prog(t, T.undo10 + 0.3, T.undo10 + 0.6) : 1, wash: u > 0.4,
          tagState: { unfold: u, mind: "Inkling", kindLabel: "you asked", body: body + (within(t, T.open10, T.snip10) ? A.B.reply("", false) + A.B.acts(false, within(t, T.dis10 - 0.3, T.snip10) ? "dismiss" : null) : "") } });
      }
    }
    // t4-t6: the room on the plan's first sentence
    if (t >= T.sew5[0]) {
      const b = block("p2"), i = p2.text.indexOf(TXT.roomTarget);
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.roomTarget.length, id: "t4 t5 t6", cls: "anc" });
        [["t4", "Inkling", "inkling", [[T.open5, 1], [T.hov5b - 0.15, 0], [T.hov9[0], 1], [T.hov9[1], 0], [T.jump9[1] + 0.1, 1], [T.jump9[1] + 0.9, 0]]],
          ["t5", "Staff engineer", "staff", [[T.hov5b, 1], [T.hov5c - 0.15, 0], [T.hov9[2], 1], [T.hov9[3], 0]]],
          ["t6", "Socratic tutor", "tutor", [[T.hov5c, 1], [T.fold5, 0], [T.hov9[4], 1], [T.hov9[5], 0]]]].forEach(([id, mind, key, keys], k) => {
          const u = unfoldAt(t, keys);
          const st = T.stream5[key];
          const body = A.B.you(TXT.roomQ) + (t < st[0] ? A.B.think("", t) : A.B.msg(streamed(TXT.room[key], st[0], st[1], t)));
          thread({ id, block: "p2", a: i, kind: "ask", c: colourOf(mind, t), side: sideOf("right"), sew: prog(t, T.sew5[0] + k * 0.06, T.sew5[1] + k * 0.06), needle: k === 0 && t < T.sew5[1],
            ropeOp: t < T.carry5 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.26, ropeTrack: t < T.carry5 + 1.4 ? { t0: T.carry5 + k * 0.08, carry: 0.5 } : null,
            tag: t >= T.carry5 + 0.45 + k * 0.08, wash: u > 0.4, tagState: { unfold: u, mind, kindLabel: "you asked", body, z: u > 0.01 ? 10 + k : null } });
        });
      }
    }
    // t7: the Skeptic, called by name (left margin)
    if (t >= T.sew6[0]) {
      const b = block("p5");
      if (b) {
        b.ranges.push({ a: 0, b: P5.length, id: "t7", cls: "anc" });
        const u = unfoldAt(t, [[T.carry6 + 0.6, 1], [T.move7[0], 0]]);
        thread({ id: "t7", block: "p5", a: 0, kind: "answer", c: colourOf("Skeptic", t), side: sideOf("left"), sew: prog(t, ...T.sew6), needle: t < T.sew6[1],
          ropeOp: t < T.carry6 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.26, ropeTrack: t < T.carry6 + 1.4 ? { t0: T.carry6, carry: 0.5 } : null,
          tag: t >= T.carry6 + 0.45, wash: u > 0.4,
          tagState: { unfold: u, mind: "Skeptic", kindLabel: "answer", body: t < T.stream6[0] ? A.B.think("", t) : A.B.msg(streamed(TXT.call, ...T.stream6, t)) } });
      }
    }
    // k1: Sketch it, on the path from the API call to a shell
    if (t >= T.sew7[0]) {
      const b = block("p1"), i = p1.indexOf(TXT.ideaQuote);
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.ideaQuote.length, id: "k1", cls: "anc" });
        const u = unfoldAt(t, [[T.carry7 + 0.35, 1], [T.fold7, 0], [T.jump9[0] + 0.1, 0]]);
        let body = "";
        if (t < T.idea7) body = `<div class="think"><span>Stitching a sketch…</span><span class="dot" style="width:5px;height:5px;opacity:${fx(0.3 + 0.6 * (0.5 + 0.5 * Math.sin(t / 1.1 * Math.PI * 2)))}"></span></div>`;
        else {
          body = A.B.msg(TXT.idea);
          if (t >= T.steps7[0]) {
            const sk = sketchSVG(t, T.steps7, 0.62);
            body += `<div class="sk">${sk.svg}</div><div class="capt"><span class="gr">${esc(sk.caption)}</span><span class="ctl">${A.icon("left", 11, "var(--faint)")}${sk.step}/${sk.of}${A.icon("right", 11, "var(--faint)")}${A.icon("replay", 10, "var(--faint)")}</span></div><div class="larger" style="${t >= T.larger7 - 0.3 && t < T.larger7 + 0.1 ? "color:var(--ink)" : ""}">See it larger</div>`;
          }
        }
        thread({ id: "k1", block: "p1", a: i, kind: "question", c: colourOf("Inkling", t), side: sideOf("right"), sew: prog(t, ...T.sew7), needle: t < T.sew7[1],
          ropeOp: t < T.carry7 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.26, ropeTrack: t < T.carry7 + 1.4 ? { t0: T.carry7, carry: 0.45 } : null, tag: t >= T.carry7 + 0.4, wash: u > 0.4,
          tagState: { unfold: u, mind: "Inkling", kindLabel: "sketch", body, z: u > 0.01 ? 14 : null } });
      }
    }
    // t8: the writer's own mind, the note's mind now, reading the next paragraph
    if (t >= T.sew16[0]) {
      const b = block("p6");
      if (b) {
        const i = Math.max(0, P6.indexOf(TXT.mindQuote));
        b.ranges.push({ a: i, b: i + TXT.mindQuote.length, id: "t8", cls: "anc" });
        const tied = t >= ED.knot;
        const u = unfoldAt(t, [[T.open16, 1], [c6 + 0.6, 0], [ED.act, 1], [ED.knot + 1.0, 0]]);
        let body = A.B.msg(TXT.mind);
        if (within(t, ED.act, ED.knot + 1.0)) body += A.B.reply("", false) + A.B.acts(tied, within(t, ED.hov, ED.tie + 0.2) ? "resolve" : null);
        thread({ id: "t8", block: "p6", a: i, kind: "question", c: colourOf(MIND.name, t), side: sideOf("right"), sew: prog(t, ...T.sew16), needle: t < T.sew16[1], tied, stitchOp: tied ? 0.42 : 1,
          ropeOp: t < T.carry16 ? 0 : tied ? 0.85 * (1 - prog(t, ED.knot + 0.15, ED.knot + 0.5)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry16 + 1.4 ? { t0: T.carry16, carry: 0.5 } : t >= ED.tie ? { t0: ED.tie, tieAt: ED.tie, speed: 0.5 } : null, knotAt: within(t, ED.knot, ED.lift[0]) ? ED.knot : null,
          tag: t >= T.carry16 + 0.45, tagOp: tied ? 1 - prog(t, ED.knot + 1.0, ED.knot + 1.4) : 1, wash: u > 0.4 && !tied,
          tagState: { unfold: u, mind: MIND.name, kindLabel: tied ? "tied off" : "question", body, nolift: true } });
      }
    }
    // open questions are numbered in reading order, and knotted in the gutter
    const order = ["p1", "p2", "p3", "p4", "p5", "p6"];
    const open = threads.filter((th) => !th.tied && th.tag !== false && !(th.id === "t3" && within(t, T.snip10, T.undo10)));
    open.sort((a, b) => order.indexOf(a.block) - order.indexOf(b.block) || (a.a || 0) - (b.a || 0) || a.id.localeCompare(b.id));
    open.forEach((th, i) => (th.num = i + 1));
    for (const id of order) {
      const first = open.find((th) => th.block === id);
      const bl = block(id);
      if (first && bl && !(bl.gutter && bl.gutter.type === "needle")) bl.gutter = { type: "knot", n: first.num, c: first.c, k: knotScale(t - (first.arrived || 0) + 1) };
    }
    // marks only: no tags, a small mark after the passage
    const marksK = placement === "marks" ? prog(t, T.marks13, T.marks13 + 0.4) : 0;
    // the border: one motif per thread tied off in this note
    const motifs = [];
    if (t >= T.fly2[1]) motifs.push({ c: colourOf("Inkling", t), k: prog(t, T.fly2[1], T.fly2[1] + 0.4) });
    if (t >= T.fly9[1]) motifs.push({ c: colourOf("Inkling", t), k: prog(t, T.fly9[1], T.fly9[1] + 0.4) });
    const loose = open.length;
    // header and chrome fade while you type (app.rs chrome_layer: out slowly, back quickly)
    const calm = calmAt(t);
    const header = { op: 1 - calm, mind: mindOf(t), mindC: `var(--voice)`, mode: within(t, M6.mode + 0.35, M6.pace) ? "Waits to be asked" : "Reads along", loose,
      hov: within(t, T.tell8 - 0.3, T.tell8 + 0.15) ? "tell" : within(t, T.menu16 - 0.25, T.pick16) ? "mind" : within(t, T.hide12 - 0.3, T.hide12 + 0.1) || within(t, T.show12 - 0.3, T.show12 + 0.1) ? "hide" : null };
    const page = { scroll: scrollAt(t), header, blocks, threads, hidden, hideK, marksOnly: placement === "marks" && marksK > 0.5,
      border: { motifs }, title: { text: TITLE.slice(0, count(S.title, t)), caret: caretIn === "title" ? count(S.title, t) : null },
      chatOpen: within(t, T.chatIn8[0] + 0.1, T.chatOut9[1]), caretOn: caretOn(t) };
    if (t >= T.save8) page.brief = TXT.brief;
    if (within(t, T.tell8 + 0.1, T.save8)) page.briefIn = { text: TXT.brief.slice(0, count(S.brief, t)), caret: caretOn(t) };
    // menus anchored to the words
    page.menus = [];
    const bar = (block, anchor, t0, t1, hov, extra) => { if (within(t, t0, t1)) page.menus.push({ type: "toolbar", block, anchor, op: spring(t - t0, 400, 32), hov, ...(extra || {}) }); };
    bar("p3", "t2", T.bar3, T.sew3[0] - 0.05, t >= T.find3 - 0.25 ? "find" : null);
    bar("p1", "t3", T.bar4, T.askClick4, t >= T.askClick4 - 0.35 ? "ask" : null);
    bar("p2", "t4", T.bar5, T.askClick5, t >= T.askClick5 - 0.35 ? "ask" : null);
    bar("p1", "k1", T.bar7, T.sketch7 + 0.1, t >= T.sketch7 - 0.3 ? "sketch" : null);
    if (within(t, T.askClick4, T.send4)) page.menus.push({ type: "ask", block: "p1", anchor: "t3", op: spring(t - T.askClick4, 400, 32), text: TXT.askQ.slice(0, count(S.q4, t)), caret: caretOn(t), chips: chipsAt(t, []) });
    if (within(t, T.askClick5, T.send5)) page.menus.push({ type: "ask", block: "p2", anchor: "t4", op: spring(t - T.askClick5, 400, 32), text: TXT.roomQ.slice(0, count(S.q5, t)), caret: caretOn(t), chips: chipsAt(t, [t >= T.chip5a ? "Staff engineer" : null, t >= T.chip5b ? "Socratic tutor" : null]) });
    if (within(t, T.menu16, T.pick16 + 0.15)) page.menus.push({ type: "mind", op: spring(t - T.menu16, 400, 32), hov: t >= T.menu16 + 0.5 ? 1 : null, minds: [{ name: "Inkling", d: "Thinks alongside you, and speaks up when something is worth a second look.", on: true }, { name: MIND.name, d: MIND.description }] });
    if (within(t, M6.mode, M6.mode + 0.95)) page.menus.push({ type: "mode", op: spring(t - M6.mode, 400, 32), cur: t >= M6.mode + 0.35 ? "ask" : "read", hov: t >= M6.mode + 0.2 ? "ask" : null });
    // the block-kind beat in ch7: the dictated question turns into each kind in turn
    if (within(t, M7.blocks, M7.code)) {
      const kinds = ["p", "h1", "h2", "h3", "q", "li", "ol", "p"];
      const k = clamp(Math.floor((t - M7.blocks - 0.3) / (BEAT / 2)), 0, 7);
      const b = block("p7");
      if (b) { b.kind = t < M7.blocks + 0.3 ? "p" : kinds[k]; b.sel = [0, P7.length]; page.menus.push({ type: "toolbar", block: "p7", anchor: "blk", op: 1, kind: [0, 1, 2, 3, 4, 5, 6, 0][k] }); b.ranges.push({ a: 0, b: P7.length, id: "blk", cls: "" }); }
    }
    // ⌘E in ch7: the flag's name becomes code
    if (within(t, M7.code + 0.35, M7.code + 1.0)) {
      const b = block("p4");
      if (b) { b.ranges.push({ a: fi, b: fi + flag.length, id: "ice", cls: "" }); page.menus.push({ type: "toolbar", block: "p4", anchor: "ice", op: spring(t - M7.code - 0.35, 400, 32), mark: t >= M7.code + 0.75 ? 4 : null }); }
    }
    return { page, L };
  }
  function chipsAt(t, extra) {
    const on = new Set(["Inkling", ...extra.filter(Boolean)]);
    return [["Inkling", "madder"], ["Staff engineer", "plum"], ["Editor", "sage"], ["Skeptic", "indigo"], ["Socratic tutor", "ochre"], ["Investor", "walnut"]].map(([name, c]) => ({ name, c, on: on.has(name) }));
  }
  const caretOn = (t) => typingAt(t) || Math.floor(t / 0.53) % 2 === 0;
  /** The chrome steps back while you type and comes back when the pointer moves. */
  function calmAt(t) {
    // typing windows in which the pointer does not move
    const spans = [[T.title2[0], T.p3[1] + 0.2], [T.p5[0], T.p5[1] + 0.2], [T.p6[0], T.p6[1] + 0.2], [T.type1[0], T.type1[1] + 0.2]];
    let v = 0;
    for (const [a, b] of spans) {
      if (t < a) continue;
      const k = 1 - Math.exp(-5 * Math.max(0, Math.min(t, b) - a));
      v = t <= b ? k : k * Math.exp(-14 * (t - b));
      if (t <= b + 0.5) break;
    }
    return clamp(v);
  }

  // ======================================================================= the Start here guide
  function startPage(t) {
    const blocks = START.blocks.slice(1).map((b, i) => ({ id: "s" + (i + 1), kind: { paragraph: "p", h2: "h2", bullet: "li", quote: "q" }[b.kind] || b.kind, text: b.text, ranges: [] }));
    const last = blocks[blocks.length - 1];
    const n = count(S.first, t);
    last.text = FIRST.slice(0, n);
    last.ph = "One sentence about something you’re working on, or something you believe.";
    last.caret = t >= T.glide1 - 0.4 ? n : null;
    blocks.forEach((b, i) => { if (i && blocks[i - 1].kind === "li" && b.kind === "li") b.gap = 4; });
    const threads = [];
    const side = { right: "right", left: "left" };
    START.threads.forEach((th, k) => {
      const bi = blocks.findIndex((b) => b.text.includes(th.quote));
      if (bi < 0) return;
      const b = blocks[bi], i = b.text.indexOf(th.quote), id = "g" + k;
      b.ranges.push({ a: i, b: i + th.quote.length, id, cls: "anc" });
      const mind = th.mind_name || "Inkling";
      const body = th.messages.map((mm) => (mm.role === "you" ? A.B.you(mm.text) : A.B.msg(mm.text))).join("");
      let unfold = 0, extra = "";
      if (th.drawing && MEAS.sketchAt) {
        unfold = unfoldAt(t, [[MEAS.sketchAt - 0.5, 1], [MEAS.sketchAt + 1.6, 0]]);
        extra = guideSketch(t, MEAS.sketchAt - 0.3);
      }
      threads.push({ id, block: b.id, a: i, kind: th.kind === "ask" ? "ask" : th.kind, c: A.mindThread(mind, "Inkling", "madder"), side: side[th.side] || "right", ropeOp: unfold > 0.05 ? lerp(0.26, 1, unfold) : 0.26,
        tagState: { unfold, mind, kindLabel: th.kind === "ask" ? "you asked" : th.kind, body: body + extra, nolift: true } });
    });
    // your first sentence: read with a question (first_run), pulled to the margin
    if (t >= T.sew1a) {
      last.ranges.push({ a: 0, b: FIRST.length, id: "f6", cls: "anc" });
      const u = unfoldAt(t, [[T.hover1, 1]]);
      threads.push({ id: "f6", block: last.id, kind: "question", c: "madder", side: "right", sew: prog(t, T.sew1a, T.sew1b), needle: t < T.sew1b,
        ropeOp: t < T.carry1 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.45, ropeTrack: t < T.carry1 + 1.6 ? { t0: T.carry1, carry: 0.55 } : null,
        tag: t >= T.carry1 + 0.5, wash: u > 0.4, tagState: { unfold: u, mind: "Inkling", kindLabel: "question", body: A.B.msg(TXT.first), nolift: true } });
    }
    const order = blocks.map((b) => b.id);
    const open = threads.slice().sort((a, b) => order.indexOf(a.block) - order.indexOf(b.block) || (a.a || 0) - (b.a || 0) || a.id.localeCompare(b.id));
    open.forEach((th, i) => (th.num = i + 1));
    for (const b of blocks) { const f = open.find((th) => th.block === b.id); if (f) b.gutter = { type: "knot", n: f.num, c: f.c }; }
    if (within(t, ...T.read1)) last.gutter = { type: "needle" };
    blocks[0].first = true;
    const calm = within(t, T.type1[0], T.read1[1]) ? clamp((t - T.type1[0]) * 2) * (1 - clamp((t - T.read1[1]) * 6)) : 0;
    return { blocks, threads, header: { op: 1 - calm, loose: open.length }, title: { text: START.title }, border: {}, scroll: startScroll(t), caretOn: caretOn(t) };
  }
  /** The guide's own sketch (welcome.rs BOOT_SKETCH), drawn on as its tag unfolds. */
  let GUIDE_SK = null;
  function guideSketch(t, t0) {
    if (!GUIDE_SK) { const keep = SK; buildSketch(START.threads.find((x) => x.drawing).drawing); GUIDE_SK = SK; SK = keep; }
    const keep = SK; SK = GUIDE_SK;
    const sk = sketchSVG(t, [t0, t0 + 0.35, t0 + 0.7], 0.62);
    SK = keep;
    return `<div class="sk">${sk.svg}</div><div class="capt"><span class="gr">${esc(sk.caption)}</span><span class="ctl">${sk.step}/${sk.of}</span></div>`;
  }
  function startScroll(t) {
    const end = MEAS.startEnd || 1400;
    return end * E.io(prog(t, T.glide0, T.glide1));
  }

  // ======================================================================= Sketch it
  let SK = null;
  function buildSketch(svgText) {
    const doc = new DOMParser().parseFromString(svgText || TXT.svg, "image/svg+xml");
    const root = doc.documentElement;
    const vb = (root.getAttribute("viewBox") || "0 0 320 300").split(/[\s,]+/).map(Number);
    const meas = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    meas.setAttribute("style", "position:absolute;left:-9999px;top:0;width:10px;height:10px");
    document.body.appendChild(meas);
    const steps = [];
    root.querySelectorAll(":scope > g[data-step]").forEach((g) => {
      const items = [];
      g.querySelectorAll("path,line,polyline,polygon,rect,circle,ellipse,text").forEach((el) => {
        const tag = el.tagName.toLowerCase(), at = (k, d) => el.getAttribute(k) ?? g.getAttribute(k) ?? d;
        if (tag === "text") { items.push({ kind: "text", x: +at("x", 0), y: +at("y", 0), text: el.textContent.trim(), size: +at("font-size", 12), cls: at("class", "sans"), anchor: at("text-anchor", "start"), fill: at("fill", "ink") }); return; }
        const clone = document.createElementNS("http://www.w3.org/2000/svg", tag);
        for (const a of el.attributes) clone.setAttribute(a.name, a.value);
        meas.appendChild(clone);
        const L = clone.getTotalLength ? clone.getTotalLength() : 0;
        items.push({ kind: "shape", tag, attrs: Array.from(el.attributes).map((a) => [a.name, a.value]), L, fill: at("fill", "none"), stroke: at("stroke", "none"), sw: +at("stroke-width", 1) });
      });
      steps.push({ caption: g.getAttribute("data-caption") || "", items });
    });
    meas.remove();
    SK = { vb, steps };
  }
  const TONE = { ink: "var(--voice)", graphite: "var(--graphite)", faint: "var(--faint)", rule: "var(--rule)", wash: "color-mix(in srgb,var(--voice) 9%,transparent)", paper: "var(--paper)", none: "none" };
  /** The drawing as it stands: each step drawn on over STEP_DRAW, one every STEP_EVERY (drawing.rs). */
  function sketchSVG(t, starts, scale) {
    let s = "", cur = 0;
    SK.steps.forEach((st, i) => {
      const a = starts[i];
      if (a == null || t < a) return;
      cur = i;
      const k = prog(t, a, a + 0.75);
      const n = st.items.length;
      st.items.forEach((it, j) => {
        const kk = clamp((k * 1.25 - (j / n) * 0.25) / 1);
        if (kk <= 0) return;
        if (it.kind === "text") {
          const col = it.cls === "voice" ? "var(--voice)" : TONE[it.fill] || "var(--graphite)";
          s += `<text x="${it.x}" y="${it.y}" text-anchor="${it.anchor}" style="font-family:Figtree;font-size:${it.size}px;font-weight:${it.cls === "voice" ? 500 : 400};fill:${col}" opacity="${fx(clamp(kk * 2))}">${esc(it.text)}</text>`;
          return;
        }
        const attrs = it.attrs.filter(([nm]) => !["stroke", "fill", "stroke-width", "class"].includes(nm)).map(([nm, v]) => `${nm}="${esc(v)}"`).join(" ");
        const stroke = TONE[it.stroke] || "none", fill = TONE[it.fill] || "none";
        // the mind's own lines are stitched; thin guides stay drawn (drawing.rs)
        const stitched = it.stroke === "ink" && it.sw >= 1.2;
        const L = Math.max(1, it.L);
        const dash = stitched ? `stroke-dasharray:${fx(it.sw * 3.4)} ${fx(it.sw * 2.1)};` : "";
        const reveal = stitched ? `mask="url(#rv${cur}_${j})"` : "";
        if (fill !== "none") s += `<${it.tag} ${attrs} style="fill:${fill};stroke:none" opacity="${fx(kk)}"/>`;
        if (stroke !== "none") {
          if (stitched) s += `<mask id="rv${cur}_${j}"><${it.tag} ${attrs} style="fill:none;stroke:#fff;stroke-width:${it.sw + 3};stroke-dasharray:${fx(L)} ${fx(L)};stroke-dashoffset:${fx(L * (1 - kk))}"/></mask>`;
          s += `<${it.tag} ${attrs} ${reveal} style="fill:none;stroke:${stroke};stroke-width:${it.sw * (scale || 1)};stroke-linecap:round;${stitched ? dash : `stroke-dasharray:${fx(L)} ${fx(L)};stroke-dashoffset:${fx(L * (1 - kk))};`}"/>`;
        }
      });
    });
    return { svg: `<svg viewBox="${SK.vb.join(" ")}" width="${fx(SK.vb[2] * (scale || 0.68))}" height="${fx(SK.vb[3] * (scale || 0.68))}" style="overflow:visible">${s}</svg>`, step: cur + 1, of: SK.steps.length, caption: SK.steps[cur] ? SK.steps[cur].caption : "" };
  }

  // ======================================================================= the camera
  // (x, y): the world point at the centre of the frame; z: screen px per point. Moves ease
  // like cloth: a little overshoot, then they settle (a spring at damping 0.72).
  const CLOTH = (() => { const z = 0.72, w = 9, wd = w * Math.sqrt(1 - z * z); const f = (k) => 1 - Math.exp(-z * w * k) * (Math.cos(wd * k) + ((z * w) / wd) * Math.sin(wd * k)); const f1 = f(1); return (k) => (k <= 0 ? 0 : k >= 1 ? 1 : f(k) / f1); })();
  const GLIDE = E.io;
  let CAM = [];
  function camAt(t) {
    let c = { x: 660, y: -330, z: 1.5 };
    for (const mv of CAM) {
      if (t <= mv.t[0]) break;
      const k = (mv.e || CLOTH)(prog(t, mv.t[0], mv.t[1]));
      const to = typeof mv.to === "function" ? mv.to() : mv.to;
      c = { x: lerp(c.x, to.x ?? c.x, k), y: lerp(c.y, to.y ?? c.y, k), z: lerp(c.z, to.z ?? c.z, k) };
    }
    // a breath of drift, so no shot is ever quite still
    const d = 1 + 0.004 * Math.sin(t * 0.41);
    return { x: c.x + 3.2 * Math.sin(t * 0.31 + 1.3), y: c.y + 2.4 * Math.sin(t * 0.27), z: c.z * d };
  }
  const toScreen = (cam, x, y) => [960 + (x - cam.x) * cam.z, 540 + (y - cam.y) * cam.z];
  const SET = { x: 1460, y: 40 }; // where the Settings window sits on the desk
  const SCROLL = {}; // per beat, how far the Settings sheet is scrolled
  /** The camera on a part of a Settings pane (measured at load), scrolled by `sc`. */
  function setFocus(pane, key, sc, z, dy) {
    const mm = (MEAS.set && MEAS.set[pane] && MEAS.set[pane][key]) || { x: 300, y: 300, w: 400, h: 100 };
    return { x: SET.x + mm.x + mm.w / 2, y: SET.y + mm.y - (sc || 0) + mm.h / 2 + (dy || 0), z };
  }

  function buildCamera() {
    const M = MEAS.main, an = (id) => M.anchors[id] || [{ x: 351, y: 300, r: 900, b: 328 }], blk = (id) => M.blocks[id] || { y: 300, h: 28, x: 351, w: 614 };
    const brief = 28;
    const a1 = an("t1")[0], a2 = an("t2")[0], a3 = an("t3")[0], a4 = an("t4")[0], k1 = an("k1")[0], b5 = blk("p5"), b6 = blk("p6"), b4 = blk("p4"), b7 = blk("p7");
    const st = MEAS.start || {};
    const sc = MEAS.sewnScroll || 0;
    CAM = [
      // cold open: in close on the sheet's border, then back to the whole window
      { t: [0, T.pull0], to: { x: 660, y: -318, z: 1.56 }, e: GLIDE },
      { t: [T.pull0, T.pull1], to: { x: 660, y: 410, z: 1.2 }, e: CLOTH },
      // ch1: glide down the guide (the page scrolls), then in on your sentence
      { t: [T.glide0, T.glide1], to: { x: 650, y: 430, z: 1.32 }, e: GLIDE },
      { t: [T.type1[0] - 0.2, T.type1[1]], to: { x: 610, y: (st.lastY || 760) + 6, z: 1.75 }, e: GLIDE },
      { t: [T.read1[0], T.sew1b], to: { x: 560, y: (st.lastY || 760) + 8, z: 2.35 }, e: CLOTH },
      { t: [T.carry1 - 0.1, T.hover1 + 0.2], to: { x: 900, y: (st.lastY || 760) + 40, z: 2.05 }, e: CLOTH },
      { t: [T.hover1 + 0.4, T.wipe1[0]], to: { x: 940, y: (st.lastY || 760) + 60, z: 2.25 }, e: GLIDE },
      // ch2: a new note, written
      { t: [c2 - 0.05, c2], to: { x: 650, y: 300, z: 1.36 }, e: (k) => k },
      { t: [c2 + 0.15, T.p1[0]], to: { x: 610, y: 250, z: 1.62 }, e: GLIDE },
      { t: [T.p1[0], T.p2[1]], to: { x: 640, y: 290, z: 1.55 }, e: GLIDE },
      { t: [T.pause2 - 0.2, T.pause2 + 1.1], to: { x: (a1.x + a1.r) / 2 + 60, y: a1.y + 30, z: 2.35 }, e: CLOTH },
      { t: [T.pause2 + 1.4, T.t1hover + 0.4], to: { x: 1000, y: a1.y + 100, z: 1.95 }, e: CLOTH },
      { t: [T.t1fold - 0.2, T.e1sel + 0.1], to: { x: 760, y: a1.y - 6, z: 2.15 }, e: CLOTH },
      { t: [T.reread[0], T.settleOpen + 0.3], to: { x: 930, y: a1.y + 130, z: 1.85 }, e: CLOTH },
      { t: [T.breath2, T.tie2 + 0.1], to: { x: 972, y: a1.y + 50, z: 4.0 }, e: GLIDE },
      { t: [T.fly2[0] - 0.1, T.fly2[1] + 0.3], to: { x: 800, y: 210, z: 1.35 }, e: CLOTH },
      { t: [T.fly2[1] + 0.5, T.p4[1] + 0.6], to: { x: 650, y: 330, z: 1.4 }, e: CLOTH },
      // ch3: asking
      { t: [c3 + 0.3, T.sel3[1]], to: { x: (a2.x + a2.r) / 2 + 230, y: a2.y + 40, z: 1.8 }, e: CLOTH },
      { t: [T.sew3[0], T.sew3[1]], to: { x: (a2.x + a2.r) / 2 + 60, y: a2.y + 18, z: 2.3 }, e: GLIDE },
      { t: [T.carry3 - 0.1, T.open3 + 0.3], to: { x: 1040, y: a2.y + 90, z: 1.85 }, e: CLOTH },
      { t: [T.fold3, T.sel4[1]], to: { x: 780, y: a3.y + 60, z: 1.62 }, e: CLOTH },
      { t: [T.send4, T.carry4 + 0.6], to: { x: 330, y: a3.y + 190, z: 1.55 }, e: CLOTH },
      { t: [T.fold4 - 0.2, T.sel5[1]], to: { x: (a4.x + a4.r) / 2 + 160, y: a4.y + 40, z: 1.7 }, e: CLOTH },
      { t: [T.send5, T.carry5 + 0.3], to: { x: 780, y: a4.y + 50, z: 1.6 }, e: CLOTH },
      { t: [T.open5 - 0.2, T.hov5b], to: { x: 1040, y: a4.y + 150, z: 1.75 }, e: CLOTH },
      { t: [T.hov5b - 0.1, T.hov5c], to: { x: 1040, y: a4.y + 210, z: 1.7 }, e: CLOTH },
      { t: [T.hov5c - 0.1, T.fold5], to: { x: 1040, y: a4.y + 260, z: 1.7 }, e: CLOTH },
      { t: [T.fold5, T.p5[1]], to: { x: 640, y: b5.y + 10, z: 1.75 }, e: CLOTH },
      { t: [T.carry6 - 0.2, T.carry6 + 0.6], to: { x: 360, y: b5.y + 60, z: 1.6 }, e: CLOTH },
      { t: [T.move7[0], T.move7[1]], to: { x: (k1.x + k1.r) / 2 + 60, y: k1.y + 20, z: 1.85 }, e: CLOTH },
      { t: [T.sew7[0], T.carry7 + 0.4], to: { x: 900, y: k1.y + 40, z: 1.7 }, e: CLOTH },
      { t: [T.idea7, T.steps7[0]], to: { x: 1060, y: k1.y + 170, z: 1.42 }, e: CLOTH },
      { t: [T.steps7[0], T.larger7], to: { x: 1070, y: k1.y + 230, z: 1.34 }, e: GLIDE },
      { t: [T.big7[0], T.big7[0] + 0.6], to: { x: 660, y: 430, z: 1.22 }, e: CLOTH },
      { t: [T.fold7, T.tell8], to: { x: 640, y: 200, z: 1.6 }, e: CLOTH },
      { t: [T.save8, T.chatIn8[1]], to: { x: 860, y: 330, z: 1.35 }, e: CLOTH },
      { t: [T.chatType8[0], T.chatStream8[0]], to: { x: 1110, y: 300, z: 1.75 }, e: CLOTH },
      { t: [T.chatStream8[0], T.chatStream8[1]], to: { x: 1130, y: 360, z: 1.75 }, e: GLIDE },
      // ch4: threads
      { t: [c4, c4 + 1.0], to: { x: 660, y: 300 + brief, z: 1.3 }, e: CLOTH },
      { t: [c4 + 1.2, c4 + 3.0], to: { x: 520, y: 330 + brief, z: 1.75 }, e: GLIDE },
      { t: [T.hov9[0] - 0.3, T.hov9[5]], to: { x: 1000, y: a4.y + 90 + brief, z: 1.75 }, e: CLOTH },
      { t: [T.jump9[0], T.jump9[0] + 0.45], to: { x: 300, y: a3.y + 80 + brief, z: 1.85 }, e: CLOTH },
      { t: [T.jump9[1], T.jump9[1] + 0.45], to: { x: 1040, y: a4.y + 80 + brief, z: 1.85 }, e: CLOTH },
      { t: [T.drag9[0] - 0.6, T.drag9[1]], to: { x: 1000, y: a2.y + 140 + brief, z: 1.55 }, e: CLOTH },
      { t: [T.replyFocus9, T.answer9[1]], to: { x: 1010, y: a2.y + 320 + brief, z: 1.55 }, e: GLIDE },
      { t: [T.breath9, T.tie9 + 0.1], to: { x: 1040, y: a2.y + 28 + 140, z: 2.7 }, e: GLIDE },
      { t: [T.fly9[0] - 0.1, T.fly9[1] + 0.3], to: { x: 820, y: 240, z: 1.3 }, e: CLOTH },
      { t: [T.open10 - 0.6, T.open10], to: { x: 330, y: a3.y + 130 + brief, z: 1.75 }, e: CLOTH },
      { t: [T.snip10 - 0.001, T.snip10], to: { x: 560, y: 560, z: 1.22 }, e: (k) => k },
      { t: [T.undo10 + 0.6, T.reopen11], to: { x: 900, y: a2.y + 170 + brief, z: 1.6 }, e: CLOTH },
      { t: [T.hide12 - 0.4, T.hide12 + 0.2], to: { x: 660, y: 300 + brief, z: 1.32 }, e: CLOTH },
      { t: [T.right13 - 0.2, T.both13 + 0.4], to: { x: 700, y: 330 + brief, z: 1.28 }, e: GLIDE },
      // ch5: Settings, then back to the note
      { t: [c5 - 0.01, c5], to: setFocus("NewMind", "chips", 0, 1.75, 30), e: (k) => k },
      { t: [T.blank15, T.desc15[1]], to: setFocus("NewMind", "mind", 0, 1.6, 0), e: CLOTH },
      { t: [T.how15[0], T.voice15], to: setFocus("NewMind", "speak", 300, 1.6, -40), e: GLIDE },
      { t: [T.pan16[0], T.pan16[1]], to: { x: 640, y: 260 + brief, z: 1.6 }, e: CLOTH },
      { t: [T.p6[0], T.p6[1]], to: { x: 640, y: b6.y + brief, z: 1.75 }, e: GLIDE },
      { t: [T.carry16 - 0.2, T.open16 + 0.4], to: { x: 960, y: b6.y + 110 + brief, z: 1.75 }, e: CLOTH },
      { t: [T.open16 + 1.6, T.wipe17[0]], to: { x: 800, y: 350 + brief, z: 1.35 }, e: GLIDE },
    ];
    // ch6 and ch7: the montages cut on the beat; each beat sets its own framing (cuts, not moves)
    const cut = (t0, to, drift) => { CAM.push({ t: [t0 - 0.001, t0], to, e: (k) => k }); if (drift) CAM.push({ t: [t0 + 0.02, t0 + drift.d], to: drift.to, e: GLIDE }); };
    const F = (pane, key, scr, z, dz, dy) => { const a = setFocus(pane, key, scr, z, dy); return [a, { ...a, z: a.z * (1 + (dz || 0.05)) }]; };
    let f;
    f = F("Appearance", "look", 0, 1.75, 0.05, 40); cut(c6, f[0], { d: 2.5, to: f[1] });
    f = F("Appearance", "dots", SCROLL.ink, 2.3, 0.06); cut(M6.ink, f[0], { d: 2.5, to: f[1] });
    f = F("Appearance", "letter", SCROLL.letter, 2.0); cut(M6.letter, f[0], { d: 1.2, to: f[1] });
    f = F("Appearance", "size", SCROLL.size, 2.0); cut(M6.size, f[0], { d: 1.2, to: f[1] });
    cut(M6.mode, { x: 560, y: 150, z: 2.1 }, { d: 1.2, to: { x: 560, y: 160, z: 2.2 } });
    f = F("Reading", "pace", 0, 2.1); cut(M6.pace, f[0], { d: 1.2, to: f[1] });
    f = F("Appearance", "inkdrop", SCROLL.drop, 2.3); cut(M6.drop, f[0], null);
    cut(M6.drop + 0.6, { x: 900, y: 470, z: 2.0 }, { d: 1.9, to: { x: 905, y: 475, z: 2.1 } });
    cut(M6.speak, { x: 980, y: 600 + brief, z: 1.6 }, { d: 2.4, to: { x: 1000, y: 610 + brief, z: 1.72 } });
    f = F("About you", "learn", 0, 1.75, 0.05, 90); cut(M6.learn, f[0], { d: 1.8, to: f[1] });
    f = F("Shortcuts", "hide", SCROLL.keys, 2.1, 0.05, -40); cut(M6.keys, f[0], { d: 1.8, to: f[1] });
    cut(M6.ring, { x: SET.x + 150, y: SET.y + 150, z: 2.1 }, { d: 1.8, to: { x: SET.x + 150, y: SET.y + 160, z: 2.2 } });
    cut(c7, { x: 380, y: 330 + brief, z: 1.45 }, null);
    cut(M7.side, { x: 380, y: 330, z: 1.45 }, { d: 2.4, to: { x: 400, y: 330, z: 1.55 } });
    cut(M7.find, { x: 660, y: 330, z: 1.5 }, { d: 1.8, to: { x: 660, y: 330, z: 1.58 } });
    cut(M7.copy, { x: 660, y: 700, z: 1.4 }, { d: 1.2, to: { x: 660, y: 710, z: 1.48 } });
    cut(M7.exp, { x: 660, y: 430, z: 1.75 }, { d: 2.4, to: { x: 660, y: 430, z: 1.86 } });
    cut(M7.obs, { x: 390, y: 200, z: 1.55 }, { d: 2.4, to: { x: 400, y: 215, z: 1.62 } });
    cut(M7.help, { x: 470, y: 130, z: 1.55 }, { d: 1.2, to: { x: 520, y: 200, z: 1.4 } });
    f = F("Model", "models", SCROLL.model, 1.65, 0.05, 40); cut(M7.model, f[0], { d: 2.4, to: f[1] });
    f = F("Model", "claude", 0, 2.0); cut(M7.claude, f[0], { d: 1.2, to: f[1] });
    cut(M7.blocks, { x: 760, y: b7.y + brief + 40, z: 1.75 }, { d: 1.6, to: { x: 760, y: b7.y + brief + 40, z: 1.85 } });
    const fl = an("ice")[0] || { x: 500, r: 640, y: b4.y };
    cut(M7.code, { x: (fl.x + fl.r) / 2 + 300, y: fl.y + brief + 60, z: 1.9 }, { d: 1.6, to: { x: (fl.x + fl.r) / 2 + 300, y: fl.y + brief + 60, z: 1.98 } });
    const s1 = blk("s1"), s3 = blk("s3"), s4 = blk("s4"), s6 = blk("s6");
    cut(M7.check, { x: 560, y: (s1.y + s3.y + s3.h) / 2 - sc + brief, z: 2.05 }, { d: 2.3, to: { x: 570, y: (s1.y + s3.y + s3.h) / 2 - sc + brief, z: 2.15 } });
    cut(M7.sewn, { x: 660, y: (s4.y + s6.y + s6.h) / 2 - sc + brief, z: 1.62 }, { d: 2.3, to: { x: 660, y: (s4.y + s6.y + s6.h) / 2 - sc + brief, z: 1.7 } });
    // the end: to the writer's mind's tag, the tie, then back as its knot lifts off the page
    CAM.push({ t: [cEnd, cEnd + 0.7], to: { x: 990, y: b6.y + 150 + brief, z: 1.7 }, e: CLOTH });
    CAM.push({ t: [ED.breath, ED.tie + 0.1], to: { x: 974, y: b6.y + brief + 52, z: 3.6 }, e: GLIDE });
    CAM.push({ t: [ED.lift[0], ED.lift[1] + 0.4], to: { x: 900, y: b6.y + brief + 40, z: 2.6 }, e: GLIDE });
  }

  // ======================================================================= captions
  // Stitched in behind a needle, tied off when their beat ends. `kind`: title (Fraunces
  // Display) or label (Figtree). Each sits on a patch of the page, sewn on.
  const CAPS = [];
  const cap = (t0, t1, text, o = {}) => CAPS.push({ t0, t1, text, kind: o.kind || "title", keys: o.keys, at: o.at || "bl", size: o.size });
  cap(c1 + 0.6, c1 + 2.95, "A new notebook opens on a guide. Everything on it works.", { kind: "label" });
  cap(c1 + 3.05, c1 + 5.45, "Mostly it stays quiet. Sometimes it answers with a sketch.", { kind: "label" });
  cap(T.hover1 + 0.6, T.wipe1[0] - 0.1, "Your first sentence always gets a question.");
  cap(T.title2[0] + 0.3, T.p1[0] + 2.4, "No bar. The controls fade while you write.", { kind: "label" });
  cap(T.read2a[0] - 0.1, T.p2[1] - 0.3, "It reads each paragraph when you pause.");
  cap(T.t1arrive + 0.1, T.pause2 + 1.5, "A question that comes mid-sentence waits, folded.", { kind: "label" });
  cap(T.t1hover + 0.2, T.t1fold, "When a line doesn’t add up, it pulls a thread.");
  cap(T.e1sel, T.settle[0], "Fix the words. It reads them again.");
  cap(T.breath2, T.fade2[0] + 0.3, "The problem is gone, so the thread ties itself off.");
  cap(T.sel3[0] + 0.3, T.fold3, "Select words. ⌘↵ finds the loose thread in them.", { kind: "label", keys: { at: T.find3, k: ["⌘", "↵"] } });
  cap(T.bar4, T.fold4 - 0.1, "Ask your own question.");
  cap(T.bar5, T.send5 + 0.2, "Ask the room: up to three minds at once.");
  cap(T.open5 + 0.2, T.fold5, "Each answers on its own thread, in its own colour.");
  cap(T.p5[0] + 0.2, T.move7[0] - 0.1, "Or start a sentence with a mind’s name.");
  cap(T.sel7[0] + 0.2, T.larger7 - 0.2, "Sketch it: a drawing, stitched out step by step.");
  cap(T.larger7, T.fold7, "Click a sketch to see it larger.", { kind: "label" });
  cap(T.tell8, T.chatKey8 - 0.15, "Tell Inkling what the note is for.");
  cap(T.chatKey8, T.chatStream8[1] + 0.2, "A chat about the whole note. It has read every thread.", { kind: "label", keys: { at: T.chatKey8, k: ["⌘", "J"] } });
  cap(T.numbers9, T.hov9[0] - 0.2, "Open questions are numbered, with a knot in the gutter.", { kind: "label" });
  cap(T.hov9[0], T.jump9[0] - 0.1, "Tags fold away. Point at one to read it.");
  cap(T.jump9[0], T.drag9[0] - 0.2, "Jump to the next loose thread.", { kind: "label", keys: { at: T.jump9[0], k: ["⌥", "⌘", "↓"], again: T.jump9[1] } });
  cap(T.drag9[0], T.replyFocus9, "Drag a tag anywhere. It opens toward the room.", { kind: "label" });
  cap(T.replyFocus9 + 0.2, T.resolveHov9 - 0.1, "Write back. The mind answers in the same thread.");
  cap(T.resolveHov9, T.fade9[1], "Resolve ties it off. The knot joins the border.");
  cap(T.open10 + 0.1, T.undo10 + 1.5, "Dismiss snips it. ⌥⌘Z brings it back.", { kind: "label", keys: { at: T.dis10, k: ["⇧", "⌘", "⌫"], again: T.undo10, k2: ["⌥", "⌘", "Z"] } });
  cap(T.reopen11 + 0.1, T.hide12 - 0.3, "Reopen it, copy it, or make it a note of its own.", { kind: "label" });
  cap(T.hide12, T.show12 + 0.6, "Hide every thread. Just your words.", { kind: "label", keys: { at: T.hide12, k: ["⇧", "⌘", "."], again: T.show12 } });
  cap(T.right13, T.snip14 - 0.3, "Both margins, the right margin, or marks only.", { kind: "label" });
  cap(T.chips15 - 0.2, T.blank15 + 0.6, "Built in: Inkling, Staff engineer, Editor, Skeptic, Socratic tutor and Investor.", { kind: "label" });
  cap(T.blank15 + 0.8, T.pan16[0], "Make your own: what it is for, how it thinks, how it speaks.");
  cap(T.menu16, T.p6[0] + 0.3, "Pick the note’s mind, above the title.");
  cap(T.open16 + 0.2, T.wipe17[0], "Each mind keeps its own thread colour.");
  const lab = (t0, t1, text, keys) => cap(t0 + 0.06, t1 - 0.04, text, { kind: "label", keys });
  lab(M6.dark, M6.ink, "Linen, Lamplight, or follow the system.");
  lab(M6.ink, M6.letter, "Your thread colour.");
  lab(M6.letter, M6.size, "Inkling’s lettering: its own, or like your notes.");
  lab(M6.size, M6.mode, "Text size.");
  lab(M6.mode, M6.pace, "Reads along, or waits to be asked. Per note.");
  lab(M6.pace, M6.drop, "Reading pace: regular, or slow.");
  lab(M6.drop, M6.speak, "Ink from the pointer.");
  lab(M6.speak, M6.learn, "Hold ⌥ to speak. The words appear as you talk.", { at: M6.speak + 0.15, k: ["⌥"], hold: 1.8 });
  lab(M6.learn, M6.keys, "Inkling can learn about you from your replies.");
  lab(M6.keys, M6.ring, "Every shortcut can be changed.");
  lab(M6.ring, M6.end, "Tab reaches every control; a stitched ring shows where.", { at: M6.ring + 0.3, k: ["⇥"], again: M6.ring + 0.9 });
  lab(M7.side, M7.find, "Your notes and folders. A dot marks open threads.", { at: M7.side + 0.1, k: ["⌘", "\\"] });
  lab(M7.find, M7.copy, "Find a note, or start one.", { at: M7.find + 0.1, k: ["⌘", "K"] });
  lab(M7.copy, M7.exp, "Copy a note as Markdown.", { at: M7.copy + 0.1, k: ["⇧", "⌘", "C"] });
  lab(M7.exp, M7.obs, "Export to Markdown, Word or HTML, with the threads or without.", { at: M7.exp + 0.1, k: ["⇧", "⌘", "E"] });
  lab(M7.obs, M7.help, "File › Import from Obsidian: a vault comes in, folders and all.");
  lab(M7.help, M7.model, "Help › Welcome Guide.");
  lab(M7.model, M7.claude, "Pick the model, and how much effort it spends.");
  lab(M7.claude, M7.blocks, "It runs on the Claude login already on this Mac.");
  lab(M7.blocks, M7.code, "Title, three heading sizes, lists and quotes.");
  lab(M7.code, M7.check, "Code inside a sentence.", { at: M7.code + 0.75, k: ["⌘", "E"] });
  lab(M7.check, M7.sewn, "Checklists: tick one, and a cross is sewn in.");
  lab(M7.sewn, M7.end, "Callouts, and code blocks with a Copy.");

  // chapters: woven labels that hang in on a thread, then tie off; the knot flies to the border
  const CH = PLAN.filter((ch) => ch.name).map((ch) => ({ n: ch.n, name: ch.name, t: CHS[ch.n] + ch.tag, c: ch.c, x: ch.x }));
  const CH_HOLD = 2.6;

  // ======================================================================= the end: Initial
  // The last thread is tied off, and its knot leaves the page: the cloth comes up behind
  // it, the icon's tile is laid down and stitched round, the i is satin-stitched under
  // the knot, and the knot settles in as its dot. Then the wordmark is sewn on beside it.
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
    const from = (() => { const g = geo && geo.tags && geo.tags.t8; const e = g && g.ropeEnd ? g.ropeEnd : [1030, 640]; return toScreen(camAt(ED.lift[0]), e[0], e[1]); })();
    const lk = CLOTH(prog(t, ...ED.lift));
    const kx = lerp(from[0], D.x, lk), ky = lerp(from[1], D.y, lk) - 120 * Math.sin(Math.PI * prog(t, ...ED.lift));
    const kr = lerp(3.2 * camAt(ED.lift[0]).z, D.r, E.io(lk));
    const pop = t >= ED.settle ? knotScale(t - ED.settle + 0.12) : 1;
    const ko = 1 - prog(t, ED.settle + 0.05, ED.settle + 0.3);
    if (ko > 0) html += `<g opacity="${fx(ko)}">${knotSVG(kx, ky, kr, "var(--voice)", pop)}</g>`;
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

  // ======================================================================= frames
  function worldState(t) {
    // which note the main window shows
    const L = looks(t);
    let main = null;
    if (t < c2) {
      const page = startPage(t);
      main = { page, chromeOp: page.header.op };
    } else if (within(t, M7.help + 0.7, M7.model)) {
      const page = startPage(T.glide0); page.scroll = 0; page.header.op = 1;
      main = { page, chromeOp: 1 };
    } else {
      const { page } = mainNote(t);
      main = { page, chromeOp: page.header.op };
    }
    return { main, L };
  }

  /** Renders one shot of the desk into a scene layer. */
  function renderShot(layer, t, cam, state, o = {}) {
    const L = state.L;
    A.applyTheme(layer, L.dark, L.ink, { note: L.note, lettering: L.lettering });
    const world = layer.querySelector(".world");
    world.style.transform = `translate(${fx(960 - cam.x * cam.z)}px, ${fx(540 - cam.y * cam.z)}px) scale(${fx(cam.z * 1000) / 1000})`;
    let html = `<div class="desk"></div><div class="win" id="${layer.id}-win"></div>`;
    if (o.settings) html += `<div class="setwin" style="position:absolute;left:${SET.x}px;top:${SET.y}px;opacity:${fx(o.settings.op == null ? 1 : o.settings.op)};transform:translateY(${fx((1 - (o.settings.op == null ? 1 : o.settings.op)) * 24)}px)"></div>`;
    world.innerHTML = html;
    const win = world.querySelector(".win");
    const W = Object.assign({ voice: L.ink }, state.main, o.win || {});
    const geo = A.renderWindow(win, W, t, cam.z);
    if (o.settings) window.SETTINGS.render(world.querySelector(".setwin"), o.settings, t);
    return geo;
  }

  // ======================================================================= renderAt
  let lastGeo = null;
  function renderAt(t) {
    let cam = camAt(t);
    const A_ = $("#sceneA"), B_ = $("#sceneB");
    B_.style.display = "none";
    B_.style.clipPath = "";
    const st = worldState(t);
    // transitions show two moments at once; they render both layers themselves
    const tr = transitionAt(t);
    let fxHTML = "", geo;
    if (tr) { const r = tr(t, A_, B_); fxHTML = r.html; geo = r.geo; cam = r.cam; }
    else geo = renderShot(A_, t, cam, st, shotExtras(t));
    lastGeo = geo;
    A.applyTheme($("#stage"), st.L.dark, st.L.ink, {});
    $("#fx").innerHTML = fxHTML + coldOpenFX(t, cam);
    $("#film").innerHTML = captionsHTML(t) + chaptersHTML(t) + borderHTML(t) + flyHTML(t, cam, geo) + endHTML(t, cam, geo);
    renderPointer(t, cam, geo);
    $("#fade").style.opacity = String(fx(Math.max(1 - prog(t, 0, 0.8), prog(t, DUR - 1.2, DUR))));
  }

  /** Per-moment additions to the desk: the Settings window, overlays inside the app window. */
  function shotExtras(t) {
    const o = { win: {} };
    // Settings: ch5 New mind, ch6 most beats, ch7 Model
    const setPane = (() => {
      if (within(t, c5, T.pan16[0] + 0.4)) return "NewMind";
      if (within(t, c6, M6.mode) || within(t, M6.drop, M6.drop + 0.6)) return "Appearance";
      if (within(t, M6.pace, M6.drop)) return "Reading";
      if (within(t, M6.learn, M6.keys)) return "About you";
      if (within(t, M6.keys, c7 + 0.6)) return "Shortcuts";
      if (within(t, M7.model, M7.blocks)) return "Model";
      return null;
    })();
    if (setPane) {
      const S = { pane: setPane, op: within(t, T.pan16[0], T.pan16[0] + 0.4) ? 1 - prog(t, T.pan16[0], T.pan16[0] + 0.4) : 1 };
      const L = looks(t);
      if (setPane === "NewMind") {
        S.tmpl = t >= T.blank15 ? "Blank" : null;
        const chipsHov = ["Staff engineer", "Editor", "Skeptic", "Socratic tutor", "Investor"];
        if (within(t, T.chips15, T.blank15)) S.hovChip = chipsHov[clamp(Math.floor((t - T.chips15) / 0.3), 0, 4)];
        if (within(t, T.blank15 - 0.3, T.blank15)) S.hovChip = "Blank";
        S.name = MIND.name.slice(0, count(S_name, t)); S.desc = MIND.description.slice(0, count(S_desc, t)); S.how = MIND.howItThinks.slice(0, count(S_how, t));
        S.focus = within(t, T.name15[0], T.desc15[0]) ? "name" : within(t, T.desc15[0], T.how15[0]) ? "desc" : within(t, T.how15[0], T.voice15) ? "how" : null;
        S.voice = t >= T.voice15 ? "Blunt" : "Plain"; S.often = t >= T.often15 ? "Often" : "When it matters";
        S.hovCreate = within(t, T.create15 - 0.3, T.create15 + 0.2);
        S.scroll = 300 * E.io(prog(t, T.how15[0] - 0.2, T.voice15));
      }
      if (setPane === "Appearance") {
        S.look = t >= M6.dark + 0.85 ? "dark" : "light"; S.size = "default"; S.lettering = "own"; S.ink = L.ink; S.inkDrop = t >= M6.drop + 0.3;
        if (within(t, M6.ink, M6.letter)) S.scroll = SCROLL.ink;
        if (within(t, M6.letter, M6.size)) { S.scroll = SCROLL.letter; S.lettering = t >= M6.letter + 0.35 ? "notes" : "own"; }
        if (within(t, M6.size, M6.mode)) { S.scroll = SCROLL.size; S.size = t < M6.size + 0.45 ? "small" : t < M6.size + 0.95 ? "large" : "default"; }
        if (within(t, M6.drop, M6.drop + 0.6)) S.scroll = SCROLL.drop;
      }
      if (setPane === "Reading") { S.pace = t >= M6.pace + 0.35 ? "slow" : "regular"; S.placement = "both"; }
      if (setPane === "About you") { S.learnK = E.out(prog(t, M6.learn + 0.25, M6.learn + 0.45)); S.learn = S.learnK > 0.5; S.about = t >= M6.learn + 0.95 ? streamed(TXT.learned, M6.learn + 0.95, M6.learn + 1.7, t) : ""; }
      if (setPane === "Shortcuts") {
        S.scroll = SCROLL.keys; S.rec = { recording: within(t, M6.keys + 0.45, M6.keys + 1.3), hide: t >= M6.keys + 1.3 ? "⌥⌘H" : within(t, M6.keys + 0.45, M6.keys + 1.3) ? "Press keys" : "⇧⌘.", hov: within(t, M6.keys + 0.2, M6.keys + 0.5) };
        // Tab, then Tab again: the stitched ring moves down the rail
        if (within(t, M6.ring + 0.3, c7)) S.ring = t < M6.ring + 0.9 ? "Appearance" : "Minds";
      }
      if (setPane === "Model") { S.model = 0; S.hovModel = within(t, M7.model + 0.4, M7.model + 1.0) ? 1 : null; S.effRead = t >= M7.model + 1.5 ? "medium" : "low"; S.effAsk = "medium"; S.scroll = t >= M7.claude ? 0 : SCROLL.model; }
      o.settings = S;
    }
    const W = o.win;
    // chat (ch3 ends in it)
    if (within(t, T.chatIn8[0], T.chatOut9[1])) {
      const k = t < T.chatOut9[0] ? E.out(prog(t, ...T.chatIn8)) : 1 - E.out(prog(t, ...T.chatOut9));
      const ans = t >= T.chatStream8[0] ? streamed(TXT.chat, ...T.chatStream8, t) : "";
      const { page } = mainNote(t);
      const all = page.threads.length, open = page.threads.filter((th) => !th.tied).length;
      W.chat = { k, title: TITLE, threads: `${all} threads (${open} open)`, empty: t < T.chatSend8, hovPill: within(t, T.pills8, T.pills8 + 0.5) ? 0 : within(t, T.pills8 + 0.5, T.chatType8[0]) ? 2 : null,
        me: t >= T.chatSend8 ? TXT.chatQ : null, think: within(t, T.chatSend8 + 0.1, T.chatStream8[0]), ans, ansShift: Math.max(0, (ans.length - 520) * 0.12), input: t < T.chatSend8 ? TXT.chatQ.slice(0, count(S.chat, t)) : "", caret: t < T.chatSend8 && t >= T.chatType8[0] && caretOn(t), hovClear: t >= T.clear8 };
    }
    // the sidebar (ch7), the palette, export, menus of the menu bar
    if (within(t, M7.side, M7.find) || within(t, M7.obs, M7.help)) {
      const obs = within(t, M7.obs, M7.help);
      const k = obs ? prog(t, M7.obs + 0.7, M7.obs + 0.95) : E.out(prog(t, M7.side + 0.15, M7.side + 0.45)) * (1 - prog(t, M7.find - 0.25, M7.find));
      W.sideK = k;
      const vault = obs ? NOTEBOOK.vault.map(([f, ns], i) => ({ name: f, open: true, notes: ns.slice(0, clamp(Math.floor((t - M7.obs - 1.1 - i * 0.35) / 0.18), 0, ns.length)).map((nm) => ({ title: nm })), show: t >= M7.obs + 1.0 + i * 0.35 })).filter((f) => f.show) : [];
      const [plat, home, reading] = NOTEBOOK.folders;
      W.side = { who: "you@example.com", folders: [
        { name: plat[0], open: true, notes: plat[1].map((nm) => ({ title: nm, on: nm === TITLE, dot: nm === TITLE })) },
        { name: home[0], open: true, notes: home[1].map((nm, i) => ({ title: nm, dot: i === 1, hov: i === 1 && within(t, M7.side + 1.6, M7.side + 2.2) })) },
        { name: reading[0], open: !obs && t >= M7.side + 1.2, hov: within(t, M7.side + 0.9, M7.side + 1.25), notes: reading[1].map((nm) => ({ title: nm })) }, ...vault],
        loose: NOTEBOOK.loose.map((nm) => ({ title: nm })), hovNew: within(t, M7.side + 2.2, M7.find) };
    }
    if (within(t, M7.find, M7.copy)) W.overlay = paletteHTML(t);
    if (within(t, M7.copy, M7.exp)) W.toast = { text: "Copied as Markdown", op: prog(t, M7.copy + 0.2, M7.copy + 0.35) * (1 - prog(t, M7.exp - 0.2, M7.exp)) };
    if (within(t, M7.exp, M7.obs)) W.overlay = exportHTML(t);
    if (within(t, M7.obs, M7.obs + 0.9)) W.overlay = macMenuHTML("File", ["New Note  ⌘N", "Find or Start a Note…  ⌘K", "—", "Copy This Note  ⇧⌘C", "Export This Note…  ⇧⌘E", "—", "Import from Obsidian…"], t >= M7.obs + 0.5 ? 6 : -1, t >= M7.obs + 0.2);
    if (within(t, M7.help, M7.help + 0.7)) W.overlay = macMenuHTML("Help", ["Welcome Guide"], t >= M7.help + 0.35 ? 0 : -1, t >= M7.help + 0.12);
    // the snip's label, with its Undo
    if (within(t, T.snip10, T.undo10 + 0.3)) W.undo = { op: prog(t, T.snip10 + 0.3, T.snip10 + 0.55) * (1 - prog(t, T.undo10, T.undo10 + 0.25)) };
    // the listening badge and ink drops (ch6)
    if (within(t, M6.speak + 0.15, M6.speak + 2.1)) W.listen = { op: prog(t, M6.speak + 0.15, M6.speak + 0.3) * (1 - prog(t, M6.speak + 1.95, M6.speak + 2.1)), text: t < M6.speak + 0.6 ? "Listening…" : P7.slice(0, Math.round(P7.length * prog(t, M6.speak + 0.6, M6.speak + 1.7))) };
    if (within(t, M6.drop, M6.speak)) W.blots = blotsSVG(t);
    if (within(t, T.big7[0], T.fold7 + 0.3)) W.overlay = enlargedHTML(t);
    return o;
  }
  const S_name = S.name, S_desc = S.desc, S_how = S.how;

  function paletteHTML(t) {
    const word = "boot";
    const q = word.slice(0, clamp(Math.floor((t - M7.find - 0.5) / 0.09), 0, word.length));
    const all = [["Home", "Kitchen shelf measurements"], ["Reading", "Reading list for winter"], ["Platform", "Kernel upgrade notes"], ["", "Questions for the landlord"], ["Platform", TITLE, "Open now"], ["Platform", NOTEBOOK.related.title]];
    const list = q ? all.filter(([, nm]) => nm.toLowerCase().includes(q)) : all;
    const k = E.out(prog(t, M7.find + 0.1, M7.find + 0.3)) * (1 - prog(t, M7.copy - 0.2, M7.copy));
    return `<div class="modal-dim" style="background:rgba(var(--shade),.12);opacity:${fx(k)}"></div><div class="palette" style="opacity:${fx(k)};transform:translateY(${fx((1 - k) * -8)}px)"><div class="s">${A.icon("search", 14, "var(--faint)")}${q ? esc(q) + '<span class="icaret"></span>' : '<span class="ph">Find or start a note</span>'}</div>` +
      list.map(([f, nm, s], i) => `<div class="li${i === 0 ? " on" : ""}">${f ? `<span class="f">${f} /</span>` : ""}<span class="grow">${esc(nm)}</span>${s ? `<small>${s}</small>` : ""}</div>`).join("") +
      `<div class="ft" style="top:auto;bottom:0;position:absolute"><span>↵ open</span><span>“Folder / Name” files a new note</span><span>⌘⌫ delete</span></div></div>`;
  }
  function exportHTML(t) {
    const k = E.out(prog(t, M7.exp + 0.1, M7.exp + 0.3)) * (1 - prog(t, M7.obs - 0.2, M7.obs));
    const sel = t < M7.exp + 0.95 ? 0 : t < M7.exp + 1.55 ? 1 : 2;
    const inc = !(within(t, M7.exp + 1.9, M7.exp + 2.25));
    const opt = (i, tl, d) => `<div class="opt${sel === i ? " on" : ""}"><div class="t">${tl}</div><div class="d">${d}</div></div>`;
    return `<div class="modal-dim" style="background:rgba(var(--shade),.12);opacity:${fx(k)}"></div><div class="export" style="opacity:${fx(k)};transform:scale(${fx(0.97 + 0.03 * k)})"><h4>Export “${esc(TITLE)}”</h4><div class="opts">${opt(0, "Markdown", "Plain text for anywhere, another AI included.")}${opt(1, "Word", "To keep editing in Word, Pages or Google Docs.")}${opt(2, "HTML", "To read or print to PDF.")}</div>` +
      `<div class="inc"><div style="flex:1"><div class="t">Include the margin</div><div class="d">Each thread sits under the paragraph it’s about.</div></div><span class="sw${inc ? " on" : ""}"><i style="left:${inc ? 14 : 2}px"></i></span></div><div class="btns"><span>Cancel</span><b>Export…</b></div></div>`;
  }
  function macMenuHTML(name, items, hov, open) {
    const tab = (nm) => `<span style="padding:1px 8px;border-radius:4px;${nm === name ? "background:color-mix(in srgb,var(--ink) 14%,transparent)" : ""}">${nm}</span>`;
    const bar = `<div class="macmenu" style="left:-20px;top:-38px;width:1360px;height:26px;display:flex;gap:10px;align-items:center;padding-left:22px;background:color-mix(in srgb,var(--shelf) 82%,transparent);color:var(--ink);border-radius:6px;box-shadow:0 0 0 1px var(--rule)"><b style="font-weight:600;padding:0 6px">Inkling</b>${tab("File")}${tab("View")}${tab("Help")}</div>`;
    if (!open) return bar;
    const x = name === "File" ? 76 : 214;
    return bar + `<div class="macmenu card" style="left:${x}px;top:-11px;width:250px;padding:5px">` + items.map((it, i) => (it === "—" ? `<div style="height:1px;background:var(--rule);margin:5px 8px"></div>` :
      `<div style="display:flex;justify-content:space-between;padding:5px 10px;border-radius:5px;${i === hov ? "background:var(--indigo);color:#fff" : ""}"><span>${esc(it.split("  ")[0])}</span><span style="opacity:.6">${esc(it.split("  ")[1] || "")}</span></div>`)).join("") + `</div>`;
  }
  const DROPS = [[M6.drop + 1.05, 812, 452], [M6.drop + 1.55, 905, 520], [M6.drop + 2.05, 990, 470]];
  function blotsSVG(t) {
    let s = "";
    DROPS.forEach(([b, x, y], n) => {
      const age = t - b;
      if (age < 0) return;
      const r0 = rng(17 + n);
      const alpha = age < 0.35 ? 0.85 : Math.max(0, 0.85 - (age - 0.35) * 0.36), grow = Math.min(1, age * 6);
      const r = (2.4 + 2.6 * r0()) * (0.6 + 0.4 * grow);
      s += `<circle cx="${x}" cy="${y}" r="${fx(r)}" style="fill:var(--ink)" opacity="${fx(alpha)}"/>`;
      const mm = 2 + Math.floor(r0() * 4);
      for (let i = 0; i < mm; i++) { const a = r0() * Math.PI * 2, d = r + 2 + r0() * 7 * grow; s += `<circle cx="${fx(x + Math.cos(a) * d)}" cy="${fx(y + Math.sin(a) * d)}" r="${fx(0.6 + r0() * 1.4)}" style="fill:var(--ink)" opacity="${fx(alpha)}"/>`; }
    });
    return s;
  }
  function enlargedHTML(t) {
    const k = E.out(prog(t, T.big7[0], T.big7[0] + 0.35)) * (1 - prog(t, T.fold7 - 0.1, T.fold7 + 0.2));
    const sk = sketchSVG(t, T.bigSteps, 1.35);
    return `<div class="modal-dim" style="background:rgba(var(--shade),.28);opacity:${fx(k)}"></div><div class="card" style="left:360px;top:40px;width:600px;height:780px;z-index:37;padding:24px 40px;opacity:${fx(k)};transform:scale(${fx(0.96 + 0.04 * k)})">` +
      `<div style="display:flex;justify-content:center">${sk.svg}</div><div style="margin-top:10px;font-size:13px;color:var(--graphite);text-align:center">${esc(sk.caption)}</div></div>`;
  }

  // ======================================================================= transitions
  function transitionAt(t) {
    if (within(t, ...T.wipe1)) return (tt, a, b) => unfoldWipe(tt, a, b, "f6", T.wipe1, c2 - 0.01);
    if (within(t, ...T.wipe17)) return (tt, a, b) => unfoldWipe(tt, a, b, "t8", T.wipe17, c6 - 0.01);
    if (within(t, c7 - 0.6, c7 + 0.2)) return (tt, a, b) => stitchWipe(tt, a, b, [c7 - 0.6, c7 + 0.2], () => [66, 1254, 68]);
    return null;
  }
  /** A tag unfolding is the wipe: its panel grows over the frame, and the next scene is
   *  inside it. The outgoing side keeps moving until `outEnd`; the incoming side waits at
   *  its first frame. */
  function unfoldWipe(t, a, b, tagId, span, outEnd) {
    const tOut = Math.min(t, outEnd), camO = camAt(tOut);
    const geo = renderShot(a, tOut, camO, worldState(tOut), shotExtras(tOut));
    const g = geo && geo.tags && geo.tags[tagId];
    const r0 = g && g.panel ? [g.panel.x, g.panel.y, g.panel.w, g.panel.h] : [900, 500, 248, 160];
    const p0 = toScreen(camO, r0[0], r0[1]), p1 = toScreen(camO, r0[0] + r0[2], r0[1] + r0[3]);
    const k = CLOTH(prog(t, span[0] + 0.05, span[1] - 0.1));
    const x0 = lerp(p0[0], -8, k), y0 = lerp(p0[1], -8, k), x1 = lerp(p1[0], 1928, k), y1 = lerp(p1[1], 1088, k), rad = lerp(10 * camO.z, 0, k);
    const tIn = span[1] + 0.001, camI = camAt(tIn);
    b.style.display = "block";
    b.style.clipPath = `inset(${fx(y0)}px ${fx(1920 - x1)}px ${fx(1080 - y1)}px ${fx(x0)}px round ${fx(rad)}px)`;
    renderShot(b, tIn, camI, worldState(tIn), shotExtras(tIn));
    const op = 1 - prog(t, span[1] - 0.3, span[1]);
    const inset = lerp(4, 16, k);
    const html = `<div style="position:absolute;left:${fx(x0)}px;top:${fx(y0)}px;width:${fx(x1 - x0)}px;height:${fx(y1 - y0)}px;border-radius:${fx(rad)}px;box-shadow:0 0 0 1px rgba(var(--shade),.15),0 30px 60px -20px rgba(var(--shade),.6);opacity:${fx(op)}"></div>` +
      `<div style="position:absolute;left:${fx(x0 + inset)}px;top:${fx(y0 + inset)}px;width:${fx(x1 - x0 - 2 * inset)}px;height:${fx(y1 - y0 - 2 * inset)}px;border-radius:${fx(Math.max(4, rad * 0.8))}px;border:${fx(lerp(1, 2.5, k))}px dashed color-mix(in srgb,var(--voice) 50%,transparent);opacity:${fx(op)}"></div>`;
    return { html, geo, cam: camO };
  }
  /** A running stitch sews across the frame. Behind the needle is the next scene; when it
   *  has crossed, the stitch line draws in to the next scene's own line (`target`, world
   *  [x0, x1, y]) and hands over to it. */
  function stitchWipe(t, a, b, span, target) {
    const tIn = span[1] + 0.001, tOut = span[0] - 0.001;
    const camI = camAt(tIn), camO = camAt(tOut);
    const geo = renderShot(a, tIn, camI, worldState(tIn), shotExtras(tIn));
    const cross = prog(t, span[0], span[1] - 0.25), draw = CLOTH(prog(t, span[1] - 0.3, span[1]));
    const nx = lerp(-60, 1980, E.io(cross));
    b.style.display = "block";
    b.style.clipPath = `inset(0 0 0 ${fx(clamp(nx, 0, 1920))}px)`;
    renderShot(b, tOut, camO, worldState(tOut), shotExtras(tOut));
    // the line across the frame, and where it ends up
    const tg = target();
    const ty = tg ? toScreen(camI, 0, tg[2])[1] : 600;
    const lx0 = tg ? toScreen(camI, tg[0], 0)[0] : 0, lx1 = tg ? toScreen(camI, tg[1], 0)[0] : 1920;
    const y = ty;
    const X0 = lerp(-60, lx0, draw), X1 = lerp(Math.min(nx, 1980), lx1, draw);
    let st = "";
    const n = 140;
    for (let i = 0; i < n; i++) {
      const u0 = i / n, u1 = (i + 0.62) / n;
      const xa = lerp(-60, 1980, u0), xb = lerp(-60, 1980, u1);
      if (xa > nx) break;
      const m0 = lerp(X0, X1, (xa + 60) / 2040), m1 = lerp(X0, X1, Math.min(xb, nx) + 60 > 0 ? (Math.min(xb, nx) + 60) / 2040 : 0);
      const yy = y + A.jitter(97, i) * 1.4 * (1 - draw);
      st += capsule(draw > 0 ? m0 : xa, yy, draw > 0 ? m1 : Math.min(xb, nx), yy, lerp(3.4, 1.8 * camI.z, draw), "var(--voice)", 1 - prog(t, span[1] - 0.06, span[1]));
    }
    const needle = cross < 1 ? `<g transform="translate(${fx(nx)} ${fx(y)}) rotate(${fx(-6 + 12 * Math.sin(t * 26))})"><path d="M-34 0L20 0" style="stroke:var(--graphite);stroke-width:3;stroke-linecap:round"/><ellipse cx="-27" cy="0" rx="4.4" ry="1.5" style="fill:none;stroke:var(--graphite);stroke-width:1.2"/></g>` : "";
    return { html: `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible">${st}${needle}</svg>`, geo, cam: camI };
  }

  // ======================================================================= the cold open
  function coldOpenFX(t, cam) {
    if (t > T.pull1 + 0.1) return "";
    // one running stitch across the cloth, under the words; when the camera tilts down to
    // the window it settles onto the sheet's border, which is where every tie-off will go
    const sewK = E.io(prog(t, T.sew0, T.sew1));
    const morph = CLOTH(prog(t, T.pull0, T.pull1 - 0.15));
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
    const lift = -60 * E.in(prog(t, T.lineUp, T.lineUp + 0.32)) + 4 * Math.sin(prog(t, T.openTie - 0.12, T.openTie + 0.1) * Math.PI);
    let html = `<div style="position:absolute;left:0;right:0;top:${fx(400 + lift)}px;text-align:center;font:500 108px/1 'Fraunces Display';letter-spacing:-.02em;color:var(--ink);opacity:${fx(out)}">`;
    words.forEach((w, i) => {
      const wk = clamp((k * words.length - i) * 1.4);
      const rise = (1 - spring(Math.max(0, (k * words.length - i)) * 0.5, 160, 14)) * 26;
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
      const inK = spring(t - c.t0, 300, 26);
      const sew = prog(t, c.t0 + 0.08, c.t0 + 0.08 + clamp(0.3 + c.text.length * 0.012, 0.45, 0.95));
      // out: tied off. The thread under the words pulls taut, a knot pops, the patch lifts away
      const tie0 = c.t1 - 0.36, taut = prog(t, tie0, tie0 + 0.14), pop = t - (tie0 + 0.14), yank = prog(t, c.t1 - 0.17, c.t1);
      const dy = -4 * Math.sin(Math.PI * prog(t, tie0 + 0.1, c.t1 - 0.17)) - 70 * E.in(yank);
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
  function keysHTML(t) { return ""; }

  /** Chapter labels: woven tags that drop in on a thread from the top of the frame, swing,
   *  and are tied off; the knot flies down to the border. */
  const tagRopes = new Map();
  function chaptersHTML(t) {
    let html = "";
    for (const ch of CH) {
      const t0 = ch.t, t1 = ch.t + CH_HOLD;
      if (t < t0 || t > t1 + 0.6) continue;
      const ax = ch.x, ay = -12; // where its thread is pinned, above the frame
      const drop = spring(t - t0, 70, 6.2); // a weighted swing in
      const tie = t >= t1 - 0.5;
      const pull = E.in(prog(t, t1 - 0.12, t1 + 0.28));
      const ty = lerp(-260, 168, drop) - 360 * pull;
      const sway = 9 * Math.sin((t - t0) * 3.1) * Math.exp(-(t - t0) * 1.4);
      const tx = ax + sway;
      const loop = [tx, ty - 4];
      // the thread from the pin to the tag's loop
      const rope = A.simRope("ch" + ch.n, [ax, ay], loop, t0, Math.min(t, t1 + 0.3), { b: (s) => { const d = spring(s - t0, 70, 6.2); const p = E.in(prog(s, t1 - 0.12, t1 + 0.28)); return [ax + 9 * Math.sin((s - t0) * 3.1) * Math.exp(-(s - t0) * 1.4), lerp(-260, 168, d) - 360 * p - 4]; }, tieAt: t1 - 0.5, opts: { maxSlack: 60, slackK: 0.12 } });
      const c = `var(--${ch.c})`;
      html += `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible">${A.ropeSVG(rope, c, 0.95)}${t >= t1 - 0.22 ? knotSVG(loop[0], loop[1], 5.2, c, knotScale(t - (t1 - 0.22))) : ""}</svg>`;
      const rot = sway * 0.35;
      html += `<div class="chtag" style="left:${fx(tx)}px;top:${fx(ty)}px;--tc:${c};transform:translateX(-50%) rotate(${fx(rot)}deg)"><div class="lb"><div class="no">${String(ch.n).padStart(2, "0")}</div><div class="nm">${esc(ch.name)}</div></div><i class="loop"></i></div>`;
    }
    return html;
  }
  /** The sampler border at the foot of the frame: the film's progress, one motif a chapter. */
  const BORDER = { x0: 760, x1: 1160, y: 1048 };
  const slotX = (i) => BORDER.x0 + ((i + 0.5) * (BORDER.x1 - BORDER.x0)) / 7;
  function borderHTML(t) {
    if (t < T.pull1 - 0.8 || t > cEnd + 1.6) return "";
    const op = prog(t, T.pull1 - 0.8, T.pull1) * (1 - prog(t, cEnd + 0.9, cEnd + 1.6));
    let s = "";
    // the dashed ground
    const done = CH.filter((ch) => t >= ch.t + CH_HOLD + 0.7).length;
    for (let i = 0; i < 7; i++) {
      const x = slotX(i);
      const landed = CH[i] && t >= CH[i].t + CH_HOLD + 0.7;
      if (!landed) s += capsule(x - 9, BORDER.y, x + 9, BORDER.y, 1.6, "color-mix(in srgb,var(--faint) 60%,transparent)");
      else s += motifSVG(x, BORDER.y, `var(--${CH[i].c})`, prog(t, CH[i].t + CH_HOLD + 0.7, CH[i].t + CH_HOLD + 1.1) * 1.15);
      if (i < 6) s += capsule(x + 14, BORDER.y, slotX(i + 1) - 14, BORDER.y, 1.2, "color-mix(in srgb,var(--faint) 35%,transparent)");
    }
    return `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible;opacity:${fx(op)}">${s}</svg>`;
  }
  /** Knots in flight: a chapter's knot to the border; an app thread's knot to the page border. */
  function flyHTML(t, cam, geo) {
    let s = "";
    for (const ch of CH) {
      const a = ch.t + CH_HOLD - 0.1, b = a + 0.8;
      if (!within(t, a, b)) continue;
      const k = CLOTH(prog(t, a, b));
      const from = [ch.x, 160], to = [slotX(ch.n - 1), BORDER.y];
      const x = lerp(from[0], to[0], k), y = lerp(from[1], to[1], k) - 180 * Math.sin(Math.PI * prog(t, a, b));
      s += knotSVG(x, y, 6, `var(--${ch.c})`, 1);
    }
    // in the app: a tied thread's knot joins the border at the top of the page
    for (const [a, b] of [T.fly2, T.fly9]) {
      if (!within(t, a, b) || !geo) continue;
      const id = a === T.fly2[0] ? "t1" : "t2";
      const g = geo.tags[id];
      const from = g && g.ropeEnd ? g.ropeEnd : [1000, 300];
      const k = CLOTH(prog(t, a, b));
      const wx = lerp(from[0], 660, k), wy = lerp(from[1], 68, k) - 60 * Math.sin(Math.PI * prog(t, a, b));
      const p = toScreen(cam, wx, wy);
      s += knotSVG(p[0], p[1], 3.2 * cam.z, "var(--voice)", 1);
    }
    return s ? `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible">${s}</svg>` : "";
  }

  // ======================================================================= the pointer
  // Moves ease like a hand, on a slight arc; a press dips it. Targets are world points, or
  // functions of the frame's measured geometry (tags, the selection menu's rows, chips).
  function pointerPath(t, cam, geo) {
    const G = geo || { tags: {} };
    const brief = (x) => (x >= T.save8 ? 28 : 0);
    const tag = (id, dx = 26, dy = 13) => () => { const g = G.tags[id]; return g ? [g.x + dx, g.y + dy] : null; };
    const at = (x, y) => () => [x, y];
    const word = (id, f = 0.5, line = 0, from = "main") => () => { const Ls = (MEAS[from].anchors[id] || []); const L = Ls[line < 0 ? Ls.length + line : line]; return L ? [lerp(L.x, L.r, f), L.y + 15 + brief(t) - (from === "main" ? scrollAt(t) : 0)] : null; };
    const centre = (r) => (r ? [r.x + r.w / 2, r.y + r.h / 2] : null);
    const act = (k) => () => centre(G.acts && G.acts[k]);
    const chip = (name) => () => centre(G.chips && G.chips[name]);
    const panel = (id, dx, dy) => () => { const g = G.tags[id]; return g && g.panel ? [g.panel.x + (dx >= 0 ? dx : g.panel.w + dx), g.panel.y + g.panel.h + dy] : null; };
    const blk = (id, dx, dy) => () => { const b = MEAS.main.blocks[id]; return b ? [b.x + dx, b.y + dy + brief(t) - scrollAt(t)] : null; };
    const keys = [
      [T.hover1 - 0.7, T.hover1, tag("f6")],
      [T.t1hover - 0.7, T.t1hover, tag("t1")],
      [T.e1sel - 0.4, T.e1sel, word("e1", 0.05, 0, "draft"), true],
      [T.e2drag[0] - 0.3, T.e2drag[0], word("e2", 0, 0, "draft"), true],
      [T.e2drag[0], T.e2drag[1], word("e2", 1, -1, "draft"), true],
      [T.sel3[0] - 0.4, T.sel3[0], word("t2", 0), true], [T.sel3[0], T.sel3[1], word("t2", 1), true], [T.find3 - 0.35, T.find3, act("find")],
      [T.sel4[0] - 0.4, T.sel4[0], word("t3", 0), true], [T.sel4[0], T.sel4[1], word("t3", 1), true], [T.askClick4 - 0.4, T.askClick4, act("ask")],
      [T.sel5[0] - 0.4, T.sel5[0], word("t4", 0), true], [T.sel5[0], T.sel5[1], word("t4", 1, -1), true], [T.askClick5 - 0.4, T.askClick5, act("ask")],
      [T.chip5a - 0.3, T.chip5a, chip("Staff engineer")], [T.chip5b - 0.3, T.chip5b, chip("Socratic tutor")],
      [T.hov5b - 0.3, T.hov5b, tag("t5")], [T.hov5c - 0.3, T.hov5c, tag("t6")],
      [T.sel7[0] - 0.4, T.sel7[0], word("k1", 0), true], [T.sel7[0], T.sel7[1], word("k1", 1, -1), true], [T.sketch7 - 0.4, T.sketch7, act("sketch")],
      [T.larger7 - 0.4, T.larger7, panel("k1", 52, -18)],
      [T.tell8 - 0.45, T.tell8, at(480, 106)],
      [T.hov9[0] - 0.3, T.hov9[0], tag("t4")], [T.hov9[2] - 0.3, T.hov9[2], tag("t5")], [T.hov9[4] - 0.3, T.hov9[4], tag("t6")],
      [T.drag9[0] - 0.4, T.drag9[0], tag("t2")], [T.drag9[0], T.drag9[1], tag("t2")],
      [T.replyFocus9 - 0.3, T.replyFocus9, panel("t2", 60, -60)],
      [T.resolveHov9 - 0.35, T.resolveHov9, panel("t2", 30, -20)],
      [T.open10 - 0.4, T.open10, tag("t3")],
      [T.reopen11 - 0.4, T.reopen11, word("t2", 0.5)],
      [T.reopen11 + 0.5, T.reopen11 + 0.8, panel("t2", 30, -20)],
      [T.reopen11 + 1.0, T.reopen11 + 1.3, panel("t2", -22, -20)],
      [T.reopen11 + 1.45, T.reopen11 + 1.75, panel("t2", -80, -20)],
      [T.menu16 - 0.45, T.menu16, at(385, 106 + 28)], [T.pick16 - 0.45, T.pick16, at(400, 106 + 28 + 92)],
      ...DROPS.map(([b, x, y]) => [b - 0.45, b, at(x, y)]),
      [M7.code + 0.0, M7.code + 0.2, word("ice", 0), true], [M7.code + 0.2, M7.code + 0.55, word("ice", 1), true], [M7.code + 0.6, M7.code + 0.75, () => centre(G.marks && G.marks[4])],
      [M7.check + 0.55, M7.check + 1.0, blk("s2", 9, 14)],
      [M7.sewn + 0.4, M7.sewn + 0.9, blk("s6", 420, 8)], [M7.sewn + 1.2, M7.sewn + 1.5, blk("s5", 614 - 30, -15)],
      [ED.act - 0.4, ED.act, tag("t8", 120, 40)],
      [ED.hov - 0.35, ED.hov, panel("t8", 30, -20)],
      [ED.tie - 0.1, ED.tie, panel("t8", 31, -19)],
    ];
    let cur = null, ibeam = false, press = 0, visible = false;
    for (let i = 0; i < keys.length; i++) {
      const [a, b, fn, ib] = keys[i];
      if (t < a - 0.5 || t > b + 0.9 || (t > ED.knot + 0.5 && b < ED.act)) continue;
      const p = fn();
      if (!p) continue;
      const prev = i > 0 && keys[i - 1][1] > a - 1.0 ? keys[i - 1][2]() : null;
      const from = prev || [p[0] + 120, p[1] + 160];
      const k = E.io(prog(t, a, b));
      const arc = Math.sin(Math.PI * k) * 18;
      cur = [lerp(from[0], p[0], k) + arc * 0.3, lerp(from[1], p[1], k) - arc];
      ibeam = !!ib;
      visible = true;
      press = within(t, b, b + 0.12) ? 1 : 0;
    }
    if (t > ED.knot + 0.4) visible = false;
    return visible ? { p: cur, ibeam, press } : null;
  }
  function renderPointer(t, cam, geo) {
    const el = $("#cursor");
    const P = pointerPath(t, cam, geo);
    if (!P) { el.style.display = "none"; return; }
    const s = toScreen(cam, P.p[0], P.p[1]);
    el.style.display = "block";
    const sc = 1.25 * (P.press ? 0.9 : 1);
    el.style.transform = `translate(${fx(s[0])}px, ${fx(s[1])}px) scale(${fx(sc)})`;
    el.innerHTML = P.ibeam
      ? `<svg width="14" height="26" viewBox="-7 -13 14 26"><path d="M-4 -11h8M-4 11h8M0 -11v22" style="stroke:#fff;stroke-width:3.4;stroke-linecap:round"/><path d="M-4 -11h8M-4 11h8M0 -11v22" style="stroke:#111;stroke-width:1.4;stroke-linecap:round"/></svg>`
      : `<svg width="22" height="30" viewBox="0 0 22 30"><path d="M1.5 1.5v22.5l6-5.4 3.8 8.6 3.6-1.6-3.8-8.4 8-.6z" style="fill:#111;stroke:#fff;stroke-width:1.5;stroke-linejoin:round"/></svg>`;
  }

  // ======================================================================= the score's cues
  // The score (audio.js) is one quiet piece under the whole film, not a sound per action.
  // It reads only these: where each chapter starts (its harmony moves there), the three
  // tie-offs (a low, soft knot under the music), the hidden threads (the melody rests),
  // and the end (the knot becomes the dot of the i; the piece comes home).
  const EVENTS = [];
  function buildEvents() {
    const ev = (o) => EVENTS.push(o);
    for (const ch of PLAN) ev({ type: "chapter", t: CHS[ch.n], n: ch.n, name: ch.name || (ch.n === 0 ? "open" : "end") });
    ev({ type: "tie", t: T.knot2 }); ev({ type: "tie", t: T.knot9 }); ev({ type: "tie", t: ED.knot, last: true });
    ev({ type: "hush", t0: T.hide12, t1: T.show12 + 0.3 });
    ev({ type: "settle", t: ED.settle });
    ev({ type: "fin", t: DUR });
    EVENTS.sort((a, b) => (a.t ?? a.t0) - (b.t ?? b.t0));
  }

  // ======================================================================= coverage
  // Every row of design/capabilities.md, and when the film shows it (render.ts coverage
  // writes coverage.md from this, so the times always match the cut).
  const COVERAGE = [];
  function buildCoverage() {
    const c = (section, row, t, what) => COVERAGE.push({ section, row, t: Math.round(t * 10) / 10, what });
    const openAt = mainNote(T.numbers9 + 0.5).page.header.loose;
    c("Writing", "A plain page: title, paragraphs, three heading sizes, bullet and numbered lists, quotes", M7.blocks + 0.3, "The dictated question is selected and the selection menu’s first row turns it into H1, H2, H3, a quote (italic, on a back-stitched seam), a bullet and a numbered item, one per half beat; the guide’s headings and bullets go past in chapter 1");
    c("Writing", "Checklists", M7.check, "At the foot of the note, a three-item checklist with one item done and tacked through; a click on the next box sews its cross-stitch in, then tacks the line; the guide’s own checklist goes past in chapter 1");
    c("Writing", "Callouts: Note, Warning, Tip", M7.sewn, "A Warning in ochre on its patch with a running-stitch edge; the guide’s Tip goes past in chapter 1, and the selection menu offers Note, Warning, Tip and Code");
    c("Writing", "Code blocks", M7.sewn, "A sh block with line numbers, a faint comment and the command; pointing at it brings up Copy, and a click turns it to Copied; the guide’s Rust block goes past in chapter 1");
    c("Writing", "Inline code", M7.code, "snapshot_restore is selected in ¶4 and ⌘E (</> in the selection menu) sets it in the code face, on a chip");
    c("Writing", "Speak instead of typing", M6.speak + 0.15, `⌥ is held, the corner shows Listening… and then the words as they come; letting go puts “${P7}” in the note`);
    c("Writing", "Ink from the pointer", M6.drop, "The switch in Settings › Appearance turns on; on the page, three drops of ink fall where the pointer rests, dry and fade");
    c("Writing", "Your look", M6.dark, "Settings › Appearance: System, Light and Dark (the whole desk turns to Lamplight), then the seven thread colours, lettering (Inkling’s own, like your notes) and text size (small, large, default)");
    c("Writing", "An immersive window", T.title2[0], "No bar: as the title and first paragraphs are typed, the sidebar toggle, Chat and the row above the title fade; caption “No bar. The controls fade while you write.”");
    c("Reading along", "Reads each paragraph at your pause", T.read2a[0], "After each paragraph the needle bobs in the gutter and leaves a tacking stitch; the real read-along let ¶1, ¶3 and ¶4 pass without a word; Settings › Reading shows pace Regular and Slow at " + mmss(M6.pace));
    c("Reading along", "Speaks only when it is worth it", T.t1arrive, "Three of the four paragraphs get only a tacking stitch; ¶2 gets a correction (×) on its arithmetic (4 GiB over 25 Gbit/s is about 1.4 s, not 160 ms); the guide’s boot-order correction unfolds with its sketch at " + mmss(MEAS.sketchAt || c1 + 3));
    c("Reading along", "Waits while you type", T.t1arrive, "The correction arrives while ¶3 is being typed and stays folded on its tab until the writer pauses");
    c("Reading along", "Fix the words, the thread settles", T.e1sel, "160 ms becomes 1.4 s and the plan becomes loading pages on demand; Inkling reads it again, says why that settles it, and the thread ties itself off");
    c("Reading along", "Your first sentence", T.type1[0], "In the new notebook’s guide, the first sentence you write gets a question (real output)");
    c("Reading along", "Reads along, or waits to be asked", M6.mode, "The mode menu in the row above the title switches Reads along to Waits to be asked; Settings › Reading shows the default for new notes");
    c("Reading along", "Tell Inkling what a note is for", T.tell8, "Tell Inkling, above the title: the brief (a design review on Thursday; p99, not the median) is typed and saved; the chat that follows answers with it in mind");
    c("Reading along", "Learns about you", M6.learn, "Settings › About you: the switch turns on and what it learned from a reply appears (real output)");
    c("Threads", "A thread from the words to the margin", T.sew1a, "Running stitch under your sentence, the thread carried to a folded tag; a correction starts with a cross-stitch at " + mmss(T.t1arrive));
    c("Threads", "Tags fold away", T.hov9[0], "Pointing at three folded tabs unfolds each in turn; clicking opens one with its reply box at " + mmss(T.dragOpen9));
    c("Threads", "Move a tag", T.drag9[0], "A tab is dragged down the margin, its thread stretching; it opens toward the side with room");
    c("Threads", "Numbered questions", T.numbers9, `Open questions numbered 1 to ${openAt} in reading order, each with a knot and its number in the gutter`);
    c("Threads", "Reply in the tag", T.replyFocus9, "A reply is written in the tag (what staging must show, under what load) and the mind answers in the same thread (real output)");
    c("Threads", "Tie off", T.tie9, "Resolve: the thread pulls taut, the knot is tied and flies to the border at the top of the page");
    c("Threads", "Snip", T.dis10, "⇧⌘⌫ snips a thread, the stitches come out, “Thread dismissed · Undo ⌥⌘Z” shows at the foot of the page, and ⌥⌘Z brings it back");
    c("Threads", "Reopen, Copy, Make a note", T.reopen11, "A tied-off thread is opened again: Reopen, the copy icon and Make a note are each pointed at; caption names all three");
    c("Threads", "Jump between threads", T.jump9[0], "⌥⌘↓ twice: the camera jumps to the next open thread each time");
    c("Threads", "Hide threads", T.hide12, "⇧⌘. lifts every mark off the page and leaves just the words; again, and they come back");
    c("Threads", "Where threads go", T.right13, "Both margins, then everything moves to the right margin, then marks only (a small mark after each passage); Settings › Reading has the three cards");
    c("Threads", "How many are open", T.numbers9, `“${openAt} loose threads” above the title, counting down as threads are tied off`);
    c("Asking", "Find a loose thread", T.sel3[0], "Words selected, the selection menu, ⌘↵: a question on the staging line (real output)");
    c("Asking", "Ask your own question", T.sel4[0], `Selection menu › Ask Inkling, “${TXT.askQ}” typed in the ask box, the answer streams into a tag in the left margin (real output)`);
    c("Asking", "Sketch it", T.sel7[0], "Words selected › Sketch it: “Stitching a sketch…”, then the path from API call to shell stitched out in five captioned steps, and See it larger (real output)");
    c("Asking", "Ask the room", T.sel5[0], "Staff engineer and Socratic tutor added in the ask box: three threads in three colours, three answers (real output)");
    c("Asking", "Call a mind by name", T.p5[0], "A paragraph starting “Skeptic,” gets the Skeptic’s answer on an indigo thread (real output)");
    c("Asking", "Chat about the whole note", T.chatKey8, "⌘J opens the chat with its three suggested questions; a question, the answer streams (real output); Clear is pointed at");
    c("Minds", "Pick the note’s mind", T.menu16, `The name above the title opens the mind menu; ${MIND.name} is picked`);
    c("Minds", "Built-in minds", T.chips15, "New mind › Start from: Staff engineer, Editor, Skeptic, Socratic tutor, Investor (and Blank); caption lists them with Inkling; the room’s chips show them too");
    c("Minds", "Make your own", T.blank15, `From Blank: name (${MIND.name}), what it is for, how it thinks, voice (Blunt) and how often (Often), Create mind`);
    c("Minds", "Each mind has its thread colour", T.open16, `${MIND.name}’s question arrives on its own thread; the margin shows Inkling’s, the Staff engineer’s, the Socratic tutor’s and the Skeptic’s threads in their colours`);
    c("Notebook", "Notes and folders", M7.side, "⌘\\: the sidebar with folders (one unfolds), dots on notes with open threads, New note, New folder");
    c("Notebook", "Find or start a note", M7.find, `⌘K: the palette filters to “boot” as it is typed, down to ${NOTEBOOK.related.title}`);
    c("Notebook", "Copy a note", M7.copy, "⇧⌘C: “Copied as Markdown”, on a stitched label at the foot of the page");
    c("Notebook", "Export", M7.exp, "⇧⌘E: Markdown, then Word, then HTML; Include the margin off and on");
    c("Notebook", "Import from Obsidian", M7.obs, "File › Import from Obsidian…: a vault’s folders and notes arrive in the sidebar");
    c("Notebook", "Welcome guide", M7.help, "Help › Welcome Guide opens the Start here note");
    c("Model and keys", "Runs on your Claude", M7.claude, "Settings › Model: Claude, signed in through this Mac’s Claude Code login; caption says so");
    c("Model and keys", "Pick the model and effort", M7.model, "Settings › Model: the models Claude Code offers (Default, Opus 5.5, Fable 5.1, Sonnet 5.5, Haiku 5.5, specific versions) and effort for reading along and for asking");
    c("Model and keys", "Every shortcut can be changed", M6.keys, "Settings › Shortcuts: the Hide threads key is clicked, “Press the new keys”, and ⌥⌘H takes its place");
    c("Model and keys", "Everything from the keyboard", M6.ring, "In Settings, Tab and Tab again move a running-stitch ring in your thread down the rail, from Appearance to Minds");
    c("Model and keys", "Messages in passing", T.snip10, "The dismissed thread’s stitched label at the foot of the page, with Undo and ⌥⌘Z; “Copied as Markdown” on the same kind of label at " + mmss(M7.copy));
  }
  const mmss = (t) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`;

  // ======================================================================= boot
  const FILM = { FPS, DUR, BEAT, T, M6, M7, ED, PLAN, CHS, MOMENT, ready: false, error: null, renderAt, events: EVENTS, coverage: COVERAGE, chapters: CH, camAt, real: { note: N } };
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
      await Promise.all(["400 17px 'Fraunces Text'", "italic 400 17px 'Fraunces Text'", "500 17px 'Fraunces Text'", "600 30px 'Fraunces Display'", "500 30px 'Fraunces Display'", "400 14px Figtree", "500 14px Figtree", "600 14px Figtree",
        "400 13.5px 'Spline Sans Mono'", "500 13.5px 'Spline Sans Mono'", "400 17px 'Spline Sans Mono Inline'"].map((f) => document.fonts.load(f)));
      A.measureWordmark();
      buildSketch();
      buildLogo();
      // Settings: where each part of each pane sits, for the camera
      const sh = document.createElement("div");
      sh.style.cssText = "position:absolute;left:0;top:0;width:900px;height:721px";
      mm.appendChild(sh);
      MEAS.set = {};
      const panes = { Appearance: { pane: "Appearance", look: "light", size: "default", lettering: "own", ink: "madder" }, Reading: { pane: "Reading", placement: "both" }, NewMind: { pane: "NewMind" },
        "About you": { pane: "About you" }, Shortcuts: { pane: "Shortcuts", rec: {} }, Model: { pane: "Model", model: 0 } };
      for (const k in panes) MEAS.set[k] = window.SETTINGS.measure(sh, panes[k]);
      sh.remove();
      const ms = MEAS.set;
      SCROLL.ink = Math.max(0, ms.Appearance.dots.y - 330); SCROLL.letter = Math.max(0, ms.Appearance.letter.y - 300); SCROLL.size = Math.max(0, ms.Appearance.size.y - 300);
      SCROLL.drop = Math.max(0, ms.Appearance.inkdrop.y - 380); SCROLL.keys = Math.max(0, ms.Shortcuts.hide.y - 430); SCROLL.model = Math.max(0, ms.Model.models.y - 150);
      // the main note, finished, for the camera's targets
      const P2F = N.p2Fixed;
      const full = { blocks: [["p1", P1], ["p2", P2F], ["p3", P3], ["p4", P4], ["p5", P5], ["p6", P6], ["p7", P7]].map(([id, text]) => ({ id, kind: "p", text, ranges: [] })).concat(SEWN.map((b) => ({ ...b, ranges: [] }))),
        threads: [], header: {}, title: { text: TITLE }, border: {} };
      const rng2 = (bid, phrase, id) => { const b = full.blocks.find((x) => x.id === bid); const i = b.text.indexOf(phrase); if (i < 0) throw new Error(`no «${phrase}» in ${bid}`); b.ranges.push({ a: i, b: i + phrase.length, id }); full.threads.push({ id, block: bid, kind: "question", noTag: true, sew: 0 }); };
      rng2("p2", P2F.slice(P2F.indexOf(TXT.corrQuote.split(" ").slice(0, 3).join(" ")), P2F.indexOf(EDIT1.to) + EDIT1.to.length), "t1"); rng2("p3", TXT.probeQuote, "t2");
      rng2("p1", TXT.askTarget, "t3"); rng2("p2", TXT.roomTarget, "t4"); rng2("p5", P5, "t7"); rng2("p6", TXT.mindQuote, "t8"); rng2("p1", TXT.ideaQuote, "k1"); rng2("p4", "snapshot_restore", "ice");
      measureNote("main", full);
      // the paragraph before it is fixed, for the pointer's selections
      const draft = { blocks: [["p1", P1], ["p2", P2], ["p3", P3]].map(([id, text]) => ({ id, kind: "p", text, ranges: [] })), threads: [], header: {}, title: { text: TITLE }, border: {} };
      [["e1", EDIT1.from], ["e2", EDIT2.from]].forEach(([id, phrase]) => { const i = P2.indexOf(phrase); draft.blocks[1].ranges.push({ a: i, b: i + phrase.length, id }); draft.threads.push({ id, block: "p2", kind: "question", noTag: true, sew: 0 }); });
      measureNote("draft", draft);
      // the foot of the note is below the window; scroll it into view for its beats
      const s6 = MEAS.main.blocks.s6;
      MEAS.sewnScroll = s6 ? Math.max(0, s6.b + 28 + 10 + 60 - 860) : 0;
      // the guide: where "Your turn" ends, so the glide stops with your sentence in view
      const sp = startPage(T.glide0);
      sp.scroll = 0;
      measureNote("startRaw", sp);
      const lastB = sp.blocks[sp.blocks.length - 1];
      const lb = MEAS.startRaw.blocks[lastB.id];
      MEAS.startEnd = Math.max(0, lb.y - 520);
      MEAS.start = { lastY: lb.y - MEAS.startEnd + 14 };
      const drawnB = sp.blocks.find((b) => START.threads.some((th) => th.drawing && b.text.includes(th.quote)));
      if (drawnB) {
        const my = MEAS.startRaw.blocks[drawnB.id].y;
        for (let tt = T.glide0; tt < T.glide1; tt += 0.02) if (my - startScroll(tt) < 300) { MEAS.sketchAt = tt; break; }
      }
      buildCamera();
      buildEvents();
      buildCoverage();
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

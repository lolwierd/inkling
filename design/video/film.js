/* Inkling: the walkthrough film, re-cut on 6 Nov 2026.
 *
 * Every frame is renderAt(t), a pure function of time. The rope is the one simulation, and
 * it is stepped at a fixed rate from the start of each motion, so frame 4321 comes out the
 * same whichever worker renders it.
 *
 * The film is a desk of linen with the app's windows on it and a virtual camera moving over
 * them: pushing in to the words as a thread is pulled, tracking the thread to its tag,
 * pulling back to the sheet. Captions are stitched in and tied off; chapters hang in on
 * woven labels whose knots fly down to the sampler border at the foot of the frame, which
 * is the film's progress bar. Inkling's words are the brain's real output (data/real.js).
 * FILM.events is the cue list audio.js scores from; FILM.coverage maps design/capabilities.md.
 */
(function () {
  "use strict";
  const A = window.APP, { fx, clamp, lerp, prog, esc, E, spring, knotSVG, knotScale, motifSVG, capsule, runningStitch } = A;
  const REAL = window.REAL, C = REAL.calls, START = window.START;
  const FPS = 30, BEAT = 0.625;
  const $ = (s, r) => (r || document).querySelector(s);
  const within = (t, a, b) => t >= a && t < b;
  const rng = A.rng;

  // ======================================================================= words (all real)
  const P1 = "Last year I ran the city 10k in 56:40. I went out too fast and walked twice after the 7 km mark. This year I want to finish under 50 minutes.";
  const P2 = "The plan is to run four days a week and hold 5:30 per km on every run, so my legs learn race pace. If I hold 5:30 for the whole race, I’ll come in just under 50.";
  const P3 = "Sunday is the long run. I’ll add a kilometre every week until I reach 16.";
  const P4 = "Strength work twice a week, mostly calves and hips, because that’s where it fell apart last year.";
  const P5 = "Skeptic, is going from 56:40 to 49:10 in one season realistic?";
  const P6 = REAL.v2.mind.line;
  const P7 = "Race morning I’ll eat by six.";
  const FIRST = REAL.v2.first.sentence;
  const TXT = {
    first: C.v2_first.output.text, correction: C.read_p2.output.text, settle: C.settle.output.text, probe: C.probe_p3.output.text,
    ask: C.v2_ask.output.text, askQ: REAL.v2.ask.question, askTarget: REAL.v2.ask.target,
    room: { inkling: C.v2_room_inkling.output.text, skeptic: C.v2_room_skeptic.output.text, tutor: C.v2_room_tutor.output.text },
    roomQ: "Is four days a week enough? One sentence.", call: C.v2_call.output.text, chat: C.v2_chat.output.text, chatQ: "What’s the weakest part of this plan now?",
    reply: "From 8 km. Every fourth week is an easy one, and I taper the last two.", answer: C.v2_reply.output.text,
    coach: C.v2_mind.output.text, brief: "I want a strong last 3 km this time, not just a fast first 7.",
    idea: C.v2_sketch.output.text, svg: C.v2_draw.output.svg, learned: C.v2_learn.output.aboutMe, learnMsg: REAL.v2.learn.message,
    bread: REAL.v2.bread,
  };
  const COACH = REAL.v2.mind;

  // ======================================================================= the timeline
  // 96 bpm: a beat is 0.625 s. Knots and cuts land on beats; the montages run on them.
  const c1 = 8.75, c2 = 29.75, c3 = 61.25, c4 = 111.875, c5 = 142.5, c6 = 159.375, c7 = 179.375, cEnd = 197.5;
  const DUR = cEnd + 10.5;
  const T = {
    // cold open
    pierce: 0.9, sew0: 1.0, sew1: 3.2, tail: 3.25, openTie: 5.625, lineUp: 5.9, pull0: 6.2, pull1: 8.9,
    // ch1: the guide, then your first sentence
    glide0: c1 + 0.2, glide1: c1 + 5.6, type1: [c1 + 6.1, c1 + 9.4], read1: [c1 + 9.9, c1 + 11.0], sew1a: c1 + 11.0, sew1b: c1 + 12.1, carry1: c1 + 12.15,
    hover1: c1 + 13.2, wipe1: [c2 - 1.0, c2 + 0.35],
    // ch2: reading along
    title2: [c2 + 0.45, c2 + 1.85], p1: [c2 + 2.15, c2 + 6.65], read2a: [c2 + 6.85, c2 + 7.65], p2: [c2 + 7.85, c2 + 11.65], read2b: [c2 + 11.75, c2 + 12.75],
    p3: [c2 + 11.95, c2 + 14.15], t1arrive: c2 + 12.75, pause2: c2 + 14.15, t1hover: c2 + 16.25, t1fold: c2 + 19.85,
    e1sel: c2 + 20.25, e1type: [c2 + 20.55, c2 + 21.15], e2drag: [c2 + 21.35, c2 + 21.85], e2type: [c2 + 22.05, c2 + 23.45],
    reread: [c2 + 23.65, c2 + 24.65], settleOpen: c2 + 24.65, settle: [c2 + 24.85, c2 + 26.45], breath2: c2 + 26.9, tie2: c2 + 27.75, knot2: c2 + 28.375,
    fly2: [c2 + 28.45, c2 + 29.25], fade2: [c2 + 29.95, c2 + 30.4], p4: [c2 + 29.4, c2 + 30.7], read2d: [c2 + 30.8, c2 + 31.3],
    // ch3: asking
    sel3: [c3 + 0.75, c3 + 1.35], bar3: c3 + 1.45, find3: c3 + 1.75, sew3: [c3 + 2.05, c3 + 2.85], carry3: c3 + 2.85, open3: c3 + 3.55, fold3: c3 + 6.35,
    sel4: [c3 + 6.65, c3 + 7.15], bar4: c3 + 7.35, askClick4: c3 + 7.85, type4: [c3 + 8.15, c3 + 9.35], send4: c3 + 9.45, sew4: [c3 + 9.55, c3 + 10.15], carry4: c3 + 10.15, stream4: [c3 + 10.85, c3 + 13.35], fold4: c3 + 13.55,
    sel5: [c3 + 13.75, c3 + 14.25], bar5: c3 + 14.45, askClick5: c3 + 14.9, chip5a: c3 + 15.25, chip5b: c3 + 15.65, type5: [c3 + 15.95, c3 + 17.25], send5: c3 + 17.35,
    sew5: [c3 + 17.45, c3 + 18.05], carry5: c3 + 18.05, open5: c3 + 18.75, stream5: { inkling: [c3 + 18.85, c3 + 20.75], skeptic: [c3 + 19.05, c3 + 21.65], tutor: [c3 + 19.25, c3 + 22.15] },
    hov5b: c3 + 20.45, hov5c: c3 + 22.05, fold5: c3 + 23.75,
    caret6: c3 + 23.95, p5: [c3 + 24.35, c3 + 26.15], read6: [c3 + 26.45, c3 + 27.05], sew6: [c3 + 27.05, c3 + 27.65], carry6: c3 + 27.65, stream6: [c3 + 28.35, c3 + 30.55],
    wipe7: [c3 + 30.95, c3 + 31.75], sel7: c3 + 32.15, bar7: c3 + 32.45, sketch7: c3 + 32.95, sew7: [c3 + 33.05, c3 + 33.55], carry7: c3 + 33.55, think7: [c3 + 34.05, c3 + 34.75],
    idea7: c3 + 34.75, steps7: [c3 + 35.05, c3 + 36.4, c3 + 37.75], larger7: c3 + 39.15, big7: [c3 + 39.25, c3 + 41.35], wipe8: [c3 + 41.35, c3 + 42.15],
    tell8: c3 + 42.45, brief8: [c3 + 42.75, c3 + 44.15], save8: c3 + 44.35, chatKey8: c3 + 44.75, chatIn8: [c3 + 44.85, c3 + 45.25], pills8: c3 + 45.35, chatType8: [c3 + 45.95, c3 + 47.15],
    chatSend8: c3 + 47.25, chatStream8: [c3 + 47.65, c3 + 50.15], clear8: c3 + 50.05,
    // ch4: threads
    chatOut9: [c4 + 0.05, c4 + 0.45], hov9: [c4 + 3.3, c4 + 4.1, c4 + 4.2, c4 + 5.0, c4 + 5.1, c4 + 5.8], jump9: [c4 + 6.0, c4 + 7.0],
    drag9: [c4 + 8.2, c4 + 9.2], dragOpen9: c4 + 9.55, replyFocus9: c4 + 10.1, reply9: [c4 + 10.25, c4 + 12.15], send9: c4 + 12.25, answer9: [c4 + 12.85, c4 + 14.55],
    resolveHov9: c4 + 15.05, breath9: c4 + 15.6, tie9: c4 + 16.25, knot9: c4 + 16.875, fly9: [c4 + 16.95, c4 + 17.75], fade9: [c4 + 18.55, c4 + 19.0],
    open10: c4 + 19.15, dis10: c4 + 19.75, snip10: c4 + 19.875, undo10: c4 + 21.25,
    reopen11: c4 + 23.0, hide12: c4 + 25.6, show12: c4 + 27.0, right13: c4 + 27.9, marks13: c4 + 28.9, both13: c4 + 29.9, snip14: c5 - 0.12,
    // ch5: minds
    chips15: c5 + 0.7, blank15: c5 + 2.3, name15: [c5 + 2.5, c5 + 2.95], desc15: [c5 + 3.05, c5 + 4.05], how15: [c5 + 4.15, c5 + 5.75], voice15: c5 + 5.95, often15: c5 + 6.35, create15: c5 + 6.85,
    pan16: [c5 + 7.1, c5 + 8.1], menu16: c5 + 8.3, pick16: c5 + 9.1, p6: [c5 + 10.15, c5 + 11.75], read16: [c5 + 11.95, c5 + 12.55], sew16: [c5 + 12.55, c5 + 13.05], carry16: c5 + 13.05, open16: c5 + 13.55,
    wipe17: [c6 - 0.5, c6 + 0.55],
  };
  // the ending: the Coach's thread is tied off, the border grows into the mark (Darn · Border)
  const ED = { act: cEnd + 0.3, hov: cEnd + 0.8, breath: cEnd + 0.95, tie: cEnd + 1.25, knot: cEnd + 1.875, fly: [cEnd + 1.95, cEnd + 2.75], weave: [cEnd + 2.9, cEnd + 5.3],
    morph: [cEnd + 5.3, cEnd + 6.6], word: [cEnd + 6.5, cEnd + 7.8], dot: [cEnd + 8.125, cEnd + 8.4375], line: [cEnd + 8.6, cEnd + 9.4] };
  // montage beats (start times, on the grid)
  const M6 = { dark: c6 + 0.625, ink: c6 + 3.125, letter: c6 + 5.625, size: c6 + 6.875, mode: c6 + 8.125, pace: c6 + 9.375, drop: c6 + 10.625, speak: c6 + 13.125, learn: c6 + 15.625, keys: c6 + 17.5, end: c7 };
  const M7 = { side: c7 + 0.625, find: c7 + 3.125, copy: c7 + 5.0, exp: c7 + 6.25, obs: c7 + 8.75, help: c7 + 11.25, model: c7 + 12.5, claude: c7 + 15.0, blocks: c7 + 16.25, end: cEnd };

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
  const EDIT1 = { from: "5:30", to: "4:55" }, EDIT2 = { from: " and hold 5:30 per km on every run", to: ", two of them at 4:55 per km" };
  const S = {
    first: sched(FIRST, ...T.type1, 3), title: sched("Sub-50 10k by March", ...T.title2, 11), p1: sched(P1, ...T.p1, 12), p2: sched(P2, ...T.p2, 13), p3: sched(P3, ...T.p3, 14),
    e1: sched(EDIT1.to, ...T.e1type, 15), e2: sched(EDIT2.to, ...T.e2type, 16), p4: sched(P4, ...T.p4, 17), q4: sched(TXT.askQ, ...T.type4, 18), q5: sched(TXT.roomQ, ...T.type5, 19),
    p5: sched(P5, ...T.p5, 20), brief: sched(TXT.brief, ...T.brief8, 21), chat: sched(TXT.chatQ, ...T.chatType8, 22), reply: sched(TXT.reply, ...T.reply9, 23),
    name: sched("Coach", ...T.name15, 24), desc: sched(COACH.description, ...T.desc15, 25), how: sched(COACH.howItThinks, ...T.how15, 26), p6: sched(P6, ...T.p6, 27),
  };
  const ALLKEYS = [S.first, S.title, S.p1, S.p2, S.p3, S.e1, S.e2, S.p4, S.p5, S.p6];
  const typingAt = (t) => ALLKEYS.some((k) => t - lastKey(k, t) < 0.55 && t >= k[0] - 0.05);
  /** Streamed words: the brain's own chunks when it sent them, else word by word. */
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
  const stateAt = (t, keys, d) => { let s = d; for (const [tk, v] of keys) if (tk <= t) s = v; return s; };

  // ======================================================================= measurements
  // Laid out once at load (scale 1) so the camera knows where the words will be.
  const MEAS = {};
  function measureNote(name, page) {
    const host = $("#measure .page");
    const geo = A.renderPage(host, page, 0, 1);
    const out = { blocks: {}, anchors: {} };
    for (const b of page.blocks) { const bb = geo.blockBox(b.id); if (bb) out.blocks[b.id] = bb; }
    for (const th of page.threads || []) { const L = geo.lineRects(th.block, th.id); if (L.length) out.anchors[th.id] = L; }
    MEAS[name] = out;
  }

  // ======================================================================= the main note
  const MAIN_MINDS = { inkling: "madder", skeptic: "indigo", tutor: "ochre" };
  /** Paragraph 2 over time: typed, then fixed (5:30 → 4:55 twice). */
  function p2At(t) {
    let text = P2.slice(0, count(S.p2, t)), sel = null, caret = text.length;
    if (t < T.e1sel) return { text, sel, caret };
    const i1 = P2.indexOf(EDIT1.from, P2.indexOf("If I hold"));
    if (t < T.e1type[0]) return { text, sel: [i1, i1 + 4], caret: null };
    const n1 = count(S.e1, t);
    text = P2.slice(0, i1) + EDIT1.to.slice(0, n1) + P2.slice(i1 + 4);
    caret = i1 + n1;
    if (t < T.e2drag[0]) return { text, sel, caret };
    const i2 = text.indexOf(EDIT2.from);
    if (t < T.e2type[0]) {
      const k = E.sine(prog(t, T.e2drag[0], T.e2drag[1]));
      const end = Math.round(i2 + EDIT2.from.length * k);
      return { text, sel: end > i2 ? [i2, end] : null, caret: end > i2 ? null : i2 };
    }
    const n2 = count(S.e2, t);
    return { text: text.slice(0, i2) + EDIT2.to.slice(0, n2) + text.slice(i2 + EDIT2.from.length), sel: null, caret: i2 + n2 };
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
  const mindOf = (t) => (t >= T.pick16 + 0.1 ? "Coach" : "Inkling");
  /** A mind's thread colour at t (note_page.rs `mind_thread`). */
  const colourOf = (mind, t) => A.mindThread(mind, mindOf(t), looks(t).ink);

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
      if (within(t, T.caret6, T.wipe7[0])) return "p5";
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
    selPhrase("p3", p3, "I’ll add a kilometre every week until I reach 16.", ...T.sel3, T.sew3[0]);
    selPhrase("p1", p1, TXT.askTarget, ...T.sel4, T.send4);
    selPhrase("p2", p2.text, "four days a week", ...T.sel5, T.send5);
    if (p2.sel) sel.p2 = p2.sel;
    const gut = {};
    const add = (id, kind, text, caret) => blocks.push({ id, kind, text, caret: caretIn === id ? caret : null, sel: sel[id], ranges: [], gutter: gut[id] });
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
    if (t >= T.p4[0]) add("p4", "p", p4, p4.length);
    if (t >= T.p5[0]) { const s = P5.slice(0, count(S.p5, t)); add("p5", "p", s, s.length); }
    if (t >= T.p6[0]) { const s = P6.slice(0, count(S.p6, t)); add("p6", "p", s, s.length); }
    const spoken = within(t, M6.speak + 1.95, cEnd + 5);
    if (spoken) add("p7", "p", P7, P7.length);
    // the empty paragraph you write into
    if (t >= T.p1[0] && t < T.p6[0] && !(t >= T.p4[0] && t < T.caret6)) { /* typing in place */ }
    if (t < T.p1[0] && t >= c2) blocks.push({ id: "p0", kind: "p", text: "", ph: "Just start writing.", caret: null });

    // ---- threads
    const threads = [];
    const placement = t >= T.right13 && t < T.marks13 ? "right" : t >= T.marks13 && t < T.both13 ? "marks" : "both";
    const sideOf = (s) => (placement === "right" ? "right" : s);
    const thread = (o) => { threads.push(o); return o; };
    // t1: the correction on the race sentence (read along), fixed and tied off
    if (t >= T.t1arrive) {
      const i = p2.text.indexOf("If I hold");
      if (i >= 0) {
        const b = blocks.find((x) => x.id === "p2");
        b.ranges.push({ a: i, b: p2.text.length, id: "t1", cls: "anc" });
        const tied = t >= T.knot2;
        const u = unfoldAt(t, [[T.t1hover, 1], [T.t1fold, 0], [T.settleOpen, 1], [T.fade2[0] + 0.3, 0]]);
        let body = A.B.msg(TXT.correction);
        if (t >= T.reread[0] && t < T.settle[0]) body += A.B.think("Reading your change", t);
        if (t >= T.settle[0]) body += A.B.msg(streamed(TXT.settle, T.settle[0], T.settle[1], t));
        const gone = t >= T.fade2[1];
        thread({ id: "t1", block: "p2", kind: "correction", c: colourOf("Inkling", t), side: sideOf("right"), sew: 1, tied, tag: !gone, rope: !gone,
          stitchOp: tied ? 0.42 : 1, ropeOp: tied ? 0.8 * (1 - prog(t, ...T.fade2)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t >= T.tie2 ? { t0: T.tie2, tieAt: T.tie2, speed: 0.5 } : null, knotAt: t >= T.knot2 ? T.knot2 : null,
          tagOp: 1 - prog(t, ...T.fade2), wash: u > 0.4 && !tied,
          tagState: { unfold: u, mind: "Inkling", kindLabel: tied ? "tied off" : "correction", body } });
      }
    }
    // t2: Find a loose thread on the long run; answered and resolved in ch4; reopened after
    const t2on = t >= T.sew3[0];
    if (t2on) {
      const i = p3.indexOf("I’ll add");
      const b = blocks.find((x) => x.id === "p3");
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: p3.length, id: "t2", cls: "anc" });
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
        thread({ id: "t2", block: "p3", kind: "question", c: colourOf("Inkling", t), side: sideOf("right"), offset: off,
          sew: prog(t, ...T.sew3), needle: t < T.sew3[1], tied: tied,
          stitchOp: tied ? 0.42 : 1, ropeOp: t < T.carry3 ? 0 : tied && !reopened ? 0.8 * (1 - prog(t, ...T.fade9)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry3 + 1.4 ? { t0: T.carry3, carry: 0.5 } : within(t, T.drag9[0], T.tie9) ? { t0: T.drag9[0], offsetFn: (x) => { const k = E.out(prog(x, ...T.drag9)); return [lerp(0, 92, k), lerp(0, 132, k)]; } } : within(t, T.tie9, T.fade9[1]) ? { t0: T.tie9, tieAt: T.tie9, speed: 0.5 } : null,
          knotAt: within(t, T.knot9, T.fade9[1]) ? T.knot9 : null, rope: !(tied && t >= T.fade9[1] && !within(t, T.reopen11, T.hide12)),
          tag: t >= T.carry3 + 0.45 && (!tied || t < T.fade9[1] || within(t, T.reopen11, T.hide12)), tagOp: tied && t < T.reopen11 ? 1 - prog(t, ...T.fade9) : 1, wash: u > 0.4,
          tagState: { unfold: u, mind: "Inkling", kindLabel: tied ? "tied off" : "question", body, openRight: t >= T.drag9[0] ? false : undefined } });
      }
    }
    // t3: Ask Inkling, your own question (left margin); snipped and brought back in ch4
    if (t >= T.sew4[0]) {
      const i = p1.indexOf(TXT.askTarget);
      const b = blocks.find((x) => x.id === "p1");
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + TXT.askTarget.length, id: "t3", cls: "anc" });
        const snipped = within(t, T.snip10, T.undo10 + 0.1);
        const u = unfoldAt(t, [[T.carry4 + 0.6, 1], [T.fold4, 0], [T.jump9[0] + 0.1, 1], [T.jump9[1], 0], [T.open10, 1], [T.snip10, 0]]);
        let body = A.B.you(TXT.askQ) + A.B.msg(streamed(TXT.ask, ...T.stream4, t));
        if (t < T.stream4[0]) body = A.B.you(TXT.askQ) + A.B.think("", t);
        const back = prog(t, T.undo10, T.undo10 + 0.45);
        thread({ id: "t3", block: "p1", kind: "ask", c: colourOf("Inkling", t), side: sideOf("left"),
          sew: snipped ? 1 : t < T.undo10 ? prog(t, ...T.sew4) : back, needle: t < T.sew4[1], unpick: snipped ? prog(t, T.snip10 + 0.15, T.snip10 + 0.95) : 0,
          ropeOp: t < T.carry4 ? 0 : snipped ? 0.7 * (1 - prog(t, T.snip10 + 0.5, T.snip10 + 0.95)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry4 + 1.4 ? { t0: T.carry4, carry: 0.5 } : snipped ? { t0: T.snip10, cutAt: T.snip10 } : within(t, T.undo10, T.undo10 + 1.2) ? { t0: T.undo10, carry: 0.45 } : null,
          tag: t >= T.carry4 + 0.45, tagOp: snipped ? 1 - prog(t, T.snip10, T.snip10 + 0.4) : t >= T.undo10 ? prog(t, T.undo10 + 0.3, T.undo10 + 0.6) : 1, wash: u > 0.4,
          tagState: { unfold: u, mind: "Inkling", kindLabel: "you asked", body: body + (within(t, T.open10, T.snip10) ? A.B.reply("", false) + A.B.acts(false, within(t, T.dis10 - 0.3, T.snip10) ? "dismiss" : null) : "") } });
      }
    }
    // t4-t6: the room on "four days a week"
    if (t >= T.sew5[0]) {
      const i = p2.text.indexOf("four days a week");
      const b = blocks.find((x) => x.id === "p2");
      if (b && i >= 0) {
        b.ranges.push({ a: i, b: i + 16, id: "t4 t5 t6", cls: "anc" });
        [["t4", "Inkling", "inkling", [[T.open5, 1], [T.hov5b - 0.15, 0], [T.hov9[0], 1], [T.hov9[1], 0], [T.jump9[1] + 0.1, 1], [T.jump9[1] + 0.9, 0]]],
          ["t5", "Skeptic", "skeptic", [[T.hov5b, 1], [T.hov5c - 0.15, 0], [T.hov9[2], 1], [T.hov9[3], 0]]],
          ["t6", "Socratic tutor", "tutor", [[T.hov5c, 1], [T.fold5, 0], [T.hov9[4], 1], [T.hov9[5], 0]]]].forEach(([id, mind, key, keys], k) => {
          const u = unfoldAt(t, keys);
          const st = T.stream5[key];
          let body = A.B.you(TXT.roomQ) + (t < st[0] ? A.B.think("", t) : A.B.msg(streamed(TXT.room[key], st[0], st[1], t)));
          thread({ id, block: "p2", kind: "ask", c: colourOf(mind, t), side: sideOf("right"), sew: prog(t, T.sew5[0] + k * 0.06, T.sew5[1] + k * 0.06), needle: k === 0 && t < T.sew5[1],
            ropeOp: t < T.carry5 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.26, ropeTrack: t < T.carry5 + 1.4 ? { t0: T.carry5 + k * 0.08, carry: 0.5 } : null,
            tag: t >= T.carry5 + 0.45 + k * 0.08, wash: u > 0.4, tagState: { unfold: u, mind, kindLabel: "you asked", body, z: u > 0.01 ? 10 + k : null } });
        });
      }
    }
    // t7: Skeptic, called by name (left margin)
    if (t >= T.sew6[0]) {
      const b = blocks.find((x) => x.id === "p5");
      if (b) {
        b.ranges.push({ a: 0, b: P5.length, id: "t7", cls: "anc" });
        const u = unfoldAt(t, [[T.carry6 + 0.6, 1], [T.wipe7[0], 0]]);
        thread({ id: "t7", block: "p5", kind: "answer", c: colourOf("Skeptic", t), side: sideOf("left"), sew: prog(t, ...T.sew6), needle: t < T.sew6[1],
          ropeOp: t < T.carry6 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.26, ropeTrack: t < T.carry6 + 1.4 ? { t0: T.carry6, carry: 0.5 } : null,
          tag: t >= T.carry6 + 0.45, wash: u > 0.4,
          tagState: { unfold: u, mind: "Skeptic", kindLabel: "answer", body: t < T.stream6[0] ? A.B.think("", t) : A.B.msg(streamed(TXT.call, ...T.stream6, t)) } });
      }
    }
    // t8: the Coach, the note's own mind now, reading the new paragraph
    if (t >= T.sew16[0]) {
      const b = blocks.find((x) => x.id === "p6");
      if (b) {
        const i = P6.indexOf(C.v2_mind.output.quote) >= 0 ? P6.indexOf(C.v2_mind.output.quote) : 0;
        b.ranges.push({ a: i, b: P6.length, id: "t8", cls: "anc" });
        const tied = t >= ED.knot;
        const u = unfoldAt(t, [[T.open16, 1], [ED.knot + 1.0, 0]]);
        let body = A.B.msg(TXT.coach);
        if (within(t, ED.act, ED.knot + 1.0)) body += A.B.reply("", false) + A.B.acts(tied, within(t, ED.hov, ED.tie + 0.2) ? "resolve" : null);
        thread({ id: "t8", block: "p6", kind: "question", c: colourOf("Coach", t), side: sideOf("right"), sew: prog(t, ...T.sew16), needle: t < T.sew16[1], tied, stitchOp: tied ? 0.42 : 1,
          ropeOp: t < T.carry16 ? 0 : tied ? 0.85 * (1 - prog(t, ED.knot + 1.0, ED.knot + 1.4)) : u > 0.05 ? lerp(0.26, 1, u) : 0.26,
          ropeTrack: t < T.carry16 + 1.4 ? { t0: T.carry16, carry: 0.5 } : t >= ED.tie ? { t0: ED.tie, tieAt: ED.tie, speed: 0.5 } : null, knotAt: t >= ED.knot ? ED.knot : null,
          tag: t >= T.carry16 + 0.45, tagOp: tied ? 1 - prog(t, ED.knot + 1.0, ED.knot + 1.4) : 1, wash: u > 0.4 && !tied,
          tagState: { unfold: u, mind: "Coach", kindLabel: tied ? "tied off" : "question", body, nolift: true } });
      }
    }
    // open questions are numbered in reading order, and knotted in the gutter
    const order = ["p1", "p2", "p3", "p4", "p5", "p6"];
    const open = threads.filter((th) => !th.tied && th.tag !== false && !(th.id === "t3" && within(t, T.snip10, T.undo10)));
    open.sort((a, b) => order.indexOf(a.block) - order.indexOf(b.block) || a.id.localeCompare(b.id));
    open.forEach((th, i) => (th.num = i + 1));
    for (const id of order) {
      const first = open.find((th) => th.block === id);
      const bl = blocks.find((x) => x.id === id);
      if (first && bl && !(bl.gutter && bl.gutter.type === "needle")) bl.gutter = { type: "knot", n: first.num, c: first.c, k: knotScale(t - (first.arrived || 0) + 1) };
    }
    // marks only: no tags, a small mark after the passage
    const marksK = placement === "marks" ? prog(t, T.marks13, T.marks13 + 0.4) : 0;
    // the border: one motif per thread tied off in this note
    const motifs = [];
    if (t >= T.fly2[1]) motifs.push({ c: colourOf("Inkling", t), k: prog(t, T.fly2[1], T.fly2[1] + 0.4) });
    if (t >= T.fly9[1]) motifs.push({ c: colourOf("Inkling", t), k: prog(t, T.fly9[1], T.fly9[1] + 0.4) });
    if (t >= ED.fly[1]) motifs.push({ c: colourOf("Coach", t), k: prog(t, ED.fly[1], ED.fly[1] + 0.4) });
    const loose = open.length;
    // header and chrome fade while you type (app.rs chrome_layer: out slowly, back quickly)
    const calm = calmAt(t);
    const header = { op: 1 - calm, mind: mindOf(t), mindC: `var(--voice)`, mode: within(t, M6.mode + 0.35, M6.pace) ? "Waits to be asked" : "Reads along", loose,
      hov: within(t, T.tell8 - 0.3, T.tell8 + 0.15) ? "tell" : within(t, T.menu16 - 0.25, T.pick16) ? "mind" : within(t, T.hide12 - 0.3, T.hide12 + 0.1) || within(t, T.show12 - 0.3, T.show12 + 0.1) ? "hide" : null };
    const page = { scroll: 0, header, blocks, threads, hidden, hideK, marksOnly: placement === "marks" && marksK > 0.5,
      border: { motifs }, title: { text: "Sub-50 10k by March".slice(0, count(S.title, t)), caret: caretIn === "title" ? count(S.title, t) : null },
      chatOpen: within(t, T.chatIn8[0] + 0.1, T.chatOut9[1]), caretOn: caretOn(t) };
    if (t >= T.save8) page.brief = TXT.brief;
    if (within(t, T.tell8 + 0.1, T.save8)) page.briefIn = { text: TXT.brief.slice(0, count(S.brief, t)), caret: caretOn(t) };
    // menus anchored to the words
    page.menus = [];
    const bar = (block, anchor, t0, t1, hov) => { if (within(t, t0, t1)) page.menus.push({ type: "toolbar", block, anchor, op: spring(t - t0, 400, 32), hov }); };
    bar("p3", "t2", T.bar3, T.sew3[0] - 0.05, t >= T.find3 - 0.25 ? "find" : null);
    bar("p1", "t3", T.bar4, T.askClick4, t >= T.askClick4 - 0.35 ? "ask" : null);
    bar("p2", "t4", T.bar5, T.askClick5, t >= T.askClick5 - 0.35 ? "ask" : null);
    if (within(t, T.askClick4, T.send4)) page.menus.push({ type: "ask", block: "p1", anchor: "t3", op: spring(t - T.askClick4, 400, 32), text: TXT.askQ.slice(0, count(S.q4, t)), caret: caretOn(t), chips: chipsAt(t, []) });
    if (within(t, T.askClick5, T.send5)) page.menus.push({ type: "ask", block: "p2", anchor: "t4", op: spring(t - T.askClick5, 400, 32), text: TXT.roomQ.slice(0, count(S.q5, t)), caret: caretOn(t), chips: chipsAt(t, [t >= T.chip5a ? "Skeptic" : null, t >= T.chip5b ? "Socratic tutor" : null]) });
    if (within(t, T.menu16, T.pick16 + 0.15)) page.menus.push({ type: "mind", op: spring(t - T.menu16, 400, 32), hov: t >= T.menu16 + 0.5 ? 1 : null, minds: [{ name: "Inkling", d: "Thinks alongside you, and speaks up when something is worth a second look.", on: true }, { name: "Coach", d: COACH.description }] });
    if (within(t, M6.mode, M6.mode + 0.95)) page.menus.push({ type: "mode", op: spring(t - M6.mode, 400, 32), cur: t >= M6.mode + 0.35 ? "ask" : "read", hov: t >= M6.mode + 0.2 ? "ask" : null });
    // the block-kind beat in ch7
    if (within(t, M7.blocks, M7.end)) {
      const kinds = ["p", "h1", "h2", "h3", "q", "li", "ol", "p"];
      const k = clamp(Math.floor((t - M7.blocks - 0.3) / (BEAT / 2)), 0, 7);
      const b = blocks.find((x) => x.id === "p4");
      if (b) { b.kind = t < M7.blocks + 0.3 ? "p" : kinds[k]; b.sel = [0, P4.length]; page.menus.push({ type: "toolbar", block: "p4", anchor: "blk", op: 1, kind: [0, 1, 2, 3, 4, 5, 6, 0][k] }); b.ranges.push({ a: 0, b: P4.length, id: "blk", cls: "" }); }
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
      const out = 1 - Math.exp(-5 * Math.max(0, Math.min(t, b) - a));
      v = t <= b ? out : out * Math.exp(-14 * (t - b));
      if (t <= b + 0.5) break;
    }
    return clamp(v);
  }

  // ======================================================================= the Start here guide
  function startPage(t) {
    const blocks = START.blocks.slice(1).map((b, i) => ({ id: "s" + (i + 1), kind: { paragraph: "p", h2: "h2", bullet: "li" }[b.kind] || "p", text: b.text, ranges: [] }));
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
      const body = th.messages.map((m) => (m.role === "you" ? A.B.you(m.text) : A.B.msg(m.text))).join("");
      let unfold = 0, extra = "";
      if (th.drawing && MEAS.moonAt) {
        unfold = unfoldAt(t, [[MEAS.moonAt - 0.5, 1], [MEAS.moonAt + 1.6, 0]]);
        extra = moonSketch(t, MEAS.moonAt - 0.3);
      }
      threads.push({ id, block: b.id, kind: th.kind === "ask" ? "ask" : th.kind, c: A.mindThread(mind, "Inkling", "madder"), side: side[th.side] || "right", ropeOp: unfold > 0.05 ? lerp(0.26, 1, unfold) : 0.26,
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
    const open = threads.slice().sort((a, b) => order.indexOf(a.block) - order.indexOf(b.block) || a.id.localeCompare(b.id));
    open.forEach((th, i) => (th.num = i + 1));
    for (const b of blocks) { const f = open.find((th) => th.block === b.id); if (f) b.gutter = { type: "knot", n: f.num, c: f.c }; }
    if (within(t, ...T.read1)) last.gutter = { type: "needle" };
    blocks[0].first = true;
    const calm = within(t, T.type1[0], T.read1[1]) ? clamp((t - T.type1[0]) * 2) * (1 - clamp((t - T.read1[1]) * 6)) : 0;
    return { blocks, threads, header: { op: 1 - calm, loose: open.length }, title: { text: START.title }, border: {}, scroll: startScroll(t), caretOn: caretOn(t) };
  }
  /** The guide's Moon sketch (welcome.rs), drawn on as its tag unfolds. */
  let MOON = null;
  function moonSketch(t, t0) {
    if (!MOON) { const keep = SK; buildSketch(START.threads.find((x) => x.drawing).drawing); MOON = SK; SK = keep; }
    const keep = SK; SK = MOON;
    const sk = sketchSVG(t, [t0, t0 + 0.35, t0 + 0.7], 0.62);
    SK = keep;
    return `<div class="sk">${sk.svg}</div><div class="capt"><span class="gr">${esc(sk.caption)}</span><span class="ctl">${sk.step}/${sk.of}</span></div>`;
  }
  function startScroll(t) {
    const end = MEAS.startEnd || 1400;
    return end * E.io(prog(t, T.glide0, T.glide1));
  }

  // ======================================================================= the bread note (Sketch it)
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
    let out = "", cur = 0;
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
          out += `<text x="${it.x}" y="${it.y}" text-anchor="${it.anchor}" style="font-family:Figtree;font-size:${it.size}px;font-weight:${it.cls === "voice" ? 500 : 400};fill:${col}" opacity="${fx(clamp(kk * 2))}">${esc(it.text)}</text>`;
          return;
        }
        const attrs = it.attrs.filter(([n]) => !["stroke", "fill", "stroke-width", "class"].includes(n)).map(([n, v]) => `${n}="${esc(v)}"`).join(" ");
        const stroke = TONE[it.stroke] || "none", fill = TONE[it.fill] || "none";
        // the mind's own lines are stitched; thin guides stay drawn (drawing.rs)
        const stitched = it.stroke === "ink" && it.sw >= 1.2;
        const L = Math.max(1, it.L);
        const dash = stitched ? `stroke-dasharray:${fx(it.sw * 3.4)} ${fx(it.sw * 2.1)};` : "";
        const reveal = stitched ? `mask="url(#rv${cur}_${j})"` : "";
        if (fill !== "none") out += `<${it.tag} ${attrs} style="fill:${fill};stroke:none" opacity="${fx(kk)}"/>`;
        if (stroke !== "none") {
          if (stitched) out += `<mask id="rv${cur}_${j}"><${it.tag} ${attrs} style="fill:none;stroke:#fff;stroke-width:${it.sw + 3};stroke-dasharray:${fx(L)} ${fx(L)};stroke-dashoffset:${fx(L * (1 - kk))}"/></mask>`;
          out += `<${it.tag} ${attrs} ${reveal} style="fill:none;stroke:${stroke};stroke-width:${it.sw * (scale || 1)};stroke-linecap:round;${stitched ? dash : `stroke-dasharray:${fx(L)} ${fx(L)};stroke-dashoffset:${fx(L * (1 - kk))};`}"/>`;
        }
      });
    });
    return { svg: `<svg viewBox="${SK.vb.join(" ")}" width="${fx(SK.vb[2] * (scale || 0.68))}" height="${fx(SK.vb[3] * (scale || 0.68))}" style="overflow:visible">${out}</svg>`, step: cur + 1, of: SK.steps.length, caption: SK.steps[cur] ? SK.steps[cur].caption : "" };
  }
  function breadPage(t) {
    const b1 = TXT.bread.paras[0], b2 = TXT.bread.paras[1];
    const blocks = [{ id: "b1", kind: "p", text: b1, ranges: [], gutter: { type: "tack" }, first: true }, { id: "b2", kind: "p", text: b2, ranges: [], gutter: { type: "tack" } }];
    const i = b2.indexOf(TXT.bread.target);
    if (t >= T.wipe7[1] - 0.2 && t < T.sew7[0]) blocks[1].sel = [i, i + TXT.bread.target.length];
    const threads = [];
    blocks[1].ranges.push({ a: i, b: i + TXT.bread.target.length, id: "k1", cls: "anc" });
    if (t >= T.sew7[0]) {
      const u = unfoldAt(t, [[T.carry7 + 0.35, 1]]);
      let body = "";
      if (t < T.idea7) body = `<div class="think"><span>Stitching a sketch…</span><span class="dot" style="width:5px;height:5px;opacity:${fx(0.3 + 0.6 * (0.5 + 0.5 * Math.sin(t / 1.1 * Math.PI * 2)))}"></span></div>`;
      else {
        body = A.B.msg(TXT.idea);
        if (t >= T.steps7[0]) {
          const sk = sketchSVG(t, T.steps7, 0.68);
          body += `<div class="sk">${sk.svg}</div><div class="capt"><span class="gr">${esc(sk.caption)}</span><span class="ctl">${A.icon("left", 11, "var(--faint)")}${sk.step}/${sk.of}${A.icon("right", 11, "var(--faint)")}${A.icon("replay", 10, "var(--faint)")}</span></div><div class="larger" style="${t >= T.larger7 - 0.3 && t < T.larger7 + 0.1 ? "color:var(--ink)" : ""}">See it larger</div>`;
        }
      }
      threads.push({ id: "k1", block: "b2", kind: "question", c: "madder", side: "right", sew: prog(t, ...T.sew7), needle: t < T.sew7[1],
        ropeOp: t < T.carry7 ? 0 : u > 0.05 ? lerp(0.26, 1, u) : 0.26, ropeTrack: t < T.carry7 + 1.4 ? { t0: T.carry7, carry: 0.45 } : null, tag: t >= T.carry7 + 0.4, wash: u > 0.4,
        tagState: { unfold: u, mind: "Inkling", kindLabel: "sketch", body, nolift: false } });
      threads[0].num = 1; blocks[1].gutter = { type: "knot", n: 1, c: "var(--voice)" };
    }
    const page = { blocks, threads, header: { op: 1, loose: threads.length }, title: { text: TXT.bread.title }, border: {}, scroll: 0, caretOn: false, menus: [] };
    if (within(t, T.bar7, T.sketch7 + 0.1)) page.menus.push({ type: "toolbar", block: "b2", anchor: "k1", op: spring(t - T.bar7, 400, 32), hov: t >= T.sketch7 - 0.3 ? "sketch" : null });
    return page;
  }

  // ======================================================================= the camera
  // (x, y): the world point at the centre of the frame; z: screen px per point. Moves ease
  // like cloth: a little overshoot, then they settle (a spring at damping 0.72).
  const CLOTH = (() => { const z = 0.72, w = 9, wd = w * Math.sqrt(1 - z * z); const f = (k) => 1 - Math.exp(-z * w * k) * (Math.cos(wd * k) + ((z * w) / wd) * Math.sin(wd * k)); const f1 = f(1); return (k) => (k <= 0 ? 0 : k >= 1 ? 1 : f(k) / f1); })();
  const GLIDE = E.io;
  let CAM = [];
  function camAt(t) {
    let c = { x: 660, y: -330, z: 1.5 };
    for (const m of CAM) {
      if (t <= m.t[0]) break;
      const k = (m.e || CLOTH)(prog(t, m.t[0], m.t[1]));
      const to = typeof m.to === "function" ? m.to() : m.to;
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
    const m = (MEAS.set && MEAS.set[pane] && MEAS.set[pane][key]) || { x: 300, y: 300, w: 400, h: 100 };
    return { x: SET.x + m.x + m.w / 2, y: SET.y + m.y - (sc || 0) + m.h / 2 + (dy || 0), z };
  }

  function buildCamera() {
    const M = MEAS.main, an = (id) => M.anchors[id] || [{ x: 351, y: 300, r: 900, b: 328 }], blk = (id) => M.blocks[id] || { y: 300, h: 28 };
    const brief = 28;
    const a1 = an("t1")[0], a2 = an("t2")[0], a3 = an("t3")[0], a4 = an("t4")[0], b5 = blk("p5"), b6 = blk("p6");
    const st = MEAS.start || {};
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
      { t: [c3 + 0.3, T.sel3[1]], to: { x: (a2.x + a2.r) / 2 + 120, y: a2.y + 10, z: 1.9 }, e: CLOTH },
      { t: [T.sew3[0], T.sew3[1]], to: { x: (a2.x + a2.r) / 2 + 60, y: a2.y + 18, z: 2.3 }, e: GLIDE },
      { t: [T.carry3 - 0.1, T.open3 + 0.3], to: { x: 1040, y: a2.y + 70, z: 1.95 }, e: CLOTH },
      { t: [T.fold3, T.sel4[1]], to: { x: (a3.x + a3.r) / 2, y: a3.y + 30, z: 1.75 }, e: CLOTH },
      { t: [T.send4, T.carry4 + 0.6], to: { x: 330, y: a3.y + 150, z: 1.7 }, e: CLOTH },
      { t: [T.fold4 - 0.2, T.sel5[1]], to: { x: (a4.x + a4.r) / 2 + 160, y: a4.y + 40, z: 1.7 }, e: CLOTH },
      { t: [T.send5, T.carry5 + 0.3], to: { x: 780, y: a4.y + 50, z: 1.6 }, e: CLOTH },
      { t: [T.open5 - 0.2, T.hov5b], to: { x: 1040, y: a4.y + 140, z: 1.8 }, e: CLOTH },
      { t: [T.hov5b - 0.1, T.hov5c], to: { x: 1040, y: a4.y + 190, z: 1.8 }, e: CLOTH },
      { t: [T.hov5c - 0.1, T.fold5], to: { x: 1040, y: a4.y + 240, z: 1.8 }, e: CLOTH },
      { t: [T.fold5, T.p5[1]], to: { x: 640, y: b5.y + 10, z: 1.75 }, e: CLOTH },
      { t: [T.carry6 - 0.2, T.carry6 + 0.6], to: { x: 360, y: b5.y + 40, z: 1.7 }, e: CLOTH },
      { t: [T.wipe7[0], T.wipe7[1]], to: { x: 760, y: 300, z: 1.75 }, e: CLOTH },
      { t: [T.sew7[0], T.carry7 + 0.4], to: { x: 900, y: 320, z: 1.7 }, e: CLOTH },
      { t: [T.idea7, T.steps7[0]], to: { x: 1060, y: 470, z: 1.4 }, e: CLOTH },
      { t: [T.steps7[0], T.larger7], to: { x: 1070, y: 560, z: 1.32 }, e: GLIDE },
      { t: [T.big7[0], T.big7[0] + 0.6], to: { x: 660, y: 430, z: 1.25 }, e: CLOTH },
      { t: [T.wipe8[0], T.wipe8[1]], to: { x: 640, y: 200, z: 1.6 }, e: CLOTH },
      { t: [T.save8, T.chatIn8[1]], to: { x: 860, y: 330, z: 1.35 }, e: CLOTH },
      { t: [T.chatType8[0], T.chatStream8[0]], to: { x: 1110, y: 300, z: 1.75 }, e: CLOTH },
      { t: [T.chatStream8[0], T.chatStream8[1]], to: { x: 1130, y: 360, z: 1.75 }, e: GLIDE },
      // ch4: threads
      { t: [c4, c4 + 1.0], to: { x: 660, y: 300 + brief, z: 1.3 }, e: CLOTH },
      { t: [c4 + 1.2, c4 + 3.0], to: { x: 520, y: 330 + brief, z: 1.75 }, e: GLIDE },
      { t: [T.hov9[0] - 0.3, T.hov9[5]], to: { x: 1000, y: a4.y + 90 + brief, z: 1.75 }, e: CLOTH },
      { t: [T.jump9[0], T.jump9[0] + 0.45], to: { x: 300, y: a3.y + 80 + brief, z: 1.85 }, e: CLOTH },
      { t: [T.jump9[1], T.jump9[1] + 0.45], to: { x: 1040, y: a4.y + 80 + brief, z: 1.85 }, e: CLOTH },
      { t: [T.drag9[0] - 0.6, T.drag9[1]], to: { x: 1000, y: a2.y + 120 + brief, z: 1.55 }, e: CLOTH },
      { t: [T.replyFocus9, T.answer9[1]], to: { x: 1010, y: a2.y + 300 + brief, z: 1.6 }, e: GLIDE },
      { t: [T.breath9, T.tie9 + 0.1], to: { x: 1040, y: a2.y + 28 + 100, z: 2.7 }, e: GLIDE },
      { t: [T.fly9[0] - 0.1, T.fly9[1] + 0.3], to: { x: 820, y: 240, z: 1.3 }, e: CLOTH },
      { t: [T.open10 - 0.6, T.open10], to: { x: 330, y: a3.y + 130 + brief, z: 1.75 }, e: CLOTH },
      { t: [T.snip10 - 0.001, T.snip10], to: { x: 560, y: 560, z: 1.22 }, e: (k) => k },
      { t: [T.undo10 + 0.6, T.reopen11], to: { x: 900, y: a2.y + 150 + brief, z: 1.6 }, e: CLOTH },
      { t: [T.hide12 - 0.4, T.hide12 + 0.2], to: { x: 660, y: 300 + brief, z: 1.32 }, e: CLOTH },
      { t: [T.right13 - 0.2, T.both13 + 0.4], to: { x: 700, y: 320 + brief, z: 1.28 }, e: GLIDE },
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
    const sx = (x) => SET.x + x, sy = (y) => SET.y + y;
    const F = (pane, key, sc, z, dz, dy) => { const a = setFocus(pane, key, sc, z, dy); return [a, { ...a, z: a.z * (1 + (dz || 0.05)) }]; };
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
    cut(c7, { x: 380, y: 330 + brief, z: 1.45 }, null);
    cut(M7.side, { x: 380, y: 330, z: 1.45 }, { d: 2.4, to: { x: 400, y: 330, z: 1.55 } });
    cut(M7.find, { x: 660, y: 330, z: 1.5 }, { d: 1.8, to: { x: 660, y: 330, z: 1.58 } });
    cut(M7.copy, { x: 660, y: 640, z: 1.4 }, { d: 1.2, to: { x: 660, y: 660, z: 1.48 } });
    cut(M7.exp, { x: 660, y: 430, z: 1.75 }, { d: 2.4, to: { x: 660, y: 430, z: 1.86 } });
    cut(M7.obs, { x: 390, y: 200, z: 1.55 }, { d: 2.4, to: { x: 400, y: 215, z: 1.62 } });
    cut(M7.help, { x: 470, y: 130, z: 1.55 }, { d: 1.2, to: { x: 520, y: 200, z: 1.4 } });
    f = F("Model", "models", SCROLL.model, 1.65, 0.05, 40); cut(M7.model, f[0], { d: 2.4, to: f[1] });
    f = F("Model", "claude", 0, 2.0); cut(M7.claude, f[0], { d: 1.2, to: f[1] });
    cut(M7.blocks, { x: 760, y: blk("p4").y + brief + 40, z: 1.75 }, { d: 1.6, to: { x: 760, y: blk("p4").y + brief + 40, z: 1.85 } });
    // the end: to the Coach's tag, up with its knot to the border, and in on the border
    CAM.push({ t: [cEnd, cEnd + 0.7], to: { x: 990, y: b6.y + 150 + brief, z: 1.7 }, e: CLOTH });
    CAM.push({ t: [ED.breath, ED.tie + 0.1], to: { x: 974, y: b6.y + brief + 52, z: 3.8 }, e: GLIDE });
    CAM.push({ t: [ED.fly[0] - 0.05, ED.fly[1] + 0.25], to: { x: 660, y: 92, z: 2.2 }, e: CLOTH });
    CAM.push({ t: [ED.fly[1] + 0.25, ED.weave[1]], to: { x: 660, y: 90, z: 2.5 }, e: GLIDE });
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
  cap(T.p5[0] + 0.2, T.wipe7[0] - 0.1, "Or start a sentence with a mind’s name.");
  cap(T.sel7 + 0.2, T.larger7 - 0.2, "Sketch it: a drawing, stitched out step by step.");
  cap(T.larger7, T.wipe8[0], "Click a sketch to see it larger.", { kind: "label" });
  cap(T.tell8, T.chatKey8 - 0.15, "Tell Inkling what the note is for.");
  cap(T.chatKey8, T.chatStream8[1] + 0.2, "A chat about the whole note. It has read every thread.", { kind: "label", keys: { at: T.chatKey8, k: ["⌘", "J"] } });
  cap(c4 + 1.1, T.hov9[0] - 0.2, "Open questions are numbered, with a knot in the gutter.", { kind: "label" });
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
  lab(M6.keys, M6.end, "Every shortcut can be changed.");
  lab(M7.side, M7.find, "Your notes and folders. A dot marks open threads.", { at: M7.side + 0.1, k: ["⌘", "\\"] });
  lab(M7.find, M7.copy, "Find a note, or start one.", { at: M7.find + 0.1, k: ["⌘", "K"] });
  lab(M7.copy, M7.exp, "Copy a note as Markdown.", { at: M7.copy + 0.1, k: ["⇧", "⌘", "C"] });
  lab(M7.exp, M7.obs, "Export to Markdown, Word or HTML, with the threads or without.", { at: M7.exp + 0.1, k: ["⇧", "⌘", "E"] });
  lab(M7.obs, M7.help, "File › Import from Obsidian: a vault comes in, folders and all.");
  lab(M7.help, M7.model, "Help › Welcome Guide.");
  lab(M7.model, M7.claude, "Pick the model, and how much effort it spends.");
  lab(M7.claude, M7.blocks, "It runs on the Claude login already on this Mac.");
  lab(M7.blocks, M7.end, "Title, three heading sizes, lists and quotes.");

  // chapters: woven labels that hang in on a thread, then tie off; the knot flies to the border
  const CH = [
    { n: 1, name: "Start here", t: c1 - 0.6, c: "madder", x: 1430 }, { n: 2, name: "Reading along", t: c2 + 0.35, c: "madder", x: 1420 }, { n: 3, name: "Asking", t: c3 + 0.0, c: "indigo", x: 470 },
    { n: 4, name: "Threads", t: c4 + 0.6, c: "ochre", x: 1450 }, { n: 5, name: "Minds", t: c5 + 0.1, c: "sage", x: 1460 }, { n: 6, name: "Make it yours", t: c6 + 0.0, c: "plum", x: 1420 }, { n: 7, name: "Your notebook", t: c7 + 0.0, c: "walnut", x: 1430 },
  ];
  const CH_HOLD = 2.6;

  // ======================================================================= the end: Darn · Border
  // Every thread you tie off adds a darn to the note's border, each in its own weave. The
  // border's motifs give way to the darns, woven warp first and then weft, block by block;
  // the woven band moves into the icon, and the wordmark is sewn on, dotted with a cross.
  let LG = null;
  function buildLogo() {
    const P = new DOMParser();
    const host = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    host.setAttribute("style", "position:absolute;left:-99999px;top:0;width:1024px;height:1024px");
    document.body.appendChild(host);
    const parse = (src) => { const d = P.parseFromString(src, "image/svg+xml").documentElement; d.querySelectorAll("style").forEach((n) => n.remove()); return d; };
    // the icon (dark: the film ends in Lamplight); its parts by colour
    const ic = parse(LOGO.iconDark);
    const items = [];
    ic.querySelectorAll("path,rect").forEach((el) => {
      if (el.closest("clipPath")) return;
      const fill = (el.getAttribute("fill") || "").toUpperCase();
      const clone = el.cloneNode(true);
      host.appendChild(clone);
      const b = clone.getBBox();
      clone.remove();
      const role = fill === "#D9705E" ? "warp" : fill === "#EDE6DA" ? "weft" : el.tagName === "rect" ? "line" : "tile";
      items.push({ role, html: el.outerHTML.replace(/fill="#[0-9A-Fa-f]+"/, ""), b: { x: b.x, y: b.y, w: b.width, h: b.height } });
    });
    const band = items.filter((i) => i.role === "warp" || i.role === "weft");
    const bx0 = Math.min(...band.map((i) => i.b.x)), bx1 = Math.max(...band.map((i) => i.b.x + i.b.w)), by0 = Math.min(...band.map((i) => i.b.y)), by1 = Math.max(...band.map((i) => i.b.y + i.b.h));
    // the darn blocks: warp stitches grouped where there is a gap along the band
    const warp = band.filter((i) => i.role === "warp").sort((a, b) => a.b.x - b.b.x);
    const blocks = [];
    warp.forEach((w) => { const last = blocks[blocks.length - 1]; if (!last || w.b.x - last.x1 > 14) blocks.push({ x0: w.b.x, x1: w.b.x + w.b.w }); else last.x1 = Math.max(last.x1, w.b.x + w.b.w); });
    band.forEach((i) => { const cx = i.b.x + i.b.w / 2; i.block = Math.max(0, blocks.findIndex((bl) => cx >= bl.x0 - 8 && cx <= bl.x1 + 8)); });
    // the wordmark: letters in ink, the cross in madder
    const wm = parse(LOGO.wordmark);
    const vb = wm.getAttribute("viewBox").split(/\s+/).map(Number);
    const letters = [], cross = [];
    wm.querySelectorAll("path").forEach((el) => { (el.getAttribute("class") === "mad" ? cross : letters).push(el.getAttribute("d")); });
    host.remove();
    LG = { items, band: { x0: bx0, x1: bx1, y0: by0, y1: by1 }, blocks, letters, cross, vb };
  }
  function endHTML(t, cam) {
    if (t < ED.fly[1] - 0.05 || !LG) return "";
    const B0 = LG.band, bw = B0.x1 - B0.x0, bh = B0.y1 - B0.y0;
    // where the band is: first over the page's border, then in the icon
    const at = toScreen(cam, 660, 68);
    const sWeave = 820 / bw;
    const icon = { x: 470, y: 330, size: 400 };
    const sIcon = icon.size / 1024;
    const m = CLOTH(prog(t, ...ED.morph));
    const s0 = sWeave, s1 = sIcon;
    const sc = lerp(s0, s1, m);
    const cx = lerp(at[0], icon.x + ((B0.x0 + B0.x1) / 2) * sIcon, m), cy = lerp(at[1] + 4, icon.y + ((B0.y0 + B0.y1) / 2) * sIcon, m);
    const tx = cx - ((B0.x0 + B0.x1) / 2) * sc, ty = cy - ((B0.y0 + B0.y1) / 2) * sc;
    const W = prog(t, ...ED.weave), nb = LG.blocks.length;
    let band = "", rest = "";
    for (const it of LG.items) {
      if (it.role === "warp" || it.role === "weft") {
        // each block: its warp drops in, then its weft is woven across
        const b0 = it.block / nb, b1 = (it.block + 1) / nb;
        const k = prog(W, b0, b1);
        const bl = LG.blocks[it.block];
        const rel = it.role === "warp" ? (it.b.x - bl.x0) / Math.max(1, bl.x1 - bl.x0) : (it.b.y - B0.y0) / Math.max(1, bh);
        const g = it.role === "warp" ? clamp((k / 0.55 - rel * 0.5) * 2) : clamp(((k - 0.5) / 0.5 - rel * 0.55) * 2.2);
        if (g <= 0) continue;
        const e = E.out(g);
        const tr = it.role === "warp" ? `translate(0 ${fx(it.b.y)}) scale(1 ${fx(e)}) translate(0 ${fx(-it.b.y)})` : `translate(${fx(it.b.x)} 0) scale(${fx(e)} 1) translate(${fx(-it.b.x)} 0)`;
        band += `<g transform="${tr}" style="fill:${it.role === "warp" ? "var(--madder)" : "var(--ink)"}">${it.html}</g>`;
      } else {
        const k = it.role === "tile" ? E.out(prog(t, ED.morph[0] + 0.25, ED.morph[1])) : clamp((prog(t, ED.morph[0] + 0.5, ED.morph[1] + 0.3) * 1.4 - (it.b.y - 400) / 700 * 0.4 - (it.b.x - 100) / 800 * 0.3) * 1.6);
        if (k <= 0) continue;
        rest += it.role === "tile" ? `<g style="fill:var(--paper)" opacity="${fx(k)}">${it.html}</g>` : `<g transform="translate(${fx(it.b.x)} 0) scale(${fx(E.out(k))} 1) translate(${fx(-it.b.x)} 0)" style="fill:color-mix(in srgb,var(--rule) 70%,var(--faint))" opacity="${fx(k)}">${it.html}</g>`;
      }
    }
    let html = "";
    const plate = prog(t, ED.morph[0], ED.morph[0] + 0.8);
    if (plate > 0) html += `<div style="position:absolute;inset:0;background-color:var(--cloth);background-image:var(--weave);background-size:48px 48px;opacity:${fx(plate)}"></div>`;
    html += `<svg width="1920" height="1080" style="position:absolute;inset:0;overflow:visible">`;
    if (rest) html += `<g transform="translate(${fx(icon.x)} ${fx(icon.y)}) scale(${fx(sIcon)})">${rest}</g>`;
    html += `<g transform="translate(${fx(tx)} ${fx(ty)}) scale(${fx(sc)})">${band}</g>`;
    // the wordmark, sewn on beside the icon, and its cross-stitch dot
    const vb = LG.vb, ws = 590 / vb[2], wx = 960, wy = 400;
    const rk = prog(t, ...ED.word);
    if (rk > 0) {
      const reveal = vb[0] + E.io(rk) * (vb[2] + 10);
      html += `<defs><clipPath id="wmclip"><rect x="${fx(vb[0] - 5)}" y="${fx(vb[1] - 5)}" width="${fx(Math.max(0, reveal - vb[0] + 5))}" height="${fx(vb[3] + 10)}"/></clipPath></defs>`;
      html += `<g transform="translate(${fx(wx - vb[0] * ws)} ${fx(wy - vb[1] * ws)}) scale(${fx(ws)})"><g clip-path="url(#wmclip)" style="fill:var(--ink)">${LG.letters.map((d) => `<path d="${d}"/>`).join("")}</g>`;
      LG.cross.forEach((d, i) => {
        const k = knotScale(t - (i ? ED.dot[1] : ED.dot[0]));
        if (k <= 0) return;
        html += `<g transform="translate(0 0)"><path d="${d}" style="fill:var(--madder)" opacity="${fx(clamp(k))}" transform="scale(1)"/></g>`;
      });
      html += `</g>`;
      if (rk < 1) { const nx = wx + (reveal - vb[0]) * ws; html += `<g transform="translate(${fx(nx)} ${fx(wy + 230)}) scale(2.4)">${A.NEEDLE(0, -7, (t * 2.4) % 1, "var(--graphite)")}</g>`; }
    }
    html += `</svg>`;
    // one line under the mark
    const lk = prog(t, ...ED.line);
    if (lk > 0) {
      const line = "Every thread you tie off adds a darn to the border.";
      const reveal = E.io(lk) * 1300;
      html += `<div style="position:absolute;left:0;right:0;top:${fx(790 + (1 - E.out(lk)) * 8)}px;text-align:center;font:400 34px/1.3 Figtree;color:var(--graphite);-webkit-mask-image:linear-gradient(90deg,#000 ${fx(310 + reveal - 40)}px,transparent ${fx(310 + reveal)}px)">${line}</div>`;
    }
    return html;
  }

  // ======================================================================= the pointer
  // Moves ease like a hand, on a slight arc; a press dips it. Targets: world points, or
  // functions of the frame's measured geometry.
  const PTR = [];
  const ptr = (t0, t1, to, o = {}) => PTR.push({ t0, t1, to, ibeam: o.ibeam, from: o.from, show: o.show });

  // ======================================================================= frames
  function worldState(t) {
    // which note the main window shows, and what else is on the desk
    const L = looks(t);
    let main = null, settings = null, extra = "";
    if (t < c2) {
      const page = startPage(t);
      main = { page, chromeOp: page.header.op };
    } else if (within(t, M7.help + 0.7, M7.model)) {
      const page = startPage(T.glide0); page.scroll = 0; page.header.op = 1;
      main = { page, chromeOp: 1 };
    } else if (within(t, T.wipe7[0], T.wipe8[0])) {
      main = { page: breadPage(t), chromeOp: 1 };
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
    if (o.worldHTML) html += o.worldHTML;
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
    const endK = prog(t, ED.morph[0], ED.morph[0] + 0.8);
    A_.style.opacity = String(fx(1 - endK));
    $("#film").innerHTML = captionsHTML(t) + chaptersHTML(t) + borderHTML(t) + keysHTML(t) + flyHTML(t, cam, geo) + endHTML(t, cam);
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
        S.name = "Coach".slice(0, count(S_name, t)); S.desc = COACH.description.slice(0, count(S_desc, t)); S.how = COACH.howItThinks.slice(0, count(S_how, t));
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
      if (setPane === "Shortcuts") { S.scroll = SCROLL.keys; S.rec = { recording: within(t, M6.keys + 0.45, M6.keys + 1.3), hide: t >= M6.keys + 1.3 ? "⌥⌘H" : within(t, M6.keys + 0.45, M6.keys + 1.3) ? "Press keys" : "⇧⌘.", hov: within(t, M6.keys + 0.2, M6.keys + 0.5) }; }
      if (setPane === "Model") { S.model = 0; S.hovModel = within(t, M7.model + 0.4, M7.model + 1.0) ? 1 : null; S.effRead = t >= M7.model + 1.5 ? "medium" : "low"; S.effAsk = "medium"; S.scroll = t >= M7.claude ? 0 : SCROLL.model; }
      o.settings = S;
    }
    const W = o.win;
    // chat (ch3 ends in it)
    if (within(t, T.chatIn8[0], T.chatOut9[1])) {
      const k = t < T.chatOut9[0] ? E.out(prog(t, ...T.chatIn8)) : 1 - E.out(prog(t, ...T.chatOut9));
      const ans = t >= T.chatStream8[0] ? streamed(TXT.chat, ...T.chatStream8, t) : "";
      W.chat = { k, title: "Sub-50 10k by March", threads: "7 threads (6 open)", empty: t < T.chatSend8, hovPill: within(t, T.pills8, T.pills8 + 0.5) ? 0 : within(t, T.pills8 + 0.5, T.chatType8[0]) ? 2 : null,
        me: t >= T.chatSend8 ? TXT.chatQ : null, think: within(t, T.chatSend8 + 0.1, T.chatStream8[0]), ans, ansShift: Math.max(0, (ans.length - 520) * 0.12), input: t < T.chatSend8 ? TXT.chatQ.slice(0, count(S.chat, t)) : "", caret: t < T.chatSend8 && t >= T.chatType8[0] && caretOn(t), hovClear: t >= T.clear8 };
    }
    // the sidebar (ch7), the palette, export, menus of the menu bar
    if (within(t, M7.side, M7.find) || within(t, M7.obs, M7.help)) {
      const obs = within(t, M7.obs, M7.help);
      const k = obs ? prog(t, M7.obs + 0.7, M7.obs + 0.95) : E.out(prog(t, M7.side + 0.15, M7.side + 0.45)) * (1 - prog(t, M7.find - 0.25, M7.find));
      W.sideK = k;
      const vault = obs ? [["Training log", ["Week 1", "Week 2", "Shoes"]], ["Recipes", ["Sourdough", "Porridge"]]].map(([f, ns], i) => ({ name: f, open: true, notes: ns.slice(0, clamp(Math.floor((t - M7.obs - 1.1 - i * 0.35) / 0.18), 0, ns.length)).map((n) => ({ title: n })), show: t >= M7.obs + 1.0 + i * 0.35 })).filter((f) => f.show) : [];
      W.side = { who: "you@example.com", folders: [
        { name: "Running", open: true, notes: [{ title: "Shoes: what wore out where" }, { title: "Sub-50 10k by March", on: true, dot: true }] },
        { name: "Home", open: true, notes: [{ title: "Kitchen shelf measurements" }, { title: "Why the bread didn’t rise", dot: true, hov: within(t, M7.side + 1.6, M7.side + 2.2) }] },
        { name: "Reading", open: !obs && t >= M7.side + 1.2, hov: within(t, M7.side + 0.9, M7.side + 1.25), notes: [{ title: "Reading list for winter" }] }, ...vault],
        loose: [{ title: "Questions for the landlord" }, { title: "Start here" }], hovNew: within(t, M7.side + 2.2, M7.find) };
    }
    if (within(t, M7.find, M7.copy)) W.overlay = paletteHTML(t);
    if (within(t, M7.copy, M7.exp)) W.toast = { text: "Copied as Markdown", op: prog(t, M7.copy + 0.2, M7.copy + 0.35) * (1 - prog(t, M7.exp - 0.2, M7.exp)) };
    if (within(t, M7.exp, M7.obs)) W.overlay = exportHTML(t);
    if (within(t, M7.obs, M7.obs + 0.9)) W.overlay = macMenuHTML("File", ["New Note  ⌘N", "Find or Start a Note…  ⌘K", "—", "Copy This Note  ⇧⌘C", "Export This Note…  ⇧⌘E", "—", "Import from Obsidian…"], t >= M7.obs + 0.5 ? 6 : -1, t >= M7.obs + 0.2);
    if (within(t, M7.help, M7.help + 0.7)) W.overlay = macMenuHTML("Help", ["Welcome Guide"], t >= M7.help + 0.35 ? 0 : -1, t >= M7.help + 0.12);
    // undo chip after the snip
    if (within(t, T.snip10, T.undo10 + 0.3)) W.undo = { op: prog(t, T.snip10 + 0.3, T.snip10 + 0.55) * (1 - prog(t, T.undo10, T.undo10 + 0.25)) };
    // the listening badge and ink drops (ch6)
    if (within(t, M6.speak + 0.15, M6.speak + 2.1)) W.listen = { op: prog(t, M6.speak + 0.15, M6.speak + 0.3) * (1 - prog(t, M6.speak + 1.95, M6.speak + 2.1)), text: t < M6.speak + 0.6 ? "Listening…" : P7.slice(0, Math.round(P7.length * prog(t, M6.speak + 0.6, M6.speak + 1.7))) };
    if (within(t, M6.drop, M6.speak)) W.blots = blotsSVG(t);
    if (within(t, T.big7[0], T.wipe8[0] + 0.3)) W.overlay = enlargedHTML(t);
    // the Start here guide: nothing yet but the border in the cold open
    return o;
  }
  const S_name = S.name, S_desc = S.desc, S_how = S.how;

  function paletteHTML(t) {
    const q = "bread".slice(0, clamp(Math.floor((t - M7.find - 0.5) / 0.09), 0, 5));
    const all = [["Home", "Kitchen shelf measurements"], ["Reading", "Reading list for winter"], ["Running", "Shoes: what wore out where"], ["", "Questions for the landlord"], ["Running", "Sub-50 10k by March", "Open now"], ["Home", "Why the bread didn’t rise"]];
    const list = q ? all.filter(([, n]) => n.toLowerCase().includes(q.slice(0, 3))) : all;
    const k = E.out(prog(t, M7.find + 0.1, M7.find + 0.3)) * (1 - prog(t, M7.copy - 0.2, M7.copy));
    return `<div class="modal-dim" style="background:rgba(var(--shade),.12);opacity:${fx(k)}"></div><div class="palette" style="opacity:${fx(k)};transform:translateY(${fx((1 - k) * -8)}px)"><div class="s">${A.icon("search", 14, "var(--faint)")}${q ? esc(q) + '<span class="icaret"></span>' : '<span class="ph">Find or start a note</span>'}</div>` +
      list.map(([f, n, s], i) => `<div class="li${i === 0 ? " on" : ""}">${f ? `<span class="f">${f} /</span>` : ""}<span class="grow">${n}</span>${s ? `<small>${s}</small>` : ""}</div>`).join("") +
      `<div class="ft" style="top:auto;bottom:0;position:absolute"><span>↵ open</span><span>“Folder / Name” files a new note</span><span>⌘⌫ delete</span></div></div>`;
  }
  function exportHTML(t) {
    const k = E.out(prog(t, M7.exp + 0.1, M7.exp + 0.3)) * (1 - prog(t, M7.obs - 0.2, M7.obs));
    const sel = t < M7.exp + 0.95 ? 0 : t < M7.exp + 1.55 ? 1 : 2;
    const inc = !(within(t, M7.exp + 1.9, M7.exp + 2.25));
    const opt = (i, tl, d) => `<div class="opt${sel === i ? " on" : ""}"><div class="t">${tl}</div><div class="d">${d}</div></div>`;
    return `<div class="modal-dim" style="background:rgba(var(--shade),.12);opacity:${fx(k)}"></div><div class="export" style="opacity:${fx(k)};transform:scale(${fx(0.97 + 0.03 * k)})"><h4>Export “Sub-50 10k by March”</h4><div class="opts">${opt(0, "Markdown", "Plain text for anywhere, another AI included.")}${opt(1, "Word", "To keep editing in Word, Pages or Google Docs.")}${opt(2, "HTML", "To read or print to PDF.")}</div>` +
      `<div class="inc"><div style="flex:1"><div class="t">Include the margin</div><div class="d">Each thread sits under the paragraph it’s about.</div></div><span class="sw${inc ? " on" : ""}"><i style="left:${inc ? 14 : 2}px"></i></span></div><div class="btns"><span>Cancel</span><b>Export…</b></div></div>`;
  }
  function macMenuHTML(name, items, hov, open) {
    const tab = (n) => `<span style="padding:1px 8px;border-radius:4px;${n === name ? "background:color-mix(in srgb,var(--ink) 14%,transparent)" : ""}">${n}</span>`;
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
      const m = 2 + Math.floor(r0() * 4);
      for (let i = 0; i < m; i++) { const a = r0() * Math.PI * 2, d = r + 2 + r0() * 7 * grow; s += `<circle cx="${fx(x + Math.cos(a) * d)}" cy="${fx(y + Math.sin(a) * d)}" r="${fx(0.6 + r0() * 1.4)}" style="fill:var(--ink)" opacity="${fx(alpha)}"/>`; }
    });
    return s;
  }
  function enlargedHTML(t) {
    const k = E.out(prog(t, T.big7[0], T.big7[0] + 0.35)) * (1 - prog(t, T.wipe8[0] - 0.1, T.wipe8[0] + 0.2));
    const starts = [T.big7[0] + 0.25, T.big7[0] + 0.85, T.big7[0] + 1.45];
    const sk = sketchSVG(t, starts, 1.5);
    return `<div class="modal-dim" style="background:rgba(var(--shade),.28);opacity:${fx(k)}"></div><div class="card" style="left:360px;top:70px;width:600px;height:700px;z-index:37;padding:28px 40px;opacity:${fx(k)};transform:scale(${fx(0.96 + 0.04 * k)})">` +
      `<div style="display:flex;justify-content:center">${sk.svg}</div><div style="margin-top:10px;font-size:13px;color:var(--graphite);text-align:center">${esc(sk.caption)}</div></div>`;
  }

  // ======================================================================= transitions
  function transitionAt(t) {
    if (within(t, ...T.wipe1)) return (tt, a, b) => unfoldWipe(tt, a, b, "f6", T.wipe1, c2 - 0.01);
    if (within(t, ...T.wipe17)) return (tt, a, b) => unfoldWipe(tt, a, b, "t8", T.wipe17, c6 - 0.01);
    if (within(t, ...T.wipe7)) return (tt, a, b) => stitchWipe(tt, a, b, T.wipe7, () => { const L = (MEAS.bread.anchors.k1 || [])[0]; return L ? [L.x, L.r, L.y + 25] : null; });
    if (within(t, ...T.wipe8)) return (tt, a, b) => stitchWipe(tt, a, b, T.wipe8, () => [437, 515, 117]);
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
    const y = lerp(ty, ty, 0) + 0;
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
    for (const [a, b] of [T.fly2, T.fly9, ED.fly]) {
      if (!within(t, a, b) || !geo) continue;
      const id = a === T.fly2[0] ? "t1" : a === T.fly9[0] ? "t2" : "t8";
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
  function pointerPath(t, cam, geo) {
    // world-space targets, resolved against this frame's geometry
    const G = geo || { tags: {} };
    const tag = (id, dx = 26, dy = 13) => () => { const g = G.tags[id]; return g ? [g.x + dx, g.y + dy] : null; };
    const at = (x, y) => () => [x, y];
    const word = (id, f = 0.5, line = 0) => () => { const L = (MEAS.main.anchors[id] || [])[line]; return L ? [lerp(L.x, L.r, f), L.y + 15 + (t >= T.save8 ? 28 : 0)] : null; };
    const keys = [
      [T.hover1 - 0.7, T.hover1, tag("f6")],
      [T.t1hover - 0.7, T.t1hover, tag("t1")],
      [T.e1sel - 0.4, T.e1sel, word("t1", 0.12, 0), true],
      [T.e2drag[0] - 0.3, T.e2drag[0], at(552, 292), true],
      [T.e2drag[0], T.e2drag[1], at(940, 292), true],
      [T.sel3[0] - 0.4, T.sel3[0], word("t2", 0), true], [T.sel3[0], T.sel3[1], word("t2", 1), true], [T.find3 - 0.35, T.find3, () => [1050, (MEAS.main.anchors.t2 || [{ y: 350 }])[0].y + 88]],
      [T.sel4[0] - 0.4, T.sel4[0], word("t3", 0), true], [T.sel4[0], T.sel4[1], word("t3", 1), true], [T.askClick4 - 0.4, T.askClick4, () => [1060, (MEAS.main.anchors.t3 || [{ y: 220 }])[0].y + 120]],
      [T.sel5[0] - 0.4, T.sel5[0], word("t4", 0), true], [T.sel5[0], T.sel5[1], word("t4", 1), true], [T.askClick5 - 0.4, T.askClick5, () => [1060, (MEAS.main.anchors.t4 || [{ y: 270 }])[0].y + 120]],
      [T.chip5a - 0.3, T.chip5a, () => [1140, (MEAS.main.anchors.t4 || [{ y: 270 }])[0].y + 46]], [T.chip5b - 0.3, T.chip5b, () => [1046, (MEAS.main.anchors.t4 || [{ y: 270 }])[0].y + 72]],
      [T.hov5b - 0.3, T.hov5b, tag("t5")], [T.hov5c - 0.3, T.hov5c, tag("t6")],
      [T.sketch7 - 0.8, T.sketch7 - 0.3, () => [1040, 377]], [T.larger7 - 0.4, T.larger7, () => { const g = G.tags.k1; return g && g.panel ? [g.panel.x + 52, g.panel.y + g.panel.h - 18] : [1050, 760]; }],
      [T.tell8 - 0.45, T.tell8, at(480, 106)],
      [T.hov9[0] - 0.3, T.hov9[0], tag("t4")], [T.hov9[2] - 0.3, T.hov9[2], tag("t5")], [T.hov9[4] - 0.3, T.hov9[4], tag("t6")],
      [T.drag9[0] - 0.4, T.drag9[0], tag("t2")], [T.drag9[0], T.drag9[1], tag("t2")],
      [T.replyFocus9 - 0.3, T.replyFocus9, () => { const g = G.tags.t2; return g && g.panel ? [g.panel.x + 60, g.panel.y + g.panel.h - 60] : null; }],
      [T.resolveHov9 - 0.35, T.resolveHov9, () => { const g = G.tags.t2; return g && g.panel ? [g.panel.x + 30, g.panel.y + g.panel.h - 20] : null; }],
      [T.open10 - 0.4, T.open10, tag("t3")],
      [T.reopen11 - 0.4, T.reopen11, word("t2", 0.5)],
      [T.reopen11 + 0.5, T.reopen11 + 0.8, () => { const g = G.tags.t2; return g && g.panel ? [g.panel.x + 30, g.panel.y + g.panel.h - 20] : null; }],
      [T.reopen11 + 1.0, T.reopen11 + 1.3, () => { const g = G.tags.t2; return g && g.panel ? [g.panel.x + g.panel.w - 22, g.panel.y + g.panel.h - 20] : null; }],
      [T.reopen11 + 1.45, T.reopen11 + 1.75, () => { const g = G.tags.t2; return g && g.panel ? [g.panel.x + g.panel.w - 80, g.panel.y + g.panel.h - 20] : null; }],
      [T.menu16 - 0.45, T.menu16, at(385, 106 + 28)], [T.pick16 - 0.45, T.pick16, at(400, 106 + 28 + 92)],
      [ED.act - 0.4, ED.act, tag("t8", 120, 40)],
      [ED.hov - 0.35, ED.hov, () => { const g = G.tags.t8; return g && g.panel ? [g.panel.x + 30, g.panel.y + g.panel.h - 20] : null; }],
      [ED.tie - 0.1, ED.tie, () => { const g = G.tags.t8; return g && g.panel ? [g.panel.x + 31, g.panel.y + g.panel.h - 19] : null; }],
      ...DROPS.map(([b, x, y], i) => [b - 0.45, b, at(x, y)]),
    ];
    let cur = null, ibeam = false, press = 0, visible = false;
    for (let i = 0; i < keys.length; i++) {
      const [a, b, fn, ib] = keys[i];
      if (t < a - 0.5 || t > b + 0.9 || t > ED.knot + 0.5 && b < ED.act) continue;
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

  // ======================================================================= sound cues
  // The score is made from these (audio.js). Every cue says where it is: `pan` is where the
  // thing sits on screen at that moment (-1 left .. 1 right), from the camera.
  const EVENTS = [];
  function buildEvents() {
    const E_ = EVENTS;
    const ev = (o) => E_.push(o);
    const pw = (t, x, y) => clamp((toScreen(camAt(t), x, y || 400)[0] - 960) / 960, -0.9, 0.9);
    const ps = (sx) => clamp((sx - 960) / 960, -0.9, 0.9);
    const M = MEAS.main, an = (id) => (M.anchors[id] || [{ x: 351, y: 300, r: 900, b: 328 }]);
    const brief = (t) => (t >= T.save8 ? 28 : 0);
    const colL = 351, col = 614;
    const caretX = (text, i) => colL + ((i % 74) / 74) * col;
    const keys = (times, text, by, pan) => times.forEach((tk, i) => ev({ type: "tap", t: tk, pan: pan != null ? pan : pw(tk, caretX(text, i), by), v: /[ .,]/.test(text[i]) ? 0.55 : 0.8 + 0.2 * Math.sin(i * 7.1) }));
    const cap = (o) => ev(o);
    // ---- the cold open
    ev({ type: "pierce", t: T.pierce, pan: ps(330) });
    ev({ type: "sew", t0: T.sew0, t1: T.sew1, pan0: ps(330), pan1: ps(1590), r0: 5, r1: 15, gain: 0.9 });
    ev({ type: "loose", t: T.sew1 + 0.18, pan: ps(1590), mind: "inkling" });
    ev({ type: "duck", t0: T.openTie - 0.55, t1: T.openTie, level: 0.12 });
    ev({ type: "taut", t0: T.openTie - 0.5, t1: T.openTie, pan0: ps(1590), pan1: ps(1590), f0: 110, f1: 165 });
    ev({ type: "knot", t: T.openTie, pan: ps(1590), resolve: 1 });
    ev({ type: "whoosh", t: T.lineUp, dur: 0.35, pan0: 0, pan1: 0, v: 0.5 });
    ev({ type: "whoosh", t: T.pull0, dur: T.pull1 - T.pull0, pan0: 0, pan1: 0, v: 0.55, low: true });
    // ---- chapters: a tag drops in on its thread, tied off; its knot lands in the border
    for (const ch of CH) {
      const t0 = ch.t, tie = ch.t + CH_HOLD - 0.22;
      ev({ type: "whoosh", t: t0, dur: 0.4, pan0: ps(ch.x), pan1: ps(ch.x), v: 0.35 });
      ev({ type: "thump", t: t0 + 0.33, pan: ps(ch.x), v: 0.7 });
      ev({ type: "taut", t0: tie - 0.3, t1: tie, pan0: ps(ch.x), pan1: ps(ch.x), f0: 130, f1: 196, v: 0.5 });
      ev({ type: "knot", t: tie, pan: ps(ch.x), resolve: 1, v: 0.75 });
      ev({ type: "whoosh", t: ch.t + CH_HOLD - 0.1, dur: 0.8, pan0: ps(ch.x), pan1: ps(slotX(ch.n - 1)), v: 0.3 });
      ev({ type: "stitch", t: ch.t + CH_HOLD + 0.7, pan: ps(slotX(ch.n - 1)) });
      ev({ type: "stitch", t: ch.t + CH_HOLD + 0.82, pan: ps(slotX(ch.n - 1)) });
    }
    // ---- captions: stitched in behind a needle, tied off, lifted
    for (const c of CAPS) {
      const big = c.kind === "title";
      ev({ type: "stitchrun", t0: c.t0 + 0.08, t1: c.t0 + 0.08 + clamp(0.3 + c.text.length * 0.012, 0.45, 0.95), pan0: -0.85, pan1: big ? -0.1 : -0.35, v: big ? 0.5 : 0.32 });
      ev({ type: "stitch", t: c.t1 - 0.22, pan: -0.3, v: 0.4 });
      if (c.keys) { ev({ type: "key", t: c.keys.at, pan: -0.2 }); if (c.keys.again) ev({ type: "key", t: c.keys.again, pan: -0.2 }); }
    }
    // ---- ch1: the guide, your sentence, its question
    ev({ type: "whoosh", t: T.glide0, dur: T.glide1 - T.glide0, pan0: 0, pan1: 0, v: 0.25, low: true });
    if (MEAS.moonAt) {
      ev({ type: "unfold", t: MEAS.moonAt - 0.5, pan: -0.45 });
      [0, 0.35, 0.7].forEach((d, i) => ev({ type: "sew", t0: MEAS.moonAt - 0.3 + d, t1: MEAS.moonAt - 0.3 + d + 0.3, pan0: -0.55, pan1: -0.4, r0: 11, r1: 12, gain: 0.4, soft: true }));
      ev({ type: "fold", t: MEAS.moonAt + 1.6, pan: -0.45 });
    }
    const lastY = MEAS.start.lastY;
    keys(S.first, FIRST, lastY);
    ev({ type: "needle", t0: T.read1[0], t1: T.read1[1], pan: pw(T.read1[0], 325, lastY) });
    ev({ type: "duck", t0: T.read1[1] - 0.45, t1: T.read1[1] + 0.05, level: 0.35 });
    ev({ type: "sew", t0: T.sew1a, t1: T.sew1b, pan0: pw(T.sew1a, 351, lastY), pan1: pw(T.sew1b, 740, lastY), r0: 9, r1: 13 });
    ev({ type: "taut", t0: T.carry1, t1: T.carry1 + 0.55, pan0: pw(T.carry1, 760, lastY), pan1: pw(T.carry1 + 0.5, 1020, lastY), f0: 98, f1: 147 });
    ev({ type: "thump", t: T.carry1 + 0.5, pan: pw(T.carry1 + 0.5, 1030, lastY) });
    ev({ type: "unfold", t: T.hover1, pan: pw(T.hover1, 1100, lastY) });
    ev({ type: "loose", t: T.hover1 + 0.1, pan: pw(T.hover1, 1100, lastY), mind: "inkling" });
    ev({ type: "voice", t0: T.hover1 + 0.5, t1: T.hover1 + 3.4, mind: "inkling", text: TXT.first, pan: pw(T.hover1, 1100, lastY) });
    ev({ type: "whoosh", t: T.wipe1[0], dur: T.wipe1[1] - T.wipe1[0], pan0: 0.4, pan1: 0, v: 0.55 });
    // ---- ch2: written, read, a thread, fixed, tied off
    keys(S.title, "Sub-50 10k by March", 150);
    const by = (id) => (M.blocks[id] || { y: 300 }).y + 14;
    keys(S.p1, P1, by("p1")); keys(S.p2, P2, by("p2")); keys(S.p3, P3, by("p3"));
    ev({ type: "needle", t0: T.read2a[0], t1: T.read2a[1], pan: pw(T.read2a[0], 325, by("p1")) });
    ev({ type: "tack", t: T.read2a[1], pan: pw(T.read2a[1], 325, by("p1")) });
    ev({ type: "needle", t0: T.read2b[0], t1: T.read2b[1], pan: pw(T.read2b[0], 325, by("p2")), v: 0.5 });
    ev({ type: "tick", t: T.t1arrive, pan: pw(T.t1arrive, 1030, by("p2")), v: 0.35 });
    const a1 = an("t1")[0];
    ev({ type: "unfold", t: T.t1hover, pan: pw(T.t1hover, 1100, a1.y) });
    ev({ type: "loose", t: T.t1hover + 0.1, pan: pw(T.t1hover, 1100, a1.y), mind: "inkling" });
    ev({ type: "voice", t0: T.t1hover + 0.45, t1: T.t1hover + 3.2, mind: "inkling", text: TXT.correction, pan: pw(T.t1hover, 1100, a1.y) });
    ev({ type: "fold", t: T.t1fold, pan: pw(T.t1fold, 1100, a1.y) });
    ev({ type: "tick", t: T.e1sel, pan: pw(T.e1sel, a1.x + 60, a1.y), v: 0.5 });
    keys(S.e1, EDIT1.to, a1.y); keys(S.e2, EDIT2.to, by("p2"));
    ev({ type: "needle", t0: T.reread[0], t1: T.reread[1], pan: pw(T.reread[0], 325, by("p2")) });
    ev({ type: "unfold", t: T.settleOpen, pan: pw(T.settleOpen, 1100, a1.y) });
    ev({ type: "voice", t0: T.settle[0], t1: T.settle[1], mind: "inkling", text: TXT.settle, pan: pw(T.settle[0], 1100, a1.y) });
    tieOff(T.breath2, T.tie2, T.knot2, T.fly2, (t) => pw(t, 1018, a1.y + 10), 2);
    keys(S.p4, P4, by("p4"));
    ev({ type: "needle", t0: T.read2d[0], t1: T.read2d[1], pan: pw(T.read2d[0], 325, by("p4")), v: 0.5 });
    ev({ type: "tack", t: T.read2d[1], pan: pw(T.read2d[1], 325, by("p4")) });
    // ---- ch3: asking
    const a2 = an("t2")[0], a3 = an("t3")[0], a4 = an("t4")[0];
    ev({ type: "tick", t: T.bar3, pan: pw(T.bar3, 1100, a2.y), v: 0.4 });
    sewThread(T.sew3, T.carry3, a2, "right", T.open3, "inkling", TXT.probe, T.open3 + 0.4, T.open3 + 3.6);
    ev({ type: "tick", t: T.bar4, pan: pw(T.bar4, 1100, a3.y), v: 0.4 }); ev({ type: "tick", t: T.askClick4, pan: pw(T.askClick4, 1100, a3.y), v: 0.6 });
    keys(S.q4, TXT.askQ, a3.y, pw(T.type4[0], 1100, a3.y));
    ev({ type: "key", t: T.send4, pan: pw(T.send4, 1100, a3.y), v: 0.6 });
    sewThread(T.sew4, T.carry4, a3, "left", T.carry4 + 0.6, "inkling", TXT.ask, ...T.stream4);
    ev({ type: "tick", t: T.bar5, pan: pw(T.bar5, 1100, a4.y), v: 0.4 }); ev({ type: "tick", t: T.askClick5, pan: pw(T.askClick5, 1100, a4.y), v: 0.6 });
    ev({ type: "chip", t: T.chip5a, pan: pw(T.chip5a, 1140, a4.y), mind: "skeptic" }); ev({ type: "chip", t: T.chip5b, pan: pw(T.chip5b, 1050, a4.y), mind: "tutor" });
    keys(S.q5, TXT.roomQ, a4.y, pw(T.type5[0], 1100, a4.y));
    ev({ type: "key", t: T.send5, pan: pw(T.send5, 1100, a4.y), v: 0.6 });
    ev({ type: "sew", t0: T.sew5[0], t1: T.sew5[1] + 0.12, pan0: pw(T.sew5[0], a4.x, a4.y), pan1: pw(T.sew5[1], a4.r, a4.y), r0: 11, r1: 16, gain: 1.1, triple: true });
    [["inkling", 0, 147], ["skeptic", 1, 110], ["tutor", 2, 196]].forEach(([m, k, f]) => ev({ type: "taut", t0: T.carry5 + k * 0.08, t1: T.carry5 + 0.5 + k * 0.08, pan0: pw(T.carry5, 970, a4.y), pan1: pw(T.carry5 + 0.5, 1030, a4.y + k * 40), f0: f * 0.75, f1: f, mind: m, v: 0.6 }));
    for (const [m, k] of [["inkling", 0], ["skeptic", 1], ["tutor", 2]]) ev({ type: "thump", t: T.carry5 + 0.5 + k * 0.08, pan: pw(T.carry5 + 0.5, 1030, a4.y + k * 40), v: 0.5 });
    ev({ type: "unfold", t: T.open5, pan: pw(T.open5, 1100, a4.y) });
    ev({ type: "voice", t0: T.stream5.inkling[0], t1: T.stream5.inkling[1], mind: "inkling", text: TXT.room.inkling, pan: pw(T.open5, 1110, a4.y) });
    ev({ type: "voice", t0: T.stream5.skeptic[0], t1: T.stream5.skeptic[1], mind: "skeptic", text: TXT.room.skeptic, pan: pw(T.hov5b, 1110, a4.y + 40), gainAt: [[T.hov5b, 1]], gain: 0.45 });
    ev({ type: "voice", t0: T.stream5.tutor[0], t1: T.stream5.tutor[1], mind: "tutor", text: TXT.room.tutor, pan: pw(T.hov5c, 1110, a4.y + 80), gainAt: [[T.hov5c, 1]], gain: 0.4 });
    ev({ type: "unfold", t: T.hov5b, pan: pw(T.hov5b, 1110, a4.y + 40) }); ev({ type: "unfold", t: T.hov5c, pan: pw(T.hov5c, 1110, a4.y + 80) }); ev({ type: "fold", t: T.fold5, pan: 0.4 });
    const b5 = M.blocks.p5 || { y: 470 };
    keys(S.p5, P5, b5.y + 14);
    ev({ type: "needle", t0: T.read6[0], t1: T.read6[1], pan: pw(T.read6[0], 325, b5.y) });
    sewThread(T.sew6, T.carry6, { x: 351, r: 880, y: b5.y }, "left", T.carry6 + 0.6, "skeptic", TXT.call, ...T.stream6);
    ev({ type: "sew", t0: T.wipe7[0], t1: T.wipe7[1] - 0.25, pan0: -0.9, pan1: 0.9, r0: 16, r1: 20, gain: 0.8 });
    ev({ type: "whoosh", t: T.wipe7[0], dur: T.wipe7[1] - T.wipe7[0], pan0: -0.6, pan1: 0.6, v: 0.35 });
    const k1 = (MEAS.bread.anchors.k1 || [{ x: 420, r: 760, y: 230 }])[0];
    ev({ type: "tick", t: T.bar7, pan: pw(T.bar7, 1100, k1.y), v: 0.4 }); ev({ type: "tick", t: T.sketch7, pan: pw(T.sketch7, 1100, k1.y), v: 0.6 });
    sewThread(T.sew7, T.carry7, k1, "right", null, null, null);
    ev({ type: "needle", t0: T.think7[0], t1: T.think7[1], pan: pw(T.think7[0], 1100, k1.y), v: 0.5 });
    ev({ type: "voice", t0: T.idea7 + 0.1, t1: T.idea7 + 0.9, mind: "inkling", text: TXT.idea, pan: pw(T.idea7, 1100, k1.y) });
    T.steps7.forEach((st) => ev({ type: "sew", t0: st, t1: st + 0.75, pan0: pw(st, 1040, 500), pan1: pw(st + 0.7, 1160, 500), r0: 10, r1: 12, gain: 0.55, soft: true }));
    T.steps7.forEach((st, i) => ev({ type: "loose", t: st + 0.78, pan: pw(st, 1100, 500), mind: "inkling", step: i }));
    ev({ type: "tick", t: T.larger7, pan: pw(T.larger7, 1050, 760), v: 0.6 });
    [T.big7[0] + 0.25, T.big7[0] + 0.85, T.big7[0] + 1.45].forEach((st) => ev({ type: "sew", t0: st, t1: st + 0.6, pan0: -0.2, pan1: 0.2, r0: 12, r1: 14, gain: 0.5, soft: true }));
    ev({ type: "sew", t0: T.wipe8[0], t1: T.wipe8[1] - 0.25, pan0: -0.9, pan1: 0.9, r0: 16, r1: 20, gain: 0.8 });
    ev({ type: "whoosh", t: T.wipe8[0], dur: T.wipe8[1] - T.wipe8[0], pan0: -0.6, pan1: 0.6, v: 0.35 });
    ev({ type: "tick", t: T.tell8, pan: pw(T.tell8, 480, 106), v: 0.6 });
    keys(S.brief, TXT.brief, 140);
    ev({ type: "tick", t: T.save8, pan: pw(T.save8, 900, 180), v: 0.6 });
    ev({ type: "whoosh", t: T.chatIn8[0], dur: 0.45, pan0: 0.9, pan1: 0.5, v: 0.35 });
    ev({ type: "tick", t: T.pills8, pan: 0.5, v: 0.3 }); ev({ type: "tick", t: T.pills8 + 0.5, pan: 0.5, v: 0.3 });
    keys(S.chat, TXT.chatQ, 820, 0.45);
    ev({ type: "key", t: T.chatSend8, pan: 0.5, v: 0.6 });
    ev({ type: "voice", t0: T.chatStream8[0], t1: T.chatStream8[1], mind: "inkling", text: TXT.chat, pan: 0.45 });
    ev({ type: "tick", t: T.clear8, pan: 0.5, v: 0.35 });
    // ---- ch4: threads
    ev({ type: "whoosh", t: T.chatOut9[0], dur: 0.4, pan0: 0.5, pan1: 0.9, v: 0.3 });
    [["t4", 0, "inkling", TXT.room.inkling], ["t5", 2, "skeptic", TXT.room.skeptic], ["t6", 4, "tutor", TXT.room.tutor]].forEach(([id, k, m, text]) => {
      const tt = T.hov9[k];
      ev({ type: "unfold", t: tt, pan: pw(tt, 1100, a4.y + 28) }); ev({ type: "fold", t: T.hov9[k + 1], pan: pw(tt, 1100, a4.y + 28) });
      ev({ type: "voice", t0: tt + 0.15, t1: tt + 0.75, mind: m, text, pan: pw(tt, 1100, a4.y + 28), gain: 0.7, words: 5 });
    });
    ev({ type: "whoosh", t: T.jump9[0], dur: 0.45, pan0: 0.5, pan1: -0.6, v: 0.45 }); ev({ type: "whoosh", t: T.jump9[1], dur: 0.45, pan0: -0.6, pan1: 0.6, v: 0.45 });
    const a2b = { ...a2, y: a2.y + 28 };
    ev({ type: "tick", t: T.drag9[0], pan: pw(T.drag9[0], 1030, a2b.y), v: 0.6 });
    ev({ type: "taut", t0: T.drag9[0], t1: T.drag9[1], pan0: pw(T.drag9[0], 1030, a2b.y), pan1: pw(T.drag9[1], 1120, a2b.y + 132), f0: 110, f1: 165, v: 0.55 });
    ev({ type: "thump", t: T.drag9[1], pan: pw(T.drag9[1], 1120, a2b.y + 132), v: 0.6 });
    ev({ type: "unfold", t: T.dragOpen9, pan: pw(T.dragOpen9, 1000, a2b.y + 132) });
    keys(S.reply, TXT.reply, a2b.y + 300, pw(T.reply9[0], 1000, a2b.y + 300));
    ev({ type: "key", t: T.send9, pan: pw(T.send9, 1000, a2b.y + 300), v: 0.6 });
    ev({ type: "needle", t0: T.send9 + 0.1, t1: T.answer9[0], pan: pw(T.send9, 1000, a2b.y + 300), v: 0.4 });
    ev({ type: "voice", t0: T.answer9[0], t1: T.answer9[1], mind: "inkling", text: TXT.answer, pan: pw(T.answer9[0], 1000, a2b.y + 300) });
    tieOff(T.breath9, T.tie9, T.knot9, T.fly9, (t) => pw(t, 1110, a2b.y + 145), 2);
    const a3b = { ...a3, y: a3.y + 28 };
    ev({ type: "unfold", t: T.open10, pan: pw(T.open10, 260, a3b.y) });
    ev({ type: "key", t: T.dis10, pan: -0.2 });
    ev({ type: "snip", t: T.snip10, pan: pw(T.snip10, 300, a3b.y), stop: 0.85 });
    ev({ type: "tick", t: T.snip10 + 0.95, pan: pw(T.snip10 + 0.9, 100, 820), v: 0.35 });
    ev({ type: "key", t: T.undo10, pan: -0.2 });
    ev({ type: "unsnip", t: T.undo10 + 0.02, pan: pw(T.undo10, 300, a3b.y) });
    ev({ type: "tick", t: T.reopen11, pan: pw(T.reopen11, 700, a2b.y), v: 0.55 });
    ev({ type: "unfold", t: T.reopen11 + 0.05, pan: pw(T.reopen11, 1000, a2b.y + 132) });
    [0.5, 0.95, 1.4].forEach((d) => ev({ type: "tick", t: T.reopen11 + d, pan: 0.35, v: 0.3 }));
    ev({ type: "key", t: T.hide12, pan: -0.2 });
    ev({ type: "lift", t: T.hide12, pan: 0 });
    ev({ type: "duck", t0: T.hide12 + 0.12, t1: T.show12, level: 0.06, hard: true });
    ev({ type: "key", t: T.show12, pan: -0.2 });
    ev({ type: "lift", t: T.show12 + 0.02, pan: 0, back: true });
    ev({ type: "whoosh", t: T.right13, dur: 0.6, pan0: -0.6, pan1: 0.6, v: 0.35 });
    ev({ type: "marks", t: T.marks13, pan: 0.1 });
    ev({ type: "whoosh", t: T.both13, dur: 0.5, pan0: 0.5, pan1: -0.4, v: 0.3 });
    ev({ type: "snip", t: T.snip14, pan: 0.4, stop: 0.32 });
    // ---- ch5: minds
    ["staff", "editor", "skeptic", "tutor", "investor"].forEach((m, i) => ev({ type: "chip", t: T.chips15 + i * 0.3 + 0.02, pan: -0.5 + i * 0.25, mind: m }));
    ev({ type: "tick", t: T.blank15, pan: 0.4, v: 0.6 });
    keys(S.name, "Coach", 0, -0.1); keys(S.desc, COACH.description, 0, -0.05); keys(S.how, COACH.howItThinks, 0, 0);
    ev({ type: "tick", t: T.voice15, pan: 0.3, v: 0.55 }); ev({ type: "tick", t: T.often15, pan: 0.35, v: 0.55 }); ev({ type: "tick", t: T.create15, pan: 0.3, v: 0.7 });
    ev({ type: "chip", t: T.create15 + 0.05, pan: 0.2, mind: "coach" });
    ev({ type: "whoosh", t: T.pan16[0], dur: T.pan16[1] - T.pan16[0], pan0: 0.8, pan1: -0.6, v: 0.5, low: true });
    ev({ type: "tick", t: T.menu16, pan: pw(T.menu16, 385, 134), v: 0.55 }); ev({ type: "tick", t: T.pick16, pan: pw(T.pick16, 400, 226), v: 0.6 });
    const b6 = M.blocks.p6 || { y: 520 };
    keys(S.p6, P6, b6.y + 42);
    ev({ type: "needle", t0: T.read16[0], t1: T.read16[1], pan: pw(T.read16[0], 325, b6.y + 28) });
    sewThread(T.sew16, T.carry16, { x: 351, r: 760, y: b6.y + 28 }, "right", T.open16, "coach", TXT.coach, T.open16 + 0.3, T.open16 + 2.8);
    ev({ type: "whoosh", t: T.wipe17[0], dur: T.wipe17[1] - T.wipe17[0], pan0: 0.3, pan1: 0, v: 0.55 });
    // ---- ch6 and ch7: the montages run on a sewing-machine groove; each cut lands on a beat
    ev({ type: "groove", t0: c6, t1: cEnd, bpm: 96 });
    const riff = [40, 43, 45, 47, 50, 47, 45, 43]; // E2 G2 A2 B2 D3 ... (E dorian)
    const beats = [M6.dark, M6.ink, M6.letter, M6.size, M6.mode, M6.pace, M6.drop, M6.speak, M6.learn, M6.keys, M7.side, M7.find, M7.copy, M7.exp, M7.obs, M7.help, M7.model, M7.claude, M7.blocks];
    beats.forEach((b, i) => ev({ type: "bass", t: b, note: riff[i % riff.length] }));
    ev({ type: "tick", t: M6.dark + 0.75, pan: 0.3, v: 0.6 }); ev({ type: "swell", t0: M6.dark + 0.85, t1: M6.dark + 2.4, note: 40 });
    for (let i = 0; i < 8; i++) ev({ type: "pluck", t: M6.ink + i * (BEAT / 2), note: [64, 66, 67, 69, 71, 73, 74, 76][i], pan: -0.35 + i * 0.1, v: 0.55 });
    ev({ type: "tick", t: M6.letter + 0.35, pan: 0.2, v: 0.6 }); ev({ type: "pluck", t: M6.letter + 0.35, note: 71, pan: 0.2, v: 0.5 });
    [[0.05, 62], [0.45, 74], [0.95, 67]].forEach(([d, n]) => { ev({ type: "tick", t: M6.size + d, pan: 0.1, v: 0.5 }); ev({ type: "pluck", t: M6.size + d, note: n, pan: 0.1, v: 0.4 }); });
    ev({ type: "tick", t: M6.mode, pan: -0.3, v: 0.55 }); ev({ type: "tick", t: M6.mode + 0.35, pan: -0.25, v: 0.6 });
    ev({ type: "tick", t: M6.pace + 0.35, pan: 0.3, v: 0.6 });
    ev({ type: "tick", t: M6.drop + 0.3, pan: 0.35, v: 0.6 });
    DROPS.forEach(([b, x, y]) => ev({ type: "drop", t: b, pan: pw(b, x, y) }));
    ev({ type: "key", t: M6.speak + 0.15, pan: -0.2, hold: 1.8 });
    ev({ type: "breath", t0: M6.speak + 0.3, t1: M6.speak + 1.9, pan: 0.5 });
    ev({ type: "tick", t: M6.learn + 0.3, pan: 0.4, v: 0.6 });
    ev({ type: "voice", t0: M6.learn + 0.95, t1: M6.learn + 1.7, mind: "inkling", text: TXT.learned, pan: 0.1, words: 6 });
    ev({ type: "tick", t: M6.keys + 0.45, pan: 0.4, v: 0.6 }); [1.1, 1.2, 1.3].forEach((d) => ev({ type: "tap", t: M6.keys + d, pan: 0.3, v: 0.9 }));
    ev({ type: "sew", t0: c7 - 0.6, t1: c7 - 0.05, pan0: -0.9, pan1: 0.9, r0: 16, r1: 20, gain: 0.7 });
    ev({ type: "key", t: M7.side + 0.1, pan: -0.2 }); ev({ type: "whoosh", t: M7.side + 0.15, dur: 0.35, pan0: -0.9, pan1: -0.5, v: 0.35 }); ev({ type: "tick", t: M7.side + 1.2, pan: -0.6, v: 0.5 });
    ev({ type: "key", t: M7.find + 0.1, pan: -0.2 }); for (let i = 0; i < 5; i++) ev({ type: "tap", t: M7.find + 0.5 + i * 0.09, pan: 0, v: 0.7 }); ev({ type: "key", t: M7.find + 1.3, pan: 0, v: 0.5 });
    ev({ type: "key", t: M7.copy + 0.1, pan: -0.2 }); ev({ type: "pop", t: M7.copy + 0.22, pan: 0 });
    ev({ type: "key", t: M7.exp + 0.1, pan: -0.2 }); [0.95, 1.55, 1.9, 2.25].forEach((d) => ev({ type: "tick", t: M7.exp + d, pan: 0, v: 0.55 }));
    ev({ type: "tick", t: M7.obs + 0.2, pan: -0.6, v: 0.5 }); ev({ type: "tick", t: M7.obs + 0.6, pan: -0.55, v: 0.6 });
    for (let i = 0; i < 7; i++) ev({ type: "stitch", t: M7.obs + 1.1 + i * 0.18, pan: -0.7, v: 0.5 });
    ev({ type: "tick", t: M7.help + 0.12, pan: -0.3, v: 0.5 }); ev({ type: "tick", t: M7.help + 0.45, pan: -0.25, v: 0.6 });
    ev({ type: "tick", t: M7.model + 0.5, pan: 0.2, v: 0.4 }); ev({ type: "tick", t: M7.model + 1.5, pan: 0.3, v: 0.55 });
    for (let k = 0; k < 7; k++) ev({ type: "pluck", t: M7.blocks + 0.3 + k * (BEAT / 2), note: [52, 55, 57, 59, 62, 64, 67][k], pan: 0.3, v: 0.5 });
    // ---- the end: the Coach's thread tied off; the border grows into the mark
    const b6y = b6.y + 28 + 28;
    ev({ type: "tick", t: ED.act, pan: pw(ED.act, 1100, b6y), v: 0.5 });
    tieOff(ED.breath, ED.tie, ED.knot, ED.fly, (t) => pw(t, 1010, b6y + 10), 2);
    ev({ type: "weave", t0: ED.weave[0], t1: ED.weave[1], blocks: 4 });
    ev({ type: "swell", t0: ED.morph[0], t1: ED.morph[1] + 1.2, note: 52 });
    ev({ type: "sew", t0: ED.word[0], t1: ED.word[1], pan0: 0.0, pan1: 0.55, r0: 12, r1: 4, gain: 0.7 });
    ev({ type: "stitch", t: ED.dot[0], pan: 0.0, v: 1 }); ev({ type: "stitch", t: ED.dot[1], pan: 0.0, v: 1 });
    ev({ type: "knot", t: ED.dot[1], pan: 0, resolve: 3 });
    E_.sort((a, b) => (a.t ?? a.t0) - (b.t ?? b.t0));

    // a held breath, a pull, the knot (and its flight to the page's border)
    function tieOff(breath, tie, knot, fly, panAt, resolve) {
      ev({ type: "duck", t0: breath, t1: tie + 0.02, level: 0.14 });
      ev({ type: "tick", t: tie, pan: panAt(tie), v: 0.55 });
      ev({ type: "taut", t0: tie, t1: knot, pan0: panAt(tie), pan1: panAt(knot), f0: 98, f1: 196, v: 0.9 });
      ev({ type: "knot", t: knot, pan: panAt(knot), resolve });
      ev({ type: "whoosh", t: fly[0], dur: fly[1] - fly[0], pan0: panAt(fly[0]), pan1: 0, v: 0.35 });
      ev({ type: "stitch", t: fly[1], pan: 0, v: 0.8 }); ev({ type: "stitch", t: fly[1] + 0.1, pan: 0, v: 0.7 });
    }
    // a thread pulled from the words to the margin, and its tag
    function sewThread(sew, carry, a, side, open, mind, text, s0, s1) {
      const y = a.y + 14;
      ev({ type: "sew", t0: sew[0], t1: sew[1], pan0: pw(sew[0], a.x, y), pan1: pw(sew[1], a.r, y), r0: 9, r1: 14 });
      const tabX = side === "right" ? 1030 : 290;
      ev({ type: "taut", t0: carry, t1: carry + 0.5, pan0: pw(carry, side === "right" ? 977 : 339, y), pan1: pw(carry + 0.5, tabX, y), f0: 98, f1: 147, mind });
      ev({ type: "thump", t: carry + 0.48, pan: pw(carry + 0.5, tabX, y) });
      if (open != null) { ev({ type: "unfold", t: open, pan: pw(open, tabX + (side === "right" ? 120 : -120), y) }); ev({ type: "loose", t: open + 0.1, pan: pw(open, tabX, y), mind }); }
      if (text) ev({ type: "voice", t0: s0, t1: s1, mind, text, pan: pw(s0, tabX + (side === "right" ? 120 : -120), y) });
    }
  }

  // ======================================================================= coverage
  // Every row of design/capabilities.md, and when the film shows it (render.ts coverage
  // writes coverage.md from this, so the times always match the cut).
  const COVERAGE = [];
  function buildCoverage() {
    const c = (section, row, t, what) => COVERAGE.push({ section, row, t: Math.round(t * 10) / 10, what });
    c("Writing", "A plain page: title, paragraphs, three heading sizes, bullet and numbered lists, quotes", M7.blocks + 0.3, "A paragraph is selected and the selection menu's first row turns it into H1, H2, H3, a quote, a bullet and a numbered item, one per half beat; the guide's headings and bullets are seen in chapter 1");
    c("Writing", "Speak instead of typing", M6.speak + 0.15, "⌥ is held, the corner shows Listening… and then the words as they come; letting go puts “Race morning I’ll eat by six.” in the note");
    c("Writing", "Ink from the pointer", M6.drop, "The switch in Settings › Appearance turns on; on the page, three drops of ink fall where the pointer rests, dry and fade");
    c("Writing", "Your look", M6.dark, "Settings › Appearance: System, Light and Dark (the whole desk turns to Lamplight), then the seven thread colours, lettering (Inkling’s own, like your notes) and text size (small, large, default)");
    c("Writing", "An immersive window", T.title2[0], "No bar: as the title and first paragraphs are typed, the sidebar toggle, Chat and the row above the title fade; caption “No bar. The controls fade while you write.”");
    c("Reading along", "Reads each paragraph at your pause", T.read2a[0], "After each paragraph the needle bobs in the gutter and leaves a tacking stitch; Settings › Reading shows pace Regular and Slow at " + mmss(M6.pace));
    c("Reading along", "Speaks only when it is worth it", T.t1arrive, "Most paragraphs get only a tacking stitch; ¶2 gets a correction (×); the guide’s Moon thread unfolds with its sketch at " + mmss(MEAS.moonAt || c1 + 3));
    c("Reading along", "Waits while you type", T.t1arrive, "The correction arrives while ¶3 is being typed and stays folded on its tab until the writer pauses");
    c("Reading along", "Fix the words, the thread settles", T.e1sel, "5:30 becomes 4:55 in the text; Inkling reads it again, says why it settles, and the thread ties itself off");
    c("Reading along", "Your first sentence", T.type1[0], "In the new notebook’s guide, the first sentence you write gets a question (real output)");
    c("Reading along", "Reads along, or waits to be asked", M6.mode, "The mode menu in the row above the title switches Reads along to Waits to be asked; Settings › Reading shows the default for new notes");
    c("Reading along", "Tell Inkling what a note is for", T.tell8, "Tell Inkling, above the title: the brief is typed and saved; the chat that follows answers with it in mind");
    c("Reading along", "Learns about you", M6.learn, "Settings › About you: the switch turns on and what it learned from a reply appears (real output)");
    c("Threads", "A thread from the words to the margin", T.sew1a, "Running stitch under your sentence, the thread carried to a folded tag; a correction starts with a cross-stitch at " + mmss(T.t1arrive));
    c("Threads", "Tags fold away", T.hov9[0], "Pointing at three folded tabs unfolds each in turn; clicking opens one with its reply box at " + mmss(T.dragOpen9));
    c("Threads", "Move a tag", T.drag9[0], "A tab is dragged down the margin, its thread stretching; it opens toward the side with room");
    c("Threads", "Numbered questions", c4 + 1.1, "Open questions numbered 1 to 6 in reading order, each with a knot and its number in the gutter");
    c("Threads", "Reply in the tag", T.replyFocus9, "A reply is written in the tag and the mind answers in the same thread (real output)");
    c("Threads", "Tie off", T.tie9, "Resolve: the thread pulls taut, the knot is tied and flies to the border at the top of the page");
    c("Threads", "Snip", T.dis10, "⇧⌘⌫ snips a thread (the sound stops dead), the stitches come out, “Thread dismissed · Undo ⌥⌘Z” shows, and ⌥⌘Z brings it back");
    c("Threads", "Reopen, Copy, Make a note", T.reopen11, "A tied-off thread is opened again: Reopen, the copy icon and Make a note are each pointed at; caption names all three");
    c("Threads", "Jump between threads", T.jump9[0], "⌥⌘↓ twice: the camera jumps to the next open thread each time");
    c("Threads", "Hide threads", T.hide12, "⇧⌘. lifts every mark off the page and leaves just the words (the sound drops away too); again, and they come back");
    c("Threads", "Where threads go", T.right13, "Both margins, then everything moves to the right margin, then marks only (a small mark after each passage); Settings › Reading has the three cards");
    c("Threads", "How many are open", c4 + 1.1, "“6 loose threads” above the title, counting down as threads are tied off");
    c("Asking", "Find a loose thread", T.sel3[0], "Words selected, the selection menu, ⌘↵: a question on the long-run line (real output)");
    c("Asking", "Ask your own question", T.sel4[0], "Selection menu › Ask Inkling, a question typed in the ask box, the answer streams into a tag in the left margin (real output)");
    c("Asking", "Sketch it", T.sel7, "In another note, words selected › Sketch it: “Stitching a sketch…”, then three steps stitched out with captions, and See it larger (real output)");
    c("Asking", "Ask the room", T.sel5[0], "Skeptic and Socratic tutor added in the ask box: three threads in three colours, three answers (real output)");
    c("Asking", "Call a mind by name", T.p5[0], "A paragraph starting “Skeptic,” gets the Skeptic’s answer on an indigo thread (real output)");
    c("Asking", "Chat about the whole note", T.chatKey8, "⌘J opens the chat with its three suggested questions; a question, the answer streams (real output); Clear is pointed at");
    c("Minds", "Pick the note’s mind", T.menu16, "The name above the title opens the mind menu; Coach is picked");
    c("Minds", "Built-in minds", T.chips15, "New mind › Start from: Staff engineer, Editor, Skeptic, Socratic tutor, Investor (and Blank); caption lists them with Inkling; the room’s chips show them too");
    c("Minds", "Make your own", T.blank15, "From Blank: name, description, how it thinks, voice (Blunt) and how often (Often), Create mind");
    c("Minds", "Each mind has its thread colour", T.open16, "The Coach’s question arrives on its own thread; the margin shows madder, indigo and ochre threads side by side");
    c("Notebook", "Notes and folders", M7.side, "⌘\\: the sidebar with folders (one unfolds), dots on notes with open threads, New note, New folder");
    c("Notebook", "Find or start a note", M7.find, "⌘K: the palette filters to “bread” as it is typed");
    c("Notebook", "Copy a note", M7.copy, "⇧⌘C: “Copied as Markdown”");
    c("Notebook", "Export", M7.exp, "⇧⌘E: Markdown, then Word, then HTML; Include the margin off and on");
    c("Notebook", "Import from Obsidian", M7.obs, "File › Import from Obsidian…: a vault’s folders and notes arrive in the sidebar");
    c("Notebook", "Welcome guide", M7.help, "Help › Welcome Guide opens the Start here note");
    c("Model and keys", "Runs on your Claude", M7.claude, "Settings › Model: Claude, signed in through this Mac’s Claude Code login; caption says so");
    c("Model and keys", "Pick the model and effort", M7.model, "Settings › Model: the models Claude Code offers (Default, Opus 5.5, Fable 5.1, Sonnet 5.5, Haiku 5.5, specific versions) and effort for reading along and for asking");
    c("Model and keys", "Every shortcut can be changed", M6.keys, "Settings › Shortcuts: the Hide threads key is clicked, “Press the new keys”, and ⌥⌘H takes its place");
  }
  const mmss = (t) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`;

  // ======================================================================= boot
  const FILM = { FPS, DUR, BEAT, T, M6, M7, ready: false, error: null, renderAt, events: EVENTS, coverage: COVERAGE, chapters: CH, camAt };
  window.FILM = FILM;
  async function boot() {
    try {
      const m = document.createElement("div");
      m.id = "measure";
      m.style.cssText = "position:absolute;left:0;top:0;width:1320px;height:860px;visibility:hidden;overflow:hidden";
      m.innerHTML = '<div class="page"></div>';
      $("#stage").appendChild(m);
      A.applyTheme(m, 0, "madder", {});
      await document.fonts.ready;
      await Promise.all(["400 17px 'Fraunces Text'", "500 17px 'Fraunces Text'", "600 30px 'Fraunces Display'", "500 30px 'Fraunces Display'", "400 14px Figtree", "500 14px Figtree", "600 14px Figtree"].map((f) => document.fonts.load(f)));
      A.measureWordmark();
      buildSketch();
      buildLogo();
      // Settings: where each part of each pane sits, for the camera
      const sh = document.createElement("div");
      sh.style.cssText = "position:absolute;left:0;top:0;width:900px;height:721px";
      m.appendChild(sh);
      MEAS.set = {};
      const panes = { Appearance: { pane: "Appearance", look: "light", size: "default", lettering: "own", ink: "madder" }, Reading: { pane: "Reading", placement: "both" }, NewMind: { pane: "NewMind" },
        "About you": { pane: "About you" }, Shortcuts: { pane: "Shortcuts", rec: {} }, Model: { pane: "Model", model: 0 } };
      for (const k in panes) MEAS.set[k] = window.SETTINGS.measure(sh, panes[k]);
      sh.remove();
      const ms = MEAS.set;
      SCROLL.ink = Math.max(0, ms.Appearance.dots.y - 330); SCROLL.letter = Math.max(0, ms.Appearance.letter.y - 300); SCROLL.size = Math.max(0, ms.Appearance.size.y - 300);
      SCROLL.drop = Math.max(0, ms.Appearance.inkdrop.y - 380); SCROLL.keys = Math.max(0, ms.Shortcuts.hide.y - 430); SCROLL.model = Math.max(0, ms.Model.models.y - 150);
      // the main note, finished, for the camera's targets
      const full = { blocks: [["p1", P1], ["p2", P2.replace(" and hold 5:30 per km on every run", ", two of them at 4:55 per km").replace("If I hold 5:30", "If I hold 4:55")], ["p3", P3], ["p4", P4], ["p5", P5], ["p6", P6], ["p7", P7]].map(([id, text]) => ({ id, kind: "p", text, ranges: [] })),
        threads: [], header: {}, title: { text: "Sub-50 10k by March" }, border: {} };
      const rng2 = (bid, phrase, id) => { const b = full.blocks.find((x) => x.id === bid); const i = b.text.indexOf(phrase); b.ranges.push({ a: i, b: i + phrase.length, id }); full.threads.push({ id, block: bid, kind: "question", noTag: true, sew: 0 }); };
      rng2("p2", "If I hold 4:55 for the whole race, I’ll come in just under 50.", "t1"); rng2("p3", "I’ll add a kilometre every week until I reach 16.", "t2");
      rng2("p1", TXT.askTarget, "t3"); rng2("p2", "four days a week", "t4"); rng2("p5", P5, "t7"); rng2("p6", P6, "t8");
      measureNote("main", full);
      const bp = breadPage(0); bp.threads = [{ id: "k1", block: "b2", kind: "question", noTag: true, sew: 0 }]; bp.menus = [];
      measureNote("bread", bp);
      // the guide: where "Your turn" ends, so the glide stops with your sentence in view
      const sp = startPage(T.glide0);
      sp.scroll = 0;
      measureNote("startRaw", sp);
      const lastB = sp.blocks[sp.blocks.length - 1];
      const lb = MEAS.startRaw.blocks[lastB.id];
      MEAS.startEnd = Math.max(0, lb.y - 520);
      MEAS.start = { lastY: lb.y - MEAS.startEnd + 14 };
      const moonB = sp.blocks.find((b) => b.text.startsWith("The far side of the Moon"));
      if (moonB) {
        const my = MEAS.startRaw.blocks[moonB.id].y;
        for (let tt = T.glide0; tt < T.glide1; tt += 0.02) if (my - startScroll(tt) < 300) { MEAS.moonAt = tt; break; }
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
      FILM.error = String(e && e.stack || e);
      console.error(e);
    }
  }
  boot();
})();

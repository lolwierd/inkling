/* Proof direction page: the proof sheet, the working app, the states and the reference
   sections. Needs site.js (window.Board) and marks.js (window.ProofMarks).
   The sheet is rendered from a small model; every hand-drawn mark is laid out from the
   real text positions after each render, so marks follow the words like they will in
   the app. */
(function () {
  "use strict";
  const M = window.ProofMarks, B = window.Board, N = B.note;
  const SVGNS = "http://www.w3.org/2000/svg";
  const $ = (s, r) => (r || document).querySelector(s);
  const $$ = (s, r) => Array.from((r || document).querySelectorAll(s));
  const esc = (t) => String(t).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const wait = B.wait;
  const f = M.f;
  let uid = 0;

  // ------------------------------------------------------------------ the note as data
  const T1 = N.threads[0], T2 = N.threads[1];
  const SRC = { 1: T1, 2: T2 };
  const QUOTE1_FIXED = T1.quote.replace(T1.fixFrom, T1.fixTo);
  const OPEN = ["open", "waiting", "rechecking"];
  const CLOSED = ["settled", "resolved", "worked"];

  function paraText(i, m) {
    const t = N.paras[i];
    return i === T1.para && m.fixed ? t.replace(T1.quote, QUOTE1_FIXED) : t;
  }
  const quoteOf = (t, m) => (t.id === 1 && m.fixed ? QUOTE1_FIXED : SRC[t.id].quote);

  function baseModel() {
    return {
      title: N.title,
      paras: [0, 1, 2, 3],
      fixed: false,
      threads: [
        { id: 1, kind: "correction", para: T1.para, status: "open", open: false, msgs: [], drawing: null },
        { id: 2, kind: "question", para: T2.para, status: "open", open: false, msgs: [], drawing: null },
      ],
      read: new Set(N.held),
      reading: new Set(),
      figure: false,
      selection: null,
      room: null,
      roomTools: false,
      quiet: false,
      banner: null,
      placeholder: null,
      firstRun: false,
      toast: null,
    };
  }

  // ------------------------------------------------------------------ drawing helpers
  /** Draw a ribbon on along its centre line, like a pencil, by animating a stroked mask. */
  function drawOn(p, c, w, ms, delay) {
    const svg = p.ownerSVGElement;
    if (!svg || !c) return;
    if (B.still) { p.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 120, delay: delay || 0, fill: "backwards" }); return; }
    let defs = svg.querySelector("defs");
    if (!defs) { defs = document.createElementNS(SVGNS, "defs"); svg.insertBefore(defs, svg.firstChild); }
    const id = "dm" + (++uid);
    const mask = document.createElementNS(SVGNS, "mask");
    mask.setAttribute("id", id);
    mask.setAttribute("maskUnits", "userSpaceOnUse");
    mask.setAttribute("x", "-4000"); mask.setAttribute("y", "-4000");
    mask.setAttribute("width", "12000"); mask.setAttribute("height", "12000");
    const mp = document.createElementNS(SVGNS, "path");
    mp.setAttribute("d", c);
    mp.setAttribute("fill", "none");
    mp.setAttribute("stroke", "#fff");
    mp.setAttribute("stroke-width", String(w * 2 + 4));
    mp.setAttribute("stroke-linecap", "round");
    mp.setAttribute("stroke-linejoin", "round");
    mask.appendChild(mp);
    defs.appendChild(mask);
    p.setAttribute("mask", "url(#" + id + ")");
    const len = mp.getTotalLength() + 2;
    mp.style.strokeDasharray = len + " " + len;
    const a = mp.animate([{ strokeDashoffset: len }, { strokeDashoffset: 0 }],
      { duration: ms || 280, delay: delay || 0, easing: "cubic-bezier(.2,.7,.2,1)", fill: "both" });
    a.onfinish = () => { p.removeAttribute("mask"); mask.remove(); };
  }
  /** A mark as SVG markup; fresh marks carry their centre line so they can be drawn on. */
  function pm(mark, cls, fresh, ms, delay) {
    let s = '<path class="' + cls + '" d="' + mark.d + '"';
    if (fresh) s += ' data-c="' + mark.c + '" data-w="' + mark.w + '" data-ms="' + (ms || 280) + '" data-delay="' + (delay || 0) + '"';
    return s + "/>";
  }
  function animateFresh(root) {
    $$("[data-c]", root).forEach((p) => {
      drawOn(p, p.getAttribute("data-c"), +p.getAttribute("data-w"), +p.getAttribute("data-ms"), +p.getAttribute("data-delay"));
      p.removeAttribute("data-c");
    });
    $$("[data-fade]", root).forEach((el) => {
      const d = +el.getAttribute("data-fade") || 0;
      el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: B.still ? 120 : 220, delay: d, fill: "backwards", easing: "cubic-bezier(.2,.7,.2,1)" });
      el.removeAttribute("data-fade");
    });
  }

  // ------------------------------------------------------------------ the sketch (figure)
  /** Inkling's sketch for AQ1, three steps. `shown` is how many steps are on the page;
   *  `fresh` is the step to draw on now (0 for none). Same geometry in the card and the note. */
  function figureSVG(shown, fresh) {
    const W = 256, H = 128, x0 = 18, x1 = 238, base = 90, n = 7, sx = (x1 - x0) / (n - 1);
    const X = (i) => x0 + i * sx;
    const D = N.drawing;
    const steps = [[], [], []];
    const nf = (k) => fresh === k + 1;
    // a · pace per km
    steps[0].push(pm(M.mark(M.resample([[x0 - 8, base + 0.3], [W / 2, base - 0.4], [x1 + 8, base + 0.2]], 1), 1.25, [0.03, 0.04], 0.12, 51), "fig-ink", nf(0), 520));
    for (let i = 0; i < n; i++) steps[0].push(pm(M.mark([[X(i), base - 4], [X(i) + 0.3, base + 4]], 1.05, [0.25, 0.25], 0, 60 + i), "fig-ink", nf(0), 120, 300 + i * 40));
    [0, 2, 4, 6].forEach((i, k) => steps[0].push('<text class="fig-txt" x="' + f(X(i)) + '" y="' + (base + 22) + '" text-anchor="middle"' + (nf(0) ? ' data-fade="' + (380 + k * 60) + '"' : "") + ">" + D.paces[i] + "</text>"));
    // b · times ten is your finish
    [0, 2, 4, 6].forEach((i, k) => {
      M.arrow(X(i), base - 9, X(i), base - 27, 80 + i).forEach((a, j) => steps[1].push(pm(a, "fig-ink", nf(1), 220, k * 120 + j * 160)));
      steps[1].push('<text class="fig-txt" x="' + f(X(i)) + '" y="' + (base - 33) + '" text-anchor="middle"' + (nf(1) ? ' data-fade="' + (k * 120 + 260) + '"' : "") + ">" + D.finishes[i] + "</text>");
    });
    // c · under 50 starts at 5:00, and where 5:30 lands
    steps[2].push('<rect class="fig-wash" x="' + f(X(0) - 16) + '" y="' + (base - 48) + '" width="' + f(X(2) - X(0) + 32) + '" height="58" rx="3"' + (nf(2) ? ' data-fade="0"' : "") + "/>");
    steps[2].push(pm(M.bracket(X(0) - 16, X(2) + 16, base - 54, 6, 71), "fig-ink", nf(2), 360, 120));
    steps[2].push('<text class="fig-lab" x="' + f((X(0) + X(2)) / 2) + '" y="' + (base - 62) + '" text-anchor="middle"' + (nf(2) ? ' data-fade="380"' : "") + ">under 50</text>");
    steps[2].push(pm(M.loop(X(4) - 18, base - 44, 36, 13, 73, { pad: 4, width: 1.15 }), "fig-ink", nf(2), 420, 520));
    steps[2].push('<text class="fig-lab" x="' + f(X(4) + 30) + '" y="' + (base - 62) + '" text-anchor="start"' + (nf(2) ? ' data-fade="820"' : "") + ">your plan</text>");
    steps[2].push(pm(M.mark(M.resample(M.cubic([X(4) + 28, base - 66], [X(4) + 16, base - 64], [X(4) + 8, base - 58], [X(4) + 6, base - 50], 12), 0.6), 1, [0.1, 0.2], 0, 77), "fig-ink", nf(2), 200, 900));
    let body = "";
    for (let s = 0; s < shown; s++) body += '<g class="fig-step">' + steps[s].join("") + "</g>";
    return '<svg viewBox="0 0 ' + W + " " + H + '" width="' + W + '" height="' + H + '" role="img" aria-label="Sketch: pace per km against finish time; under 50 minutes needs 5:00 a km or faster; 5:30 gives 55:00.">' + body + "</svg>";
  }
  function figureCard(t, fresh) {
    const s = t.drawing.step;
    const cap = s ? ["a", "b", "c"][s - 1] + "  " + N.drawing.steps[s - 1] : "";
    return '<div class="fig" data-fig="card"><div class="fig-drag" title="Drag into the note">' + figureSVG(s, fresh) + "</div>" +
      '<div class="fig-cap"><span><b>' + esc(cap.slice(0, 1)) + "</b>" + esc(cap.slice(1)) + '</span><span class="fig-ctl">' +
      '<button data-act="fig-prev" aria-label="Previous step">‹</button><button data-act="fig-next" aria-label="Next step">›</button><button data-act="fig-replay" aria-label="Replay">↻</button></span></div></div>';
  }
  function figureNote(inked) {
    return '<figure class="n-fig fig' + (inked ? " inked" : "") + '" style="margin:2px 0 16px">' +
      '<svg class="frame" aria-hidden="true" width="300" height="10" style="position:absolute;left:0;top:0;overflow:visible"></svg>' +
      figureSVG(3, 0).replace('width="256" height="128"', 'width="300" height="150"') +
      '<figcaption class="fig-cap"><span><b>Fig. 1</b>  ' + esc(N.drawing.title) + "</span></figcaption></figure>";
  }

  // ------------------------------------------------------------------ the sheet
  const sheets = [];
  let lifted = B.state === "hero-off";

  class Sheet {
    constructor(root, o) {
      this.root = root;
      this.o = Object.assign({ g: 56, tw: 560, mw: 320, head: "", headR: "", crops: true, title: true, below: false, interactive: false, seed: 0 }, o);
      this.m = o.model || baseModel();
      this.fresh = new Set();
      root.classList.add("sheet");
      sheets.push(this);
      if (this.o.interactive) this.bind();
      this.render();
    }
    t(id) { return this.m.threads.find((x) => x.id === +id); }
    openCount() { return this.m.threads.filter((t) => OPEN.includes(t.status)).length; }
    paraState(i) {
      const m = this.m;
      if (m.reading.has(i)) return "reading";
      const ts = m.threads.filter((t) => t.para === i);
      if (ts.some((t) => t.status === "rechecking")) return "reading";
      if (ts.some((t) => t.status === "open" || t.status === "waiting")) return "query";
      if (m.read.has(i) || ts.length) return "read";
      return "unread";
    }

    // ---------- markup ----------
    paraHTML(i) {
      const m = this.m, txt = paraText(i, m), spans = [];
      for (const t of m.threads) if (t.para === i) {
        const q = quoteOf(t, m), at = txt.indexOf(q);
        if (at >= 0) spans.push({ at, end: at + q.length, t });
      }
      if (m.selection && m.selection.para === i) {
        const at = txt.indexOf(m.selection.quote);
        if (at >= 0) spans.push({ at, end: at + m.selection.quote.length, sel: true });
      }
      spans.sort((a, b) => a.at - b.at);
      let out = "", pos = 0;
      for (const s of spans) {
        if (s.at < pos) continue;
        out += esc(txt.slice(pos, s.at));
        const q = txt.slice(s.at, s.end);
        let inner = esc(q);
        if (s.t && s.t.id === 1) {
          const num = m.fixed ? T1.fixTo : T1.fixFrom, k = q.indexOf(num);
          if (k >= 0) inner = esc(q.slice(0, k)) + '<span class="fixnum">' + num + "</span>" + esc(q.slice(k + num.length));
        }
        out += s.sel ? '<span class="sel">' + inner + "</span>" : '<span class="anchor" data-t="' + s.t.id + '">' + inner + "</span>";
        pos = s.end;
      }
      return out + esc(txt.slice(pos));
    }
    queryHTML(t) {
      const m = this.m, src = SRC[t.id], fresh = this.fresh;
      const num = "AQ" + t.id, kind = t.kind === "correction" ? "Fix" : "Query";
      if (t.status === "dismissed") return '<div class="q-stet" data-t="' + t.id + '"' + (fresh.has("stet-" + t.id) ? ' data-fade="200"' : "") + ">stet</div>";
      const chip = CLOSED.includes(t.status) && !t.open;
      const compact = (t.status === "waiting" || (m.quiet && OPEN.includes(t.status))) && !t.open;
      let h = '<div class="q' + (chip ? " chip" : "") + (compact ? " waiting" : "") + '" data-t="' + t.id + '" tabindex="0" role="button" aria-expanded="' + t.open + '" aria-label="' + esc(this.aria(t)) + '">';
      h += '<div class="q-tag"><span class="q-num">' + num + "</span>";
      if (!compact) h += '<span class="q-kind">· ' + kind + "</span>";
      if (t.status === "resolved") h += '<span class="q-ok">OK</span>' + tickSVG("mk-graphite", fresh.has("tick-" + t.id));
      if (t.status === "settled") h += tickSVG("mk-pencil", fresh.has("tick-" + t.id)) + '<span class="q-settled">settled</span>';
      if (t.status === "worked") h += tickSVG("mk-pencil", fresh.has("tick-" + t.id)) + '<span class="q-settled">worked out</span>';
      h += "</div>";
      if (compact) return h + "</div>";
      if (t.status === "settled") h += '<p class="q-text"' + (fresh.has("tick-" + t.id) ? ' data-fade="120"' : "") + ">" + esc(T1.settle) + "</p>";
      else if (t.status === "worked" && !t.open) h += '<p class="q-text">' + esc(src.answer) + "</p>";
      else h += '<p class="q-text' + (t.status === "rechecking" ? " dim" : "") + '">' + esc(src.text) + "</p>";
      if (t.status === "rechecking") h += '<p class="q-note">Reading your change <span class="q-dots" aria-hidden="true"><span></span><span></span><span></span></span></p>';
      if (t.open) {
        for (const msg of t.msgs) h += '<p class="q-msg ' + (msg.who === "you" ? "you" : "ink") + '">' + esc(msg.text) + "</p>";
        if (t.thinking) h += '<p class="q-msg q-dots" aria-label="Inkling is writing"><span></span><span></span><span></span></p>';
        if (t.drawing) h += figureCard(t, this.fresh.has("fig") ? t.drawing.step : 0);
        if (this.o.interactive && !t.busy) {
          h += '<div class="q-actions">';
          if (OPEN.includes(t.status)) {
            if (t.id === 1 && !m.fixed) h += '<button class="btn go" data-act="fix">Fix 5:30 for me</button>';
            if (t.id === 1 && !t.drawing && !m.figure) h += '<button class="btn" data-act="draw">Draw it</button>';
            if (t.id === 1 && t.drawing && !m.figure) h += '<button class="btn" data-act="use">Use in note</button>';
            h += '<button class="btn" data-act="resolve" title="Mark this query answered.">Resolve<span class="k">⌘↵</span></button>';
            h += '<button class="btn" data-act="dismiss" title="Leave your words as they are. Editors call this stet.">Dismiss<span class="k">⌘⌫</span></button>';
          } else {
            h += '<button class="btn" data-act="reopen">Reopen</button>';
          }
          h += "</div>";
          if (OPEN.includes(t.status) && t.status !== "rechecking") {
            const ph = t.id === 2 ? src.reply : "Reply…";
            h += '<form class="q-reply" data-act="send"><input type="text" aria-label="Reply to ' + num + '" placeholder="' + esc(ph) + '" value="' + (t.id === 2 && !t.msgs.length ? esc(src.reply) : "") + '"><button class="btn" type="submit">Reply</button></form>';
          }
        } else if (!this.o.interactive && OPEN.includes(t.status) && t.status !== "rechecking") {
          h += '<div class="q-actions"><span class="btn">Reply</span><span class="btn">Resolve</span><span class="btn">Dismiss</span></div>';
        }
      }
      return h + "</div>";
    }
    aria(t) {
      const n = this.m.threads.length, kind = t.kind === "correction" ? "a fix" : "a question";
      return "Query " + t.id + " of " + n + ", " + kind + ", about “" + quoteOf(t, this.m).slice(0, 40) + "”. " +
        ({ open: "Open.", waiting: "Waiting.", rechecking: "Re-reading.", settled: "Settled by Inkling.", resolved: "Resolved by you.", worked: "Worked out." }[t.status] || "");
    }
    render() {
      const o = this.o, m = this.m, r = this.root;
      r.classList.toggle("below", o.below);
      r.style.setProperty("--g", o.g + "px");
      r.style.setProperty("--tw", o.tw + "px");
      r.style.setProperty("--mw", o.mw + "px");
      let text = "";
      if (m.firstRun) text += '<div class="firstrun"><p class="label">First run</p><p class="n-ph" style="margin-bottom:12px">' + esc(N.firstRun.prompt) + "</p></div>";
      if (o.title && m.title) text += '<h3 class="n-title">' + esc(m.title) + "</h3>";
      if (m.placeholder) text += '<p class="n-ph">' + esc(m.placeholder) + "</p>";
      for (const i of m.paras) {
        text += '<p class="n-p" data-p="' + i + '">' + (m.firstRun ? '<span class="anchor" data-t="9">' + esc(N.firstRun.example) + "</span>" : this.paraHTML(i)) + "</p>";
        if (m.figure && i === T1.para) text += figureNote(!this.fresh.has("ink"));
        if (o.below) for (const t of m.threads) if (t.para === i) text += this.queryHTML(t);
      }
      let margin = "";
      if (!o.below) {
        for (const t of m.threads) if (m.paras.includes(t.para)) margin += this.queryHTML(t);
        if (m.firstRun) margin += '<div class="q" data-t="9" tabindex="0" role="button" aria-expanded="false"><div class="q-tag"><span class="q-num">AQ1</span><span class="q-kind">· Query</span></div><p class="q-text">' + esc(N.firstRun.question) + "</p></div>";
        if (m.room) m.room.forEach((a, k) => {
          margin += '<div class="q room" data-room="' + k + '" data-pencil="' + a.pencil + '"' + (this.fresh.has("room-" + k) ? ' data-fade="60"' : "") + '><div class="q-tag"><span class="q-who"><i></i>' + esc(a.mind) + '</span></div><p class="q-text">' + esc(a.text) + "</p>" +
            (k === m.room.length - 1 && this.o.interactive && m.room.length === 3 ? '<div class="q-actions"><button class="btn" data-act="room-close">Done</button></div>' : "") + "</div>";
        });
      }
      r.innerHTML =
        (o.crops ? '<svg class="s-crops" aria-hidden="true"></svg>' : "") +
        '<svg class="mk mk-under" aria-hidden="true"></svg>' +
        (o.head ? '<div class="s-head"><span>' + esc(o.head) + "</span><span>" + esc(o.headR) + "</span></div>" : "") +
        (m.banner ? '<div class="s-banner" role="status"><span>' + esc(m.banner.text) + "</span>" + (m.banner.retry ? '<span class="btn">Try again</span>' : "") + "</div>" : "") +
        '<div class="s-body"><div class="s-gutter" aria-hidden="true"></div><div class="s-text">' + text + "</div>" + (o.below ? "" : '<div class="s-margin">' + margin + "</div>") + "</div>" +
        '<svg class="mk mk-over" aria-hidden="true"></svg>' +
        (m.roomTools ? '<div class="room-tools" aria-hidden="true"><span>Ask</span><span class="on">Ask the room</span><span>Riddle</span><span>Draw</span></div>' : "") +
        (m.toast ? '<div class="s-toast" role="status"><span>' + esc(m.toast.text) + '</span><button data-act="undo">Undo</button><i></i></div>' : "");
      r.classList.toggle("lifted", lifted && !o.noLift);
      r.classList.toggle("has-toast", !!m.toast);
      this.layout();
      if (m.toast && m.toast.fresh) {
        const bar = $(".s-toast i", r);
        if (bar) bar.animate([{ transform: "scaleX(1)" }, { transform: "scaleX(0)" }], { duration: Math.max(0, m.toast.until - Date.now()), fill: "forwards" });
        m.toast.fresh = false;
      }
      this.fresh.clear();
      if (this.onRender) this.onRender();
    }

    // ---------- layout: margin placement, then every mark from real text positions ----------
    layout() {
      const r = this.root, o = this.o, m = this.m, fresh = this.fresh;
      const R = r.getBoundingClientRect(), k = R.width / (r.offsetWidth || 1) || 1;
      const rel = (b) => ({ x: (b.left - R.left) / k, y: (b.top - R.top) / k, w: b.width / k, h: b.height / k, r: (b.right - R.left) / k, b: (b.bottom - R.top) / k });
      const seed = o.seed;
      const textEl = $(".s-text", r), textRight = textEl ? rel(textEl.getBoundingClientRect()).r + 6 : null;
      // one rect per line: nested spans (the number a fix retypes) split an anchor into
      // overlapping fragments, so merge everything that shares a line
      const lines = (rects) => {
        const out = [];
        rects.map(rel).filter((q) => q.w > 0.5).sort((p, q) => p.y - q.y || p.x - q.x).forEach((q) => {
          const l = out.find((o) => Math.abs(o.y - q.y) < 3 || Math.abs(o.b - q.b) < 3);
          if (l) {
            const x = Math.min(l.x, q.x), rr = Math.max(l.r, q.r), y = Math.min(l.y, q.y), bb = Math.max(l.b, q.b);
            Object.assign(l, { x, r: rr, y, b: bb, w: rr - x, h: bb - y });
          } else out.push(Object.assign({}, q));
        });
        return out;
      };
      const anchors = {};
      $$(".anchor", r).forEach((sp) => { anchors[sp.dataset.t] = lines([...sp.getClientRects()]); });
      const selEl = $(".sel", r);
      const sel = selEl ? lines([...selEl.getClientRects()]) : null;

      // 1. margin: queries sit level with the last line of their words, top down, never overlapping
      const margin = $(".s-margin", r);
      if (margin) {
        const M0 = rel(margin.getBoundingClientRect());
        const items = $$(".q, .q-stet", margin).map((el, n) => {
          let want = 0;
          if (el.dataset.room != null) want = sel && sel.length ? sel[0].y - M0.y - 4 : 0;
          else {
            const rs = anchors[el.dataset.t];
            if (rs && rs.length) want = rs[rs.length - 1].y - M0.y - 4;
          }
          return { el, want, n };
        });
        items.sort((a, b) => a.want - b.want || a.n - b.n);
        let y = 0;
        for (const it of items) {
          const top = Math.max(it.want, y);
          it.el.style.top = top + "px";
          y = top + it.el.offsetHeight + (it.el.classList.contains("q-stet") ? 16 : 30);
        }
        margin.style.minHeight = Math.max(0, y - 20) + "px";
      }
      if (this.o.fitBox) { // a sheet inside .fit: tell it how tall the sheet now is
        this.o.fitBox.dataset.h = Math.ceil(r.offsetHeight + 2);
      }

      // 2. gutter: paragraph numbers with their read state
      const gutter = $(".s-gutter", r), G = rel(gutter.getBoundingClientRect());
      let gutterHTML = "";
      const pnum = [];
      $$(".n-p", r).forEach((p) => {
        const i = +p.dataset.p, st = m.firstRun ? "unread" : this.paraState(i);
        const top = rel(p.getBoundingClientRect()).y - G.y;
        gutterHTML += '<span class="pn' + (st === "reading" ? " reading" : "") + '" style="top:' + f(top) + 'px">' + (i + 1) + "</span>";
        pnum.push({ i, st });
      });
      gutter.innerHTML = gutterHTML;

      // 3. marks
      const under = [], over = [];
      const P = (mk, cls, key, ms, delay) => pm(mk, cls, fresh.has(key), ms, delay);
      $$(".pn", gutter).forEach((el, n) => {
        const { i, st } = pnum[n], b = rel(el.getBoundingClientRect());
        const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
        if (st === "read") over.push('<g class="pl">' + P(M.tick(b.x - 13, cy - 3.2, 0.6, 300 + i + seed), "mk-pencil", "read-" + i, 180) + "</g>");
        if (st === "query") over.push('<g class="pl">' + P(M.ring(cx, cy, 7.4, 310 + i + seed), "mk-pencil", "ring-" + i, 260) + "</g>");
      });
      const qEls = {};
      $$(".q[data-t], .q-stet", r).forEach((el) => { qEls[el.dataset.t] = el; });
      const threads = m.firstRun ? [{ id: 9, status: "open", open: false }] : m.threads;
      for (const t of threads) {
        const rs = anchors[t.id];
        if (!rs || !rs.length) continue;
        const qe = qEls[t.id], last = rs[rs.length - 1];
        const s = 1000 * t.id + seed;
        if (t.status === "dismissed") {
          const dots = [];
          rs.forEach((q, n) => M.stetDots(q.x, q.r, q.b + 1.5, s + n).forEach((d) => dots.push('<circle class="mk-graphite" cx="' + f(d[0]) + '" cy="' + f(d[1]) + '" r="' + f(d[2]) + '"/>')));
          over.push('<g class="pl"' + (fresh.has("stet-" + t.id) ? ' data-fade="0"' : "") + ">" + dots.join("") + "</g>");
          continue;
        }
        const live = OPEN.includes(t.status);
        if (live && !(m.quiet && !t.open)) {
          const hot = t.open ? " hot" : "";
          under.push('<g class="pl"' + (fresh.has("wash-" + t.id) ? ' data-fade="0"' : "") + ">" + rs.map((q) => '<rect class="mk-wash' + hot + '" data-t="' + t.id + '" x="' + f(q.x - 1.5) + '" y="' + f(q.y + q.h * 0.1) + '" width="' + f(q.w + 3) + '" height="' + f(q.h * 0.84) + '" rx="2"/>').join("") + "</g>");
        }
        const base = last.b - last.h * 0.21;
        const car = M.caret(last.r + 2.5, base, 1, s + 1);
        const g = [];
        g.push(P(car, "mk-soft", "caret-" + t.id, 160));
        if (qe) {
          const qb = rel(qe.getBoundingClientRect());
          let tgt;
          if (o.below) {
            // on a phone the query sits under its paragraph: drop straight down from the caret to the loop
            const from = car.foot;
            tgt = [Math.max(qb.x + 24, Math.min(from[0] - 2, qb.r - 24)), qb.y - 10];
            g.push(pm(M.mark(M.resample(M.cubic(from, [from[0] + 2, from[1] + 6], [tgt[0] + 2, tgt[1] - 6], tgt, 16), 0.8), (u) => 1.1 * (1 - 0.25 * u), [0.04, 0.22], 0.08, s + 2), "mk-soft", fresh.has("guide-" + t.id), 200, 140));
          } else {
            tgt = [qb.x - 11, qb.y + 7];
            g.push(P(M.guide(car.foot, tgt, s + 2, textRight), "mk-soft", "guide-" + t.id, 300, 140));
          }
          if (!qe.classList.contains("q-stet")) {
            const loop = M.loop(qb.x, qb.y, qb.w, qb.h, s + 3, { pad: qe.classList.contains("waiting") ? 6 : 9, width: 1.3 });
            g.push(P(loop, "mk-pencil", "loop-" + t.id, 380, 300));
          }
        }
        over.push('<g class="pl' + (live ? "" : " faded") + '">' + g.join("") + "</g>");
      }
      // the room: your selection in graphite, three pencils in the margin
      if (sel && sel.length) {
        under.push('<g class="pl">' + sel.map((q) => '<rect class="mk-gwash" x="' + f(q.x - 1.5) + '" y="' + f(q.y + q.h * 0.1) + '" width="' + f(q.w + 3) + '" height="' + f(q.h * 0.84) + '" rx="2"/>').join("") + "</g>");
        const tools = $(".room-tools", r);
        if (tools) { tools.style.left = f(sel[0].x) + "px"; tools.style.top = f(sel[0].y - 40) + "px"; }
      }
      if (m.room && margin) {
        $$(".q.room", margin).forEach((el) => {
          const qb = rel(el.getBoundingClientRect()), kk = +el.dataset.room, a = m.room[kk];
          over.push('<g class="pl">' + P(M.loop(qb.x, qb.y, qb.w, qb.h, 700 + kk + seed, { pad: 9, width: 1.3 }), a.pencil === "inkling" ? "mk-pencil" : "mk-" + a.pencil, "room-" + kk, 380, 80) + "</g>");
        });
        if (sel && sel.length) {
          const l = sel[sel.length - 1], first = $(".q.room", margin);
          if (first) {
            const qb = rel(first.getBoundingClientRect());
            const car = M.caret(l.r + 2.5, l.b - l.h * 0.21, 1, 777 + seed);
            over.push('<g class="pl">' + P(car, "mk-graphite", "room-0", 160) + P(M.guide(car.foot, [qb.x - 11, qb.y + 7], 778 + seed, textRight), "mk-graphite", "room-0", 300, 120) + "</g>");
          }
        }
      }
      $(".mk-under", r).innerHTML = under.join("");
      $(".mk-over", r).innerHTML = over.join("");

      // 4. crop marks in the bleed, outside the trim
      const crops = $(".s-crops", r);
      if (crops) {
        const w = r.offsetWidth, hh = r.offsetHeight, a = 16, L = 12;
        crops.innerHTML = [[a, a, -1, -1], [w - a, a, 1, -1], [a, hh - a, -1, 1], [w - a, hh - a, 1, 1]].map(([x, y, dx, dy]) =>
          '<path d="M' + (x + dx * 3) + " " + y + " h" + dx * L + " M" + x + " " + (y + dy * 3) + " v" + dy * L + '"/>').join("");
      }
      // 5. a note figure gets its frame
      const nf = $(".n-fig", r);
      if (nf) {
        const fr = $(".frame", nf), sv = $("svg[role=img]", nf);
        if (fr && sv) {
          const w = sv.getBoundingClientRect().width / k, hh = sv.getBoundingClientRect().height / k + 12, c = 10;
          fr.setAttribute("width", f(w)); fr.setAttribute("height", f(hh));
          fr.innerHTML = '<path d="M0 ' + c + " V0 H" + c + " M" + (w - c) + " 0 H" + w + " V" + c + " M0 " + (hh - c) + " V" + hh + " H" + c + " M" + (w - c) + " " + hh + " H" + w + " V" + (hh - c) + '"/>';
        }
      }
      animateFresh(r);
    }

    // ---------- interaction (the app only) ----------
    bind() {
      const r = this.root;
      r.addEventListener("click", (e) => {
        if (lifted) return;
        const act = e.target.closest("[data-act]");
        if (act && act.tagName !== "FORM") { e.preventDefault(); this.act(act.dataset.act, act); return; }
        if (e.target.closest("input, form, .fig")) return;
        const a = e.target.closest(".anchor"), q = e.target.closest(".q[data-t]");
        if (a) this.toggle(a.dataset.t);
        else if (q) this.toggle(q.dataset.t);
      });
      r.addEventListener("submit", (e) => {
        e.preventDefault();
        const form = e.target, q = form.closest(".q[data-t]");
        if (q) this.reply(+q.dataset.t, $("input", form).value.trim());
      });
      r.addEventListener("keydown", (e) => {
        const q = e.target.closest && e.target.closest(".q[data-t]");
        if (!q || e.target.tagName === "INPUT") return;
        if (e.key === "Enter" || e.key === " ") { e.preventDefault(); this.toggle(q.dataset.t); }
        if ((e.key === "r" || e.key === "R") && !e.metaKey) { const t = this.t(q.dataset.t); if (!t.open) this.toggle(t.id); setTimeout(() => { const i = $('.q[data-t="' + t.id + '"] input', r); if (i) i.focus(); }, 30); e.preventDefault(); }
      });
      r.addEventListener("mouseover", (e) => {
        const a = e.target.closest(".anchor, .q[data-t]");
        $$(".mk-wash", r).forEach((w) => w.classList.toggle("hot", !!a && w.dataset.t === a.dataset.t || (w.classList.contains("hot") && this.t(w.dataset.t) && this.t(w.dataset.t).open)));
      });
      // drag the sketch into the note
      r.addEventListener("pointerdown", (e) => {
        const d = e.target.closest(".fig-drag");
        if (!d || lifted || e.button !== 0) return;
        e.preventDefault();
        const svg = $("svg", d), b = svg.getBoundingClientRect();
        const ghost = svg.cloneNode(true);
        ghost.classList.add("fig-ghost");
        ghost.style.width = b.width + "px"; ghost.style.height = b.height + "px";
        ghost.style.left = b.left + "px"; ghost.style.top = b.top + "px";
        document.body.appendChild(ghost);
        const sx = e.clientX, sy = e.clientY;
        const move = (ev) => { ghost.style.transform = "translate(" + (ev.clientX - sx) + "px," + (ev.clientY - sy) + "px)"; };
        const up = (ev) => {
          removeEventListener("pointermove", move); removeEventListener("pointerup", up);
          const tb = $(".s-text", r).getBoundingClientRect();
          const over = ev.clientX >= tb.left && ev.clientX <= tb.right && ev.clientY >= tb.top && ev.clientY <= tb.bottom;
          if (over) { this.useFigure(ghost.getBoundingClientRect()); ghost.remove(); }
          else ghost.animate([{ transform: ghost.style.transform }, { transform: "none" }], { duration: 200, easing: "cubic-bezier(.2,.7,.2,1)" }).onfinish = () => ghost.remove();
        };
        addEventListener("pointermove", move); addEventListener("pointerup", up);
      });
    }
    act(name, el) {
      const q = el.closest(".q[data-t]"), id = q ? +q.dataset.t : 1;
      ({
        fix: () => this.fix(),
        draw: () => this.draw(),
        use: () => this.useFigure(),
        resolve: () => this.resolve(id),
        dismiss: () => this.dismiss(id),
        reopen: () => { const t = this.t(id); t.status = "open"; t.open = true; this.fresh.add("loop-" + id); this.fresh.add("wash-" + id); this.render(); this.changed(); },
        undo: () => this.undo(),
        "room-close": () => this.closeRoom(),
        "fig-prev": () => this.figStep(-1),
        "fig-next": () => this.figStep(1),
        "fig-replay": () => this.draw(true),
      }[name] || (() => {}))();
    }
    changed() { if (this.onChange) this.onChange(); }
    focusQ(id) { const el = $('.q[data-t="' + id + '"]', this.root); if (el) el.focus({ preventScroll: true }); }
    toggle(id) {
      const t = this.t(id);
      if (!t || t.busy) return;
      const was = t.open;
      this.m.threads.forEach((x) => { x.open = false; });
      t.open = !was;
      if (t.open && OPEN.includes(t.status)) { this.fresh.add("loop-" + id); }
      if (t.open && this.m.quiet) this.closeRoom(true);
      this.render();
      if (t.open) this.focusQ(id);
    }
    resolve(id) {
      const t = this.t(id);
      if (!t || !OPEN.includes(t.status) || t.busy) return;
      t.status = "resolved"; t.open = false;
      this.fresh.add("tick-" + id);
      this.render(); B.sound("tick"); this.changed();
    }
    dismiss(id) {
      const t = this.t(id);
      if (!t || !OPEN.includes(t.status) || t.busy) return;
      const prev = t.status;
      t.status = "dismissed"; t.open = false;
      this.fresh.add("stet-" + id);
      this.m.toast = { text: "Dismissed AQ" + id, id, prev, until: Date.now() + 8000, fresh: true };
      clearTimeout(this.toastT);
      this.toastT = setTimeout(() => { this.m.toast = null; this.render(); }, B.still ? 8000 : 8000);
      this.render(); B.sound("tick"); this.changed();
    }
    undo() {
      const ts = this.m.toast;
      if (!ts) return;
      clearTimeout(this.toastT);
      const t = this.t(ts.id);
      t.status = ts.prev; t.open = false;
      this.m.toast = null;
      this.fresh.add("loop-" + ts.id); this.fresh.add("caret-" + ts.id); this.fresh.add("guide-" + ts.id); this.fresh.add("wash-" + ts.id);
      this.render(); this.changed();
    }
    async fix() {
      const m = this.m, t = this.t(1);
      if (m.fixed || this.busy || !OPEN.includes(t.status)) return;
      this.busy = true; t.busy = true;
      if (m.quiet) this.closeRoom(true);
      const span = $(".anchor[data-t='1'] .fixnum", this.root);
      if (span) {
        span.classList.add("typing");
        await wait(380);
        const out = T1.fixTo;
        for (let i = 1; i <= out.length; i++) { span.textContent = out.slice(0, i); this.layout(); await wait(110); }
        await wait(160);
        span.classList.remove("typing");
      }
      m.fixed = true; t.status = "rechecking"; t.open = false;
      this.render(); this.changed();
      await wait(1600);
      t.status = "settled"; t.busy = false; this.busy = false;
      m.read.add(1);
      this.fresh.add("tick-1");
      this.render(); B.sound("tick"); this.changed();
    }
    async reply(id, text) {
      const t = this.t(id), src = SRC[id];
      if (!t || t.busy || !OPEN.includes(t.status)) return;
      text = text || src.reply || "";
      if (!text) return;
      t.busy = true; t.open = true;
      t.msgs.push({ who: "you", text });
      t.thinking = true;
      this.render();
      await wait(900);
      t.thinking = false;
      const answer = src.answer || "Thanks. That answers it.";
      const msg = { who: "ink", text: "" };
      t.msgs.push(msg);
      const words = answer.split(" ");
      for (let i = 0; i < words.length; i++) { msg.text += (i ? " " : "") + words[i]; this.render(); await wait(45); }
      await wait(900);
      t.status = "worked"; t.open = false; t.busy = false;
      this.fresh.add("tick-" + id);
      this.render(); B.sound("tick"); this.changed();
    }
    async draw(replay) {
      const t = this.t(1), m = this.m;
      if (m.figure || this.drawing) return;
      this.drawing = true;
      if (m.quiet) this.closeRoom(true);
      this.m.threads.forEach((x) => { x.open = false; });
      t.open = true;
      t.drawing = { step: 0 };
      for (let s = 1; s <= 3; s++) {
        if (!t.drawing) break;
        t.drawing.step = s;
        this.fresh.add("fig");
        this.render();
        await wait(1350);
      }
      this.drawing = false;
    }
    figStep(d) {
      const t = this.t(1);
      if (!t.drawing || this.drawing) return;
      const s = Math.min(3, Math.max(1, t.drawing.step + d));
      if (s === t.drawing.step) return;
      t.drawing.step = s;
      if (d > 0) this.fresh.add("fig");
      this.render();
    }
    async useFigure(fromRect) {
      const m = this.m, t = this.t(1);
      if (m.figure) return;
      if (!fromRect) {
        if (!t.drawing) { t.open = true; t.drawing = { step: 3 }; this.render(); }
        const cs = $('.fig[data-fig="card"] svg', this.root);
        fromRect = cs && cs.getBoundingClientRect();
      }
      this.drawing = false;
      m.figure = true; t.drawing = null;
      this.fresh.add("ink");
      this.render();
      const fig = $(".n-fig", this.root), sv = fig && $("svg[role=img]", fig);
      if (sv && fromRect && !B.still) {
        const to = sv.getBoundingClientRect();
        const dx = fromRect.left - to.left, dy = fromRect.top - to.top, sc = fromRect.width / to.width;
        sv.animate([{ transform: "translate(" + dx + "px," + dy + "px) scale(" + sc + ")", transformOrigin: "0 0" }, { transform: "none", transformOrigin: "0 0" }],
          { duration: 420, easing: "cubic-bezier(.2,.7,.2,1)" });
      }
      // paint it blue first, then let the colour transition carry it to ink
      if (fig) { void fig.offsetWidth; setTimeout(() => fig.classList.add("inked"), 60); }
      B.sound("paper");
      this.changed();
    }
    async room() {
      const m = this.m;
      if (m.room || this.busy) return;
      this.m.threads.forEach((x) => { x.open = false; });
      m.selection = { para: 1, quote: N.room.quote };
      m.roomTools = true; m.quiet = true;
      this.render();
      await wait(900);
      m.roomTools = false;
      m.room = [];
      const pencils = ["inkling", "skeptic", "tutor"];
      for (let k = 0; k < N.room.answers.length; k++) {
        const a = N.room.answers[k];
        m.room.push({ mind: a.mind, text: a.text, pencil: pencils[k] });
        this.fresh.add("room-" + k);
        this.render();
        await wait(420);
      }
      this.render();
    }
    closeRoom(silent) {
      const m = this.m;
      m.room = null; m.selection = null; m.roomTools = false; m.quiet = false;
      if (!silent) this.render();
    }
    jump(dir) {
      const open = this.m.threads.filter((t) => OPEN.includes(t.status)).map((t) => t.id);
      if (!open.length) return;
      const cur = document.activeElement && document.activeElement.closest ? document.activeElement.closest(".q[data-t]") : null;
      const at = cur ? +cur.dataset.t : (dir > 0 ? 0 : 99);
      let next = dir > 0 ? open.find((id) => id > at) : open.slice().reverse().find((id) => id < at);
      if (next == null) next = dir > 0 ? open[0] : open[open.length - 1];
      this.fresh.add("loop-" + next);
      this.render();
      this.focusQ(next);
    }
  }

  function tickSVG(cls, fresh) {
    const t = M.tick(2, 5, 0.95, 13);
    return '<svg class="q-tick" viewBox="0 0 16 14" width="16" height="14" aria-hidden="true" style="overflow:visible;align-self:center">' + pm(t, cls, fresh, 180) + "</svg>";
  }

  // ------------------------------------------------------------------ the pencil (global)
  function setPencil(down, quiet) {
    lifted = !down;
    $$("[data-pencil]").forEach((b) => {
      b.setAttribute("aria-checked", String(down));
      const st = $(".state", b);
      if (st) st.textContent = down ? "on" : "off";
    });
    sheets.forEach((s) => { if (!s.o.noLift) s.root.classList.toggle("lifted", lifted); });
    if (!quiet) B.sound("paper");
  }
  const togglePencil = () => setPencil(lifted);

  // ------------------------------------------------------------------ hero
  let hero = null, heroBelow = null;
  function buildHero() {
    const fit = $("#hero-fit"), below = $("#hero-below"), narrow = fit.parentElement.clientWidth < 600;
    fit.style.display = narrow ? "none" : "";
    below.style.display = narrow ? "" : "none";
    const model = () => { const m = baseModel(); m.paras = [0, 1, 2]; m.title = null; return m; };
    if (!narrow && !hero) {
      hero = new Sheet($("#hero-sheet"), { g: 48, tw: 420, mw: 244, head: "Galley 1 · " + N.title, headR: "6 Nov", model: model(), seed: 5, fitBox: fit });
      hero.fresh.add("loop-1"); hero.fresh.add("loop-2");
    }
    if (narrow && !heroBelow) {
      below.innerHTML = '<div id="hero-sheet-below"></div>';
      heroBelow = new Sheet($("#hero-sheet-below"), { g: 40, tw: 0, mw: 0, below: true, crops: false, head: "", model: model(), seed: 5 });
    }
    B.fit();
  }

  // ------------------------------------------------------------------ the app
  let app = null;
  function sideList() {
    const n = app ? app.openCount() : 2;
    const f = N.sidebar.folders;
    const li = (name, cnt, on) => '<li' + (on ? ' class="on"' : "") + "><span>" + esc(name) + "</span>" + (cnt != null ? '<span class="cnt' + (cnt ? "" : " none") + '">' + (cnt ? cnt : "✓") + "</span>" : "") + "</li>";
    let h = "";
    f.forEach((fo) => {
      h += "<h4>" + esc(fo.name) + "</h4><ul>";
      fo.notes.forEach((nm) => { h += li(nm, nm === N.sidebar.current ? n : nm === "Why the bread didn’t rise" ? 1 : null, nm === N.sidebar.current); });
      h += "</ul>";
    });
    h += "<h4>Notes</h4><ul>" + N.sidebar.loose.map((nm) => li(nm, null, false)).join("") + "</ul>";
    $("#side-list").innerHTML = h;
  }
  function appChanged() {
    const n = app.openCount(), oc = $("#open-count");
    oc.innerHTML = n ? '<span class="n">' + n + "</span> open" : "None open";
    oc.classList.toggle("none", !n);
    sideList();
    updateTry();
  }
  function updateTry() {
    if (!app) return;
    const m = app.m, t1 = app.t(1), t2 = app.t(2);
    const dis = {
      open1: !OPEN.includes(t1.status),
      fix: m.fixed || !OPEN.includes(t1.status),
      draw: m.figure || !OPEN.includes(t1.status),
      use: m.figure || !OPEN.includes(t1.status),
      reply2: !OPEN.includes(t2.status),
      dismiss2: !OPEN.includes(t2.status),
      room: !!m.room,
    };
    $$("#try [data-do]").forEach((b) => { b.disabled = !!dis[b.dataset.do]; });
  }
  function freshApp(state) {
    const m = baseModel();
    if (app) { app.m = m; clearTimeout(app.toastT); app.busy = false; app.drawing = false; }
    applyState(m, state || "");
    return m;
  }
  /** Put a model into a named state, final frame, no animation. */
  function applyState(m, s) {
    const t1 = m.threads[0], t2 = m.threads[1];
    switch (s) {
      case "open": t1.open = true; break;
      case "reading": t2.status = "open"; m.read.delete(3); m.reading.add(3); break;
      case "waiting": t2.status = "waiting"; break;
      case "fixing": m.fixed = true; t1.status = "rechecking"; break;
      case "settled": m.fixed = true; t1.status = "settled"; m.read.add(1); break;
      case "resolved": t2.status = "resolved"; break;
      case "dismissed": t2.status = "dismissed"; m.toast = { text: "Dismissed AQ2", id: 2, prev: "open", until: Date.now() + 8000, fresh: false }; break;
      case "drawing": t1.open = true; t1.drawing = { step: 3 }; break;
      case "room": m.selection = { para: 1, quote: N.room.quote }; m.quiet = true; m.room = N.room.answers.map((a, k) => ({ mind: a.mind, text: a.text, pencil: ["inkling", "skeptic", "tutor"][k] })); break;
      case "empty": m.title = "Untitled"; m.paras = []; m.threads = []; m.placeholder = "Start writing. Inkling reads along and puts its questions in the margin, in blue pencil."; break;
      case "offline": m.banner = { text: "Inkling can’t reach Claude right now. Keep writing; it will read what it missed when it’s back.", retry: true }; t1.status = "open"; m.reading.clear(); break;
      case "limit": m.banner = { text: "Inkling has used its reading for now. It can read again at " + N.limitResets + ". Your notes are fine." }; break;
      case "first-run": m.title = null; m.paras = [0]; m.threads = []; m.firstRun = true; m.read = new Set(); break;
      case "figure": m.figure = true; break;
    }
    return m;
  }
  function buildApp() {
    const m = freshApp(B.state);
    app = new Sheet($("#app-sheet"), { g: 56, tw: 560, mw: 320, head: "Galley 1 · " + N.title, headR: "6 Nov · proof 1", interactive: true, model: m, seed: 0 });
    app.onChange = appChanged;
    if (B.state === "chat") $("#chat").classList.add("on");
    appChanged();
    $("#open-count").addEventListener("click", () => app.jump(1));
    $("#chat-btn").addEventListener("click", () => $("#chat").classList.toggle("on"));
    $("#chat-close").addEventListener("click", () => $("#chat").classList.remove("on"));
    $("#try").addEventListener("click", (e) => {
      const b = e.target.closest("[data-do]");
      if (!b || b.disabled) return;
      if (lifted && b.dataset.do !== "pencil" && b.dataset.do !== "reset") setPencil(true, true);
      ({
        open1: () => { if (!app.t(1).open) app.toggle(1); },
        fix: () => app.fix(),
        draw: () => app.draw(),
        use: () => app.useFigure(),
        reply2: () => { const t = app.t(2); if (!t.open) app.toggle(2); app.reply(2); },
        dismiss2: () => app.dismiss(2),
        room: () => app.room(),
        chat: () => $("#chat").classList.toggle("on"),
        pencil: () => togglePencil(),
        reset: () => { app.m = freshApp(""); $("#chat").classList.remove("on"); if (lifted) setPencil(true, true); app.render(); appChanged(); },
      }[b.dataset.do])();
    });
  }

  // ------------------------------------------------------------------ keys
  addEventListener("keydown", (e) => {
    const inInput = e.target && (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA");
    if (e.metaKey && e.shiftKey && (e.key === "." || e.key === ">" || e.code === "Period")) { e.preventDefault(); togglePencil(); return; }
    if (!app) return;
    if (e.metaKey && e.altKey && (e.key === "ArrowDown" || e.key === "ArrowUp")) { e.preventDefault(); app.jump(e.key === "ArrowDown" ? 1 : -1); return; }
    if (e.metaKey && (e.key === "j" || e.key === "J")) { e.preventDefault(); $("#chat").classList.toggle("on"); return; }
    const open = app.m.threads.find((t) => t.open);
    if (e.metaKey && e.key === "Enter" && open) { e.preventDefault(); app.resolve(open.id); return; }
    if (e.metaKey && e.key === "Backspace" && open && !inInput) { e.preventDefault(); app.dismiss(open.id); return; }
    if (e.metaKey && (e.key === "z" || e.key === "Z") && app.m.toast && !inInput) { e.preventDefault(); app.undo(); return; }
    if (e.key === "Escape") {
      if ($("#chat").classList.contains("on")) { $("#chat").classList.remove("on"); return; }
      if (open) { open.open = false; app.render(); }
      else if (app.m.room) app.closeRoom();
    }
  });
  document.addEventListener("click", (e) => { if (e.target.closest("[data-pencil]")) togglePencil(); });

  // ------------------------------------------------------------------ state panels
  const STATES = [
    ["reading", "Reading", "¶4 is being read. Its number pulses blue; nothing else moves.", (m) => { m.paras = [3]; }],
    ["waiting", "Waiting while you type", "A query that arrives mid-sentence comes in closed, just its number. It opens when you pause.", (m) => { m.paras = [2]; }],
    ["open", "Open", "AQ1 open: reply, resolve, dismiss, or ask Inkling to draw it.", (m) => { m.paras = [1]; }],
    ["fixing", "Re-reading your change", "You changed 5:30 to 4:55. Inkling reads the sentence again before it says anything.", (m) => { m.paras = [1]; }],
    ["settled", "Settled by Inkling", "The fix answered it. A blue tick and one line of why; the wash goes, the caret fades.", (m) => { m.paras = [1]; }],
    ["resolved", "Resolved by you", "Your graphite OK. The query folds to a chip you can reopen.", (m) => { m.paras = [2]; }],
    ["dismissed", "Dismissed, with undo", "Stet dots under the words, stet in the margin, and eight seconds to change your mind.", (m) => { m.paras = [2]; }],
    ["drawing", "A sketch", "Three steps in blue pencil, a, b and c, with ‹ › to step and ↻ to replay.", (m) => { m.paras = [1]; }],
    ["figure", "Taken into the note", "Dragged into the text, the sketch turns from blue to ink: now it's yours, and it prints.", (m) => { m.paras = [1]; m.threads = m.threads.filter((t) => t.id === 1); m.threads[0].status = "resolved"; }],
    ["room", "Ask the room", "Three minds on one phrase, each in its own pencil. Your selection is graphite.", (m) => { m.paras = [1]; }],
    ["hero-off", "Pencil lifted", "⌘⇧. and only your words are left, exactly as they will print.", (m) => { m.paras = [1]; }],
    ["empty", "Empty note", "A blank proof and one line on what will happen.", () => {}],
    ["first-run", "First run", "One sentence of yours, and Inkling's first question is about it.", () => {}],
    ["offline", "Offline", "Writing still works. Inkling says what is wrong and that it will catch up.", (m) => { m.paras = [0]; m.threads = []; }],
    ["limit", "Limit reached", "When it can read again, in plain words.", (m) => { m.paras = [0]; m.threads = []; }],
  ];
  function buildStates() {
    const grid = $("#state-grid");
    grid.innerHTML = STATES.map(([id, title, desc]) =>
      '<div class="state"><h3><a href="?state=' + id + '#app" data-state="' + id + '">' + esc(title) + '</a><span class="mono">?state=' + id + "</span></h3><p>" + esc(desc) + '</p><div class="fit" data-w="580" data-h="200"><div class="fit-inner"><div class="mini" id="mini-' + id + '"></div></div></div></div>').join("");
    STATES.forEach(([id, , , trim], n) => {
      const m = baseModel();
      m.title = null;
      applyState(m, id === "hero-off" ? "" : id);
      trim(m);
      if (id === "empty") m.title = "Untitled";
      const box = $("#mini-" + id).closest(".fit");
      const wide = id === "drawing";
      const s = new Sheet($("#mini-" + id), { g: 44, tw: wide ? 250 : 300, mw: wide ? 286 : 236, crops: false, title: id === "empty", model: m, seed: 20 + n, fitBox: box, noLift: id !== "hero-off" });
      if (id === "hero-off") { s.o.noLift = true; s.root.classList.add("lifted"); s.render = ((orig) => function () { orig.call(this); this.root.classList.add("lifted"); })(s.render); s.render(); }
    });
    grid.addEventListener("click", (e) => {
      const a = e.target.closest("[data-state]");
      if (!a) return;
      e.preventDefault();
      const id = a.dataset.state;
      app.m = freshApp(id === "hero-off" ? "" : id);
      $("#chat").classList.toggle("on", id === "chat");
      if (id === "hero-off") setPencil(false); else if (lifted) setPencil(true, true);
      app.render(); appChanged();
      $("#app").scrollIntoView({ behavior: B.still ? "auto" : "smooth", block: "start" });
    });
  }

  // ------------------------------------------------------------------ reference sections
  function buildMarks() {
    $("#hero-wordmark").innerHTML = M.wordmark({ ink: "var(--ink)", pencil: "var(--pencil)" });
    $("#wm-light").innerHTML = M.wordmark({ ink: "#1B1A17", pencil: "#2F6FB3" });
    $("#wm-dark").innerHTML = M.wordmark({ ink: "#E9E6DE", pencil: "#7CB2EC" });
    // the detail: the whole word, large, with two callouts
    const wm = M.wordmark({ ink: "var(--ink)", pencil: "var(--pencil)" }).replace('viewBox="-13 -107.2 372.1 151"', 'viewBox="-30 -150 420 200"');
    const callouts = '<g style="stroke:var(--ink-3);stroke-width:.6;fill:none"><path d="M17 -88 L17 -122 L60 -122"/><path d="M200 -76 L200 -110 L232 -110"/></g>' +
      '<text x="64" y="-118">caret, not a dot</text><text x="236" y="-106">ordinary dot</text>';
    $("#wm-detail").innerHTML = wm.replace("</svg>", callouts + "</svg>");
    // app icons, every size, both appearances
    const icons = (file, cls) => {
      let h = '<img class="big" src="assets/proof/' + file + '" alt="Proof app icon' + (cls === "d" ? ", dark" : "") + '"><div class="icon-row">';
      [128, 64, 32, 16].forEach((s) => { h += '<div><img src="assets/proof/' + file + '" width="' + s + '" height="' + s + '" alt=""><span>' + s + "</span></div>"; });
      return h + "</div>";
    };
    $("#icons-l").innerHTML = icons("icon.svg", "l");
    $("#icons-d").innerHTML = icons("icon-dark.svg", "d");
    // the three details of the object
    const E = M.ELEMENTS;
    $("#det-blue").outerHTML = E["blue-pencil"].draw().replace("<svg ", '<svg class="det" ');
    $("#det-query").outerHTML = E["query-circle"].draw().replace("<svg ", '<svg class="det" ');
    $("#det-stet").outerHTML = E["stet-dots"].draw().replace("<svg ", '<svg class="det" ');
    // the element sheet
    $("#els").innerHTML = Object.keys(E).map((key) => {
      const e = E[key], svg = e.draw();
      // enlarged copy: up to 4x, never wider than the tile (about 230 px of room)
      const scale = Math.min(104 / e.h, 220 / e.w, 4);
      const big = svg.replace(/ width="[\d.]+" height="[\d.]+"/, ' style="width:' + f(e.w * scale) + "px;height:" + f(e.h * scale) + 'px"');
      return '<div class="el"><div class="el-art"><span class="at1" title="Actual size">' + svg + '</span><span class="big">' + big + "</span></div><h3>" + esc(e.name) + "</h3><p>" + esc(e.job) + '</p><p class="where mono">' + esc(e.where) + "</p></div>";
    }).join("");
    // type specimen in Inkling's voice
    $("#spec-q").innerHTML = '<span style="font-family:var(--font-mono);font-size:var(--t1)">AQ1</span> · Fix<br>' + esc(T1.text);
  }

  function buildColour() {
    const L = [
      ["Paper", "#FBFBF8", "The sheet your words are on.", null],
      ["Desk", "#ECEBE6", "The table the proof lies on; the page around the sheet.", null],
      ["Ink", "#1B1A17", "Your words.", "#FBFBF8"],
      ["Ink 2", "#4A4740", "Secondary text.", "#FBFBF8"],
      ["Ink 3", "#67635A", "Paragraph numbers, running heads, labels.", "#ECEBE6"],
      ["Rule", "#E1DED6", "Hairlines on paper.", null],
      ["Pencil", "#2F6FB3", "Inkling: every query and every mark. Only ever on paper.", "#FBFBF8"],
      ["Soft pencil", "#5E95CF", "Carets and guide lines: lighter strokes of the same pencil.", "#FBFBF8", true],
      ["Wash", "rgba(47,111,179,.12)", "Under the words a query is about.", null],
      ["Graphite", "#55524B", "You: OK, stet, your replies.", "#FBFBF8"],
      ["Skeptic", "#2E7A4C", "Another mind's pencil, in Ask the room.", "#FBFBF8"],
      ["Tutor", "#6A4FA6", "Another mind's pencil.", "#FBFBF8"],
    ];
    const D = [
      ["Paper", "#171A1E", "The sheet, on the lightbox.", null],
      ["Desk", "#0F1114", "The dark room around it.", null],
      ["Ink", "#E9E6DE", "Your words.", "#171A1E"],
      ["Ink 2", "#B8B3A8", "Secondary text.", "#171A1E"],
      ["Ink 3", "#8E897F", "Numbers, heads, labels.", "#171A1E"],
      ["Rule", "#2A2E34", "Hairlines.", null],
      ["Pencil", "#7CB2EC", "Inkling.", "#171A1E"],
      ["Soft pencil", "#5E95CF", "Carets and guide lines.", "#171A1E", true],
      ["Wash", "rgba(124,178,236,.16)", "Under the anchored words.", null],
      ["Graphite", "#A8A395", "You.", "#171A1E"],
      ["Skeptic", "#7CC79A", "Another mind.", "#171A1E"],
      ["Tutor", "#B9A3EC", "Another mind.", "#171A1E"],
    ];
    const row = ([name, hex, job, on, mark]) => {
      let ratio = "surface";
      if (on) { const r = B.contrast(hex, on); ratio = r.toFixed(1) + ":1" + (mark ? " mark" : ""); }
      const shown = hex.startsWith("rgba") ? hex.replace(/rgba\(([^)]+)\)/, (s, v) => { const p = v.split(","); return "#" + p.slice(0, 3).map((x) => (+x).toString(16).padStart(2, "0")).join("").toUpperCase() + " at " + Math.round(+p[3] * 100) + "%"; }) : hex;
      return '<div class="tok"><i style="background:' + hex + '"></i><b>' + name + '</b><code>' + shown + (on ? ' · <span class="ratio">' + ratio + "</span>" : "") + "</code><span>" + job + "</span></div>";
    };
    $("#pal-light").insertAdjacentHTML("beforeend", L.map(row).join(""));
    $("#pal-dark").insertAdjacentHTML("beforeend", D.map(row).join(""));
  }

  function soundPlay(name) {
    if (!B.soundOn) { const b = $('[data-board="sound"]'); if (b) b.click(); }
    setTimeout(() => B.sound(name), 60);
  }
  function buildMotion() {
    const rows = [
      ["draw", "A pencil stroke draws on", "Every mark Inkling makes: carets, guide lines, circles.", "280 ms · cubic-bezier(.2,.7,.2,1)"],
      ["lift", "The pencil lifts", "Rises 4 px and fades; comes back without the rise.", "360 ms off · 240 ms on"],
      ["read", "Reading", "The paragraph number pulses until the read is done.", "1.2 s loop"],
      ["resolve", "You resolve", "Graphite OK; the query folds to a chip.", "tick 180 ms · fold 260 ms"],
      ["settle", "It settles itself", "Blue tick, and the question gives way to the reason.", "tick 180 ms · text 200 ms"],
      ["sketch", "Sketch steps", "Each step draws, then waits so you can read it.", "750 ms a step · 1.35 s apart"],
      ["ink", "Into the note", "Moves into place, then blue turns to ink.", "move 420 ms · colour 400 ms"],
      ["s-tick", "Sound: pencil tick", "A query arrives, settles or is answered. Off until you turn sound on.", "40 ms · synthesised here"],
      ["s-paper", "Sound: paper slide", "The pencil lifts or a sketch is taken into the note.", "300 ms · synthesised here"],
    ];
    $("#moves").innerHTML = rows.map(([id, t, d, spec]) =>
      '<div class="move"><div><b>' + t + "</b><span>" + d + '</span><span class="mono">' + spec + '</span></div><div class="stage" id="st-' + id + '"></div><button class="btn" data-play="' + id + '">Play</button></div>').join("");
    const stage = (id, html) => { const el = $("#st-" + id); el.innerHTML = '<svg aria-hidden="true">' + html + "</svg>"; return el; };
    const drawStage = () => stage("draw", '<rect class="mk-wash" x="16" y="22" width="80" height="18" rx="2"/>' + pm(M.loop(118, 18, 78, 26, 3, { pad: 6 }), "mk-pencil", true, 280) + '<text class="st-text" x="124" y="36">AQ1 · Fix</text>');
    const liftStage = () => stage("lift", '<text class="st-body" x="16" y="38">just under 50.</text><g class="lift">' + pm(M.caret(112, 38, 1, 3), "mk-soft", false) + pm(M.loop(140, 20, 60, 24, 4, { pad: 5 }), "mk-pencil", false) + '<text class="st-text" x="146" y="36">AQ1</text></g>');
    const readStage = () => stage("read", '<text class="st-num" x="24" y="38">4</text><text class="st-body" x="44" y="38">Strength work twice…</text>');
    const resolveStage = (fresh) => stage("resolve", '<text class="st-text" x="16" y="38">AQ2 · Query</text><text class="st-text" x="100" y="38" style="fill:var(--graphite);font-weight:600">OK</text><g transform="translate(124 26)">' + pm(M.tick(0, 4, 1, 13), "mk-graphite", fresh, 180) + "</g>");
    const settleStage = (fresh) => stage("settle", '<g transform="translate(16 26)">' + pm(M.tick(0, 4, 0.95, 13), "mk-pencil", fresh, 180) + '</g><text class="st-text" x="38" y="38">4:55 a km comes to 49:10.</text>');
    const sketchStage = (step) => { const el = $("#st-sketch"); el.innerHTML = '<div class="fig" style="margin:0;transform:scale(.5);transform-origin:0 0;position:absolute;left:52px;top:0">' + figureSVG(step, step) + "</div>"; animateFresh(el); };
    const inkStage = (inked) => { const el = $("#st-ink"); el.innerHTML = '<div class="fig' + (inked ? " inked" : "") + '" style="margin:0;transform:scale(.5);transform-origin:0 0;position:absolute;left:52px;top:0">' + figureSVG(3, 0) + "</div>"; };
    drawStage(); liftStage(); readStage(); resolveStage(false); settleStage(false); sketchStage(3); inkStage(false);
    stage("s-tick", pm(M.tick(100, 26, 1, 13), "mk-pencil", false));
    stage("s-paper", '<rect x="60" y="14" width="100" height="36" style="fill:var(--desk)"/><rect x="70" y="18" width="100" height="36" style="fill:var(--paper);stroke:var(--rule)"/>');
    $("#moves").addEventListener("click", async (e) => {
      const b = e.target.closest("[data-play]");
      if (!b) return;
      const id = b.dataset.play;
      if (id === "draw") { drawStage(); animateFresh($("#st-draw")); }
      if (id === "lift") { const g = $("#st-lift .lift"); g.style.transition = "opacity 360ms var(--ease), transform 360ms var(--ease)"; g.style.opacity = 0; g.style.transform = "translateY(-4px)"; await wait(900); g.style.transition = "opacity 240ms var(--ease), transform 240ms var(--ease)"; g.style.opacity = 1; g.style.transform = "none"; }
      if (id === "read") { const n = $("#st-read .st-num"); n.style.fill = "var(--pencil)"; n.animate([{ opacity: 1 }, { opacity: .35 }, { opacity: 1 }], { duration: 1200, iterations: 3, easing: "cubic-bezier(.2,.7,.2,1)" }).onfinish = () => { n.style.fill = ""; }; }
      if (id === "resolve") { resolveStage(true); animateFresh($("#st-resolve")); }
      if (id === "settle") { settleStage(true); animateFresh($("#st-settle")); $("#st-settle text").animate([{ opacity: 0 }, { opacity: 1 }], { duration: 200, delay: 120, fill: "backwards" }); }
      if (id === "sketch") { for (let s = 1; s <= 3; s++) { sketchStage(s); await wait(1350); } }
      if (id === "ink") { inkStage(false); await wait(300); $("#st-ink .fig").classList.add("inked"); await wait(1400); inkStage(false); }
      if (id === "s-tick") soundPlay("tick");
      if (id === "s-paper") soundPlay("paper");
    });
  }

  function buildAha() {
    const bar = (x, y, w, cls) => '<rect class="' + (cls || "mk-ink") + '" x="' + x + '" y="' + y + '" width="' + w + '" height="5" rx="2.5" opacity=".78"/>';
    const arrow = (x, y) => M.arrow(x, y, x + 20, y, 91).map((m) => pm(m, "mk-ink")).join("");
    const lines = (x) => bar(x, 26, 70) + bar(x, 42, 62) + bar(x, 58, 66);
    const marked = (x) => '<rect class="mk-wash" x="' + (x - 2) + '" y="38" width="66" height="13" rx="2"/>' + lines(x) +
      pm(M.caret(x + 63, 47, 1, 3), "mk-soft") + pm(M.loop(x + 84, 30, 14, 12, 5, { pad: 5, radius: 8 }), "mk-pencil") +
      '<text x="' + (x + 84) + '" y="40" style="font:500 12px var(--font-mono);fill:var(--pencil)">1</text>';
    const fig = (x, cls, framed) => pm(M.mark(M.resample([[x, 62], [x + 90, 62]], 1), 1.3, [0.05, 0.05], 0.1, 51), cls) +
      [0, 30, 60, 90].map((d, i) => pm(M.mark([[x + d, 58], [x + d, 66]], 1.1, [0.2, 0.2], 0, 60 + i), cls)).join("") +
      pm(M.mark(M.resample(M.cubic([x + 4, 52], [x + 30, 18], [x + 56, 46], [x + 88, 16], 24), 1), 1.4, [0.08, 0.1], 0.05, 8), cls) +
      (framed ? '<path d="M' + (x - 8) + " 18 V8 H" + x + " M" + (x + 90) + " 8 H" + (x + 98) + " V18 M" + (x - 8) + " 66 V76 H" + x + " M" + (x + 90) + " 76 H" + (x + 98) + ' V66" style="stroke:var(--ink);stroke-width:1;fill:none"/>' : "");
    const items = [
      ["Lift the pencil", "⌘⇧. and every blue mark rises off the page. What is left is exactly what you wrote, ready to send.",
        marked(2) + arrow(122, 44) + lines(156)],
      ["The sketch becomes yours", "Drag Inkling's sketch into your note and it turns from blue to ink. Blue never prints; ink does. Taking it is the moment it is yours.",
        fig(4, "mk-pencil", false) + arrow(110, 44) + fig(146, "mk-ink", true)],
      ["Fix the words, the query closes", "Change 5:30 to 4:55 and Inkling re-reads the sentence, ticks the query, and says why: 4:55 a km comes to 49:10.",
        '<text x="0" y="50" style="font:400 28px var(--font-body);fill:var(--ink-3)">5:30</text>' + pm(M.mark(M.resample([[0, 41], [56, 39]], 1), 1.4, [0.05, 0.05], 0.1, 5), "mk-graphite") +
        '<text x="70" y="50" style="font:400 28px var(--font-body);fill:var(--ink)">4:55</text>' +
        '<g transform="translate(146 30)">' + pm(M.tick(0, 6, 1.5, 13), "mk-pencil") + '</g><text x="176" y="47" style="font:400 14px var(--font-ui);fill:var(--pencil)">49:10</text>'],
      ["The numbers show what was read", "A tick beside a paragraph number means Inkling read it and had nothing to ask. A circle means it has a question. Silence you can see.",
        '<g style="font:400 12px var(--font-mono);fill:var(--ink-3)"><text x="26" y="20">1</text><text x="26" y="46">2</text><text x="26" y="72">3</text></g>' +
        pm(M.tick(4, 12, 0.75, 1), "mk-pencil") + pm(M.ring(29.6, 41.6, 7.6, 2), "mk-pencil") + pm(M.tick(4, 64, 0.75, 3), "mk-pencil") +
        bar(46, 13, 150) + bar(46, 39, 120) + '<rect class="mk-wash" x="44" y="35" width="124" height="13" rx="2"/>' + bar(46, 65, 170)],
    ];
    $("#ahas").innerHTML = items.map(([t, d, art], n) =>
      '<div class="aha"><svg viewBox="0 0 240 84" aria-hidden="true">' + art + '</svg><span class="n">0' + (n + 1) + "</span><h3>" + t + "</h3><p>" + d + "</p></div>").join("");
  }

  /** ?run=<try-chip> clicks a Try chip after load, so animated paths can be screenshotted. */
  function runAfterLoad() {
    const run = B.params.get("run");
    if (!run) return;
    const steps = run.split(",");
    let delay = 400;
    steps.forEach((s) => {
      setTimeout(() => { const b = document.querySelector('#try [data-do="' + s + '"]'); if (b) b.click(); }, delay);
      delay += 4200;
    });
  }

  // ------------------------------------------------------------------ boot
  function relayoutAll() {
    sheets.forEach((s) => s.layout());
    B.fit();
  }
  function boot() {
    // ?only=a,b shows just those sections (for headless screenshots of a long page)
    const only = B.params.get("only");
    if (only) {
      const keep = only.split(",");
      $$("main > section").forEach((s) => { if (!keep.includes(s.id || "hero")) s.style.display = "none"; });
    }
    buildMarks();
    buildHero();
    buildApp();
    buildStates();
    buildColour();
    buildMotion();
    buildAha();
    if (lifted) setPencil(false, true);
    B.fit();
    runAfterLoad();
    // text metrics change once the fonts arrive; lay every mark out again
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => { relayoutAll(); setTimeout(relayoutAll, 120); });
    let rt = 0;
    addEventListener("resize", () => {
      clearTimeout(rt);
      rt = setTimeout(() => {
        const narrow = $("#hero-fit").parentElement.clientWidth < 600;
        if ((narrow && !heroBelow) || (!narrow && !hero)) buildHero();
        $("#hero-fit").style.display = narrow ? "none" : "";
        $("#hero-below").style.display = narrow ? "" : "none";
        relayoutAll();
      }, 80);
    });
    if (B.state && B.state !== "hero-off") {
      const target = $("#app");
      // a deep link to a state lands on the app (skipped for headless checks, which can't paint after a scroll)
      if (target && !B.params.has("qa") && !B.params.has("only")) setTimeout(() => target.scrollIntoView({ block: "start" }), 50);
    }
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", boot);
  else boot();
})();

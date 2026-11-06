/* The Settings window (src/settings.rs after the merge on 6 Nov 2026): its own 900 x 721
 * window, a rail of panes on the left and the pane on a sheet. Rebuilt from screenshots of
 * the merged build. render(host, S, t) draws one frame; S says which pane and what state.
 */
(function () {
  "use strict";
  const A = window.APP, { fx, clamp, lerp, esc, icon, E } = A;
  const PANES = [["Reading", "book"], ["Appearance", "sun"], ["Minds", "spool"], ["About you", "user"], ["Model", "spark"], ["Shortcuts", "keys"], ["Help", "buoy"]];
  const PI = {
    book: '<path d="M10 5.5C8 4 5.5 3.5 3 3.5v11c2.5 0 5 .5 7 2 2-1.5 4.5-2 7-2v-11c-2.5 0-5 .5-7 2zM10 5.5v11"/>',
    sun: '<path d="M10 6.5a3.5 3.5 0 1 0 0 7"/><path d="M10 6.5a3.5 3.5 0 0 1 0 7" style="opacity:.0"/><path d="M10 2.5v1.5M4.7 4.7l1 1M2.5 10H4M4.7 15.3l1-1M10 16v1.5"/><path d="M13 7.5l1.5-1.5M13.5 10H17M15 4.5"/>',
    spool: '<path d="M5 3.5h10M5 16.5h10M6.5 3.5v13M13.5 3.5v13M6.5 7l7 3-7 3"/>',
    user: '<circle cx="10" cy="7" r="3.2"/><path d="M4 17c.7-3 3.2-4.5 6-4.5s5.3 1.5 6 4.5"/>',
    spark: '<path d="M8 3l1.2 3.8L13 8l-3.8 1.2L8 13l-1.2-3.8L3 8l3.8-1.2zM14.5 12l.6 1.9 1.9.6-1.9.6-.6 1.9-.6-1.9-1.9-.6 1.9-.6z"/>',
    keys: '<rect x="2.5" y="5" width="15" height="10" rx="1.6"/><path d="M5.5 8h1M8.5 8h1M11.5 8h1M14.5 8h.5M6 12h8"/>',
    buoy: '<circle cx="10" cy="10" r="7"/><circle cx="10" cy="10" r="3"/><path d="M5 5l2.9 2.9M12.1 12.1L15 15M15 5l-2.9 2.9M7.9 12.1L5 15"/>',
  };
  const pi = (n, c) => `<svg viewBox="0 0 20 20" width="16" height="16" style="fill:none;stroke:${c};stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round;flex:none">${PI[n]}</svg>`;
  const CHECK = `<span style="position:absolute;right:9px;top:9px;width:17px;height:17px;border-radius:50%;background:var(--ink);display:grid;place-items:center">${icon("check", 11, "var(--paper)", 2.2)}</span>`;

  const css = `
  .sw-win { position:absolute; width:900px; height:721px; border-radius:14px; overflow:hidden; background: color-mix(in srgb, var(--cloth) 70%, var(--shelf)); font-family:"Figtree";
    box-shadow: 0 0 0 1px rgba(var(--shade),.18), 0 2px 6px rgba(var(--shade),.08), 0 50px 90px -40px rgba(var(--shade),.7); }
  .sw-rail { position:absolute; left:10px; top:10px; width:209px; height:699px; border-radius:12px; background: color-mix(in srgb, var(--shelf) 80%, var(--paper)); box-shadow: 0 0 0 1px var(--rule); }
  .sw-rail .lt { position:absolute; left:16px; top:20px; display:flex; gap:9px; } .sw-rail .lt i { width:13px; height:13px; border-radius:50%; background:#FF5F57; } .sw-rail .lt i:nth-child(2){background:#FEBC2E} .sw-rail .lt i:nth-child(3){background:#28C840}
  .sw-rail .nav { position:absolute; left:12px; right:12px; top:52px; display:flex; flex-direction:column; gap:3.5px; }
  .sw-rail .nav span { display:flex; align-items:center; gap:10px; height:29px; padding:0 10px; border-radius:8px; font-size:13.2px; color:var(--ink); }
  .sw-rail .nav span.on { background:var(--paper); box-shadow: 0 1px 3px rgba(var(--shade),.14); }
  .sw-rail .ft { position:absolute; left:18px; right:18px; bottom:14px; display:flex; align-items:flex-end; justify-content:space-between; font-size:10.5px; color:var(--graphite); }
  .sw-sheet { position:absolute; left:230px; top:10px; width:660px; height:699px; border-radius:12px; background:var(--paper); box-shadow: 0 0 0 1px var(--rule); overflow:hidden; }
  .sw-sc { position:absolute; left:33px; right:33px; top:0; }
  .sw-sheet h3 { margin:0; padding-top:34px; font-size:19.5px; font-weight:600; color:var(--ink); letter-spacing:-.005em; }
  .sw-sheet .back { padding-top:30px; font-size:11.5px; color:var(--graphite); display:flex; align-items:center; gap:4px; }
  .sw-sheet .back + h3 { padding-top:8px; }
  .sw-sheet .bl { margin-top:7px; font-size:11.6px; line-height:17px; color:var(--graphite); }
  .sw-sec { margin-top:22px; display:flex; justify-content:space-between; align-items:baseline; font-size:11.6px; font-weight:600; color:var(--ink); padding: 0 1px; }
  .sw-sec small { font-weight:400; font-size:10.4px; color:var(--graphite); }
  .sw-cards { margin-top:10px; display:flex; gap:11px; }
  .sw-card { position:relative; flex:1; min-width:0; border-radius:10px; border:1px solid var(--rule); background: color-mix(in srgb, var(--paper) 60%, var(--card)); padding:12px 13px; }
  .sw-card.on { border:1.5px solid var(--ink); }
  .sw-card .t { font-size:12.6px; color:var(--ink); } .sw-card .d { margin-top:5px; font-size:10.8px; line-height:15.5px; color:var(--graphite); }
  .sw-card .th { height:86px; border-radius:6px; margin:-4px -5px 10px; overflow:hidden; position:relative; }
  .sw-group { margin-top:10px; border-radius:10px; border:1px solid var(--rule); background: color-mix(in srgb, var(--paper) 60%, var(--card)); }
  .sw-row { display:flex; align-items:center; gap:14px; min-height:46px; padding:9px 14px; }
  .sw-row + .sw-row { border-top:1px solid var(--rule); }
  .sw-row .grow { flex:1; min-width:0; } .sw-row .t { font-size:12.6px; color:var(--ink); } .sw-row .d { margin-top:3px; font-size:10.8px; color:var(--graphite); line-height:15px; }
  .seg { display:flex; padding:2.5px; border-radius:8px; background: color-mix(in srgb, var(--rule) 60%, var(--paper)); font-size:12px; color:var(--graphite); }
  .seg span { padding:4px 12px; border-radius:6px; } .seg span.on { background:var(--paper); color:var(--ink); box-shadow: 0 1px 2px rgba(var(--shade),.2); font-weight:500; }
  .kc { display:inline-flex; align-items:center; gap:1px; padding:3px 8px; border-radius:6px; border:1px solid var(--rule); border-bottom-width:2px; background:var(--paper); font-size:12px; color:var(--ink); white-space:nowrap; }
  .kc.rec { border-color: var(--voice); color: var(--voice); }
  .sw-in { margin-top:6px; padding:10px 14px; border-radius:8px; border:1px solid var(--rule); background: color-mix(in srgb, var(--paper) 40%, var(--shelf)); font-size:12.4px; line-height:18px; color:var(--ink); }
  .sw-in .ph { color:var(--faint); }
  .sw-chips { margin-top:10px; display:flex; flex-wrap:wrap; gap:7px; }
  .sw-chips span { padding:6px 11px; border-radius:999px; border:1px solid var(--rule); font-size:12px; color:var(--ink); background:var(--paper); }
  .sw-chips span.on { background:var(--ink); color:var(--paper); border-color:var(--ink); } .sw-chips span.hov { border-color:var(--faint); }
  .sw-btn { padding:6px 13px; border-radius:999px; background:var(--ink); color:var(--paper); font-size:12px; }
  .sw-search { display:flex; align-items:center; gap:10px; width:208px; height:28px; padding:0 12px; border-radius:8px; border:1px solid var(--rule); font-size:11.6px; color:var(--graphite); }
  .sw-dots { display:flex; gap:9px; align-items:center; } .sw-dots i { width:21px; height:21px; border-radius:50%; display:grid; place-items:center; border:1.5px solid transparent; } .sw-dots i.on { border-color:var(--ink); } .sw-dots i b { width:15px; height:15px; border-radius:50%; display:block; }
  .tg-sw { width:34px; height:20px; border-radius:10px; background: color-mix(in srgb, var(--rule) 80%, var(--faint)); position:relative; flex:none; } .tg-sw i { position:absolute; top:2.5px; width:15px; height:15px; border-radius:50%; background:var(--paper); box-shadow:0 1px 2px rgba(0,0,0,.25); } .tg-sw.on { background:var(--ink); }
  `;
  let styled = false;
  function ensureCSS() { if (styled) return; const s = document.createElement("style"); s.textContent = css; document.head.appendChild(s); styled = true; }

  /** A little window for the appearance and placement thumbnails: cloth, page, lines, a stitched underline and a tag. */
  function thumb(kind, opt) {
    const L = { cloth: "#ebe3d4", page: "#fbf8f2", line: "#c9bfae", m: "#a63f32" }, D = { cloth: "#18161a", page: "#1f1d22", line: "#4a444d", m: "#d9705e" };
    const pageSVG = (P, x, w, tags) => {
      let s = `<rect x="${x}" y="22" width="${w}" height="70" fill="${P.page}"/>`;
      [32, 44, 56, 68, 80].forEach((y, i) => (s += `<rect x="${x + 12}" y="${y}" width="${w - 24 - (i % 2) * 18}" height="2.2" rx="1" fill="${P.line}"/>`));
      if (tags) s += tags(P);
      return s;
    };
    if (kind === "appearance") {
      const P = opt === "dark" ? D : L;
      const tag = (Q) => `<path d="M88 49h18" stroke="${Q.m}" stroke-width="1.3" stroke-dasharray="3 2"/><path d="M106 49h30" stroke="${Q.m}" stroke-width=".9"/><rect x="136" y="43" width="22" height="12" rx="3" fill="${Q.page}" stroke="${Q.m}" stroke-width=".9"/><path d="M140 47.5h13M140 50.5h9" stroke="${Q.line}" stroke-width="1"/>`;
      let s = `<svg viewBox="0 0 200 92" width="100%" height="100%" preserveAspectRatio="xMidYMid slice"><rect width="200" height="92" fill="${P.cloth}"/><circle cx="9" cy="8" r="2" fill="${P.line}"/><circle cx="16" cy="8" r="2" fill="${P.line}"/><circle cx="23" cy="8" r="2" fill="${P.line}"/>` + pageSVG(P, 52, 96, tag);
      if (opt === "system") s += `<g clip-path="url(#half)"><clipPath id="half"><rect x="100" y="0" width="100" height="92"/></clipPath><rect width="200" height="92" fill="${D.cloth}"/>${pageSVG(D, 52, 96, tag)}</g>`;
      return s + "</svg>";
    }
    // placement: both, right, marks
    const P = L;
    let s = `<svg viewBox="0 0 200 92" width="100%" height="100%" preserveAspectRatio="xMidYMid slice"><rect width="200" height="92" fill="${P.cloth}"/>` + pageSVG(P, 60, 80);
    const tag = (x, y) => `<rect x="${x}" y="${y - 6}" width="24" height="12" rx="3" fill="${P.page}" stroke="${P.m}" stroke-width=".9"/><path d="M${x + 4} ${y - 1.5}h13M${x + 4} ${y + 1.5}h9" stroke="${P.line}" stroke-width="1"/>`;
    if (opt === "both") s += `<path d="M84 46h22" stroke="${P.m}" stroke-width="1.3" stroke-dasharray="3 2"/><path d="M84 46H38" stroke="${P.m}" stroke-width=".8"/>${tag(14, 46)}<path d="M74 70h22" stroke="${P.m}" stroke-width="1.3" stroke-dasharray="3 2"/><path d="M96 70h66" stroke="${P.m}" stroke-width=".8"/>${tag(162, 70)}`;
    if (opt === "right") s += `<path d="M84 46h22" stroke="${P.m}" stroke-width="1.3" stroke-dasharray="3 2"/><path d="M106 46h56" stroke="${P.m}" stroke-width=".8"/>${tag(162, 46)}<path d="M74 70h22" stroke="${P.m}" stroke-width="1.3" stroke-dasharray="3 2"/><path d="M96 70h66" stroke="${P.m}" stroke-width=".8"/>${tag(162, 70)}`;
    if (opt === "marks") s += `<path d="M84 46h22" stroke="${P.m}" stroke-width="1.3" stroke-dasharray="3 2"/><circle cx="111" cy="44" r="2.4" fill="${P.m}"/><path d="M74 70h22" stroke="${P.m}" stroke-width="1.3" stroke-dasharray="3 2"/><circle cx="101" cy="68" r="2.4" fill="${P.m}"/>`;
    return s + "</svg>";
  }

  const PANE = {
    Reading(S) {
      const card = (on, t, d) => `<div class="sw-card${on ? " on" : ""}">${on ? CHECK : ""}<div class="t">${t}</div><div class="d">${d}</div></div>`;
      const pl = (k, t, d) => `<div class="sw-card${S.placement === k ? " on" : ""}" style="padding-top:12px">${S.placement === k ? CHECK : ""}<div class="th">${thumb("placement", k)}</div><div class="t">${t}</div><div class="d">${d}</div></div>`;
      return `<h3>Reading</h3><div class="bl">How Inkling reads along, and where its questions go.</div>` +
        `<div class="sw-sec">New notes start in</div><div class="sw-cards" data-m="mode">${card(S.mode !== "ask", "Reads along", "Reads along and speaks up when something’s worth a second look.")}${card(S.mode === "ask", "Waits to be asked", "Quiet until you say its name, select words, or open the chat.")}</div>` +
        `<div style="margin-top:7px;font-size:10.6px;color:var(--graphite)">Change any note on its own, from the row above its title.</div>` +
        `<div class="sw-sec">Reading pace</div><div class="sw-group" data-m="pace"><div class="sw-row"><div class="grow"><div class="t">Pace</div><div class="d">${S.pace === "slow" ? "Waits about twice as long, for when you pause mid-thought." : "Reads a moment after you stop typing."}</div></div><div class="seg"><span class="${S.pace !== "slow" ? "on" : ""}">Regular</span><span class="${S.pace === "slow" ? "on" : ""}">Slow</span></div></div></div>` +
        `<div class="sw-sec">Where questions appear</div><div class="sw-cards" data-m="place">${pl("both", "Both margins", "Your words in the middle, questions to either side.")}${pl("right", "Right margin", "Every question in one column beside your writing.")}${pl("marks", "Marks only", "A small mark after the passage. Open it when you’re ready.")}</div>`;
    },
    Appearance(S) {
      const ap = (k, t, d) => `<div class="sw-card${S.look === k ? " on" : ""}">${S.look === k ? CHECK : ""}<div class="th">${thumb("appearance", k)}</div><div class="t">${t}</div><div class="d">${d}</div></div>`;
      const size = (k, px, label) => `<div class="sw-card${S.size === k ? " on" : ""}" style="padding:14px 15px 13px">${S.size === k ? CHECK : ""}<div style="font:600 ${px}px/1.3 'Fraunces Text';color:var(--ink)">A thought.</div><div class="d" style="margin-top:7px;font-size:11px">${label}</div></div>`;
      const let_ = (k, fam, label) => `<div class="sw-card${S.lettering === k ? " on" : ""}" style="padding:15px 15px 14px">${S.lettering === k ? CHECK : ""}<div style="font:${k === "own" ? "500" : "600"} 15px/1.3 ${fam};color:var(--voice)">Does this still hold?</div><div class="d" style="margin-top:9px;font-size:11px">${label}</div></div>`;
      const names = { madder: "Madder (Inkling’s own)", indigo: "Indigo", sage: "Sage", rose: "Rose", plum: "Plum", walnut: "Walnut", ochre: "Ochre" };
      const dots = A.THREAD_ORDER.map((n) => `<i class="${S.ink === n ? "on" : ""}"><b style="background:var(--${n})"></b></i>`).join("");
      return `<h3>Appearance</h3><div class="bl">How Inkling looks. Your note changes as you choose.</div>` +
        `<div class="sw-sec">Appearance</div><div class="sw-cards" data-m="look">${ap("system", "System", "Follows macOS.")}${ap("light", "Light", "Linen: paper and ink.")}${ap("dark", "Dark", "Lamplight: the same cloth after dark.")}</div>` +
        `<div class="sw-sec">Text size<small>Your words, in Fraunces</small></div><div class="sw-cards" data-m="size">${size("small", 15, "Small")}${size("default", 17, "Default")}${size("large", 19, "Large")}</div>` +
        `<div class="sw-sec">Inkling’s lettering</div><div class="sw-cards" data-m="letter">${let_("own", "'Figtree'", "Inkling’s own")}${let_("notes", "'Fraunces Text'", "Like your notes")}</div>` +
        `<div class="sw-sec">Inkling’s thread</div><div class="sw-group"><div class="sw-row" data-m="dots"><div class="grow"><div class="t">Colour</div><div class="d">${names[S.ink] || ""}</div></div><div class="sw-dots">${dots}</div></div>` +
        `<div class="sw-row" data-m="inkdrop"><div class="grow"><div class="t">Ink from the pointer</div><div class="d">Now and then a drop of ink falls as you move the pointer, then dries and fades.</div></div><span class="tg-sw${S.inkDrop ? " on" : ""}"><i style="left:${S.inkDrop ? 16.5 : 2.5}px"></i></span></div></div>`;
    },
    NewMind(S) {
      const chips = ["Staff engineer", "Editor", "Skeptic", "Socratic tutor", "Investor", "Blank"].map((c) => `<span class="${S.tmpl === c ? "on" : ""}${S.hovChip === c ? " hov" : ""}">${c}</span>`).join("");
      const field = (label, hint, value, ph, caret, tall) => `<div style="padding:12px 14px 12px;${label !== "Name" ? "border-top:1px solid var(--rule)" : ""}"><div class="t" style="font-size:12.6px">${label}${hint ? `<small style="margin-left:7px;font-size:10.4px;color:var(--graphite)">${hint}</small>` : ""}</div>` +
        `<div class="sw-in" style="${tall ? "min-height:76px" : ""}">${value ? esc(value) : `<span class="ph">${esc(ph)}</span>`}${caret ? '<span class="icaret"></span>' : ""}</div></div>`;
      const seg = (opts, cur) => `<div class="seg">${opts.map((o) => `<span class="${o === cur ? "on" : ""}">${o}</span>`).join("")}</div>`;
      return `<div class="back">${icon("left", 11, "var(--graphite)")}Minds</div><h3>New mind</h3><div class="bl">A mind is what reads your notes: what it looks for, and how it asks. Give any note the one it needs.</div>` +
        `<div class="sw-sec">Start from</div><div class="sw-chips" data-m="chips">${chips}</div>` +
        `<div class="sw-sec">The mind</div><div class="sw-group" data-m="mind">` +
        field("Name", "", S.name, "Staff engineer", S.focus === "name") +
        field("Description", "Shown when you pick it", S.desc, "Reads designs as if it will be on call for them.", S.focus === "desc") +
        field("How it thinks", "What it looks for, pushes on, and leaves alone", S.how, "Read like a senior engineer reviewing a design doc. Ask about scale and failure. Ignore style.", S.focus === "how", true) + `</div>` +
        `<div class="sw-sec">How it speaks</div><div class="sw-group" data-m="speak"><div class="sw-row"><div class="grow t">Voice</div>${seg(["Plain", "Warm", "Blunt", "Socratic"], S.voice || "Plain")}</div>` +
        `<div class="sw-row"><div class="grow t">How often it speaks up</div>${seg(["Rarely", "When it matters", "Often"], S.often || "When it matters")}</div></div>` +
        `<div data-m="create" style="margin-top:16px;display:flex;justify-content:flex-end;align-items:center;gap:16px;font-size:12px;color:var(--faint)"><span>Cancel</span><span class="sw-btn"${S.hovCreate ? ' style="opacity:.86"' : ""}>Create mind</span></div><div style="height:40px"></div>`;
    },
    "About you"(S) {
      return `<h3>About you</h3><div class="bl">A short paragraph every note starts out knowing. It stays on this Mac.</div>` +
        `<div class="sw-group" data-m="learn" style="margin-top:22px"><div class="sw-row" style="padding:12px 14px"><div class="grow"><div class="t" style="font-size:13px">Inkling learns about you</div><div class="d">When you say something lasting about yourself, like what you study or how you like feedback, it can be worked in here.</div></div><span class="tg-sw${S.learn ? " on" : ""}"><i style="left:${fx(lerp(2.5, 16.5, S.learnK == null ? (S.learn ? 1 : 0) : S.learnK))}px"></i></span></div></div>` +
        `<div class="sw-sec">What Inkling knows<small>${(S.about || "").length} of 700</small></div>` +
        `<div class="sw-in" data-m="about" style="margin-top:10px;min-height:118px;padding:14px 16px;font-size:12.8px;line-height:19px;background:var(--paper)">${S.about ? esc(S.about) + (S.aboutCaret ? '<span class="icaret"></span>' : "") : '<span class="ph">Nothing yet. What you do, what you’re working toward, how you like feedback.</span>'}</div>` +
        `<div style="margin-top:9px;font-size:10.6px;color:var(--graphite)">Edit it any time. Inkling only adds to it when the switch above is on.</div>`;
    },
    Model(S) {
      const rows = [["Default (recommended)", "Opus 5.5 · Best for everyday, complex tasks"], ["Opus 5.5", "For complex work and everyday tasks"], ["Fable 5.1", "For your toughest challenges"], ["Sonnet 5.5", "Most efficient for simpler tasks"], ["Haiku 5.5", "Fastest for quick answers"]]
        .map(([n, d], i) => `<div class="sw-row" style="${S.hovModel === i ? "background:var(--wash)" : ""}"><div class="grow"><div class="t" style="font-size:13px">${n}</div><div class="d">${d}</div></div>${S.model === i ? icon("check", 14, "var(--ink)", 1.8) : ""}</div>`).join("");
      const eff = (label, cur) => `<div class="sw-row"><div class="grow t">${label}</div><div class="seg">${["low", "medium", "high", "xhigh", "max"].map((o) => `<span class="${o === cur ? "on" : ""}">${o}</span>`).join("")}</div></div>`;
      return `<h3>Model</h3><div class="bl">Inkling thinks with Claude through the Claude Agent SDK, using this Mac’s Claude Code login, so it runs on your Claude subscription. No API key.</div>` +
        `<div class="sw-group" data-m="claude" style="margin-top:16px"><div class="sw-row"><div class="grow"><div class="t" style="display:flex;align-items:center;gap:7px;font-size:13px"><span class="dot" style="width:6px;height:6px"></span>Claude</div><div class="d">Signed in as you@example.com</div></div><span style="padding:6px 12px;border-radius:8px;border:1px solid var(--rule);font-size:12px">Check again</span></div></div>` +
        `<div class="sw-sec">Model<small>From Claude Code</small></div><div class="sw-group" data-m="models">${rows}<div class="sw-row"><div class="grow d" style="font-size:12px;color:var(--ink)">${icon("right", 11, "var(--graphite)")} Specific versions (8)</div></div></div>` +
        `<div class="sw-sec">Effort<small>For ${S.model === 0 || S.model == null ? "Default (recommended)" : ["Default (recommended)", "Opus 5.5", "Fable 5.1", "Sonnet 5.5", "Haiku 5.5"][S.model]}</small></div><div class="sw-group" data-m="effort">${eff("Reading along", S.effRead || "low")}${eff("Asking, drawing", S.effAsk || "medium")}</div>` +
        `<div style="margin-top:7px;font-size:10.6px;color:var(--graphite)">Reading along runs after every paragraph, so it defaults to low effort.</div>`;
    },
    Shortcuts(S) {
      const row = (name, keys, o = {}) => `<div class="sw-row"${o.m ? ` data-m="${o.m}"` : ""} style="${o.hov ? "background:var(--wash)" : ""}"><div class="grow"><div class="t" style="font-size:13px">${name}</div>${o.sub ? `<div class="d">${o.sub}</div>` : ""}${o.rec ? `<div class="d">Press the new keys. Esc cancels.</div>` : ""}</div>${o.add ? `<span style="font-size:11.5px;color:var(--graphite);margin-right:2px">Add</span>` : ""}<span class="kc${o.rec ? " rec" : ""}">${keys}</span></div>`;
      const R = S.rec || {};
      return `<div style="display:flex;justify-content:space-between;align-items:flex-end"><h3>Keyboard shortcuts</h3><div style="display:flex;gap:10px"><span class="sw-search">${icon("search", 12, "var(--graphite)")}Search shortcuts</span><span style="padding:6px 12px;border-radius:8px;border:1px solid var(--rule);font-size:11.6px;color:var(--faint)">Reset all</span></div></div>` +
        `<div class="bl">Click keys to change them. New keys work at once, in your notes and in the menu bar.</div>` +
        `<div class="sw-sec">Notes<small>Anywhere in Inkling</small></div><div class="sw-group">${row("New note", "⌘N")}${row("Find or start a note", "⌘K")}${row("Show or hide the notes list", "⌘\\")}${row("Copy as Markdown", "⇧⌘C")}${row("Export…", "⇧⌘E")}</div>` +
        `<div class="sw-sec">Threads<small>Anywhere in Inkling</small></div><div class="sw-group" data-m="threads">${row("Find a loose thread", "⌘↵", { sub: "In a selection. With a thread open, it ties it off." })}${row("Next open thread", "⌥⌘↓")}${row("Previous open thread", "⌥⌘↑")}${row("Dismiss the open thread", "⇧⌘⌫")}${row("Undo dismiss", "⌥⌘Z")}` +
        row("Hide threads", R.hide || "⇧⌘.", { rec: R.recording, hov: R.hov, m: "hide" }) + `</div>`;
    },
  };

  /** Draws the Settings window into `host` (a positioned element in world space). */
  function render(host, S, t) {
    ensureCSS();
    const pane = S.pane || "Reading";
    const navKey = pane === "NewMind" ? "Minds" : pane;
    const nav = PANES.map(([n, ic]) => `<span class="${n === navKey ? "on" : ""}">${pi(ic, n === navKey ? "var(--voice)" : "var(--graphite)")}${n}</span>`).join("");
    const body = (PANE[pane] || (() => ""))(S, t);
    host.innerHTML = `<div class="sw-win"><div class="sw-rail"><div class="lt"><i></i><i></i><i></i></div><div class="nav">${nav}</div><div class="ft"><span>${A.WM_HTML(17)}</span><span>v0.1.0</span></div></div>` +
      `<div class="sw-sheet"><div class="sw-sc" style="transform:translateY(${fx(-(S.scroll || 0))}px)">${body}</div></div></div>`;
  }

  function measure(host, S) {
    render(host, { ...S, scroll: 0 }, 0);
    const hr = host.getBoundingClientRect(), out = {};
    host.querySelectorAll("[data-m]").forEach((el) => { const r = el.getBoundingClientRect(); out[el.dataset.m] = { x: r.left - hr.left, y: r.top - hr.top, w: r.width, h: r.height }; });
    return out;
  }
  window.SETTINGS = { render, measure };
})();

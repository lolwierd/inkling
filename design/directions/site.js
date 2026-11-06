/* Inkling directions board: shared behaviour.
   Loaded in <head> (not deferred) so the theme is set before the first paint.
   Exposes window.Board: theme, still, sound, contrast, wait, draw, note, state, fit. */
(function () {
  "use strict";
  const root = document.documentElement;
  const params = new URLSearchParams(location.search);
  const KEY = "inkling-board-theme";
  const SOUND_KEY = "inkling-board-sound";

  function load(key) { try { return localStorage.getItem(key); } catch (e) { return null; } }
  function save(key, v) { try { localStorage.setItem(key, v); } catch (e) { /* private window */ } }

  // ---- theme: ?theme= wins, then the viewer's choice, then the system ----
  const sys = window.matchMedia ? matchMedia("(prefers-color-scheme: dark)") : { matches: false };
  let theme = params.get("theme") || load(KEY) || (sys.matches ? "dark" : "light");
  root.dataset.theme = theme;

  const still = (window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches) || params.has("still");
  if (still) root.classList.add("still");
  if (params.has("qa")) root.classList.add("qa");

  const themeListeners = [];
  function setTheme(t, remember) {
    theme = t;
    root.dataset.theme = t;
    if (remember) save(KEY, t);
    document.querySelectorAll('[data-board="theme"]').forEach((b) => {
      // a toggle keeps one label; its pressed state says whether it is on
      b.setAttribute("aria-pressed", String(t === "dark"));
      b.textContent = "Dark";
      b.title = t === "dark" ? "Dark mode is on" : "Dark mode is off";
    });
    themeListeners.forEach((fn) => { try { fn(t); } catch (e) { console.error(e); } });
  }

  // ---- sound: synthesised, so the page loads no audio files; off until asked ----
  let soundOn = load(SOUND_KEY) === "on";
  let ctx = null;
  function audio() {
    if (!ctx) {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return null;
      ctx = new AC();
    }
    if (ctx.state === "suspended") ctx.resume().catch(() => {});
    return ctx;
  }
  function noise(ac, seconds) {
    const len = Math.max(1, Math.floor(ac.sampleRate * seconds));
    const buf = ac.createBuffer(1, len, ac.sampleRate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < len; i++) d[i] = Math.random() * 2 - 1;
    const src = ac.createBufferSource();
    src.buffer = buf;
    return src;
  }
  function env(ac, g, t, peak, attack, decay) {
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(peak, t + attack);
    g.gain.exponentialRampToValueAtTime(0.0001, t + attack + decay);
  }
  const SOUNDS = {
    // pencil touching paper
    tick(ac, t) {
      const n = noise(ac, 0.05), f = ac.createBiquadFilter(), g = ac.createGain();
      f.type = "bandpass"; f.frequency.value = 3800; f.Q.value = 1.4;
      env(ac, g, t, 0.35, 0.002, 0.04);
      n.connect(f).connect(g).connect(ac.destination); n.start(t); n.stop(t + 0.06);
    },
    // a sheet sliding off another
    paper(ac, t) {
      const n = noise(ac, 0.4), f = ac.createBiquadFilter(), g = ac.createGain();
      f.type = "lowpass"; f.frequency.setValueAtTime(900, t); f.frequency.linearRampToValueAtTime(2400, t + 0.3);
      env(ac, g, t, 0.12, 0.08, 0.28);
      n.connect(f).connect(g).connect(ac.destination); n.start(t); n.stop(t + 0.4);
    },
    // highlighter across a sheet
    swipe(ac, t) {
      const n = noise(ac, 0.3), f = ac.createBiquadFilter(), g = ac.createGain();
      f.type = "bandpass"; f.Q.value = 2.2;
      f.frequency.setValueAtTime(1100, t); f.frequency.exponentialRampToValueAtTime(2600, t + 0.22);
      env(ac, g, t, 0.16, 0.03, 0.2);
      n.connect(f).connect(g).connect(ac.destination); n.start(t); n.stop(t + 0.3);
    },
    // a rubber stamp on a desk
    stamp(ac, t) {
      const o = ac.createOscillator(), g = ac.createGain();
      o.type = "sine"; o.frequency.setValueAtTime(140, t); o.frequency.exponentialRampToValueAtTime(55, t + 0.16);
      env(ac, g, t, 0.55, 0.004, 0.18);
      o.connect(g).connect(ac.destination); o.start(t); o.stop(t + 0.22);
      SOUNDS.tick(ac, t);
    },
    // thread drawn through cloth
    pull(ac, t) {
      const n = noise(ac, 0.3), f = ac.createBiquadFilter(), g = ac.createGain();
      f.type = "bandpass"; f.Q.value = 7;
      f.frequency.setValueAtTime(700, t); f.frequency.exponentialRampToValueAtTime(3200, t + 0.26);
      env(ac, g, t, 0.2, 0.02, 0.24);
      n.connect(f).connect(g).connect(ac.destination); n.start(t); n.stop(t + 0.3);
    },
    // two blades meeting
    snip(ac, t) {
      [0, 0.075].forEach((dt) => {
        const n = noise(ac, 0.02), f = ac.createBiquadFilter(), g = ac.createGain();
        f.type = "highpass"; f.frequency.value = 4200;
        env(ac, g, t + dt, 0.4, 0.001, 0.015);
        n.connect(f).connect(g).connect(ac.destination); n.start(t + dt); n.stop(t + dt + 0.03);
      });
    },
    // a knot snugging tight
    pop(ac, t) {
      const o = ac.createOscillator(), g = ac.createGain();
      o.type = "sine"; o.frequency.setValueAtTime(560, t); o.frequency.exponentialRampToValueAtTime(260, t + 0.09);
      env(ac, g, t, 0.28, 0.003, 0.09);
      o.connect(g).connect(ac.destination); o.start(t); o.stop(t + 0.12);
    },
    // something settled
    chime(ac, t) {
      [660, 990].forEach((hz, i) => {
        const o = ac.createOscillator(), g = ac.createGain();
        o.type = "sine"; o.frequency.value = hz;
        env(ac, g, t + i * 0.06, 0.07, 0.01, 0.7);
        o.connect(g).connect(ac.destination); o.start(t + i * 0.06); o.stop(t + i * 0.06 + 0.75);
      });
    },
  };
  function sound(name) {
    if (!soundOn || !SOUNDS[name]) return;
    const ac = audio();
    if (!ac) return;
    try { SOUNDS[name](ac, ac.currentTime + 0.01); } catch (e) { /* audio refused */ }
  }
  function setSound(on) {
    soundOn = on;
    save(SOUND_KEY, on ? "on" : "off");
    document.querySelectorAll('[data-board="sound"]').forEach((b) => {
      b.setAttribute("aria-pressed", String(on));
      b.textContent = on ? "Sound on" : "Sound off";
    });
    if (on) { audio(); sound("tick"); }
  }

  // ---- colour: measured contrast, so the page shows its numbers ----
  function toRGB(c, resolved) {
    if (!c) return null;
    c = c.trim();
    if (c[0] === "#") {
      let h = c.slice(1);
      if (h.length === 3) h = h.split("").map((x) => x + x).join("");
      return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
    }
    let m = c.match(/^rgba?\(([^)]+)\)/);
    if (m) return m[1].split(/[ ,/]+/).filter(Boolean).slice(0, 3).map(Number);
    m = c.match(/^color\(srgb\s+([-\d.e]+)\s+([-\d.e]+)\s+([-\d.e]+)/);
    if (m) return [m[1], m[2], m[3]].map((v) => Math.round(Math.min(1, Math.max(0, parseFloat(v))) * 255));
    if (resolved) return null;
    // let the browser resolve anything else (named colours, color-mix), once
    const probe = document.createElement("i");
    probe.style.color = c; document.body.appendChild(probe);
    const out = toRGB(getComputedStyle(probe).color, true); probe.remove();
    return out;
  }
  function alphaOf(c) {
    const m = (c || "").match(/rgba\([^)]*,\s*([\d.]+)\)|\/\s*([\d.]+)\s*\)/);
    return m ? parseFloat(m[1] !== undefined ? m[1] : m[2]) : (c === "transparent" ? 0 : 1);
  }
  function lum(rgb) {
    const [r, g, b] = rgb.map((v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }
  function contrast(a, b) {
    const A = toRGB(a), B = toRGB(b);
    if (!A || !B) return 0;
    const la = lum(A), lb = lum(B);
    return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
  }
  function token(name, el) { return getComputedStyle(el || root).getPropertyValue(name).trim(); }
  function hex(c) { const v = toRGB(c); return v ? "#" + v.map((x) => x.toString(16).padStart(2, "0")).join("").toUpperCase() : c; }

  // ---- motion helpers ----
  const wait = (ms) => new Promise((r) => setTimeout(r, still ? Math.min(ms, 60) : ms));
  /** Draw an SVG stroke on, like a pen. Falls back to a fade under reduced motion. */
  function draw(el, ms, delay) {
    ms = ms || 280; delay = delay || 0;
    if (still || !el.getTotalLength) {
      el.style.opacity = 0;
      el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 120, delay, fill: "forwards" });
      return wait(120 + delay);
    }
    const len = el.getTotalLength();
    el.style.strokeDasharray = len + " " + len;
    const a = el.animate([{ strokeDashoffset: len }, { strokeDashoffset: 0 }],
      { duration: ms, delay, easing: "cubic-bezier(.2,.7,.2,1)", fill: "forwards" });
    return a.finished.then(() => { el.style.strokeDasharray = ""; }).catch(() => {});
  }

  // ---- the app mocks keep their design size and scale down to fit ----
  function fit() {
    document.querySelectorAll(".fit").forEach((box) => {
      const inner = box.firstElementChild;
      if (!inner) return;
      const w = +box.dataset.w || 1200, h = +box.dataset.h || 780;
      const s = Math.min(1, box.clientWidth / w);
      inner.style.width = w + "px"; inner.style.height = h + "px";
      inner.style.transform = "scale(" + s + ")";
      box.style.height = Math.round(h * s) + "px";
      box.style.setProperty("--fit-scale", s);
    });
  }

  // ---- the same note in every direction ----
  const note = {
    title: "Sub-50 10k by March",
    paras: [
      "Last year I ran the city 10k in 56:40. I went out too fast and walked twice after the 7 km mark. This year I want to finish under 50 minutes.",
      "The plan is to run four days a week and hold 5:30 per km on every run, so my legs learn race pace. If I hold 5:30 for the whole race, I’ll come in just under 50.",
      "Sunday is the long run. I’ll add a kilometre every week until I reach 16.",
      "Strength work twice a week, mostly calves and hips, because that’s where it fell apart last year.",
    ],
    threads: [
      {
        id: 1, kind: "correction", para: 1, mind: "Inkling",
        quote: "If I hold 5:30 for the whole race, I’ll come in just under 50.",
        short: "5:30",
        text: "At 5:30 a km, 10 km takes 55:00. Under 50 needs 5:00 a km or faster; 4:55 leaves a little room. Which do you want to change: the pace or the goal?",
        fixFrom: "5:30", fixTo: "4:55",
        settle: "4:55 a km comes to 49:10. That works.",
      },
      {
        id: 2, kind: "question", para: 2, mind: "Inkling",
        quote: "add a kilometre every week until I reach 16",
        text: "Starting from how far? If you start at 8 km, 16 km is eight weeks out. Is the race at least eight weeks away, with an easier week before it?",
        reply: "From 8. The race is 11 weeks out.",
        answer: "Then you have three weeks to spare. Spend one on an easier week before the race and the plan holds.",
      },
    ],
    held: [0, 3], // paragraphs read with nothing to ask
    drawing: {
      title: "Pace and finish time",
      steps: ["Pace per km", "Times ten is your finish", "Under 50 starts at 5:00"],
      paces: ["4:30", "4:45", "5:00", "5:15", "5:30", "5:45", "6:00"],
      finishes: ["45:00", "47:30", "50:00", "52:30", "55:00", "57:30", "60:00"],
    },
    room: {
      quote: "four days a week",
      answers: [
        { mind: "Inkling", text: "Four works if two of them are easy. Which two are hard?" },
        { mind: "Skeptic", text: "You walked after 7 km last year. Is pace the problem, or the last 3 km?" },
        { mind: "Tutor", text: "Race-pace work is usually one or two sessions a week, not every run. Want a sample week?" },
      ],
    },
    sidebar: {
      folders: [
        { name: "Running", notes: ["Sub-50 10k by March", "Shoes: what wore out where"] },
        { name: "Home", notes: ["Why the bread didn’t rise", "Kitchen shelf measurements"] },
      ],
      loose: ["Reading list for winter", "Questions for the landlord"],
      current: "Sub-50 10k by March",
    },
    firstRun: {
      prompt: "Write one sentence about something you’re working on, or something you believe.",
      example: "Our team ships faster since we stopped doing code review on Fridays.",
      question: "Faster by what measure, and compared with when? If Fridays were already your slowest day, the change might not be the cause.",
    },
    limitResets: "4:30 pm",
  };

  // ---- QA: layout facts for headless checks (?qa=1, read with --dump-dom) ----
  function bgOf(el) {
    for (let n = el; n && n.nodeType === 1; n = n.parentElement) {
      const cs = getComputedStyle(n);
      if (cs.backgroundImage && cs.backgroundImage !== "none") return null; // cannot measure a gradient or image
      if (alphaOf(cs.backgroundColor) >= 0.98 && toRGB(cs.backgroundColor)) return cs.backgroundColor;
    }
    return getComputedStyle(document.body).backgroundColor;
  }
  function describe(el) {
    let s = el.tagName.toLowerCase();
    if (el.id) s += "#" + el.id;
    if (el.classList.length) s += "." + [...el.classList].slice(0, 3).join(".");
    return s;
  }
  function runQA() {
    const out = { viewport: innerWidth, docWidth: root.scrollWidth, theme, overflowX: [], offscreen: [], lowContrast: [], fontSizes: {}, families: {}, colors: {} };
    document.querySelectorAll("body *").forEach((el) => {
      if (el.id === "qa" || el.closest("#qa")) return;
      if (el instanceof SVGElement && el.tagName.toLowerCase() !== "svg") return; // shapes inside a drawing are not layout
      const cs = getComputedStyle(el);
      if (cs.display === "none" || cs.visibility === "hidden") return;
      const r = el.getBoundingClientRect();
      if (r.width === 0 && r.height === 0) return;
      const scrolls = cs.overflowX === "auto" || cs.overflowX === "scroll";
      if (!scrolls && cs.overflowX !== "hidden" && cs.overflowX !== "clip" && el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0 && cs.display !== "inline")
        out.overflowX.push(describe(el) + " " + el.scrollWidth + ">" + el.clientWidth);
      if (r.right > innerWidth + 1 || r.left < -1) {
        if (!el.closest(".bar-nav") && !el.closest("[data-qa-skip]")) out.offscreen.push(describe(el) + " " + Math.round(r.left) + ".." + Math.round(r.right));
      }
      const ownText = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
      if (ownText && +cs.opacity !== 0) {
        out.fontSizes[cs.fontSize] = (out.fontSizes[cs.fontSize] || 0) + 1;
        const fam = cs.fontFamily.split(",")[0].replace(/["']/g, "");
        out.families[fam] = (out.families[fam] || 0) + 1;
        out.colors[hex(cs.color)] = (out.colors[hex(cs.color)] || 0) + 1;
        const bg = bgOf(el);
        if (bg && !el.closest("[aria-hidden=true]")) {
          const ratio = contrast(cs.color, bg);
          const big = parseFloat(cs.fontSize) >= 24 || (parseFloat(cs.fontSize) >= 18.66 && +cs.fontWeight >= 700);
          if (ratio < (big ? 3 : 4.5)) out.lowContrast.push(describe(el) + " " + hex(cs.color) + " on " + hex(bg) + " = " + ratio.toFixed(2) + " “" + el.textContent.trim().slice(0, 30) + "”");
        }
      }
    });
    out.overflowX = out.overflowX.slice(0, 40);
    out.offscreen = out.offscreen.slice(0, 40);
    out.lowContrast = out.lowContrast.slice(0, 60);
    let pre = document.getElementById("qa");
    if (!pre) { pre = document.createElement("pre"); pre.id = "qa"; document.body.appendChild(pre); }
    pre.textContent = "QA" + JSON.stringify(out, null, 1) + "END";
  }

  window.Board = {
    get theme() { return theme; },
    setTheme: (t) => setTheme(t, true),
    onTheme: (fn) => themeListeners.push(fn),
    still,
    sound,
    get soundOn() { return soundOn; },
    contrast, token, hex, wait, draw, fit, note,
    state: params.get("state") || "",
    params,
  };

  document.addEventListener("DOMContentLoaded", () => {
    // mark the current page in the bar
    const here = (location.pathname.split("/").pop() || "index.html").toLowerCase();
    document.querySelectorAll(".bar-nav a").forEach((a) => {
      if ((a.getAttribute("href") || "").toLowerCase() === here) a.setAttribute("aria-current", "page");
    });
    document.querySelectorAll('[data-board="theme"]').forEach((b) =>
      b.addEventListener("click", () => setTheme(theme === "dark" ? "light" : "dark", true)));
    document.querySelectorAll('[data-board="sound"]').forEach((b) =>
      b.addEventListener("click", () => setSound(!soundOn)));
    setTheme(theme, false);
    setSound(soundOn && false); // never start with sound on: a page should not make noise unasked
    fit();
    if (window.ResizeObserver) {
      const ro = new ResizeObserver(() => fit());
      document.querySelectorAll(".fit").forEach((b) => ro.observe(b));
    }
    addEventListener("resize", fit);
    if (!params.get("theme") && sys.addEventListener) {
      sys.addEventListener("change", (e) => { if (!load(KEY)) setTheme(e.matches ? "dark" : "light", false); });
    }
    if (params.has("qa")) {
      const go = () => setTimeout(runQA, 400);
      (document.fonts && document.fonts.ready ? document.fonts.ready : Promise.resolve()).then(go);
    }
  });
})();

// What a run left in a notebook folder, readable at a glance: the notes with their
// threads and chat, the dumps, the brain traffic, the app log.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { dumpsOf, readBrainLog, readLog, readStore } from "./app.ts";
import { compactDump } from "./e2e.ts";

const cut = (s: string, n = 140) => (s.length > n ? s.slice(0, n - 1) + "…" : s);

export function show(dir: string, what: string, json: boolean): number {
  if (!existsSync(dir)) {
    console.error(`no folder ${dir}`);
    return 2;
  }
  const want = (w: string) => what === "all" || what === w;
  const log = readLog(join(dir, "app.log"));
  const out: Record<string, unknown> = {};

  if (want("notes")) {
    const store = readStore(dir);
    if (json) out.store = store;
    else {
      console.log("== notes");
      for (const n of store.notes) {
        console.log(`  NOTE ${JSON.stringify(n.title)} id=${n.id} mode=${n.mode} mind=${n.mind_id} read=${(n.read_hashes ?? []).length} chat=${(n.chat ?? []).length}${n.first_run ? " first_run" : ""}`);
        for (const b of n.blocks ?? []) console.log(`    [${b.kind}] ${JSON.stringify(cut(b.text))}`);
        for (const t of n.threads ?? []) {
          console.log(`    THREAD ${t.kind}/${t.status}/${t.side} bytes ${t.start}..${t.end} by ${t.mind_name || "?"} quote=${JSON.stringify(cut(t.quote, 60))}${t.drawing ? " +drawing" : ""}`);
          for (const m of t.messages ?? []) console.log(`      ${m.role}: ${JSON.stringify(cut(m.text))}`);
        }
        for (const m of n.chat ?? []) console.log(`    CHAT ${m.role}: ${JSON.stringify(cut(m.text))}`);
      }
      const s = store.settings;
      if (Object.keys(s).length) console.log(`  settings: last_note=${s.last_note} mode=${s.default_mode} pace=${s.pace} placement=${s.placement} appearance=${s.appearance} about_me=${JSON.stringify(cut(s.about_me ?? "", 80))}`);
      if (store.minds.length) console.log(`  minds: ${store.minds.map((m: any) => m.name).join(", ")}`);
    }
  }

  if (want("dumps")) {
    const dumps = dumpsOf(log);
    if (json) out.dumps = dumps;
    else {
      console.log("== dumps");
      for (const d of dumps) console.log(`  [${d.label}] at ${d.ms ?? "?"}ms\n    ${compactDump(d.state).replace(/\n/g, "\n    ")}`);
    }
  }

  if (want("brain")) {
    const brain = readBrainLog(join(dir, "brain.log"));
    if (json) out.brain = brain;
    else {
      console.log("== brain (fake brain traffic; the real brain logs only problems)");
      for (const b of brain) {
        const d = b.data;
        if (b.dir === "recv") {
          const { ctx, ...rest } = d.input ?? {};
          console.log(`  ${b.t.toFixed(3).padStart(8)} <- ${d.op} ${d.id ?? ""} ${cut(JSON.stringify(rest), 200)}`);
        } else console.log(`  ${b.t.toFixed(3).padStart(8)} -> ${d.ev} ${d.id ?? ""} ${cut(JSON.stringify(d.data ?? d.message ?? d.kind ?? ""), 200)}`);
      }
    }
  }

  if (want("log")) {
    if (json) out.log = log;
    else {
      console.log("== app log");
      for (const r of log) console.log(`  ${String(r.ms ?? "").padStart(6)} ${r.level.padEnd(5)} ${cut(r.msg.replace(/^script dump (.*?): .*/s, "script dump $1: (see dumps)"), 300)}`);
    }
  }
  if (json) console.log(JSON.stringify(out, null, 1));
  return 0;
}

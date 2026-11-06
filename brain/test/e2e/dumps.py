import json, re, sys
d = sys.argv[1]
keys = sys.argv[2].split(",") if len(sys.argv) > 2 else ["reading", "waiting", "pending", "chat_pending", "active", "error", "threads"]
for l in open(d + "/app.log", errors="replace"):
    m = re.search(r"script dump (\S+): (.*)$", l)
    if m:
        x = json.loads(m.group(2))
        out = {}
        for k in keys:
            v = x.get(k)
            if k == "threads":
                v = [f"{t['id'][:4]}:{t['kind'][:3]}/{t['status'][:4]}/{t['side'][0]} m{t['msgs']}{' D' if t['drawing'] else ''} q={t['quote'][:24]!r}" for t in v]
            if k == "pending":
                v = [f"{p['thread'][:4]}={p['state'][:40]}" for p in v]
            if k == "slots":
                v = [f"{s['id'][:4]}:{s['side'][0]} a{s['anchor']:.0f} t{s['top']:.0f} h{s['h']:.0f}" for s in v]
            out[k] = v
        print(f"  [{m.group(1)}] note={x['note']['title']!r} " + json.dumps(out, ensure_ascii=False))
    elif "script copy" in l or "script log" in l or "WARN" in l or "ERROR" in l or "panicked" in l:
        print("  " + l.strip()[:300])

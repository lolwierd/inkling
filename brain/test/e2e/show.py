import json, glob, sys, os
d = sys.argv[1]
what = sys.argv[2] if len(sys.argv) > 2 else "all"
if what in ("all", "log"):
    print("== app.log")
    for l in open(os.path.join(d, "app.log"), errors="replace"):
        l = l.rstrip()
        if l:
            print("  " + l[:400])
if what in ("all", "brain"):
    print("== brain.log")
    p = os.path.join(d, "brain.log")
    if os.path.exists(p):
        for l in open(p, errors="replace"):
            l = l.rstrip()
            if "] recv " in l:
                try:
                    ts = l.split("]")[0].replace("[fake", "").strip()
                    req = json.loads(l.split("recv ", 1)[1])
                    inp = req.get("input", {})
                    keys = {k: v for k, v in inp.items() if k not in ("ctx",)}
                    s = json.dumps(keys, ensure_ascii=False)
                    print(f"  {ts} recv {req.get('op')} {req.get('id')} {s[:300]}")
                except Exception:
                    print("  " + l[:300])
            elif "] send " in l:
                print("  " + l[:300])
            else:
                print("  " + l[:300])
if what in ("all", "notes"):
    print("== notes")
    for f in sorted(glob.glob(os.path.join(d, "notes", "*.json")), key=os.path.getmtime):
        n = json.load(open(f))
        print(f"  NOTE {n['title']!r} mode={n['mode']} mind={n['mind_id']} hashes={len(n['read_hashes'])} chat={len(n['chat'])}")
        for b in n["blocks"]:
            print(f"    [{b['kind']}] {b['text']!r}")
        for t in n["threads"]:
            print(f"    T {t['kind']}/{t['status']}/{t['side']} {t['start']}..{t['end']} q={t['quote']!r} draw={'yes' if t.get('drawing') else 'no'}")
            for m in t["messages"]:
                print(f"       {m['role']}: {m['text'][:140]!r}")
        for m in n["chat"]:
            print(f"    chat {m['role']}: {m['text'][:140]!r}")
    s = os.path.join(d, "settings.json")
    if os.path.exists(s):
        st = json.load(open(s))
        print("  settings last_note=", st.get("last_note"), "about_me=", repr(st.get("about_me")))

"""Write a throwaway notebook with the board's running-plan note and two open threads,
for screenshots of the app. usage: demo_notebook.py <dir> [left]"""
import json, sys, uuid
from pathlib import Path

d = Path(sys.argv[1]); (d / "notes").mkdir(parents=True, exist_ok=True)
left = len(sys.argv) > 2 and sys.argv[2] == "left"
title = "Sub-50 10k by March"
paras = [
    "Last year I ran the city 10k in 56:40. I went out too fast and walked twice after the 7 km mark. This year I want to finish under 50 minutes.",
    "The plan is to run four days a week and hold 5:30 per km on every run, so my legs learn race pace. If I hold 5:30 for the whole race, I’ll come in just under 50.",
    "Sunday is the long run. I’ll add a kilometre every week until I reach 16.",
    "Strength work twice a week, mostly calves and hips, because that’s where it fell apart last year.",
    "",
]
blocks = [{"kind": "title", "text": title}] + [{"kind": "paragraph", "text": t} for t in paras]
joined = "\n".join(b["text"] for b in blocks)
def span(q):
    i = joined.index(q).__index__()
    b = len(joined[:i].encode()); return b, b + len(q.encode())
t = "2026-11-06T12:00:00Z"
q1 = "If I hold 5:30 for the whole race, I’ll come in just under 50."
q2 = "add a kilometre every week until I reach 16"
s1, e1 = span(q1); s2, e2 = span(q2)
threads = [
    {"id": "t1", "start": s1, "end": e1, "quote": q1, "kind": "correction", "status": "open", "side": "right", "mind_name": "Inkling", "created_at": t,
     "messages": [{"role": "mind", "text": "At 5:30 a km, 10 km takes 55:00. Under 50 needs 5:00 a km or faster; 4:55 leaves a little room. Which do you want to change: the pace or the goal?", "at": t}]},
    {"id": "t2", "start": s2, "end": e2, "quote": q2, "kind": "question", "status": "open", "side": "left" if left else "right", "mind_name": "Inkling", "created_at": t,
     "messages": [{"role": "mind", "text": "Starting from how far? If you start at 8 km, 16 km is eight weeks out. Is the race at least eight weeks away, with an easier week before it?", "at": t}]},
]
nid = str(uuid.uuid4())
note = {"id": nid, "title": title, "folder": None, "blocks": blocks, "marks": [], "brief": None, "mode": "thinks_with_you",
        "mind_id": "inkling", "threads": threads, "chat": [], "read_hashes": [], "created_at": t, "updated_at": t}
(d / "notes" / f"{nid}.json").write_text(json.dumps(note, ensure_ascii=False, indent=1))
(d / "settings.json").write_text(json.dumps({"last_note": nid, "placement": "both"}))
print(nid)

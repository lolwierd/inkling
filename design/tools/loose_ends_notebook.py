#!/usr/bin/env python3
"""Write the two notebooks the Loose ends scenarios start from (le01-le04), with fixed ids
and dates so runs compare:

  brain/test/e2e/fixtures/loose-ends       four notes: two in folders (one folded), one that
                                           waits to be asked, one untitled, threads open,
                                           tied off and dismissed, one thread id in two notes
  brain/test/e2e/fixtures/loose-ends-many  forty notes in four folders, 200 open threads
                                           over most of a year, 40 more tied off

Run from the repository root: python3 design/tools/loose_ends_notebook.py
Thread start/end are byte offsets into the note's blocks joined by "\\n", title included.
"""

import json
import os
from datetime import datetime, timedelta, timezone

ROOT = os.path.join(os.path.dirname(__file__), "..", "..", "brain", "test", "e2e", "fixtures")


def iso(t: datetime) -> str:
    return t.strftime("%Y-%m-%dT%H:%M:%SZ")


def note(nid, title, blocks, threads, folder=None, mode="thinks_with_you", mind="inkling", when="2026-09-01T09:00:00Z"):
    """A note whose threads are (id, quote, kind, status, mind, created, message): each quote is
    found in the joined text and its byte range stored."""
    texts = [title] + blocks
    joined = "\n".join(texts).encode("utf-8")
    out = []
    for tid, quote, kind, status, mind_name, created, message in threads:
        at = joined.find(quote.encode("utf-8"))
        assert at >= 0, (title, quote)
        out.append({
            "id": tid,
            "start": at,
            "end": at + len(quote.encode("utf-8")),
            "quote": quote,
            "kind": kind,
            "status": status,
            "side": "right",
            "mind_name": mind_name,
            "created_at": created,
            "messages": [{"role": "mind", "text": message, "at": created}],
        })
    return {
        "id": nid,
        "title": title,
        "folder": folder,
        "blocks": [{"kind": "title", "text": title}] + [{"kind": "paragraph", "text": b} for b in blocks] + [{"kind": "paragraph", "text": ""}],
        "marks": [],
        "brief": None,
        "mode": mode,
        "mind_id": mind,
        "threads": out,
        "chat": [],
        "read_hashes": [],
        "created_at": when,
        "updated_at": when,
    }


def write(name, notes, folders, minds, settings):
    d = os.path.join(ROOT, name)
    os.makedirs(os.path.join(d, "notes"), exist_ok=True)
    for f in os.listdir(os.path.join(d, "notes")):
        if f.endswith(".json"):
            os.remove(os.path.join(d, "notes", f))
    for n in notes:
        with open(os.path.join(d, "notes", n["id"] + ".json"), "w") as fh:
            json.dump(n, fh, indent=1, ensure_ascii=False)
            fh.write("\n")
    for fname, value in (("folders.json", folders), ("minds.json", minds), ("settings.json", settings)):
        with open(os.path.join(d, fname), "w") as fh:
            json.dump(value, fh, indent=1, ensure_ascii=False)
            fh.write("\n")


SKEPTIC = {"id": "skeptic", "name": "Skeptic", "description": "Asks for the evidence.", "how_it_thinks": "Question claims.", "voice": "plain", "frequency": "when_it_matters"}


def loose_ends():
    plan_id = "00000000-0000-4000-8000-0000000000d1"
    plan = note(
        plan_id,
        "Sub-50 10k by March",
        [
            "Last year I ran the city 10k in 56:40. I went out too fast and walked twice after the 7 km mark. This year I want to finish under 50 minutes.",
            "The plan is to run four days a week and hold 5:30 per km on every run, so my legs learn race pace. If I hold 5:30 for the whole race, I’ll come in just under 50.",
            "Sunday is the long run. I’ll add a kilometre every week until I reach 16.",
            "Strength work twice a week, mostly calves and hips, because that’s where it fell apart last year.",
        ],
        [
            ("t1", "If I hold 5:30 for the whole race, I’ll come in just under 50.", "correction", "open", "Inkling", "2026-10-10T09:00:00Z",
             "At 5:30 a km, 10 km takes 55:00. Under 50 needs 5:00 a km or faster. Which do you want to change: the pace or the goal?"),
            ("t2", "add a kilometre every week until I reach 16", "question", "open", "Inkling", "2026-10-12T09:00:00Z",
             "Starting from how far? If you start at 8 km, 16 km is eight weeks out. Is the race at least eight weeks away?"),
        ],
        folder="f-run",
        when="2026-10-01T09:00:00Z",
    )
    budget = note(
        "00000000-0000-4000-8000-0000000000d2",
        "Q3 budget",
        [
            "We cut the cloud bill by moving the cache to one node.",
            "Hiring two more engineers pays for itself by March.",
            "The support rota stays as it is.",
        ],
        [
            ("b1", "We cut the cloud bill by moving the cache to one node.", "question", "open", "Skeptic", "2026-09-20T09:00:00Z",
             "What happens to the bill, and to you, the day that one node goes down?"),
            ("b2", "pays for itself by March", "question", "resolved", "Skeptic", "2026-09-02T09:00:00Z",
             "On what numbers? Two salaries need a lot of saved hours by March."),
            ("b3", "stays as it is", "question", "dismissed", "Skeptic", "2026-09-01T09:00:00Z",
             "Has anyone asked the people on it?"),
        ],
        folder="f-work",
        mode="when_you_ask",
        mind="skeptic",
        when="2026-09-01T09:00:00Z",
    )
    untitled = note(
        "00000000-0000-4000-8000-0000000000d3",
        "",
        ["Every meeting should end with a decision or be cancelled."],
        [
            # the same id as a thread in the running plan: rows go by note and thread together
            ("t2", "Every meeting should end with a decision", "question", "open", "Editor", "2026-10-20T09:00:00Z",
             "Every one? What about the meetings whose point is to listen?"),
        ],
        when="2026-10-20T09:00:00Z",
    )
    groceries = note("00000000-0000-4000-8000-0000000000d4", "Groceries", ["Eggs, oats, coffee."], [], when="2026-10-21T09:00:00Z")
    write(
        "loose-ends",
        [plan, budget, untitled, groceries],
        [{"id": "f-run", "name": "Running", "collapsed": False}, {"id": "f-work", "name": "Work", "collapsed": True}],
        [SKEPTIC],
        {"last_note": plan_id, "placement": "both"},
    )


SENTENCES = [
    "The long run moves to Saturday so Sunday can be a rest day.",
    "We will cache the search results for an hour and nobody will notice.",
    "Reading twenty pages a night gets me through a book a fortnight.",
    "The tomatoes go in the bed by the fence, where it is warmest.",
    "Interval sessions at 4:40 per km build the speed for race day.",
    "Moving the build to bigger machines halves the wait for everyone.",
    "A weekly review on Friday afternoon keeps the inbox at zero.",
    "The kitchen shelves can hold the cookbooks if we drop the top one.",
    "Two easy runs a week are enough to keep the base mileage up.",
    "The migration is safe because every write goes to both databases.",
    "Notes in the margin help me remember more of what I read.",
    "Watering in the evening loses less to the sun than the morning does.",
]
FOLDERS = [("f-run", "Running"), ("f-work", "Work"), ("f-read", "Reading"), ("f-home", "Home")]
MINDS = ["Inkling", "Skeptic", "Editor", "Staff engineer", "Socratic tutor"]


def loose_ends_many():
    start = datetime(2026, 1, 5, 9, 0, tzinfo=timezone.utc)
    notes = []
    for n in range(40):
        fid, fname = FOLDERS[n % 4]
        blocks = [f"{SENTENCES[(n + k) % len(SENTENCES)][:-1]}, in week {n + 1} of the plan." for k in range(6)]
        threads = []
        for k in range(6):
            created = iso(start + timedelta(hours=(n * 6 + k) * 25))
            status = "resolved" if k == 5 else "open"
            quote = blocks[k][:-1]
            kind = "correction" if (n + k) % 7 == 0 else "question"
            threads.append((f"m{n:02d}-{k}", quote, kind, status, MINDS[(n + k) % len(MINDS)], created, "Which part of this would you check first?"))
        notes.append(note(
            f"00000000-0000-4000-8000-0000000002{n:02d}",
            f"{fname} notes, week {n + 1}",
            blocks,
            threads,
            folder=fid,
            when=iso(start + timedelta(days=n)),
        ))
    write("loose-ends-many", notes, [{"id": f, "name": name, "collapsed": False} for f, name in FOLDERS], [SKEPTIC], {"last_note": notes[0]["id"], "placement": "both"})


if __name__ == "__main__":
    loose_ends()
    loose_ends_many()
    print("wrote loose-ends and loose-ends-many")

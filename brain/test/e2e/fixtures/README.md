# Notebook fixtures

Each folder is a notebook (`INKLING_DATA_DIR`) that a scenario starts from with
`# fixture: <name>`, or a look starts from with `scripts/dev state --fixture <name>`. The
runner copies the folder; nothing here is changed by a run. Without a fixture a run starts
from an empty notebook, which opens the first-run note ("Start here").

- `beta`: one note titled "Beta" with an unread paragraph, for `switch: Beta`.
- `corrupt`: a note whose thread has an impossible byte range (start after end).
- `mind`: no notes, one custom mind "Skeptic" in minds.json.
- `sewn-blocks`: "Race week", one of each of your blocks: inline code, a checklist, a callout of each kind, a rust code block with a question on `Pace::new(4, 55)` (thread `c1`) and a long line that wraps, and a quote; for looking at them (`scripts/dev shot --fixture sewn-blocks`); fixed id `00000000-0000-4000-8000-0000000000b1`.
- `running-plan`: the design board's "Sub-50 10k by March" note with two open threads (a correction on "5:30", a question on the long run); fixed id `00000000-0000-4000-8000-0000000000a1`.
- `hard-content`: content that tests the layout: "Quarterly planning…", a title that wraps three times, words and a path with no spaces, twelve open threads from six minds (one with a long name, `h03`) plus one tied off and one dismissed, two threads on one line made out of reading order (`h11`, `h12`), an untitled empty note, a sixty-paragraph note, twenty more notes, a folder with a long name, a folded folder and an empty one; fixed id `00000000-0000-4000-8000-0000000000c1`.
- `show-working`: the running plan with its correction's working (a sum from the writer's numbers, thread `t1`), and "Training week", a correction backed by the writer's own words (`w1`); for looking at Show its working (`scripts/dev shot --fixture show-working --step "hover: 1"`); fixed ids `00000000-0000-4000-8000-0000000000d1` and `…d2`.
- `stale`: a note whose thread quote ("alpha beta") appears twice, stored nearest the second.
- `loose-ends`: for Loose ends (`le*` scenarios): "Sub-50 10k by March" in folder Running (open, threads `t1`, `t2`), "Q3 budget" in the folded folder Work, waiting to be asked, its mind the Skeptic (`b1` open and oldest, `b2` tied off, `b3` dismissed), an untitled note whose open thread is also called `t2`, and "Groceries" with none; ids `…d1` to `…d4`. Written by `design/tools/loose_ends_notebook.py`.
- `loose-ends-many`: forty notes in four folders, 200 open threads (`m00-0` … `m39-4`) opened a day apart through 2026, and 40 tied off; same script.

Ids and timestamps are fixed so runs compare. Thread `start`/`end` are byte offsets in the
note's blocks joined by `\n` (title included): edit a fixture's text and they must move
with it. `design/tools/demo_notebook.py` wrote `running-plan` (its id was then fixed).
- `between`: threads between notes. "Race calendar" (`…e1`, written 3 Sep 2026: a heading and three paragraphs, the bib pickup on Friday 13 March among them), "Race-week plan" (`…e2`, with thread `b1` on "Sunday 22 March" already joined to the calendar's "Sunday 15 March" in its `elsewhere`, not yet placed), "March budget" (`…e3`, shares only the month) and "Journal" (`…e4`, waits to be asked, and shares the bib words, so it must never be sent).
  Its `minds.json` holds one mind of your own, "Skeptic", so a note can have a mind other than the thread's (t20).

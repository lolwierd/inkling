# Notebook fixtures

Each folder is a notebook (`INKLING_DATA_DIR`) that a scenario starts from with
`# fixture: <name>`, or a look starts from with `scripts/dev state --fixture <name>`. The
runner copies the folder; nothing here is changed by a run. Without a fixture a run starts
from an empty notebook, which opens the first-run note ("Start here").

- `beta`: one note titled "Beta" with an unread paragraph, for `switch: Beta`.
- `corrupt`: a note whose thread has an impossible byte range (start after end).
- `mind`: no notes, one custom mind "Skeptic" in minds.json.
- `running-plan`: the design board's "Sub-50 10k by March" note with two open threads (a correction on "5:30", a question on the long run); fixed id `00000000-0000-4000-8000-0000000000a1`.
- `stale`: a note whose thread quote ("alpha beta") appears twice, stored nearest the second.

Ids and timestamps are fixed so runs compare. Thread `start`/`end` are byte offsets in the
note's blocks joined by `\n` (title included): edit a fixture's text and they must move
with it. `design/tools/demo_notebook.py` wrote `running-plan` (its id was then fixed).

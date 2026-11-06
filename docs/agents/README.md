# Inkling, built for agents: what that means here and what was done

The owner's instruction (6 Nov 2026): build for agents, not for humans; write the codebase
for the agent, not for the owner. This page says what I took that to mean, what I built,
and what I chose not to do. The map itself is `AGENTS.md`.

## The bottom line

An agent arrives in this repo with no memory. To be fast and correct it has to do four
things that a person does from habit, and before this branch each of them was hard:

| An agent has to | Before | Now |
|---|---|---|
| Learn the codebase in one read | 20k lines; a six-line module list in `README.md` | `AGENTS.md`: rules, commands, the map, how a question reaches the margin, invariants, quirks |
| Not hurt anyone by accident | Rules lived in people's heads | The app refuses a script without a scratch notebook; every tool run is scratch notebook + fake brain + background window |
| Know whether a change works | 32 scenarios that drove the real app and printed JSON, and checked nothing | `scripts/dev check` (20 s) and `scripts/dev e2e`: scenarios with `expect:` lines that fail the run |
| See the running app | Run it by hand and look | `scripts/dev state` prints the page as JSON; `scripts/dev shot` prints a PNG path |

"For agents" did not mean less for people. Everything here is plain text, plain commands
and plain words; a person can use all of it. It meant choosing, at each fork, the option
that works for a reader with no memory, no screen and no patience for guessing.

## Decisions

### 1. `AGENTS.md` is the map, and `CLAUDE.md` imports it

`AGENTS.md` is the file Codex, Cursor, Amp and other agents read; Claude Code reads
`CLAUDE.md` and follows `@AGENTS.md` imports. One file serves all of them, so there is
no second copy to drift. `CLAUDE.md` adds four lines only Claude needs (it can Read a
PNG; long runs go in the background; the two big files have `// -----` banners as a
table of contents).

The map is loaded into every session, so every line in it costs every agent tokens on
every turn. It is about 170 lines, dense, and points to `docs/agents/` for what is needed
only sometimes: `recipes.md` (adding a feature end to end, a setting, a fixture,
debugging a scenario), `testing.md` (the five test layers and the scenario format).

The map is checked: `scripts/dev lint` fails when `AGENTS.md` names a path that does not
exist or when a module in `src/` or `brain/src/` is missing from it. A map that is wrong
is worse than none, because an agent trusts it.

### 2. One command door: `scripts/dev`, in TypeScript on Bun

Every check and every look at the app goes through `scripts/dev <command>`: one help
text, one set of safe defaults, `--json` on every command that reports, exit codes that
mean something. I considered:

- `just`: not installed on this machine; an agent would have to install it first.
- `make`: poor at arguments, and the logic (expectations, screenshots, a lock) is real code.
- Python: fine, but the brain and the fake brain are TypeScript, and the `expect:` lines
  are JavaScript expressions that Bun evaluates in-process. Bun is already required to
  run the app and starts in about 20 ms.

The parts live in `scripts/lib/` (`app.ts` launches and reads runs, `e2e.ts` runs and
checks scenarios, `lint.ts`, `show.ts`, `setup.ts`).

### 3. The scenarios became tests

The 32 scenarios in `brain/test/e2e/scenarios/` were the most valuable thing an agent
could have here: they drive the real app through the same code paths a person does, with
a fake brain whose answers are chosen by directives in the typed text. But they only
printed the page state; someone had to read it and remember what it should say. Now:

- **`expect:` lines** under a `dump:` are JavaScript expressions checked after the run,
  against that dump, the notebook files the app saved, the brain traffic and the log.
  JavaScript because agents write it fluently and it needs no new language; after the
  run because the app cannot evaluate expressions and files are only final at exit.
  The app's script parser skips these lines, so the app did not change for them.
- **Header lines** (`# fixture:`, `# after:`, `# env:`, `# timeout:`, `# brain: real`,
  `# allow-warn:`, `# tags:`) carry what each scenario needs. Before, which scenario
  needed the Beta fixture, which one relaunched on another's notebook and which needed a
  slow fake read was known only to whoever wrote it.
- **A step that only warns fails the run.** The app skips an unknown step or a phrase
  that is not in the note with a warning and carries on, so a typo made a scenario pass
  while testing nothing. Unknown steps and expectations that do not parse are caught
  before any window opens.

Writing the expectations from the dumps found three scenarios that had stopped testing
what they say:

- `u03-wait-while-typing` slowed the fake `read` so a question would land mid-typing.
  Since the first-run feature, a new notebook's first sentence is a `probe`, so the
  question came back in 0.2 s and `waiting_tags` was always empty. It now slows the probe,
  and checks that the question waits folded and opens on the pause.
- `t01-lifecycle` and `t02-selection` dumped "sketching" states after the drawing had
  already arrived (the fake draw takes 0.2 s), and `t02`'s "ask-streaming" and `t03`'s
  "chat-streaming" landed within 50 ms of the answer. Their fake delays are now long
  enough that each dump sees the state its label names, with about a second to spare.

### 4. Screenshots are taken from outside the app

`shot: label` in a scenario, or `scripts/dev shot`. The app only logs the step and holds
still for one second; the runner finds the window by process id (CoreGraphics through
JavaScript for Automation, 0.2 s) and captures it with `screencapture -l`. The
alternatives were capture inside the app (Objective-C or CoreGraphics FFI in the product,
for something only tools use) and GPUI's headless test platform (the `test-support`
feature builds a second copy of GPUI, and the page needs its fonts, store and brain wired
up before it can paint). Outside capture needed two lines in the script parser.

### 5. Tool runs keep the window drawing

macOS stops drawing a window that is covered, which is where an agent's window always
is. Measured on this branch: the same scenario dumped `"slots": []` without
`INKLING_JIGGLE` and the real tag position with it. It is not only cosmetic:
`pick_side`, which decides the margin a new thread goes to, reads the editor's layout,
which only updates while the window draws. `scripts/dev` sets `INKLING_JIGGLE=120`; the
cost is a window width that wobbles by one pixel.

### 6. Safety in code where a mistake is silent and permanent

`INKLING_SCRIPT` without `INKLING_DATA_DIR` now exits with a message. A script types into
notes and starts new ones; run against the default folder it would edit the owner's
notebook, and nothing would say so. This is the one rule enforced in the app. The other
rules (fake brain, background window, no reformatting) are enforced by the tools being
the easy path and stated in `AGENTS.md`.

### 7. Logs a program can read, without breaking the old reader

`INKLING_LOG_JSON=1` makes every log line a JSON object with milliseconds since launch.
The default lines have one-second timestamps and can span lines (panics, errors), which
is fragile to parse and useless for timing. It is opt-in, so `brain/test/e2e/run.sh` and
`dumps.py`, which other sessions use, keep working.

### 8. The dump says what the screen shows

The `dump:` JSON computed the page geometry with a different formula from the renderer
(it left out the sheet's padding), so `metrics` and the margin `slots` it reported were
not what was on screen. It now uses the renderer's formula; two lines in `debug_state`.

### 9. Wait for the page, not the clock

The scenarios waited on wall-clock time ("type, then wait 3.5 s"), which leaves about a
second to spare, and this machine often runs several agents' builds and a video render at
once. A new step, `settle`, waits until the page has nothing scheduled or in flight (no
pause pending, nothing reading, no reply, sketch or chat on its way, not typing), up to
20 s, and fails the run if it gives up. `scripts/dev state` and `shot` settle before they
look, so `--step "type: ..."` returns the page after the read, not before it. This was
the one change that needed code in `src/app.rs` and `src/note_page.rs`, which other
agents were editing: a `settled()` predicate next to `typing_recently` and a branch in
`run_script`, 25 lines together. The older scenarios keep their clock waits (they pass);
new ones should settle.

For the clock waits that remain, `scripts/dev e2e` takes a lock under
`$TMPDIR/inkling-agent`, so two agents' suites do not slow each other into failures, and
runs scenarios one at a time unless `--jobs` says otherwise. A full run takes about
7.5 minutes (two of them are the read-timeout scenario), so it belongs in the background;
`scripts/dev e2e smoke` takes about a minute.

### 10. Small guards that catch drift early

- `brain/test/parity.test.ts`: the brain's protocol, the app's requests and the fake
  brain must list the same tasks. Adding a task to one and not the others is the usual
  way a new feature passes unit tests and then cannot be tested in the app.
- `scripts/dev lint` also checks the owner's word rule (none of Riddled's words in any
  string the app shows or sends to the model, or in `README.md`), with the stored values
  that must keep an old spelling listed and explained.
- Fixtures have fixed ids, so expectations can name threads (`d.active === "t1"`). The
  design board's running-plan note is now fixture `running-plan`.

### 11. Starting work takes one command

The brief for this branch had to spell out three commands to make a usable worktree
(add it, then clone `target/` and `brain/node_modules` from the main checkout, without
which the first build takes minutes and every brain test fails). `scripts/dev worktree
NAME` does all three. `scripts/dev doctor` says what is missing and what else is running.

### 12. A project skill for running the app

`.claude/skills/run-inkling/` tells Claude Code how to launch the app safely. Claude's
generic "run the app" behaviour looks for a project skill first; without one it would
reach for `cargo run`, which takes the owner's focus and opens their real notebook.

## What I did not do, and why

- **No rewrite of the old scenarios to `settle`.** They pass with their clock waits, and
  several catch in-flight states on purpose; converting them is churn with no new check.
- **No headless GPUI tests.** See decision 4.
- **No renaming of `Step::Riddle`, `Step::Visualize` or `tb-riddle`.** They are
  identifiers, never shown, in files under active edit. Scenarios now use the `probe` and
  `sketch` step names; the parser still reads the old ones.
- **No permission allowlist or hooks in `.claude/settings.json`.** They would change how
  every session in this repo runs, which is the owner's call.
- **No formatter.** rustfmt is not installed and the code is not rustfmt-shaped;
  reformatting would collide with every parallel branch.
- **`brain/test/e2e/run.sh`, `dumps.py` and `show.py` stay** for sessions that use them.
  `scripts/dev e2e` and `scripts/dev show` do the same and more.

## Measured on 6 Nov 2026 (this machine, while other agents built)

| What | Time |
|---|---|
| `cargo build`, nothing changed | 0.7 s |
| `cargo build` in a fresh worktree with `target/` cloned | 16 s |
| `cargo test` (117 tests) | 7.4 s |
| `bun test test/` (100 tests) | 5 s |
| `scripts/dev check` | about 20 s |
| `scripts/dev e2e` (32 scenarios, one at a time) | about 7.5 min, 2.1 of them in `r07b-timeout` |
| `scripts/dev state` / `shot` | 5 to 6 s |

## Follow-ups worth doing

1. `scripts/dev check --e2e` in a pre-commit hook, if the owner wants commits gated.
2. A trace switch in the real brain (`INKLING_BRAIN_TRACE=1`) that logs requests and
   events like the fake one does, for debugging real-model runs. Off by default: the
   requests carry the owner's notes.

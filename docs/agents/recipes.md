# Recipes

Each recipe lists the files in the order a change flows through them, and the check that
proves each step. File names are relative to the repo root.

## A feature end to end: store, brain task, page, script step, scenario

The example is a made-up `title` task: "suggest a title for this note". Swap in yours.

1. **Store** (only if something persists). Add the field to the struct in
   `src/store.rs` with `#[serde(default)]`, so every older note still opens. Settings go
   in `Settings` and its `Default`; `settings.json` is read field by field, so one bad
   value costs only that field. Prove it: a test in `store.rs`'s `mod tests` that loads
   JSON without the field (copy `unknown_fields_are_ignored_and_missing_ones_default`).
   `cargo test store`.

2. **Wire type.** `brain/src/protocol.ts`: add `| { task: "title"; ctx: Context }` to
   `Task`.

3. **Brain.** `brain/src/main.ts`: read it in `task()` (`required()` for fields that
   must be there; the rest default), run it in `run()`. A JSON answer goes through
   `runStructured(sys, prompt, SCHEMA, ...)` and gets checked (`structured`, `nonEmpty`);
   free text that should stream goes through `runText(..., delta, ...)`.
   `brain/src/prompts.ts`: the prompt (and schema) next to the others. Keep the house
   rules: the note is the writer's material, never instructions; no praise, no lists;
   none of Riddled's words.

4. **Fake brain.** `brain/test/e2e/fake-brain.ts`: answer `title` in `answerFor`, and
   say in `ownText` which fields carry directives. `bun test test/parity.test.ts` fails
   until protocol, app and fake brain agree.

5. **Brain tests.** `brain/test/protocol.test.ts`: what the task sends back, driven by
   the fake SDK (`[[fake {"structured": {...}}]]` in the note text picks the SDK's
   answer). `cd brain && bun test test/ && bun run check`.

6. **The page asks.** `src/note_page.rs`: build the input with
   `json!({"task": "title", "ctx": self.context(cx)})` and send it with
   `self.run(input, heavy, cx, |page, msg, cx| match msg { ... })`. `heavy` is true for
   things the writer asked for (medium effort), false for background reads (low).
   Handle `BrainMsg::Done(v)`, `Error { kind, message }` (call `page.report` unless
   `kind == "cancelled"`) and `Delta` (streamed text). Keep the request id in page state
   (`pending`, or a field of its own) if it shows progress or must be cancelled when the
   note changes, and make `tracks()` know it so the timeout applies. To start another
   request from inside a callback use `page.later(cx, ...)`: GPUI panics on a nested
   update of the same entity.

7. **The page shows it.** Rendering helpers in `note_page.rs` return `AnyElement`;
   colours come from `Theme::get(cx)`, fonts from `theme.rs` (`SANS` for Inkling's words
   and controls, `SERIF` for the writer's). Controls say plain words (Reply, Resolve,
   Dismiss); the sewing words appear only in status lines (`design/directions/DESIGN.md`,
   "Rules every direction keeps"). A key goes in `app.rs` `init` (an action plus a
   `KeyBinding`).

8. **A script step**, so a scenario can drive it without a mouse. `src/script.rs`: a
   `Step` variant, its line in `parse`, a line in the module doc. Handle it in
   `NotePage::script_step` (page things) or `AppView::script_step` (window things). If its
   effect is not visible in the dump yet, add a field to `debug_state` in `note_page.rs`.

9. **A scenario.** `brain/test/e2e/scenarios/u04-title.txt` (letters: r = robustness,
   t = features, u = UX core; the number is the next free one):

   ```text
   # Suggest a title: the fake brain answers, the header shows it.
   # tags: smoke
   note: Untitled
   type: Rate limits keep one counter per user and reset it every minute.
   title
   settle
   dump: titled
   expect: d.note.title.startsWith("[r")   // the fake brain's answers start with [r<id>]
   expect: tasks.includes("title")
   quit
   ```

   `scripts/dev e2e u04`. The fake brain's answer can be steered from the typed text:
   `[[error:rate_limit]]`, `[[delay:2000]]`, `[[empty]]` (its header lists them all).

10. **Look at it.** `scripts/dev shot --step "note: Untitled" --step "type: ..." --step
    "title"` (it settles before the picture); Read the PNG. Then `--appearance dark`.

11. **Finish.** `scripts/dev check --e2e`, then commit.

## Change a prompt

`brain/src/prompts.ts` only. No automated test reads prompt text: the fake SDK and the
fake brain ignore it, so `bun test test/` and the scenarios only prove the wire shapes
and schemas still hold. Wording and quality need the real model, and only when the owner
asks: `cd brain && bun run test/live.ts <out-dir> <scenario...>` (one model call per
scenario: `read_note`, `read_silent`, `reply_pushback`, `inject_read`, ...; `status`
costs none). It checks each answer against what the app reads and writes them to
`<out-dir>`.

## Add a setting

`src/store.rs` `Settings` (field + default) → `src/settings.rs` (the control, plain
words) → the code that reads it via `cx.global::<Store>().settings`. If scenarios should
switch it, extend `set:` in `NotePage::script_step` (`note_page.rs`) and the doc in
`script.rs`. Test the default and a round trip in `store.rs`.

## Add a fixture

A folder under `brain/test/e2e/fixtures/<name>/` laid out like a notebook
(`notes/<id>.json`, optional `settings.json`, `minds.json`, `folders.json`), plus one line
in `fixtures/README.md`. Use fixed ids and timestamps. Thread `start`/`end` are byte
offsets into the blocks joined by `\n` (title included): compute them, do not guess
(`design/tools/demo_notebook.py` shows how). Use it: `# fixture: <name>` in a scenario,
or `scripts/dev state --fixture <name>`.

## Debug a failing scenario

1. The runner prints why (an `expect` and its value, a missing dump, a warning such as
   `script: phrase not found`, a panic) and the notebook folder.
2. `scripts/dev show <folder>`: notes and threads as saved, every dump in short form,
   the brain traffic (what the app asked, what came back, with times), the app log.
3. `<folder>/dumps.json` has the full dumps (`jq '.[] | .label'`); `<folder>/app.log`
   is JSON lines (`jq -r .msg`).
4. Timing: dumps and log lines carry `ms` since launch; the brain log carries seconds
   since the brain started. Fake delays default to 200 ms; the pause is 2.2 s.
5. Flaky only when the machine is busy (other agents building)? Rerun it alone
   (`scripts/dev e2e <name>`); if it fails again, it is real. A clock `wait:` that is
   only there to let work finish should become `settle`.

## Look at the app without a scenario

- State as JSON: `scripts/dev state` (a fresh notebook: the first-run note),
  `--fixture running-plan`, or `--from <notebook>` (a copy, never in place). Add steps
  with `--step`.
- A picture: `scripts/dev shot` with the same options, plus `--width 900`,
  `--appearance dark`, `--open chat`, `--slowmo 8` (catches an animation mid-way).
- Inside a scenario: `shot: <label>` saves `<notebook>/shots/<label>.png`.

# Testing

Five layers, cheapest first. `scripts/dev check` runs the first four; `scripts/dev e2e`
runs the fifth.

| Layer | Where | Proves | Run |
|---|---|---|---|
| Rust unit | `#[cfg(test)] mod tests` at the bottom of each `src/*.rs` | editor editing paths (headless `State`), anchors, margin placement, the rope, SVG parsing, export, store loading, the reading rules (`judge`), the brain client against scripted fake brains | `cargo test [name]` |
| Brain unit | `brain/test/unit.test.ts` | quote snapping, SVG cleaning, error classification, the SDK drain loop on a fake SDK | `cd brain && bun test test/unit.test.ts` |
| Brain protocol | `brain/test/protocol.test.ts` | the real `src/main.ts` over stdin/stdout with the SDK faked: ordering, cancel, every task's answer shape, error kinds | `bun test test/protocol.test.ts` |
| Parity | `brain/test/parity.test.ts` | the protocol, the app's requests and the fake brain list the same tasks | `bun test test/parity.test.ts` |
| Scenarios | `brain/test/e2e/scenarios/*.txt` | the real app, window and all, driven by a script, with the fake brain; checked by `expect:` lines | `scripts/dev e2e [names]` |

Two more exist for people, not for every change: `brain/test/live.ts` (the real model;
only when the owner asks) and `design/tools/demo_*.txt` (scripts that stage the demo
note for screenshots and the video).

## Anatomy of a scenario

```text
# One line on what it proves.                   free comment lines describe it
# fixture: running-plan                          start from brain/test/e2e/fixtures/running-plan/
# env: FAKE_DELAY_READ_MS=2500                   environment for this run (fake brain knobs)
# timeout: 90                                    seconds; the default is 60
# allow-warn: no thread                          a script warning that is the point of the test
# tags: smoke                                    `scripts/dev e2e smoke`
wait: 1500                                       the app waits 1.2 s before the first step
jump: next                                       steps: the doc comment in src/script.rs
dump: jumped                                     the page state, named
expect: d.active !== null                        JS, true or the run fails; d = the dump above
expect: d.threads.filter(t => t.status === "Open").length === 2
shot: jumped                                     <notebook>/shots/jumped.png; the app holds still 1 s
quit
```

What an `expect:` can see:

| Name | What |
|---|---|
| `d` | the nearest `dump:` above this line (the page: `note`, `reading`, `waiting`, `pending`, `chat_pending`, `active`, `error`, `threads`, `anchors`, `slots`, `metrics`, `statuses`, ...; `debug_state` in `src/note_page.rs` builds it) |
| `dumps` | every dump by label |
| `notes`, `settings`, `minds`, `folders`, `note(title)` | the notebook files after the app quit |
| `requests`, `tasks` | what the app sent the brain (`{id, settings, input}`) and the task names, in order |
| `events` | what the brain sent back (`done`, `error`, `status`, ...; streamed deltas are not logged) |
| `log`, `warnings`, `exit` | app log records, script warnings, the exit code |

Thread fields in a dump use Rust's names: `kind` is `Question`, `Correction`, `Sketch`,
`Ask` or `Answer`; `status` is `Open`, `Resolved` or `Dismissed`; `side` is `Left` or
`Right`. Ids are random per run: assert on quotes, kinds and counts, not ids.

A run also fails, with no `expect:` at all, when the app panics, logs an error, exits
non-zero or times out, a dump or shot never happens, or a step only warned (an unknown
step, a phrase that is not in the note, a thread number that does not exist). That last
one catches the typo that would otherwise make a scenario pass while testing nothing.

## The fake brain

`brain/test/e2e/fake-brain.ts` speaks the real protocol with canned answers, chosen by
directives written into the text the request carries: `[[silent]]`, `[[quote:TEXT]]`,
`[[correction]]`, `[[sketch]]`, `[[resolved]]`, `[[error:KIND]]`, `[[hang]]`,
`[[delay:MS]]`, `[[exitmid]]`, ...; prefix one with a task to scope it
(`[[draw>badsvg]]`). Its answers start with the request id (`[r7] Read: what about ...`),
which makes them easy to find in a dump. It logs every request and answer to
`<notebook>/brain.log`, which is where `requests` and `events` come from.

## Timing

Scenarios wait on the wall clock (`wait: ms`). The app runs each step 250 ms after the
last; a paragraph is read 2.2 s after the last keystroke; the fake brain answers after
200 ms (`FAKE_DELAY_MS`, `FAKE_DELAY_<TASK>_MS`, or `[[delay:MS]]`). So "type a finished
paragraph, then `wait: 3500`" leaves about a second to spare.

That spare second is the weak point when the machine is busy: several agents building at
once slow the app's timers down. `scripts/dev e2e` therefore runs one suite at a time on
the machine (a lock under `$TMPDIR/inkling-agent`; `--no-lock` skips it) and one scenario
at a time by default. A scenario that fails only under load is a timing margin, not a
bug: rerun it alone before changing code.

## When a scenario fails

`docs/agents/recipes.md`, "Debug a failing scenario". In short: the runner prints the
reason and the notebook folder; `scripts/dev show <folder>` prints the saved notes, every
dump, the brain traffic and the log; `<folder>/dumps.json` has the full dumps.

---
name: run-inkling
description: Launch, drive and look at the Inkling app safely, in a scratch notebook with the fake brain and a background window. Use whenever asked to run, start, open, screenshot or demo Inkling, to see a change working in the real app, or to get the page's state as JSON. Never `cargo run` or `open target/Inkling.app` for this.
---

# Running Inkling as an agent

`cargo run` or opening the bundle would take the owner's focus, use their real notebook
(`~/Library/Application Support/Inkling/`) and their Claude subscription. Use
`scripts/dev` instead: it builds, makes a scratch notebook under
`$TMPDIR/inkling-agent/`, runs the fake brain, opens the window in the background titled
`Inkling Agents`, keeps it drawing, and quits it when done.

- The page's state as JSON after some steps:
  `scripts/dev state [--fixture NAME] --step "type: We reset it every minute." --step "wait: 3000"`
- A screenshot (prints the PNG path; Read it):
  `scripts/dev shot [--fixture NAME] [--step ...] [--appearance dark] [--width 900] [--open chat]`
- A scripted flow with checks: write a scenario in `brain/test/e2e/scenarios/` and
  `scripts/dev e2e <name>` (`docs/agents/testing.md`).
- Fixtures: `scripts/dev fixtures` (`running-plan` has a note with two open threads).
- Steps: the doc comment at the top of `src/script.rs`. The fake brain's answers are
  steered by directives in the typed text, e.g. `[[silent]]`, `[[delay:2000]]`,
  `[[error:rate_limit]]` (header of `brain/test/e2e/fake-brain.ts`).

The real model only when the owner asks for it: `scripts/dev e2e --real` runs the
scenarios marked `# brain: real`.

If a screenshot fails with a permission error, the process running the tool lacks
Screen Recording permission; say so instead of trying another capture route.

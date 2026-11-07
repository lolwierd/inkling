# Working on Inkling

[AGENTS.md](../../AGENTS.md) maps the app, the brain and the wire protocol. Use
[the recipes](recipes.md) to make a change and [the testing guide](testing.md)
to choose the checks that cover it.

Use `scripts/dev check` for local checks and `scripts/dev e2e smoke` for a short
run through the app. Scripted runs use a scratch notebook and the fake brain.
They leave the app in the background and print the paths to their logs and
screenshots. Use `scripts/dev show <notebook dir>` to inspect a run.

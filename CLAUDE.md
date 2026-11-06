@AGENTS.md

## Claude Code

- `scripts/dev shot ...` prints a PNG path: Read it to see the window. Look at light and
  dark before calling a visual change done.
- The full scenario suite takes minutes: run `scripts/dev e2e` in the background and
  carry on; run single scenarios (`scripts/dev e2e t02`) in the foreground.
- `src/note_page.rs` (3.7k lines) and `src/editor.rs` (4.1k) are sectioned with
  `// -----` banners: `rg -n "// -----" src/note_page.rs` is their table of contents. Read
  the section you need, not the file.
- Before you say a change works in the app, show it: a scenario with `expect:` lines that
  passes, or a dump or screenshot you looked at.

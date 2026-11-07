# Inkling, for agents

A personal macOS notebook: you write, and a model reads along and stitches questions
into the margin. Two processes: the app (Rust + GPUI, `src/`) and the brain (Bun +
TypeScript, `brain/`), which reaches Claude through the Agent SDK on the owner's Claude
Code login. They speak JSON lines. This file is the map; `docs/agents/` has the
reasoning (`README.md`), recipes (`recipes.md`) and the test layers (`testing.md`).

## Rules

1. **Never touch the owner's notebook**, `~/Library/Application Support/Inkling/`. Every
   run of the app gets a scratch `INKLING_DATA_DIR`; `scripts/dev` makes one for you, and
   the app refuses a script without one.
2. **Never call the real model unless the owner asked.** It spends their subscription.
   Runs use the fake brain (`brain/test/e2e/fake-brain.ts`); `--real`, `REAL=1` and
   `brain/test/live.ts` are the only ways to the model.
3. **Never take focus.** The app opens in the background (`INKLING_BACKGROUND=1`) with
   the window titled `Inkling Agents`. Do not drive it with clicks or keystrokes; use a
   script.
4. Use Inkling’s labels in user-facing text and prompts. Say Find a loose thread,
   Sketch it, Reads along, Waits to be asked, Hide threads, and mind.
   `scripts/dev lint` checks the wording.
5. **Other agents work in this repo at the same time.** Work in your own worktree under
   `.worktrees/`, commit early (an uncommitted worktree has been lost before), keep
   diffs in `src/note_page.rs` and `src/app.rs` small, and never reformat a file:
   rustfmt is not installed and the code is not rustfmt-shaped.
6. No `rm -rf` on a computed path. Runs leave their folders under `$TMPDIR/inkling-agent`.

## Commands

All checks go through one door. Every command is non-interactive, exits non-zero on
failure, and those that report take `--json`.

| To | Run | Takes |
|---|---|---|
| Check everything before a commit | `scripts/dev check` | ~15 s |
| ... plus the smoke scenarios in the app | `scripts/dev check --e2e` | ~1 min |
| Run scenarios (all, by name, prefix or tag) | `scripts/dev e2e`, `scripts/dev e2e u01 t02`, `scripts/dev e2e smoke` | all: ~8 min |
| List scenarios, their fixtures and expects | `scripts/dev e2e --list` | |
| Page state as JSON after some steps (it waits for reads and replies to finish) | `scripts/dev state --fixture running-plan --step "jump: next"` | ~5 s |
| Screenshot of that state | `scripts/dev shot --fixture running-plan --step "activate: 1"` → prints a PNG path; read it | ~6 s |
| What a run left (notes, threads, dumps, brain traffic, log) | `scripts/dev show <notebook dir>` | |
| Banned words, map drift, scenario typos | `scripts/dev lint` | 1 s |
| A new worktree, ready to build and test | `scripts/dev worktree NAME` (clones `target/` and `brain/node_modules`) | ~5 s |
| What is missing or running | `scripts/dev doctor` | |
| One Rust test / one brain test file | `cargo test <name>` / `cd brain && bun test test/<file>` | |
| Build the bundle (for the owner, not for checks) | `./scripts/bundle.sh` | |

`scripts/dev` builds the debug binary first (a no-op build is under a second), so a run
never tests a stale binary. `brain/test/e2e/run.sh` is the older runner; it does not
build, check expectations or take screenshots.

## Verify a change

| You changed | Run at least |
|---|---|
| `brain/src/*` | `cd brain && bun test test/` and `bun run check`; the scenarios that use the task |
| Editor logic (`State` in `src/editor.rs`) | `cargo test editor`, then `scripts/dev e2e` for typing scenarios (r08, r09, r15, u03) |
| The reading loop, threads, margin (`src/note_page.rs`) | `cargo test`, `scripts/dev e2e` (all) |
| Anything drawn | `scripts/dev shot ...` and look at the PNG; light and dark (`--appearance dark`) |
| Store fields (`src/store.rs`) | `cargo test store`; open an old fixture with `scripts/dev state --fixture beta` |
| Words on screen or in prompts | `scripts/dev lint` |
| A user-facing capability, or the welcome tour (`src/welcome.rs`) | `design/capabilities.md` stays true; `scripts/dev e2e u02 u03` (they start on the tour) |

A new behaviour gets a scenario with `expect:` lines (`docs/agents/testing.md`), and
unit tests where the logic is pure.

## Map

```
app (Rust, GPUI)                                brain (Bun, brain/src)
  main.rs → app.rs (window, overlays)             main.ts    requests → tasks → events
    note_page.rs (page, reading loop, threads)    prompts.ts every prompt and JSON schema
      editor.rs (text, anchors, layout)           claude.ts  Agent SDK calls, errors, limits
        code.rs (lexer)  sewn.rs (your blocks)
      margin.rs  stitch.rs  drawing.rs            output.ts  snap quotes, clean SVG
                                                  working.ts a correction's check, re-worked
                                                  web.ts     sources only when real, links
    brain.rs  ⇄ JSON lines on stdin/stdout ⇄      protocol.ts the wire types
  store.rs: notes/<id>.json, settings.json, minds.json, folders.json in INKLING_DATA_DIR
```

| File | Owns | Look here for |
|---|---|---|
| `src/main.rs` | logging, the notebook guard, the one window | `INKLING_LOG_JSON`, window options |
| `src/app/folders.rs` | folder pages (on a note's sheet, `note_page::sheet_page`), shared note drag, saving moves between folders | `open_folder`, `move_note`, `folder_view` |
| `src/app/menus.rs` | right-click menus on notes and folders (macOS's own; a test window keeps the items for `menu: Context > …`), deleting a folder with Undo, renaming one in place | `note_menu`, `folder_menu`, `delete_folder`, `start_rename` |
| `src/app.rs` | the window: no title bar (traffic lights, sidebar toggle and Chat sit on the cloth above the sheet and fade while you type), the sidebar with the wordmark, ⌘K palette and export overlays, opening Settings, hold-⌥ dictation, the script runner (`run_script`) and window-level steps | actions and menus (`init`), `chrome_layer`, `INKLING_OPEN`, `INKLING_JIGGLE` |
| `src/shortcuts.rs` | every shortcut: the table (app actions here, the editor's from `editor::bindings()`), your keys from `settings.json` `shortcuts`, clashes, the keymap rebuild and menu refresh when a key changes | `table`, `apply`, `clashes`, `take` (what recording accepts) |
| `src/note_page.rs` | one note: header row, editor, the margin's tags (folded to a tab until pointed at or opened; draggable, `Thread.tag_offset`), threads, selection toolbar, Ask / Ask the room / Sketch it, chat, the reading loop, every brain request the page makes, page-level script steps, `debug_state` (the `dump:` JSON) | `judge` (when a paragraph is read), `on_pause`, `read_line`, `send_task` (timeouts), `report` (error words), `recheck` (fix the words) |
| `src/note_page/lifecycle.rs` | request ownership, a page visit's generation, queued rechecks and serialized profile learning | `Lifecycle`, `resume_rechecks`, `next_learning`; `docs/agents/robustness.md` explains the invariants and remaining work |
| `src/note_page/ends.rs` | threads between notes on the page: the tab's second loop, the other end inside the tag (Go there, `· deleted`), following (Go there, ⌥⌘→ `FollowThread`, `follow:`), a far end's side on its first show, replies and second looks by the thread's mind with the other end, Copy's far line, the dump's `ends` | `far_end_block`, `follow`, `arrive`, `between_reply` |
| `src/note_page/read_through.rs` | Read it through: one pass down the note's unread paragraphs by the reading loop's own rule (`judge`), the pill in the window's top band, the mode menu's last item | `step_through`, `next_for`, `Pass` |
| `src/note_page/own.rs` | threads of your own: Ask yourself, the box with nobody seated, lines you add, calling a mind in (`ask` with `history`), your tag's lines and Ask button | `hang_own`, `add_to_own`, `call_into_own`, `called_mind` |
| `src/note_page/thread_pane.rs` | bounded thread conversations, Markdown messages, follow-the-answer scrolling and interruption when a new message steers an answer | `Conversation`, `markdown`, `interrupt_thread` |
| `src/note_page/write_in.rs` | Write it in: an open tag whose thread your reply tied off offers the reply, word for word, as a paragraph under the thread's (`NoteEditor::insert_paragraph_after`), which becomes its answer; the toast's Undo | `reply_to_write_in`, `write_in_offer`, `write_in`, `undo_write_in` |
| `src/editor.rs` | the document as one string with `\n` between blocks; block kinds, marks, undo, IME, layout, painting; `Anchor`s (thread ranges) that follow edits; gutter marks; checklists, callouts and code blocks (typing them, Return, the box's click, Copy) | `State` (pure, tested headless), `map_anchor`, `NoteEditor` API, `shortcut_kind`, `layout_doc` |
| `src/code.rs` | code blocks' languages, a small lexer per language (a comment or string carries across lines), each token's thread, the contrast test on the code ground | `lex_block`, `color` |
| `src/sewn.rs` | your blocks, sewn: the checklist box and its cross-stitch (and its timing), a callout's patch, a quote's seam, the block geometry the editor lays out and hit-tests | `patch`, `check_box`, `cross_at`, `lay_along` |
| `src/answers.rs`, `src/note_page/answers.rs` | answers in the note: a tied-off thread's `answer` (an editor anchor `<id>#answer` that follows edits), its knot after your words, the tag's "Answered in your note", `tied_by`; the rules the app checks itself before a paragraph may tie a thread off | `answerable`, `copies_mind`, `answer_knots`, `before_status`, `sync_answers` |
| `src/margin.rs` | where tags go without overlapping (`place`), and the `Measured` wrapper that reports their heights | |
| `src/working.rs`, `src/note_page/show_working.rs` | Show its working: a correction's check (a sum from your numbers, or your words) off the wire, in words for Copy, exports and the minds, and drawn under its correction in the tag with Disagree | `from_wire`, `parts`, `place`; `working_block`, `dispute_message` |
| `src/stitch.rs` | the rope (Verlet), running stitch, knots, needle, border | `Rope` |
| `src/dock.rs` | the Dock icon follows the system's light or dark appearance while the app is open, when the bundle has only the `.icns` (no Xcode at bundle time, or the bare binary); it stands down when `Info.plist` names a compiled icon (`CFBundleIconName`) | `follow` |
| `src/drawing.rs` | parses the model's SVG into steps and plays them like ink | `parse`, `Playback` |
| `src/between.rs` | threads between notes, the data side: an in-memory index of every searchable paragraph and the gate that picks at most three passages for `relate`; a thread's far end (`Note.elsewhere`) shown in the other note as a visiting end (`add_visitors`) and sent home on save (`save_visitors_home`); the wire shapes. The page's half is `src/note_page/between.rs` (`look_elsewhere`, after a read). | `Index::passages`, `searchable`, `send_home` |
| `src/brain.rs` | spawns and restarts the sidecar, ids, callbacks (`Pending`), parses events | `Brain::run`, `Launch` (`INKLING_BRAIN_ENTRY`, `INKLING_BUN`) |
| `src/store.rs` | the data model and its JSON files; tolerant loading (unknown fields ignored, bad entries dropped, an unknown block kind reads as a paragraph); notes as Markdown | `Note`, `Thread`, `Elsewhere` (a thread's far end in another note), `Settings`, `BlockKind` (`Check`, `Callout`, `Code(Lang)`), `markdown_blocks` |
| `src/script.rs` | the `INKLING_SCRIPT` language (its doc comment lists every step) | `parse` |
| `src/export.rs` | Markdown, HTML, Word (via `textutil`) with threads under paragraphs; checklists, callouts and code in each | `to_markdown`, `to_html`, `to_word_html` |
| `src/import.rs` | File > Import from Obsidian…: a vault's Markdown as notes and folders, as Obsidian shows it; every imported paragraph goes in `Note.left_alone`; the vault is only read | `convert` (one note), `read_vault`, `add_to_store` |
| `src/welcome.rs` | the first-run note "Start here": a tour with real threads, whose own paragraphs are never read (`Note.left_alone`); the writer's first sentence is probed | `first_run_note` |
| `src/theme.rs`, `src/ui.rs`, `src/palette.rs` | colours and fonts, small widgets, ⌘K | |
| `src/loose_ends.rs` | Loose ends (⇧⌘K): every open thread in the notebook, oldest first, in ⌘K's card; rows show your words only; built from the notes in memory, no index. Going to a row: `app/goto.rs` (the window), `note_page/goto.rs` (the page) | `collect`, `narrow`, `start_at`, `Card`; the dump's `loose_ends` |
| `src/perf.rs` | frame and layout counters for performance work, on with `INKLING_FRAME_LOG=1`; off, nothing runs | `frame`, `layout` |
| `src/toast.rs` | every toast, in one stack at the bottom centre: a note deleted, a thread dismissed (Undo, ⌥⌘Z), a problem (ochre, closable), anything done | `show`, `Kind`, `stack`; the dump's `toasts` |
| `src/stand.rs` | Let it stand: what every mind is told about the points you let stand (`ctx.letStand`), the Let stand button's words and toasts | `points` (which points, in what order, how short), `dismiss` (the reason from the tag's box) |
| `src/back.rs` | Turn it over: the back of the note (⇧⌘B), every thread it has had, oldest first, with how each ended (`Thread::ending`), the words before and after, what was said, your reasons (added or changed there), Reopen and Show on the page; ⇧⌘C there copies the back as Markdown | `entries` (the pure model), `entry` (one thread, for any view), `view`, `to_markdown`, `turn` (the sheet's timing) |
| `src/settings.rs`, `src/settings/parts.rs` | the Settings window (its own window, ⌘,): sidebar of panes, the glass, grouped rows, the shortcut recorder, the model list from the brain's `status` | `open_window`, `Glass`, `INKLING_SETTINGS_*` |
| `brain/test/` | `unit.test.ts`, `protocol.test.ts` (real brain, fake SDK), `parity.test.ts` (task lists agree), `working.test.ts` (a correction's check), `web.test.ts` (sources from the web), `live.ts` (real model, by hand) | |
| `brain/test/e2e/` | `fake-brain.ts` (directives like `[[silent]]` in typed text pick its answer), `scenarios/*.txt`, `fixtures/` | `fake-brain.ts` header |
| `scripts/` | `dev` (the command door), `lib/` (its parts), `bundle.sh` | |
| `assets/` | the app icon: `Inkling.icon` (the Initial in layers, for light, dark, clear and tinted; `icon-layers.py` remakes the layers from `assets/icon.svg` and `assets/icon-dark.svg`, `scripts/bundle.sh` compiles it when Xcode is installed), `Inkling.icns` (the fallback), the Dock's images, fonts | `Inkling.icon/icon.json` |
| `design/` | the product manifesto and capability list. Keep the list in sync with the welcome guide | |

## How a question gets to the margin

1. You type: `editor.rs` emits `EditorEvent::Edited { line }`; `note_page.rs`
   `on_editor_event` remembers the line (`note_edit`) and schedules a pause (2.2 s, or
   4.6 s on the slow pace).
2. The pause (`on_pause`) looks at the caret's line, the edited line and lines kept
   from before, and `judge` decides each: Read (finished: ends a sentence or the caret
   left, at least four words, nothing else reading), Answer (it calls the mind by name),
   Later, or Skip. A paragraph's hash goes into `note.read_hashes` only after its read
   succeeds, so an interrupted read is still unread next time.
3. `read_line` sends `{"task":"read"}` (or `probe` for a new notebook's first
   sentence) through `send_task` → `Brain::run` → one JSON line to the brain. After a
   read, `look_elsewhere` (`src/note_page/between.rs`) asks `relate` about passages from
   other notes, only when the local index finds some (threads between notes).
4. The brain (`main.ts` `run`) builds the prompt (`prompts.ts`), calls the SDK
   (`claude.ts runStructured`), snaps the quote onto the paragraph (`output.ts`), and
   answers with exactly one `done` or `error`.
5. The page anchors the quote in the paragraph (`add_thought` / `locate_thought`), opens
   a `Thread`, tells the editor (`Anchor`), and the margin places its tag (`margin.rs`),
   tied to the words by a thread (`stitch.rs`). A question that lands while you type
   waits, folded, until a 2 s pause.

## Invariants

- Thread `start`/`end`, mark spans and anchor ranges are **byte** offsets into the
  note's blocks joined by `\n`, title included; they must sit on char boundaries.
- A paragraph is read once: its trimmed text's hash goes into `note.read_hashes`
  (or was put in `note.left_alone` because the app wrote it) and is skipped from then on.
- A quote from the brain is always a substring of the text it was asked about
  (`snapQuote`); the app anchors by searching for it verbatim.
- Every `run` request ends in exactly one `done` or `error`, and never `done` after a
  cancel; the Rust `Pending` delivers nothing after the end. Tests hold both sides to it.
- A page request and its timer have one owner. Completion or cancellation drops the
  timer; switching notes changes the visit generation, invalidating deferred work.
  Learning survives note switches, runs serially, and checks the profile revision and
  consent again before saving. A recheck may resolve only the exact words it read.
- Failed edit/settings writes retain the newest bytes for retry. Imports count only
  notes actually saved. Deletion removes the disk file before removing the note from
  memory and clears any queued older save, so retry cannot resurrect it.
- One read in flight per page (`reading`); `send_task` allows 120 s for light requests
  (including reads and replies), or 600 s when its `heavy` flag is set.
- The page never calls back into an entity it is updating: brain callbacks are deferred
  (`send_task`, `later`). GPUI panics on a nested update of the same entity.
- Store files load tolerantly: missing fields default, unknown ones are ignored, one bad
  entry costs that entry. New fields need `#[serde(default)]`.
- The brain's working folder is `<notebook>/brain-cwd`. It loads Claude Code user instructions and can read local files and existing project memories. Notes inherit `Folder.local_directories` unless `Note.local_directories` overrides them.
- Task names agree across `brain/src/protocol.ts`, the app's requests and the fake
  brain; `brain/test/parity.test.ts` fails when they drift.

## GPUI and platform quirks

- A window that is covered or in the background stops drawing. Anything computed while
  painting is then stale: editor layout, the margin slots in a dump, which side a new
  thread goes (`pick_side`), scroll-to-words. `INKLING_JIGGLE=<ms>` nudges the window so
  it keeps drawing; `scripts/dev` turns it on (the height then wobbles by 1 px; the width
  stays, so the note is not laid out again on every nudge).
- Rust 2024: `impl IntoElement` in a return type captures every lifetime in scope, so
  view helpers return `AnyElement` (`.into_any_element()`).
- `gpui_kit::*` brings in GPUI's `test` macro, which shadows std's: write
  `#[std::prelude::v1::test]`.
- Only the Command Line Tools are installed (no Xcode), hence `runtime_shaders` in
  `Cargo.toml`: shaders compile at launch. No `xcrun metal`, no Instruments.
- `Hsla::opacity(f)` multiplies the alpha it already has; it does not set it.
- Bun is found at `~/.bun/bin`, `/opt/homebrew/bin`, `/usr/local/bin` or on `PATH`
  (`INKLING_BUN` overrides). Brain edits apply on the next launch, no rebuild.
- Window screenshots use `screencapture -l <window id>`, which needs Screen Recording
  permission for whatever runs it.

## Environment

| Variable | Effect |
|---|---|
| `INKLING_DATA_DIR` | the notebook folder (required with a script) |
| `INKLING_SCRIPT` | play a script (`src/script.rs`) |
| `INKLING_BRAIN_ENTRY` | another brain entry, e.g. the fake one |
| `INKLING_BACKGROUND=1` | do not take focus |
| `INKLING_WINDOW_TITLE` | the window title (`Inkling Agents` for agent runs) |
| `INKLING_JIGGLE=<ms>` | keep a covered window drawing |
| `INKLING_LOG_JSON=1` | JSON log lines (`ms`, `level`, `target`, `msg`) |
| `INKLING_FRAME_LOG=1` | log frames per second and the editor's layout cost (`perf:` lines), for performance work |
| `INKLING_OPEN=chat,sidebar,...` | panels open at launch |
| `INKLING_SETTINGS_TAB=<pane>` | the pane Settings opens on (with `INKLING_OPEN=settings`) |
| `INKLING_SETTINGS_RECORD=<action>`, `INKLING_SETTINGS_KEYS=settings:<keys>,main:<keys>`, `INKLING_SETTINGS_SEARCH=<text>` | drive the shortcut recorder without clicks: listen for an action's keys, press keys in Settings or the note, fill the search |
| `INKLING_SETTINGS_SCROLL=<px>` | scroll the Settings pane down that far, for a screenshot of a long pane (`scripts/dev shot --window settings --env ...`) |
| `INKLING_SLOWMO=<factor>` | slow every thread animation |
| `INKLING_BUN`, `INKLING_BRAIN_DIR`, `INKLING_CLAUDE_PATH`, `INKLING_USE_API_KEY=1` | where Bun, the brain and `claude` are; keep an API key in the brain's env |
| `FAKE_*` | the fake brain's delays and failures (its header) |
| `INKLING_AGENT_OUT` | where `scripts/dev` keeps its runs |

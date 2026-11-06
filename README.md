# Inkling

A local notebook that asks. You write; Inkling reads along, and when something does not
hold up it stitches a running stitch under the exact words and hangs a thread to a tag in
the margin with its question. Answer it, or fix the words, and the thread ties itself off.

It runs on your Claude subscription: the model calls go through the Claude Agent SDK,
which uses this Mac's Claude Code login. No API key.

The look is "Loose Threads" (linen, Fraunces, stitches, knots). How it was chosen, and the
two directions it was chosen over: `design/directions/DESIGN.md`.

## Run it

You need Rust (stable), Bun, and a logged-in Claude Code (`claude`, then `/login`).

```bash
cd brain && bun install && cd ..
./scripts/bundle.sh          # builds target/Inkling.app (debug); pass `release` for an optimized build
open target/Inkling.app
```

The bundle links to this checkout's `brain/` folder, so sidecar edits apply on the next
launch. Notes are JSON files in `~/Library/Application Support/Inkling/`.

## What's in it

| | |
|---|---|
| Reads along | After you pause, reads the paragraph you just finished; usually quiet, sometimes one question stitched to the exact words. The gutter shows what it made of each paragraph: a needle while it reads, a tacking stitch when it is fine, a numbered knot when it has a question. |
| Threads | Quiet until you look at one. Click a tag or the stitched words to open it; reply, resolve (⌘↵), dismiss (⌘⇧⌫, with Undo for 8 s or ⌥⌘Z), copy, or make it a note. ⌥⌘↓ / ⌥⌘↑ jump between open threads. |
| Fix the words | Edit the words a question is about and Inkling looks again: if the edit answers it, the thread ties itself off with one line saying why; if not, it says what is still missing. |
| While you type | A question that lands mid-sentence waits, folded, until you pause. |
| Selection | Block types, marks, thread colours, and **Find a loose thread** (⌘↵), **Ask Inkling**, **Sketch it**. |
| Ask the room | Ask up to three minds about a selection at once; each answers in its own thread colour. |
| Sketches | Stitched step by step, with captions, ‹ › and replay. |
| Call by name | Start a paragraph with "Inkling," (or the mind's name) to ask directly, in either mode. |
| Chat | ⌘J, about the whole note and its threads. |
| Modes | Per note: Reads along, or Waits to be asked. |
| Minds | Readers with their own voice and thread colour (staff engineer, editor, skeptic, Socratic tutor, investor, or your own). |
| First run | A new notebook asks for one sentence about something you are working on; the first question is about your own words. |
| Tell Inkling | A per-note brief it keeps in mind. |
| About you | A short profile every note starts with; optionally learned from what you say. |
| Notes | Sidebar, folders, ⌘K find or start, ⌘⌫ delete, copy as Markdown (⌘⇧C), export Markdown / Word / HTML with the threads under their paragraphs (⌘⇧E), hide threads (⌘⇧.). |
| Voice | Hold ⌥ to dictate (macOS speech recognition). |
| Settings | ⌘, opens a Settings window: Reading, Appearance, Minds, About you, Model, Shortcuts, Help. Changes show in the note as you make them. |
| Shortcuts | Every key is yours to change, the editor's included: click keys to record new ones, add or remove keys, clashes are flagged on both rows, reset one or all. New keys work at once, in the note and the menu bar. |
| Models | The list comes from Claude Code (the Agent SDK's `supportedModels()`), with each model's effort levels; it is kept for when the brain is not running. Empty means Claude Code's own default. |

## How it's built

```
GPUI app (Rust, src/)                        brain sidecar (Bun + TypeScript, brain/)
  editor, margin, drawings, chat     ⇄ JSONL ⇄   Claude Agent SDK → Claude Code runtime
  notes as JSON on disk                           (your subscription login)
```

- `src/editor.rs`: the block editor (title, paragraphs, headings, lists, quotes, marks,
  IME, undo) and the anchors that keep threads attached to their words while you type.
- `src/note_page.rs`: the page, the reading loop, threads, toolbar, Ask / Sketch it / Ask the room, chat.
- `src/margin.rs`: tag placement without overlaps.
- `src/stitch.rs`: the thread (a small rope simulation), stitches, knots, the needle, the sampler border.
- `src/drawing.rs`: parses the mind's SVG and draws it like ink, one step at a time.
- `src/settings.rs` (and `src/settings/parts.rs`): the Settings window, its glass and controls.
- `src/shortcuts.rs`: every shortcut, its default keys and yours (`shortcuts` in settings.json), the
  keymap rebuild and the menu refresh.
- `brain/src/prompts.ts`: every prompt Inkling sends; `brain/src/claude.ts`: the SDK calls.

## Testing hooks

Agents, and anyone scripting the app: start at `AGENTS.md`. `scripts/dev help` lists the
commands (checks, scenarios with expectations, the page state as JSON, screenshots).

| Variable | Effect |
|---|---|
| `INKLING_DATA_DIR=<dir>` | Use another notebook folder. |
| `INKLING_SCRIPT=<file>` | Play a script against the UI (see `src/script.rs`): type, select, ask, sketch, room, reply, jump, dismiss, undo, chat, open panels. Needs `INKLING_DATA_DIR`, so a script never edits the real notebook. End-to-end scenarios: `scripts/dev e2e <name>`. |
| `INKLING_OPEN=sidebar,chat,...` | Open panels at launch. |
| `INKLING_BACKGROUND=1` | Don't take focus when launched. |
| `INKLING_JIGGLE=<ms>` | Nudge the window so it keeps drawing while covered (for screenshots). |
| `INKLING_SLOWMO=<factor>` | Slow every thread animation down (screenshots, the walkthrough video). |
| `INKLING_LOG_JSON=1` | Log one JSON object per line (`ms`, `level`, `target`, `msg`), for tools. |
| `INKLING_BRAIN_ENTRY=<file.ts>` | Run another brain, e.g. the fake one in `brain/test/e2e/fake-brain.ts` (no model calls). |
| `INKLING_CLAUDE_PATH=<path>` | Use a specific `claude` binary instead of the SDK's bundled one. |
| `INKLING_SETTINGS_TAB=<pane>` | Open Settings on reading, appearance, minds, newmind, about, model, shortcuts or help (with `INKLING_OPEN=settings`). |
| `INKLING_SETTINGS_RECORD=<action>` | Open Settings listening for new keys for that shortcut, e.g. `app::NewNote`. |
| `INKLING_SETTINGS_KEYS="settings:<keys> main:<keys>"` | Press keys in Settings or in the note window, a second apart, through GPUI's own dispatch. |
| `INKLING_SETTINGS_SEARCH=<text>` | Fill the shortcut search. |
| `FAKE_MODELS=none` | The fake brain reports no model list (by default it reports the list the real SDK gave, `brain/test/fixtures/supported-models.json`). |

`cargo test` covers the editor, margin placement, the rope, drawings, export, the store and the page's reading rules; `cd brain && bun test test/` covers the sidecar.

## Notes

- Anthropic's docs do not allow offering claude.ai login inside products you ship to
  others. This is a personal tool on your own machine; keep it that way, or switch the
  sidecar to an API key (`INKLING_USE_API_KEY=1` keeps `ANTHROPIC_API_KEY` in its env).
- Codex (ChatGPT sign-in with the device code flow) is the next provider; it slots in
  beside `brain/src/claude.ts` without app changes.

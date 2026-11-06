# Inkling: what we are rebuilding

Inkling is a local, native rebuild of riddled.ai ("a notebook that thinks with you"),
written in Rust with GPUI. The model runs through the Claude Agent SDK on the
machine's Claude Code login, so it uses the Claude subscription instead of an API key.
The name is different on purpose; the behaviour and look follow Riddled closely.

Everything below was observed on riddled.ai on 6 Nov 2026 (account created for testing,
about 9 of the 30 free responses used). Their backend was flaky that day: chat and
"call by name" requests hung for minutes, read-along returned a 503 once.

## The product in one paragraph

You write. The AI reads along and, when something does not add up, underlines the words
it is about and writes a short question in the margin, in a serif "ink" voice. You can
open any margin note and reply, push back, keep going ("follow the thought"); it can
mark the thread resolved when you get it. Select any text to ask your own question,
have it "riddle" the passage, or have it draw the idea as a step-by-step ink sketch.
Each note can be "Thinks with you" (reads along) or "When you ask" (quiet until called).
Custom "minds" (Riddled calls them Riddlers) change how it reads: a staff engineer, an
editor, a skeptic. The page stays yours: it never rewrites your text.

## Features and how Riddled implements them

| Feature | What it does | Riddled's mechanism (observed) |
|---|---|---|
| Read-along | After you pause, reads the paragraph you just wrote; usually silent, sometimes one margin note anchored to an exact quote | `POST /api/riddled {task:"read", target:<paragraph>, noteText, thoughtId}` → `{said, thought:{kind, quote, message:{text}}}`. ~5-10 s. Not streamed. |
| Reply in margin | Thread under each note; reply box "Write back…"; Resolve / Dismiss; menu: Resolve, Copy, Make a note, Dismiss | `task:"reply"` → `{said, message:{text}, redraw, resolved}`. History is server-side. |
| Riddle this (⌘↵) | On a selection: the most useful question about it | selection toolbar action |
| Ask Riddled | On a selection: your own question; answer goes to the margin | `task:"ask", target, question` → thought with the question shown first |
| Visualize | On a selection: explanation, then a step-by-step drawing | `task:"visualize"` (text) then `task:"draw"` → `{sketch:{type:"drawing", svg}}` |
| Drawings | Hand-drawn SVG, 2-7 steps with captions, ‹ n/N › and replay, "See it larger" modal | SVG with `<g data-step data-caption>`; colours `var(--ink/--graphite/--faint/--rule)`, fonts `--font-voice` (serif) / `--font-sans`; arrows drawn as paths |
| Call by name | In "When you ask" mode, a paragraph like "Riddled, is 5 minutes too long?" is answered | `task:"answer", target:<paragraph>` |
| Chat (⌘J) | Right panel about the whole note and its threads; suggestions: "What am I still misunderstanding?", "What have we figured out so far?", "What should I work on next?"; Copy, Clear | `task:"chat", message, near:<paragraph at cursor>` |
| Tell Riddled | Per-note brief: "What's this note for, or how should Riddled be here?" shown in serif under the header row | stored on the note (`brief_goal`) |
| Modes | "Thinks with you" (reads along) / "When you ask" (quiet until name, selection or chat); per note, default in settings | `read_along` flag on the note |
| Riddlers | Picker above the title; New Riddler form: templates (Staff engineer, Editor, Skeptic, Socratic tutor, Investor, Blank), name, description, how it thinks, voice (Plain, Warm, Blunt, Riddle), how often (Rarely, When it matters, Often); versioned | `riddler_id` on the note |
| About you | ≤700-char paragraph every note starts out knowing; optional "learns about you" | settings |
| Customize | New notes start in (mode); Ink from the pointer; Reading pace (Regular / Slow ≈2× wait); Where questions appear (Both margins / Right margin / Marks only); Appearance (System/Light/Dark); Text size (Small/Default/Large); Lettering (Riddled's own serif / Like your notes); Colour (Ink, Blue, Green, Red, Purple, Brown, Orange) | settings |
| Notes | Sidebar overlay: search ⌘K, New note, notes, folders, New folder; Copy note ⌘⇧C, Export ⌘⇧E (PDF/Word/Markdown, include riddles), Just my words ⌘⇧. (hides the margin) | |
| ⌘K | Find or start a note; full text; "Folder / Name" files a new note; ⌘⌫ deletes | |
| Editor | Title + paragraphs, H1-H4, quote, bullet and numbered lists, bold/italic/underline/strike/code, 9 text colours; floating toolbar on selection | TipTap/ProseMirror doc |
| Voice | "Hold ⌥ to speak" | not tested |
| Share | Read-only link, optionally with threads | not needed locally |

## Look

- Paper `#f8f7f4`, ink `#1d1c1a`, graphite `#5c5953`, faint `#8e8a83`, rule `#dedbd4`,
  wash `#1d1c1a0e`, bulb `#d99a1e`. Dark: paper `#1a1a19`, ink `#e9e7e2`, graphite `#a9a69e`,
  faint `#7a776f`, rule `#33322f`. Ink colours: blue `#33689c`, green `#47763d`,
  red `#b23b33`, purple `#774f9c`, brown `#8a5a3b`, orange `#b5621f`.
- Fonts: Instrument Sans for your words and UI; Newsreader for the AI's voice and drawings.
- Note text 16 px (15/18 for small/large). Column ~420 px wide, centred; margin notes
  ~230 px wide on either side, right-aligned on the left, with a small dot and a faint
  tether curve to the underlined quote. The active thread washes its quote; others fade.

## Inkling's architecture

```
GPUI app (Rust)                      brain sidecar (Bun + TypeScript)
  editor, margin, drawings, chat  ⇄    Claude Agent SDK  →  Claude Code runtime
  notes as JSON files on disk     JSONL    (subscription login, no API key)
```

- `brain/` speaks one JSON object per line: `run`, `cancel`, `status` in; `delta`, `done`,
  `error`, `limit`, `status` out. Tasks: read, riddle, ask, answer, visualize, draw, reply, chat.
  Structured tasks use the SDK's `outputFormat` JSON schema; ask/answer/chat stream.
- Measured on this machine (Opus 5.5, low/medium effort): read 5 s, visualize 6 s,
  reply 5 s, ask 5.6 s, draw 29 s. First call after start pays ~10 s of runtime start-up.
- Notes live in `~/Library/Application Support/Inkling/` as JSON (one file per note).
- Codex comes later as a second provider inside `brain/` (device-code login), with no
  change to the app.

<p align="center">
  <img src="assets/icon.svg" width="128" alt="Inkling's icon: a linen page whose top border is a row of darns, the last one still being woven">
</p>

# Inkling

Inkling is a notebook for your Mac that asks you questions. You write, and it reads along. When a line doesn't add up, it pulls a thread from those exact words out into the margin, with a question or a correction hanging on it. It never writes your notes for you. The thinking stays yours; Inkling only makes sure you have done it.

It runs on your own Claude subscription, through the Claude Code login already on your Mac. Your notes stay on your Mac as plain files.

## Watch it first

<video src="design/video/page/film.mp4" poster="design/video/page/poster.jpg" controls width="100%"></video>

The film is three and a half minutes long and shows everything Inkling does, in seven chapters, with real answers from the model. If the player doesn't appear, open [`design/video/page/film.mp4`](design/video/page/film.mp4). Every capability and the moment it appears in the film are listed in [`design/video/coverage.md`](design/video/coverage.md).

## What it's like to use

**You write, and it reads along.** When you pause, Inkling reads the paragraph you just finished. Most of the time it says nothing: a small stitch in the gutter shows it has read the paragraph and found nothing to ask. When something doesn't hold up, it sews a running stitch under the exact words and hangs a thread out to a tag in the margin, with a ? for a question or a × for a correction. A question that arrives while you are mid-sentence waits until you stop.

**Threads stay quiet until you look at them.** A tag stays folded until you rest the pointer on it, then drops open to show the question. Click it to write back, and the mind answers in the same thread. If you fix the words a thread is about, Inkling reads them again; when the problem is gone, the thread ties itself off and its knot joins the border at the top of the page. You can drag a tag anywhere, dismiss it (and take that back), jump between open threads with ⌥⌘↓ and ⌥⌘↑, or hide every thread with ⇧⌘. to see only your words.

**You can ask whenever you want.** Select some words and press ⌘↵, and Inkling picks the one thing in them most worth a question. You can also ask your own question, or have it answer with a sketch that is stitched out step by step. Ask the room to hear from up to three minds at once, or start a sentence with a mind's name to call it directly. ⌘J opens a chat about the whole note.

**The minds have their own voices.** Inkling comes with a staff engineer, an editor, a skeptic, a Socratic tutor and an investor, and you can write your own: what it is for, how it thinks, its voice and how often it speaks. Each mind keeps its own thread colour, so you can see at a glance who asked what. Tell Inkling, above each note's title, says what that note is for, and a note can read along or wait to be asked.

**The window stays out of the way.** There is no toolbar. The few controls fade while you write and come back when you move the pointer. Settings is a proper Mac window, every keyboard shortcut can be changed, and you choose Linen (light) or Lamplight (dark), your thread colour, the lettering and the text size. Hold ⌥ to speak instead of typing.

**Your own blocks are sewn too.** Type `[ ] ` to start a checklist: a ticked item takes a cross-stitch and its line is tacked through. Type `> [!note] ` (or `warning`, or `tip`) for a callout, a patch sewn onto the page in the colour of its kind. Three backticks open a code block with line numbers, a Copy button and syntax in thread colours. Madder is Inkling's own colour: it never appears in your blocks or among the colours for your words, so you can always tell your marks from its questions.

**It is a real notebook.** Notes and folders live in the sidebar, ⌘K finds a note or starts one, ⇧⌘C copies a note as Markdown, and ⇧⌘E exports it as Markdown, HTML or Word, with its threads or without. File > Import from Obsidian brings in a whole vault, folders and all, and Inkling leaves the imported words alone so it reads only what you write afterwards.

The complete list, row by row, is [`design/capabilities.md`](design/capabilities.md).

## What it believes

Inkling holds a few beliefs, written out in full in [`design/manifesto.md`](design/manifesto.md). You find out what you think by writing it down yourself, which is why Inkling never writes your notes for you. The most useful thing a reader can do is ask a good question in the right place, beside the exact words, and leave the answer to you. Most of what you write needs no comment, so Inkling's silence means something. And deciding that your words should stand is an answer too.

## Install and run

You need macOS 13 or later, Rust (stable), [Bun](https://bun.sh), and Claude Code signed in: run `claude`, then `/login`.

```bash
cd brain && bun install && cd ..
./scripts/bundle.sh release
open target/Inkling.app
```

The first time Inkling opens, a note called Start here walks you through everything, with real threads to try, and ends by asking you for one sentence about something you are working on. Help > Welcome Guide brings it back later. Your notes are JSON files in `~/Library/Application Support/Inkling/`.

## How it works

Inkling is two programs that talk over a pipe. The app, written in Rust with [GPUI](https://www.gpui.rs), draws the page, the threads and the margin, and keeps your notes. A small helper called the brain, written in TypeScript and run with Bun, talks to Claude through the Claude Agent SDK, using your Claude Code login. When you pause, the app sends the brain the paragraph and enough of the note for context, and the brain answers with silence, a question, a correction or a sketch.

You pick the model in Settings > Model, from the list Claude Code offers; reading along and answering each have their own effort setting. Nothing leaves your Mac except the text Inkling sends to Claude to read along or to answer you.

## The look

The design is called Loose Threads, after embroidery: running stitches under your words, a thread that hangs with real rope physics, fabric tags, a knot when a thread is tied off, and a sampler border that grows as you work. The logo, Darn · Border, is a page whose top border is a row of darns, one for every question answered, the newest still on the needle. The type is Fraunces for writing and Figtree for the controls.

How the design was chosen, and the two directions it was chosen over, is in [`design/directions/DESIGN.md`](design/directions/DESIGN.md). The motion, the stitched diagrams and code blocks are explored in [`design/explorations/`](design/explorations/), and the logo rounds are in [`design/logo/`](design/logo/).

## Working on Inkling

Start with [`AGENTS.md`](AGENTS.md). It maps the code, lists the rules, and explains the one command that matters: `scripts/dev check` runs every check in about fifteen seconds. `scripts/dev e2e` drives the real app through scripted scenarios against a fake brain, so it makes no model calls, and `scripts/dev shot` takes a screenshot of the app without touching your screen.

| Where | What |
|---|---|
| `src/editor.rs` | The editor: blocks, marks, typing, undo, and the anchors that keep threads on their words |
| `src/note_page.rs` | The page: the reading loop, threads and tags, asking, sketches, chat |
| `src/stitch.rs` | The thread's rope physics, stitches, knots, the needle and the border |
| `src/settings.rs`, `src/shortcuts.rs` | The Settings window and every shortcut |
| `src/import.rs` | Import from Obsidian |
| `brain/src/prompts.ts` | Every prompt Inkling sends to the model |

## A note on the Claude login

Anthropic's terms don't allow offering a claude.ai login inside products you ship to other people. Inkling is a personal tool that runs on your own machine with your own login, and it should stay that way. To share it, switch the brain to an API key: `INKLING_USE_API_KEY=1` keeps `ANTHROPIC_API_KEY` in its environment.

Fraunces and Figtree are used under the SIL Open Font License; the licence texts are in [`assets/fonts/`](assets/fonts/).

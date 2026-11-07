<p align="center">
  <img src="assets/icon.svg" width="128" alt="Inkling's icon: a satin-stitched i dotted with a French knot, on linen inside a hemstitched border">
</p>

# Inkling

Inkling is a Mac notebook that reads what you write and asks you questions about it. When a line doesn't add up, it stitches a thread from those words out to a tag in the margin, with a question or a correction on it. Most of the time it stays quiet.

It runs on your Claude subscription through Claude Code, so there's no API key to set up. Notes are files on your Mac. [How it works](#how-it-works) lists exactly what gets sent to Claude.

[Install Inkling](#install-and-run) · [Changelog](CHANGELOG.md)

## Watch it first

<a href="design/video/page/film.mp4"><img src="design/video/page/preview.gif" alt="A line that doesn't add up gets a thread from its words, and the correction shows its working from your own numbers" width="100%"></a>

A 16-second cut, no sound. The full film is 1:13 with sound: [`film.mp4`](design/video/page/film.mp4). It follows one note from its first sentence to a thread getting tied off, and everything Inkling says in it is real model output. There's a shot list in [`coverage.md`](design/video/coverage.md).

## Using it

When you pause, Inkling reads the paragraph you just finished. Usually it has nothing to say, and a small stitch in the gutter shows it read it. When something's off, it sews a running stitch under the words and hangs a thread out to a tag in the margin: ? for a question, × for a correction. If you're still mid-sentence, it waits until you stop.

Tags stay folded until you point at one. Click it to reply, and the mind answers in the same thread. Edit the words a thread is about and Inkling reads them again. If the problem's gone, the thread ties itself off and its knot goes up into the border at the top of the page. You can drag tags around, jump between open threads with ⌥⌘↓ and ⌥⌘↑, and hide them all with ⇧⌘. when you just want your own text.

A correction always shows its working: a sum using only your numbers, or a quote from elsewhere in the note. If it can't do either, it asks a question instead. To close a thread you can fix the words, reply in the tag (Write it in can then put your reply into the note under the paragraph, for you to rework), or answer it somewhere else in the note, where a knot links your answer back to the question. If you disagree, Let stand keeps your words as they are, with a reason if you want, and no mind brings that point up again. ⇧⌘B turns the note over: the back lists every thread it's had, oldest first, and how each one ended.

You can ask it things too. Select some words and press ⌘↩ and it finds the question worth asking in them, or type your own, or ask for a sketch, which gets stitched out step by step. Ask the room asks up to three minds at once, and starting a sentence with a mind's name calls that mind in. ⌘J opens a chat about the whole note. Ask yourself (⇧⌘↵) hangs your own question on your words, and no mind answers it unless you call one in. In a note that waits to be asked, ⇧⌘R reads the whole draft through once. Anything you ask can use the web, and a source only shows up if the page really says what it's quoted as saying.

It comes with five minds: a staff engineer, an editor, a skeptic, a Socratic tutor and an investor. You can write your own: what it's for, how it thinks, how it talks and how often it speaks up. Each mind sews in its own colour. Above each note's title, Tell Inkling is where you say what the note is for, and you choose whether the note reads along or waits to be asked.

There's no toolbar. The few controls fade out while you type and come back when you move the mouse. Every shortcut can be changed in Settings, where you also pick Linen (light) or Lamplight (dark), your thread colour, the font and the text size. Hold ⌥ to dictate.

Checklists, callouts and code are sewn too. `[ ] ` starts a checklist, and ticking an item cross-stitches the box and runs a line through it. `> [!note] ` (or `warning`, or `tip`) makes a callout, a patch in the colour of its kind. Three backticks start a code block with line numbers, a Copy button and highlighting in thread colours. Madder red is Inkling's colour and nothing of yours ever uses it, so its marks can't be mixed up with yours.

And the normal notebook things: notes and folders in the sidebar, ⌘K to find or start a note, ⇧⌘C to copy one as Markdown, ⇧⌘E to export as Markdown, HTML or Word, with or without threads. File > Import from Obsidian brings in a vault with its folders. Inkling doesn't read the imported text, only what you write after. ⇧⌘K lists every open thread in the notebook, oldest first. If something you write contradicts or depends on another note, the thread quotes both, and ⌥⌘→ jumps to the other one.

The full list is in [`design/capabilities.md`](design/capabilities.md).

## Why it works like this

The reasoning is in [`design/manifesto.md`](design/manifesto.md). Roughly: you work out what you think by writing it yourself, so Inkling's part is reading and asking. A question next to the words is more use than a suggested edit, because the answer stays yours to find. Most paragraphs don't need a comment, so most of the time it says nothing. And if you decide your words are fine as they are, that settles the thread too.

## Install and run

### Download the app

You need an Apple Silicon Mac on macOS 13 or later, and [Claude Code](https://code.claude.com/docs/en/setup) installed and signed in (run `claude`, then `/login`). Inkling uses that install, so it gets Claude Code's updates. You don't need Bun; the app ships its own.

1. Download the latest `Inkling-<version>-macos-arm64.zip` and its `.zip.sha256` from [Releases](https://github.com/lolwierd/inkling/releases/latest). If there isn't one yet, build from source (below).
2. In the download folder, run `shasum -a 256 -c Inkling-<version>-macos-arm64.zip.sha256` with your version in place of `<version>`. It should print `OK`.
3. Unzip it, move `Inkling.app` to Applications, and open it. Release builds are signed ad hoc and not notarized, so macOS may block the first launch. If it does, see [Apple's steps for opening an app from an unknown developer](https://support.apple.com/en-us/102445).

To update, quit Inkling and replace the app. Your notes live in `~/Library/Application Support/Inkling/` and aren't touched.

Inkling looks for `claude` where the native installer and Homebrew put it. If yours is somewhere else, launch Inkling from Terminal with the path:

```sh
INKLING_CLAUDE_PATH="/absolute/path/to/claude" /Applications/Inkling.app/Contents/MacOS/inkling
```

### Build from source

Install [Rust via rustup](https://rustup.rs), [Bun](https://bun.com/docs/installation) and Claude Code. Without Xcode, install Apple's Command Line Tools first (`xcode-select --install`).

```bash
# From the root of your Inkling checkout
(cd brain && bun install --frozen-lockfile)
./scripts/bundle.sh release
open target/Inkling.app
```

This bundle runs the brain straight from the checkout, so don't move the checkout afterwards. For a standalone ZIP, run `scripts/package.sh`; it lands in `target/dist/`. With a recent Xcode, the bundle also gets the icon that follows macOS's light, dark, clear and tinted styles.

On first launch there's a Start here note with threads to try. Help > Welcome Guide brings it back.

## How it works

There are two processes. The app, in Rust with [GPUI](https://www.gpui.rs), draws everything and stores your notes. The brain is a small TypeScript helper run with Bun. It talks to Claude through the Claude Agent SDK on your Claude Code login. The two send JSON lines to each other over stdin and stdout.

About two seconds after you stop typing, the app sends each paragraph you finished to the brain as a read. A read carries the whole note and all its threads, with the finished paragraph marked as the one to look at. It also carries what you told Inkling the note is for, what it has learned about you, the points you let stand, and today's date. The model answers with silence, a question, a correction or a sketch. A paragraph is read once, and again only after you change it. In a note that waits to be asked, a finished paragraph is only sent to see whether it answers a thread that's already open, and only when there is one. After a read, Inkling searches your other notes on your Mac for anything that paragraph might contradict or depend on. Only if it finds something close does it make a second call, with at most three passages of up to 600 characters each.

Everything else happens only when you ask for it: a question on a selection, Ask the room (one call per mind), replies, sketches (two calls, one for the idea and one for the drawing) and chat. You pick the model in Settings > Model from the ones Claude Code offers. Reads run at low effort and answers at medium, and you can change both.

The brain loads your Claude Code user instructions and can read and search files on your Mac and your existing Claude Code memories. You can point a folder at some local directories (Files, on the folder's page), and its notes look there first. A note can have its own: click "linked to …" above its title, or use File > Link Files to This Note…. That's where it looks first, not a limit on what it can read. Its only file tools are Read, Glob and Grep, so it can't change anything.

Web search is only used for things you ask (Find a loose thread, a question on a selection, calling a mind by name, replies, chat). Reads, sketches, drawings, learning about you and comparing notes never search. If a correction cites a page, the brain opens the page itself and checks the quoted words are on it. If they aren't, the citation is dropped and the correction falls back to its sum, your own words, or a question. Links that didn't come from a search get stripped. A call that runs out of turns while searching tries again without the web. Searching uses more of your plan, and you can turn it off in Settings > Model > Look things up on the web.

## The look

The design is called Loose Threads, and it's embroidery: running stitches under your words, threads that hang with real rope physics, fabric tags, a knot when a thread is tied off, and a border along the top that fills with those knots. The icon is a satin-stitched i with a French knot for its dot, in a hemstitched border, like an initial sewn on a linen napkin. The wordmark in the app uses the same i, in your thread colour. Fraunces is for your writing and Figtree for the controls.

## Working on Inkling

Read [`AGENTS.md`](AGENTS.md) first. It has the code map and the rules. `scripts/dev check` runs every check in about 15 seconds. `scripts/dev e2e` plays scripted scenarios in the real app against a fake brain, so it never calls the model, and `scripts/dev shot` takes screenshots without taking over your screen.

CI runs on pull requests and pushes to `master`. Releases are in the [release runbook](docs/releasing.md).

| Where | What |
|---|---|
| `src/editor.rs` | The editor: blocks, marks, typing, undo, and the anchors that keep threads on their words |
| `src/note_page.rs` | The page: the reading loop, threads and tags, asking, sketches, chat |
| `src/stitch.rs` | The thread's rope physics, stitches, knots, the needle and the border |
| `src/settings.rs`, `src/shortcuts.rs` | The Settings window and every shortcut |
| `src/import.rs` | Import from Obsidian |
| `src/note_page/` | Each feature's own part of the page: the header, answers in the note, Write it in, threads of your own, Read it through, the ends of threads between notes, going to a thread |
| `src/stand.rs`, `src/working.rs`, `src/back.rs`, `src/loose_ends.rs`, `src/between.rs` | Let it stand, Show its working, Turn it over, Loose ends, finding the other note |
| `brain/src/prompts.ts` | Every prompt Inkling sends to the model |

## A note on the Claude login

Inkling runs Claude Code locally with the sign-in you already have. To use an API key instead, set `INKLING_USE_API_KEY=1` and `ANTHROPIC_API_KEY`. See [Anthropic's Agent SDK terms](https://code.claude.com/docs/en/agent-sdk/overview#license-and-terms).

## License

Inkling's own code is under [the Unlicense](UNLICENSE). Third-party parts keep their own licenses, including the [bundled fonts](assets/fonts/) and [Bun](assets/licenses/Bun.md).

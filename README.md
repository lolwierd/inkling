<p align="center">
  <img src="assets/icon.svg" width="128" alt="Inkling's icon: a satin-stitched i dotted with a French knot, on linen inside a hemstitched border">
</p>

# Inkling

Inkling is a Mac notebook that asks you questions. You write, and it reads along. When a line doesn't add up, it pulls a thread out of those exact words into the margin and hangs a question or a correction on it. The page is yours, and the margin is Inkling's. Its job is to notice when some of the thinking is missing.

It runs on your own Claude subscription, through the Claude Code login you already have. Your notebook is a folder of plain files on your Mac. What goes to Claude is the text Inkling needs to read or answer, plus any local files and Claude Code memories it looks at along the way.

[Install Inkling](#install-and-run) · [Changelog](CHANGELOG.md)

## Watch it first

<a href="design/video/page/film.mp4"><img src="design/video/page/preview.gif" alt="A line that doesn't add up gets a thread from its words, and the correction shows its working from your own numbers" width="100%"></a>

That's sixteen seconds of the film, without sound: a line that doesn't add up gets a thread from those words, and the correction shows its working from your own numbers. GitHub won't play a video stored in the repo, so the full film (1:13, with sound) is just a file. Click the picture or open [`design/video/page/film.mp4`](design/video/page/film.mp4). It follows one note from its first sentence to a thread tied off, and every word Inkling says in it is real model output. [`design/video/coverage.md`](design/video/coverage.md) lists what it shows, moment by moment.

## What it's like to use

When you pause, Inkling reads the paragraph you just finished. Most of the time it says nothing, and a small stitch in the gutter shows it read the paragraph and had nothing to ask. When something doesn't hold up, it sews a running stitch under the exact words and hangs a thread out to a tag in the margin, marked ? for a question or × for a correction. If a question comes up while you're mid-sentence, it waits until you stop.

Tags stay folded until you rest the pointer on one, and then it drops open to show the question. Click it to write back, and the mind answers in the same thread. Fix the words a thread is about and Inkling reads them again. If the problem's gone, the thread ties itself off and its knot joins the border at the top of the page. You can drag a tag anywhere on the page, jump between open threads with ⌥⌘↓ and ⌥⌘↑, or hide every thread with ⇧⌘. to see only your words.

How a thread ends is up to you. A correction always shows its working, either a sum built only from your own numbers or your own words from elsewhere in the note, and if it has neither it asks a question instead. You can settle a thread by fixing its words, by replying in its tag (Write it in will then put your reply under the paragraph, word for word, for you to rework), or by writing the answer somewhere else in the note, where a knot after your words ties it back to the question. If you disagree, Let stand keeps your words, with a reason if you want to give one, and no mind brings that point up again, however it's worded. ⇧⌘B turns the note over. The back lists every thread the note has had, oldest first, with how each one ended and the words before and after.

You can also ask it things yourself. Select some words and press ⌘↩, and Inkling picks the one thing in them most worth a question. You can type your own question instead, or ask for a sketch, which gets stitched out step by step. Ask the room gets you up to three minds at once, and starting a sentence with a mind's name calls that mind in. ⌘J opens a chat about the whole note. Ask yourself (⇧⌘↵) hangs a question of your own on your words, in your ink, and no mind answers it unless you call one in. In a note that waits to be asked, ⇧⌘R reads the whole draft once, paragraph by paragraph. Anything you ask it can draw on the web, and Inkling only shows a source if the page really says what it quotes.

Inkling comes with five minds: a staff engineer, an editor, a skeptic, a Socratic tutor and an investor. You can write your own, saying what it's for, how it thinks, what it sounds like and how often it speaks. Each mind has its own thread colour, so you can tell at a glance who asked what. Tell Inkling, above each note's title, says what that note is for, and each note either reads along or waits to be asked.

There's no toolbar. The few controls there are fade while you write and come back when you move the pointer. Settings is a proper Mac window, every keyboard shortcut can be changed, and you pick Linen (light) or Lamplight (dark), your thread colour, the lettering and the text size. Hold ⌥ to dictate instead of typing.

Your own blocks are sewn as well. Type `[ ] ` for a checklist; a ticked item gets a cross-stitch and its line is tacked through. Type `> [!note] ` (or `warning`, or `tip`) for a callout, a patch sewn onto the page in the colour of its kind. Three backticks open a code block with line numbers, a Copy button and syntax in thread colours. Madder is Inkling's colour. It never shows up in your blocks or among the colours for your words, so you can always tell your marks from its questions.

It also does the ordinary notebook things. Notes and folders live in the sidebar, ⌘K finds a note or starts one, ⇧⌘C copies a note as Markdown, and ⇧⌘E exports it as Markdown, HTML or Word, with or without its threads. File > Import from Obsidian brings in a whole vault, folders included, and Inkling leaves the imported text alone so it only reads what you write afterwards. ⇧⌘K lists every open thread in the notebook, oldest first, each a keystroke away from its words. And when something you write contradicts or depends on another note, Inkling asks on a thread that quotes both, and ⌥⌘→ takes you to the other end.

Everything it can do is listed in [`design/capabilities.md`](design/capabilities.md).

## What it believes

The beliefs behind Inkling are written out in [`design/manifesto.md`](design/manifesto.md). The short version: you find out what you think by writing it down yourself, so Inkling does the reading and leaves the writing to you. The most useful thing a reader can do is ask a good question in the right place, right beside the words, and leave the answer to you. Most of what you write needs no comment, which is why Inkling's silence is worth something. And deciding that your words should stand counts as an answer too.

## Install and run

### Download the app

The release ZIP is for Apple Silicon Macs on macOS 13 or later. You need a current version of [Claude Code](https://code.claude.com/docs/en/setup): run `claude` in Terminal and sign in with `/login`. Inkling uses that installation and picks up its updates. The app bundles its own runtime for the helper, so you don't need Bun.

1. Download the latest `Inkling-<version>-macos-arm64.zip` and its `.zip.sha256` file from [Releases](https://github.com/lolwierd/inkling/releases/latest). If there isn't one, build from source (below).
2. In the download folder, run `shasum -a 256 -c Inkling-<version>-macos-arm64.zip.sha256`, with the version you downloaded in place of `<version>`. It should print `OK`.
3. Unzip it, move `Inkling.app` to Applications, and open it. Release builds are signed ad hoc and not notarized, so macOS may block the first launch. If it does, follow [Apple's instructions for opening a trusted app](https://support.apple.com/en-us/102445).

To update, quit Inkling and replace the app in Applications with the new one. Your notebook lives separately, in `~/Library/Application Support/Inkling/`.

Inkling looks for Claude Code where the native installer and Homebrew put it, even when you open it from Finder. If yours is somewhere else and Inkling can't find it, quit Inkling and launch it from Terminal with the full path:

```sh
INKLING_CLAUDE_PATH="/absolute/path/to/claude" /Applications/Inkling.app/Contents/MacOS/inkling
```

### Build from source

Install [Rust via rustup](https://rustup.rs), [Bun](https://bun.com/docs/installation) and Claude Code. On a Mac without Xcode, install Apple's Command Line Tools first with `xcode-select --install`.

```bash
# From the root of your Inkling checkout
(cd brain && bun install --frozen-lockfile)
./scripts/bundle.sh release
open target/Inkling.app
```

The local bundle reads the brain from this checkout, so leave the checkout where it is, or run `scripts/package.sh` on Apple Silicon to make a portable ZIP in `target/dist/`. With a recent Xcode, the bundle also gets the icon that follows macOS's light, dark, clear and tinted styles.

The first time Inkling opens, a note called Start here walks you through it with threads to try. Help > Welcome Guide brings it back later.

## How it works

Inkling is two programs talking over a pipe. The app, written in Rust with [GPUI](https://www.gpui.rs), draws the page, the threads and the margin, and keeps your notes. A small helper called the brain, written in TypeScript and run with Bun, talks to Claude through the Claude Agent SDK using your Claude Code login.

When you stop typing for a couple of seconds, the app sends the brain each paragraph you changed. The whole note goes with it, along with its threads, what the note is for and what Inkling knows about you. The brain answers with silence, a question, a correction or a sketch. A paragraph it has read isn't sent again until you change it, and a note that waits to be asked is never read on its own. After a read, Inkling searches your other notes, on your Mac, for a passage the paragraph might contradict or depend on. Only when one is close enough does it make one more call to compare the two.

Everything else happens only when you ask: a question about a selection, the room (one call per mind), a reply in a tag, a sketch (two calls, one for the idea and one for the drawing) and chat. You pick the model in Settings > Model from the list Claude Code offers. Reading along uses low effort and answering uses medium, and you can change either.

Inkling loads your Claude Code user instructions and can read and search local files and existing project memories. Link a folder's notes to local directories with Files on the folder's page. Its notes use those directories until you give a note its own: press "linked to …" above the note's title, or use File > Link Files to This Note…. These links tell Inkling where to look first. They don't restrict what it can read on disk. Whatever files turn out to be relevant become context for Claude, and the file tools it has can read and search but not edit.

For the things you ask it (Find a loose thread, a question about a selection, a mind called by name, a reply in a tag, chat), Inkling can also search the web. Reading along, sketches, drawings, learning about you and comparing your notes never do. A correction that cites a page keeps the citation only if a search in that same call returned the page and the quoted words are actually on it, which the brain checks by opening the page itself. Otherwise the correction falls back to its arithmetic, your own words, or a question. Links that no search returned are taken out of answers. Lookups use more of your Claude plan. Calls have turn limits, and one that runs out while searching tries again without the web. To turn web lookups off, use Settings > Model > Look things up on the web; local files and memories stay available.

## The look

The design is called Loose Threads, and it borrows from embroidery: running stitches under your words, a thread that hangs with real rope physics, fabric tags, a knot when a thread is tied off, and a sampler border that grows as you work. The logo, Initial, is a satin-stitched i dotted with a French knot inside a hemstitched border, the way people used to mark their linen with an initial. The wordmark in the app has the same i, in your thread colour. The type is Fraunces for writing and Figtree for the controls.

## Working on Inkling

Start with [`AGENTS.md`](AGENTS.md). It maps the code, lists the rules, and covers the command you'll run most: `scripts/dev check`, which runs every check in about fifteen seconds. `scripts/dev e2e` drives the real app through scripted scenarios against a fake brain, so it makes no model calls, and `scripts/dev shot` screenshots the app without touching your screen.

GitHub Actions checks pull requests and pushes to `master`. The [release runbook](docs/releasing.md) covers preview builds, versions and tags.

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

Inkling launches Claude Code locally and uses the sign-in you already have. To use an API key instead, set `INKLING_USE_API_KEY=1` and `ANTHROPIC_API_KEY`. See [Anthropic's Agent SDK terms](https://code.claude.com/docs/en/agent-sdk/overview#license-and-terms).

## License

Inkling's own code is released under [the Unlicense](UNLICENSE). Third-party parts keep their own licenses, including the [bundled fonts](assets/fonts/) and [Bun](assets/licenses/Bun.md).

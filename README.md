<p align="center">
  <img src="assets/icon.svg" width="128" alt="Inkling's icon: a satin-stitched i dotted with a French knot, on linen inside a hemstitched border">
</p>

# Inkling

A Mac notebook that reads along as you write. When a line doesn't add up, it stitches a thread from those words to a tag in the margin with a question or a correction on it. Most paragraphs get nothing. It uses your Claude subscription through Claude Code.

<a href="design/video/page/film.mp4"><img src="design/video/page/preview.gif" alt="A line that doesn't add up gets a thread from its words, and the correction shows its working from your own numbers" width="100%"></a>

The full film, 1:13 with sound: [`film.mp4`](design/video/page/film.mp4). Everything Inkling says in it is real model output.

## Install

You need an Apple Silicon Mac on macOS 13 or later, and [Claude Code](https://code.claude.com/docs/en/setup) signed in (`claude`, then `/login`).

1. Download `Inkling-<version>-macos-arm64.zip` and its `.sha256` from [Releases](https://github.com/lolwierd/inkling/releases/latest).
2. `shasum -a 256 -c Inkling-<version>-macos-arm64.zip.sha256` should print `OK`.
3. Unzip, move `Inkling.app` to Applications and open it. Builds aren't notarized, so macOS may block the first launch. [Here's how to open it anyway](https://support.apple.com/en-us/102445).

Notes are kept in `~/Library/Application Support/Inkling/`. To update, replace the app.

If Inkling can't find `claude`, give it the path:

```sh
INKLING_CLAUDE_PATH="/path/to/claude" /Applications/Inkling.app/Contents/MacOS/inkling
```

To use an API key instead of your login, set `INKLING_USE_API_KEY=1` and `ANTHROPIC_API_KEY` (see the [Agent SDK terms](https://code.claude.com/docs/en/agent-sdk/overview#license-and-terms)).

## Using it

The Start here note you get on first launch walks through everything with real threads. It comes with five minds (a staff engineer, an editor, a skeptic, a Socratic tutor and an investor), and you can write your own. Each note either reads along or waits to be asked.

| Keys | Does |
|---|---|
| ⌘↩ on a selection | Ask about those words |
| ⌘J | Chat about the note |
| ⇧⌘↩ on a selection | Ask yourself: hang your own question on those words |
| ⌥⌘↓, ⌥⌘↑ | Next or previous open thread |
| ⇧⌘. | Hide threads |
| ⇧⌘B | Turn it over: every thread the note has had |
| ⇧⌘K | Loose ends: every open thread in the notebook |
| ⇧⌘R | Read it through, in a note that waits to be asked |
| ⌘K | Find or start a note |

The full list is in [`design/capabilities.md`](design/capabilities.md).

## How it works

The app (Rust, [GPUI](https://www.gpui.rs)) draws the page and stores your notes. The brain (TypeScript, Bun) calls Claude through the Agent SDK. They talk in JSON lines over stdin and stdout.

About two seconds after you stop typing, the app sends each paragraph you finished to the brain. Each read includes the whole note and its threads, what you told Inkling the note is for, what it has learned about you, and the points you let stand. A paragraph isn't read again until you change it. A note that waits to be asked only checks whether a paragraph answers an open thread. After a read, a local search of your other notes can add one more call with up to three related passages.

The brain can read and search local files and your Claude Code memories, with Read, Glob and Grep only. Web search is only for things you ask, and you can turn it off in Settings > Model. A cited page is kept only if the brain finds the quoted words on it.

## Build from source

You need Rust ([rustup](https://rustup.rs)), [Bun](https://bun.com/docs/installation) and Claude Code. Without Xcode, run `xcode-select --install` first.

```bash
(cd brain && bun install --frozen-lockfile)
./scripts/bundle.sh release
open target/Inkling.app
```

That bundle runs the brain from your checkout, so leave the checkout where it is. `scripts/package.sh` makes a standalone ZIP in `target/dist/`.

## Working on it

[`AGENTS.md`](AGENTS.md) has the code map and the rules. `scripts/dev check` runs every check in about 15 seconds, and `scripts/dev e2e` drives the real app against a fake brain, so it never calls the model. Releases are in [`docs/releasing.md`](docs/releasing.md), and the thinking behind the design is in [`design/manifesto.md`](design/manifesto.md).

## License

[The Unlicense](UNLICENSE). The [bundled fonts](assets/fonts/) and [Bun](assets/licenses/Bun.md) keep their own licenses.

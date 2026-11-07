# Changelog

## Unreleased

- Rearranged the header above a note's title into two lines. The first says what the note has been given: its folder, the directories it is linked to, and Tell Inkling. The second reads as a sentence, such as "Inkling reads along", with the open threads on the right, marked with the same knot as the gutter. A note linked to nothing no longer shows Files; File > Link Files to This Note… links one.

## 0.1.3

- Added local file and Claude Code memory context, with folder mappings inherited by notes and optional note overrides.
- Added folder pages, drag-and-drop note moves, and clearer folder and local-directory controls.
- Bounded thread cards with internal scrolling while keeping the reply box and actions visible.
- Added immediate steering: sending during an answer redirects it with the thread history and any interrupted words.
- Rendered Markdown in thread messages, including links, lists, emphasis, code and tables.
- Fixed resolving and reopening threads so their margin marks stay usable and reflect the thread's state.

## 0.1.2

- Packaged Inkling’s helper as a standalone executable so the downloaded app runs without a separate Bun install. It still uses the Claude Code installed on your Mac.

## 0.1.1

- Switched to the Claude Code already installed on your Mac, shrinking the download and picking up Claude Code updates without an Inkling release.

## 0.1.0

- Added a Mac notebook that reads along and stitches questions and corrections to the words that prompted them. Corrections can show their arithmetic or the passage they rely on.
- Added configurable minds, note chat, sketches, and optional web lookups with source links.
- Added threads between notes, questions you can leave for yourself, and ways to settle threads by replying, editing, answering in the note, or letting your words stand.
- Added a welcome guide, searchable notes and folders, dictation, customizable shortcuts, Obsidian import, and Markdown, HTML and Word export.
- Added Apple Silicon install archives with SHA-256 checksums and a tagged GitHub release workflow.

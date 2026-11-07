# Changelog

## Unreleased

## 0.1.4

- Split the header above a note's title into two lines. The top one shows where the note is filed, which directories it's linked to, and Tell Inkling. The second reads as a sentence ("Inkling reads along"), with the open-thread count on the right, marked with the same knot as the gutter. A note with nothing linked no longer shows a bare Files button; use File > Link Files to This Note… instead.
- Added right-click menus for notes (Open, Move To, Delete Note) and folders (New Note, Open, Rename…, Delete Folder), and File > Delete This Note. Every delete can be undone, and deleting a folder moves its notes to Unfiled. The hover × that deleted a folder in one click, with no way back, is gone, and right-clicking a row no longer opens it.
- Moved folder pages onto the same sheet as a note, so the title and column stay put when you go between them. Unfiled now folds like the other folders.
- Replaced the grey scroll bar in long threads with a short length of the thread's colour beside the card's edge.
- Kept failed saves of notes, settings, folders and minds, and retried them with a warning, instead of losing the change. A note that can't be deleted now stays and says so, and a note that fails to import shows up as not saved instead of looking imported.
- Fixed late replies, rechecks and answers. If you've changed the words, switched notes or quit by the time one comes back, it no longer ties off the newer words, lands on the wrong note, or marks a paragraph as read when it wasn't.
- Fixed Read it through repeating a read, call by name or recheck that was already running, and skipping a paragraph that was only being checked for an answer.
- Fixed replies in Ask the room going out with another mind's context, and threads of your own disappearing if you left them before writing the first line.
- Fixed learning about you: an update started before you turned learning off no longer lands afterwards, and an older update can't overwrite a newer one.
- Capped how much memory undo history and fetched web pages can use. When the helper is overloaded, the request now fails with an error instead of freezing the window.

## 0.1.3

- Let Inkling read local files and your Claude Code memories for context. Directories linked to a folder apply to its notes unless a note has its own.
- Added folder pages and drag-and-drop for moving notes between folders, and made the folder and local-directory controls clearer.
- Made long threads scroll inside their card, so the reply box and buttons stay in view.
- Added steering: send a message while an answer is still coming and it changes course, keeping the thread so far and whatever it had already written.
- Rendered Markdown in thread messages, including links, lists, emphasis, code and tables.
- Fixed margin marks that stopped responding, or showed the wrong state, after a thread was resolved or reopened.

## 0.1.2

- Packaged Inkling's helper as a standalone executable so the downloaded app runs without a separate Bun install. It still uses the Claude Code installed on your Mac.

## 0.1.1

- Switched to the Claude Code already installed on your Mac, shrinking the download and picking up Claude Code updates without an Inkling release.

## 0.1.0

- Added a Mac notebook that reads along and stitches questions and corrections to the words that prompted them. Corrections can show their arithmetic or the passage they rely on.
- Added configurable minds, note chat, sketches, and optional web lookups with source links.
- Added threads between notes, questions you can leave for yourself, and ways to settle threads by replying, editing, answering in the note, or letting your words stand.
- Added a welcome guide, searchable notes and folders, dictation, customizable shortcuts, Obsidian import, and Markdown, HTML and Word export.
- Added Apple Silicon install archives with SHA-256 checksums and a tagged GitHub release workflow.

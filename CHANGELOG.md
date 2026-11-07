# Changelog

## Unreleased

## 0.1.5

- Added a first look at each paragraph while reading along. Claude's small model (Haiku) reads the paragraph and the text around it and decides whether it's worth a full read, so most paragraphs no longer cost a call to the model you picked. It's told what the mind and the note are for, it leans toward reading when unsure, and if it fails the full read happens as before.
- Made Read it through read the draft in one call, up to 40 paragraphs at a time, instead of one call per paragraph. The needle still works down the page and threads still arrive as it goes.
- Added corrections backed by a file of yours. When a mind opened a file that settles the point, the tag shows the quoted words under In your files, with the file's name and line; clicking it shows the file in the Finder. Inkling opens the file itself to make sure the words are there.
- Added a line under each answer in an open tag that says what the mind looked at, such as "Looked at README.md, example.org".
- Stopped reading along from failing on every pause once your Claude plan's limit is reached. Paragraphs wait, unread, until it resets and are read then. Asking still works.
- Gave each task only the tools it uses. Comparing notes, settling, learning and sketches no longer get file tools, and a call that runs out of turns while looking things up now answers without them instead of showing an error.
- Passed on every point you let stand whose words are still in the note, not only the newest 12, so an older point isn't raised again.
- Reworked the prompts. Claude Code memories are described instead of read on every call, a mind set to speak often is no longer told to stay quiet, a Socratic mind asked directly leads you to the answer, answers don't narrate what they're looking up, and web answers link in the sentence instead of ending with a Sources list.
- Kept Read it through's prompt cache when a thread lands, and added one line per request to brain.log with its time, tokens, cost, turns and tools.

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

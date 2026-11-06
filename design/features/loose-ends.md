# Loose ends

The manifesto's section 2, item 7: "A single list, in ⌘K or the sidebar, shows every open
thread in the notebook, oldest first, each a click away from its words." Approved by the
owner; designed and built on 7 Nov 2026 on the branch `feat-loose-ends`.

## In short

⇧⌘K turns ⌘K's card to Loose ends: every open thread in the notebook, one row each, oldest
first. A row shows the words of yours that the thread hangs from, the thread's mark in its
mind's colour, the note, the mind, and how long ago the thread was opened. It never shows
what Inkling said. Return or a click opens the note at those words, with the tag open and
the keyboard in its reply box. The list is built from the notes already in memory: no
index, no new file, no new field, and no call to the brain.

## What the writer sees and does

### Where it lives, and why ⌘K

The list lives in ⌘K's card, as its second list. The sidebar gets a row that opens it, not
a copy of it.

- **Finding a thread is the same act as finding a note.** ⌘K already takes typing to
  narrow, ↑ ↓, Return and Escape. One card for both means one way of finding things.
- **It is there only while you use it.** A list in the sidebar would sit beside whatever
  page you are on and list questions about other pages. Belief 3 says Inkling speaks only
  in the margin of the page you are on.
- **A quote needs width.** The sidebar is a 240 px drawer for finding notes by place; the
  ⌘K card is 460 px.

### How it opens

| Way | What happens |
|---|---|
| ⇧⌘K | The card opens on Loose ends, the search box has the keyboard. The key is in Settings › Shortcuts as "Loose ends", under Threads, with the line "Every open thread in the notebook, oldest first." |
| File › Loose Ends… | The same, from the menu bar, under Find or Start a Note… |
| The sidebar's "Loose ends" row, under New note | The same; the sidebar closes. The row has the spool icon and its keys (⇧⌘K). It carries no count. |
| ⇧⌘K while ⌘K is open | The card turns to Loose ends and keeps what you typed, so "budget" then ⇧⌘K shows what is open in the Budget note. ⌘K turns it back. |

⌘K's footer gains one hint at its end, "⇧⌘K loose ends", so the second list can be found
from the first.

### The card, word for word

```
┌────────────────────────────────────────────────────────────┐
│ (spool)  Find a loose end                                    │
├────────────────────────────────────────────────────────────┤
│  ×  If I hold 5:30 for the whole race, I'll come in just…   ↩ │  ← the highlighted row
│     Running / Sub-50 10k by March · Inkling · 3 weeks ago     │
│  ?  add a kilometre every week until I reach 16               │
│     Running / Sub-50 10k by March · Inkling · 3 weeks ago     │
│  ?  We cut the cloud bill by moving the cache to one node.    │
│     Work / Q3 budget · Skeptic · 2 weeks ago                  │
│  ?  Four runs a week is enough to get faster.                 │
│     This note · Socratic tutor · today                        │
├────────────────────────────────────────────────────────────┤
│ ↩ go to the words    4 open, oldest first          ⌘K notes │
└────────────────────────────────────────────────────────────┘
```

- **The search box** says "Find a loose end". Its icon is the spool (⌘K's is the
  magnifier), so the two lists are told apart at a glance.
- **Footer, left:** "↩ go to the words" while a row is highlighted.
- **Footer, middle:** "4 open, oldest first"; while typing narrows it, "1 of 4 open".
- **Footer, right:** "⌘K notes". Both key hints are the keys bound now, so a key changed in
  Settings › Shortcuts shows here, and an unbound one is left out.
- **No open threads at all:** "No open threads in any note." Nothing else: no praise, no
  illustration (belief 7: nothing rewards you for an empty list).
- **Typing that matches nothing:** "No open thread matches “xyz”."

### What a row shows

Two lines, always the same height:

1. **The mark and your words.** A ? for a question or × for a correction, in the thread's
   colour, exactly as its folded tab shows it (shape and colour, so colour is never the
   only signal). Then the quoted words, in the page's own typeface (Fraunces Text, 14 px,
   ink), on one line, cut with an ellipsis. A quote that spans paragraphs reads as one
   line. These are your words, so they are set like your words.
2. **Where and who.** In Figtree at 11 px: the folder and note ("Running / Sub-50 10k by
   March"), or "This note" for the note you are on; the mind's name in its thread colour,
   named as the tag's header names it (a thread with no mind's name says Inkling); and the
   age, in the quietest grey. A long folder name gives way before its slash, a long title
   before the mind, and a long mind's name before the age, which always shows.

The colour is the colour the page would draw: the note's own mind wears your thread, every
other mind its own (`note_page::mind_thread`, with the note's mind and the ink in
Settings). A row and the tag it opens never disagree.

**Ages** are counted in your calendar days: "today", "yesterday", "3 days ago" (up to 13),
"2 weeks ago" (up to 8), "2 months ago" (up to 11), "a year ago", "3 years ago". An old
thread looks exactly like a new one apart from those words: no colour, no "overdue".

**Order:** the oldest thread first, by when it was opened. Threads opened in the same
instant (the welcome guide's, a fixture's) go by the older note, then reading order.

**Typing narrows** the list to rows where every word you type appears somewhere in the
quote, the note's title, its folder or the mind's name, ignoring case. "skeptic budget"
finds the Skeptic's threads in the Budget note.

### Going to the words

Return, or a click on a row:

1. The card closes.
2. If the thread is on another note, the page saves the note you are on and opens that one,
   as the sidebar does.
3. The page scrolls so the words sit about a third of the way down the window, unless they
   are already well in view (then it does not move).
4. The caret goes to the start of the words, the thread's tag drops open, and the keyboard
   is in its reply box ("Write back…"): the state ⌥⌘↓ leaves. From there ⌘↵ ties it off,
   ⇧⌘⌫ dismisses it, and Escape folds the tag and leaves the caret on the words.

If threads are hidden (⇧⌘.), going to one shows them again: you asked for a thread. In
Marks only, the tag opens as a click on its ? would open it.

**Walking the list.** Opening Loose ends again highlights the row after the one you went to
last, whether or not you tied it off. So ⇧⌘K, ↩, answer, ⇧⌘K, ↩ works through the list
from the oldest without counting rows.

### How it stays calm

Belief 3 says Inkling speaks only in the margin of the page you are on. The list keeps to it:

- **It shows only your words.** A row never carries Inkling's question, correction or
  answer. Inkling's words appear where they always do, in the margin beside your words,
  once you go there. The list is a list of your unfinished sentences, not a feed of
  remarks.
- **It speaks only when asked.** It opens on ⇧⌘K, the menu or the sidebar row, never by
  itself. Nothing outside the open card counts the notebook's open threads: no badge, no
  number on the sidebar row, no toast. (The page header's "N loose threads" and the
  sidebar's per-note dots stay as they are: each counts one note.)
- **Nothing moves under you.** The rows are fixed for as long as the card is open (see
  below), so the highlighted row is always the one Return goes to.
- **It asks the brain nothing**, and going to a thread starts no read.

## Every state

| State | What happens |
|---|---|
| **Linen and Lamplight** | The card is ⌘K's: `card` ground, `rule` edges, the modal shadow, `hover` for the highlighted row, the dim over the page. The quote is `ink`, the note `graphite`, the age `faint` (4.5:1 on the card in both lights, as the theme measures it). The mark and the mind's name use thread colours, which `theme.rs` keeps at 4.5:1 on a tag in both lights. Screenshots of both are in `design/features/shots/`. |
| **Hundreds of threads** | The rows are a `uniform_list`, so only the rows in view are built, whether there are 7 or 7,000. ↑ ↓ move one row, Page Up and Page Down seven; the list scrolls to keep the highlight in view; typing narrows. |
| **None** | "No open threads in any note." The footer keeps only "⌘K notes". |
| **Keyboard only** | ⇧⌘K opens it with the keyboard in the search box. Type to narrow, ↑ ↓ and Page Up / Page Down to move, Return to go, Escape to close (the keyboard goes back to the note, the caret where it was). Tab stays inside the card, as in ⌘K. ⌘K turns to notes. Going leaves the keyboard in the tag's reply box. |
| **A thread tied off while the list is open** | The rows stay where they are. That row's mark becomes the knot a tied tab shows, and its age gives way to "tied off". Going to it still works: the tag opens tied off, with Reopen. A dismissed thread's mark fades and its age gives way to "dismissed"; going to it puts the caret where its words are. A thread that is gone altogether (its note deleted, or a thread that never got its first words) reads "gone" and Return does nothing. The next time the list opens, none of them is there. This is how a recheck that lands after you fixed the words, an answer that arrives in a reply, or a thread answered elsewhere in the note shows up while you are looking. |
| **Notes in folders** | The row names the folder before the note, "Work / Q3 budget". A folded folder hides nothing here: the list is the notebook, not the sidebar's view. A folder's name is searchable. A note whose folder was removed shows no folder, as ⌘K does. |
| **A note that waits to be asked** | Its open threads (from a selection, Ask, a call by name, or Read it through when that lands) are listed like any others. Going to one moves the caret and opens a tag and edits nothing, so no read starts; the note keeps waiting. |
| **The note you are on** | Its rows say "This note". They come from the page's own copy, so a thread opened or tied off a moment ago is right before the 600 ms save. Going to one scrolls and opens its tag without reloading the note. |
| **Words that are gone** | A thread whose words you deleted keeps the words it last quoted, and the row shows those. Going to it lands where they were, with its tag open, so you can tie it off or dismiss it. |
| **No date** | A thread written by hand or by an old build without `created_at` reads as 1 January 1970. It shows no age and sorts first. |
| **The same thread id in two notes** | Rows are keyed by note and thread together. (The `hard-content` fixture has `h01` in two notes.) |
| **Untitled notes, long names** | "Untitled". A long note title, folder or mind's name is cut as above. |

**Why the rows are fixed while the card is open.** The card takes a snapshot of which
threads are open, in order, when it opens; typing narrows that snapshot. Each row's state is
read again on every frame. A thread that opens on the page behind the card while you look is
not added (you will see it on its page, which is where Inkling speaks), and a thread that
ends is marked, not removed. Removing rows would move the highlight under your finger, and
Return would go somewhere you did not choose.

## Performance

**The list is built from the notes already in memory, not from an index.** `Store::open`
reads every note file at launch (`load_notes`), and the sidebar and ⌘K's full-text search
already run over `Store.notes`. Building the list is a walk over those notes' threads,
keeping the open ones, and one sort; it reads no file.

| Notebook | Threads | Open | Build and sort the list | Narrow it by two words | Read every row's state again |
|---|---|---|---|---|---|
| 1,000 notes | 20,000 | 2,000 | 1.1 ms | 18 µs | 0.12 ms |

Measured by `loose_ends::tests::a_thousand_notes_build_and_narrow_quickly` in the debug
build (opt-level 1) on this Mac, 7 Nov 2026. A frame is 16 ms.

An index was considered and rejected:

- **It would be a second copy of every thread's status.** It would have to change with
  every tie-off, dismissal, Undo, recheck, deleted note, Undo of a delete, import and hand
  edit of a note file. A stale index shows a thread as open when it is not, or hides one
  that is, which is the failure this list exists to prevent.
- **It would save nothing.** The cost of a big notebook is reading its files at launch, and
  the app does that anyway. An index would not remove it.
- **The walk is cheap.** Two thousand open threads out of twenty thousand take about a
  millisecond, a fifteenth of a frame.

The list is built once when the card opens; each keystroke narrows that snapshot (a
lowercase `contains` per row), and drawing builds only the rows in view.

## The data model

Nothing is added. No new field on `Note` or `Thread`, no new file, no index, so there is
nothing for old notes to default and nothing a newer build could write that an older one
cannot read.

What it reads:

| Type | Fields |
|---|---|
| `Note` | `id`, `title`, `folder`, `mind_id`, `threads` |
| `Thread` | `id`, `quote`, `kind` (× for `Correction`, ? for everything else), `status` (only `ThreadStatus::Open` is listed), `mind_name`, `start` (to break ties), `created_at` |
| `Folder` | `id`, `name` |
| `Mind` | `name`, through `Store::mind(note.mind_id)`, for the thread's colour |
| `Settings` | `ink` (the colour), `shortcuts` (the keys shown) |

The only thing that can be saved is a key you give "Loose ends" in Settings › Shortcuts,
through the mechanism every shortcut uses (`settings.json` `shortcuts`).

Kept in memory only, on the window: the last thread you went to and its row, for walking
the list. It does not outlive the app; a new launch starts at the oldest.

## Fit with the features designed alongside

The list has one rule: a thread is listed when its `status` is `ThreadStatus::Open`. It
never tests for `Dismissed` or `Resolved`, so any new way for a thread to end is left out
without a change here.

| Feature | What it means for the list | Shared types |
|---|---|---|
| **Answer in the note**, **Write it in** | A thread answered elsewhere in the note, or tied off by a reply that is written in, becomes `Resolved` and leaves the list. One that ends while the card is open shows "tied off" (above). | reads `Thread.status` |
| **Let it stand** | A point you let stand is not open, so it is not listed, whether it is stored as `Dismissed` with a reason or as a new status. One warning for that design: `ThreadStatus` has no fallback for a value it does not know, so a new variant in a note file makes the whole note fail to load in an older build. A reason on `Dismissed`, or a tolerant reader like `Block`'s `kind_or_paragraph`, avoids that. | reads `Thread.status` |
| **Turn it over** | The back of a note lists every thread with how it ended; Loose ends lists the open ones across notes. Their rows can share pieces: `loose_ends::age_words`, the mark, the colour through `mind_thread`, and choosing a still-open thread on the back of a note can call the same `NotePage::go_to_thread`. | none changed |
| **Threads of your own** | Your own open threads are listed like any other. The row's colour comes from `mind_thread` and its name from `loose_ends::who`; however that feature marks a thread as yours (an empty or reserved `mind_name`, or a new field), those two functions are the one place to teach. Today an empty `mind_name` means the note's own mind, for old notes, so it cannot also mean "you" without a field. | reads `Thread.mind_name`; `who` is the one place to change |
| **Show its working** | A correction still shows ×. A correction that turns into a question shows ?. A new field for the check (a sum or a source) is not read here. | reads `Thread.kind` |
| **Threads between notes** | One thread on two notes must be one row. If it is stored once, in the note where it was raised, with a link to the other, the list shows it once and goes to the words where it was raised; the row could name both notes ("Plan ↔ Budget"). If it is stored as two threads, one per note, they must share a link id, and `loose_ends::collect` is the one place that keeps the older of the pair. | `loose_ends::collect` is the one place to change |

## Tests

**Unit tests** in `src/loose_ends.rs`:

- Only open threads are listed, oldest first; ties go by the older note, then reading order.
- The page's copy of the open note wins over the store's: a thread tied off on the page is
  gone before the save, a thread opened a moment ago is there.
- The same thread id in two notes makes two rows, each going to its own note.
- Rows name the folder and note, "This note" for the open one, "Untitled" for a note with
  no title, no folder for a removed folder; a quote across paragraphs reads as one line.
- Ages: today, yesterday, 3 days, 2 weeks, 2 months, a year, 3 years, a time in the future
  (a clock that moved), and no age for a thread without a date.
- Narrowing: every word must match the quote, title, folder or mind, ignoring case, with
  characters whose lowercase changes length (İ, K).
- The snapshot keeps its rows and reads each row's state again: open, tied off, dismissed,
  gone.
- Walking: the row after the last one you went to, or the row now in its place if it left.
- A thousand notes: building and narrowing the list for 1,000 notes and 20,000 threads,
  with a generous time bound and the time printed.

In `src/note_page/goto.rs`, the scroll: words below or above the comfortable band scroll
to a third of the window, words inside it do not move, and the page never scrolls above
its top.

`src/shortcuts.rs`'s own tests cover the new key: every binding has a labelled row, and the
defaults do not clash.

**Scenarios**, with the fake brain (`brain/test/e2e/scenarios/le*.txt`, tag `loose-ends`;
`scripts/dev e2e loose-ends` runs them):

| Scenario | Fixture | What it proves |
|---|---|---|
| `le01-loose-ends` | `loose-ends`: four notes, two in folders (Work folded), one waiting to be asked, one untitled, threads open, tied off and dismissed, the id `t2` in two notes | ⇧⌘K opens it with the keyboard in the search box; rows `b1@Q3 budget, t1, t2@Sub-50…, t2@Untitled`, open only; folder, mind, mark and "This note"; Return goes to the waiting note's thread with the keyboard in the reply box; reopening starts on the next row; ↓ Return goes to another note; typing "kilo" narrows to one; ⌘K and ⇧⌘K carry "kilo" across; Escape gives the keyboard back to the note; `tasks.length === 0`. Shots in Linen and Lamplight. |
| `le02-tied-while-open` | `running-plan` | Fixing "5:30" with the card open: after the recheck the rows read `t1:tied off, t2:open`, still in place; Return opens `t1`'s tied-off tag; reopened, only `t2`; with both tied off, the list is empty and says so; the brain saw one `reply`. |
| `le03-hundreds` | `loose-ends-many`: 40 notes, 200 open threads | 200 rows oldest first; thirty Page Downs end on row 199; Page Up and ↑ come back to 191; "skeptic week 39" narrows to one; Return goes there. Shots in Linen and Lamplight. |
| `le04-hidden-marks-far` | `hard-content` | With threads hidden, going to one shows them; in Marks only, just its tag opens; "paragraph 30" in the sixty-paragraph note scrolls the page down to the words, with the keyboard in the reply box. |

The dump gains `loose_ends`: `open`, `query`, `selected`, `count`, and `rows` (each `note`,
`folder`, `thread`, `quote`, `mind`, `mark`, `state`, `here`, `age`). With the card closed,
`rows` is what it would show if opened now.

## Files and functions

New:

| File | What |
|---|---|
| `src/loose_ends.rs` | The list: `collect` (rows from the store and the page's note), `narrow`, `age_words`, `who`, `start_at` (walking), `refresh` (each row's state), `describe` (the dump); the `LooseEnds` view (the card, its keys, `uniform_list` rows) and its `LooseEndsEvent` (`Go`, `Notes`, `Dismiss`). |
| `src/app/goto.rs` | `AppView::open_loose_ends`, its events, `go_to_thread` (open the note, then the page's `go_to_thread`), switching between ⌘K and Loose ends with the typed words. |
| `src/note_page/goto.rs` | `NotePage::go_to_thread` (show threads, caret, tag, reply box) and `finish_goto` (the scroll, once the note is laid out), `goto_offset` (the pure scroll rule). |
| `brain/test/e2e/fixtures/loose-ends`, `loose-ends-many` | The notebooks above, written by `design/tools/loose_ends_notebook.py`. |
| `brain/test/e2e/scenarios/le01…le04` | The scenarios above. |

Small hooks in shared files, each a few lines, new struct fields last:

| File | Hook |
|---|---|
| `src/main.rs` | `mod loose_ends;` |
| `src/app.rs` | `LooseEnds` action; File menu item; three fields on `AppView` (the card, its subscription, the last thread gone to); `mod goto;`; Escape, focus, the corner hint and `close`/`open:` script steps know the card; the sidebar row; `on_action` and `children` in `render`. |
| `src/note_page.rs` | `mod goto;`; one field (`goto`); one call in `render` (`finish_goto`); one line in `debug_state` (`loose_ends`). |
| `src/palette.rs` | `query` and `set_query`, so the typed words carry across; the footer hint. |
| `src/shortcuts.rs` | the binding (⇧⌘K) and its label. |
| `AGENTS.md`, `design/capabilities.md`, `docs/agents/testing.md` | the map, the capability, the dump's new field. |

Size: **S**, as the manifesto has it: about a day with the tests.

Not touched: `src/welcome.rs` (the film agent's). The welcome guide must cover every
capability, so its Keys list needs one line for ⇧⌘K; that is for the film agent or the
owner.

## Open question for the owner

1. **Should a row also show the first line of Inkling's question?** It would make rows
   easier to tell apart, and it would bring Inkling's words off the page they belong to.
   Recommendation: no, rows show only your words, for belief 3; the mark, the mind and the
   note are enough to recognise a thread, and its question is one Return away. Built that
   way; showing it is a one-line change if you want it.

## Build notes (7 Nov 2026)

Built as designed, on `feat-loose-ends`. What the build taught:

- **The reply box waits for the tag.** A note opened a moment ago has no layout on its
  first frame, so it draws no tag, and a reply box that is not drawn loses the keyboard
  (the window then gives it back to the note). Going to a thread therefore leaves the
  keyboard in the note until the note is laid out, scrolls, and hands the keyboard to the
  reply box on the next frame, once the tag is drawn.
- **The dump cannot read the page.** The page builds its dump while it is being updated,
  so `loose_ends::describe` takes the page's note as an argument instead of reading the
  page (a read there panics).
- **`set_value` sends no Change.** The card narrows its rows itself when it opens with
  words carried over from ⌘K.
- **Scenario names.** Other agents number new scenarios `u12`, `u13`, …; these are `le01`
  to `le04` (tag `loose-ends`) so the names cannot collide.

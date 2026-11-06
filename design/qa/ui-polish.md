# UI polish (7 Nov 2026)

A pass over the small things, after the audit (`ui-audit.md`) and the critique
(`ui-critique.md`). Branch `ui-polish-2` (the old, merged `ui-polish` worktree still
exists, so `scripts/dev worktree ui-polish` refused the name). Every surface in
`design/capabilities.md` was driven by a script against the fake brain and looked at in
shots, in Linen and Lamplight, at window widths of 760, 1000, 1320 and 1800 and at a
520 px height, with the chat closed and open, on `running-plan`, `hard-content`,
`sewn-blocks`, `beta`, the Obsidian vault and a first run.

Alignment was measured, not eyeballed: every 2x shot was read pixel by pixel (ffmpeg's
raw RGB, runs of colour along a row or a column), so the figures below are device pixels
halved. Before shots come from master's binary at `4720ec4` (`INKLING_BIN`), after shots
from this branch, on the same fixture, size and steps.

The owner's decisions were kept: tags fold to a tab and drop open after a 120 ms rest,
with no hover sway, and drag anywhere on the paper; no top bar; the sewn blocks; one
stack of toasts; madder only ever Inkling's; neutral hovers; the selection toolbar's
filled keys; mind colours in Settings; Settings' rounded rectangles; ⌘K's light dim. The
icon and the wordmark were left alone (they are being changed on master).

**Severity.** High: broken, or seen by everyone often. Medium: plainly wrong once seen.
Low: polish.

**Shots** are pairs in `design/qa/shots/`, `polish-<name>-before.jpg` and
`polish-<name>-after.jpg`.

## Summary

| | Count |
|---|---|
| Findings | 33 |
| Fixed | 25 |
| Left, with the reason | 8 (2 of them for the owner) |
| High | 2, both fixed |

The ten a user would notice first:
1. An open tag covered your words: every left tag, and a right tag in a narrow window
   or beside the chat. Tags now open outward, over the cloth if they must, or narrower.
2. Opening Inkling on a long note showed it scrolled to the end, its title cut in half
   under the band of cloth. A note now opens at its top.
3. A question or reply that got no answer kept your words in the tag and nothing else
   once the toast had gone. The tag now says why, with Try again.
4. The sidebar appeared and vanished in one frame. It slides in and out.
5. ⌘H and ⌘M did nothing; the Inkling menu held only Settings and Quit. The menus now
   have Hide, Hide Others, Show All, Services and a Window menu.
6. In a narrow column the open count and Hide threads jumped from the left to the right
   of the header's second line as the window narrowed. They keep to the right.
7. ☰, Chat, the sidebar's × and the chat's header sat 1 to 2 px off the traffic
   lights' line. They sit on it.
8. Return was written two ways, ↵ in Settings, the selection menu and Help, ↩ in ⌘K;
   the ask box said "Enter to ask". It is ↩ everywhere.
9. A tag's send arrow looked ready with nothing to send and had no hover; a choice card
   under the pointer looked chosen.
10. Apostrophes were typewriter marks beside curly quotation marks.

## Fixed

Ordered by what a user would notice first. Commits are on `ui-polish-2`.

| # | Surface | What was wrong | Severity | What changed | Shots, checks |
|---|---|---|---|---|---|
| 1 | Margin tags | An unfolded or open tag covered the start of the text column. A left tab hangs 24 px out past the gutter, so its side of the paper was always 4 px short of a full tag and every left tag flipped rightwards over the words (the owner's Moon case on Start here). A right tag whose margin was short of a panel, in a narrow window or beside the chat, flipped leftwards over them. | High | The owner's rule: a thread never covers your words unless you dragged its tag onto them. A tag opens outward: on the paper; else over the sheet's edge onto the cloth, to 8 px inside the window (over the chat's edge when it is open); else narrower, down to 160 px, wrapping taller. Only a dragged tag opens back across the words, and only when not even the narrowest fits outward (`panel_fit`). The dump gains `panels` and `column`. | left-tag, never-cover, never-cover-moon-after; u13 (every tag of hard-content and the guide, 760 to 1800 px, both layouts, chat closed and open, both lights; master flips the 760 px case beside the chat, as the before shot shows), unit test `no_tag_opens_across_the_words` (every width 720 to 1800) |
| 2 | Page, at launch | A note longer than the window opened scrolled to its caret at the end (159 px on hard-content), the title cut under the band; switching notes opened at the top. | High | Every open holds the scroll, as the welcome guide did; the caret still waits at the end and the page follows it once you type. | u11 (opens at 0, typing at the end scrolls) |
| 3 | Open tag | A question or reply that got no answer (a stopped brain, a limit, an overload) left only your words; the reason went with the toast. The chat already said why with Try again. | Medium | The reason sits under your words, with Try again in the open tag, which sends the question or reply again. It goes when you switch notes, as the chat's does. A `retry:` step and the dump's `unanswered` drive it. | unanswered; u12 |
| 4 | Sidebar | It appeared and vanished in one frame (the owner's ask). | Medium | It slides in from the window's edge and fades up over 200 ms, easing out, and slides out over 160 ms, easing in, as the toasts do; the scrim fades with it; turned midway it heads back from where it is. Reduced motion: a 120 ms fade. Closing with the keyboard on a row hands the keyboard to the note at once. | sidebar-motion-after (caught mid-slide); u14 (partway in and out; ends fully open or closed after fast toggles) |
| 5 | Menu bar | No Hide Inkling (⌘H), Hide Others (⌥⌘H), Show All or Services, and no Window menu: ⌘H and ⌘M did nothing. | Medium | All of them, and Window › Minimise (⌘M) and Zoom, which act on the window in front, Settings included. Their keys stay out of Settings › Shortcuts (System Settings can change any menu item's keys). | not seen on screen: menus need focus |
| 6 | Header row | In a narrow column the second line (the open count, Hide threads) started at the left at some widths and the right at others, as the spacer wrapped with it or not. | Low | The group keeps to the right edge on whichever line it lands. | header-wrap |
| 7 | Window controls | The traffic lights centre 21.75 px down. ☰ sat 1 px above that line; Chat, the sidebar's × and the chat's header row 1 to 2 px below. | Low | All on the line. | chrome-line |
| 8 | Keys | Return was ↵ in Settings › Shortcuts, the selection menu and Help, ↩ in ⌘K. ↵ is not a Mac key glyph. | Low | ↩ everywhere; the README and the capabilities list follow. | return-key; unit test in `shortcuts.rs` |
| 9 | Ask box | "Enter to ask". | Low | "↩ ask", as ⌘K's footer writes its keys. | ask-hint |
| 10 | Open tag | The send arrow had no hover and looked the same empty, typed into or with an answer on its way; the chat's dims. | Low | Quiet until there is something to send, ink then, with a hover; the box redraws as you type. | reply-send |
| 11 | Export, Settings | A choice card or tile under the pointer turned its edge to the faint ink, close to the chosen one's ink edge (#6e6863 against #25232a on a shot where the pointer rested on Right margin). | Low | The hover edge is half that. | hover-tile-before (the after needs a real pointer) |
| 12 | Selection menu | T, H1 to H3, the quote, lists and checklist, B, I, U, S and </> had no tooltip while the colours did; "Code" (a block) and </> (code in a line) read alike. | Low | Each says what it makes; the marks and the checklist show their keys as bound now (⌘B, ⌘I, ⌘U, ⇧⌘X, ⌘E, ⌘L). | not seen: hover needs a real pointer |
| 13 | Icons | ☰, every close ×, and a sketch's ‹ › and replay had no words and no tooltip. | Low | Notes ⌘\, Chat about this note ⌘J, Close, Previous step, Next step, Play it again. | as 12 |
| 14 | Sidebar | New note showed ⌘N whatever its key was; every other row reads Settings › Shortcuts. | Low | It shows its bound keys. | |
| 15 | Mode menu, Settings | "Reads along" was explained as "Reads along and speaks up when…". | Low | "Speaks up when something’s worth a second look." | mode-words |
| 16 | Words | Apostrophes were typewriter marks (Couldn't, it's, Inkling's) beside curly quotation marks and the guide's ’. | Low | ’ in every string on screen; the about-you matcher still reads your own. | apostrophes; t02, r11 |
| 17 | Copy, Make a note | Copy the thread and the chat's Copy wrote "You:", Export and Make a note "Me:"; an older thread with no mind's name copied as ": …". | Low | Me in all four, Inkling for a nameless mind. | t13 reads what Copy writes |
| 18 | Dictation | "Listening…" sat 14 px from the window's corner, across the sheet's corner and the weave, where the corner hint was moved off. | Low | It takes the hint's place on the paper (beside the chat when open), lifted like the toasts. | listening |
| 19 | Sidebar | "No notes in here yet" started at 42 px, between a note's icon (28) and its title (49). | Low | At 49, under the titles. | folder-hint |
| 20 | Toasts | "Thread dismissed · Undo · ⌥⌘Z" ended 11 px from the label's edge; its words start 16.5 px in. | Low | 17 px. | toast-keys |
| 21 | ⌘K | The rows' note icons sat 2.5 px right of the search icon and the footer's ↩. | Low | Under the search icon (0.5 px); the titles stay under the search box's words. | palette-icons |
| 22 | Corners | ⌘K's rows had a 7 px corner, the other rows 6; the sketch seen larger 14, the other dialogs 12. | Low | 6 and 12. | |
| 23 | Export | It sat on the page's paper; ⌘K and the sketch dialog sit on the card. | Low | On the card. | export-card |
| 24 | Settings › Model | Coming next said "using the device code flow". | Low | "Sign in with your ChatGPT account, and pick Codex for any note." | |
| 25 | Window menu | "Minimize", where Inkling spells Colour and kilometre. | Low | "Minimise", as macOS spells it in British English. | |

## Left

| # | Surface | What is not right | Severity | Why it was left |
|---|---|---|---|---|
| 26 | Hairlines | Every rule, divider and card edge is 1 pt, 2 device pixels on a Retina screen, not one. Measured on ⌘K, the sidebar, the chat, Settings' groups and the tags: all crisp, all the same. | Low | **Owner's call.** Recommendation: keep 1 pt. The page's other lines are thread, drawn heavier than a hairline, and 0.5 pt lines would also blur on a 1x display. |
| 27 | Menu bar | No Edit menu (Undo, Redo, Cut, Copy, Paste, Select All, Emoji & Symbols). | Low | **Owner's call**, and a change of its own: a menu item's keys would compete with the component library's inputs (the reply box, the chat, ⌘K, Settings' fields). Recommendation: add it with macOS's own actions (OsAction) and check ⌘Z and ⌘V in each input. |
| 28 | View menu | Hide Threads keeps its name while threads are hidden (the sidebar's row does change). | Low | GPUI's menus do not change their titles as state changes. |
| 29 | Settings | The version (v0.1.0) sits about 1.75 px above the wordmark's baseline. | Low | The wordmark is being replaced on master. |
| 30 | Open tag | In the narrowest windows with the chat open, an open tag hangs over the chat's edge. | Low | The never-cover rule's order: over the window's cloth before narrowing. It is a popover, and the chat stays usable once it closes. |
| 31 | Page | "Just start writing." shows on the empty last line of a note that has words. | Low | By design (`placeholder_rules`). |
| 32 | Tags | The thread meets a folded tab at its lower edge, not at its loop. | Low | The rope runs under the tab to the loop; drawing it over the tab would cross its words. |
| 33 | Hover and tooltips | Not seen in a shot: background windows get no hover. | Low | 11, 12 and 13 were checked by reading the code. |

## Start here (`src/welcome.rs`, not changed)

Another agent rewrote the guide during this pass (master's "one engineer's note" on VM
cold starts), so it was only read and shot, in the version this branch started from.
Nothing in its words needed changing: its keys are written as macOS writes them (⌥⌘↓,
⌥⌘Z, ⌘J). Its fifth thread, the Moon correction with a sketch in the left margin, was
the owner's case for finding 1. u13 opens the guide's five threads at every width it
checks, so the rewritten guide is checked too, as long as `open: welcome` shows it and it
keeps at least five threads (it has five).

## How it was checked

- During the pass: `cargo test`, `scripts/dev lint` and the scenarios each change
  touches, and `scripts/dev check` along the way. At the end: `scripts/dev check --e2e`
  and the full `scripts/dev e2e`.
- New scenarios: u12-polish (the no-answer line and Try again), u13-tags-clear-the-words
  (121 opened tags against the column), u14-sidebar-motion. New expects in u11 (the
  scroll at open) and t13 (what Copy writes). New unit test
  `no_tag_opens_across_the_words`; the shortcut glyph test reads ↩.
- New script step `retry: n`; new dump fields `unanswered`, `panels`, `column` and
  `sidebar`; `tabs` now lists only the tabs drawn.

Not looked at: dictation itself (the badge was shown with a temporary knob, not
committed), the real model, VoiceOver, a 1x display.

# UI critique (6 Nov 2026)

A fresh look at every screen and state of Inkling, after the toasts, the keyboard ring and
the sewn blocks landed. Branch `ui-critique`. The surfaces in `design/capabilities.md`
were driven by a script against the fake brain and looked at in screenshots, in Linen and
Lamplight, at window widths of 760, 1000, 1320 and 1800 and at a 520 px height. The hard
content came from a new fixture, `hard-content`: a title that wraps three times, words and
a path with no spaces, twelve open threads from six minds (one with a long name), an
untitled empty note, a sixty-paragraph note, twenty more notes and folders with long,
folded and empty names.

The owner's decisions were kept: tags fold to a tab, drop open after a 120 ms rest and
drag anywhere; threads do not sway; there is no top bar; the logo and wordmark; the sewn
blocks; toasts in one stitched stack at the bottom centre. Madder is now only ever
Inkling's own colour, which four of the findings below were about (10, 12, 13, 21).

**Severity.** High: broken, or seen by everyone often. Medium: plainly wrong once seen.
Low: polish.

**Screenshots** are pairs in `design/qa/shots/`: `critique-<name>-before.jpg` is master
(`a068912`), `critique-<name>-after.jpg` is this branch; `-dark` pairs are in Lamplight.

## Summary

| | Count |
|---|---|
| Findings | 33 |
| Fixed | 25 |
| Left, with the reason | 8 |
| High | 2, both fixed |

The five a user would notice first:
1. Scrolled, the note ran under the traffic lights, ☰ and Chat; at 760 px the title sat
   on top of them. The page now scrolls under a band of cloth.
2. A reply you wrote in an open tag ran out of the tag and was cut off.
3. In Marks only, the ? or × after a thread's words sat on the next word.
4. In any one-margin layout (a window under about 1120 px, or the chat open in one under
   about 1430 px), the gutter's knots and numbers sat on the sheet's edge.
5. The corner hint ran across the sheet's corner onto the weave, and into the toasts in
   a narrow window.

## Fixed

Ordered by what a user would notice first.

| # | Surface | What was wrong | Severity | What changed | Shots |
|---|---|---|---|---|---|
| 1 | Page, scrolled | The page scrolled under the window's controls. At 760 px the title and the header row sat under the traffic lights and ☰; at any width a scrolled note's words ran under Chat. | High | The page scrolls below a 40 px band of cloth that holds the controls, so they always sit on the cloth, as the design says. Once scrolled, the band lays a soft hem shadow on the paper. | scroll-band, scroll-band-dark |
| 2 | Open tag | Your reply did not wrap. It ran past the tag's edge and was cut off mid-word, in every tag with a reply. | High | It wraps under itself, beside "You". | reply-wraps, reply-wraps-dark |
| 3 | Marks only | The ? or × after a thread's words was set at full size at the start of the next word ("?on", "×assuming"). | Medium | A small superscript, raised above the x-height into the space after the words, like a footnote mark. | marks-superscript |
| 4 | Page, one margin | In a window under about 1120 px, or with the chat open in one under about 1430 px, the column started at the sheet's padding, so the gutter's knot, number, read stitch and needle sat on the sheet's edge. | Medium | One-margin layouts keep 14 px more paper left of the column; at the narrowest it comes out of the cloth, not the words. A unit test checks every width from 326 to 2400 px. | gutter-on-paper |
| 5 | Page corner | "Hold ⌥ to speak · ⌘J chat · ⌘K notes" sat across the sheet's bottom-right corner, half on the paper and half on the weave. | Medium | It sits on the paper, in the sheet's padding, 12 px above the sheet's bottom edge. | corner-hint |
| 6 | Page bottom | The corner hint and the toasts share the bottom line; in a 760 px window a toast ran into the hint. | Medium | The hint steps aside while any toast is showing. | hint-and-toast |
| 7 | Open tag | A mind's long name was cut mid-word at the tag's edge, and its kind and number were pushed out of view. | Medium | The name ends in an ellipsis; "· question" and the number stay. | long-mind-name |
| 8 | Header, brief | With a brief that wraps, "Tell Inkling" sat beside its last line, at the bottom left of the block. | Medium | The label sits beside the first line. | brief-label |
| 9 | Header, toolbar, Ask the room | A mind with a long name ran out of the selection toolbar's Ask row to the window's edge, out of Ask the room's box as a chip, and past the column in the header row; the ask box's placeholder was cut mid-word. | Medium | The name ends in an ellipsis in each place, and the ask box says "Ask about this…" when the name is long. | long-mind-toolbar, long-mind-elsewhere, long-mind-header |
| 10 | Selection toolbar | Madder was offered as a colour for your words, so your marks could look like Inkling's. | Medium | Madder is gone from the swatches. Words already coloured madder keep it, with its Lamplight twin; a unit test holds both. | madder-swatch |
| 11 | Margin, many threads | Tags on one line stacked in the order the threads were made, while they are numbered in reading order: 10, 12, 11. | Low | They stack in reading order; threads on the same words keep the order they were made in. Scenario u11 checks it and fails on master. | tag-order |
| 12 | Page, another mind's thread | An open thread washed its words in madder, whichever mind asked. | Low | The wash is the thread's own colour. | thread-wash |
| 13 | Menus, sidebar, ⌘K, toolbar, toasts, icon buttons | Hovers, the chosen row and a selection the keyboard has left were a madder tint. Choosing another thread colour left the interface madder anyway. | Low | A plain shade of the ink (`Palette::hover`). The thread colour still marks where the keyboard is, as designed. | neutral-hover, neutral-hover-dark |
| 14 | Header menus | The mind menu hung 5 px right of its button; the mode menu's titles sat 24 px right of "Reads along". | Low | Each menu hangs so its words line up with its button's: dot over dot, title under title, the mode icons out to the left. | menu-alignment |
| 15 | Sheet, toasts | The sheet ended 12 px above the window's bottom on a short note and 28 px at the end of a long one, so a short note scrolled by 16 px, and toasts sat 6 px above the sheet's edge. | Low | The sheet always ends 12 px above the bottom; toasts sit 12 px above its edge (u08 checks where). | corner-hint, hint-and-toast |
| 16 | Open tag, gutter | Sizes off `theme::ts`: 14.5, 13.5 and 12.5 px in the tag, 10 px for the gutter's numbers. | Low | 14, 13, 12 and 11 px, named from the scale (the tab's literals too). | reply-wraps |
| 17 | Sidebar | A folded folder's count stopped 23 px short of the notes' counts, because the hidden remove × kept its place; long folder names were cut early for the same reason. | Low | The count lines up; the × takes its place under the pointer. | sidebar-counts |
| 18 | ⌘K | A long folder name was cut with its slash. The footer said "↩ open" and offered "⌘⌫ delete" while a Start row was chosen. | Low | The slash stays; the footer says "↩ start" on a Start row and shows ⌘⌫ only on a note. | palette-folder-slash, palette-footer |
| 19 | Export | A long title lost its closing quotation mark. | Low | It is cut at a word inside both marks. | export-title |
| 20 | Sidebar | The link to Settings was called "Customize Inkling", where the menu, Help and Shortcuts say Settings. | Low | Settings. | sidebar-settings-link |
| 21 | Settings › Model | The dot beside Claude was madder whatever the state. | Low | Sage when Claude answers, ochre when the brain is not running (as a problem's toast), faint while it starts. | claude-status-dot |
| 22 | Settings › New mind | How it thinks set its words 2 px left of the fields above it. | Low | They line up. | mind-fields-aligned |
| 23 | Start here | Keys written ⌘⇧ (⌘⇧C, ⌘⇧.), where the sidebar, Settings and macOS write ⇧⌘. | Low | ⇧⌘ throughout. | guide-key-order |
| 24 | Toasts | A stopped brain read "Inkling's brain isn't running (the brain stopped (exit status: 3))". | Low | "Inkling's brain stopped (exit status: 3)." t04 checks the words. | brain-stopped |
| 25 | Open tag | "Write back…" sat about 18 px inside its box, twice the inset of the ask box and the chat, because the field kept its own padding inside the box's. Found in review, after the pass. | Low | The box's padding is the only inset, as in the other two. | reply-box |

## Left

| # | Surface | What is not right | Severity | Why it was left |
|---|---|---|---|---|
| 26 | Buttons | Settings' buttons are rounded rectangles (New mind, Edit, Check again); the page's dialogs use pills (Save, Export…). | Low | Settings follows macOS on purpose (the design record, Settings). One family for both is a call for the owner. |
| 27 | Selection toolbar | The current block is shown filled in ink for T, H1 to H3 and the lists, and outlined in its thread for Note, Warning, Tip and Code. | Low | Filling a coloured key would put a block of colour in the menu. It reads as two kinds of key; a call for the owner. |
| 28 | ⌘K | Its backdrop is a quarter as dark as Export's and the enlarged sketch's. | Low | It reads as deliberate: a quick switcher, as Spotlight dims nothing. |
| 29 | Tags | A tag dragged to the window's edge hangs over the sheet's edge and the cloth. | Low | "Tags drag anywhere" is the owner's decision; the drag keeps them inside the window and below the controls. |
| 30 | Note | Tab in a plain paragraph makes it a bullet. | Low | The note keeps Tab for its lists by design; ⌃Tab leaves the note. |
| 31 | Open tag | Its top row starts 14 px right of its words, to clear the loop the thread is tied to. | Low | By design: the loop stays where the thread ties on. |
| 32 | Callouts | A callout's stitched edge is heavier than a tag's or a toast's. | Low | The sewn blocks are the owner's decision, sized from the explorations board; a patch is meant to be bolder than a label. |
| 33 | Settings › Minds | Minds are listed without their thread colour, which the page uses to tell minds apart. | Low | A small feature rather than polish. |

## How it was checked

- Surfaces: the page and header row, every block kind (sewn ones included), gutter
  marks, folded, unfolded, open, replied, dragged and tied-off tags, tags at the window's
  edges, twelve on one page, three minds on the same words, Both margins, Right margin
  and Marks only, the sidebar and folders, ⌘K, export, chat (with its failure line),
  sketches inline and enlarged, the ask box and Ask the room, every Settings pane
  (recording a shortcut and a clash included), Start here, toasts, a stopped brain, an
  empty note, the reading needle, ink from the pointer, the focus ring through the Tab
  order.
- Before shots were taken with master's binary (`INKLING_BIN`), after shots with this
  branch, on the same fixture and window size.
- `scripts/dev check --e2e` and the full `scripts/dev e2e` pass (42 scenarios; `real-1`
  needs the model and was skipped). New: unit tests
  `the_gutter_marks_sit_on_the_paper` and `madder_is_never_offered_for_your_words`; the
  `hard-content` fixture and scenario u11 (twelve tags without overlaps, in reading order,
  the gutter on the paper at 760 px), which fails on master; an expect in u08 for where
  the toasts sit; t04 checks the new words for a stopped brain.

Not looked at: dictation (it needs the microphone), the real model, VoiceOver.

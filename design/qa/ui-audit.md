# UI audit and polish pass (6 Nov 2026)

Branch `ui-polish`, rebased on master `90bdf62` (tags that fold to a tab, no top bar,
the Start here tour, agent tooling). Every screen and flow below was driven by a script
against the fake brain and checked in a screenshot, in linen and lamplight. Fixes are
committed on the branch. Findings in code other agents own are reported here, not
changed: the window chrome, the margin tags and threads, the page geometry, Settings
and the welcome tour.

**Severity**: High means broken, or data at risk. Medium means wrong, confusing or
plainly visible. Low means polish.
**Status**: Fixed (on this branch); Reported (in code someone else owns); Open (mine, but
needs a design call); Gone (no longer applies after the rebase).

Screenshots are in `design/qa/shots/`:
- `before-*`: the build this pass started on;
- `after-*`: this branch, on the new UI;
- `report-*`: findings left to the owners of that code;
- `new-ui-*`: what master changed.

## Summary

| | Count |
|---|---|
| Findings | 59 |
| Fixed on this branch | 45 |
| Reported to the owners of that code | 11 |
| Open (design call) | 1 |
| Gone after the rebase | 2 |
| High severity | 7, all fixed |

The worst, all fixed:
- Keyboard focus was lost after a thread closed from the keyboard. Every shortcut and
  all typing stopped working until the next click.
- Clicking any chrome took the keyboard away from the note.
- ⌘⌫ in ⌘K did nothing, and deleting a note had no undo.
- Coloured words were nearly invisible in dark mode.
- The sketch seen larger closed on any click inside it, so its step controls could not
  be used.
- The palette card stretched to its full height whatever it held.
- Escape did nothing in most of the boxes that said "Esc to cancel".

## Rebase onto master `90bdf62`

**What conflicted.**

| File | What both sides changed | How it was resolved |
|---|---|---|
| `src/script.rs` | Both added script steps. Master: `hover: n` (a thread's tag), `drag:`, `shot:`, `settle`, `expect:`. This branch: `key:`, `click:`, `hover: x, y`, `size:`, `snap:`. | All of master's steps kept. This branch's pointer step became `point: x, y`, because `hover: n` already means a tag. `snap:` and the `qa` cargo feature behind it were dropped in favour of master's `shot:` and `scripts/dev shot`. |
| `src/app.rs` | The script runner (master added `settle`, this branch sends `key:`, `click:`, `point:` to the window as platform input). The sidebar header (master moved the wordmark there, this branch changed its close button). | One runner handles all of the steps. The header keeps the wordmark, with the new close button. |
| `src/note_page.rs` | The sketch caption (both edited the same lines). The model error text (master cuts it between words; this branch names a brain that is not running). | Both kept in each case. |
| `brain/test/e2e/scenarios/t09-robust.txt` | Not a text conflict: a chat with nothing to say now says so in the chat, not in the page notice. | The expectation reads the new `chat_error` field of the dump. |

The first full `scripts/dev e2e` after the rebase failed four scenarios.
- **t04-dying and r11-errors**: the app crashed. This branch's failure text reads the brain
  (when the usage limit resets, whether the brain is running), and a brain that stops
  fails its waiting requests from inside its own update. Reading the brain there panics.
  This was a regression from this branch, and it is fixed: the text is now set right after
  that update.
- **t09-robust**: the expectation above.
- **r13-width**: a resize that had not landed when the dump ran, on a loaded machine. It
  passed on the rerun.

**Findings that no longer apply**:
- #18: "Show threads" twice. The top bar that held the second one is gone.
- #26: threads on the same words were numbered in id order. Master now numbers them in
  the order they were made, as their tags stack.

Every other finding was checked again on the new UI, in light and dark.

**What is new in the new UI** (from master):
- Tags fold to a small stitched tab (loop, number, × or ?). They unfold on hover, open on
  click, drag anywhere and keep their place.
- There is no top bar. The traffic lights, ☰ and Chat float over the cloth, and they fade
  while you type (so does the page's header row). The wordmark is in the sidebar.
- Start here is a working tour with real threads, and Help › Welcome Guide reopens it.
- `scripts/dev` (check, e2e with `expect:`, state, shot, show, lint) and JSON logs.

Three findings come from the new UI:
- #59 is new: the open tag's Copy is an unlabelled icon.
- #56 is easier to see, because the folded tabs now sit close to the column.
- #57 is in the new floating chrome.

`new-ui-tab-unfolded-dark.jpg`, `after-page.jpg` and `after-thread-open.jpg` show the new
UI: folded tabs, a tab unfolded under the pointer, and an open tag.

## Keyboard and focus

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 1 | Page | ⌥⌘↓ then ⌘⇧⌫ (or Resolve or Dismiss on a tag) removed the reply box that had the keyboard, and nothing else took it. ⌥⌘↓, ⌘J, ⌘K and typing then did nothing until a click. The window now gives the keyboard back to the note, or to the open dialog, whenever focus is lost. | High | Fixed | (state dumps) |
| 2 | Page | A click on the linen, the header row or a menu moved focus to the window itself, so typing went nowhere. The window no longer takes focus; the export dialog takes it instead. | High | Fixed | |
| 3 | Ask the room | "Esc to cancel" did nothing. The box also stayed open after Sketch it or Find a loose thread was started from somewhere else. | Medium | Fixed | before-ask-box-stays-open.jpg |
| 4 | Thread reply box | Esc in the reply box did not close the thread (⌥⌘↓ puts you in that box), though the design says Esc closes. | Medium | Fixed | |
| 5 | Header menus, brief editor, sketch seen larger, chat box | Esc did nothing in any of them. Now one Escape path closes the innermost thing, one per press, and gives the keyboard back to the note. | Medium | Fixed | |
| 6 | ⌘K palette | ⌘⌫ never deleted the highlighted note, because the search box's own ⌘⌫ binding ran first. The documented shortcut did nothing. | High | Fixed | |
| 7 | ⌘K palette | Deleting a note had no confirmation and no undo, and ⌘⌫ is one key away from clearing the search box. Now "Note deleted · Undo" shows for 8 s. Undo puts the note back where it was, opens it, and removes the blank note made to stand in for it. | High | Fixed | after-delete-undo-toast.jpg |
| 8 | Everywhere | No visible focus ring on any control, no Tab order between buttons, no arrow keys in the mind and mode menus, and the sidebar works only with the mouse. | Low | Open (design call: needs focusable controls) | |

## ⌘K palette

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 9 | Palette | The card stretched to its 420 px maximum whatever it held: one row, then a large empty block. | High (seen at every ⌘K) | Fixed | before-palette-stretched-light.jpg, before-palette-stretched-dark.jpg → after-palette.jpg, after-palette-dark.jpg |
| 10 | Palette | Rows had no hover state, long titles did not truncate, and the arrow keys moved the highlight out of view in a long list. | Low | Fixed | after-palette-scrolled.jpg |
| 11 | Palette | Footer hints were 10.5 px, and two different glyphs (↵ and ↩) were used for Return. | Low | Fixed | |

## The page header (mind, Tell Inkling, mode, open count, Hide threads)

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 12 | Mode menu | It hung about 30 px left of "Reads along", at a fixed offset that moved further off whenever the mind's name or "Tell Inkling" changed width. Each menu now hangs under its own button. | Medium | Fixed | before-mode-menu-offset.jpg → after-mode-menu.jpg |
| 13 | Header menus | Pressing an open menu's button closed it on mouse down and opened it again on click. | Medium | Fixed | |
| 14 | Header | None of the five controls had a hover or pressed state. | Low | Fixed | |
| 15 | Mind menu | Every mind's dot was ink, but the design gives each mind its thread colour. | Low | Fixed | before-mind-menu.jpg → after-mind-menu.jpg |
| 16 | Mode menu | Its descriptions differed from the same choice in Settings, and its icons differed in size and colour. | Low | Fixed | |
| 17 | Header | "Hide threads" showed on notes with no threads. | Low | Fixed | |
| 18 | Hidden threads | "Show threads" appeared twice: in the top bar and in the header. | Low | Gone (master removed the top bar) | before-hidden-show-threads-twice.jpg |
| 19 | Hidden threads | An open thread stayed open behind the clean page, so ⌘⇧⌫ could dismiss a thread you could not see. Hiding threads now also closes the thread, the question box, the toolbar and the menus. | Medium | Fixed | |

## Selection toolbar

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 20 | Toolbar | Several problems: hints truncated ("Stitched out step by s…"); B, I, U and S were plain letters; the actions switched to Fraunces under "Like your notes" lettering, though controls stay Figtree; nothing showed the active colour; the swatches were unnamed, so colour was the only signal. | Medium | Fixed | before-toolbar.jpg → after-toolbar.jpg, after-toolbar-dark.jpg |
| 21 | Toolbar | It said "Ask Inkling" whatever the note's mind was. | Low | Fixed | |
| 22 | Words, dark | Coloured words drew their linen value on the lamplight page, so indigo, plum and walnut fell under 2:1 contrast. Each toolbar colour now has a lamplight twin, tested at 4.5:1 on both pages, and colours stored by older builds are lightened on the dark page. | High (dark mode) | Fixed | after-colours-dark.jpg |

## Ask the room

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 23 | Room chips | "Blank", the empty template, was offered as a reader. | Medium | Fixed | before-ask-box.jpg → after-ask-box.jpg |
| 24 | Room chips | Pressing a fourth mind's chip did nothing, with no sign why. It now greys out while three minds are seated. | Low | Fixed | |
| 25 | Ask box | The hint wrapped to two lines at the margin's width. | Low | Fixed | |
| 26 | Room tags | Room threads on the same words were numbered in id order, so the tags read 3, 2, 1 from the top. | Low | Gone (fixed on master) | report-room-numbers-and-stitches.jpg |
| 27 | Room stitches | Several threads on the same words draw their running stitches on top of each other, so only one mind's colour shows. One idea: interleave the dashes, one colour per mind. | Low | Reported (`stitch.rs`) | report-room-numbers-and-stitches.jpg |

## Chat

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 28 | Chat | It did not follow the conversation: new answers landed below the fold. | Medium | Fixed | before-chat-long.jpg |
| 29 | Chat | A failed question hung unanswered, and the reason showed in the far corner of the page. The chat now says why under the question, with Try again. | Medium | Fixed | before-chat-offline.jpg → after-chat-failure.jpg |
| 30 | Chat header | The subtitle wrapped mid-phrase ("Reading this note: 0 / threads (0 open)"). Copy and Clear were active on an empty chat, Copy gave no confirmation, and the close button sat higher than the words beside it. | Low | Fixed | before-chat-empty.jpg → after-chat.jpg, after-chat-dark.jpg |
| 31 | Chat | The send button looked active with nothing to send, or while an answer was on its way. | Low | Fixed | |
| 32 | Chat | The empty state said "I've read the whole note and its threads" on notes with no threads. | Low | Fixed | |

## Sketches

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 33 | Sketch seen larger | Any click inside it closed it, so its ‹ › and replay controls could not be used. | High | Fixed | before-enlarged-sketch.jpg → after-enlarged-sketch.jpg |
| 34 | Sketch seen larger | It offered "See it larger" inside itself. | Low | Fixed | |
| 35 | Sketch controls | ‹ › and replay were 10 to 11 px glyphs with no hit area and no hover. | Low | Fixed | after-sketch.jpg |

## Tell Inkling

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 36 | Brief editor | Esc did not cancel. The typed text sat 10 px in from its label. The brief, which is the writer's own words, took Inkling's lettering. The saved brief had no hover state. | Low | Fixed | before-brief-editor.jpg → after-brief-editor.jpg |

## Sidebar

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 37 | Sidebar | The open note looked exactly like a hovered row, and open threads showed as an unlabelled dot. | Low | Fixed | before-sidebar.jpg → after-sidebar.jpg, after-sidebar-dark.jpg |
| 38 | Sidebar | "Hide threads" still said "Hide threads" while threads were hidden. | Low | Fixed | |
| 39 | Folders | An empty open folder showed nothing, and a new folder could repeat an existing name. | Low | Fixed | after-sidebar-folders.jpg |
| 40 | Folders | Removing the open note's folder left the page holding the old folder id, which it wrote back on the next save. | Low | Fixed | |

## Export

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 41 | Export | Markdown, HTML and Word dropped every mark (bold, italic, struck, underlined, code and colours), and so did ⌘⇧C. | Medium | Fixed | |
| 42 | Export, HTML | It was styled in the old palette: Helvetica for your words and Georgia for Inkling's, the serif voice the design rules out. Now your words are in Fraunces, the threads in Figtree, with a madder running stitch beside each thread. | Low | Fixed | after-export-html.jpg |
| 43 | Export dialog | At 330 px wide, each option wrapped to five lines. Return and the arrow keys did nothing, and the threads switch was labelled "Include the margin". | Low | Fixed | before-export.jpg → after-export.jpg, after-export-dark.jpg |

## States and messages

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 44 | Errors | The usage-limit message never said when the limit resets, though the brain knows. The sign-in message showed literal backticks. A brain that was not running was reported as "Couldn't reach the model". | Low | Fixed | |
| 45 | Errors | The error notice and "Thread dismissed · Undo" both sit at the bottom left, and they overlap. | Medium | Reported (`render`) | report-error-and-undo-overlap.jpg |
| 46 | Errors | The error notice cannot be closed, and it stays for 12 s. | Low | Reported (`render`) | |
| 47 | First run | The invitation placeholder ran past the column into the margin. The wider column on master hides this at the default width, but it still happens in narrow windows. | Medium | Fixed | before-first-run-placeholder-overflow.jpg |
| 48 | Corner hint | "Hold ⌥ to speak · ⌘J chat · ⌘K notes" drew on top of the dimmed dialogs and the drawer. | Low | Fixed | before-export.jpg |
| 49 | Thread to note | "Make a note" cut the title mid-word at 60 characters. | Low | Fixed | |

## Type and surfaces

| # | Screen | What was wrong | Severity | Status | Screenshot |
|---|---|---|---|---|---|
| 50 | Everywhere | The interface used thirteen text sizes, from 9.5 to 17 px. There is now a five-step scale in `theme::ts` (11, 12, 13, 14, 15). It is used by the palette, sidebar, header, menus, toolbar, question box, chat, export dialog, toasts and shared controls. | Low | Fixed (in the code this branch owns) | |
| 51 | Everywhere | Each dialog and popover had its own shadow or none, and buttons had no pressed state. There are now shared `popover_shadow`, `modal_shadow`, `icon_button`, `switch` and `key_hint` helpers, and every shared control has a pressed state. | Low | Fixed | |

## Reported to the owners of that code

| # | Area (owner) | What is wrong | Severity | Screenshot |
|---|---|---|---|---|
| 52 | Settings | The "Ink from the pointer" switch overflows the right edge of its card. | Medium | report-settings-switch-overflow.jpg |
| 53 | Settings | Several small issues: text inside fields is indented by the component kit's padding; the dots by the title and in the minds list are ink, not thread colours; the close ✕ has no hover; the 15, 12.5, 11 and 10.5 px sizes are off the shared scale; it keeps its own switch, though `ui::switch` now exists; it has no shadow, unlike the other dialogs. | Low | report-settings-dark.jpg, report-settings-newmind-dark.jpg |
| 54 | Settings and the page | In Marks only, Settings promises "a small ? after the passage", but the page draws a plain dot. | Low | report-marks-only-dot.jpg |
| 55 | Page geometry (`Metrics`) | In a narrow window with the chat open, the page runs under the chat, and the header row and the right-hand tags are cut off. | Medium | report-narrow-chat-overflow.jpg |
| 56 | Margin and gutter (`render`) | A left-margin tab's thread runs across the gutter knot and number of its paragraph. | Low | report-gutter-under-left-thread.jpg |
| 57 | Window chrome | The Chat pill's text is 11.5 px, off the scale. | Low | |
| 58 | QA tooling | The `computer-use` app tools pick one process per bundle id, and every build is `dev.local.inkling`. With the owner's Inkling running, a QA window cannot be screenshotted or clicked that way. Scripts with `scripts/dev shot` work. | Low | |
| 59 | Open tag (`card`, new on master) | Copy is an unlabelled icon, with no tooltip, no hover state and no confirmation when it copies. The design's rule 3 asks for plain words on every control. | Low | |

## How it was tested

- New script steps in `src/script.rs`:
  - `key:` sends keystrokes as the platform would;
  - `click: x, y` and `point: x, y` act at a point in the window;
  - `size: WxH` resizes the window;
  - `set:` now also takes `appearance`, `size`, `ink`, `lettering` and `pointer`.

  With these steps every flow above ran without taking focus. The `dump:` JSON now also
  reports focus, menus, the selection, the chat and its failure line.
- Notebooks used:
  - the `running-plan` fixture;
  - a notebook with every block kind and mark;
  - a notebook with 40 notes;
  - a long note and an empty note;
  - a folder with a long name;
  - first run (the new tour);
  - a missing brain.
- Settings and window sizes covered:
  - light and dark;
  - text sizes Small to Large, and both letterings;
  - placements Both, Right and Marks only;
  - windows from 720 × 480 to 1700 × 1000, with the chat open and closed.
- Checks that pass: `scripts/dev check --e2e`, the full `scripts/dev e2e` (33 scenarios,
  `real-1` skipped), `cargo test` and `bun test`.

## Not tested

- Dictation (hold ⌥), which needs the microphone and speech permission.
- The real model; every run used the fake brain.
- Word output beyond the export test's round trip through `textutil`.
- VoiceOver labels.
- Settings tabs behind the ones listed above. Settings belongs to another agent, so it was
  only looked at, not driven.

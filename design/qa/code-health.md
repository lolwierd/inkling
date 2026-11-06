# Code health pass, 7 November 2026

Branch `code-health`, rebased onto master `0839bc9` (all nine features merged: Let it stand,
Show its working, Loose ends, Turn it over, Threads between notes, Read it through, web
sources, Threads of your own, Answer in the note and Write it in, with the UI polish and
the app icon) with every side's changes kept: the features' new page state (`goto`,
`disputes`, `unanswered`, `read_through`, the visitors of threads between notes) sits beside
`Tag` and `ChatReply` and needed no folding, since none of it read the old tag maps or the
chat's two Options. The ask: find the performance problems, bugs, implicit state
machines, code smells and complexity in the app (`src/`) and the brain (`brain/src`), and
fix them, with the behaviour the owner designed left exactly as it is. This page is the
record: each finding, what changed, and the evidence; the measurements before and after;
what was left and why.

In short: ten findings, all fixed on the branch (three of them the owner would feel: the
tag drag that followed the pointer in jumps, the whole note laid out twice a frame and
scanned once per line every frame, and the threads re-solved on every reflow), the
compiler's twelve dead-code warnings gone, two implicit state machines made explicit
(a tag's hover and spring; the chat's answer), and the structural splits listed for a pass
of their own.

How things were measured: `INKLING_FRAME_LOG=1` (new, `src/perf.rs`) logs frames a second,
the longest gap between frames, what the editor's layouts cost and what each timed span of
a frame cost. Runs used a scratch notebook with a 600-paragraph note (164 KB of text, 20
open threads, every paragraph already read: ten copies of the sixty-paragraph note in
`hard-content`) and the `running-plan` fixture (a short note with two threads). CPU is the
process's CPU time over a window of wall time, from `ps`. The debug build has `opt-level
= 1` for the app and 2 for its dependencies. Two cautions learnt on the way: a covered
window stops drawing (no frames, no CPU), so a run without frame reports measures nothing;
and the jiggle that keeps a covered window drawing changes the window's width, which lays
the note out again every tick, so idle numbers were taken without it.

## Findings

### 1. Dragging a tag followed the pointer in jumps (the owner's "so jittery")

Where: `src/note_page.rs`, `render` (the page root's `on_mouse_move` and `on_mouse_up`),
`card` (the tab's `occlude()`), `tag_panel` (the panel's `occlude()`).

What was wrong: the drag handler was a mouse-move listener on the page's root element.
GPUI runs such a listener only while the element's hitbox is in the hit test, and
`Frame::hit_test` stops at the first occluding hitbox under the pointer. The tab is
occluding on purpose (a tag dragged over words must not move the caret) and so is the
panel that unfolds under the pointer during a drag. While the pointer sat on the tag it was
holding, the root heard nothing; a move got through only once the pointer had slipped off
the tag, the tag then jumped to the pointer, which put it under the pointer again, and so
on: jumps of about half a tab. The root's mouse-up listener was gated the same way, so a
drag ended only when the pointer next left the tag, and until then tabs ignored clicks
(`drag_moved_recently`).

Why it matters: moving a tag is one of the few things the owner does with the pointer, and
it was the first thing they reported from the release build.

What changed: while a tag is held, the page listens to the window itself (`drag_listener`:
`window.on_mouse_event` for mouse moves and the left button coming up, registered from a
canvas in `render`), not to what is under the pointer. Two small methods (`drag_pointer`,
`drag_listener`), the two root listeners gone; `tag_panel` untouched.

Evidence: a scripted pointer drag (new `press:`/`release:` steps; `point:` moves carry the
held button) of 36 steps of 6 px on the running-plan note, debug build, frame log on.
Before: 1 of 36 moves reached the handler; the tag moved once, by 42 px, in the whole drag,
and its hover flapped on and off at every step as the panel appeared over the tab. After:
36 of 36 moves reached it; the tag's offset tracks the pointer step by step (6, 12, 18 …
px) and the release ends the drag at once. Scenario `u17-pointer-drag` pins it.

Seen on the way, left for the polish branch's `panel_fit` (see "Left"): the open panel
flips sides at one fixed x with no hysteresis, so a tag dragged back and forth across that
line flips its panel every frame.

### 2. Every frame scanned the whole note for every line (O(lines × text))

Where: `src/editor.rs` `State::line_range` / `line_of`; `src/note_page.rs` `render` (the
gutter loop calls `line_range` for every line, every frame; `consider`, `note_rechecks`,
`on_pause` and `pick_side` call them too).

What was wrong: the editor keeps the note as one string with no line index, so
`line_range(i)` split the text from the start to find line i and `line_of(offset)` counted
newlines up to the offset. The page's render called `line_range` once per line to draw the
gutter: on the 600-paragraph note that is 600 scans of 164 KB, about 100 MB of `memchr`,
every frame. `sample` showed `memchr` under `CharSearcher::next_match` under
`editor::line_range_in` as the hottest user-space frames both idle and typing.

What changed: `State` keeps `starts`, where each line begins, rebuilt by `reindex` after
every change to the text (the three places the text changes: `load`, `restore`, `replace`).
`line_range` is a lookup, `line_of` a binary search, `title` and `lines` read through the
index. The headless test harness asserts the index matches the text after every operation
(`check`), so the random-editing test covers it.

Evidence (long note, debug): idle CPU 3.9% → 1.0%; typing 17.3% → 5.7%.

### 3. The editor shaped the whole note twice a frame, every frame

Where: `src/editor.rs` `EditorElement::request_layout` / `prepaint`, `layout_doc`.

What was wrong: the measured-layout closure built a `DocSnapshot` (cloning the whole text)
and laid the whole note out on every call, and the layout engine calls it twice a frame;
`prepaint` had a third path. Nothing was kept between frames. GPUI's own shaping cache made
a warm layout cheap (0.4–1 ms for 602 lines) but it is two frames deep, so after any pause
in frames (the caret blink is 530 ms) the next edit shaped everything cold: 20–40 ms, a
visible hitch on the first keystroke.

What changed: the editor keeps its layout with a `LayoutKey` of what it was built from (the
text's version, block kinds, marks, the IME range, width, text size, palette) and reuses
it while the key matches: an idle frame, a caret move or a scroll shapes nothing. When the
key changes, `layout_doc` carries over every line whose kind, words and runs are unchanged
(found at its old index, or that index shifted by the lines the edit added or removed), so
a keystroke shapes one line, whatever GPUI's cache holds. `DocSnapshot` borrows the text.
Painting skips lines scrolled out of view, and the needle asks for frames only while it is
in view. `Palette` derives `PartialEq` for the key.

Evidence (long note, debug): idle frames do 0 layouts (were 2 a frame); typing 2.8% CPU
(was 17.3%), one layout a keystroke shaping 1 of 603 lines in 0.2–0.4 ms (was two warm
layouts of 1 ms, or one cold of 20–40 ms); opening the note is one cold layout of 17–30 ms.
Unit test `a_layout_carries_unchanged_lines_over_and_shapes_only_the_rest`: what is shaped
again after typing, a new line, a merge, a mark, a kind change and a width change, and
that the carried layout equals one made from scratch.

### 4. An open panel flipped sides at one x while its tag was dragged

Where: `src/note_page.rs` `tag_panel`, `panel_fit` (the polish branch's placement: a tag
opens outward, hangs over the sheet's edge, narrows to `PANEL_FLOOR`, and only a dragged
tag opens back across the words, when even the narrowest does not fit outward).

What was wrong: the side was a pure function of the room at the last paint, with one
threshold (`out < PANEL_FLOOR`, with room back), so a tag dragged back and forth across
that x flipped its open panel from one side to the other every frame; in the drag trace
before the hysteresis the old placement did the same at `room < panel width`.

What changed: `panel_fit_held`, used only for the tag being dragged, with the side its
panel opened on last frame (`panel_sides`, cleared when the note changes): the panel opens
back across the words only once the room outward is `PANEL_HYSTERESIS` (24 px) under the
floor, and outward again only once it is 24 px over it; in between it keeps its side at
the width that side allows. Undragged tags keep the plain `panel_fit`, so the never-cover
rule is as it was (`no_tag_opens_across_the_words`, scenario `u13-tags-clear-the-words`).

Evidence: unit test `a_held_tag_keeps_its_side_through_the_band_round_the_floor` walks
the room down and up through the band from both sides; `u13` and `u17` pass.

### 5. Every thread re-solved from scratch whenever its ends moved

Where: `src/stitch.rs` `Rope::set_ends`, `settle`.

What was wrong: when a thread's ends move without animating (text above it reflowed, the
page resized), the rope is laid out afresh and settled for a fixed 480 simulation steps
(16 constraint passes over 24 points each): about 3 ms a rope in the release build. Each
keystroke typed above threads moved the ends of every thread below it, so on a note with
20 threads a keystroke cost ~60 ms of rope settling, a frame or more; in the frame log of
a release drag on the long note the `ropes` span was 26–66 ms a frame on every jiggle
tick. `Rope::new` settled 360 steps the same way.

What changed: both ends moving by the same shift (which is what a reflow above does) just
shifts the points, no solving; otherwise `settle` stops once no point has moved more than
`QUIET` (0.02 px) for `QUIET_STEPS` (12) steps running, with `SETTLE_MAX` (480) as the
ceiling.

Evidence: tests `settling_stops_once_the_thread_is_still_and_lands_where_the_long_run_would`
(fewer than half the steps; every point within 0.5 px of the 480-step rest) and
`a_thread_whose_ends_move_together_keeps_its_shape`; the sag, tie-off, cut and no-zigzag
tests still pass. Measurements: see the table below.

### 6. Thirteen compiler warnings, twelve of them dead code

Where: `src/ui.rs`, `src/editor.rs`, `src/margin.rs` (unused `prelude` imports);
`margin::tether`, `Rope::nudge`, `Rope::is_cut`, `NoteEditor::set_anchors`,
`NoteEditor::set_line_kind`, `Store::note_mut`, `Palette::bulb`, `Menu::Thread`,
`NotePage::thread_menu_for` (never used); `Playback::animating`, `Rope::drop_depth` (used
by tests only); `simple_enum!`'s `ALL` (unused for some choices); `welcome::welcome_note`
(used by tests only).

What changed: the unused items are gone; the test-only ones are `#[cfg(test)]`; every
choice's `ALL` is now exercised by `every_choice_has_its_own_label` (labels present and
distinct, seven thread colours), which is a real check. `welcome_note` stays: `src/welcome.rs`
is the film agent's; it is used by the export and drawing tests, so it wants
`#[cfg(test)]` when that file is free.

### 7. Small things in the brain client (`src/brain.rs`)

- `Pending::deliver` took a callback out of the map and put it back for every streamed
  delta (a String key allocation a delta); deltas now reach the callback in place and
  only the final Done or Error removes it. The invariant tests still hold.
- `Brain::run` cloned the whole model list on every request to pick the effort; it
  borrows it.
- `shutdown` left `state` at `Ready` with no process; the invariant is now stated and
  kept (`Failed` whenever there is no process).

### 8. State left behind when the page switched notes, and four smaller bugs

Where: `src/note_page.rs` `open`, `read_line`, `send_task`, `flush`, `note_rechecks`;
`src/store.rs` `save_json`.

- `open` reset the editor, the threads' replies and the menus, and left two dozen fields
  of the old note in place: looks-again due at the next pause (`rechecks`), the needle on
  a thread being looked at (`rechecking`), questions waiting for a pause (`arrived`), the
  a drag in progress, the pointer's rest on tags, the springs, the ropes, the measured tag
  heights, the room a script had seated. It clears them. (Not the last dismissal: master's
  `undo_dismiss` already skips a thread the note does not have and re-finds the words, and
  Threads between notes puts a cut thread back from either end by design, `t32`.) No
  scenario caught the rest because switching notes mid-drag or mid-recheck is rare.
- A read's callback anchored its question at the line the paragraph had when the read was
  sent. `note_edit` moved `reading.line` as lines were added or removed meanwhile, but the
  callback used the captured value, so a quote that appears twice could land in the wrong
  paragraph after a line was inserted above during the read. It uses the moved line.
- A request's timeout cancelled it only while the page still tracked it. Learning about
  the writer (`maybe_learn`) is tracked by nothing, so a hung learn request stayed in the
  brain's `running` map, with its callback in `Pending`, for the life of the process. The
  brain is now told either way; the page hears about it only for a request it shows.
- `flush` cloned the whole note twice (once to save, once into the store); once.
  `note_rechecks` copied the whole text on every keystroke (164 KB on the long note); it
  borrows it.
- A note that could not be written (`save_json`) was logged and nothing else: the writer
  went on typing into a note that was not reaching the disk. `Store::try_save_note` says so,
  and `flush` puts it in the toasts ("Couldn't save this note: …"); the other saves keep
  logging, as before.

Evidence: 218 unit tests pass; `t05-switch`, `r02-switch`, `t03-chat`, `u17` pass; the
Undo case by hand. These are behaviour changes only where the old behaviour was wrong.

### 9. The tag's hover, rest and spring: five maps that had to agree

Where: `src/note_page.rs` `set_tag_hover`, `tag_pointed`, `tag_rested`, `unfold_target`,
`escape`, the `hover:` script step, the spring loop in `render`.

What was wrong: where the pointer was on a tag lived in a set of `"id:part"` keys
(`hover_tag`), a map of when it came onto each (`hover_at`), a map of when it left each
(`hover_left`), and the drop spring in two more maps (`unfold`, `unfold_v`); seven places
read some of them and kept the rest consistent by hand, each with its own `Instant`
arithmetic. A tag's state was the conjunction of five lookups.

What changed: each visible tag has one `Tag`: a `Hover` (`Away`; `Over` its tab, its
panel or both since a moment; `Left` an open tag at a moment) with the rules on it
(`rested`, `lingering`, `settling`, and `part` for the pointer coming onto or leaving a
part) and the spring's position and speed with its `step`. Coming back to a tag within the
grace period is the same rest, which is what the lingering made it before. `open` clears
one map.

Evidence: `a_tag_opens_after_the_pointer_rests_and_lingers_after_it_leaves` (headless,
explicit instants: the rest, a key during it, moving from the tab onto the panel, leaving
an open tag and coming back, leaving before it opened) and
`the_drop_spring_overshoots_a_little_and_the_fold_does_not_bounce`; `u06` and `u17` pass.

### 10. The chat's answer: two Options that had to agree

Where: `src/note_page.rs` `chat_pending`, `chat_error` and their seven writers.

What changed: `ChatReply` is `None`, `Streaming { req, partial }` or `Failed(why)`, so an
answer on its way and a reason none came can no longer show at once; stopping waiting
(`take_request`) names the request to cancel; `settled`, `tracks`, the dump and the panel
read one value. The dump keeps the names `chat_pending` and `chat_error`.

Evidence: `the_chat_waits_for_one_answer_or_says_why_none_came_never_both`; `t03-chat`.

## Follow-ups: the structural splits, with their seams

Not done in this pass, at the coordinator's request (seven feature branches are in these
files); listed here with the seam each would cut along, for a pass of their own.

- `src/note_page.rs` (now ~5,100 lines) splits along what the file's own banners already
  mark, each piece a module with one job:
  - `reading.rs`: the reading loop (`judge`, `calls_by_name`, `locate_thought`,
    `note_edit`, `on_pause`, `consider`, `after_read`, `read_line`, `answer_call`,
    `recheck`). Pure functions already; the loop's own state (`last_edited`, `waiting`,
    `lines_seen`, `reading`, `pause_task`, `rechecks`, `rechecking`) wants a `ReadLoop`
    struct with `edited(line, count)` and `take_due()` so the line-shifting in `note_edit`
    gets the headless test it lacks.
  - `requests.rs`: `send_task`, `run`, `run_any_note`, `tracks`, `cancel_request`,
    `later`, `report`, `describe_failure`, and the request callbacks' shared ending (six
    `BrainMsg::Error` arms do the same four things; two now also call `fail_thread`, so the
    helper waits for the retry feature to settle).
  - `tags.rs`: `Tag`, `Hover`, `TagDrag`, `panel_fit`, `panel_fit_held`, `card`,
    `tag_panel`, `set_tag_hover`, `unfold_target`, the drag.
  - `margin_view.rs`: `margin_layout`, `thread_ends`, the ropes and the canvases.
  - `chrome.rs` for the page: `header`, `brief_line`, `menu_card`, the menu keyboard,
    `toolbar`, `sewn_keys`, `ask_box`, `chat_panel`, `enlarged_drawing`.
  - `render` itself (~400 lines) then reads as the frame's order: sync the editor's marks,
    step the tags, lay out the margin, build the canvases, the sheet.
- `tag_panel` (~290 lines): the header row, the body (messages, drawing, pending states),
  and the action row are three functions.
- `toolbar` (~140 lines): the three key kinds are closures that could be one `fn key_row`.
- `src/editor.rs` (~5,100 lines): `State` and its text helpers (the headless core) from
  `NoteEditor` and `EditorElement` (the GPUI half); the tests already split that way.
- `pending: HashMap<String, (String, Pending)>` wants `struct InFlight { req, stage }`; a
  rename across a dozen sites, left for the split.

## Measurements

Before = master at the start of the pass (`be9a38a`: master plus the frame log, nothing
else); after = this branch at its head. "Long note" is the 600-paragraph scratch note with
20 open threads; "small note" is the `running-plan` fixture. Release rows ran under the
jiggle (8–9 forced frames a second: the window was covered for most of the night, and a
covered window draws nothing), which on the baseline binary wobbles the width and so
re-lays the note out and re-hangs every thread on each tick, and on this branch wobbles
the height; the baseline's release numbers therefore include what a tool run costs on it,
and its idle frames are not comparable one for one, only the work each does. Debug rows
ran without the jiggle while the window was visible.

| What | Build | Before | After |
|---|---|---|---|
| Idle with the long note open: CPU, frames, editor layouts | debug, no jiggle | 3.9%; 2 frames/s (the caret blink); 2 full layouts a frame, 0.4–0.8 ms each warm | 0.9%; 2 frames/s; 0 layouts |
| Idle, long note, 9 frames/s forced | release | 94.9% CPU; 18 layouts/s at ~4.7 ms (85 ms/s) and every thread re-hung each tick | 7.5% CPU; 0 layouts; page render 0.5 ms a frame (ropes 0.02), editor paint 0.35 ms |
| Typing 40 words at the end of the long note, 4 a second | debug, no jiggle | 17.3% CPU; 2 warm layouts a keystroke, or 1 cold of 20–40 ms after any pause in frames | 2.8% CPU; 1 layout a keystroke shaping 1 line in 0.2–0.4 ms |
| The same, 9 frames/s forced | release | 94.1% CPU; 18 layouts/s at ~4.6 ms | 7.6% CPU; 4–5 layouts/s, one line each, 0.2–0.3 ms |
| Switching to the long note: `switch:` to the first laid-out frame | release | 79 ms (one cold layout of 53 ms) | 58 ms (one cold layout of 53 ms; the first layout of a note is the one cost kept) |
| A scripted 36-step pointer drag of a tag, small note: moves that reached the handler | debug | 1 of 36; the tag moved once, by 42 px | 36 of 36; the tag at the pointer after every step |
| The same drag, what a frame spends | release | — (the handler saw one move) | page render 0.2 ms a frame (ropes 0.07), editor paint 0.25 ms; 0 layouts |
| One-shot drags of a tag on the long note (`drag:` step) | release | 18 layouts/s at 1.6–2.4 ms (29–43 ms/s), CPU 59% | 0 layouts; page render 0.2 ms a frame at rest and 1.2 ms on the frame a drag lands (the one re-hung thread, 1.0 ms) |
| The ropes' share of a frame when the ends of 20 threads move (jiggle on, width) | release, before the rope fix | 26–66 ms a frame (595–613 ms/s) | — |
| The same, after the rope fix | debug | long note 63 ms a frame | long note 0.2–3.3 ms a second (one 63 ms frame when the 20 threads are first hung); small note 100–118 → 0.03–2.2 ms a second |
| Hottest user-space frames (`sample`), idle and typing | debug | `memchr` under `line_range_in`: the gutter scanning the whole note once per line | GPUI's own `paint_line` and `BoundsTree::insert` |
| The brain: start to `ready` (no model call) | — | 50 ms | 50 ms; nothing to fix |
| Compiler warnings (`cargo build`) | — | 13 | 0 of them (the last, `Lettering::ALL`, by Settings listing the lettering choices from it, as the other rows do); on the rebased head 2 remain, both from master's new features (`answers::kind_wire`, `Note::elsewhere_of_mut`) |
| Tests | — | 189 Rust, 101 brain | 218 Rust, 136 brain (master added some; this branch adds 9 Rust tests and scenario `u17`) |

## Left, and why

- Two scenarios fail on master itself, not from this branch: `s01-shortcuts` records ⌥⌘→
  as New note, which Threads between notes now binds to Follow (the recording clashes, no
  note is made); `u10-keyboard`'s Tab order gains the buttons of whichever tag the owner's
  parked pointer has opened (its first dump shows `t2` open before any step). Both for their
  authors.

- `u06-drop-tags`'s first check (`just-pointed`: not yet unfolded 250 ms after `hover: 1`)
  failed whenever the window drew at full rate, on the baseline binary too: the rest is
  120 ms and the drop spring passes half open about 75 ms later, so the state the line
  wanted could only be caught when frames were throttled (a covered window drawing on the
  jiggle alone). Fixed by running the scenario's thread animations eight times slower
  (`INKLING_SLOWMO=8`; the rest, on the wall clock, is not slowed) and by checking the tag
  it points at rather than "nothing is open". That second part names a hazard for every
  scenario that asserts the exact set of open tags: the owner's own pointer, resting over a
  tab's spot on the covered test window, opens that tag (GPUI tracks the real mouse; seen
  on the baseline binary as well). `u17` checks its own tag the same way; `u06`'s later
  lines (`rested`, `moved-on`, `typed`, `escaped`) still assert the exact set and would
  fail with the pointer parked there, so they are left for a quiet pointer.
- Two warnings on the rebased head, both from master's new features (`answers::kind_wire`,
  `Note::elsewhere_of_mut`), in code the feature agents are mid-way through; theirs to
  settle.
- The six `BrainMsg::Error` arms that do the same four things (take the request out of
  `pending`, drop an empty thread, report unless cancelled, notify): two of them now also
  call the retry feature's `fail_thread`, which is still moving, so the helper waits.
- The structural splits (above): not in this pass, by request.
- `brain/src/claude.ts` reads the SDK's messages through `as any` casts (`drain`,
  `accountStatus`); the SDK's own types (`SDKResultMessage`, `SDKRateLimitEvent`, …) would
  type them, but the fields it reads (`total_cost_usd`, `rate_limit_info`, `structured_output`)
  are not all on the public types in this SDK version, so the casts stay with a note.
  The brain's error handling, request parsing and cancel paths were read and found sound
  (one `done` or `error` per request, cancel never `done`, junk lines ignored), and its
  tests cover them (136, one with no `expect`: "junk lines are ignored and later requests
  still work", which passes by not timing out, which is what it checks).
- The jiggle (`INKLING_JIGGLE`) now wobbles the window's height instead of its width (a
  width change laid the whole note out again every tick and re-placed every tag, so every
  number measured under it carried that cost, and the dumps' viewport width was off by
  one). The price: anything measured against the window's bottom edge moves by a pixel
  between frames, so `onPaper` and `u08-toasts`' bottom check allow that pixel.
- The reading loop's own state (`last_edited`, `waiting`, `lines_seen`, `reading`) stays
  in the page; its line-shifting in `note_edit` has no headless test. Listed under the
  splits as `ReadLoop`.
- `pending: HashMap<String, (String, Pending)>`: the tuple wants a named struct; a rename
  across a dozen sites, left for the split.
- The weave on the cloth is painted as ~730 quads a frame; it is the design, and cheap on
  the GPU; it draws only when something else does.

# Let it stand, and Turn it over

Design for items 2 and 8 of `design/manifesto.md` section 2, written 7 Nov 2026 against
master `fc06104`. Two agents build it: one builds Let it stand, the other Turn it over. Both
start from the commit that adds the shared data (section 3), so their branches merge cleanly.

**In short.** On a mind's question or correction, the tag's Dismiss becomes **Let stand**.
Whatever you typed in the tag's box goes with it as your one-line reason. The thread leaves the
page as a dismissed thread does today, but from then on every mind that reads the note is told
the point, who raised it and your reason, and is asked not to raise it again in any words.
**Turn over** shows the back of the note: every thread it has had, oldest first, with how each
one ended (tied off, let stand, snipped or still open), the words it was about before and
after, and your reasons, which you can add or change there. Nothing new is stored except three
optional fields on a thread.

## 1. What the writer sees and does

The words below are the exact words on screen. Controls use plain words (DESIGN.md rule 3);
the craft words (tied off, snipped, the back) appear in status lines and tooltips.

### 1.1 Let it stand

1. A mind asks something you have already weighed. Skeptic, on "four days a week": *You walked
   after 7 km last year. Is pace the problem, or the last 3 km?*
2. You open its tag (click it, or ⌥⌘↓). The buttons under the box read **Resolve**, **Let
   stand**, **Make a note**, and the copy icon. On a tag that is not a mind's point (your own
   question, a sketch, an answer when you called a mind by name) the second button still reads
   **Dismiss**, because there is no point to let stand. A tag that is already tied off keeps
   **Dismiss** too.
3. You can give a reason in the box you would reply in: *Pace is the problem; the walk was
   pace.* Pointing at Let stand shows its tooltip: *Keep your words as they are. No mind will
   raise this again, and what you typed above goes with it as your reason.*
4. You click **Let stand** (or press ⇧⌘⌫). Pressing Return instead would send the text as a
   reply, and the mind would answer as it does today. Let stand gets no answer: the decision
   is yours.
5. The thread is cut as a dismissed thread is today: the scissors close, the tail falls, and
   the stitches come out. The toast reads **Let stand, with your reason. No mind will raise it
   again.** With an empty box it reads **Let stand. No mind will raise it again.** Both
   offer **Undo** with ⌥⌘Z for 8 seconds, as now.
6. From the next request on, every mind that reads this note (while reading along, in Find a
   loose thread, Ask, Ask the room, a call by name, a reply, chat or a sketch) is told the
   point. Edit the paragraph and it is read again, but the same question does not come back,
   in its old words or in new ones.
7. If you select those words and press ⌘↵ (Find a loose thread), Inkling looks for a different
   point. If the only point it finds is the one you let stand, no thread opens and the toast
   reads **Nothing new to ask here: you let its point stand.**

### 1.2 Turn it over

1. The header row above the title, on the right, gains **Turn over** beside "N loose threads"
   and "Hide threads". It shows once the note has any thread, of any kind and however it ended.
   ⇧⌘B and View › **Turn Over** do the same.
2. The sheet turns: the page's contents fade out over 140 ms while a soft fold shadow sweeps
   across the sheet from its right edge to its left (280 ms, ease in-out), and the back fades
   in over the last 140 ms. With reduced motion it is a 120 ms crossfade. Nothing moves while
   you type, because the note is not on screen on the back.
3. The back is the same sheet on the same linen, with the sampler border along its top. Its
   header row reads, on the left, **Every thread, oldest first** (faint), and on the right
   **Turn back**. Below it is the note's title in the display face, in graphite rather than
   ink, because this is the record of the note and not the note.
4. Below the title is one entry per thread, in the order the threads were opened. Each entry
   lines up with the page's text column; the gutter to its left holds its ending mark.
   - A top row: the mind's name in its thread colour, then the kind and the date it was opened,
     faint: **Skeptic · question · 5 Nov** (kinds as the tags write them: question,
     correction, you asked, sketch, answer; the year appears when it is not this year).
   - The words it was about, in your own typeface. If the words changed while the thread was
     open (you fixed them, or rewrote them later), the old words come first, struck through and
     faint, and the words as they are now come under them. That is "the words before and after".
   - What was said: the mind's first message, then the last one if there were more. Messages in
     between fold into **2 more**, which unfolds them. A thread with a sketch says **With a
     sketch**.
   - The ending line, set like a status line: **Tied off · 6 Nov**, **Let stand · 6 Nov**,
     **Snipped · 6 Nov** or **Still open**. Threads that ended before this feature have no date.
   - For a point let stand, your reason in your typeface and in quotation marks, or **No reason
     given.** in faint type.
   - Quiet actions in small type, as the tags have: **Show on the page** (open and tied-off
     threads), **Add a reason** or **Change the reason** (let stand), **Reopen** (let stand and
     snipped, while the words are still in the note). When the words are gone the entry says
     **Its words are no longer in the note.** instead of offering Reopen.
5. **Add a reason** turns the reason line into a one-line box, *Why your words stand…*;
   Return keeps it, Esc leaves it as it was. The minds get the new reason from the next request.
6. **Reopen** turns the sheet back to the front and opens the thread on its words, as an open
   thread again. The reason and the date go, and every mind may raise the point again.
7. **Show on the page** turns the sheet back and opens the thread's tag at its words, as
   ⌥⌘↓ does.
8. **Turn back**, ⇧⌘B or Esc returns to the front, with the caret where you left it. ⇧⌘C on
   the back copies the back as Markdown; the toast reads **Copied the back as Markdown**.
9. A note with no threads has no Turn over control. If you turn it over with ⇧⌘B, the back
   reads: *Nothing here yet. Every thread a mind opens on this note will be listed here, however
   it ends.*

### 1.3 How each ending looks on the back

Each mark sits in the gutter, where the front draws its read marks. Each is drawn in the
mind's thread colour, and the four differ in shape, so colour is never the only signal
(rule 4).

| Ending | When | Gutter mark | Ending line | Below it |
|---|---|---|---|---|
| Tied off | `status` Resolved | a French knot (`stitch::french_knot`), as on the folded tab | **Tied off · 6 Nov** | the mind's last message, which says why when the words were fixed |
| Let stand | `status` Dismissed on a question or correction | a tacking stitch (`stitch::tacking_stitch`): the words held | **Let stand · 6 Nov** | the reason, or **No reason given.** |
| Snipped | `status` Dismissed on anything else | a short loose tail, faint | **Snipped · 6 Nov** | nothing |
| Still open | `status` Open | the tab's own **?** or **×** | **Still open** | the reply count, as the tag shows it |

## 2. Every state

| State | Let it stand | Turn it over |
|---|---|---|
| Linen and Lamplight | No new colours: the button is a ghost button and the toast is the stack's own. | The back uses the page's own paper, ink and thread colours, so every pair keeps its measured contrast. Shots in both appearances. |
| Right margin, both margins | Same tag, same buttons. | Unaffected: the back is a list in the column, with no margin tags. |
| Marks only | The ? or × after the words opens the same card, with the same buttons and box. | Unaffected. |
| Hidden threads (⇧⌘.) | Nothing of Inkling's shows, so there is nothing to let stand. Points already let stand are still told to the minds, because hiding changes the view, not the decisions. | The Turn over control hides with the other controls. ⇧⌘B and the menu still turn the note over, because you asked to see the back. Hiding threads while on the back turns the note back to the front first. |
| A note that waits to be asked | Threads come from Find a loose thread, Ask and calls by name. Find a loose thread looks past points let stand (1.1, step 7). | The same back. |
| Export and copy | The front's export and Copy as Markdown are unchanged: open and tied-off threads go under their paragraphs, and threads that were let stand stay out, as dismissed threads do today. | ⇧⌘C on the back copies the back (section 7 gives the format). Export still writes the front. |
| Undo | Undo or ⌥⌘Z within 8 s puts the thread back open on its words. It finds the words again rather than trusting the range saved when the thread was cut, which may have moved if you typed in the meantime. The reason and date are cleared, and the text stays in the tag's box as you left it. | Reopen on the back is Undo without the clock. Leaving the back has no undo, because turning over changes nothing. |
| Several minds | The let-stand list is for the note, so every mind gets it, Ask the room included, and each entry names the mind that raised it. A point Skeptic raised and you let stand is not raised by Editor either. | Each entry names its mind in that mind's thread colour. |
| Dozens of threads | The minds get at most 12 points: the ones whose words are still in the note first, then the most recently let stand, each cut to a set length (section 4). | Entries are compact (two messages, the rest folded) and scroll with the page. There are no counts and no summary, because belief 7 rules out any count of what Inkling caught. |
| Switching notes | Nothing pending. | Opening another note shows its front. |
| Old notes | Questions and corrections dismissed before this feature are let stand with no reason and no date, and the minds are told them from the first read after the update. Belief 7 counts cutting a thread as a decision. | They appear with their endings and without dates. |

## 3. The data model (shared by both features)

This is the only data either feature stores. It goes in one commit, made before either
feature is built, which both branches start from.

### 3.1 New fields on `store::Thread`, at the end of the struct

```rust
    /// Why you let it stand, in your words: one line, at most REASON_MAX characters, from
    /// the tag's box when you chose Let stand, or written later on the back of the note.
    /// Only a mind's point that was let stand has one; it goes when the thread opens again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When it was tied off or dismissed; cleared when it opens again. Threads that ended
    /// before this field existed have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// The words the thread was opened on, kept the first time its words change (you edit
    /// them; Fix the words reads them again). `quote` is always the words now. None while
    /// the words never changed, and for threads whose words changed before this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_quote: Option<String>,
```

`skip_serializing_if` keeps old notes' files byte-for-byte the same shape until something new
happens to a thread. Older builds ignore the unknown fields, so a note saved by this build
still opens in an older one.

**No new `ThreadStatus` variant.** A build that does not know a status value drops the whole
note when it loads it (`broken_note_files_are_skipped_and_left_alone`, the `bad-enum` case).
Let stand is therefore stored as `Dismissed`, and the ending is worked out from the status
and the kind, never stored.

### 3.2 Helpers in `store.rs`

```rust
/// The most characters a reason keeps.
pub const REASON_MAX: usize = 200;

/// How a thread ended, as the back of the note and the minds see it. Worked out, never
/// stored, so it can grow without breaking older builds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending { Open, TiedOff, LetStand, Snipped }

impl Ending {
    /// The ending as the back of the note writes it.
    pub fn label(self) -> &'static str {
        match self { Ending::Open => "Still open", Ending::TiedOff => "Tied off", Ending::LetStand => "Let stand", Ending::Snipped => "Snipped" }
    }
}

impl Thread {
    /// A point a mind raised on its own: a question or a correction (reading along, Find a
    /// loose thread) that the mind has said. Only these are let stand; dismissing anything
    /// else (a loose thread still being looked for included) snips it.
    pub fn is_point(&self) -> bool;
    /// Open → Open; Resolved → TiedOff; Dismissed → LetStand on a point, else Snipped.
    pub fn ending(&self) -> Ending;
    /// Change the status: tied off or dismissed stamps `ended_at` (now); open again clears
    /// `ended_at` and `reason`. Setting the status it already has changes nothing.
    pub fn set_status(&mut self, status: ThreadStatus);
    /// The words the thread is on now. The first time they change, the old words are kept in
    /// `first_quote`.
    pub fn set_quote(&mut self, words: &str);
    /// The words it was opened on: `first_quote`, else `quote`.
    pub fn words_before(&self) -> &str;
    /// Your reason as one line: whitespace and line breaks fold to single spaces, cut to
    /// REASON_MAX characters at a char boundary; empty clears it.
    pub fn set_reason(&mut self, text: &str);
}
```

### 3.3 Where the shared data is written (`note_page.rs`, one line each)

| Where | Today | Becomes |
|---|---|---|
| `sync_from_editor` | `t.quote = text[a.range.clone()].to_string()` | `t.set_quote(&text[a.range.clone()])` |
| `recheck`, on `Done` | `t.quote = now_text.clone()` | `t.set_quote(&now_text)` |
| `set_status` | `t.status = status` | `t.set_status(status)` |
| `send_reply`, resolved | `t.status = ThreadStatus::Resolved` | `t.set_status(ThreadStatus::Resolved)` |
| `undo_dismiss` | `t.status = ThreadStatus::Open` | `t.set_status(ThreadStatus::Open)` |

The `dump:` thread entries gain `"ending"` (`"Open"`, `"TiedOff"`, `"LetStand"`, `"Snipped"`),
`"reason"` and `"before"` (`first_quote`), so scenarios can check what both features
read. The four `Thread { … }` literals (`note_page.rs` `open_thread`, the two in
`welcome.rs`, the test helper in `export.rs`) gain the three fields as `None`.

How Turn it over reads them: the ending from `ending()`, its date from `ended_at`, the
opening date from `created_at`, the reason from `reason`, the words before from
`words_before()` (shown struck through only when `first_quote` is set and differs from
`quote`), and the words after from `quote`.

## 4. The brain

### 4.1 What every mind is told

`Context` gains one list, built by the app from the note's threads:

```ts
// brain/src/protocol.ts
export interface LetStandPoint {
  quote: string;                    // the words, as they were when let stand
  point: string;                    // the mind's first message: what it asked or corrected
  kind: "question" | "correction";
  mind: string;                     // the mind that raised it
  reason: string | null;            // your one line, if you gave one
}
export interface Context { /* … */ letStand: LetStandPoint[]; }
```

The app builds it in a new pure module, `src/stand.rs`, with `stand::points(threads, text)`.
It takes every thread whose `ending()` is LetStand and has a first message from its mind. It
puts the points whose words are still in the note's text first, then the rest, and sorts each
group newest first by `ended_at`, falling back to `created_at`. It keeps at most 12 points and
cuts each field at a char boundary, adding an ellipsis: the quote at 160 characters, the
point at 240 and the reason at 200. `NotePage::context_with` adds `"letStand"` beside
`"threads"`, so every task with a `ctx` carries it. The brain's `context()` reads it
tolerantly: it drops entries without a quote or point, keeps at most 20, and cuts each field
again.

**Why the prompt does not bloat.** The list holds one line per point, never the whole
conversation (the margin block already sends every message of every thread that is not
dismissed). It is capped at 12 points, about 1,000 tokens at the most, and is empty in most
notes. It goes in the system prompt, the stable part of each request, because it is a
standing decision about the note like the brief, and it changes only when you let something
stand.

### 4.2 The words in the prompt

`systemPrompt(ctx)` adds this block after the brief and About you, only when the list is not
empty:

```
Points the writer let stand. They weighed each one and decided their words stay as they are.
Never raise any of them again, in any words or about any other part of the note, unless the
writer asks about it. Your point is the same one when the writer's reason, or their decision
to keep the words, answers it too.
1. On «four days a week», Skeptic's question: You walked after 7 km last year. Is pace the
problem, or the last 3 km? Their reason: Pace is the problem; the walk was pace.
2. On «always», Editor's question: Always? What is the evidence? (No reason given.)
```

The clause "unless the writer asks about it" covers the tasks where you ask (Ask, chat, a
call by name, a reply), so a mind can still answer about a point when you bring it up.

`readPrompt` changes "Never repeat a point that an existing margin thread already makes." to
"Never repeat a point that an existing margin thread already makes, or one the writer let
stand." `probePrompt` adds: "If the most useful point is one the writer let stand, find the
next most useful one."

### 4.3 The same point in new words

Rephrasing is the failure that matters, and word overlap cannot detect it, so the brain
uses no word-matching filter. It relies on two things.

1. **A test, not just a ban.** The instruction says what "the same point" means: one that your
   reason, or your decision to keep the words, would also answer. A model can apply that test
   to a rephrasing; it cannot apply "don't repeat" to words it has not seen.
2. **The model checks its own note.** `THOUGHT_SCHEMA` and `PROBE_SCHEMA` gain one field,
   last in `required` and in `properties` so it is written after the note: `repeats`
   (integer). The instruction reads: "repeats: after writing your note, compare it with the
   points the writer let stand: the number of the one it makes again, in any words; 0 when it
   makes none of them." The brain (`main.ts`, read and probe) turns a note whose `repeats` is
   between 1 and the length of the list into `{speak: false, repeats: n}` and logs it. The
   app treats a read like that as a silent read. For a probe, it closes the empty thread and
   shows the toast from 1.1, step 7. A `repeats` outside that range counts as 0.

The fake brain mirrors this rule: when the quote it would use is exactly one of the
`ctx.letStand` quotes, a read answers `{speak: false, repeats: n}`, and a probe the same,
unless the request carries `[[again]]`. The real rule is checked against the real model with a
handful of calls through `brain/test/live.ts` (a new scenario, `letstand`): the same
paragraph read again, the same point asked in new words, a different point on the same words
(which must still be raised), and a probe on the words.

**Checked on 7 Nov 2026** (`bun run test/live.ts <dir> letstand`, Opus 5.5, reads at low
effort, the probe at medium; four calls), against the running plan with two points let stand:
the long run's start (with the reason "I start at 8 km and the race is ten weeks out") and
Skeptic's "Why four?" (no reason).

| Call | What came back |
|---|---|
| The long-run paragraph read again | silent |
| The same point in new words ("By race week the Sunday run is 16 km, one more kilometre each week.") | silent |
| A new point on words with one let stand ("Four days a week, 10 km each, is 50 km a week.") | a correction: "Four runs of 10 km is 40 km, not 50. Also, the Sunday long run goes from 8 km up to 16…", which uses the writer's reason as a fact |
| Find a loose thread on "add a kilometre every week until I reach 16" | a different point: how long the other three runs are |

None of the four needed the `repeats` hold: the instruction alone kept the minds off the
points, and the reason was used as a fact. The hold remains as the guard for a model that
writes a point anyway.

## 5. Fit with the features designed in parallel

The shared types this design changes, exactly: `store::Thread` (three fields at the end),
`store::Ending` (new), `impl Thread` (six helpers), `protocol.ts` `Context` (`letStand`) and
`LetStandPoint` (new), `prompts.ts` `systemPrompt`, `readPrompt`, `probePrompt`,
`THOUGHT_SCHEMA` and `PROBE_SCHEMA` (`repeats`, last), and the read and probe `done` data
(optional `repeats`). Neither `ThreadStatus`, `ThoughtKind`, `Note` nor `Settings` changes.

- **Answer in the note, Write it in.** When another paragraph ties a thread off, set the status
  with `Thread::set_status(ThreadStatus::Resolved)` so `ended_at` is stamped. If you add a
  field such as the answering paragraph, put it after `first_quote`; the back's Tied off line
  can then say where it was answered. Your knot changes nothing here.
- **Threads of your own.** Your own thread is not a mind's point, so `is_point()` is false.
  It shows **Dismiss**, ends as **Snipped**, and is never told to the minds. A new
  `ThoughtKind` variant would make older builds drop the note, just as a new status would;
  read the kind with a tolerant deserializer, as `Block.kind` does with `kind_or_paragraph`.
- **Show its working.** A correction's check is part of its first message or a field of
  yours; `stand::points` sends only the first message. Add your schema field before `repeats`,
  or after it if it should be written after the check: the order is the one the model writes
  in.
- **Loose ends.** Count `ending() == Ending::Open`; a point let stand is never a loose end. The
  back's entry view (`back::entry`) is written to take a thread and its colour, so the list can
  reuse it.
- **Threads between notes.** A point let stand on a thread that links two notes should reach
  the minds of both notes. `stand::points` takes a slice of threads, so that feature can pass
  the cross-note threads too. It is out of scope here.

## 6. Tests

### Unit

`store.rs`
- `old_threads_load_without_the_new_fields`: a thread written before these fields loads with
  each as `None`; one saved again without them gains no `null`s.
- `endings_follow_status_and_kind`: Dismissed on a question or correction is LetStand;
  Dismissed on an ask, sketch or answer is Snipped; Resolved is TiedOff; Open is Open.
- `set_status_stamps_and_clears`: tying off and dismissing stamp `ended_at`; opening again
  clears it and the reason; setting the same status keeps the stamp.
- `set_quote_keeps_the_first_words`: the first change keeps the old words; later changes do
  not overwrite them; the same words change nothing.
- `set_reason_is_one_line`: line breaks and runs of spaces fold, the length is capped at a
  char boundary with multi-byte text, and an empty or blank reason clears it.

`stand.rs`
- `only_points_let_stand_are_told`: open, tied-off and snipped threads, and points with no
  message from their mind, are left out.
- `points_in_the_note_come_first_then_newest`
- `at_most_twelve_and_each_cut_short`: the cap, and cuts at char boundaries with an ellipsis.
- `old_dismissed_points_are_told_without_a_reason`
- `dismissing_a_point_lets_it_stand_with_what_you_typed`: the reason and toast for a mind's
  open point. An asked thread and a tied-off one are dismissed as before, with no reason.

`brain/test/unit.test.ts`
- `letStandBlock` numbers the points, writes "Their reason:" or "(No reason given.)"; the
  system prompt carries the block before the date and leaves it out when the list is empty.
- The read and probe prompts carry their lines; both schemas list `repeats` last, as an
  integer.

`brain/test/protocol.test.ts` (the real `main.ts`, fake SDK)
- `context()` reads `letStand`: each field folded to one line, bad entries dropped, at most
  20, long fields cut; a missing list is none.
- A read whose structured answer has `repeats: 1` with one point comes back
  `{speak: false, repeats: 1}`; `repeats: 3` with one point, or any `repeats` with no points,
  is ignored and the read speaks.
- A probe with `repeats: 1` comes back `{speak: false, repeats: 1}`.

### End to end (fake brain, `scripts/dev e2e`)

`brain/test/e2e/scenarios/u12-let-stand.txt`, from the `running-plan` fixture, where `t2` is
the question on the long run:

```text
# Let it stand: a mind's point let stand with the words in its box as the reason; Undo puts it
# back on its words; every later read and probe carries the point, and the same point does not
# come back, though a new one on the same words still lands (design/features/let-it-stand.md).
# fixture: running-plan
wait: 1500
stand: 2 I start at 8 km, and the race is ten weeks out.
dump: stood
expect: d.threads.find(t => t.id === "t2").status === "Dismissed" && d.threads.find(t => t.id === "t2").ending === "LetStand"
expect: d.threads.find(t => t.id === "t2").reason === "I start at 8 km, and the race is ten weeks out."
expect: d.toasts.some(t => t.kind === "thread_dismissed" && t.live && t.text === "Let stand, with your reason. No mind will raise it again.")
expect: d.undo === "t2" && !d.anchors.some(a => a.id === "t2")
undo-dismiss
dump: undone
expect: d.threads.find(t => t.id === "t2").status === "Open" && d.threads.find(t => t.id === "t2").reason === null
expect: d.anchors.some(a => a.id === "t2" && a.open)
stand: 2 I start at 8 km, and the race is ten weeks out.
caret: end
type: I add a kilometre every week until I reach 16, then hold it [[quote:add a kilometre every week until I reach 16]].
settle
dump: told
expect: requests.filter(r => r.input.task === "read").at(-1).input.ctx.letStand.length === 1
expect: requests.filter(r => r.input.task === "read").at(-1).input.ctx.letStand[0].quote === "add a kilometre every week until I reach 16"
expect: requests.filter(r => r.input.task === "read").at(-1).input.ctx.letStand[0].reason === "I start at 8 km, and the race is ten weeks out."
expect: requests.filter(r => r.input.task === "read").at(-1).input.ctx.letStand[0].mind === "Inkling"
expect: events.some(e => e.ev === "done" && e.data && e.data.repeats === 1)
expect: log.some(r => r.msg.includes("read held back"))
expect: d.threads.filter(t => t.status === "Open").length === 1   // only t1: the point did not come back
type: Sunday will add a kilometre every week until I reach 16, then hold [[quote:add a kilometre every week until I reach 16]] [[again]].
settle
dump: other
expect: d.threads.filter(t => t.status === "Open").length === 2   // a new point on the same words still lands
select: add a kilometre every week until I reach 16
probe
settle
dump: probed
expect: d.toasts.some(t => t.kind === "done" && t.text === "Nothing new to ask here: you let its point stand.")
expect: d.threads.filter(t => t.status === "Open").length === 2 && d.threads.length === 3   // the probe's empty thread went
expect: note("Sub-50 10k by March").threads.find(t => t.id === "t2").reason === "I start at 8 km, and the race is ten weeks out."
expect: typeof note("Sub-50 10k by March").threads.find(t => t.id === "t2").ended_at === "string"
expect: note("Sub-50 10k by March").threads.find(t => t.id === "t1").reason === undefined   // nothing new written on threads that did not change
quit
```

`u13-let-stand-room.txt`: Ask the room (`room: Inkling, Skeptic`, then `ask:`) sends
`letStand` to each mind. In Marks only, a question's card has **Let stand** and an asked
thread's card has **Dismiss** (the dump's `lets_stand`: true, false, or null with no tag
open). It also takes shots of the open tag in linen and in lamplight.

`u14-turn-over.txt` (Turn it over), from `running-plan`:

```text
# Turn it over: the back lists every thread in order with how it ended, the words before and
# after, and reasons that can be added; ⇧⌘C copies the back; Reopen turns back and opens the
# point on its words again.
# fixture: running-plan
wait: 1500
edit: hold 5:30 for the whole race => hold 4:55 for the whole race
settle
resolve: 1
stand: 2
turn
dump: back
expect: d.turned === true && d.back.length === 2
expect: d.back[0].ending === "TiedOff" && d.back[0].before.includes("5:30") && d.back[0].after.includes("4:55")
expect: d.back[1].ending === "LetStand" && d.back[1].reason === null
expect: d.back[0].acts.join() === "show" && d.back[1].acts.join() === "reason,reopen"
shot: back
set: appearance=dark
shot: back-dark
set: appearance=light
back-reason: 2 Ten weeks out, from 8 km.
dump: reasoned
expect: d.back[1].reason === "Ten weeks out, from 8 km."
expect: d.focus === "back:reason-t2"
key: cmd-shift-c
dump: copied
expect: d.toasts.some(t => t.text === "Copied the back as Markdown")
expect: log.some(r => r.msg.startsWith("script copy back:") && r.msg.includes("~~If I hold 5:30") && r.msg.includes("Your reason: Ten weeks out, from 8 km."))
back-reopen: 2
dump: reopened
expect: d.turned === false && d.threads.find(t => t.id === "t2").status === "Open" && d.active === "t2"
expect: d.threads.find(t => t.id === "t2").reason === null && d.anchors.some(a => a.id === "t2" && a.open)
expect: d.back.find(e => e.id === "t2").ending === "Open" && !d.back.find(e => e.id === "t2").ended
stand: 2 Not ten weeks after all.
turn
back-reopen: 2
dismiss: 2
dump: again
expect: d.threads.find(t => t.id === "t2").ending === "LetStand" && d.threads.find(t => t.id === "t2").reason === null   // reopened on an empty box
quit
```

The steps it needs: `stand: N [reason]` (Let it stand: types the reason into thread N's box,
numbered as `dismiss:` numbers them, then Let stand); `turn` (turns the note over or back);
`back-reason: N text`, `back-reopen: N` and `back-show: N`, where N is the entry's place on
the back, counted from 1. A thread whose words were edited needs a `settle` before the dump,
so `first_quote` is saved. The dump gains `turned`; `back`, a list of `{id, ending, reason,
before, after, changed, mind, kind, found, ended, acts}` (`before` is always the words it was
opened on, `changed` says whether they differ from `after`, `acts` lists the entry's controls:
`show`, `reason`, `reopen`); `back_editing`, the entry whose reason box is open; and `focus`
reads `back` or `back:reason-box` when the back or the box has the keyboard. ⇧⌘C during a
script logs the Markdown (`script copy back: "…"`) and leaves the clipboard alone, as `copy:`
does.

`u14-turn-over-edges.txt`, from `running-plan` with silent reads: on the back, keys, ⌘↩ and
⇧⌘⌫ never reach the note or its open tag; Tab reaches the back's controls, Return opens the
reason box, Esc leaves the reason as it was and Esc again turns back with the caret where it
was; a point whose words were edited away offers no Reopen; Show on the page opens the tag;
hiding threads turns the note to its front; an untitled note with no threads has the empty
back. The fixture `every-ending` (one thread that ended each way) is for looking at the back.

## 7. Files and functions, and size

**Shared commit** (before either feature): `store.rs` (fields, `Ending`, helpers, tests),
`note_page.rs` (the five one-line changes in 3.3 and the dump keys), the `Thread` literals in
`welcome.rs` and `export.rs`.

**Let it stand: S** (under a day).
- New `src/stand.rs`: `points`, its tests, and the toast words.
- `note_page.rs` hooks: `context_with` (+1 key); `set_status`, Dismissed on a point (takes the
  box's text as the reason and picks the toast); `undo_dismiss` (finds the words again);
  `tag_panel` (the button's label and tooltip); `find_loose_thread` and `read_line` (handle
  `repeats`); the `stand:` step.
- `script.rs`: `Step::Stand(usize, String)`.
- `shortcuts.rs`: the labels **Let stand or dismiss the open thread** and **Undo let stand or
  dismiss**.
- `brain/src/protocol.ts`, `main.ts` (`context`, read and probe), `prompts.ts` (4.2, the
  schemas); `brain/test/e2e/fake-brain.ts` (`letStand`, `[[again]]`); `live.ts`
  (`letstand`).
- `design/capabilities.md`: the Snip row becomes Let stand. `AGENTS.md`: a map row for
  `src/stand.rs` (lint asks for one). The dump gains `lets_stand`: whether the open tag's second
  button reads Let stand.
- Shots: `design/features/shots/let-it-stand-reason-{light,dark}.jpg` (a reason typed in the
  tag's box) and `let-it-stand-toast-{light,dark}.jpg` (after Let stand: the toast, the next tag
  open).

**Turn it over: M** (a few days).
- New `src/back.rs`: the back's view (`view`, `entry`), its words, the ending marks,
  `to_markdown(note) -> String`, and the pure entry model with its tests. The model is
  `Entry { id, ending, opened, ended, mind, kind, before: Option<String>, after, messages,
  reason, words_found }`.
- `note_page.rs` hooks: one field (`back: back::Back`: shown, turned_at, editing reason,
  unfolded entries), `turn_over()`, the render branch that swaps the page's contents for
  `back::view` (the sheet, border and linen stay), the header's Turn over control, `escape`,
  `open()` (show the front), `set_hidden` (turn back first), `show_thread(id)` (pulled out of
  `jump_to_open`), the steps and the dump keys. Reopen from the back goes through
  `NotePage::set_status(id, Open)`, then finds the words again as `undo_dismiss` does
  (`nearest_match` from the old `start`) and adds the anchor. It also drops the thread's reply
  box (`replies`, `reply_subs`): Let stand leaves the reason typed there, so that Undo can put
  it back.
- `app.rs`: the `TurnOver` action, View › Turn Over, and `copy_note` copying the back while it
  shows. `shortcuts.rs`: ⇧⌘B and the label **Turn the note over**. `script.rs`: four steps.
- `design/capabilities.md`: a Turn it over row under Threads.

**As built (7 Nov 2026).** Where this design was silent or the build had a reason to differ:
- A tied-off thread's last message from its mind goes under the ending line (1.3, "Below it"),
  so above the line are its first message and the fold; nothing shows twice. A thread that
  never got a word (its first answer did not come) is left off the back.
- Turning back puts the front on the sheet from the first frame, fading in as the fold sweeps
  the other way, so a key pressed straight after Esc reaches the note (rule 6). Turning over
  is as written. While the back shows it has the keyboard: typing, ⌘↩ and ⇧⌘⌫ do nothing,
  and ⌥⌘↓ and ⌥⌘↑ turn back and jump. The window's controls do not stay faded on the back,
  because nothing is written there.
- Reopen drops the thread's reply box, so the reason Let stand left in it for Undo does not
  come back as the reason of the next Let stand. If the thread was the last one let stand,
  its Undo goes too, so it cannot be put back twice.
- The snipped tail is drawn at 78% of its thread, the faintest that keeps 3:1 on the page in
  every thread (sage in linen is the closest, at 3.3:1); a test holds it there.
- The Markdown's heading date is when the thread was opened, as on the entry's top row;
  every message is copied, the folded ones too.
- Shots: `design/features/shots/turn-it-over-{light,dark}.jpg` (the fixture `every-ending`),
  `turn-it-over-turning.jpg` (the fold mid-sweep) and `turn-it-over-reason.jpg` (a reason
  being written).

The back's Markdown, for ⇧⌘C:

```markdown
# The back of “Sub-50 10k by March”

## Tied off · Inkling’s correction · 4 Nov

~~If I hold 5:30 for the whole race, I’ll come in just under 50~~
If I hold 4:55 for the whole race, I’ll come in just under 50

> *Inkling:* At 5:30 a km, 10 km takes 55:00. …
>
> *Inkling:* 4:55 a km comes to 49:10. That works.

## Let stand · Inkling’s question · 6 Nov

“add a kilometre every week until I reach 16”

> *Inkling:* Starting from how far? …

Your reason: Ten weeks out, from 8 km.
```

The welcome tour and the film are not changed here (`src/welcome.rs` and `design/video/**`
belong to the film agent). `design/capabilities.md` lists both features, so the tour and the
film can pick them up.

## 8. Decisions made here, so nobody has to ask

- **Let stand replaces Dismiss only on a mind's open point.** Belief 7 counts cutting a thread
  as deciding the words stand, and now that the minds are told, the label should say what the
  button does. Your own threads, sketches and answers keep Dismiss.
- **The reason comes from the box you already reply in,** so letting a point stand stays one
  action, and adding a reason takes no new step or field. Return replies; Let stand decides.
  A reason can be added later on the back.
- **Existing dismissals count as let stand.** They were decisions under belief 7. Leaving them
  out would leave the oldest notes with the problem this feature fixes.
- **The let-stand list belongs to one note.** A lasting preference about you belongs in About
  you, not in a list that every note carries.
- **Export is unchanged.** Manifesto 3.2 argues exports already carry too much; the back can
  be copied whole with ⇧⌘C.
- **The border is unchanged.** It still counts tied-off threads only, until the owner decides.
- **Known limit.** A thread already tied off and then dismissed reads as let stand on the
  back, because both are stored as Dismissed. That takes opening its faint stitches, so it is
  rare, and its words stand either way.

No open questions.

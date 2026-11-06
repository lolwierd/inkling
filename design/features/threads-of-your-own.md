# Threads of your own

Manifesto section 2, item 3, approved by the owner: "Select some words and hang your own
question on them, in your colour. No mind answers unless you call one, and you tie it off or
cut it like any other thread." Designed on 7 Nov 2026 on branch `feat-own-threads`, then built
on the same branch. What changed while building is under "As built" at the end.

## The design in brief

- Select words, then **Ask yourself** in the selection menu, or ⇧⌘↩. The question box opens
  with no mind seated. Type the question and press Return: a thread hangs from your words, in
  your ink, and no mind answers.
- Your thread is sewn in **ink**, the colour of your words. Every mind's thread is coloured,
  so a coloured thread is a mind and an ink thread is you. Under the words it is a row of
  **seed stitches** (small dots), not a running stitch, so it differs in shape as well as
  colour. Its tag says **You · question** and sets your words in your own lettering.
- To bring a mind in, press **Ask Inkling** on the open tag (it names the note's mind), or
  start a line with any mind's name ("Skeptic, is that fair?"). Anything else you write in the
  tag is a line for yourself, and no mind sees it as a message.
- Only you tie it off or cut it. Editing its words never starts a reread, and no mind ever
  closes it.
- Minds see your threads, marked as yours, with one rule: unless you ask about one directly,
  don't answer it and don't ask it again in other words.

## Why ink and not the thread colour in Settings

The manifesto says "in your colour". Settings › Appearance names its colour setting
**Inkling's thread**, and the note's own mind asks in it. In a new notebook that colour is
madder, which is also Inkling's own colour. If your threads used it, your questions and
Inkling's would share one colour in the default setup, and colour is how the page tells who
asked what. Your colour in Inkling is the ink your words are written in, and it is already
the thread your own sewing uses: a checklist item's cross-stitch is sewn in ink. Ink is never a
mind's colour, so it keeps one meaning: yours. The choice lives in one function
(`own::ink`), so switching to the Settings colour would be a one-line change.

## What the writer sees and does

### 1. Start one

From the selection menu. Select "four days a week". The menu's actions read:

| Action | Hint on the right |
|---|---|
| Find a loose thread | ⌘↩ |
| Ask Inkling | Answered in the margin (was "Your own question", which now belongs to the next row) |
| **Ask yourself** | ⇧⌘↩ |
| Sketch it | Stitched step by step |

"Ask Inkling" names the note's mind, as it does today ("Ask Skeptic" on a Skeptic note).

Ask yourself opens the question box under the words:

- the dot before the box is in your ink;
- placeholder: **Ask yourself about this…**;
- the minds' chips, none of them seated;
- the line under them: **No mind answers. Add one to ask it.** on the left, **Enter to ask** on
  the right.

Type "Is four too many with the long run on Sunday?" and press Return. The box closes, the
caret goes to the end of the words, and the keyboard stays in the note. A row of ink seed
stitches appears under "four days a week", its thread comes up beside the column, and a
folded tab hangs in the margin with its number and **?** in ink. It does not open: you wrote
it, so there is nothing to read yet, and you can keep writing. Nothing is sent to the brain.

From the keyboard: select with ⇧ and the arrows, press ⇧⌘↩, type, press Return. ⇧⌘↩ with
nothing selected does nothing, as Ask does. If threads are hidden, ⇧⌘↩ shows them again
first, because you asked for a thread and should see it hang.

The question box is the same one Ask uses, so it also works the other way. In Ask Inkling's
box, unseating every mind turns it into Ask yourself (the line under the chips changes to
"No mind answers. Add one to ask it."), and seating a mind in Ask yourself's box turns it into
an ordinary Ask. Today a box cannot have fewer than one mind; that limit goes.

### 2. How it looks next to a mind's thread

| | A mind's question | A mind's correction | Your own thread |
|---|---|---|---|
| Colour | the mind's thread | the mind's thread | ink (your words' colour) |
| Under the words | running stitch | cross-stitch, then running stitch | seed stitches: dots in pairs |
| Folded tab | number, ? | number, × | number, ? |
| Tag's top row | **Inkling** · question | **Inkling** · correction | **You** · question |
| Its words | Inkling's lettering (Figtree, unless Settings says otherwise) | same | your lettering (Fraunces), in ink |
| Gutter | French knot in the mind's thread, numbered | same | French knot in ink, numbered |
| Marks only | ? in the mind's thread | × | ? in ink |
| Its thread to the tag | the mind's colour, faint at rest | same | ink, faint at rest |
| The wash on its words when open | the mind's colour | same | ink, as a light grey |

In greyscale the seed stitches and the "You" label tell your thread apart; in colour, ink
against a hued thread does it at a glance. Where your thread and a mind's share words, they
share one row of stitches as threads on the same words do today, a stitch each in turn, and
your turns are drawn as dots.

Numbering, the open count above the title ("3 loose threads"), the jump keys (⌥⌘↓ ⌥⌘↑), the
sidebar's dot for a note with open threads, dragging a tag, and placement in both margins or
the right margin all treat your thread like any other. It is a loose thread you have not
worked out yet, which is what belief 5 says the count shows.

### 3. Add to it, and call a mind in

Click the tab (or jump to it with ⌥⌘↓). It opens:

```
 ○ You · question                                   2
   Is four too many with the long run on Sunday?        ← your words, in your lettering
   Two easy, two hard. Check the club plan.             ← a line you added later
   Inkling                                              ← a mind you called, in its thread
   Four works if two are easy and the long run is
   one of them. Which two are hard?
 ┌ Add to it…                                  ➤ ┐
 Resolve   Dismiss   Ask Inkling        Make a note ⧉
```

- **Add to it…** is the box's placeholder (a mind's tag says "Write back…"). Return or ➤ adds
  your line under the others. No mind sees it as a message and nothing is sent.
- **Ask Inkling** brings the note's mind in. If the box has words, they go in first as your
  line and Inkling answers them. If it is empty, Inkling answers the thread as it stands. The
  answer streams in under Inkling's name, set in its thread colour, with the thinking dot
  while it comes. A mind with a name longer than 20 characters makes the button read
  **Ask a mind**, which still asks the note's mind.
- **A mind by name**: a line that starts with a mind's name and a comma or colon ("Skeptic,
  is two hard days enough?"), or "Hey Skeptic" or "@Skeptic", goes in as your line and that
  mind answers, the same rule as calling a mind from the note. Any mind you have made, and the
  built-in ones, can be called this way.
- A called mind answers once. It does not stay in the thread: your next plain line is for
  yourself again. That is the manifesto's rule taken literally, and it keeps the tag
  predictable: a mind speaks in your thread only right after you call it.
- One mind per call. To ask the room about the same words, select them and use Ask; that makes
  each mind its own thread, as today.

### 4. Tie it off, or cut it

- **Resolve** (or ⌘↩ with the tag open and nothing selected) ties it off as any thread: the
  thread pulls taut, the knot pops in ink, the tag reads **You · tied off**, and the border
  grows one motif, the same rule as every tied-off thread. If the manifesto's change to the
  border lands (marking a note once nothing in it is open), your threads follow it like the
  rest. **Reopen** loosens it again.
- **Dismiss** (⇧⌘⌫) cuts it: the scissors cut, the ink tail falls, and "Thread dismissed" shows
  with Undo for 8 s (⌥⌘Z). Cutting your own question means you dropped it. No mind is told,
  because it was never a mind's point.
- Neither needs a mind, and a mind never does either for you:
  - Editing the words under your thread moves the thread with them. It does not start a
    reread (the "fix the words, the thread settles" path skips your threads), because that
    reread is a mind deciding whether the question is settled.
  - A mind you called cannot tie it off. Its answer arrives as a message. Whether the
    question is settled is yours to decide.
  - Deleting all the words collapses the thread to where they were, as with any thread; the
    tag stays until you tie it off or cut it.

## Every state

| State | What happens |
|---|---|
| Linen | Ink is #26232B on the #FBF8F2 page (14:1): seed stitches, the ? and "You". |
| Lamplight | Ink is #EDE6DA on #1F1D22 (13:1). Nothing else changes; the thread colours are not involved. |
| Both margins | Your tag takes a side the way any new thread does (`pick_side`). |
| Right margin | On the right with the rest. |
| Marks only | A raised ? in ink after the words, set like a footnote mark; clicking it opens the tag. The seed stitches show under the words, as stitches do in this placement today. |
| Hidden threads | Hidden with every other thread, the box and the menu (belief 4: your page exactly as you left it). ⇧⌘↩ brings threads back before it opens the box. |
| A note that waits to be asked | Works the same; nothing is read either way. This is where your own threads help most, because the minds are quiet until called. |
| A note that reads along | The reading loop sees your threads as yours (see The brain) and does not ask your question back. |
| Export | Markdown: `> **Me** on “four days a week”`, then `> *Me:* …` for your lines and `> *Inkling:* …` for a mind's answer, under the paragraph like every thread. HTML and Word: "Me" as the thread's name, a called mind's paragraphs start with its name. Follows the export's own threads switch. |
| Copy (⇧⌘C, Copy on the tag) | Copy as Markdown is the export's Markdown. The tag's Copy gives `> quote`, then `You: …` and `Inkling: …`. |
| Make a note | As today: the quote, then "Me: …" and each mind's lines under its own name. |
| Undo | Cutting has the usual toast and ⌥⌘Z. ⌘Z in the note undoes text, never a thread, as today. A line added to your own thread has no undo, like a reply. |
| Several minds | Each answer in your thread is under its mind's name and colour. A mind's own thread on the same words shares the stitch row with yours. |
| A call fails | The usual toast (rate limit with its reset time, overloaded, the brain not running). Your thread and your line stay. An empty answer says "Inkling came back with nothing to say. Try again." |
| Switching notes mid-call | The call is cancelled; the thread stays (it has your words). |
| Learning about you | Lines you keep to yourself are never sent to the learn task. A line that calls a mind is sent like a reply, under the existing switch. |
| Keyboard only | Select, ⇧⌘↩, type, Return. ⌥⌘↓ opens the next open thread with the keyboard in "Add to it…". Return adds; a name at the start calls; Tab reaches Resolve, Dismiss, Ask Inkling, Make a note and Copy, with the stitched ring; ⌘↩ ties it off; ⇧⌘⌫ cuts it; Esc folds it. ⇧⌘↩ is listed under Threads in Settings › Shortcuts and can be rebound. |

## The data model

Three fields, each added at the end of its struct, each with a serde default so every older
note loads unchanged, and each left out of the file when it holds its default, so a note
without your threads is written exactly as before. An older build reading a newer note
ignores the fields: your thread shows there as Inkling's, which loses nothing.

```rust
// src/store.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]   // + Default
pub struct Thread {
    // ... every field as today ...
    /// You hung this thread yourself (Ask yourself): it is sewn in your ink, and no mind
    /// answers in it unless you call one. `mind_name` stays empty.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
}

pub struct Message {
    // ... role, text, at ...
    /// The mind that wrote it, when that is not the thread's own: a mind you called into
    /// your own thread. Empty otherwise.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mind: String,
}

// src/editor.rs
pub struct AnchorStyle {
    pub color: Hsla,
    pub correction: bool,
    /// Your own thread: seed stitches (dots) instead of a running stitch.
    pub own: bool,
}
```

`Thread` gains `Default` so that a struct literal can end in `..Default::default()`. The two
literals in `src/welcome.rs` and the one in `export.rs`'s tests take that line, and so does
the one in `open_thread`. If every feature that adds a `Thread` field ends those literals the
same way, the changes merge as identical lines.

No new `ThoughtKind` and no new `ThreadStatus`. Your thread is a `Question` with `own: true`.
A new enum value would make an older build drop the whole note, because one unknown value
fails the note's parse, and "who hung it" is a different question from "what it says".

Page state: `NotePage` gains `own_calls: HashMap<String, String>` (thread id to the mind
answering in it now), at the end of the struct, so the streaming answer shows under the right
name.

## The brain

**Yes, the reading loop sees your threads.** It needs to: without them a mind reading the
paragraph will often ask the very question you hung there. They are sent in the context's
thread list, marked:

```ts
// brain/src/protocol.ts
export interface ThreadMessage {
  role: "mind" | "you";
  text: string;
  mind?: string;          // which mind wrote it, when the app knows and it matters
}
export interface ThreadSummary {
  quote: string;
  messages: ThreadMessage[];
  resolved: boolean;
  own?: boolean;          // the writer hung it themselves
}
// the ask task takes the thread so far when a mind is called into the writer's own thread
| { task: "ask"; ctx: Context; target: string; question: string; history?: ThreadMessage[] }
```

`main.ts` keeps both new fields when it rebuilds the context (`context()` and `messages()`
copy known fields only, so without this they would be dropped silently).

In `prompts.ts`, the margin block labels the thread and adds one rule after the list, only
when the note has any of your threads:

```
Thread 3 (the writer's own) on «four days a week»:
  Writer: Is four too many with the long run on Sunday?

Threads marked as the writer's own are questions they hung on their own words, for
themselves. Unless they ask you about one directly, do not answer it and do not ask it again
in other words.
```

A message by a mind other than the one being prompted is labelled with that mind's name
instead of "You", so a mind called into your thread sees who said what.

- **Which threads.** Your open and tied-off threads go (tied-off ones are marked resolved, so
  no mind reopens what you settled). Cut ones do not, as for every dismissed thread today.
- **Calling a mind.** The `ask` task, as that mind (`context_with(mind)`), with `target` = the
  thread's words as they are now (its last words if they were deleted), `question` = the
  thread's first line, your question, and `history` = everything said since, in order, the
  line that called last. With no history the prompt is today's Ask prompt word for word. With
  history it reads: "On «…» the writer hung a question of their own: … What has been said in
  that thread since: … They have now called you into the thread.", then today's Ask
  instructions, "If their last line there is addressed to you, answer that", and "Stay on
  this thread's point". No new task, so `parity.test.ts` is untouched.
- **No other model call touches your thread.** No reread on edit, no tie-off, no learning from
  lines you keep to yourself.

## Fit with the other features

| Feature | What it means for your threads | Shared types |
|---|---|---|
| Answer in the note + Write it in | Answer in the note should skip `own` threads: a mind deciding a paragraph answers your question is a mind answering. Write it in should treat a tied-off own thread like any tied-off thread and offer your last line. | reads `Thread.own` |
| Let it stand + Turn it over | Cutting your own thread is not letting a point stand: the list of points let stand that goes to every mind must skip `own` threads. The back of the note lists your threads with the rest, named "You" (`Thread::asker`). | reads `Thread.own` |
| Show its working | No overlap: your threads hold no corrections. A mind called in answers as Ask does. | none |
| Loose ends | Your open threads are loose ends. Show them in ink and named "You", through `own::ink` and `Thread::asker`. | reads `Thread.own` |
| Threads between notes | No overlap: those threads are always a mind's. | none |

Shared types I change: `store::Thread` (+`own`, +`Default`), `store::Message` (+`mind`),
`editor::AnchorStyle` (+`own`), and in the brain `ThreadSummary` (+`own?`), `ThreadMessage`
(+`mind?`) and the `ask` task (+`history?`). Every addition is optional and at the end. The
two worth agreeing on early: `Thread` deriving `Default` with literals ending in
`..Default::default()`, and `ThreadSummary` as the place where per-thread facts for the
minds go (Let it stand will add its reason there too).

## Tests

Unit (Rust):

- `store`: a thread without `own` loads as false and a message without `mind` as empty; a
  thread with `own: true` round-trips; a note with no own threads serializes without either
  key.
- `note_page::own`: `called_mind` finds the mind a line calls ("Skeptic, …", "hey staff
  engineer …", "@Editor …"), prefers the longer of two names that both match, and finds
  nothing in "Skeptics say…"; `split_call` makes your first line the question and the rest,
  in order, the history; `Thread::asker` is "You" for your threads and the mind otherwise; `ink` is the page's
  ink in both lights and never a thread colour.
- `stitch`: `seed_dots` stays inside its run and keeps an even step.
- `export`: an own thread prints `**Me** on`, its lines as `*Me:*`, a called mind's as
  `*Inkling:*`.

Unit (brain): `readPrompt` labels own threads and adds the rule only when one is there; a
message by another mind is labelled with its name; `askPrompt` with history uses the
own-thread framing and without history is unchanged. Protocol: `ask` with `history` runs,
and `own` and `mind` survive `context()`.

Scenarios, against the fake brain (whose `ask` answer adds "(history N)" when it gets a
history):

`u15-own-threads.txt` (fixture `running-plan`):

```
select: four days a week
ask-yourself: Is four too many with the long run on Sunday?
settle
dump: hung
expect: d.threads.find(t => t.quote === "four days a week").own === true
expect: d.threads.find(t => t.own).msgs === 1 && d.active === null
expect: !tasks.includes("ask") && !tasks.includes("reply")   // no mind was asked
activate: 3
reply: Two easy, two hard. Check the club plan.
settle
dump: added
expect: d.threads.find(t => t.own).msgs === 2 && !tasks.includes("reply")
call: Inkling
settle
dump: called
expect: tasks.filter(t => t === "ask").length === 1
expect: requests.find(r => r.input.task === "ask").input.history.length === 1
expect: d.threads.find(t => t.own).msgs === 3
reply: Skeptic, is two hard days enough?
settle
dump: by-name
expect: requests.filter(r => r.input.task === "ask")[1].input.ctx.mind.name === "Skeptic"
expect: notes[0].threads.find(t => t.own).messages.some(m => m.mind === "Skeptic")
edit: four days a week => five days a week
settle
dump: edited
expect: d.threads.find(t => t.own).quote === "five days a week" && d.rechecking.length === 0
expect: !tasks.includes("reply")   // editing your thread's words asks no mind
type: Tempo runs on Tuesdays, intervals on Thursdays, easy the rest.
settle
expect: requests.filter(r => r.input.task === "read").pop().input.ctx.threads.some(t => t.own === true)
resolve: 3
dump: tied
expect: d.threads.find(t => t.own).status === "Resolved"
```

`u16-own-threads-states.txt` (fixture `running-plan`): ⇧⌘↩ from the keyboard opens the box
with nobody seated (`key: cmd-shift-enter`, `expect: d.asking && d.room.length === 0`);
Marks only shows the ink ? (`d.marks`); hiding threads hides it; a note that waits to be asked
still hangs it and sends nothing; export carries `**Me** on`; cutting it shows Undo, and ⌥⌘Z
puts it back.

Script steps added (`src/script.rs`): `ask-yourself: <question>` (Ask yourself on the
selection, as Return in the box does), and `call: <mind>` (call that mind into the open
thread, as Ask Inkling does for the note's mind). The dump gains `own` on each thread and
`room` while the box is open.

Real model, a handful of calls through `brain/test/live.ts`: a read of a paragraph whose
obvious question is already your own thread (the mind should stay silent on it or raise
something else), and a call into your own thread with history.

## Files and functions, and size

| File | Change |
|---|---|
| `src/note_page/own.rs` (new) | `ask_yourself`, `hang_own`, `add_to_own`, `call_into_own`, `stream_own`, the own tag's body and its Ask button, `called_mind`, `split_call`, `asker`, `ink`, unit tests. A child module, so it reaches the page's private state without widening it. |
| `src/note_page.rs` | small hooks: `mod own;`; the toolbar row and the Ask hint; `toggle_room` lets the last mind go; `submit_ask` hangs your thread when nobody is seated; `ask_box` hint and dot; `send_reply` hands your thread to `add_to_own`; `activate`'s placeholder; `note_rechecks` skips yours; `context_with` adds `own` and `mind`; `color_of` and `card` use ink; `tag_panel` names "You" and draws your lines; `thread_text` and `make_note_from_thread` name each mind; `debug_state` adds `own` and `room`; two script steps |
| `src/store.rs` | `Thread.own`, `Thread: Default`, `Message.mind`, tests |
| `src/editor.rs` | `AnchorStyle.own`; own runs drawn as seed stitches (about ten lines) |
| `src/stitch.rs` | `seed_dots`, and its test |
| `src/export.rs` | "Me" for your threads, each mind's name on its lines; a test |
| `src/app.rs`, `src/shortcuts.rs` | the `AskYourself` action, ⇧⌘↩, its row in Settings › Shortcuts |
| `src/script.rs` | `ask-yourself:` and `call:` |
| `src/welcome.rs` | `..Default::default()` at the end of its two thread literals, nothing else |
| `brain/src/protocol.ts`, `main.ts`, `prompts.ts` | the optional fields, keeping them in `context()`, the margin label and rule, the own-thread Ask framing |
| `brain/test/e2e/fake-brain.ts` | "(history N)" on an `ask` that has a history |
| `brain/test/unit.test.ts`, `protocol.test.ts`, `live.ts` | as in Tests |
| `brain/test/e2e/scenarios/u15-own-threads.txt`, `u16-own-threads-states.txt` | as in Tests |
| `design/capabilities.md` | "Ask yourself" under Asking; your threads under Threads |

Size: S, at the top of S. About 450 lines with tests, two thirds of them in the new module and
the tests.

## Open questions

1. **Ink, or the colour in Settings?** I sewed your threads in ink, for the reason above. The
   alternative honours "in your colour" literally but makes your threads and the note's mind's
   look alike in a new notebook. Recommendation: keep ink.
2. **Should a called mind stay in your thread?** Built: no, a mind answers once per call, so
   your next plain line stays private. The alternative is a conversation that continues until
   you close the tag, which is friendlier for a long back-and-forth but means a plain line
   might reach a mind. Recommendation: keep one answer per call; the button is one Tab away.

## As built (7 Nov 2026)

Everything above works end to end on `feat-own-threads`, with these differences from the
design:

- **Names live on `Thread`.** `Thread::asker()` ("You", or the mind) and `Thread::author(m)`
  (who wrote a message) are in `src/store.rs` rather than in the new module, because the
  export, the page and the features to come (Loose ends, Turn it over) all need them and the
  export should not depend on the page.
- **A called mind stays on the thread's point.** The first real call answered the burst
  question well, then went on to another paragraph and ran to three paragraphs. The
  own-thread framing now ends "Stay on this thread's point; other parts of the note belong in
  other threads.", the line the reply prompt already uses, and the second call stayed on point.
- **The question box's hint wraps.** "No mind answers. Add one to ask it." is longer than the
  hint it replaces and pushed "Enter to ask" past the box's edge; the hint now wraps.

Real model, four calls through `brain/test/live.ts` (Opus 5.5): `read_note` on the rate
limiter's "can never send more than 100 requests in any 60-second span" corrects the burst
across the minute's edge; `read_own`, the same read with that question already hung there as
the writer's own thread, stays silent; `ask_own`, a call into that thread with two lines of
history, answers the burst directly (twice: before and after the stay-on-point line).

Seen in the screenshots, not changed: a tag in the left margin at the default window width
opens toward the words, over the column, as every left tag does today (`tag_panel` looks for
room on the paper). It is more noticeable for your threads only because, like any new thread,
every other one takes the left margin.

Screenshots: `design/features/shots/own-threads-*.jpg`, Linen and Lamplight.

Left for the owner or another agent: the welcome tour (`src/welcome.rs`, the film agent's)
and the film do not show Ask yourself yet, though `design/capabilities.md` now lists it;
Settings › Minds still says the note's mind "asks in your thread colour", which reads oddly
next to threads of your own in ink.

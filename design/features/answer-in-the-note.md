# Answer in the note, and Write it in

Manifesto items 1 and 6 (`design/manifesto.md`, section 2), designed 7 Nov 2026 on branch
`feat-answer`. Both move what you worked out into the note itself, and both end the same way:
a thread whose knot points at your words in the note.

**State (7 Nov 2026):** the shared part and Answer in the note are built on `feat-answer`
(scenarios t14, t15, t16, t18; six live checks against the real model, below). Write it in
is being built by a second agent on top of the shared part. Screenshots, light and dark:
`design/features/shots/answer-in-the-note-{knot,hover,tag}-{light,dark}.jpg`.

They are built by two agents at once. This doc is the contract between them: the
**shared part** (the answer link and its knot) is built first, on `feat-answer`; then
**Answer in the note** (detection) and **Write it in** (the offer) are built on top of it,
each in its own files, so the branches merge cleanly. "Files and functions" says exactly
who touches what.

## The problem, in plain words

Belief 6 says what you work out belongs in the note, because the note is what is left when
the margin is hidden. Today two things get in the way.

1. **An answer written in the note is not noticed.** A thread ties itself off only when you
   edit the words it hangs on, or when your reply in the margin settles it. If you answer it
   the natural way, by writing a new sentence, the thread stays open beside a note that
   already says the answer. Even a sentence added to the same paragraph does not count,
   unless it changes the quoted words.
2. **A reply that settles a thread stays in the margin.** In the walkthrough film, the reply
   about the long run ties its thread off, yet the plan never says where the long run
   starts. Hide threads and the answer is gone from the page.

Answer in the note fixes the first: a paragraph you write anywhere in the note can tie a
thread off, and its knot points to your answer. Write it in fixes the second: when your
reply ties a thread off, the tag offers to put your reply, word for word, under the
paragraph, for you to rework.

Neither writes a mind's words into your note (belief 1). Answer in the note only links to
words you wrote. Write it in only moves your own reply, and only when you press it.

## What you see and do

The example is the running-plan note (`brain/test/e2e/fixtures/running-plan`). Thread 2,
Inkling's question on "add a kilometre every week until I reach 16", asks: "Starting from
how far? If you start at 8 km, 16 km is eight weeks out. Is the race at least eight weeks
away, with an easier week before it?"

### Answer in the note

1. Further down, you write a new paragraph: "The long run starts at 8 km on 6 December, so
   it reaches 16 km on 31 January, six weeks before the race, and the last week is easy."
   You pause.
2. Inkling reads the paragraph, as it reads every finished paragraph: the needle bobs in the
   gutter beside it, then a tacking stitch marks it read. If the paragraph raises something
   new, a new thread comes as usual.
3. The read notices that the paragraph may answer thread 2. The needle moves to thread 2's
   words while the mind that asked it checks your paragraph against its own question, as it
   does when you fix the words.
4. If your words settle it, thread 2 ties itself off:
   - Its thread pulls taut and knots, and the border at the top of the page grows one motif,
     exactly as for any tie-off.
   - Its mind adds one line to the thread saying what settled it, for example (the real
     model's words): "Starting at 8 km on 6 December gets you to 16 km on 31 January, six
     weeks before 14 March, and the last week is easy."
   - The count above the title drops ("1 loose thread"), and the numbered knot leaves the
     gutter beside thread 2's paragraph.
   - **At your paragraph, a small French knot sews in just after the last word that
     answered**, in the thread's colour. That knot is the thread's end, tied off at your
     answer. It is the only new mark on the page.
5. Point at the knot: the words that answered wash faintly in the thread's colour, and a
   tooltip reads: **"Answers Inkling’s thread on “add a kilometre every week until I reach
   16”. Click to see it."**
6. Click the knot: the page scrolls to thread 2's words and opens its tag. The tag reads
   "Inkling · tied off", shows the conversation, and under it a new row:
   **"Answered in your note:"** followed by the first words of your answer in your own
   typeface, **“The long run starts at 8 km on 6 December, so it reaches 16 km…”**. Clicking those words takes
   you back to your answer and washes it. The tag's first button, as on any tied-off tag,
   is **Reopen**.
7. If the knot is wrong (the paragraph does not really answer it), Reopen: the thread is
   open again, numbered, with its knot back in the gutter; the knot at your paragraph comes
   out; and the thread remembers that those words are not its answer, so the same paragraph
   never ties it off again.
8. If you later edit the answering words, the knot follows them. If you delete them all, the
   knot goes and the thread stays tied off; its tag no longer shows the "Answered in your
   note" row.

A wrong answer does not count, and the read says what is wrong instead. In the first live
check the test paragraph itself had a slip ("16 km on 24 January": a kilometre a week from
8 km on 6 December reaches 16 km on 31 January). The read raised a correction on those
words, and the thread's mind refused to settle the thread on them.

An explanation you asked for counts the same way. Select "VO2 max", choose Ask Inkling,
type "What is it?", and Inkling explains in the margin. When you later write "VO2 max is the
most oxygen my body can use in a minute, which caps how fast I can run for long", that ask
thread ties itself off with its knot at your sentence. Pasting Inkling's own explanation
does not count: an explanation counts once it is in your words.

What you never see: a toast, a sound, or anything that stops your typing. If the tie-off
arrives while you are typing again, the knot pops in beside your words without moving
them.

### Write it in

1. You open thread 2 and reply: "I start at 8 km in the first week of December and take
   the last week easy." Its mind answers "That settles it: 16 km lands seven weeks before
   the race." and the thread ties itself off, as today. The tag stays open, now reading
   "Inkling · tied off".
2. Under the conversation, above the buttons, the tag shows a new row, after a short
   running-stitch rule:

   > Your note doesn’t say this yet.
   > **[Write it in]** under its paragraph, word for word

   The button has the tooltip: **"Puts your reply under the paragraph this thread is
   about, for you to rework in your own words. The thread is already tied off."**
3. Press Write it in (or Tab to it and press Return):
   - Your reply appears as a new paragraph right under "Sunday is the long run. I’ll add a
     kilometre every week until I reach 16.", word for word, in your typeface.
   - It washes in the thread's colour for a moment, the tag closes, and the caret waits at
     the end of the new paragraph, so you can start reworking it straight away.
   - The knot from Answer in the note sews in at its end: the thread now points to it.
   - A toast says **"Your reply is in the note, ready to rework."** with **Undo** (8 s).
4. Rework it into a sentence of your own. The knot follows your edits. Inkling reads the
   reworked paragraph like any edited paragraph; the words exactly as you replied are not
   read again, because the mind has already answered them.
5. Undo (the toast's, or ⌘Z) takes the paragraph out again; the offer comes back the next
   time the tag is open.

How it explains itself, since "word for word under its paragraph" is hard to picture
before you see it: every part of the question "what will this do?" is answered on screen
at the moment it matters. *What*: your reply, which sits just above the offer. *Why*: "Your
note doesn’t say this yet." *Where*: "under its paragraph". *How much*: "word for word",
then the result itself, washed, with the caret in it. *Can I take it back*: Undo.

When it is offered: the thread is tied off by your reply (`tied_by == Reply`), it has no
answer in the note yet, threads are not hidden, and your last reply in it has at least four
words (the floor the reading loop uses for a paragraph worth reading; "Yes, exactly." is
not worth writing in). It is not offered for threads tied off by hand, by an edit, or by an
answer in the note. Closing the tag without pressing it is the "no": nothing is stored.

## Every state

| State | Answer in the note | Write it in |
|---|---|---|
| **Linen and Lamplight** | The knot is drawn in the thread's colour from the palette (madder #A63F32 in Linen, #D9705E in Lamplight for Inkling; other minds their own), radius 3.4 px, at full strength so it keeps 3:1 against the page like other marks. The wash under the answer is the thread colour at 8% (Linen) or 12% (Lamplight), the active thread's wash. | The offer row uses the tag's own colours: the sentence in `faint`, the button a ghost button with the stitched focus ring in the thread's colour. The new paragraph's wash is the same as the answer wash. |
| **Right margin** | Same; tags hang on the right. Clicking the knot opens the tag on the right of the question. | Same. |
| **Marks only** | The knot after your answer is a mark like the ? and ×: click it and the thread's card opens beside its question (scrolled into view), as clicking its ? would. | The offer shows in that card. |
| **Hidden threads** (⇧⌘.) | Knots and washes hide with every other mark. Reading goes on, so a thread can tie off while hidden; showing threads again shows it tied with its knot in place, without replaying the animation. | No offer while hidden (tags are hidden, and you cannot reply). |
| **A note that waits to be asked** | It never starts a thread. While it has open threads (from your asks, Find a loose thread, or a call by name), each finished paragraph is read once for answers only (`answersOnly`; the needle shows while it reads, as for any read); nothing is read when no thread is open. A paragraph written before the thread can be read once too, when the reading loop next looks at it. See the open question. | Works the same. |
| **Imported or guide text left alone** | Paragraphs in `left_alone` (an Obsidian import, the Start here guide) are never read, so they never count as answers. Once you edit one, it is your paragraph and is read like any other. | Not affected. |
| **Export and copy** | With threads, a thread tied off by an answer prints one more line under its messages: Markdown `> *Answered in the note:* “…”`, HTML and Word `Answered in the note: “…”`. Without threads, nothing is added: the answer is already in your text. The knot never prints. Copy (⇧⌘C) is Markdown with threads, so it has the line too. | The written-in paragraph is ordinary text and exports as such; its thread gets the same "Answered in the note" line. |
| **Undo** | A tie-off is not a text edit, so ⌘Z does not untie it; Reopen does. ⌘Z that removes all of the answer's words removes the knot (the thread stays tied off). Redo brings the words back; the saved answer finds them again by their words, and the knot returns the next time the note opens. | One undo step: ⌘Z or the toast's Undo removes the paragraph; the toast's Undo works only while the paragraph is unchanged. |
| **Several minds** | Any open thread can be answered, whichever mind asked it. The check runs as the thread's own mind, so Skeptic judges Skeptic's question by its own standard; its line in the thread is in its voice, and the knot is in its colour. One paragraph can tie off several threads, such as three minds asked the same thing in Ask the room; each gets its own knot, side by side after the answer. | The offer is per thread, under whichever mind's thread your reply settled. |
| **The thread's own paragraph** | A sentence added to the paragraph a thread hangs on counts too, as long as the quoted words themselves are unchanged (if they changed, Fix the words re-checks instead, one look per edit). | The paragraph goes under it. |
| **Lists, callouts, code** | A checklist item, a callout line or a code block is read like a paragraph and can be an answer. | "Under its paragraph" means under the whole list, callout patch or code block the thread's words are in, so the new paragraph never splits them. |
| **The first sentence of a new notebook** | The probe is unchanged; there are no threads to answer yet. | Not affected. |

## Data model

Everything lives on `Thread` in `src/store.rs`, in three fields added at the end of the
struct. Old notes load unchanged (the fields default), and notes that use none of them
write no new keys (`skip_serializing_if`). `Thread` also derives `Default`, so the places
that build one (`open_thread`, the guide in `welcome.rs`, export's tests) end with
`..Default::default()` and no later field has to touch them again.

```rust
/// Where a thread was answered in the note: words you wrote that tied it off (Answer in
/// the note), or your reply, written in under its paragraph (Write it in). A byte range of
/// the joined text, like the thread's own, and the words it covered, by which it is found
/// again when the range goes stale.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Answer {
    #[serde(default)]
    pub start: usize,
    #[serde(default)]
    pub end: usize,
    #[serde(default)]
    pub quote: String,
}

/// What tied a thread off.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TiedBy {
    /// Resolve, or ⌘↵, by hand.
    Hand,
    /// Your reply in the margin, which its mind said settles it.
    Reply,
    /// You fixed the words it hangs on, and its mind said that settles it.
    Edit,
    /// A paragraph you wrote in the note answers it.
    Answer,
}

pub struct Thread {
    // ... every field as today, then:
    /// Where in the note it was answered, while it is tied off and those words are there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<Answer>,
    /// What tied it off. `None` while it is open, and for threads tied before this was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tied_by: Option<TiedBy>,
    /// Words you said are not its answer (Reopen after a paragraph tied it off), at most
    /// five: they never tie it off again, and its mind is told.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_answers: Vec<String>,
}
```

In a note file:

```json
{ "id": "t2", "status": "resolved", "...": "...",
  "answer": { "start": 538, "end": 662, "quote": "The long run starts at 8 km on 6 December, ..." },
  "tied_by": "answer" }
```

Rules:

- **`tied_by`** is set wherever a thread ties off: `send_reply` (Reply), `recheck` (Edit),
  the answer check (Answer), and `set_status(Resolved)` (Hand, only if nothing set it
  first). Reopen clears it. A dismissed thread keeps whatever it had. A value this build
  does not know (a newer build's) reads as `None`, so it costs that field, not the note.
- **`answer` follows edits** as an editor `Anchor` with the id `"<thread id>#answer"`
  (`crate::answers::anchor_id`). It has no stitch style, so the editor draws nothing for it and a
  click never opens it; it only keeps the range on its words through typing, undo and
  redo, as thread anchors do. `load_into_editor` adds it (re-finding the words nearest the
  stored offset when the offsets no longer match, as threads are re-found), and
  `sync_from_editor` writes it back. When its words are all gone (an empty range, and the
  words not found again), `answer` becomes `None`.
- **Reopen** of a thread with an answer removes the answer anchor and sets `answer` to
  `None`. If `tied_by` was `Answer`, the answer's words also go into `not_answers` (the
  oldest drops past five).
- **Write it in** sets `answer` to the new paragraph's range and words, and leaves
  `tied_by` as `Reply`. That is all it stores: an answer present means the reply is in
  the note, so the offer no longer shows.
- **The page, not the store:** `answer_checks: HashSet<u64>` on `NotePage` holds the
  hashes of paragraphs a waiting note checked for answers in this session, so a paragraph
  is checked once per version. It is not saved: after a relaunch only paragraphs you edit
  are considered again, and those have new hashes.

## The brain

### What the minds are told

The context's thread summaries (`ThreadSummary` in `brain/src/protocol.ts`, built in
`context_with` in `src/note_page.rs`) gain three optional fields:

```ts
export interface ThreadSummary {
  quote: string;
  messages: ThreadMessage[];
  resolved: boolean;
  id?: string;            // the thread's id, so an answer can name it without guessing
  open?: boolean;         // still open (without it: not resolved), so a thread let stand never counts as open
  notAnswers?: string[];  // words the writer said do not answer it
}
```

(The design first had `kind` and `mind` too; the thread's messages already show who asked
what, and `settle` runs as the thread's own mind, so they were left out.)

`threadsBlock` in `prompts.ts`, which every prompt shares, does not change: Let it stand is
changing it in parallel, and the answer check needs nothing from it but its numbers.

### `read` names the threads a paragraph may answer

When the note has open threads, `readPrompt` gains a short block after its instructions,
and the read's schema gains `answers` (`THOUGHT_WITH_ANSWERS_SCHEMA`, built from
`THOUGHT_SCHEMA` so Show its working's change to that schema flows into it):

```text
Open margin threads: 2, 3. Check whether this paragraph answers any of them.
- It answers a thread when someone who read only that thread and this paragraph would say
  the thread's point is settled: the paragraph gives the number, reason, decision or
  corrected fact the thread asked for, and gets it right; or, for a thread where the writer
  asked for an explanation, it explains that thing correctly in the writer's own words.
- It does not answer a thread when it only touches the same subject, repeats or rephrases
  the question, promises to deal with it later, answers only part of it, copies the
  margin's own words, or gives a wrong answer (then say what is wrong instead).
- answers: for each thread it answers, the thread's number and quote: the exact words of
  the paragraph that answer it, the shortest span that does, often one sentence. Most
  paragraphs answer nothing; then answers is empty.
[Thread 2: the writer said these words do not answer it: «...».]
```

```ts
answers: { type: "array", items: { type: "object", additionalProperties: false,
  required: ["thread", "quote"], properties: { thread: { type: "integer" }, quote: { type: "string" } } } }
```

`pickAnswers` (`output.ts`) turns each answer into `{ thread: <id>, quote }`: the number
picks `ctx.threads[n - 1]`, an entry for a thread that is not open, unknown or has no id is
dropped, the quote is snapped onto the paragraph (`snapQuote`, as every quote is), and at
most three are kept. The `done` data carries `answers` only when there are some, so a read
that answers nothing looks exactly as it does today.

With no open threads the prompt and the schema are today's.

**A note that waits to be asked** sends the same `read` with `answersOnly: true`
(`answersOnlyPrompt`). Its prompt drops "decide whether to say something" and says
instead: "The writer has asked you to stay quiet while they write, so raise nothing new",
followed by the same block. `main.ts` always returns `speak: false` for it, and answers
without calling the model when no thread is open.

### `settle`: the thread's own mind checks the answer

A new task, sent once for each thread a read named, with the context as the thread's mind
would read the note (`context_with(mind)`):

```ts
| { task: "settle"; ctx: Context; quote: string; messages: ThreadMessage[]; target: string; answer?: string }
// done data: { settled: boolean, quote: string, text: string }
```

`target` is the whole paragraph, `answer` the words the read named. The prompt:

```text
<note ...> and <margin> as every prompt has them

This margin thread is about «{quote}»:
You: ... / Writer: ...  (the thread so far)

Since then the writer wrote this paragraph in the note:
<paragraph>{target}</paragraph>
The words that may settle it: «{answer}»
[The writer has said these words do not answer it: «...»]

Does this paragraph settle the thread? Be strict: when in doubt, it does not.
- It settles the thread only when it gives what the thread asked for (the number, the
  reason, the decision, the corrected fact) and gets it right; or, where the writer asked
  for an explanation, when it says that explanation correctly in the writer's own words.
- It does not settle it when it only touches the subject, repeats or rephrases the
  question, promises to deal with it later, answers only part of it, copies the thread's
  own words, or is wrong.
- settled: true or false.
- quote: when settled, the exact words of the paragraph that settle it, the shortest span
  that does (often one sentence); otherwise an empty string.
- text: when settled, one short sentence for the thread saying what settled it, in the
  writer's terms, with no praise; otherwise an empty string.
```

`main.ts` snaps the quote onto the paragraph and reports `settled: false` when the model
said settled but gave no words, because an answer with no words has nothing for the knot to
point at. A `settle` that is not settled says nothing anywhere: the paragraph may never have
been meant as an answer, and belief 3 keeps Inkling quiet about it.

### Keeping false positives out

A wrong tie-off costs more than a missed one. A missed answer leaves a thread open that you
can still resolve; a wrong one takes a question out of the margin, perhaps far up the page
where you are not looking. So the check is three gates, each able only to say no:

1. **The read names candidates** (reading effort, low). It already holds the note and every
   thread, so naming costs no extra call. It leans towards naming, with the rules above.
2. **The thread's own mind confirms** (`settle`, at the answering effort, medium), one
   thread and one paragraph at a time, told to be strict and to give the exact words. This
   is the same judgment Fix the words already trusts a mind to make about an edit.
3. **The app checks what it can by itself** before tying anything:
   - the thread is still open, nothing is in flight on it (no reply, no re-check), and it
     has not changed since the paragraph was read;
   - the answer's words are in the paragraph as it is now;
   - the paragraph is not one of the thread's `not_answers`;
   - for an explanation you asked for (an ask, a call by name or a sketch), the paragraph
     does not copy the mind's words: no run of eight words in a row, ignoring case and
     punctuation, from the thread's mind messages (`copies_mind`). Corrections are exempt,
     because a corrected fact is the same words whoever writes it.

Reopen is the way back from a wrong one, and `not_answers` makes it stay back.

### What it costs

Measured on 7 Nov 2026 with Opus 5.5 through the brain (`brain/test/live.ts`, eight calls):
a read took 13 to 27 s and a `settle` 13 to 20 s; each cost about $0.03 in API terms with a
warm prompt cache, and $0.40 to $0.86 when the cache was cold (Claude Code's own prompt
dominates a cold call, on any task).

- **A read in a note with open threads** carries the block above: about 870 characters,
  some 200 tokens, plus a line per thread that has words turned down; and about 5 more
  output tokens when it answers nothing. Against a read's whole prompt (the note, its
  threads and the runtime's own) that is a few percent, and nothing at all in a note with
  no open threads. No extra call.
- **A `settle`** costs about one read (it carries the note too) plus its instructions,
  with a short answer, at medium effort. It runs only when a read names a thread, at most
  three per read; most reads name none.
- **A note that waits to be asked** spends one low-effort read per finished paragraph,
  only while it has an open thread. Today such a note spends nothing on paragraphs.

## Fit with the features designed in parallel

| Feature | How it meets these | Shared types |
|---|---|---|
| **Let it stand** (a reason when you dismiss; minds told) | A thread let stand is not open, so it is never answered. Its "never ask again" and `not_answers` are the same idea (remember a no) at different scales; both reach the minds through `ThreadSummary`. | `Thread` (they add the reason; I add `answer`, `tied_by`, `not_answers` at the end), `ThreadSummary` (each adds optional fields), `threadsBlock` (theirs; I do not touch it). |
| **Turn it over** (every thread on the back of the note) | `tied_by` and `answer` say how each thread ended and where it was answered: "Tied off by your answer: «…»". If Turn it over lands its own record of how threads end first, `tied_by` and `answer` move into it unchanged. | `Thread`, `TiedBy`. |
| **Threads of your own** | Your own question is answered the natural way, by writing the answer: such threads are answerable like any other, judged by the note's mind, with `settle` told the writer asked it. | `ThoughtKind` (they may add a kind; `answers::answerable` treats every open kind as answerable). |
| **Show its working** (a correction carries a check) | A correction's check helps `settle` judge whether your answer is right. Both change the read's schema: they add to `THOUGHT_SCHEMA`; mine is built from theirs. | `THOUGHT_SCHEMA` in `prompts.ts`. |
| **Loose ends** (every open thread in one list) | A thread tied off by an answer leaves the list at once. Nothing else is shared. | None. |
| **Threads between notes** | Out of scope: an answer is looked for only in the note the thread hangs in. A later version could add `note: Option<String>` to `Answer`. | None now. |
| **Read it through** (a waiting note read once, whole) | It reads with `read`'s rules, so it names answers for free; its read results should go through the same `take_answers` hook as reading along. | The read's `done` handling. |

Exactly which shared types change, all of them:

- `src/store.rs`: `Thread` gains `answer`, `tied_by`, `not_answers` (at the end) and derives
  `Default`; new `Answer` and `TiedBy`.
- `brain/src/protocol.ts`: `ThreadSummary` gains optional `id`, `kind`, `mind`,
  `notAnswers`; `Task` gains `settle`, and `read` gains optional `answersOnly`.
- `brain/src/prompts.ts`: new `THOUGHT_WITH_ANSWERS_SCHEMA`, `SETTLE_SCHEMA`,
  `answersBlock`, `settlePrompt`; `readPrompt` gains the block. `THOUGHT_SCHEMA` and
  `threadsBlock` are unchanged.
- `src/editor.rs`: `NoteEditor` gains `washes` (at the end). Write it in adds
  `insert_paragraph_after`.
- `src/toast.rs`: Write it in adds `Kind::WrittenIn`.

## Tests

### Rust unit tests

- `store`: a thread written before these fields loads with `answer: None`, `tied_by:
  None`, `not_answers: []`; a thread with all three round-trips; a thread with none writes
  no new keys.
- `answers` (the pure rules, `src/answers.rs`): `answerable` (open threads of every kind;
  not tied off, dismissed, or pending); `copies_mind` (an eight-word run copied, ignoring
  case and punctuation, is a copy; seven is not; a paraphrase is not); `is_not_answer`;
  `push_not_answer` keeps five; `relocate` finds an answer's words again nearest where they
  were.
- `export`: a tied-off thread with an answer prints "Answered in the note" in Markdown and
  HTML; without threads, nothing.
- `editor` (Write it in): `insert_paragraph_after` puts a paragraph after a paragraph, after
  the last item of a list, after a callout patch and after a code block; it is one undo
  step; the caret ends after it; an anchor or mark ending at the old paragraph's end does
  not grow into it.

### Brain tests

- `protocol.test.ts`: a read with answers maps numbers to ids, snaps quotes, drops resolved
  or unknown threads and keeps at most three; a read with none has no `answers` key; an
  `answersOnly` read never speaks; `settle` returns `{settled, quote, text}`, and settled
  with no words is not settled.
- `parity.test.ts` passes with `settle` in the protocol, the app and the fake brain.

### Fake brain

Directives, read from the paragraph (the read's and the settle's `target`). Put them
before the paragraph's last full stop: a paragraph ending in `]]` is not finished, so it is
not read.

- `[[answers:2]]`: the read names thread 2 (the number in the context), quoting the
  paragraph's first six words; `[[answers:2,3]]` names two.
- `[[settle>unsettled]]`: the settle says no. By default a settle says yes, with the
  read's words, and the text `[rN] Settled by “…”`.

### Scenarios

Built, in `brain/test/e2e/scenarios/`; the dump has `answers` (thread, words, range,
`tied_by`), `knots` (threads whose knot is drawn), `tied_by` and `washed`, and the script
has `knot: N` (click thread N's knot) and `hover-knot: N` (rest the pointer on it).

`t14-answer-in-note.txt`:

```text
# fixture: running-plan
wait: 1500
type: The long run starts at 8 km on 6 December and reaches 16 km by late January [[silent]] [[answers:2]].
settle
dump: answered
expect: d.statuses.includes("t2:Resolved:Inkling") && d.statuses.includes("t1:Open:Inkling")
expect: d.answers.length === 1 && d.answers[0].thread === "t2" && d.answers[0].quote.startsWith("The long run starts at 8")
expect: d.knots.includes("t2") && d.tied_by.t2 === "answer"
expect: tasks.join() === "read,settle"
expect: requests[1].input.target.startsWith("The long run starts") && requests[1].input.answer === "The long run starts at 8"
expect: requests[0].input.ctx.threads.find(t => t.id === "t2").open === true
expect: note("Sub-50 10k by March").threads.find(t => t.id === "t2").messages.at(-1).text.includes("Settled by")
export: md
knot: 2
dump: knot-clicked
expect: d.active === "t2" && d.cursor_line === 3
reopen: 2
dump: reopened
expect: d.statuses.includes("t2:Open:Inkling") && d.answers.length === 0 && d.knots.length === 0
expect: note("Sub-50 10k by March").threads.find(t => t.id === "t2").not_answers[0] === "The long run starts at 8"
expect: note("Sub-50 10k by March").threads.find(t => t.id === "t2").tied_by === undefined
expect: log.some(l => l.msg.includes("Answered in the note:* \u201cThe long run starts at 8\u201d"))
quit
```

- `t15-answer-not-settled.txt`: `[[answers:2]] [[settle>unsettled]]`; thread 2 stays open
  with its one message, no answers, no knots, no problem toast, tasks `read,settle`.
- `t16-answer-waiting.txt`: a note set to `mode: ask` reads nothing while no thread is open;
  after `ask:`, a paragraph with `[[answers:1]]` ties the ask thread off; every read it sent
  was `answersOnly`, and the `settle` ran as Inkling.
- `t18-answer-room.txt`: three minds asked the same thing (Ask the room); one paragraph
  answers all three; three knots, three tie-offs, and each `settle` ran as its own mind
  (Inkling, Skeptic, Socratic tutor).

`t17-write-it-in.txt` (for the Write it in build):

```text
# fixture: running-plan
wait: 1500
activate: 2
reply: I start at 8 km in the first week of December and take the last week easy. [[resolved]]
settle
dump: offered
expect: d.statuses.includes("t2:Resolved:Inkling") && d.offer === "t2"
writein
settle
dump: written
expect: d.kinds.length === 7 && d.answers[0].thread === "t2"
expect: textOf(note("Sub-50 10k by March")).split("\n")[4].startsWith("I start at 8 km")
expect: !requests.some(r => r.input.task === "read" && r.input.target.startsWith("I start at 8 km"))
expect: d.toasts.some(t => t.text.startsWith("Your reply is in the note"))
key: cmd-z
dump: undone
expect: d.kinds.length === 6 && d.answers.length === 0
quit
```

### Live checks (the real model, a handful of calls)

In `brain/test/live.ts`, one call each, run on 7 Nov 2026 with Opus 5.5:

| Check | Expects | Result |
|---|---|---|
| `read_answers` | a paragraph that answers thread 2 names it, and not thread 1 | named thread 2 with the whole sentence |
| `read_answers_mention` | a paragraph that only mentions the long run names nothing | nothing named, silent |
| `settle_yes` | the answer settles thread 2, with its words and a line | settled, with a line in the writer's terms |
| `settle_mention` | only mentioning the subject does not settle it | not settled |
| `settle_promise` | "I still need to work out where the long run starts" does not | not settled |
| `settle_wrong` | 8 km to 16 km "after four weeks" does not | not settled |

The first run of `read_answers` and `settle_yes` failed for the right reason: the test
paragraph's date was wrong (see "What you see and do"), and the model caught it. With the
date fixed, all six pass.

## Files and functions, and size

### Shared part (built first, on `feat-answer`): the answer link and its knot

| File | What |
|---|---|
| `src/store.rs` | `Answer`, `TiedBy`, the three `Thread` fields, `Thread: Default`, tests |
| `src/answers.rs` (new) | the pure rules: `answerable`, `copies_mind`, `is_not_answer`, `push_not_answer`, `relocate`, `answer_anchor_id` |
| `src/note_page/answers.rs` (new, `impl NotePage`) | `load_answers`, `sync_answers`, `before_status` (tied_by, Reopen), `answer_knots` (the knots, their tooltip and click), `answer_row` (the tag's "Answered in your note"), `go_to_answer`, `go_to_question`, `wash_answer`, `dump_answers`, `click_knot`, `hover_knot` |
| `src/note_page.rs` (hooks, one line each where possible) | `mod answers;`; `load_into_editor`, `sync_from_editor`, `set_status`, `send_reply`, `recheck` set or keep `tied_by`; `tag_panel` adds `answer_row`; `render` adds the knots and the washes; `debug_state` adds `answers` and `knots`; `script_step` handles `knot:` |
| `src/editor.rs` | `NoteEditor.washes: Vec<(Range<usize>, Hsla)>`, painted under the text like the active wash (about 8 lines) |
| `src/export.rs` | the "Answered in the note" line in `thread_markdown` and `thread_html` |
| `src/script.rs` | `knot: N` (click thread N's knot), `hover-knot: N` (rest the pointer on it) |
| `brain/test/e2e/fixtures/answered` | the running plan with thread 2 tied off by an answer, for looking at the knot |
| `src/welcome.rs` | two thread literals end with `..Default::default()` (nothing else) |

### Answer in the note (this branch)

| File | What |
|---|---|
| `brain/src/protocol.ts`, `prompts.ts`, `main.ts`, `output.ts` | the context fields, `answersBlock`, `answersOnlyPrompt`, `settlePrompt`, `THOUGHT_WITH_ANSWERS_SCHEMA`, `SETTLE_SCHEMA`, `answersOnly`, `settle`, `pickAnswers` |
| `brain/test/e2e/fake-brain.ts`, `unit.test.ts`, `protocol.test.ts`, `live.ts` | as in Tests |
| `src/note_page/answers.rs` | `take_answers` (from a read), `read_for_answers` (waiting notes), `listens_for_answers`, `settle_answer`, `tie_by_answer` |
| `src/note_page.rs` (hooks) | `read_line`'s `done` calls `take_answers`; `consider` sends a waiting note's paragraphs to `read_for_answers`; `context_with` adds the thread fields; `NotePage.answer_checks` |
| scenarios `t14`, `t15`, `t16`, `t18`; `design/capabilities.md` | one row under Reading along |

### Write it in (the second agent, on top of the shared part)

| File | What |
|---|---|
| `src/editor.rs` | `State::insert_paragraph_after(line, text)` and its `NoteEditor` wrapper; tests |
| `src/note_page/write_in.rs` (new, `impl NotePage`) | `write_in_offer` (the tag row), `write_in`, `undo_write_in` |
| `src/note_page.rs` (hooks) | `mod write_in;`; `tag_panel` adds `write_in_offer` |
| `src/toast.rs`, `src/app.rs` | `Kind::WrittenIn` (8 s, Undo) and its arm in the stack's Undo |
| `src/script.rs` | `writein`, `undo-writein` |
| scenario `t17`; `design/capabilities.md` | one row under Threads |

The two meet only in `tag_panel` (one line each, on different lines) and in
`capabilities.md` (one row each). Write it in reads `tied_by` and writes `answer` through
the shared part, and calls `wash_answer`; it changes nothing in Answer in the note's
files.

**Size:** Answer in the note is M (the shared part is about half of it); Write it in is S.

## Open question for the owner

**Should a note that waits to be asked check your paragraphs for answers?** Today such a
note spends nothing on your paragraphs. Checking means one low-effort read per finished
paragraph, but only while the note has an open thread, and it never starts a thread.
Without it, "an explanation you asked for counts once you have written it in your own
words" would not work in the notes where you ask the most. *Recommendation: yes, only
while a thread is open, as built.* Turning it off is one condition in `consider`.

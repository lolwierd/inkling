# Threads between notes

Manifesto, section 2, item 10 (L): when something you write contradicts or depends on
something in another note, Inkling asks about it on a thread that quotes both and links the
two. Written 7 Nov 2026. Two agents build it: one the brain and the data, one the app side.
The section "The contract" below is what they agree on; everything else explains it.

## The bottom line

- **Finding the other note costs nothing until there is something to find.** A local index
  of every paragraph in your notes, kept in memory, scores the paragraph Inkling just read
  against all the others by the words they share, weighted by how rare each word is in the
  notebook. Only when a paragraph in another note shares the subject strongly enough (at
  least two uncommon words and a high enough score) does Inkling make one extra, small
  model call, `relate`, with that paragraph and at most three short passages. Most pauses
  add no tokens at all; a pause that does costs about what a read of a short note costs:
  2,200 tokens in (900 of them the runtime's own prefix, read from the cache) and 160 to
  300 out, measured.
- **One thread, two ends.** The thread lives in the note where you were writing (its
  home). The other note's words are its far end, stored beside the thread in
  `Note.elsewhere`. When you open the other note, the same thread hangs from those words
  too, with the same conversation. Replying, tying off, cutting it or undoing at either end
  does it to the one thread.
- **It ties off when the two notes stop disagreeing**, whichever note you fix, or when you
  say why both can stand.

## What the writer sees and does

The example is two notes from the design board's running plan: "Race calendar", written on
3 September, and "Race-week plan", written today.

1. You write in "Race-week plan" and finish a paragraph: *I'll pick up my bib on Saturday
   14 March, after the shakeout run.* You pause.
2. Inkling reads the paragraph as it reads any other. The tacking stitch in the gutter says
   it read it. That read finds nothing to ask.
3. Inkling also looks through your other notes, on this Mac, for words this paragraph
   shares with them. "Race calendar" has *Pick up the race bib on Friday 13 March at the
   harbour office.* It is the only close match, so Inkling sends the two paragraphs to the
   model and asks whether they disagree about the same thing.
4. They do. A few seconds later a running stitch sews itself under *Saturday 14 March* and
   a thread hangs to a folded tag, as for any question. The tag shows **?** and its number,
   and a second loop beside the first: this thread goes on to another note. If you are
   typing, it waits, folded, until you pause, as every question does.
5. Unfolded, the tag reads:

   > **Inkling** · between notes &emsp; 1
   > Race calendar says you pick up the bib on Friday 13 March at the harbour office. Which
   > day is right?
   > ┃ *Friday 13 March* (in your own typeface)
   > ┃ Race calendar · edited 3 Sep &emsp; **Go there**

   with the usual reply box (*Write back…*) and Resolve, Dismiss, Make a note and Copy.
6. **Go there** (or ⌥⌘→ with the thread open) opens "Race calendar" scrolled to *Friday
   13 March*. The same thread is stitched under those words, its tag open, with the same
   conversation. Its far-end line now reads *Saturday 14 March* · Race-week plan · today,
   and its own **Go there** takes you back.
7. You fix one of them. Say the calendar was right: you change *Saturday 14 March* to
   *Friday 13 March* in the plan. At the next pause Inkling looks again at both ends, now
   *Friday 13 March* and *Friday 13 March*, and the thread ties itself off with one line:
   "Both now say Friday 13 March." The knot appears in both notes' margins. Fixing the
   calendar instead, or both, ends the same way; each edit is one look, and each look sees
   the other end as it is now.
8. Or you reply, at either end: *The office moved pickup to Saturday this year.* Inkling
   answers in the same thread. If your reply explains why both can stand, it ties off; if
   it says one note is out of date, Inkling says which words to change ("Then the calendar
   still says Friday 13 March") and the thread stays open until the words change, because
   the note is what is left when the margin is hidden (belief 6).
9. Or you decide your words stand and cut the thread at either end (Dismiss, ⇧⌘⌫). It is cut
   in both notes, Undo brings it back for eight seconds (⌥⌘Z), and Inkling never raises
   that pair of paragraphs again.

Words on screen, all of them new:

| Where | Words |
|---|---|
| tag header, both ends | `Inkling · between notes` (the mind's name, then the kind) |
| the far end, inside the tag | the other words, quoted, in the writer's typeface; under them `‹Other title› · edited 3 Sep` (`· today`, `· yesterday`, else the date) |
| the control | `Go there`; its tooltip: "Open ‹Other title› at these words. The thread runs under the cloth to them." |
| the other note was deleted | `‹Other title› · deleted`, the quote kept, no Go there |
| the key | ⌥⌘→ `Go to the other end` (Settings › Shortcuts, group Threads) |
| a tie-off | the model's one line, as for any thread that settles; status `· tied off` |

Nothing else changes: no toast when such a thread arrives, no badge, no count anywhere but
the margin, the open count above the title and the sidebar's dot (belief 3).

## Every state

**Linen and Lamplight.** The thread wears its mind's colour at both ends, from the existing
`mind_thread`. The quoted far words use the page's ink on the tag's card; the title and date
use the faint ink every tag's meta text uses; the seam beside the quote is the thread colour
at the quote block's opacity. No new colour, so the measured contrasts of both palettes
hold. The second loop on the tab is drawn like the first, so the difference is shape, not
colour (rule 4).

**The other note is renamed.** Titles are looked up by note id every time a tag is drawn,
so the far-end line follows the rename. `Elsewhere.title` keeps the last title seen, for the
deleted case and for exports.

**The other note is moved to another folder.** Nothing changes: links are by id.

**The other note is deleted.** The thread stays in its home note, open or not as it was, and
its far-end line says `Race calendar · deleted` with the quote kept and no Go there. Undo
(the eight-second toast) brings the note back and the link with it, because nothing was
removed from the home note. If the *home* note is deleted, the thread goes with it, and the
far end disappears from the other note the next time it opens.

**The other note is edited.** While a note is open, the far ends hanging in it follow edits
like any thread's words (the editor's anchors), and saving the note writes their new range
and words back into the home note. Editing the far words themselves starts the same second
look as editing the near words (step 7). A note edited outside Inkling re-finds the quote
nearest where it was, as threads do today, and an end whose words are gone sits on an empty
range at its old place, as any thread does when its words are deleted.

**Imported Obsidian text that is left alone.** Imported paragraphs are never read (that is
the import's promise: a vault costs no model calls), so they never start a thread between
notes. They can be the far end of one: when you write something new that disagrees with an
imported paragraph, that paragraph can be one of the three passages sent with `relate`. This
is where the months-old notes the manifesto means usually are. The welcome guide's
paragraphs, which are also left alone, are Inkling's words, not yours, and never count.
(Open question 1 asks the owner to confirm.)

**A note that waits to be asked.** It never reads along, so it never starts a thread between
notes on its own. It is also never the far end of one: its words are not searched and never
sent with another note's `relate`. A writer who set a note to wait has asked Inkling to
leave it alone until called, and a stitch appearing in its margin from elsewhere, or its
words leaving the Mac because of another note, would break that. If Read it through is
built, its reads run the same `relate` afterwards, because the writer asked.

**Hidden threads.** Hide threads (⇧⌘.) hides both ends like any thread. Reading and
`relate` go on while hidden, as reads do today.

**Marks only.** Each end is a **?** after its words that opens the tag, as for any thread.

**Export and copy.** With threads, a thread between notes prints under its paragraph in
both notes' exports, with one more line naming the other end: in Markdown
`> *Also in “Race calendar”:* “Friday 13 March”`; in HTML and Word the same line in the
thread's box. Copying the thread from its tag adds the same line. The other note's words
are already in the conversation, so this adds the title and the exact quote and nothing
else from that note.

**Undo.** Dismiss and its Undo work at either end and act on the one thread. ⌘Z in the
editor undoes text, never a thread's arrival or tie-off, as today. Deleting a note and its
Undo are covered above.

**Several minds.** The mind that asks is the home note's mind. Its colour is the same in
both notes, so in a note whose own mind is someone else the thread still shows who asked.
A reply at either end is answered by the thread's mind (`Thread.mind_name`), not by the
mind of the note you happen to be in. Ask the room does not look across notes.

**Busy states.** One `relate` at a time per page, separate from the one read; a paragraph
finished while one is in flight waits for it (only the newest waits). Switching notes
cancels it, like a read. `settle` in scripts waits for it.

**Failures.** A failed `relate` says so in the usual ochre toast, unless it was cancelled.
It is not retried: the paragraph was read, and the next paragraph gets its own look.

**Caps.** At most one thread between notes per paragraph read, and no new one while a note
already has three open (counting ends that visit it), so a bad match cannot flood a margin.

## The brain

### Retrieval: the options, weighed

| | Cost per pause | Latency | Recall | False alarms | Verdict |
|---|---|---|---|---|---|
| Send the whole notebook | the notebook, every pause | slow | full | many | ruled out by the brief |
| Periodic background pass over pairs of notes | grows with the square of the notebook, while you are away | none at the pause | good | many, and it speaks about notes you are not on (belief 3) | no |
| Embeddings | an outside embedding service (Claude has none) or a local model of 30 to 100 MB | small | best on paraphrase | depends on the gate | not now: a download or a third party for every note's words |
| A claim index written by the model as it reads | about 30 more output tokens on every read, and a backfill pass for old and imported notes (hundreds of calls) | none | good on numbers and dates; misses everything not yet read | low | no: imported and older notes, the main case, are missing until paid for |
| **Local lexical index (chosen)** | 0 tokens; a few ms | none | good when the two notes name the same thing, which is what a contradiction or a dependency between notes usually does; misses pure paraphrase | low with a strict gate and a strict prompt | **yes** |

The chosen index is plain words with IDF weighting, plus a small boost when both paragraphs
carry a value of the same kind (a date, an amount, a time, a count with a unit). It is local,
free, deterministic and testable without a model. Its weakness is paraphrase (*store
customer IPs* against *log the source address*). If the owner's notebook shows real misses
of that kind, Apple's on-device sentence embeddings (`NLEmbedding`) are the next step, behind
the same gate and measured on held-out pairs first.

### The index

- **What it holds.** One entry per unit of every eligible note: a paragraph, list item,
  checklist item, quote or callout line with at least four words. Headings are not units;
  the nearest heading above a unit goes with it as context. Code blocks are left out, and
  so are lines that begin with a mind's name and a colon (what Make a note writes for
  Inkling).
- **Eligible notes.** Every note that reads along, except the welcome guide (`first_run`, or
  titled "Start here" or "Welcome to Inkling" with paragraphs left alone). Never a note that
  waits to be asked.
- **Terms.** Lower case, split on anything that is not a letter or a digit, common English
  words dropped, light stemming (plural *s*, *es*, *ing*, *ed*). Numbers stay as terms.
- **Where it lives.** In memory, in a thread-local `between::Index` beside the store. Nothing
  is written to disk, so there is nothing to migrate, back up or find stale.
- **How it is rebuilt.** Lazily, at each look: every note whose `updated_at` or mode changed
  since it was indexed is indexed again, and notes gone from the store are dropped. The first
  look in a session indexes the whole notebook (tens of milliseconds for thousands of notes
  in a release build); after that a look only re-indexes the notes saved since.
- **Scoring and the gate.** Each unit in another eligible note gets the cosine similarity
  between its TF-IDF vector and the paragraph's, plus 0.04 for each kind of value both carry
  (at most 0.08). A unit passes when it shares at least two terms that are uncommon in the
  notebook (an IDF of at least 1.0), its score is at least 0.20, and it is not already the
  far end of a thread on this note (any status: open, tied off or cut). The best three that
  pass go, at most two from one note. If none pass, there is no model call.
- **The passage.** The unit's words, cut to at most 600 characters around the first shared
  term, on character boundaries, with an ellipsis where cut.

### The prompt

`relate` reuses the system prompt every task has (the mind, its voice and how often it
speaks, the brief, About you, today's date). It does not send the rest of the note: only the
paragraph, up to two paragraphs before it, and the passages. The rest of the note is the
read's business, and the read has just run. The prompt (`relatePrompt`):

```text
The writer just finished this paragraph in their note «Race-week plan»:
<paragraph>I'll pick up my bib on Saturday 14 March, after the shakeout run.</paragraph>
What comes just before it in that note:
<before>Race week is about sleep and nothing new.</before>

A search of the writer's other notes found these passages. The search only matched words, so
most of them are not related in any way that matters.
<passage n="1" note="Race calendar" written="Thursday 3 September 2026" edited="Thursday 3 September 2026" heading="Spring">Pick up the race bib on Friday 13 March at the harbour office.</passage>

Already in the margin about this paragraph:            (only when there is something)
- «...»: ...

Decide whether the paragraph and one of these passages disagree about the same thing. Most
of the time they do not; then relation is "none".
- same_subject: true only when both are about the very same thing (the same event, plan,
  number, person or decision), not merely the same words or topic.
- relation: "contradicts" when, read as written, they cannot both be true now (a different
  date, amount, count, name or choice for the same thing); "depends" when the paragraph
  builds on what the passage says and does not match it; otherwise "none". A plain change
  over time (an old goal, last year's numbers, an earlier draft the writer has moved on
  from) is "none", unless the paragraph treats the old value as still true.
If relation is not "none":
- passage: the number of that passage.
- quote: the exact words from the paragraph, character for character, the shortest span
  that names the clash.
- other: the exact words from the passage, character for character, the shortest span that
  names the clash.
- text: your margin note, at most 40 words. Say what the other note says, naming it by its
  title in plain words (no quote marks around the title), and end with one question that
  lets the writer decide. Do not say which one is right: you cannot know.
Otherwise set passage to 0 and quote, other and text to empty strings.
Never raise anything else about the paragraph here, and never repeat a point already in the
margin.
```

The schema, in the order the model fills it, so it judges the subject before it writes:

```ts
export const RELATE_SCHEMA = {
  type: "object",
  additionalProperties: false,
  required: ["same_subject", "relation", "passage", "quote", "other", "text", "repeats"],
  properties: {
    same_subject: { type: "boolean" },
    relation: { type: "string", enum: ["none", "contradicts", "depends"] },
    passage: { type: "integer" },
    quote: { type: "string" },
    other: { type: "string" },
    text: { type: "string" },
    repeats: { type: "integer" },
  },
} as const;
```

`repeats` is Let it stand's field, last as in the reads' schemas: the number of a point the
writer let stand that this note would make again (the system prompt lists them), or 0.
The brain then holds the thread back, as it does a read's.

Replies and second looks on a thread between notes go through the existing `reply` task with
the other end attached (below). The reply prompt then says, after the thread's history:

```text
This thread also quotes the writer's other note «Race calendar» (last edited Thursday 3
September 2026): «Friday 13 March», in this passage:
<passage>Pick up the race bib on Friday 13 March at the harbour office.</passage>
```

(or, when that note is gone: `This thread also quoted «Friday 13 March» from the writer's
note «Race calendar», which has since been deleted.`), and its `resolved` rule gains one
sentence: "For a thread between two notes, resolved is true when the two no longer
disagree, because either was changed, or when the writer explains why both can stand."
Every other prompt's margin list names the far end: `Thread 2 on «Saturday 14 March», which
also quotes «Friday 13 March» in the note «Race calendar»:`.

### Guarding against false alarms

A reader that is shown two passages and asked whether they clash will often find a clash.
Six guards, cheapest first:

1. **The gate** sends nothing unless a passage shares at least two uncommon words and scores
   at least 0.20. One shared word, however rare, is never enough.
2. **At most three passages**, the strongest, so the model never wades through near misses.
3. **The prompt says so**: the passages were found by matching words, most are unrelated,
   and silence is the usual answer.
4. **The schema makes it judge the subject first** (`same_subject`), and the brain speaks
   only when `same_subject` is true and `relation` is not `none`.
5. **Both quotes must be real.** The near quote snaps onto the paragraph as reads do. The far
   quote must be found in the passage (exactly, or loosely: case, spaces, curly quotes,
   dashes); if it is not, the brain stays silent rather than guess where it points.
6. **Never twice.** A pair already joined by a thread, open, tied off or cut, is never sent
   again, and a note with three open threads between notes gets no new one.

The real-model check below measures what is left.

### Cost per read, in tokens

Measured on 7 Nov 2026 with Opus 5.5 at low effort, the read's settings, through the real
brain (`brain/test/live.ts`, eight calls); the SDK reports the counts, which the brain now
passes on in `Usage`:

| Call | Input tokens | Output tokens | SDK's cost estimate |
|---|---|---|---|
| `relate`, first call of a session | 2,266 (all written to the cache) | 233 | $0.023 |
| `relate`, later calls | about 2,200: 920 read from the cache (the runtime's own prefix), 1,250 to 1,310 new | 160 silent, 230 to 300 speaking | $0.013 to $0.017 |
| a `read` of the same paragraph in a two-paragraph note, for comparison | 1,925: 897 from the cache, 1,026 new | 335 | $0.015 |

So one `relate` costs about what one read of a short note costs: the runtime's fixed prefix
is most of it, and the passages are small. Retrieval and the gate cost nothing. A pause
costs a `relate` only when the gate passes: never in a notebook with one note, and in a
notebook of a few hundred notes on overlapping subjects perhaps one paragraph in ten,
which makes reading along about 10 percent dearer on average. The estimate before
measuring was 1,000 to 1,500 input tokens; the difference is the runtime's prefix of about
900 tokens, which every task pays and which is read from the cache after the first call.
Latency: `relate` starts when the read ends and took 2.4 to 4.6 s (11.6 s for the first call
of a session, which starts the runtime), so a thread between notes lands about 5 to 10 s
after the pause.

**False alarms, measured.** Five cases, one call each: the bib dates in two notes (spoke,
both quotes exact), a budget a paragraph relies on with a different figure (spoke, `£300`
against `£120`), two different races that share *city 10k*, *Sunday* and *March* (silent),
last year's goal against this year's (silent), and the same date in both notes (silent).
No false alarms in three chances, and both real clashes found. A first version of the
budget case said *this month* rather than *March*; the model stayed silent, rightly, since
the budget note is about March. Too few calls to be a rate; enough to show the guards hold
on the obvious traps.

## The contract

### Store types (`src/store.rs`)

`Thread` does not change. A thread between notes is an ordinary `Thread` in its home note
with an `Elsewhere` entry beside it in the same note:

```rust
/// How the two ends of a thread between notes stand.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// Read as written, they cannot both be true.
    #[default]
    Contradicts,
    /// These words build on the other words and do not match them.
    Depends,
}

/// The far end of a thread between notes: words in another note that the thread `thread`
/// of this note also quotes. Kept beside the threads, so a thread's own shape does not
/// change. Every field defaults, so a hand-edited or newer file still loads.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Elsewhere {
    /// The thread in this note's `threads` it belongs to.
    pub thread: String,
    /// The other note's id.
    pub note: String,
    /// Byte range of the other words in that note's joined text (blocks joined by '\n',
    /// title included), and the words themselves.
    pub start: usize,
    pub end: usize,
    pub quote: String,
    /// That note's title and its `updated_at`, as last seen: shown once it is deleted,
    /// and used by exports.
    pub title: String,
    pub edited: DateTime<Utc>,
    pub relation: Relation,
    /// Where the far end's tag hangs in the other note, and where it was dragged to there.
    pub side: Side,
    pub tag_offset: (f32, f32),
    /// Only on a visiting end (see below): never saved.
    #[serde(skip)]
    pub visiting: bool,
    /// The far end has been shown in its note, so `side` and `tag_offset` are its own from
    /// now on. While it is false, the app picks the far end's side the first time it lays
    /// it out and sets `placed` on the visiting end; the next save carries side, offset and
    /// `placed` home. (Without it, "Right at (0, 0)" could not tell a default from a choice.)
    pub placed: bool,
}
```

On `Note`, a last field:

```rust
    /// The far ends of this note's threads between notes, one per such thread.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elsewhere: Vec<Elsewhere>,
```

and two helpers, `Note::elsewhere_of(&self, thread: &str) -> Option<&Elsewhere>` and
`Note::elsewhere_of_mut`. A note without links writes no `elsewhere` key, so files, fixtures
and other agents' tests that compare JSON are unchanged. An old note has no key and loads
with none; an older build reading a new note ignores the key and shows the thread as an
ordinary thread on the home words. Loading drops an entry whose `thread` is not in the
note, whose `note` is empty, or that points at the note itself (`drop_broken_links`, beside
`drop_broken_marks`).

### Visiting ends: how one thread shows in two notes

When a note opens, each thread from another note whose `Elsewhere.note` is this note is
added to the page's copy of the note as a **visiting end**: a clone of the home thread (same
id, messages, status, kind, mind, drawing, and any field other features add to `Thread`)
with `start`, `end`, `quote`, `side` and `tag_offset` taken from the far end; and an
`Elsewhere` with `visiting: true` that points back at the home note's words (`note` = home
id, its range, quote, title, edited, the home thread's side and offset, and the far end's
`placed`). Dismissed threads
come too, so Turn it over and Let it stand see them; the margin hides them as it hides any
dismissed thread.

Everything that already works on `note.threads` (anchors that follow edits, the margin,
numbers, the gutter knot, jumping, Hide threads, Marks only, the open count) then works on
the far end with no change. When the page saves, the visiting ends are taken out of what is
saved for this note, and each one's state goes home: the far end's range, quote, side and
offset into the home note's `Elsewhere`, and everything else (status, messages, drawing,
any new field) into the home thread. Writing a far end home sets its `placed`; a visiting
end the app has just placed goes home even when its side is the one it had by default. A
home note is written to disk only if it changed.
So status and conversation have one source, the home note, and either end can change them.

Rule for everyone: **build notebook-wide lists from `Store.notes`** (each thread once, in its
home), never from the page's `note`, which holds visiting ends while a note is open; flush
the page first, as opening a note does.

Functions (`src/between.rs`, brain-and-data agent). They take the notebook's notes as
`&store.notes`, so they can be tested without a store on disk:

| Function | What it does |
|---|---|
| `add_visitors(note: &mut Note, notes: &[Note])` | add the visiting ends, re-finding a far quote nearest its stored place when the range no longer matches (called in `load_into_editor`) |
| `save_visitors_home(note: &Note, store: &mut Store) -> Note` | write visiting ends home (saving changed home notes) and return the note without them, to be saved; also refreshes `title` and `edited` on this note's own entries (called in `flush`) |
| `send_home(note, notes: &mut [Note]) -> (Note, Vec<String>)` | the same without saving: the saved note and the ids of the home notes that changed |
| `without_visitors(note) -> Note` | the note as it is saved |
| `visiting(notes, note_id) -> Vec<(&Note, &Thread, &Elsewhere)>` | the threads whose far end is in `note_id` |
| `other_note(notes, e: &Elsewhere) -> Option<&Note>` | the note at the other end, `None` once deleted |
| `open_here(notes, note) -> usize` | open threads on a note, its own plus visiting: the sidebar's count |
| `elsewhere_wire(notes, note, thread) -> Option<Value>` | the `elsewhere` object for a `reply` (below), from the other end as it is now |
| `summary_wire(note, thread) -> Value` | `{noteTitle, quote}` or null, for `ctx.threads[i].elsewhere` |
| `locate_other(note, block, quote) -> Option<(Range, String)>` | where a new far end goes: the words in their block, else nearest it, else the whole block |
| `with_index(f)`, `Index::passages(&mut self, notes, minds, here: &Note, target) -> Vec<Passage>` | retrieval and the gate |

### Protocol (`brain/src/protocol.ts`)

A new task:

```ts
export interface Passage {
  note: string;            // the other note's id: echoed back, never put in the prompt
  noteTitle: string;
  written: string;         // that note's created_at as a date: "Thursday 3 September 2026"
  edited: string;          // its updated_at, the same way
  heading: string | null;  // the nearest heading above the passage
  text: string;            // the passage, at most 600 characters
}

| { task: "relate"; ctx: Context; target: string; before: string[]; passages: Passage[] }
```

`target` is the paragraph just read (trimmed, as for `read`); `before` is up to two
paragraphs before it, nearest last. The answer, `done.data`, is `{ "speak": false }` or:

```json
{
  "speak": true,
  "passage": 0,
  "note": "00000000-0000-4000-8000-0000000000e1",
  "quote": "Saturday 14 March",
  "other": "Friday 13 March",
  "relation": "contradicts",
  "text": "Race calendar says you pick up the bib on Friday 13 March at the harbour office. Which day is right?"
}
```

`passage` is the 0-based index into the request's `passages`; `note` is that passage's
`note`; `quote` is always a substring of `target` (the whole of it if nothing matched);
`other` is always a substring of `passages[passage].text` (otherwise the answer is
`speak: false`); `relation` is `"contradicts"` or `"depends"`; `text` is not empty. A clash
that would make a point the writer let stand again comes back as
`{ "speak": false, "repeats": n }`, as a read does. With no
passages the brain answers `{ "speak": false }` without a model call. It ends in exactly one
`done` or `error`, like every task. Effort: the read's (`heavy: false`), and the read's 120 s
timeout.

Two optional additions, both ignored by an older brain:

```ts
export interface ElsewhereEnd {
  noteTitle: string;
  edited: string;           // as Passage.edited; empty when the note is gone
  quote: string;            // the other end's words now
  text: string | null;      // its paragraph now (at most 600 characters), null when gone
  relation: "contradicts" | "depends";
  gone?: boolean;           // the other note was deleted
}

| { task: "reply"; ctx: Context; quote: string; messages: ThreadMessage[]; message: string; elsewhere?: ElsewhereEnd }

export interface ThreadSummary {
  quote: string;
  messages: ThreadMessage[];
  resolved: boolean;
  elsewhere?: { noteTitle: string; quote: string } | null;
}
```

`Usage` gains `inputTokens?`, `outputTokens?`, `cacheReadTokens?` and `cacheWriteTokens?`
from the SDK's result, so cost can be measured.

### Fake brain (`brain/test/e2e/fake-brain.ts`)

`relate` is silent unless its `target` carries `[[elsewhere]]`; then it speaks about passage
1, or `[[passage:N]]` (1-based). `[[quote:TEXT]]` and `[[other:TEXT]]` pick the two quotes
(snapped onto the paragraph and the passage); by default each is the first three words.
`[[depends]]` sets the relation. The text is `[r7] Between notes: “OTHER” in TITLE?`. Errors,
delays and hangs use the usual directives, scoped with `relate>`. With no passages it is
silent. Existing scenarios never contain `[[elsewhere]]`, so a stray `relate` stays silent
and adds no thread.

### The page, and who builds what

| Part | Brain and data agent | App agent |
|---|---|---|
| `brain/src/*`: task, prompts, schema, shaping, usage | yes | |
| fake brain, brain tests, `live.ts` checks | yes | |
| `src/store.rs`: types, helpers, `drop_broken_links`, tests | yes | |
| `src/between.rs`: index, gate, passages, visiting ends, wire builders, tests | yes | |
| `src/note_page/between.rs` (new child module): after a read, `look_elsewhere` sends `relate` and opens the thread with its `Elsewhere`; the `relating` slot | yes | |
| hooks in `src/note_page.rs`: `mod between;`, two fields at the end of `NotePage`, `read_line`'s Done branch, `settled`, `tracks`, `open` (cancel), `load_into_editor` (`add_visitors`), `flush` (`save_visitors_home`), `context_with` (`summary_wire`), `debug_state` (`state["between"]`) | yes | |
| `src/export.rs`: the far-end line in exports | yes | |
| fixture `between`, scenarios for reading and saving | yes | |
| the tag: `between notes` label, second loop, far-end block, `Go there`, deleted state | | yes |
| following: `Go there`, ⌥⌘→ (`FollowThread` in `app.rs` and `shortcuts.rs`), opening the other note at its words with the tag open, script step `follow: N` | | yes |
| tying off: `elsewhere_wire` in `send_reply` and `recheck`, replies answered by the thread's mind, side of a far end on first show | | yes |
| sidebar count (`open_here`), Copy from the tag, dump fields for the far end, scenarios for following and tying off, screenshots | | yes |

The app agent puts its page code in its own child module (`src/note_page/ends.rs`), so the
two never edit the same new file. Both add fields at the end of structs.

Dump fields the brain-and-data side adds: `d.relating` (`{req, line}` or null) and
`d.between`: one object per `Elsewhere` on the page's note,
`{thread, note, title, quote, start, end, relation, visiting, placed, gone}` (the thread id
cut to 8 characters like the others; the note id whole, since fixture ids differ only at
the end).

## Privacy

Everything stays on the Mac except what goes to Claude, as today. This feature sends:

- With each `relate`: the system prompt every task sends (the mind, the brief, About you,
  today's date), the paragraph just read and up to two paragraphs before it, and for each of
  at most three passages: the other note's title, its two dates, the heading above the
  passage and the passage itself, cut to 600 characters. Never another note's whole text,
  brief, threads or chat. The other note's id goes to the brain process but not to the
  model.
- With each reply or second look on a thread between notes: what a reply sends today, plus
  the other end's title, date, words and paragraph (600 characters at most).
- In every task's margin list, for each thread between notes on the note: the other note's
  title and the quoted words.
- Only notes that read along are ever searched or sent. A note that waits to be asked, the
  welcome guide, code blocks and Inkling's own words are never sent this way.

The index never leaves memory and is never written to disk. The new data on disk is the
`elsewhere` list in a note's file, which holds the other end's id, words, title and date.

## Fit with the features designed in parallel

Shared types this feature changes: `Note` (one field at the end, `elsewhere`), two new types
(`Elsewhere`, `Relation`) in `store.rs`; `Thread` is unchanged. On the wire: the new `relate`
task, `Passage`, the optional `elsewhere` on `reply` and on `ThreadSummary`, and token counts
on `Usage`. `PageEvent` is unchanged (following uses a field on the page, not a new event).

- **Answer in the note, Write it in.** They change what a `read` returns and when a thread
  ties off inside one note; `relate` is a separate task started after the read, so the two
  do not touch the same prompt. A thread between notes can be answered in either note's
  words; their check, if it reads `ctx.threads`, sees the far end named.
- **Let it stand, Turn it over.** A cut thread between notes is one thread, cut at both ends;
  a reason given at either end is stored on the home thread (any field they add to `Thread`
  travels with the visiting end, because a visiting end is a clone). The back of a note
  lists the threads that visit it, labelled with the home note's title from
  `elsewhere_of(id)`; built from the store, it should also take `visiting(store, id)`.
- **Threads of your own.** Not linked across notes in this version.
- **Show its working.** A thread between notes always shows how it knows: the other note's
  exact words. It is always a question, never a correction, since Inkling cannot know which
  note is right.
- **Loose ends.** Built from `Store.notes`, each thread between notes appears once, under
  its home note; show the other title after it (`and Race calendar`), and go to the home
  words on click.

## Tests

**Brain unit** (`brain/test/unit.test.ts`): the far-quote finder returns null when nothing
matches and the passage's own words when something does; `relatePrompt` lists every passage
with its title and dates and the warning that the search matched words; `replyPrompt` with
and without `elsewhere`, and the deleted wording; `threadsBlock` names a far end.

**Brain protocol** (`brain/test/protocol.test.ts`, fake SDK): `relate` that speaks returns
the 0-based `passage`, the echoed `note`, both quotes as exact substrings and the relation;
it is silent when `same_subject` is false, when `relation` is `none`, when `passage` is out of
range, when `other` is not in the passage, and when `text` is empty; with no passages it makes
no SDK call; a `reply` with `elsewhere` reaches the prompt. Parity covers the new task name in
protocol, app and fake brain.

**App unit** (`cargo test between`, `cargo test store`):

- the index finds the clashing paragraph among distractors that share one word; nothing
  passes for one shared word; the current note, waiting notes, the welcome guide, code,
  short lines and mind-written lines are never passages; at most three, at most two from a
  note; a pair already joined (open, tied off or cut) is skipped; a note saved again is
  re-indexed and a deleted one dropped; long passages are cut on character boundaries;
- an old note without `elsewhere` loads; a note with an `Elsewhere` round-trips; a broken
  entry is dropped on load; a note with no links writes no `elsewhere` key;
- `add_visitors` then `save_visitors_home` round-trips status, messages, the far range and
  offset, never leaves a visiting end in the saved note, and only writes home notes that
  changed; a far quote whose range went stale is re-found nearest its place;
- export prints the far-end line in Markdown, HTML and Word.

**Scenarios** (fixture `between`: "Race calendar", "Race-week plan" with thread `b1` already
joining *Sunday 22 March* to the calendar's *Sunday 15 March*, a distractor "March budget",
and "Journal", which waits to be asked and shares the bib words):

```text
# t16-between-read: a paragraph that disagrees with another note gets one thread quoting both
# fixture: between
# tags: between
wait: 1500
switch: Race-week plan
caret: end
type: I'll pick up my bib on Saturday 14 March, after the shakeout run. [[read>silent]] [[elsewhere]] [[quote:Saturday 14 March]] [[other:Friday 13 March]]
settle
dump: linked
expect: tasks.join() === "read,relate"
expect: requests[1].input.passages[0].noteTitle === "Race calendar"
expect: requests[1].input.passages.every(p => p.noteTitle !== "Journal" && p.noteTitle !== "March budget")   // waiting and distractor notes are never sent
expect: d.between.some(e => e.quote === "Friday 13 March" && e.title === "Race calendar" && !e.visiting)
expect: d.relating === null
quit
expect: note("Race-week plan").elsewhere.some(e => e.quote === "Friday 13 March" && e.note === note("Race calendar").id)
expect: rangeText(note("Race-week plan"), note("Race-week plan").threads.find(t => t.quote === "Saturday 14 March")) === "Saturday 14 March"

# t17-between-save: a visiting end follows edits in its note and goes home; it is never saved there
# fixture: between
# env: FAKE_READ_DEFAULT=silent
switch: Race calendar
settle
dump: visited
expect: d.between.some(e => e.thread === "b1" && e.visiting) && d.threads.some(t => t.id === "b1")
edit: City 10k: => The city 10k:
settle
switch: Race-week plan
settle
quit
expect: !note("Race calendar").threads.some(t => t.id === "b1") && !("elsewhere" in note("Race calendar"))
expect: note("Race-week plan").elsewhere.find(e => e.thread === "b1").start === textOf(note("Race calendar")).indexOf("Sunday 15 March")

# t18-between-never-twice: a pair already joined is not sent again; a cut one neither
# fixture: between
switch: Race-week plan
dismiss: 1
caret: end
type: The city 10k on Sunday 22 March starts at 9:00 by the harbour. [[read>silent]] [[elsewhere]]
settle
quit
expect: !requests.some(r => r.input.task === "relate" && r.input.passages.some(p => p.text.includes("Sunday 15 March")))
```

App agent's scenarios (same fixture): `follow: 1` from "Race-week plan" opens "Race calendar"
with `b1` active and its words in view; editing the far words with `[[resolved]]` ties `b1`
off and the home note's file says `resolved`; dismiss at the far end, `undo-dismiss`, quit:
the home thread is open; the far note deleted shows `gone` in the dump; shots in Linen and
Lamplight of both ends open.

**Real-model check** (`brain/test/live.ts`, run by hand, one call each): `relate_clash` (the
bib dates: must speak, both quotes exact), `relate_depends` (a paragraph that relies on the
budget note's figure and gets it wrong: must speak), `relate_same_words` (two different
races that share *city 10k*, *Sunday* and *March*: must stay silent), `relate_moved_on`
(last year's goal against this year's: silent), `relate_agrees` (the same date in both:
silent), and `relate_read_compare`, a `read` of the clash paragraph to compare token counts.
A speak on any of the three silent cases is a false alarm. Run on 7 Nov 2026: all six pass
(results under "Cost per read" above).

## Build order

1. Store types and their tests: everything after depends on them.
2. `src/between.rs` index and gate, with unit tests.
3. Brain: protocol, prompt, schema, shaping, fake brain, tests, parity.
4. The page: `look_elsewhere` after a read, the `relating` slot, `debug_state`; fixture
   `between`; scenario t16. **End to end here:** a thread between notes appears on a real
   page with the fake brain, stored with its far end.
5. Visiting ends in `load_into_editor` and `flush`; scenarios t17 and t18. The far end now
   shows in the other note as an ordinary thread, and the app agent's display work has
   real data to draw.
6. `elsewhere_wire` and `summary_wire`; export's far-end line.
7. The real-model check, then the token counts into this document.

## Files and functions

| File | What changes | Lines |
|---|---|---|
| `src/store.rs` | `Relation`, `Elsewhere`, `Note.elsewhere`, `elsewhere_of(_mut)`, `drop_broken_links`, tests | 120 |
| `src/between.rs` (new) | `Index`, `with_index`, `Passage`, `passages_wire`, terms and values, the gate, `add_visitors`, `save_visitors_home`, `visiting`, `other_note`, `open_here`, `elsewhere_wire`, `summary_wire`, tests | 700 |
| `src/note_page/between.rs` (new) | `look_elsewhere`, `send_relate`, `relate_landed`, `between_state` | 160 |
| `src/note_page.rs` | the hooks listed above | 30 |
| `src/export.rs` | the far-end line, tests | 40 |
| `brain/src/protocol.ts`, `main.ts`, `prompts.ts`, `output.ts`, `claude.ts` | `relate`, `Passage`, `ElsewhereEnd`, the prompt and schema, the far-quote finder, token counts | 200 |
| `brain/test/*` | fake brain `relate`, unit and protocol tests, live checks | 300 |
| fixture `between`, three scenarios | | 150 |
| `AGENTS.md`, `design/capabilities.md` | the map and the capability | 10 |

Size: **L**, as the manifesto said: about a week for both halves together, four to five days
for this half. The app half (display, following, tying off) is about as large again, mostly
in the tag and in following across notes.

## Open questions for the owner

1. **Should imported Obsidian text count as the other end?** Recommendation: yes. The
   import promises that a vault costs no model calls and that its words are left unread,
   and this keeps both promises for the import itself: imported words never start a
   thread, and they are sent only when something you write later shares their subject.
   Without them, the notes written months ago, which this feature is for, would mostly be
   out of reach. If the answer is no, one line in the eligibility rule leaves them out.

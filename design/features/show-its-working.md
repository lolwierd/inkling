# Show its working

Manifesto, section 2, item 4: a correction always carries a check you can do yourself,
either the arithmetic or a source you can open. When Inkling has neither, it asks a question
instead.

## The problem

Today a correction (the × tag) is one sentence from the model. Nothing shows how it knows.
Models slip on arithmetic, misread which number means what, and misremember facts, and the
sentence sounds just as sure either way. A confident correction that is wrong can talk you
out of a sentence that was right.

## What the brain can reach (checked 7 Nov 2026)

- **No sources at all today.** `brain/src/claude.ts` `baseOptions` passes `tools: []` (every
  built-in tool off), `settingSources: []`, `maxTurns: 1` (3 for structured answers, used
  only for schema retries) and `persistSession: false`. The model sees the prompt and its
  memory, nothing else. Any URL it wrote would come from memory: unverified, possibly made up.
- **What the SDK could add.** Agent SDK 0.3.318 has built-in `WebSearch` (returns result
  titles and URLs; page snippets only reach a PostToolUse hook) and `WebFetch` (returns a
  small model's summary of a page, not the page's text). Turning them on means
  `tools: ["WebSearch", "WebFetch"]`, permission for them (`allowedTools` or `canUseTool`),
  and more turns. The brain could then check that a cited URL came back from the search in
  the same call and fetch the page itself to find the quoted passage.
- **What a read costs now.** Measured on 6 Nov (`design/video/data/real.json`, Opus 5.5 at
  low effort, notes of 300 to 600 characters): $0.024 to $0.034 per read and 3.8 to 6.0 s.
  The session's first call was $0.85. These are the SDK's estimates at list price (Opus 5.5:
  $4 per million input tokens, $20 per million output); on the owner's subscription they
  come out of the plan's limits, not a bill.
- **What search would cost.** At least one more model turn, the results read as input
  (thousands of tokens), $10 per 1,000 searches at list price, and several seconds. That
  is roughly two to three times a read. Fetching is not charged per call, only in tokens.
- **What search would send out.** Today a read sends the note to Anthropic and nowhere
  else. A search sends a query made from the note to a search provider, and a fetch asks a
  website for a page from this Mac. Belief 8 makes that the owner's call.

So this build has no web sources. A correction must carry one of two checks that can be
verified on this Mac: **a sum** made from the writer's own numbers, or **the writer's own
words** elsewhere in the note. A correction that rests only on the model's memory becomes a
question, which can still say what Inkling remembers. The data model leaves room for a
source later (open question 1).

## What the writer sees

The check sits in the tag directly under the correction it backs, in the hover-unfolded
tag and the open one, behind a stitched seam on its left (a dashed line in the thread's
colour, like a quote's seam in the note). Its text is the interface face (Figtree) with
tabular figures, in graphite; its labels are faint.

A correction with a sum (the running plan):

```
Inkling · correction                                          1
At 5:30 a km, 10 km takes 55:00. Under 50 needs 5:00 a km or
faster. Which do you want to change: the pace or the goal?
┆ Working                                              Disagree
┆      5:30 × 10 = 55:00
┆     50:00 ÷ 10 = 5:00
┆ From your words: “hold 5:30” · “10k” · “under 50 minutes”
```

- `Working` is the label. Each line is set like a sum: the expression right-aligned in its
  column, then `=`, then the result. A line whose result is rounded shows `≈` instead.
- `From your words:` lists the words each number was taken from, in curly quotes,
  separated by ` · `. Each is a link: click it and the note scrolls to those words and
  selects them (tooltip: "Show these words in the note"). Words that are no longer in the
  note show ` (changed)` after them and are not links (tooltip: "These words are no longer
  in the note").
- `Disagree` (a quiet text button, only in the open tag) focuses the reply box with the
  placeholder "What doesn't hold?" Tooltip: "Say what's wrong with the working. If Inkling
  agrees, it takes the correction back."

A correction with your words:

```
┆ In your note                                         Disagree
┆ “run four days a week”
```

The passage is a link, as above.

**How you check it yourself.** For a sum: redo each line, in your head or in Calculator.
Every number in it is one you wrote (click it to see where), the result of a line above,
or a unit everyone knows (60, 24, 7, 12, 52, 365, 100, 1000, and 1 and 2). The brain has
already redone every line before the tag appeared, so a sum that does not add up never
reaches the page; what is left for you is whether your numbers mean what Inkling took them
to mean, which is where a model is most likely to be wrong. For your words: click the
passage, read the two places side by side, and decide which one you meant.

## Every state

| State | What shows |
|---|---|
| Linen, Lamplight | Theme tokens only: graphite lines, faint labels, the seam in the thread colour at 45%: the same tokens the tag already uses for your replies and its labels. |
| Both margins, Right margin | The same tag. At the narrowest tag (200 px) a long expression wraps inside its column; the `=` and result stay together. |
| Marks only | The × after the words opens the same tag beside them, working included. |
| Folded tab | Unchanged: the × and the number. A folded tag stays quiet. |
| Hidden threads (⇧⌘.) | Nothing: the working is margin content and hides with it. |
| A correction the brain could not back | It arrives as a question (?), in the model's own fallback question. Nothing marks it as a demoted correction. |
| Tied off by fixing the words | As today. Opened again, the working is the record of the old claim; its words show ` (changed)`. |
| Disagree, and Inkling agrees | Your reply shows as usual. Inkling answers in one sentence, the thread ties off, the header reads `Inkling · taken back` instead of `· tied off`, and the working is struck through in faint. |
| Disagree, and Inkling does not | It says where you went wrong; the thread stays open as a correction. Dismiss (or Let it stand) is still yours. |
| Old notes | Corrections made before this feature have no check and show as they always did. |
| Copy (the tag's copy button) | After the correction: `Working: 5:30 × 10 = 55:00 · 50:00 ÷ 10 = 5:00 (from your words: “hold 5:30”, “10k”, “under 50 minutes”)`, or `In your note: “run four days a week”`. |
| Export, Markdown | `> *Working:* …` (same words) under the correction; taken back: `> *Working, taken back:* ~~…~~`. |
| Export, HTML and Word | `<p class="working">Working: …</p>`, in `<s>` when taken back. |

## The brain

### Prompt

`readPrompt` and `probePrompt` change their `kind` line and gain three:

```
- kind: "correction" only when something is plainly wrong and check shows how you know; otherwise "question".
- check (corrections only): how the writer can check it themselves. Either
  sum: the arithmetic, 1 to 4 lines, each {"sum": "5:30 × 10", "is": "55:00"}: numbers, durations (5:30, 1:05:00) and percentages with + − × ÷ and brackets, no units or words. Every number in a sum is one of givens, an earlier line's "is", or 1, 2, 7, 12, 24, 52, 60, 100, 365 or 1000; with
  givens: each number you took from the writer, {"value": "5:30", "words": "hold 5:30"}, words being their exact words that contain it (1 to 6 words);
  or words: 1 or 2 passages copied exactly from elsewhere in the note that this paragraph contradicts.
  Every number in text is the writer's or is in your sum: a figure you worked out (a pace, a total, a gap) gets its own line, e.g. {"sum": "50:00 ÷ 10", "is": "5:00"}.
- question (corrections only): what to ask instead if your check does not hold up, at most 45 words.
You cannot open any source from here: no web, no books, no files. A fact that rests only on what you remember is a question, not a correction; say what you remember inside the question.
```

### Output schema

`THOUGHT_SCHEMA` and `PROBE_SCHEMA` gain two optional properties (not in `required`, so a
question or a silence spends no tokens on them):

```ts
check: {
  type: "object", additionalProperties: false,
  properties: {
    sum: { type: "array", items: { type: "object", additionalProperties: false, required: ["sum", "is"],
           properties: { sum: { type: "string" }, is: { type: "string" } } } },
    givens: { type: "array", items: { type: "object", additionalProperties: false, required: ["value", "words"],
              properties: { value: { type: "string" }, words: { type: "string" } } } },
    words: { type: "array", items: { type: "string" } },
  },
},
question: { type: "string" },
```

A `done` for a backed correction carries `check: {sum: [{sum, is, rounded}], givens:
[{value, words}], words: [string]}`, with every quoted string snapped onto the writer's own
characters. Questions carry no `check`.

### Validation: no correction reaches the page without its check

`brain/src/working.ts` (new) checks every correction from `read` and `probe` before it
leaves the brain. The fake brain imports the same function, so the scenarios go through it.

1. **The sum adds up.** Each line parses (numbers with optional thousands commas,
   `m:ss` and `h:mm:ss` durations, percentages, `+ − × ÷ * / -`, brackets) and its value
   matches its `is` to the precision `is` is written in (a second for durations, the shown
   decimals for numbers). Off by more: rejected. Within rounding: kept, marked `rounded`.
2. **Every number has a home.** Each number in a sum is a given, an earlier line's result,
   or one of the units listed in the prompt.
3. **The givens are the writer's.** Each given's words are found in the title, the note or
   the brief (loosely, as quotes are snapped today) and contain its value: `5:30` in
   "hold 5:30"; `50:00` in "under 50 minutes" (a whole duration matches its first part);
   `10` in "10k"; `4` in "four days". A number the sum uses that the model forgot to list
   is looked up in the writer's words, the corrected paragraph first, and listed with them
   ("10k"); one the writer never wrote fails the check. At least one given comes from the
   paragraph being read, so the sum is about the claim.
4. **The words are elsewhere in the note.** Each passage has two words or more, sits on
   one line, is found in the note, and is not inside the corrected words.
5. **The correction says nothing the check does not show.** Every number in the text
   (except whole numbers up to 12) is in the note, a given, or part of the sum.

When a correction fails any rule it becomes a question: `kind` becomes `question` and its
text becomes the model's fallback `question`. With no fallback, a read stays silent and a
probe fails as `bad_output` (the writer sees "The answer came back in a shape Inkling can't
use. Try again."). The brain logs the reason to stderr, e.g. `correction became a question:
line 1: 5:30 × 10 is 55:00, not 50:00`. No retry: a second call would double the cost of
the reads that need it most.

### Cost per read

The prompt grows by about 300 tokens and the schema by about 250; the schema sits at the
front of the prompt and is cached after the first read ($0.20 per million). That is about
$0.0015 more per read, about 5% of a measured read. Only corrections write a check: about
100 to 150 more output tokens plus some thinking, and most reads stay silent.

Measured on 7 Nov with five real reads (Opus 5.5, low effort, the SDK's estimates):

| Read | Came back as | Cost | Time |
|---|---|---|---|
| The running plan's paragraph | correction, working `5:30 × 10 = 55:00`, `50:00 ÷ 10 = 5:00`, from "hold 5:30 per km", "city 10k", "under 50 minutes" | $0.013 | 6.4 s |
| The same, before the prompt's line about figures | its question: the text said 5:00 with no line for it | $0.030 | 12.9 s |
| The same, before the brain looked up unlisted numbers | its question: the sum used 10 without listing it | $0.021 | 6.5 s |
| "The far side of the Moon is dark…" | a question that says what it remembers ("As I remember it, the far side gets as much sunlight as the near side…") | $0.017 | 7.2 s |
| "With three runs a week…" after "I run four days a week…" | correction with your words | $0.018 | 5.9 s |

That is within the $0.024 to $0.034 measured for reads on 6 Nov: the check costs less
than the spread between reads. The two failed tries show the gate working: each would have
been a confident correction resting on a number the tag did not show.

## Data model

`src/store.rs`. One new field at the end of `Message`; everything else is new types:

```rust
pub struct Message {
    pub role: Role,
    pub text: String,
    #[serde(default)]
    pub at: DateTime<Utc>,
    /// How a correction shows it is right. Only the brain's checked answer fills it.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "check_or_none")]
    pub check: Option<Check>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Check {
    pub sum: Vec<SumLine>,
    pub givens: Vec<Given>,
    pub words: Vec<Passage>,
    /// The mind agreed the check was wrong and took the correction back.
    pub taken_back: bool,
}
#[serde(default)] pub struct SumLine { pub sum: String, pub is: String, pub rounded: bool }
#[serde(default)] pub struct Given { pub value: String, pub words: String }
#[serde(default)] pub struct Passage { pub words: String, pub note: Option<String> }
```

`check_or_none` reads a check it cannot parse as `None`, so a note written by a newer build
still opens with its words and threads. `skip_serializing_if` keeps every existing
message's JSON exactly as it is. `Message::new` sets `check: None`; nothing builds a
`Message` by hand, so no other file changes for the field. `Passage.note` is `None` for
this note; Threads between notes can put another note's id there.

The check lives on the message, not the thread: it backs one claim, and a later message
(a pushback in a reply) could carry its own.

## Fit with the other features

Shared types this changes: `store::Message` gains `check` (last field). Nothing else:
`Thread`, `ThreadStatus`, `ThoughtKind`, `Note` and the wire `Context` are untouched.
`REPLY_SCHEMA` is untouched (Disagree uses `reply` with its own message and reads
`resolved`). `THOUGHT_SCHEMA` and `PROBE_SCHEMA` gain two optional properties.

- **Let it stand + Turn it over.** A correction taken back ends `Resolved` with
  `check.taken_back`. If Let it stand adds a status for "your words stood", taken back
  should use it (one line in `note_page/show_working.rs`). Turn it over can show the check
  with `working::text_lines`.
- **Answer in the note + Write it in.** Independent. The working is Inkling's, never
  written into the note.
- **Threads of your own.** Your threads carry no check.
- **Loose ends.** Independent.
- **Threads between notes.** `Passage.note` names the other note, and the link opens it.
- **Read it through.** Uses `read`, so it gets checks for free.

## Tests

**Brain unit** (`brain/test/working.test.ts`, new):
- evaluation: `5:30 × 10` is 3300; `50:00 ÷ 10` is 300; `(5:30 − 5:00) × 10`; `1,200 × 15%`
  is 180; `1:05:00 − 49:10`; precedence; `* / -` accepted; `five × 10` and `10 ÷ 0` refused.
- the running plan's working passes, `rounded` false; `10 ÷ 3 = 3.33` passes, `rounded`.
- each rule failing on its own turns the correction into the fallback question: a wrong
  line, a number from nowhere, a given not in the note, a given whose words lack its
  value, no given from the paragraph, a text number the sum does not show, a passage inside
  the quote, a passage not in the note, an empty check.
- no fallback: a read is silent, a probe is `bad_output`.
- a passage is snapped onto the note's own characters (curly quotes, spacing).
- questions pass through and lose any `check` the model sent.

**Brain protocol** (`brain/test/protocol.test.ts`): a backed correction's `done` carries
`check`; a failed one arrives as a question; a probe with no fallback is `bad_output`.

**App unit**: `store` loads a message without `check`, with a garbled one (as `None`), and
round-trips one; `working.rs` parses the wire `check`, writes the copy and export lines
(`=`, `≈`, taken back) and finds changed words; `export` puts the working under its
correction in Markdown and HTML.

**Scenarios** (`brain/test/e2e/scenarios/`, the fake brain takes `[[sum:EXPR = IS]]`,
`[[given:VALUE@WORDS]]`, `[[words:PASSAGE]]` and `[[question:TEXT]]`):

`t14-show-working.txt`:
```
note: Race plan
type: My pace is 5:30 per km, so the 10k takes 50 minutes. [[correction]][[quote:so the 10k takes 50 minutes]][[sum:5:30 × 10 = 55:00]][[given:5:30@5:30 per km]][[given:10@the 10k]][[question:How long does 10 km take at 5:30?]]
settle
dump: backed
expect: d.threads[0].kind === "Correction" && d.threads[0].check === "sum:1"
type: At 6:00 per km the 10k takes 50 minutes. [[correction]][[sum:6:00 × 10 = 50:00]][[given:6:00@6:00 per km]][[given:10@the 10k]][[question:What does 6:00 a km come to over 10 km?]]
settle
dump: demoted
expect: d.threads[1].kind === "Question" && d.threads[1].check === null
expect: note("Race plan").threads[1].messages[0].text.includes("What does 6:00 a km come to")
hover: 1
shot: working
set: appearance=dark
shot: working-dark
copy: 1
export: md
quit
expect: log.some(l => l.msg.includes("Working: 5:30 × 10 = 55:00"))
```

`t15-working-words.txt`: a paragraph `I run four days a week. [[silent]]`, then a correction
backed by `[[words:I run four days a week]]`; `see: 1 1` selects those words (`expect:` the
dump's `selection` spans them); `disagree: 1` then `reply: Those are different weeks.
[[reply>resolved]]`; `expect:` the saved check has `taken_back` and the thread is
`Resolved`; `export: md` logs `Working, taken back` or `In your note`.

New script steps: `see: N M` (the Mth quoted words of thread N's check, as a click) and
`disagree: N`.

## Files and size

| File | Change |
|---|---|
| `brain/src/working.ts` | new: the evaluator, the five rules, `settleCorrection` |
| `brain/src/prompts.ts` | the prompt lines, the two optional schema properties |
| `brain/src/main.ts` | read and probe go through `settleCorrection` (about 8 lines) |
| `brain/test/e2e/fake-brain.ts` | the four directives, through `settleCorrection` |
| `brain/test/working.test.ts`, `protocol.test.ts` | the tests above |
| `src/store.rs` | `Message.check` and the four types |
| `src/working.rs` | new, no GPUI: the wire check, text lines, changed words |
| `src/note_page/show_working.rs` | new child module: the working block, Disagree, showing words, the two script steps |
| `src/note_page.rs` | hooks only: attach the check in `read_line` and `find_loose_thread`, draw it in `tag_panel`, the dispute in `send_reply`, the working in the context, `thread_text` and `debug_state`, `mod show_working;` |
| `src/export.rs` | the working under its correction |
| `src/script.rs` | `see:` and `disagree:` |
| `brain/test/e2e/scenarios/t14-…`, `t15-…`, `fixtures/show-working/` | scenarios and a fixture for screenshots |
| `AGENTS.md`, `design/capabilities.md` | the map and the capability |

Size: M.

## As built (7 Nov)

- Everything above works end to end with the fake brain (`t14-show-working`,
  `t15-working-words`) and was checked against the real model (the table above).
- Screenshots: `design/features/shots/show-working-sum.jpg` and `-sum-dark.jpg` (hovered),
  `-open.jpg` (open, with Disagree), `-words.jpg` and `-words-dark.jpg`, `-taken-back.jpg`,
  `-marks.jpg` (Marks only), from the `show-working` fixture.
- The Disagree message the mind gets: "I don't think your working holds: “…”. Check each
  line of it, and the words it rests on, against my note. If I'm right, say in one
  sentence what you got wrong and mark this resolved. If I'm not, show me where, in one or
  two sentences." Your reply shows in the tag as you wrote it.
- A bare `[[correction]]` in the fake brain still passes unchecked, so other branches'
  scenarios keep working; any of the check directives sends it through the real rules.
- Not mine to change: the Start here tour (`src/welcome.rs`) has two corrections with no
  working, and `design/capabilities.md` now lists the capability the tour and the film
  should show.

## Open question for the owner

1. **Web sources.** Should Inkling ever look things up? My recommendation: not in reads.
   Later, and only if you want it, a `Look it up` control on a question that runs one
   search when you click it, shows a source only if the brain found the URL in that
   search and the quoted passage on the page, and says plainly that the query leaves the
   Mac. Until then, facts from memory stay questions.

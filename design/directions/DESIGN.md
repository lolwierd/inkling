# Inkling: three directions

This folder is a small website that lays out three directions for the redesign. One
person picks one. After that, the chosen direction gets built into the GPUI app, and the
walkthrough video is made in its style.

**Picked on 6 Nov 2026: Loose Threads.** Proof stays on the table for later, possibly with
the owner's own handwriting for the pencil. The build notes at the end of this file say
what changed after the pick.

The brief, in the owner's words: redesign everything "from very small things to the
whole interface" so Inkling feels like its own thing, not a first copy of riddled.ai.
It should feel human and made with love. Think about aha moments. Each direction needs
a logo and well-built SVG elements, plus anything the owner might have missed. And:
"dont go sooo deep in ideas/design that you forgo the ux side of things."

## The default read, and why we did not take it

The first idea anyone has for "a notebook where an AI writes in the margin" is a warm
paper notebook with serif text and the AI's notes in a softer serif italic, tethered to
dashed underlines. That is riddled.ai, and it is today's Inkling (the palette was copied
from it). It is a good design. It is also the one thing this redesign must not be, so
every direction below starts from a different object, and none of them uses a serif
italic for Inkling's voice.

## One idea under all three: Inkling lives in the dot of the i

An inkling is the smallest amount of something: a hint, a suspicion. The smallest piece
of ink in a word is the dot of an i. In every direction the wordmark is the word
"inkling" outlined from that direction's typeface, with the dot of the first i replaced
by the direction's own mark:

| Direction | The dot becomes | Why |
|---|---|---|
| Proof | a blue-pencil caret | the copy editor's "something goes here" |
| Chk'd | a yellow highlighter square | the checker's "verified" |
| Loose Threads | a French knot | how embroiderers make a dot |

The second i keeps its ordinary dot, so the change reads as a choice and not a glitch.

Files: `assets/<dir>/wordmark.svg`, `assets/<dir>/icon.svg`, `assets/<dir>/icon-dark.svg`
(Proof also has `icon-alt.svg` / `icon-alt-dark.svg`: a proof strip with a circled
query. It tells the story better at 256 px but turns into a generic document icon below
64 px, so the specimen "i" is the primary.) The generator lives in `../tools/`
(`brand.py` + `brand2.py`; the tapered-stroke helpers `ribbon`, `cubic`, `running_stitch`,
`caret`, `knot`, `tick` are in `logos.py` and `brand.py`; see `../tools/README.md`).

## Rules every direction keeps (UX guardrails)

1. **The hand is in the marks, never in the words.** Inkling's text is always set in a
   legible typeface. Hand-drawn quality lives in strokes: carets, rings, stitches,
   ticks. No handwriting fonts anywhere.
2. **No texture under text.** Grid, weave and paper grain live in margins, gutters and
   the desk around the page. The writing column is a flat colour.
3. **Plain words on every control.** Buttons say Reply, Resolve, Dismiss, Undo. The
   world's own word (stet, chk'd, tied off) appears only in status lines and as the
   tooltip's second sentence, so it is learned without being required.
4. **Colour is never the only signal.** Every mark also differs in shape or carries a
   number. A question and a correction look different in greyscale.
5. **Contrast.** All text at least 4.5:1 against what it sits on, in light and dark.
   Marks (non-text) at least 3:1.
6. **Motion never blocks typing.** Animations run on marks, never on the text you are
   editing; typing cancels any running margin animation. `prefers-reduced-motion`
   replaces draw-on animations with a 120 ms fade.
7. **Keyboard first.** Every margin action has a key (see the UX core).
8. **One accent per job.** Each colour can say what it means in one phrase.

## UX core v2: what every direction gets

These are product changes, independent of the look. Each one fixes a problem with
today's build. Cost is a rough build size: S (under a day), M (a few days), L (a week).

| # | Change | The problem it fixes | Cost |
|---|---|---|---|
| 1 | **Read marks.** A small mark beside every paragraph: nothing (not read yet), moving (reading now), settled (read, nothing to ask), numbered (has a question). | In "Thinks with you" mode Inkling is silent most of the time, so you cannot tell whether it read a paragraph, is still reading, or is broken. | S |
| 2 | **Questions wait while you type.** A question that arrives while you are typing comes in closed: just its mark and number in the margin. It opens when you pause for 2 s or leave that paragraph. | A card opening mid-sentence pulls your eyes off your own words. | S |
| 3 | **Fix the words, the question settles.** When you edit the words a question points at, Inkling re-reads that sentence after a pause. If the edit answers it, the thread resolves itself with a one-line reason; if not, it stays and says what is still missing. | Today a question stays open after you fix the text, until you close it by hand. This is the change most likely to make someone say "oh". | M |
| 4 | **Ask the room.** When you ask about a selection you can ask up to three minds at once. Their answers sit side by side, each labelled. | One reader gives one view. | M |
| 5 | **Open count and jump.** The header shows "2 open". ⌥⌘↓ / ⌥⌘↑ jump to the next / previous open question. | In a long note you lose track of what is still open. | S |
| 6 | **A first run that reads your own words.** First launch asks for one sentence about something you are working on, and Inkling's first question is about that sentence, within about 10 s. | The welcome note is a canned example about the Moon. | S |
| 7 | **Drawings become figures.** Drag a sketch from the margin into the note. It becomes a figure block that replays when clicked and exports as an image. | A drawing lives only in the margin and is lost on export. | L |
| 8 | **Undo for dismiss.** Dismiss shows "Dismissed · Undo" for 8 s; ⌘Z also works. | Dismiss is one click and final. | S |
| 9 | **Every margin action has a key.** R reply, ⌘↵ resolve, ⌘⌫ dismiss, Esc close, ⌥⌘↓/↑ jump. | Today most margin actions need the mouse. | S |
| 10 | **Export keeps the thinking (optional).** An appendix "Questions answered while writing" in Markdown, Word and HTML exports. | Export throws away every thread. | S |

## The demo note (the same in all three mocks)

The same note appears in every direction so the comparison is about the design, not
the content. It is in `site.js` as `Board.note`. It is a running plan because its
mistake is arithmetic anyone can check in their head, which makes Inkling's correction
feel earned rather than arbitrary.

- Title: **Sub-50 10k by March**
- ¶1: Last year I ran the city 10k in 56:40. I went out too fast and walked twice after
  the 7 km mark. This year I want to finish under 50 minutes.
- ¶2: The plan is to run four days a week and hold 5:30 per km on every run, so my legs
  learn race pace. If I hold 5:30 for the whole race, I'll come in just under 50.
  - **Correction 1** on "If I hold 5:30 for the whole race, I'll come in just under 50":
    "At 5:30 a km, 10 km takes 55:00. Under 50 needs 5:00 a km or faster; 4:55 leaves a
    little room. Which do you want to change: the pace or the goal?"
  - Fix: change that "5:30" to "4:55". Inkling re-reads and settles it: "4:55 a km comes
    to 49:10. That works."
- ¶3: Sunday is the long run. I'll add a kilometre every week until I reach 16.
  - **Question 2** on "add a kilometre every week until I reach 16": "Starting from how
    far? If you start at 8 km, 16 km is eight weeks out. Is the race at least eight weeks
    away, with an easier week before it?"
- ¶4: Strength work twice a week, mostly calves and hips, because that's where it fell
  apart last year. (Read, holds: no question.)
- **Drawing** (offered on correction 1), three steps: (a) "Pace per km" — a scale from
  4:30 to 6:00; (b) "Times ten is your finish" — each pace mapped to a finish time;
  (c) "Under 50 starts at 5:00" — the goal zone marked, 5:30 → 55:00 shown outside it.
- **Ask the room** on "four days a week": Inkling: "Four works if two of them are easy.
  Which two are hard?" Skeptic: "You walked after 7 km last year. Is pace the problem,
  or the last 3 km?" Tutor: "Race-pace work is usually one or two sessions a week, not
  every run. Want a sample week?"

## The three directions

Each direction is modelled on a real object from a different world, with that object's
materials and its signature: the one feature its own culture is proud of.

### 1. Proof (print world): `proof.html`

**The object: a galley proof.** The long typeset sheet a publisher sends the author
before printing, marked up by the copy editor. Three things about it we did not invent:

1. **Non-photo blue.** Marks made in this light blue pencil do not show up when the page
   is photographed for printing, so editors could write straight onto camera-ready pages.
2. **Author queries.** A question for the author goes in the margin, circled so the
   typesetter does not set it, tagged "AQ1", "AQ2". Every query must be answered before
   the proof goes back.
3. **Stet.** "Let it stand." The author rejects a change by writing stet and putting dots
   under the original words. Proofs also carry line numbers, crop marks and registration
   marks, and the caret (‸) marks where something goes.

**Concept.** Inkling is a copy editor who only writes queries, in a pencil that never
prints. Your words are the type; its marks sit on a layer above them that you can lift
off at any moment.

**Moment.** You see your note as a proof with two blue queries in the margin. Press
"Pencil" off (⌘⇧.): the blue layer lifts away the way the camera never saw it, and clean
type is left. Press again and every mark is back exactly where it was.

**Hook.** The separable blue layer. Everything Inkling does is one pencil colour on one
layer, so "Just my words" is not a filter; it is how the object works. Second hook:
drag a sketch into the note and it turns from blue to ink, because now it is yours and
it prints.

**Marks.**
- In the text: a light blue wash under the phrase (multi-line safe) and a small blue caret
  below the line at the phrase's end. Active: the wash deepens and the guide line shows.
- Guide line: a thin blue pencil line from the caret to the margin query (today's tether,
  redrawn as pencil).
- Margin: the query, circled by a hand-drawn pencil outline, tagged `AQ1 · Fix` (a
  correction) or `AQ2 · Query` (a question). Text in IBM Plex Sans Condensed, pencil blue.
- Two pencils: **blue is Inkling, graphite is you.** When you resolve, a graphite "OK ✓"
  goes next to the query; when you dismiss, graphite stet dots go under the words and
  "stet" in the margin. The conversation shows as two colours.
- Read marks: paragraph numbers in the left margin (¶1…), quiet grey. While reading, the
  number pulses blue; read and holds, a small blue tick; has a query, the number is
  circled in blue.
- Thinking: the paragraph number pulses.
- Drawings: blue pencil figures in the margin, steps a / b / c.
- Ask the room: each mind gets its own pencil (Inkling non-photo blue, Skeptic green,
  Tutor violet). In this direction colour means who, never what.

**Palette (jobs first).** Light "proof": desk #ECEBE6 (around the sheet), paper #FBFBF8,
ink #1B1A17 (your words), ink-2 #4A4740 (secondary text), ink-3 #6E6A61 (paragraph numbers,
labels; still 4.5:1), rule #E1DED6, pencil #2F6FB3 (Inkling's text and marks; 5:1 on paper),
pencil-soft #8DBBE8 (guide lines, carets), wash rgba(47,111,179,.12), graphite #55524B
(your marks). Dark "lightbox" (the proof on a light table at night): desk #0F1114, paper
#171A1E, ink #E9E6DE, ink-2 #B8B3A8, pencil #7CB2EC, graphite #A8A395. Measure every pair.

**Type.** Literata for your words (17 px / 1.6, title 30 px); IBM Plex Sans Condensed for
Inkling's queries and the UI (13–15 px); IBM Plex Mono for paragraph numbers, AQ tags and
figures. At most five sizes.

**Motion.** Pencil strokes draw on in 280 ms ease-out. Lifting the pencil: the blue layer
rises 4 px and fades over 360 ms, like lifting tracing paper; it returns in 240 ms.
Resolve: the graphite tick draws in 180 ms, the query folds to a chip in 260 ms.

**Sound (optional, off by default).** A pencil tick when a query arrives; a soft paper
slide when the pencil lifts.

**Elements to draw (SVG).** Caret; guide line; query circle; AQ tag; graphite OK tick;
stet dots; paragraph-number read states (unread, reading, read, has query); crop marks;
registration mark; blue pencil; the "Pencil" toggle icon; figure frame.

**Aha moments.** Lift the pencil and your clean draft is there. A sketch turns to ink when
you take it. Fix the number and the query ticks itself OK. Paragraph numbers show what
has been read.

**Refuses.** Texture under the text. Handwriting fonts. Red: red means "wrong" in every
app, and Inkling is not grading you. More than one blue.

**Risk.** It is the closest of the three to the paper-notebook family. What keeps it
apart: cooler, brighter paper; the one-blue layer; a condensed sans for Inkling's voice
instead of serif italic; circled queries in the right margin; paragraph numbers.

**GPUI build.** Bundle Literata + IBM Plex Sans Condensed + Plex Mono (OFL). Pencil strokes
are tapered PathBuilder fills (drawing.rs already strokes paths). The wash is quads under
text runs (the editor already paints the active wash). The caret is a path at the anchor
end. The layer toggle exists (Just my words). Paragraph numbers are a new gutter element
in the editor. Cost: M.

### 2. Chk'd (engineering world): `check.html`

**The object: an engineer's computation pad and the checker's coloured pencils.** Three
things about it we did not invent:

1. **The grid is printed on the back.** Engineering paper is pale green with the grid on
   the reverse, so it shows through faintly, survives erasing, and does not photocopy.
2. **The checker's colour code.** Yellow over everything verified, red for anything that
   must change, a separate colour for notes that are not changes.
3. **The title block.** Every sheet has boxes for By / Date / Chk'd / Date, and a sheet is
   not done until the checker initials it. A revision gets a cloud drawn around it and a
   numbered delta (Δ1).

**Concept.** Inkling is the checker. It goes over your note with yellow for what holds and
red for what does not, and when every red mark is answered, it initials the page.

**Moment.** The note's header is a title block. Yellow bars fill the gutter beside each
paragraph as Inkling checks it; a red ring lands on "5:30". Change the number: the ring
gets a yellow swipe, the last open check clears, and the CHK'D box gets "IK" and the date.

**Hook.** The sign-off. A note has a "checked" state you can earn, and the note list shows
it (✓ checked, ① open, Δ re-checking).

**Marks.**
- Gutter (read marks): a yellow bar beside each paragraph that was read and holds; a bar
  drawing downward while reading; a red segment where there is a question; no bar if
  unread. Bars live in the gutter, never over text.
- In the text: a red hand-drawn ring for short anchors (three words or fewer, like
  "5:30"); a red underline for long or multi-line anchors; a numbered red tag (①) at the
  start.
- Leader: drafting leaders are straight, angled lines with a dot at the target and a short
  horizontal shoulder at the note. Never curved. This replaces the bezier tether.
- Margin: the check note. Red number tag; text in ink (red text for whole sentences is
  alarming and tiring); a correction shows the redline (old value struck, new value).
- Blue is conversation: your replies, Ask answers and chat.
- Revision cloud: edit a checked paragraph and a scalloped cloud with Δ1 marks the change,
  and Inkling re-checks only that part. This is UX core #3 in drafting language.
- Resolve: a yellow swipe over the red ring; the tag fills yellow; the note folds to "① ✓".
  Dismiss: the ring is struck through lightly; status "Left as is".
- Drawings: drafted on the grid with dimension lines, leaders and DETAIL A callouts; steps
  "1/3".
- Ask the room: each mind signs with its initials box (IK, SK, TU).

**Palette.** Light "pad": desk #E2E9DD, sheet #F2F7EE, grid #D3E2D0 (margins only; it fades
out behind the text column, as if printed on the back), ink #22262A, ink-2 #4C5550, label
#5F6E63, rule #C4D2BF, yellow #F2C94C (fills and bars; ink on yellow is about 11:1), red
#C0362C (rings, tags, leaders), blue #2D5DA8 (conversation). Dark "blueprint": desk #0E1C33,
sheet #12233D, grid #1E3557, ink #E8EEF6, ink-2 #A9BCD6, yellow #F2C94C, red #F0624F, blue
#8DB4F0.

**Type.** Atkinson Hyperlegible Next for your words (17 px; built by the Braille Institute
so no letter can be mistaken for another, which is what an engineer's lettering is for);
B612 for labels, the title block and check notes (designed for Airbus cockpit displays);
B612 Mono or Atkinson Hyperlegible Mono for numbers, tags and dimensions.

**Motion.** Highlighter swipe 220 ms ease-out, left to right with a slight overshoot. Ring
draws in 260 ms. Initials write on in 600 ms. Cloud scallops pop in sequence, 30 ms apart.

**Sound (optional, off by default).** A soft tick on verify; a low stamp when the page is
signed.

**Elements to draw (SVG).** Title block; CHK'D initials box; yellow gutter bar (states); red
ring; numbered tag; drafting leader; revision cloud with Δ; dimension line; grid tile
(show-through); mechanical pencil; DETAIL callout; "left as is" strike.

**Aha moments.** The page gets initialed when every check is answered. Edit after checking
and a revision cloud shows what changed and gets re-checked. The yellow gutter is proof it
read every paragraph. The note list shows which notes are checked.

**Refuses.** Grid under text. Red filling large areas. All-caps body text. Fake precision
(no made-up measurements or decoration that pretends to be data).

**Risk.** It can feel technical for journaling. What keeps it warm: Atkinson's open shapes,
hand-drawn rings and ticks, and plain friendly words.

**GPUI build.** Bundle Atkinson Hyperlegible Next + B612 (OFL). Gutter bars are quads, rings
are wobbled ellipse paths, leaders are straight paths, the title block is a div grid, the
revision cloud is a run of arcs. Cost: M.

### 3. Loose Threads (craft world): `threads.html`

**The object: an embroidery sampler in its hoop.** Three things about it we did not invent:

1. **A sampler is a practice cloth**: rows of stitches, an alphabet, the maker's name and
   the date. Its border was often stitched last, when the piece was done.
2. **Embroidery has a back.** The front shows the clean stitch; the back shows every knot
   and jump, and embroiderers judge work by the back.
3. **A loose end has to be finished**: tied off with a knot or woven under. A French knot is
   how you make a dot. Our language for arguments already comes from sewing: losing the
   thread, tying up loose ends, hanging by a thread.

**Concept.** Every question is a loose thread in your note. Inkling stitches it to the
words it is about; you answer it and tie it off. A note you have worked through earns its
border.

**Moment.** A running stitch sews itself under "add a kilometre every week until I reach
16" and carries on into the margin to a fabric tag with the question. You reply; the
thread pulls tight into a knot; the tag reads "Tied off". The note's border grows by one
motif.

**Hook.** The tie-off and the border that grows. Answered questions build into a visible,
handmade record; the note becomes a sampler of your thinking.

**Marks.**
- In the text: a running-stitch underline (slightly irregular dashes) in the mind's thread
  colour; multi-line safe. A correction also gets a small cross-stitch × at the start of the
  phrase, so a question and a correction differ in shape, not only colour.
- Thread: a small rope simulation, not a curve. It comes up through a hole at the edge of
  the text column, level with the first stitched row (so it never crosses words), and is
  tied to the tag's loop. It sags under its own weight, sways when its tag is hovered,
  pulls straight when tied off (slack and gravity both ease out, or it still sags), and
  drops when snipped. Quiet unless you look at it: 26% at rest, 60% under the pointer,
  full while its tag is open (the owner's note after the first build: "the rope is too
  prominent when its not selected").
- Margin: a woven fabric tag (a stitched border inside a rounded label, a loop where the
  thread attaches). Text in Figtree 14 px, ink colour; the mind's name is a small label in
  its thread colour.
- Read marks: in the left gutter, one short tacking stitch per paragraph read; a bobbing
  needle while reading; a French knot in the mind's colour for a paragraph with a question.
- Resolve ("Tied off"): pull taut, knot, tag folds to a small knot chip; the stitches stay
  as a faint record. Dismiss ("Snipped"): scissors cut the thread, the tail drops and fades,
  the stitches come out one by one; Undo for 8 s.
- Thinking: a needle bobbing in and out of the cloth beside the paragraph.
- Drawings: stitched outlines that appear stitch by stitch; captions on small tags.
- Ask the room: each mind is a thread colour (madder, indigo, ochre, sage, plum).

**Palette.** Light "linen": cloth #EFE8DB (the desk around the page, the only place weave
shows), page #FBF8F2 (flat writing surface), ink #26232B, ink-2 #57505C, ink-3 #6F6862,
rule #E3DACB; threads madder #A63F32 (Inkling), indigo #2E3F7A, ochre #A8741A, sage
#4F7350, plum #6B3E62. Dark "lamplight": cloth #18161A, page #1F1D22, ink #EDE6DA, ink-2
#BDB4A8; threads madder #D9705E, indigo #8C9BE0, ochre #D9A548, sage #8DB88E, plum #C08AB4.

**Type.** Fraunces with SOFT 100 for your words and titles (its soft axis rounds the
corners the way thread rounds a letter; WONK off for calm body text); Figtree for
Inkling's tags and the UI (round, clear, no costume). Tabular figures for numbers.

**Motion.** Stitches appear one at a time, 40 ms apart. The thread sways when you hover
its tag. Tie-off: the thread pulls taut (spring, about 320 ms), then the knot pops (scale
with a small overshoot, 180 ms). Snip: scissors close in 120 ms, the tail falls for 500 ms
with a little rotation and fades. A border motif stitches in over 400 ms.

**Sound (optional, off by default).** A short thread pull; a two-click snip.

**Elements to draw (SVG).** Needle; spool; running stitch; loose tail; French knot;
cross-stitch ×; hoop; sampler border motifs; fabric tag; scissors; pin; thimble (optional).

**Aha moments.** Answering pulls the thread into a knot. The border grows as questions are
resolved, so a finished note looks finished. Each mind has its own thread, so you can see
who asked what. Later: "turn the work over" to see the back: every thread ever pulled.

**Refuses.** Fabric texture under text. Mascots. Handwriting fonts. Stitching on controls
(buttons stay plain). Motion that blocks typing.

**Risk.** It can tip into cute. What holds it back: texture only in the margins, a plain
sans for every control, exact layout, and real stitch geometry instead of clip-art.

**GPUI build.** Bundle Fraunces + Figtree (OFL). The stitch underline is a run of short
tapered paths; the tether is a sagging bezier; the knot is a circle plus a spiral path;
the border is a repeated motif. Tie-off needs a small spring animation. Cost: M to L.

## Comparison

| | Proof | Chk'd | Loose Threads |
|---|---|---|---|
| Feels like | An edited manuscript | A checked calculation | A handmade sampler |
| Best for | Long-form writing | Plans, design docs, anything with numbers | Learning, journaling, thinking out loud |
| Signature | Lift the pencil off | The page gets initialed | Tie off the thread |
| Dark mode | Light table | Blueprint | Lamplight |
| Risk | Closest to the paper-notebook family | Can feel technical | Can tip into cute |
| Build | M | M | M to L |

## Page template (every direction page, in this order)

1. **Top bar** (shared markup from `site.css`): Overview · Proof · Chk'd · Loose Threads,
   Light/Dark, Sound.
2. **Hero.** The wordmark, the concept in one sentence, the object with its three real
   details, and the signature demo, live, in the first view (Proof: Pencil toggle; Chk'd:
   fix "5:30" and watch the sign-off; Threads: reply and tie off).
3. **The app.** A full window mock (1200 × 780 design px, scaled to fit): sidebar, note,
   gutter read marks, margin with correction 1 and question 2, the header with the open
   count, chat button and the Just-my-words control. It works: click a margin item to open
   it, Reply / Resolve / Dismiss with Undo, fix "5:30" to "4:55" (a button in the note does
   it for you) and watch it settle, run the drawing, open Ask the room. Same geometry in all
   three pages: sidebar 232, gutter 56, text column 560, margin 320.
4. **Every state.** Small panels: reading, waiting while you type, open, correction,
   resolved, dismissed + undo, drawing (play), ask the room, empty note, offline, limit
   reached (with the reset time), first run.
5. **Logo and icon.** Wordmark light and dark, the dot detail enlarged, app icon at 1024,
   256, 128, 64, 32, 16 in light and dark, clear space.
6. **Elements.** Every SVG element at the size it is used in the app and enlarged, each with
   its name, its job and where it appears.
7. **Colour.** Tokens for light and dark with their jobs and measured contrast (computed on
   the page by `Board.contrast`).
8. **Type.** Families, why, the size scale.
9. **Motion and sound.** Each motion with duration and easing and a play button; the sound
   cues (synthesised by `Board.sound`, off until the viewer turns sound on).
10. **Words.** What the controls say, status lines, and the empty / error / limit / first-run
    copy, in the direction's voice.
11. **Aha moments.**
12. **What it refuses, and its risk.**
13. **Things you might have missed** (direction-specific).
14. **Building it in GPUI.** What changes in which file, fonts to bundle, cost per piece.

## Things you might have missed (all directions; listed on the overview)

- **The app has no icon today.** macOS 26 puts any icon that is not a full rounded square
  into a grey tile, and asks for light, dark, clear and tinted versions (Icon Composer
  layers). Each direction here has light and dark; clear and tinted come with the build.
- **Open questions on the Dock icon** as a badge (optional, off by default).
- **Sound:** off by default, one quiet sound per event, taken from the direction's object.
- **Every state written:** empty note, nothing to ask, reading, offline, limit reached
  (shows when it resets), signed out, error.
- **Accessibility:** VoiceOver labels on margin items ("Question 1 of 2, about '5:30'"),
  marks that never rely on colour alone, reduced motion.
- **Selection colour, caret colour and focus ring** belong to each direction too.
- **Export and print** use the direction's typefaces and never print Inkling's marks unless
  asked.
- **The walkthrough video** comes after the pick, in the chosen direction's motion and sound.

## Technical rules for the pages

- Plain HTML, CSS and JS. No frameworks. Fonts only from Google Fonts. No other network
  requests. Images are inline SVG or files under `assets/`.
- Tokens on `:root` for light and `:root[data-theme=dark]` for dark. `site.js` sets
  `data-theme` before first paint from the viewer's choice or the system setting.
- The page works at 390 px wide with no horizontal page scroll. The app mock scales to fit
  (`.fit` in `site.css`).
- Every interactive state of a page can be reached with `?state=<name>` so it can be
  screenshotted headless, and `?qa=1` writes layout checks (overflowing elements, font
  sizes, colours in use) into `<pre id="qa">` for `--dump-dom`.
- A page stays under about 400 KB.

## After the pick: what is in the app (6 Nov 2026)

- Fonts: Fraunces Text (opsz 14) and Fraunces Display (opsz 72), both SOFT 100 and WONK 0,
  plus Figtree, bundled as static instances with their own family names
  (`design/tools/fetch-app-fonts.py`), because GPUI cannot set variable axes.
- `src/theme.rs`: linen and lamplight palettes; the seven ink colours became threads
  (madder is Inkling's own).
- `src/stitch.rs`: the rope (Verlet, 24 points, joint-angle limit, re-hung from a fresh
  sag whenever its ends jump), running stitch, cross-stitch, tacking stitch, French knot,
  needle, border motif. Tested: sags without kinks, ties straight to within a pixel, falls
  when cut, no zig-zag after a jump, no sway while typing.
- The page: a flat sheet on linen, the sampler border along its top (one motif per tied
  thread), gutter read marks, fabric tags, threads, a title-bar wordmark with the knot.
- The icon: the stitched "?" kept, remade with even stitches, a French knot for its dot,
  and the loose end running to a parked needle. A stitched "i" read as the info symbol at
  small sizes, and a letterless version read as a winking face.
- `INKLING_SLOWMO=<factor>` slows every thread animation, for screenshots and the video.

## Words (the owner's rule, 6 Nov 2026)

None of Riddled's verbs or names, anywhere in the app or around it. "Riddle this" became
**Find a loose thread** (⌘↵), "Visualize" became **Sketch it**, the modes are **Reads along**
and **Waits to be asked**, "Just my words" became **Hide threads**. The brain's tasks were
renamed to match (`probe`, `sketch`); the stored voice value `riddle` stays for old minds
files and shows as "Socratic".

## UX core in the app (6 Nov 2026)

| # | Change | State |
|---|---|---|
| 1 | Read marks in the gutter | done |
| 2 | Questions wait while you type ("Opens when you pause") | done |
| 3 | Fix the words, the question settles (one look per edit; says what is still missing if it is not settled) | done, checked against the real model |
| 4 | Ask the room: up to three minds, each in its own thread colour | done |
| 5 | Open count and jump (⌥⌘↓ / ⌥⌘↑, scrolls to the words, focuses the reply) | done |
| 6 | First run: "Start here" asks for one sentence; the first question is about it | done |
| 7 | Drawings become figures | not started (L) |
| 8 | Undo for dismiss: chip for 8 s, ⌥⌘Z | done |
| 9 | Keys: ⌘↵ resolves the open thread (or finds a loose thread in a selection), ⌘⇧⌫ dismisses, Esc closes | done |
| 10 | Export keeps the thinking: threads under their paragraphs, on by default | already there |

Scenarios: `brain/test/e2e/scenarios/u01-ux-core.txt`, `u02-first-run.txt`, `u03-wait-while-typing.txt`
(the last with `FAKE_DELAY_READ_MS=2500`).

## Settings (6 Nov 2026)

The owner asked for Settings to "think macos", with Liquid Glass, and every shortcut customizable.

- **The default read**, kept on purpose: a System Settings window, a sidebar of panes and grouped
  rows. It is what ⌘, opens in every Mac app, and the owner asked for exactly that. What makes it
  Inkling's: the glass is linen, and choices are shown as the page they make.
- **A window, not a sheet.** GPUI only blurs behind a window (NSVisualEffectView), so real glass
  needs one. It also sits beside the note, so appearance, text size and thread colour show in the
  note as you change them, and shortcuts are recorded where the note's keys cannot fire.
- **Materials.** The desk is linen with the desktop showing through (62% cloth in linen, 72% in
  lamplight); the sidebar is frost resting on it, lighter at the top, with a rim and an inner edge
  of light; the page is flat paper, because words sit on flat paper. How much shows through the
  sidebar is set by a test: its text keeps 4.5:1 over a black or a white desktop.
- **Pictures of the choice.** "Where questions appear" shows three tiny pages (stitched words,
  tags in either margin, in one, or marks only); Appearance shows linen, lamplight, and the two
  split for System. The chosen tile is outlined in ink and ticked, so it reads without colour.
- **One accent per job.** The thread colour marks where you are (the sidebar icon) and what is
  listening (the recording keycap); ochre with a warning shape marks a clash.
- **Shortcuts.** Grouped by where they work (anywhere, or while writing), keys drawn as macOS
  draws them (⌃⌥⇧⌘), plain keys refused with the reason. Only changed shortcuts are saved.

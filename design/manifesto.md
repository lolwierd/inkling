# Inkling's manifesto

This document, written on 6 Nov 2026, has three parts: what Inkling believes, the features
those beliefs call for, and what in today's app they argue against. The beliefs are numbered so
the later parts can point at them.

## 1. What Inkling believes

1. You find out what you think by writing it down, because on the page the numbers have to add
   up and each step has to follow from the last. That only works if you write the sentence
   yourself, so Inkling never writes your notes for you. It does not finish your sentences,
   rewrite your paragraphs, draft a page, summarise into your note, or put its sentence where
   yours was. A note written for you can be correct and still leave you not knowing what you
   think.

2. The most useful thing a reader can do is ask a good question in the right place. When
   something you wrote does not hold up, Inkling asks about it beside the exact words, and the
   work stays with you: you decide what you meant, and the sentence that answers it is yours.
   When a fact is plainly wrong, Inkling says so and shows how it knows, and still leaves the
   fix to you.

3. Most of what you write needs no comment, and a reader that remarks on everything teaches you
   to skip its remarks. Inkling says nothing about most paragraphs, but it marks each one it has
   read, so its silence tells you the paragraph held up. It waits for you to pause, because a
   question that lands mid-sentence pulls you out of the sentence. It speaks only in the margin
   of the page you are on.

4. The margin lets Inkling respond to what you wrote without touching it. The column holds only
   your words; Inkling's sit beside them, and you can hide them all and find your page exactly
   as you left it. When you ask Inkling something directly, it answers plainly, in the margin.

5. A thread is the right shape for a question about your thinking. It is tied to particular
   words, so you can check the question against what you wrote. It ends when you answer it,
   change the words until it no longer applies, or decide the words stand. Until then it stays
   in view, so the open threads on a page show what you have not worked out yet. A chat about
   the whole note has no anchor and no end, which is why Inkling keeps chat at the side.

6. Often the best answer to a question is a change to the sentence it is about. When you edit
   the words a thread points at, Inkling reads them again, and if the problem has gone, the
   thread ties itself off. Whatever you work out in the margin belongs in the note, because the
   note is what is left when the margin is hidden, and putting it there is your part of the
   work.

7. Deciding that your words should stand is also an answer. When you have weighed a question
   and cut its thread, that is a decision, and Inkling should remember it. Inkling is not
   grading you: it keeps no score, and nothing in the notebook should reward you for going
   along with it.

8. Everything Inkling keeps about you or a note is somewhere you can read it: the brief you
   gave the note, what it has learned about you, and every thread it has opened, including the
   ones you cut. You will only write half-formed thoughts in a notebook if you know what its
   reader keeps.

## 2. What follows from it

These are features the beliefs call for that Inkling does not have yet. They are ranked by how
directly the manifesto asks for them and how much they would change writing in Inkling, with
size breaking ties. Sizes use the scale in `directions/DESIGN.md`: S is under a day, M a few
days, L a week.

1. **Answer in the note** (M)
   - What: A thread also ties itself off when a paragraph you write elsewhere in the note
     answers it, and its knot points to your answer; an explanation you asked for counts once
     you have written it in your own words.
   - Why: Belief 6 says what you work out belongs in the note, but today a thread ties itself
     off only after you edit the quoted words or reply in the margin.

2. **Let it stand** (S)
   - What: Dismissing a thread can carry a one-line reason, and Inkling tells every mind which
     points you let stand, so none of them raises one again.
   - Why: Belief 7 says Inkling should remember a no, but today the minds never see a dismissed
     thread, so after any edit to that paragraph the next read can ask the same question again.

3. **Threads of your own** (S)
   - What: Select some words and hang your own question on them in your colour; no mind answers
     unless you call one, and you tie it off or cut it like any other thread.
   - Why: Belief 5 says a thread is the right shape for a question about your thinking, and many
     of those questions are your own, yet today every thread belongs to a mind.

4. **Show its working** (M)
   - What: A correction always carries a check you can do yourself, either the arithmetic or a
     source you can open, and when Inkling has neither it asks a question instead.
   - Why: Belief 2 says Inkling shows how it knows, but today a correction can rest on the
     model's memory alone, and a confident correction that is wrong can talk you out of a
     sentence that was right.

5. **Keep the old words** (S)
   - What: When you edit the words and a thread ties off, the thread keeps your old words,
     struck through, above the new ones, in the margin and in exports.
   - Why: Belief 6 says the best answer is often a change to your sentence, but today the thread
     replaces its quote with your new words, so its record loses the change that answered it.

6. **Write it in** (S)
   - What: When your reply in the margin ties a thread off, the knot offers to put your reply,
     word for word, under its paragraph for you to rework.
   - Why: Belief 6 says what you work out belongs in the note; in the walkthrough film, the
     reply about the long run ties its thread off, yet the plan never says where the long run
     starts.

7. **Loose ends** (S)
   - What: A single list, in ⌘K or the sidebar, shows every open thread in the notebook, oldest
     first, each a click away from its words.
   - Why: Belief 5 says the open threads show what you have not worked out yet, but today the
     sidebar only counts them per note, so an old question is easy to lose.

8. **Turn it over** (M)
   - What: The back of a note lists every thread it has had, in order, with how each one ended
     (tied off, let stand, or still open) and the words before and after.
   - Why: Belief 8 says you can read every thread Inkling has opened, but today a dismissed
     thread vanishes from every view, and nothing lists a note's threads with how each one
     ended; `directions/DESIGN.md` already names this as a later idea.

9. **Read it through** (S)
   - What: In a note that waits to be asked, one command reads the whole draft once, paragraph
     by paragraph, with the same rules and the same silences as reading along.
   - Why: Belief 3 says Inkling waits for your pause, and for some writing the pause comes at
     the end of the draft; today a waiting note gets threads only for one selection at a time.

10. **Threads between notes** (L)
    - What: When something you write contradicts or depends on something in another note,
      Inkling asks about it on a thread that quotes both and links the two.
    - Why: Belief 2 says Inkling asks when something you wrote does not hold up, and sometimes
      what does not hold up is the gap between two notes written months apart, which it never
      reads together.

These are left out because each would contradict a belief:

- Features that write into your text: finishing sentences, rewriting or "improving" a
  paragraph, drafting, summarising into the note, fixing spelling or grammar in place, and
  suggesting titles. Belief 1 rules out all of them.
- An Apply button on corrections. Typing the fix yourself is the point, and it is what lets the
  thread tie itself off (beliefs 1 and 6).
- Drawings become figures, planned as item 7 of the UX core in `directions/DESIGN.md`. It would
  put Inkling's drawing into the column as part of your note (belief 4). A figure you draw
  yourself would not conflict.
- Reminders, notifications, and the optional Dock badge for open questions that
  `directions/DESIGN.md` suggests. Inkling speaks only in the margin of the page you are on
  (belief 3).
- Streaks, scores, and any count of what Inkling caught (belief 7).

## 3. What it argues against in today's app

These run from the most direct conflict to the mildest.

1. **Make it a note copies Inkling's words into the body of a new note.** The new note's
   paragraphs are the thread's messages, each starting "Inkling:" or "Me:", set in your
   typeface; a headless run against the fake brain confirms it. Beliefs 1 and 4 keep the column
   for your words alone, and after one edit nothing marks those sentences as Inkling's. The new
   note could open with your quoted words and the thread hanging in its margin, above an empty
   page.

2. **Copy and export carry the threads unless you stop them.** Copy as Markdown (⇧⌘C) always
   includes every thread, even while threads are hidden, and the export dialog opens with
   threads switched on. Belief 4 promises your page exactly as you left it when the margin is
   hidden, and a copy or an export is that page leaving the notebook. `directions/DESIGN.md`
   had marked this export as optional and said export should never print Inkling's marks unless
   asked. Copy could follow Hide threads, and export could start with threads off.

3. **About you learns by default and does not say what it learned.** Learning is on in a new
   notebook: Inkling rewrites your profile from replies and chat messages in which you mention
   yourself, and that profile goes into every question every mind asks. The profile sits in
   Settings, as belief 8 asks, but the toast says only that something was noted, so a new line
   stays out of sight unless you go looking and remember what the profile said before. This is
   the one place where Inkling writes about you rather than to you, which makes it the place
   where belief 8 matters most. The toast could show the new line with Undo, or learning could
   ask first.

4. **The border counts only tied-off threads.** The border along the top of the page grows one
   motif for each thread tied off. A dismissed thread adds nothing, and a note that held up on
   the first read has no motifs at all. So the border grows when you go along with a question,
   by answering it or changing your words, and not when you decide your words should stand. A
   note that needed many questions also looks more finished than one that needed none. Belief 7
   rules out rewards like these. The manifesto keeps the border, which is the signature of the
   Loose Threads design, and asks only that it count differently: it could mark a note as
   worked through once nothing in it is open, however many questions that took, and treat a
   point you let stand the same as one you tied off.

5. **A dismissed thread disappears.** It stays in the note's file, but once the eight seconds of
   Undo have passed no view shows it, and no mind is ever told about it. Beliefs 7 and 8 ask for
   your no to be remembered and for every thread to stay readable; Let it stand and Turn it
   over, above, would fix this.

6. **Chat has no anchor and no end.** It is the one surface without the thread's shape: its
   answers point at your sentences but cannot hang on them, and nothing in it is ever finished.
   Belief 5 keeps chat at the side rather than removing it. A point a chat answer makes about
   particular words should be able to become a thread on those words.

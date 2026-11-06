# What Inkling can do

Every user-facing capability, taken from the code on 6 Nov 2026. The welcome guide
(`src/welcome.rs`) and the walkthrough film (`design/video/`) must each cover all of it.
Update this list when a feature lands or goes; the key is the default binding.

## Writing

| Capability | How |
|---|---|
| A plain page: title, paragraphs, three heading sizes, bullet and numbered lists, quotes | select words, pick a block kind in the selection menu; or type `# `, `- `, `1. `, `> ` at a line start; a quote's bar is a back-stitched seam |
| Checklists | `[ ] ` (or `[] `) at a line start, or ☐ in the selection menu; click the box or ⌘L to check it off: a cross-stitch in your ink sews in, then the line is tacked through; unchecking unpicks it; Return adds the next item, Return on an empty one leaves the list |
| Callouts: Note, Warning, Tip | `> [!note] ` (or `warning`, `tip`) at a line start, or Note, Warning, Tip in the selection menu: a patch sewn in the kind's thread (indigo, ochre, sage); lines of one kind in a row share a patch |
| Code blocks | ```` ``` ```` and a language, then Return (or Code in the selection menu): line numbers, colours for keywords, strings, numbers, comments and types, long lines wrap; Return stays inside, Return twice at the end leaves; Copy on hover copies the code without its numbers |
| Inline code | ⌘E or `</>` in the selection menu: the code face at 0.86 of your words, on a small chip |
| Speak instead of typing | hold ⌥ alone; let go to stop; the words appear as you talk |
| Ink from the pointer | a drop of ink where the pointer rests (setting) |
| Your look | Linen (light) or Lamplight (dark) or follow the system; text size; lettering: Inkling's own or like your notes; your thread colour: Madder, Indigo, Sage, Rose, Plum, Walnut, Ochre |
| An immersive window | no bar; the controls fade while you write and come back when the pointer moves |

## Reading along

| Capability | How |
|---|---|
| Reads each paragraph at your pause | pace Regular or Slow; a tacking stitch in the gutter marks what it read; checklist items and callouts read like paragraphs, a code block is read whole once you leave it, and a question can sit under a single line of code |
| Speaks only when it is worth it | a question (?) or a correction (×) on a thread; sometimes with a sketch |
| Shows its working | a correction carries a check you can do yourself, under it in the tag: the sum, worked again before it is shown and made only from your numbers (each one a link to your words), or your own words from elsewhere in the note; what it cannot show it asks as a question; Disagree, and if Inkling agrees it takes the correction back |
| Waits while you type | a question that arrives mid-sentence stays folded until you pause |
| Fix the words, the thread settles | edit the words an open thread is about: it reads them again and ties off when the problem is gone |
| Your first sentence | in a new notebook, the first sentence you write always gets a question |
| Reads along, or waits to be asked | per note, in the row above the title; the default is a setting |
| Tell Inkling what a note is for | the brief, above the title; every read and answer takes it into account |
| Learns about you | About you in Settings; it can learn from your replies (switch) |

## Threads

| Capability | How |
|---|---|
| A thread from the words to the margin | stitched under the words (running stitch for a question, cross-stitch for a correction), a hanging thread to a folded tag |
| Tags fold away | point at a tag to unfold it; click to open it and reply |
| Move a tag | drag it by the tab or the top row; it keeps its place |
| Numbered questions | open questions numbered in reading order, with a knot in the gutter |
| Reply in the tag | the mind answers in the same thread |
| Tie off | Resolve: the knot is tied and joins the border at the top of the page |
| Snip | Dismiss (⇧⌘⌫); Undo for a few seconds, or ⌥⌘Z |
| Reopen, Copy, Make a note | from an open tag; Make a note turns a thread into a new note |
| Jump between threads | ⌥⌘↓ and ⌥⌘↑ |
| Hide threads | ⇧⌘. leaves just your words |
| Where threads go | both margins, the right margin, or marks only (a ? in the text, × for a correction, that opens on click) |
| How many are open | "N loose threads" above the title |

## Asking

| Capability | How |
|---|---|
| Find a loose thread | select words, ⌘↵: the one thing in them most worth a question |
| Ask your own question | select words, Ask Inkling |
| Sketch it | select words, Sketch it: a drawing stitched out step by step, with captions; click to enlarge |
| Ask the room | up to three minds at once, each on its own thread in its own colour |
| Call a mind by name | start a sentence with its name and a comma ("Skeptic, …") |
| Chat about the whole note | ⌘J; suggested questions; Clear |

## Minds

| Capability | How |
|---|---|
| Pick the note's mind | the name above the title |
| Built-in minds | Inkling, Staff engineer, Editor, Skeptic, Socratic tutor, Investor |
| Make your own | from Blank or any of them: name, what it is for, how it thinks, voice (Plain, Warm, Blunt, Socratic), how often it speaks (Rarely, When it matters, Often) |
| Each mind has its thread colour | the same colour every time, so you can see who asked what |

## Notebook

| Capability | How |
|---|---|
| Notes and folders | the sidebar (⌘\\): new note, new folder, folders fold; a dot on notes with open threads |
| Find or start a note | ⌘K |
| Copy a note | ⇧⌘C |
| Export | ⇧⌘E: Markdown, HTML or Word, with the threads or without; checklists, callouts and code come along (`- [ ]`, `> [!note]`, fenced code in Markdown) |
| Import from Obsidian | File > Import from Obsidian…: a vault's notes come in with their folders, as Obsidian shows them (tasks as checklists, callouts as callouts, code fences as code); their words are left unread, so only what you write afterwards is read |
| Welcome guide | Help > Welcome Guide |

## Model and keys

| Capability | How |
|---|---|
| Runs on your Claude | the Claude Code login on this Mac; nothing to set up |
| Pick the model and effort | Settings > Model: the models Claude Code offers (from the Agent SDK), kept for when it is not running; reading and answering have their own effort, from the levels the model takes |
| Every shortcut can be changed | Settings > Shortcuts: click keys and press new ones, add or remove keys, clashes flagged on both rows, reset one or all; new keys work at once, menus included |
| Everything from the keyboard | Tab and ⇧Tab move between controls, a running-stitch ring shows where you are, Return or Space presses; the note keeps Tab for lists, so ⌃Tab leaves it; ↑ ↓ in the mind and mode menus |
| Messages in passing | one stack of small stitched labels at the bottom: Undo for a deleted note or a snipped thread, problems in ochre with a close control; the pointer holds a label |

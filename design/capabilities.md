# What Inkling can do

Update this list and the welcome guide (`src/welcome.rs`) when a user-facing
feature changes. The shortcuts below are the default bindings.

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
| Shows its working | a correction carries a check you can do yourself, under it in the tag: the sum, worked again before it is shown and made only from your numbers (each one a link to your words), your own words from elsewhere in the note, or a page on the web that says so (its words, its title and site, a link); what it cannot show it asks as a question; Disagree, and if Inkling agrees it takes the correction back |
| Looks things up on the web, when you ask | Find a loose thread, a question, a mind called by name, a reply and chat may search when a fact can't be settled from the note; reading along never does; a cited page shows only if a search returned it and its quoted words are on it; links no search returned are taken out; no cap on what a call spends, only on its turns (eight), and one that runs out answers without the web; Settings > Model > Look things up on the web (on), with what it sends and costs |
| Waits while you type | a question that arrives mid-sentence stays folded until you pause |
| Fix the words, the thread settles | edit the words an open thread is about: it reads them again and ties off when the problem is gone |
| Answer in the note | write the answer anywhere in the note: the thread's mind checks it, the thread ties off, and a knot sews in after your words; point at the knot to see what it answers, click it to open the thread, whose tag says "Answered in your note"; an explanation you asked for counts once it is in your own words; Reopen if it is not the answer |
| Your first sentence | in a new notebook, the first sentence you write always gets a question |
| Reads along, or waits to be asked | per note, in the sentence above the title (Inkling reads along); the default is a setting. Reading along, Claude's small model looks at each finished paragraph first and only sends on the ones worth a full read |
| Read it through | the mode menu's last item, or ⇧⌘R: every paragraph it has not read, top to bottom, once, by the same rules as reading along; the needle works down the gutter, a pill at the top of the window says how far it has got, and threads arrive as it goes; Stop or ⇧⌘R again stops it, and the next pass goes on from there |
| Tell Inkling what a note is for | the brief, above the title; every read and answer takes it into account |
| Learns about you | About you in Settings; it can learn from your replies (switch) |

## Threads

| Capability | How |
|---|---|
| A thread from the words to the margin | stitched under the words (running stitch for a question, cross-stitch for a correction), a hanging thread to a folded tag |
| Tags fold away | point at a tag to unfold it; click to open it and reply; click its tab or its words again, or anywhere outside it, to fold it away (it stays folded under the pointer until the pointer leaves) |
| Move a tag | drag it by the tab or the top row; it keeps its place |
| Numbered questions | open questions numbered in reading order, with a knot in the gutter |
| Reply in the tag | the mind answers in the same thread |
| Long threads | the conversation scrolls inside a bounded tag; its header, reply box and actions stay visible; where you are is a thin length of the thread's colour beside the words, and words running on past an edge fade into the card; messages render Markdown, including links, lists, emphasis and code |
| Steer an answer | send another message while the mind is answering to redirect it immediately; the interrupted words stay in the history and the replacement uses your new direction |
| Threads of your own | Add to it: a line for you, sent nowhere; Ask Inkling on the tag, or a line that starts with a mind's name, calls that mind in for one answer; only you tie it off or cut it, and editing its words starts no reread; the minds see it as yours and do not ask it back |
| Write it in | when your reply ties a thread off, its tag offers to put the reply, word for word, under the paragraph it is about, for you to rework; it becomes the thread's answer, with its knot; Undo in the toast, or ⌘Z |
| Tie off | Resolve: the knot is tied and joins the border at the top of the page |
| Let it stand | on a mind's question or correction, Let stand (⇧⌘⌫): what you typed in its box goes with it as your reason, and every mind that reads the note is told the point, so none raises it again, in any words; Find a loose thread looks past it; Undo for a few seconds, or ⌥⌘Z |
| Snip | your own threads, asks, sketches and answers: Dismiss (⇧⌘⌫); Undo for a few seconds, or ⌥⌘Z |
| Reopen, Copy, Make a note | from an open tag; Make a note turns a thread into a new note |
| Jump between threads | ⌥⌘↓ and ⌥⌘↑ |
| Loose ends | ⇧⌘K (or File > Loose Ends…, or the sidebar's Loose ends): every open thread in the notebook, oldest first, showing your words, the mind, the note and the age; type to narrow; Return opens the note at those words with the tag open |
| Threads between notes | when what you write disagrees with, or builds on, words in another note, the thread quotes both: its tab has a second loop, and its tag shows the other words with that note's title and date; Go there (⌥⌘→) opens the other note at those words, where the same thread hangs; reply, tie it off or let it stand at either end |
| Hide threads | ⇧⌘. leaves just your words |
| Where threads go | both margins, the right margin, or marks only (a ? in the text, × for a correction, that opens on click) |
| How many are open | "N loose threads" above the title |
| Turn it over | Turn over above the title, ⇧⌘B or View › Turn Over: the back of the note lists every thread it has had, oldest first, with how each ended (tied off, let stand, snipped, still open) and when, the words before and after, what was said, and your reasons; add or change a reason, Reopen a thread on its words, or Show it on the page; ⇧⌘C copies the back as Markdown; Turn back, ⇧⌘B or Esc returns |

## Asking

| Capability | How |
|---|---|
| Find a loose thread | select words, ⌘↩: the one thing in them most worth a question |
| Ask your own question | select words, Ask Inkling |
| Ask yourself | select words, Ask yourself (⇧⌘↩): your question hangs from them as a thread of your own, in your ink with seed stitches; no mind answers |
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
| Notes and folders | the sidebar (⌘\\): new note, new folder; folder names open a page of their notes, disclosure arrows fold the list (Unfiled's too); drag notes onto folders or Unfiled to move them; notes in a folder show its name above their tools; a dot on notes with open threads |
| Right-click menus | a note (sidebar or a folder's page): Open, Move To, Delete Note; a folder: New Note, Open, Rename… (typed in place), Delete Folder (its notes go to Unfiled); Unfiled: New Note, Open; deletes have Undo in their toast; macOS's own menus, so ⌃-click works too |
| Delete a note | right-click it, File > Delete This Note, or ⌘⌫ on it in ⌘K; Undo in the toast |
| Find or start a note | ⌘K |
| Copy a note | ⇧⌘C |
| Export | ⇧⌘E: Markdown, HTML or Word, with the threads or without; checklists, callouts and code come along (`- [ ]`, `> [!note]`, fenced code in Markdown) |
| Import from Obsidian | File > Import from Obsidian…: a vault's notes come in with their folders, as Obsidian shows them (tasks as checklists, callouts as callouts, code fences as code); their words are left unread, so only what you write afterwards is read, until you ask: Read it through reads them |
| Welcome guide | Help > Welcome Guide |

## Model and keys

| Capability | How |
|---|---|
| Runs on your Claude | the Claude Code login on this Mac; nothing to set up |
| Pick the model and effort | Settings > Model: the models Claude Code offers (from the Agent SDK), kept for when it is not running; reading and answering have their own effort, from the levels the model takes |
| Every shortcut can be changed | Settings > Shortcuts: click keys and press new ones, add or remove keys, clashes flagged on both rows, reset one or all; new keys work at once, menus included |
| Everything from the keyboard | Tab and ⇧Tab move between controls, a running-stitch ring shows where you are, Return or Space presses; the note keeps Tab for lists, so ⌃Tab leaves it; ↑ ↓ in the mind and mode menus |
| Messages in passing | one stack of small stitched labels at the bottom: Undo for a deleted note or a snipped thread, problems in ochre with a close control; the pointer holds a label |

Local files: a note's first line, beside its folder, says what it is linked to (linked to warpbuilds); pressing that chooses the note's own directories, and File > Link Files to This Note… does it for a note linked to nothing. Notes inherit directories chosen on their folder page, until the note overrides them. Use folder setting, on the same line, restores inheritance. Unlink on the folder page removes its mapping. The sidebar shows notebook folders and notes only. A folder's page has a Files control in its header row: it names the first directory, with the full paths in its tooltip; a note's linked words have the same tooltip, which also says whether it inherits. A folder's page lies on a note's own sheet, its title and column where a note's are. Dragging uses whole folder sections as targets, a compact pointer-aligned preview, edge scrolling, and Escape to cancel. Claude Code user instructions load for every call. Reads, Find a loose thread, asks, replies, calls by name and chat can open existing project memories and local files with Read, Glob and Grep, and are told where they are; comparing notes, settling, learning and sketches get no tools. Web lookup retains its separate setting.

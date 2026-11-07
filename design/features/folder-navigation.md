# Folders as a contents page

The first version put a large local-directory settings section ahead of the notes,
then gave the note rows a solid grey hover background. It made the folder feel like
an administration screen. The note's location was another metadata line below its
tools, disconnected from navigation.

The default read was a file-manager dashboard with a settings panel. Two useful
objects offered alternatives: a printed notebook's contents page (titles, a short
first line, page references) and a filing cabinet's folder tab (a persistent label,
a broad receiving area, a clear place to put the sheet). The contents page is the
layout; the folder tab informs dragging in the sidebar.

The first thing on a folder page should be its writing. A compact title and count
lead directly into the notes. Each note has its own serif title, a single-line
preview and a quiet date. A loose-thread count appears only when there are loose
threads. Hover changes the title and reveals an arrow; it does not paint a large
rectangle behind the words. The entire padded row still opens or drags the note.

Linked files are supporting context, after the notes. Their full paths stay visible
there, with Change and Unlink beside the section. A folder without a mapping offers
Link a folder. Those paths do not belong in the sidebar's navigation tree.

On a note, the containing notebook folder is a breadcrumb above its tools. The Files
control names the selected local directory, with a count for additional directories.
Its tooltip carries the full paths and says whether the setting is inherited or
specific to this note. This preserves visible context without turning the space
above the title into a stack of configuration fields.

The surface follows Inkling's existing paper, serif note titles, sans-serif controls,
and one accent for action. The work here is hierarchy and interaction, not another
palette. Light, dark and narrow-window renders are checked alongside scripted
navigation and drag scenarios.

## Second pass, 7 Nov

Three things read as unfinished and are changed. The folder page sat on a narrower
card of its own, with no shadow and no stitched edge, so moving between a folder and a
note moved the column and the title. It now lies on the note's own sheet
(`note_page::sheet_page`), with the note's header row in the note's place: the count,
New note, and the folder's Files control, which works as a note's does, its full paths
in the tooltip. The rows line up with the column on both sides, with no hidden arrow
holding the date away from the edge.

In the sidebar Unfiled was drawn unlike the folders around it (a folder icon where they
have a fold arrow, its name out of line with theirs); it now folds like them. Deleting and
renaming were missing or hidden: a hover × deleted a folder in one click with no Undo.
Both now live in right-click menus, macOS's own, with Undo for every delete.

A note's folder line lost its trailing ›, which pointed at nothing.

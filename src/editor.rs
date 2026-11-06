//! The note editor: a title line plus paragraphs (headings, lists, quotes), inline
//! marks, and thread anchors that stay attached to their words while you type.
//!
//! The document is one string with blocks separated by '\n', so selection, IME and
//! undo work on a single offset space. Each line carries a block kind. Layout is
//! done here (not by a text widget) so the margin knows exactly where every
//! anchored quote sits.
//!
//! The editing logic lives in `State`, which holds no GPUI handles, so every editing
//! path runs headless in the tests at the bottom. `NoteEditor` wraps it with focus,
//! events, layout and painting.

use gpui_kit::*;
use gpui_kit::prelude::*;
use std::collections::HashMap;
use std::{
    cell::RefCell,
    ops::Range,
    rc::Rc,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::store::{BlockKind, Mark, MarkSpan};
use crate::theme::{DISPLAY, Palette, SERIF, Theme};

gpui_kit::actions!(
    editor,
    [
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        MoveWordLeft,
        MoveWordRight,
        SelectWordLeft,
        SelectWordRight,
        MoveLineStart,
        MoveLineEnd,
        SelectLineStart,
        SelectLineEnd,
        MoveDocStart,
        MoveDocEnd,
        SelectDocStart,
        SelectDocEnd,
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        DeleteLineLeft,
        Newline,
        SelectAll,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        ToggleBold,
        ToggleItalic,
        ToggleUnderline,
        ToggleStrike,
        ToggleCode,
        IndentList,
        OutdentList,
        Escape,
        ShowCharacterPalette,
    ]
);

pub const CONTEXT: &str = "NoteEditor";

pub fn bindings() -> Vec<KeyBinding> {
    let c = Some(CONTEXT);
    vec![
        KeyBinding::new("left", MoveLeft, c),
        KeyBinding::new("right", MoveRight, c),
        KeyBinding::new("up", MoveUp, c),
        KeyBinding::new("down", MoveDown, c),
        KeyBinding::new("shift-left", SelectLeft, c),
        KeyBinding::new("shift-right", SelectRight, c),
        KeyBinding::new("shift-up", SelectUp, c),
        KeyBinding::new("shift-down", SelectDown, c),
        KeyBinding::new("alt-left", MoveWordLeft, c),
        KeyBinding::new("alt-right", MoveWordRight, c),
        KeyBinding::new("alt-shift-left", SelectWordLeft, c),
        KeyBinding::new("alt-shift-right", SelectWordRight, c),
        KeyBinding::new("cmd-left", MoveLineStart, c),
        KeyBinding::new("cmd-right", MoveLineEnd, c),
        KeyBinding::new("home", MoveLineStart, c),
        KeyBinding::new("end", MoveLineEnd, c),
        KeyBinding::new("ctrl-a", MoveLineStart, c),
        KeyBinding::new("ctrl-e", MoveLineEnd, c),
        KeyBinding::new("cmd-shift-left", SelectLineStart, c),
        KeyBinding::new("cmd-shift-right", SelectLineEnd, c),
        KeyBinding::new("cmd-up", MoveDocStart, c),
        KeyBinding::new("cmd-down", MoveDocEnd, c),
        KeyBinding::new("cmd-shift-up", SelectDocStart, c),
        KeyBinding::new("cmd-shift-down", SelectDocEnd, c),
        KeyBinding::new("backspace", Backspace, c),
        KeyBinding::new("shift-backspace", Backspace, c),
        KeyBinding::new("delete", Delete, c),
        KeyBinding::new("alt-backspace", DeleteWordLeft, c),
        KeyBinding::new("alt-delete", DeleteWordRight, c),
        KeyBinding::new("cmd-backspace", DeleteLineLeft, c),
        KeyBinding::new("enter", Newline, c),
        KeyBinding::new("shift-enter", Newline, c),
        KeyBinding::new("cmd-a", SelectAll, c),
        KeyBinding::new("cmd-c", Copy, c),
        KeyBinding::new("cmd-x", Cut, c),
        KeyBinding::new("cmd-v", Paste, c),
        KeyBinding::new("cmd-z", Undo, c),
        KeyBinding::new("cmd-shift-z", Redo, c),
        KeyBinding::new("cmd-b", ToggleBold, c),
        KeyBinding::new("cmd-i", ToggleItalic, c),
        KeyBinding::new("cmd-u", ToggleUnderline, c),
        KeyBinding::new("cmd-shift-x", ToggleStrike, c),
        KeyBinding::new("cmd-e", ToggleCode, c),
        KeyBinding::new("tab", IndentList, c),
        KeyBinding::new("shift-tab", OutdentList, c),
        KeyBinding::new("escape", Escape, c),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, c),
    ]
}

#[derive(Clone, Debug)]
pub struct Anchor {
    pub id: String,
    pub range: Range<usize>,
    /// Open threads draw a dashed underline; resolved ones stay attached but quiet.
    pub open: bool,
}

#[derive(Clone)]
struct Snapshot {
    text: String,
    kinds: Vec<BlockKind>,
    marks: Vec<MarkSpan>,
    anchors: Vec<Anchor>,
    selected: Range<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EditKind {
    Insert,
    Delete,
    Other,
}

/// Edits of one kind closer together than this undo as one step.
const COALESCE: Duration = Duration::from_millis(1200);
const UNDO_LIMIT: usize = 300;

#[derive(Clone, Debug)]
enum Drag {
    Char,
    Word(Range<usize>),
    Line(Range<usize>),
}

#[derive(Clone, Debug)]
pub enum EditorEvent {
    /// Text changed; `line` is the block where the edit started.
    Edited { line: usize },
    SelectionChanged,
    /// The mouse button was released after selecting.
    SelectionSettled,
    Focused,
    Blurred,
    Escape,
    /// The person clicked words that a thread is anchored to.
    AnchorClicked(String),
    /// Line positions moved, so anything aligned to them should re-render.
    LayoutChanged,
}

pub struct LineBox {
    pub start: usize,
    pub len: usize,
    pub kind: BlockKind,
    pub top: Pixels,
    pub line_height: Pixels,
    pub indent: Pixels,
    pub font_size: Pixels,
    pub wrapped: WrappedLine,
    /// Local byte index where each visual row begins.
    pub row_starts: Vec<usize>,
}

impl LineBox {
    pub fn rows(&self) -> usize {
        self.row_starts.len()
    }

    pub fn height(&self) -> Pixels {
        self.line_height * self.rows() as f32
    }

    fn row_end(&self, row: usize) -> usize {
        self.row_starts.get(row + 1).copied().unwrap_or(self.len)
    }

    fn row_for_local(&self, local: usize) -> usize {
        let mut row = 0;
        for (i, s) in self.row_starts.iter().enumerate() {
            if *s <= local {
                row = i;
            }
        }
        row
    }

    /// The row a caret at `local` is drawn on. On a soft wrap the caret normally starts
    /// the next row; with upstream affinity it ends the row above.
    fn row_for_caret(&self, local: usize, upstream: bool) -> usize {
        let row = self.row_for_local(local);
        if upstream && row > 0 && self.row_starts[row] == local { row - 1 } else { row }
    }

    fn x_raw(&self, local: usize) -> Pixels {
        self.wrapped.unwrapped_layout.x_for_index(local.min(self.len))
    }

    /// x of a local index within its visual row, relative to the text start.
    fn x_in_row(&self, local: usize, row: usize) -> Pixels {
        self.x_raw(local) - self.x_raw(self.row_starts[row])
    }

    /// Where a caret at the end of `row` goes, and whether it needs upstream affinity.
    /// A row that wrapped after a space ends before that space, so the caret stays on
    /// the row. A row broken inside a run of text (CJK, a long word) ends after its last
    /// character, which is also the next row's start: that needs upstream affinity.
    fn row_end_caret(&self, row: usize) -> (usize, bool) {
        let end = self.row_end(row);
        if row + 1 >= self.rows() {
            return (end, false);
        }
        let text: &str = &self.wrapped.text;
        match text.get(..end).and_then(|s| s.chars().next_back()) {
            Some(c) if c.is_whitespace() => (prev_grapheme_in(text, end).max(self.row_starts[row]), false),
            Some(_) => (end, true),
            None => (end, false),
        }
    }

    /// The caret position closest to `x` on `row`, and its affinity.
    fn local_for_x(&self, row: usize, x: Pixels) -> (usize, bool) {
        let row_start = self.row_starts[row];
        let row_end = self.row_end(row);
        let base = self.x_raw(row_start);
        let ix = self.wrapped.unwrapped_layout.closest_index_for_x(base + x.max(px(0.)));
        let ix = ix.clamp(row_start, row_end);
        // A soft-wrapped row ends where the next one starts; stay on this row.
        if row + 1 < self.rows() && ix >= row_end && row_end > row_start {
            let (end, upstream) = self.row_end_caret(row);
            return (snap_grapheme_in(&self.wrapped.text, end), upstream);
        }
        // Glyph hits can fall inside a cluster (a combining mark has its own glyph).
        (snap_grapheme_in(&self.wrapped.text, ix), false)
    }
}

/// Start of the last grapheme of `s` that begins before `ix`.
fn prev_grapheme_in(s: &str, ix: usize) -> usize {
    s.grapheme_indices(true).map(|(i, _)| i).take_while(|&i| i < ix).last().unwrap_or(0)
}

/// Start of the first grapheme of `s` that begins after `ix` (or the end of `s`).
fn next_grapheme_in(s: &str, ix: usize) -> usize {
    s.grapheme_indices(true).map(|(i, _)| i).find(|&i| i > ix).unwrap_or(s.len())
}

/// `ix`, moved back to the start of the grapheme it falls inside, so a caret never splits
/// a cluster (combining marks, ZWJ emoji, flags).
fn snap_grapheme_in(s: &str, ix: usize) -> usize {
    for (i, g) in s.grapheme_indices(true) {
        if ix < i + g.len() {
            return i.min(ix);
        }
    }
    ix
}

pub struct EditorLayout {
    pub origin: Point<Pixels>,
    pub width: Pixels,
    pub height: Pixels,
    pub lines: Vec<LineBox>,
}

impl EditorLayout {
    pub fn line_of(&self, offset: usize) -> usize {
        let mut ix = 0;
        for (i, l) in self.lines.iter().enumerate() {
            if l.start <= offset {
                ix = i;
            } else {
                break;
            }
        }
        ix
    }

    /// Top-left of the caret position for `offset`, relative to the editor origin.
    pub fn point_of(&self, offset: usize) -> Point<Pixels> {
        self.caret_point(offset, false)
    }

    /// Top-left of the caret at `offset`; with upstream affinity a caret on a soft wrap
    /// sits at the end of the row above instead of the start of the next one.
    pub fn caret_point(&self, offset: usize, upstream: bool) -> Point<Pixels> {
        let li = self.line_of(offset);
        let line = &self.lines[li];
        let local = offset.saturating_sub(line.start).min(line.len);
        let row = line.row_for_caret(local, upstream);
        point(
            line.indent + line.x_in_row(local, row),
            line.top + line.line_height * row as f32,
        )
    }

    pub fn offset_at(&self, p: Point<Pixels>) -> usize {
        self.hit(p).0
    }

    /// The caret position under `p` (relative to the origin) and its affinity.
    fn hit(&self, p: Point<Pixels>) -> (usize, bool) {
        if self.lines.is_empty() {
            return (0, false);
        }
        let mut li = 0;
        for (i, l) in self.lines.iter().enumerate() {
            if p.y >= l.top {
                li = i;
            }
        }
        let line = &self.lines[li];
        if p.y < line.top {
            return (line.start, false);
        }
        let row = (((p.y - line.top) / line.line_height) as usize).min(line.rows() - 1);
        let (local, upstream) = line.local_for_x(row, p.x - line.indent);
        (line.start + local, upstream)
    }

    /// Rectangles covering `range`, one per visual row, relative to the origin.
    pub fn rects(&self, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let mut out = vec![];
        for line in &self.lines {
            let ls = line.start;
            let le = line.start + line.len;
            if range.end < ls || range.start > le {
                continue;
            }
            let a = range.start.max(ls) - ls;
            let b = range.end.min(le) - ls;
            for row in 0..line.rows() {
                let rs = line.row_starts[row];
                let re = line.row_end(row);
                let s = a.max(rs);
                let e = b.min(re);
                if s > e || (s == e && !(range.is_empty() || line.len == 0)) {
                    continue;
                }
                let x0 = line.indent + line.x_in_row(s, row);
                let x1 = line.indent + line.x_in_row(e, row);
                let y = line.top + line.line_height * row as f32;
                out.push(Bounds::from_corners(point(x0, y), point(x1, y + line.line_height)));
            }
        }
        out
    }

    fn signature(&self) -> Vec<(f32, f32)> {
        self.lines
            .iter()
            .map(|l| (l.top.as_f32(), l.height().as_f32()))
            .collect()
    }
}

// ----- editing state ---------------------------------------------------------

/// What an editing step changed, for `NoteEditor` to turn into events and a redraw.
#[derive(Default, Debug, PartialEq)]
struct Fx {
    /// The document changed; the block where the change started.
    edited: Option<usize>,
    /// The caret or the selection moved.
    moved: bool,
    /// A selection gesture finished (mouse up, select all, a scripted selection).
    settled: bool,
    /// Words a thread is anchored to were clicked.
    anchor: Option<String>,
    redraw: bool,
}

impl Fx {
    fn edited(line: usize) -> Self {
        Fx { edited: Some(line), redraw: true, ..Fx::default() }
    }

    fn moved() -> Self {
        Fx { moved: true, redraw: true, ..Fx::default() }
    }

    fn redraw() -> Self {
        Fx { redraw: true, ..Fx::default() }
    }

    fn and(self, o: Fx) -> Fx {
        Fx {
            edited: self.edited.or(o.edited),
            moved: self.moved || o.moved,
            settled: self.settled || o.settled,
            anchor: self.anchor.or(o.anchor),
            redraw: self.redraw || o.redraw,
        }
    }
}

/// The document, selection and history, without GPUI handles.
///
/// Invariants every method keeps: one kind per line, line 0 is the only Title, there are
/// at least two lines, and the selection, the marked (IME) range, marks and anchors all
/// lie inside the text on character boundaries.
struct State {
    text: String,
    kinds: Vec<BlockKind>,
    marks: Vec<MarkSpan>,
    anchors: Vec<Anchor>,
    selected: Range<usize>,
    reversed: bool,
    /// The caret sits on a soft wrap but belongs to the end of the row above.
    upstream: bool,
    marked: Option<Range<usize>>,
    /// The x (from the editor's left edge) that Up and Down aim for.
    goal_x: Option<Pixels>,
    drag: Option<Drag>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: Option<(Instant, EditKind)>,
}

impl State {
    fn new() -> Self {
        Self {
            text: "\n".into(),
            kinds: vec![BlockKind::Title, BlockKind::Paragraph],
            marks: vec![],
            anchors: vec![],
            selected: 0..0,
            reversed: false,
            upstream: false,
            marked: None,
            goal_x: None,
            drag: None,
            undo: vec![],
            redo: vec![],
            last_edit: None,
        }
    }

    fn load(&mut self, lines: Vec<(BlockKind, String)>, marks: Vec<MarkSpan>, anchors: Vec<Anchor>) {
        let mut lines = lines;
        if lines.is_empty() {
            lines.push((BlockKind::Title, String::new()));
        }
        if lines.len() == 1 {
            lines.push((BlockKind::Paragraph, String::new()));
        }
        self.text = lines.iter().map(|(_, t)| t.replace('\n', " ")).collect::<Vec<_>>().join("\n");
        // Only the first line is the title.
        self.kinds = lines
            .iter()
            .enumerate()
            .map(|(i, (k, _))| match (i, *k) {
                (0, _) => BlockKind::Title,
                (_, BlockKind::Title) => BlockKind::Paragraph,
                (_, k) => k,
            })
            .collect();
        // Stored offsets can be stale; keep them inside the text and on character boundaries.
        let text = &self.text;
        self.marks = marks
            .into_iter()
            .filter(|m| m.end <= text.len() && m.start < m.end)
            .filter_map(|m| {
                let start = text.floor_char_boundary(m.start);
                let end = text.ceil_char_boundary(m.end);
                (start < end).then_some(MarkSpan { start, end, mark: m.mark })
            })
            .collect();
        self.anchors = anchors
            .into_iter()
            .map(|mut a| {
                a.range = snap_range(text, a.range);
                a
            })
            .collect();
        self.selected = 0..0;
        self.reversed = false;
        self.upstream = false;
        self.marked = None;
        self.goal_x = None;
        self.drag = None;
        self.undo.clear();
        self.redo.clear();
        // Typing in the new note must not coalesce into the previous note's last edit.
        self.last_edit = None;
    }

    // ----- document access ------------------------------------------------

    fn lines(&self) -> Vec<(BlockKind, String)> {
        self.text
            .split('\n')
            .zip(self.kinds.iter())
            .map(|(t, k)| (*k, t.to_string()))
            .collect()
    }

    fn title(&self) -> String {
        self.text.split('\n').next().unwrap_or("").to_string()
    }

    fn line_count(&self) -> usize {
        self.kinds.len()
    }

    fn line_range(&self, ix: usize) -> Range<usize> {
        line_range_in(&self.text, ix)
    }

    fn line_kind(&self, ix: usize) -> BlockKind {
        self.kinds.get(ix).copied().unwrap_or_default()
    }

    fn line_of(&self, offset: usize) -> usize {
        line_of(&self.text, offset)
    }

    fn cursor(&self) -> usize {
        if self.reversed { self.selected.start } else { self.selected.end }
    }

    fn selected_text(&self) -> String {
        self.text.get(self.selected.clone()).unwrap_or_default().to_string()
    }

    fn clamp(&self, offset: usize) -> usize {
        self.text.floor_char_boundary(offset)
    }

    /// An offset found through the layout, made safe for the current text. The layout is
    /// rebuilt only when the next frame is drawn, so one built before the latest edit can
    /// point past the end, into a character or into a grapheme cluster.
    fn snap(&self, offset: usize) -> usize {
        snap_grapheme(&self.text, offset)
    }

    // ----- selection --------------------------------------------------------

    /// Put the caret at `offset`. An explicit move ends the current typing group for
    /// undo and forgets the column kept for Up and Down.
    fn set_caret(&mut self, offset: usize, upstream: bool) {
        let o = self.clamp(offset);
        self.selected = o..o;
        self.reversed = false;
        self.upstream = upstream;
        self.goal_x = None;
        self.last_edit = None;
    }

    /// Move the selection's moving end (the caret) to `offset`, keeping the other end.
    fn extend_to(&mut self, offset: usize, upstream: bool) {
        let o = self.clamp(offset);
        if self.reversed {
            self.selected.start = o;
        } else {
            self.selected.end = o;
        }
        if self.selected.end < self.selected.start {
            self.reversed = !self.reversed;
            self.selected = self.selected.end..self.selected.start;
        }
        self.upstream = upstream;
        self.goal_x = None;
        self.last_edit = None;
    }

    fn select(&mut self, range: Range<usize>, reversed: bool) {
        let (a, b) = (self.clamp(range.start), self.clamp(range.end));
        self.selected = a.min(b)..a.max(b);
        self.reversed = reversed && a != b;
        self.upstream = false;
        self.goal_x = None;
        self.last_edit = None;
    }

    fn select_all(&mut self) -> Fx {
        self.select(0..self.text.len(), false);
        Fx { settled: true, ..Fx::moved() }
    }

    /// Select the first occurrence of `phrase` as if dragged with the mouse.
    fn select_phrase(&mut self, phrase: &str) -> Option<Fx> {
        if phrase.is_empty() {
            return None;
        }
        let i = self.text.find(phrase)?;
        self.select(i..i + phrase.len(), false);
        Some(Fx { settled: true, ..Fx::moved() })
    }

    // ----- movement ---------------------------------------------------------

    fn move_horizontal(&mut self, right: bool, extend: bool) -> Fx {
        let c = self.cursor();
        if extend {
            let to = if right { next_grapheme(&self.text, c) } else { prev_grapheme(&self.text, c) };
            self.extend_to(to, false);
        } else if self.selected.is_empty() {
            let to = if right { next_grapheme(&self.text, c) } else { prev_grapheme(&self.text, c) };
            self.set_caret(to, false);
        } else {
            let to = if right { self.selected.end } else { self.selected.start };
            self.set_caret(to, false);
        }
        Fx::moved()
    }

    fn move_word(&mut self, right: bool, extend: bool) -> Fx {
        let c = self.cursor();
        let to = if right { next_word(&self.text, c) } else { prev_word(&self.text, c) };
        if extend { self.extend_to(to, false) } else { self.set_caret(to, false) }
        Fx::moved()
    }

    /// Up or Down by one visual row, aiming for the same x on screen.
    fn move_vertical(&mut self, layout: Option<&EditorLayout>, down: bool, extend: bool) -> Fx {
        // Without shift, a selection collapses from the edge in the direction of travel.
        let from = match (extend, self.selected.is_empty()) {
            (false, false) => {
                if down { self.selected.end } else { self.selected.start }
            }
            _ => self.cursor(),
        };
        let upstream = self.upstream && from == self.cursor();
        let (to, up, goal) = vertical_target(&self.text, layout, from, upstream, down, self.goal_x);
        let to = self.snap(to);
        if extend { self.extend_to(to, up) } else { self.set_caret(to, up) }
        self.goal_x = goal;
        Fx::moved()
    }

    /// Start or end of the visual row the caret is on.
    fn move_row_edge(&mut self, layout: Option<&EditorLayout>, end: bool, extend: bool) -> Fx {
        let (row, end_upstream) = row_bounds(&self.text, layout, self.cursor(), self.upstream);
        let (to, up) = if end { (row.end, end_upstream) } else { (row.start, false) };
        if extend { self.extend_to(to, up) } else { self.set_caret(to, up) }
        Fx::moved()
    }

    fn move_doc_edge(&mut self, end: bool, extend: bool) -> Fx {
        let to = if end { self.text.len() } else { 0 };
        if extend { self.extend_to(to, false) } else { self.set_caret(to, false) }
        Fx::moved()
    }

    // ----- editing primitives ---------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            kinds: self.kinds.clone(),
            marks: self.marks.clone(),
            anchors: self.anchors.clone(),
            selected: self.selected.clone(),
        }
    }

    fn restore(&mut self, s: Snapshot) {
        // Threads that exist now keep their open state. Anchors the snapshot knows go back
        // to where they were then. Anchors made since are carried across the change that
        // undo or redo makes, like any other edit, so they stay on their words (or
        // collapse where the words were) instead of keeping offsets meant for other text.
        let (start, old_end, new_end) = diff_span(&self.text, &s.text);
        let changed = start..old_end;
        let n = new_end - start;
        let grow = !s.text[start..new_end].contains('\n');
        let restored: Vec<Anchor> = self
            .anchors
            .iter()
            .map(|a| match s.anchors.iter().find(|b| b.id == a.id) {
                Some(b) => Anchor { open: a.open, ..b.clone() },
                None => Anchor { range: map_anchor(&a.range, &changed, n, grow), ..a.clone() },
            })
            .collect();
        self.text = s.text;
        self.kinds = s.kinds;
        self.marks = s.marks;
        self.anchors = restored;
        let text = &self.text;
        for a in &mut self.anchors {
            a.range = snap_range(text, a.range.clone());
        }
        let (a, b) = (self.clamp(s.selected.start), self.clamp(s.selected.end));
        self.selected = a.min(b)..a.max(b);
        self.reversed = false;
        self.upstream = false;
        self.marked = None;
        self.goal_x = None;
    }

    fn push_undo(&mut self, kind: EditKind) {
        let now = Instant::now();
        let coalesce = matches!(
            self.last_edit,
            Some((t, k)) if k == kind && kind != EditKind::Other && now.duration_since(t) < COALESCE
        );
        if !coalesce || self.undo.is_empty() {
            self.undo.push(self.snapshot());
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last_edit = Some((now, kind));
    }

    /// Replace `range` with `new_text`, keeping block kinds, marks and anchors aligned.
    /// `new_kind` is the kind for lines created by newlines in `new_text`.
    fn replace(&mut self, range: Range<usize>, new_text: &str, new_kind: Option<BlockKind>) -> usize {
        apply_edit(&mut self.text, &mut self.kinds, &mut self.marks, &mut self.anchors, range, new_text, new_kind)
    }

    fn place_after_edit(&mut self, end: usize) {
        self.selected = end..end;
        self.reversed = false;
        self.upstream = false;
        self.marked = None;
        self.goal_x = None;
    }

    fn edit(&mut self, range: Range<usize>, new_text: &str, kind: EditKind) -> Fx {
        self.push_undo(kind);
        let line = self.line_of(range.start);
        let end = self.replace(range, new_text, None);
        self.place_after_edit(end);
        Fx::edited(line)
    }

    fn set_line_kind(&mut self, line: usize, kind: BlockKind) -> Fx {
        if line == 0 || line >= self.kinds.len() || self.kinds[line] == kind || kind == BlockKind::Title {
            return Fx::default();
        }
        self.push_undo(EditKind::Other);
        self.kinds[line] = kind;
        Fx::edited(line)
    }

    /// Apply a block kind to every line touched by the selection; if they all have it
    /// already, they go back to paragraphs.
    fn set_selection_kind(&mut self, kind: BlockKind) -> Fx {
        let first = self.line_of(self.selected.start);
        let mut last = self.line_of(self.selected.end);
        // A selection that ends at the very start of a line does not reach into it.
        if last > first && self.selected.end == line_range_at(&self.text, self.selected.end).start {
            last -= 1;
        }
        // The title keeps its kind; a selection inside it changes nothing.
        let first = first.max(1);
        let last = last.min(self.kinds.len() - 1);
        if last < first || kind == BlockKind::Title {
            return Fx::default();
        }
        let all = (first..=last).all(|l| self.kinds[l] == kind);
        let new = if all && kind != BlockKind::Paragraph { BlockKind::Paragraph } else { kind };
        self.push_undo(EditKind::Other);
        for l in first..=last {
            self.kinds[l] = new;
        }
        Fx::edited(first)
    }

    // ----- deletion ---------------------------------------------------------

    fn backspace(&mut self) -> Fx {
        if !self.selected.is_empty() {
            return self.edit(self.selected.clone(), "", EditKind::Delete);
        }
        let cur = self.cursor();
        let line = self.line_of(cur);
        // At the start of a block: first drop its formatting, then merge upward.
        if line > 0 && cur == line_range_at(&self.text, cur).start && self.line_kind(line) != BlockKind::Paragraph {
            return self.set_line_kind(line, BlockKind::Paragraph);
        }
        if cur == 0 {
            return Fx::default();
        }
        let p = prev_grapheme(&self.text, cur);
        self.edit(p..cur, "", EditKind::Delete)
    }

    fn delete_forward(&mut self) -> Fx {
        if !self.selected.is_empty() {
            return self.edit(self.selected.clone(), "", EditKind::Delete);
        }
        let cur = self.cursor();
        if cur >= self.text.len() {
            return Fx::default();
        }
        let n = next_grapheme(&self.text, cur);
        self.edit(cur..n, "", EditKind::Delete)
    }

    fn delete_word(&mut self, right: bool) -> Fx {
        if !self.selected.is_empty() {
            return self.edit(self.selected.clone(), "", EditKind::Other);
        }
        let cur = self.cursor();
        let range = if right { cur..next_word(&self.text, cur) } else { prev_word(&self.text, cur)..cur };
        if range.is_empty() {
            return Fx::default();
        }
        self.edit(range, "", EditKind::Other)
    }

    fn delete_line_left(&mut self, layout: Option<&EditorLayout>) -> Fx {
        if !self.selected.is_empty() {
            return self.edit(self.selected.clone(), "", EditKind::Other);
        }
        let cur = self.cursor();
        let (row, _) = row_bounds(&self.text, layout, cur, self.upstream);
        let start = if row.start == cur { prev_grapheme(&self.text, cur) } else { row.start };
        if start >= cur {
            return Fx::default();
        }
        self.edit(start..cur, "", EditKind::Other)
    }

    fn newline(&mut self) -> Fx {
        let sel = self.selected.clone();
        let cur = sel.start;
        let line = self.line_of(cur);
        let kind = self.line_kind(line);
        let lr = line_range_at(&self.text, cur);
        let continues = matches!(kind, BlockKind::Bullet | BlockKind::Numbered | BlockKind::Quote);
        // An empty list item or quote ends the list instead of adding another.
        if lr.is_empty() && sel.is_empty() && continues {
            return self.set_line_kind(line, BlockKind::Paragraph);
        }
        let heading = matches!(kind, BlockKind::H1 | BlockKind::H2 | BlockKind::H3);
        // Whether any text follows the selection on its last line (it moves down).
        let rest_empty = sel.end >= line_range_at(&self.text, sel.end).end;
        let (above, below) = if line > 0 && cur == lr.start && !lr.is_empty() {
            // Splitting at the very start of a block keeps the block on the new line. The
            // line left empty above is a paragraph, or another item of the same list or quote.
            (if continues { kind } else { BlockKind::Paragraph }, kind)
        } else if continues || (heading && !rest_empty) {
            // Lists and quotes continue; a heading split in the middle stays a heading.
            (kind, kind)
        } else {
            // After a heading, the title or a paragraph comes a paragraph.
            (kind, BlockKind::Paragraph)
        };
        self.push_undo(EditKind::Other);
        let end = self.replace(sel, "\n", Some(below));
        if line > 0 {
            self.kinds[line] = above;
        }
        self.place_after_edit(end);
        Fx::edited(line)
    }

    fn indent(&mut self) -> Fx {
        let line = self.line_of(self.cursor());
        if line == 0 {
            // Tab from the title moves into the body.
            let r = self.line_range(1);
            self.set_caret(r.start, false);
            return Fx::moved();
        }
        if self.line_kind(line) == BlockKind::Paragraph {
            return self.set_line_kind(line, BlockKind::Bullet);
        }
        Fx::default()
    }

    fn outdent(&mut self) -> Fx {
        let line = self.line_of(self.cursor());
        if self.line_kind(line).is_list() {
            return self.set_line_kind(line, BlockKind::Paragraph);
        }
        Fx::default()
    }

    // ----- clipboard and history ------------------------------------------

    fn cut(&mut self) -> Option<(String, Fx)> {
        if self.selected.is_empty() {
            return None;
        }
        let text = self.selected_text();
        Some((text, self.edit(self.selected.clone(), "", EditKind::Other)))
    }

    fn paste(&mut self, text: &str) -> Fx {
        // Pasting several lines into the title keeps the first as the title and puts the
        // rest in the body (apply_edit gives new lines after the title paragraph kind).
        let text = normalize_newlines(text);
        self.edit(self.selected.clone(), &text, EditKind::Other)
    }

    fn undo(&mut self) -> Fx {
        let Some(s) = self.undo.pop() else { return Fx::default() };
        self.redo.push(self.snapshot());
        self.restore(s);
        self.last_edit = None;
        Fx::edited(self.line_of(self.cursor()))
    }

    fn redo(&mut self) -> Fx {
        let Some(s) = self.redo.pop() else { return Fx::default() };
        self.undo.push(self.snapshot());
        self.restore(s);
        self.last_edit = None;
        Fx::edited(self.line_of(self.cursor()))
    }

    // ----- marks ------------------------------------------------------------

    fn has_mark(&self, mark: Mark) -> bool {
        let r = self.selected.clone();
        !r.is_empty() && covered(&self.marks, mark, r)
    }

    fn toggle_mark(&mut self, mark: Mark) -> Fx {
        let r = self.selected.clone();
        if r.is_empty() {
            return Fx::default();
        }
        self.push_undo(EditKind::Other);
        self.marks = toggled_marks(&self.marks, mark, r.clone());
        Fx::edited(self.line_of(r.start))
    }

    /// Markdown-style shortcuts at the start of a paragraph ("# ", "- ", "1. ", "> ").
    fn try_shortcut(&mut self) -> Fx {
        let cur = self.cursor();
        let line = self.line_of(cur);
        if line == 0 || self.line_kind(line) != BlockKind::Paragraph {
            return Fx::default();
        }
        let lr = line_range_at(&self.text, cur);
        let Some(kind) = shortcut_kind(&self.text[lr.start..cur]) else { return Fx::default() };
        self.push_undo(EditKind::Other);
        self.replace(lr.start..cur, "", None);
        self.kinds[line] = kind;
        self.place_after_edit(lr.start);
        Fx::edited(line)
    }

    // ----- dictation and scripted typing --------------------------------------

    /// Insert text where the caret is (dictation), replacing any selection.
    fn insert_at_cursor(&mut self, text: &str) -> Fx {
        let range = self.selected.clone();
        let (spaced, trailing) = dictation_spacing(&self.text, range.clone(), text);
        let fx = self.edit(range, &spaced, EditKind::Other);
        if trailing {
            // The caret stays after the words, before the space that separates them from
            // what follows.
            let c = self.selected.start - 1;
            self.selected = c..c;
        }
        fx
    }

    /// Type `text` as a new paragraph at the end of the note (or into the empty
    /// last paragraph), like someone typing it.
    fn append_paragraph(&mut self, text: &str) -> Fx {
        let end = self.text.len();
        let last = self.kinds.len() - 1;
        let last_empty = last > 0 && self.line_range(last).is_empty();
        let insert = if last_empty { text.to_string() } else { format!("\n{text}") };
        self.edit(end..end, &insert, EditKind::Other)
    }

    // ----- mouse ------------------------------------------------------------

    fn mouse_down(&mut self, hit: (usize, bool), clicks: usize, shift: bool) -> Fx {
        let (off, upstream) = (self.snap(hit.0), hit.1);
        match clicks {
            2 => {
                let w = word_at(&self.text, off);
                self.select(w.clone(), false);
                self.drag = Some(Drag::Word(w));
            }
            n if n >= 3 => {
                let l = line_range_at(&self.text, off);
                self.select(l.clone(), false);
                self.drag = Some(Drag::Line(l));
            }
            _ => {
                if shift {
                    self.shift_click(off, upstream);
                } else {
                    self.set_caret(off, upstream);
                }
                self.drag = Some(Drag::Char);
            }
        }
        Fx::moved()
    }

    /// Shift-click grows the selection to the click and keeps the far end, as on macOS;
    /// inside the selection it moves the caret end.
    fn shift_click(&mut self, off: usize, upstream: bool) {
        let s = self.selected.clone();
        if off < s.start {
            self.select(off..s.end, true);
        } else if off > s.end {
            self.select(s.start..off, false);
        } else {
            self.extend_to(off, upstream);
            return;
        }
        self.upstream = upstream;
    }

    fn mouse_drag(&mut self, hit: (usize, bool)) -> Fx {
        let Some(drag) = self.drag.clone() else { return Fx::default() };
        let (off, upstream) = (self.snap(hit.0), hit.1);
        match drag {
            Drag::Char => self.extend_to(off, upstream),
            Drag::Word(anchor) => {
                let w = word_at(&self.text, off);
                self.span_select(anchor, w);
            }
            Drag::Line(anchor) => {
                let l = line_range_at(&self.text, off);
                self.span_select(anchor, l);
            }
        }
        Fx::moved()
    }

    fn span_select(&mut self, anchor: Range<usize>, other: Range<usize>) {
        if other.start < anchor.start {
            self.select(other.start..anchor.end, true);
        } else {
            self.select(anchor.start..other.end.max(anchor.end), false);
        }
    }

    fn mouse_up(&mut self, hide_anchors: bool) -> Fx {
        let Some(drag) = self.drag.take() else { return Fx::default() };
        if matches!(drag, Drag::Char) && self.selected.is_empty() && !hide_anchors {
            let at = self.selected.start;
            let hit = self
                .anchors
                .iter()
                .filter(|a| a.open && a.range.start < at && at < a.range.end)
                .min_by_key(|a| a.range.len())
                .map(|a| a.id.clone());
            if let Some(id) = hit {
                return Fx { anchor: Some(id), ..Fx::default() };
            }
        }
        Fx { settled: true, ..Fx::default() }
    }

    // ----- input method (UTF-16 offsets) --------------------------------------

    fn range_from_utf16(&self, r: &Range<usize>) -> Range<usize> {
        let (a, b) = (offset_from_utf16(&self.text, r.start), offset_from_utf16(&self.text, r.end));
        a.min(b)..a.max(b)
    }

    fn range_to_utf16(&self, r: &Range<usize>) -> Range<usize> {
        offset_to_utf16(&self.text, r.start)..offset_to_utf16(&self.text, r.end)
    }

    /// The range an input-method call means: the one it names, else the composition,
    /// else the selection.
    fn ime_target(&self, range16: Option<&Range<usize>>) -> Range<usize> {
        let r = match range16 {
            Some(r) => self.range_from_utf16(r),
            None => self.marked.clone().unwrap_or_else(|| self.selected.clone()),
        };
        let (a, b) = (self.clamp(r.start), self.clamp(r.end));
        a.min(b)..a.max(b)
    }

    fn ime_set_selection(&mut self, range16: &Range<usize>) -> Fx {
        self.selected = self.range_from_utf16(range16);
        self.reversed = false;
        self.upstream = false;
        self.goal_x = None;
        Fx::redraw()
    }

    fn ime_unmark(&mut self) -> Fx {
        self.marked = None;
        Fx::redraw()
    }

    fn ime_replace(&mut self, range16: Option<&Range<usize>>, new_text: &str) -> Fx {
        let range = self.ime_target(range16);
        let composing = self.marked.take().is_some();
        let text = normalize_newlines(new_text);
        if text == "\n" {
            // Return typed through the input system: the Newline action, on the range it
            // names. In the title this moves the rest of the title into the body.
            self.selected = range;
            self.reversed = false;
            return self.newline();
        }
        let fx = if composing {
            // Committing a composition: the snapshot taken when it began covers it.
            let line = self.line_of(range.start);
            let end = self.replace(range, &text, None);
            self.place_after_edit(end);
            self.last_edit = Some((Instant::now(), EditKind::Other));
            Fx::edited(line)
        } else {
            let kind = if text.graphemes(true).count() <= 1 { EditKind::Insert } else { EditKind::Other };
            self.edit(range, &text, kind)
        };
        if text == " " { fx.and(self.try_shortcut()) } else { fx }
    }

    fn ime_replace_and_mark(
        &mut self,
        range16: Option<&Range<usize>>,
        new_text: &str,
        selected16: Option<&Range<usize>>,
    ) -> Fx {
        let range = self.ime_target(range16);
        // A composition is one undo step, however many times the input method rewrites it.
        if self.marked.is_none() {
            self.push_undo(EditKind::Other);
        }
        let line = self.line_of(range.start);
        let end = self.replace(range, new_text, None);
        let start = end - new_text.len();
        self.marked = (!new_text.is_empty()).then_some(start..end);
        let start16 = offset_to_utf16(&self.text, start);
        let len16 = new_text.encode_utf16().count();
        self.selected = match selected16 {
            Some(s) => self.range_from_utf16(&(start16 + s.start.min(len16)..start16 + s.end.min(len16))),
            None => end..end,
        };
        self.reversed = false;
        self.upstream = false;
        self.goal_x = None;
        Fx::edited(line)
    }
}

// ----- text helpers ------------------------------------------------------------

/// Number of the line holding `offset`. Counts '\n' bytes, so any offset is safe.
fn line_of(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())].iter().filter(|&&b| b == b'\n').count()
}

/// Byte range of line `ix`, without its '\n'.
fn line_range_in(text: &str, ix: usize) -> Range<usize> {
    let mut start = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i == ix {
            return start..start + l.len();
        }
        start += l.len() + 1;
    }
    text.len()..text.len()
}

/// Byte range of the line holding `offset`, without its '\n'.
fn line_range_at(text: &str, offset: usize) -> Range<usize> {
    let o = offset.min(text.len());
    let b = text.as_bytes();
    let start = b[..o].iter().rposition(|&c| c == b'\n').map_or(0, |i| i + 1);
    let end = b[o..].iter().position(|&c| c == b'\n').map_or(text.len(), |i| o + i);
    start..end
}

/// Start of the grapheme before `offset` (a '\n' counts as one).
fn prev_grapheme(text: &str, offset: usize) -> usize {
    let o = offset.min(text.len());
    if o == 0 {
        return 0;
    }
    let lr = line_range_at(text, o);
    if o <= lr.start {
        return o - 1;
    }
    lr.start + prev_grapheme_in(&text[lr.clone()], o - lr.start)
}

/// Start of the grapheme after the one at `offset` (a '\n' counts as one).
fn next_grapheme(text: &str, offset: usize) -> usize {
    let o = offset.min(text.len());
    let lr = line_range_at(text, o);
    if o >= lr.end {
        return (o + 1).min(text.len());
    }
    lr.start + next_grapheme_in(&text[lr.clone()], o - lr.start)
}

/// `offset` on a character boundary and outside any grapheme cluster.
fn snap_grapheme(text: &str, offset: usize) -> usize {
    let o = text.floor_char_boundary(offset);
    let lr = line_range_at(text, o);
    lr.start + snap_grapheme_in(&text[lr.clone()], o - lr.start)
}

/// `r` inside `text`, on character boundaries, never reversed.
fn snap_range(text: &str, r: Range<usize>) -> Range<usize> {
    let start = text.floor_char_boundary(r.start);
    start..text.floor_char_boundary(r.end).max(start)
}

/// Start of the word before `offset`, within its line; at a line start, the end of the
/// line above.
fn prev_word(text: &str, offset: usize) -> usize {
    let o = text.floor_char_boundary(offset);
    let line = line_range_at(text, o);
    if o == line.start {
        return prev_grapheme(text, o);
    }
    let mut last = line.start;
    for (i, w) in text[line.start..o].split_word_bound_indices() {
        if w.chars().any(char::is_alphanumeric) {
            last = line.start + i;
        }
    }
    last
}

/// End of the word after `offset`, within its line; at a line end, the start of the
/// line below.
fn next_word(text: &str, offset: usize) -> usize {
    let o = text.floor_char_boundary(offset);
    let line = line_range_at(text, o);
    if o >= line.end {
        return next_grapheme(text, o);
    }
    for (i, w) in text[o..line.end].split_word_bound_indices() {
        if w.chars().any(char::is_alphanumeric) {
            return o + i + w.len();
        }
    }
    line.end
}

/// The word (or run of spaces or punctuation) at `offset`, for double-click. At the end
/// of a word it picks that word, which is where a click on the right half of its last
/// letter lands.
fn word_at(text: &str, offset: usize) -> Range<usize> {
    let o = text.floor_char_boundary(offset);
    let line = line_range_at(text, o);
    let is_word = |w: &str| w.chars().any(char::is_alphanumeric);
    let mut ending_here: Option<(Range<usize>, bool)> = None;
    for (i, w) in text[line.clone()].split_word_bound_indices() {
        let r = line.start + i..line.start + i + w.len();
        if r.end == o {
            ending_here = Some((r, is_word(w)));
        } else if r.start <= o && o < r.end {
            return match ending_here {
                Some((prev, true)) if !is_word(w) => prev,
                _ => r,
            };
        }
    }
    ending_here.map(|(r, _)| r).unwrap_or(o..o)
}

fn offset_from_utf16(text: &str, offset16: usize) -> usize {
    let mut utf8 = 0;
    let mut utf16 = 0;
    for ch in text.chars() {
        if utf16 >= offset16 {
            break;
        }
        utf16 += ch.len_utf16();
        utf8 += ch.len_utf8();
    }
    utf8
}

fn offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16 = 0;
    let mut utf8 = 0;
    for ch in text.chars() {
        if utf8 >= offset {
            break;
        }
        utf8 += ch.len_utf8();
        utf16 += ch.len_utf16();
    }
    utf16
}

/// Where Up or Down from `offset` lands: the visual row above or below (rows of wrapped
/// lines and lines of any height count alike), at the same x from the editor's left edge,
/// so list and quote indents do not shift the caret. Returns the offset, its affinity and
/// the x to aim for on the next vertical move. Before the first layout it moves by
/// logical lines, keeping the column in graphemes.
fn vertical_target(
    text: &str,
    layout: Option<&EditorLayout>,
    offset: usize,
    upstream: bool,
    down: bool,
    goal_x: Option<Pixels>,
) -> (usize, bool, Option<Pixels>) {
    let Some(layout) = layout else {
        return (logical_vertical(text, offset, down), false, None);
    };
    let li = layout.line_of(offset);
    let line = &layout.lines[li];
    let local = offset.saturating_sub(line.start).min(line.len);
    let row = line.row_for_caret(local, upstream);
    let x = goal_x.unwrap_or_else(|| line.indent + line.x_in_row(local, row));
    let target = if down {
        if row + 1 < line.rows() {
            Some((li, row + 1))
        } else {
            (li + 1 < layout.lines.len()).then_some((li + 1, 0))
        }
    } else if row > 0 {
        Some((li, row - 1))
    } else {
        li.checked_sub(1).map(|l| (l, layout.lines[l].rows() - 1))
    };
    match target {
        Some((tl, tr)) => {
            let t = &layout.lines[tl];
            let (local, up) = t.local_for_x(tr, x - t.indent);
            (t.start + local, up, Some(x))
        }
        None if down => (text.len(), false, Some(x)),
        None => (0, false, Some(x)),
    }
}

/// Up or Down by logical line, keeping the column in graphemes.
fn logical_vertical(text: &str, offset: usize, down: bool) -> usize {
    let o = text.floor_char_boundary(offset);
    let line = line_of(text, o);
    let lr = line_range_at(text, o);
    let col = text[lr.start..o.max(lr.start)].graphemes(true).count();
    let target = if down {
        if line + 1 > line_of(text, text.len()) {
            return text.len();
        }
        line + 1
    } else {
        let Some(l) = line.checked_sub(1) else { return 0 };
        l
    };
    let tr = line_range_in(text, target);
    text[tr.clone()].grapheme_indices(true).nth(col).map_or(tr.end, |(i, _)| tr.start + i)
}

/// The visual row around a caret at `offset`: where it starts, and where a caret at its
/// end goes (with that caret's affinity), snapped onto `text`. Before the first layout,
/// the logical line.
fn row_bounds(text: &str, layout: Option<&EditorLayout>, offset: usize, upstream: bool) -> (Range<usize>, bool) {
    let Some(layout) = layout else {
        return (line_range_at(text, offset), false);
    };
    let li = layout.line_of(offset);
    let line = &layout.lines[li];
    let local = offset.saturating_sub(line.start).min(line.len);
    let row = line.row_for_caret(local, upstream);
    let (end, up) = line.row_end_caret(row);
    let start = snap_grapheme(text, line.start + line.row_starts[row]);
    (start..snap_grapheme(text, line.start + end).max(start), up)
}

/// The block kind a markdown shortcut typed at the start of a paragraph asks for.
fn shortcut_kind(prefix: &str) -> Option<BlockKind> {
    Some(match prefix {
        "# " => BlockKind::H1,
        "## " => BlockKind::H2,
        "### " => BlockKind::H3,
        "- " | "* " => BlockKind::Bullet,
        "1. " | "1) " => BlockKind::Numbered,
        "> " => BlockKind::Quote,
        _ => return None,
    })
}

/// Line breaks from other apps ("\r\n", "\r", Unicode line and paragraph separators) as '\n'.
fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace(['\r', '\u{2028}', '\u{2029}'], "\n")
}

/// Spaces dictated words need among the words around them: one before them after a word,
/// one after them before a word. Returns the text to insert and whether a trailing space
/// was added.
fn dictation_spacing(doc: &str, range: Range<usize>, text: &str) -> (String, bool) {
    let (Some(first), Some(last)) = (text.chars().next(), text.chars().next_back()) else {
        return (String::new(), false);
    };
    let before = doc.get(..range.start).and_then(|s| s.chars().next_back());
    let after = doc.get(range.end..).and_then(|s| s.chars().next());
    let lead = matches!(before, Some(c) if !c.is_whitespace() && !"([{“‘".contains(c))
        && !first.is_whitespace()
        && !",.;:!?)]}…”’".contains(first);
    let trail = matches!(after, Some(c) if c.is_alphanumeric()) && !last.is_whitespace();
    let mut out = String::with_capacity(text.len() + 2);
    if lead {
        out.push(' ');
    }
    out.push_str(text);
    if trail {
        out.push(' ');
    }
    (out, trail)
}

/// The single replacement that turns `from` into `to`: (start, end in `from`, end in `to`),
/// on character boundaries of both.
fn diff_span(from: &str, to: &str) -> (usize, usize, usize) {
    let (f, t) = (from.as_bytes(), to.as_bytes());
    let mut start = f.iter().zip(t).take_while(|(a, b)| a == b).count();
    while !from.is_char_boundary(start) || !to.is_char_boundary(start) {
        start -= 1;
    }
    let max = (from.len() - start).min(to.len() - start);
    let mut tail = f.iter().rev().zip(t.iter().rev()).take_while(|(a, b)| a == b).count().min(max);
    while !from.is_char_boundary(from.len() - tail) || !to.is_char_boundary(to.len() - tail) {
        tail -= 1;
    }
    (start, from.len() - tail, to.len() - tail)
}

/// Where `pos` lands when `range` is replaced by `n` bytes. For an insertion, `after`
/// picks the side of the new text. With `sticky`, the start of a replaced range stays
/// before its replacement and its end lands after it (as ProseMirror maps positions), so
/// a span that began or ended with the replaced text begins or ends with the new text.
fn map_pos(pos: usize, range: &Range<usize>, n: usize, after: bool, sticky: bool) -> usize {
    let (a, b) = (range.start, range.end);
    if pos < a {
        return pos;
    }
    if pos > b {
        return pos - (b - a) + n;
    }
    if sticky && a < b {
        if pos == a {
            return a;
        }
        if pos == b {
            return a + n;
        }
    }
    if after { a + n } else { a }
}

/// An anchor across an edit. Anchors never grow at their end, and text replacing the
/// first words of a quote stays in it; across a new paragraph the start does not stick.
fn map_anchor(r: &Range<usize>, range: &Range<usize>, n: usize, single_line: bool) -> Range<usize> {
    let s = map_pos(r.start, range, n, true, single_line);
    let e = map_pos(r.end, range, n, false, false).max(s);
    s..e
}

/// Replace `range` with `new_text`, keeping block kinds, marks and anchors aligned.
/// `new_kind` is the kind for lines created by newlines in `new_text`.
pub(crate) fn apply_edit(
    text: &mut String,
    kinds: &mut Vec<BlockKind>,
    marks: &mut Vec<MarkSpan>,
    anchors: &mut [Anchor],
    range: Range<usize>,
    new_text: &str,
    new_kind: Option<BlockKind>,
) -> usize {
    // Keep the range inside the text, in order, and on character boundaries, so a stale
    // or reversed range can never split a character.
    let (a, b) = (range.start.min(text.len()), range.end.min(text.len()));
    let range = text.floor_char_boundary(a.min(b))..text.ceil_char_boundary(a.max(b));
    // One kind per line; repair a mismatch rather than index past it.
    kinds.resize(line_of(text, text.len()) + 1, BlockKind::Paragraph);
    let start_line = line_of(text, range.start);
    let end_line = line_of(text, range.end);
    let inserted_lines = new_text.matches('\n').count();
    let continue_kind = match kinds.get(start_line).copied().unwrap_or_default() {
        k @ (BlockKind::Bullet | BlockKind::Numbered | BlockKind::Quote) => k,
        _ => BlockKind::Paragraph,
    };
    let kind = new_kind.unwrap_or(continue_kind);
    // Lines start_line+1..=end_line are merged away; new ones are inserted.
    kinds.splice(start_line + 1..end_line + 1, std::iter::repeat_n(kind, inserted_lines));
    text.replace_range(range.clone(), new_text);
    let n = new_text.len();

    // Marks grow when you type at their end (bold keeps going as you type), but not
    // across a new paragraph. Text typed over a selection takes the marks of the first
    // character it replaces. Anchors never grow.
    let single_line = !new_text.contains('\n');
    let mut kept = vec![];
    for m in marks.iter() {
        let (s, e) = if single_line {
            (map_pos(m.start, &range, n, true, true), map_pos(m.end, &range, n, true, true))
        } else {
            (map_pos(m.start, &range, n, true, false), map_pos(m.end, &range, n, false, false))
        };
        if s < e {
            kept.push(MarkSpan { start: s, end: e, mark: m.mark });
        }
    }
    *marks = kept;
    for a in anchors.iter_mut() {
        a.range = map_anchor(&a.range, &range, n, single_line);
    }
    // Keep the title single-line with Title kind.
    if let Some(k) = kinds.first_mut() {
        *k = BlockKind::Title;
    }
    if kinds.len() < 2 {
        text.push('\n');
        kinds.push(BlockKind::Paragraph);
    }
    range.start + n
}

fn covered(marks: &[MarkSpan], mark: Mark, r: Range<usize>) -> bool {
    let mut pos = r.start;
    let mut spans: Vec<&MarkSpan> = marks.iter().filter(|m| m.mark == mark && m.end > r.start && m.start < r.end).collect();
    spans.sort_by_key(|m| m.start);
    for m in spans {
        if m.start > pos {
            return false;
        }
        pos = pos.max(m.end);
        if pos >= r.end {
            return true;
        }
    }
    pos >= r.end
}

fn remove_range(marks: &[MarkSpan], r: Range<usize>, matches: impl Fn(Mark) -> bool) -> Vec<MarkSpan> {
    let mut out = vec![];
    for m in marks {
        if !matches(m.mark) || m.end <= r.start || m.start >= r.end {
            out.push(*m);
            continue;
        }
        if m.start < r.start {
            out.push(MarkSpan { start: m.start, end: r.start, mark: m.mark });
        }
        if m.end > r.end {
            out.push(MarkSpan { start: r.end, end: m.end, mark: m.mark });
        }
    }
    out
}

fn merge_marks(mut marks: Vec<MarkSpan>) -> Vec<MarkSpan> {
    marks.sort_by(|a, b| (format!("{:?}", a.mark), a.start).cmp(&(format!("{:?}", b.mark), b.start)));
    let mut out: Vec<MarkSpan> = vec![];
    for m in marks {
        if let Some(last) = out.last_mut() {
            if last.mark == m.mark && m.start <= last.end {
                last.end = last.end.max(m.end);
                continue;
            }
        }
        out.push(m);
    }
    out
}

/// `marks` with `mark` toggled over `r`: removed where `r` is fully covered, else applied
/// to all of `r`. Colours replace each other, and `Color(0)` (the default colour) clears them.
fn toggled_marks(marks: &[MarkSpan], mark: Mark, r: Range<usize>) -> Vec<MarkSpan> {
    let remove = covered(marks, mark, r.clone());
    let is_color = matches!(mark, Mark::Color(_));
    let mut out = remove_range(marks, r.clone(), |m| m == mark || (is_color && matches!(m, Mark::Color(_))));
    if !remove && mark != Mark::Color(0) {
        out.push(MarkSpan { start: r.start, end: r.end, mark });
        out = merge_marks(out);
    }
    out
}

// ----- the GPUI view -------------------------------------------------------------

/// How a thread's words are stitched (set by the page for every thread it shows).
#[derive(Clone, Debug)]
pub struct AnchorStyle {
    /// The mind's thread colour.
    pub color: Hsla,
    /// A correction starts with a cross-stitch; a question does not.
    pub correction: bool,
}

/// The mark beside a paragraph in the gutter: what Inkling has made of it.
#[derive(Clone, Debug)]
pub enum GutterMark {
    /// Read, nothing to ask: a tacking stitch.
    Read,
    /// Has an open question: a French knot in the mind's thread, with its number.
    Question { number: usize, color: Hsla },
}

pub struct NoteEditor {
    focus: FocusHandle,
    st: State,
    pub active_anchor: Option<String>,
    /// Offsets inside paragraphs being read: the needle works beside them.
    pub thinking: Vec<usize>,
    /// Stitch style per thread id; anchors without one are not drawn.
    pub anchor_styles: HashMap<String, AnchorStyle>,
    /// Read marks by line index.
    pub gutter: Vec<(usize, GutterMark)>,
    /// What the last empty paragraph invites you to write (the first-run note asks
    /// for one sentence); `None` keeps the usual "Just start writing."
    pub placeholder_hint: Option<String>,
    pub layout: Option<Rc<EditorLayout>>,
    layout_sig: Vec<(f32, f32)>,
    cursor_on: bool,
    blink_epoch: usize,
    _blink: Option<Task<()>>,
    was_focused: bool,
    pub hide_anchors: bool,
}

impl EventEmitter<EditorEvent> for NoteEditor {}

impl Focusable for NoteEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl NoteEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            focus: cx.focus_handle(),
            st: State::new(),
            active_anchor: None,
            thinking: vec![],
            anchor_styles: HashMap::new(),
            gutter: vec![],
            placeholder_hint: None,
            layout: None,
            layout_sig: vec![],
            cursor_on: true,
            blink_epoch: 0,
            _blink: None,
            was_focused: false,
            hide_anchors: false,
        };
        this.start_blink(cx);
        this
    }

    /// Turn what an editing step changed into events and a redraw.
    fn apply(&mut self, fx: Fx, cx: &mut Context<Self>) {
        if fx.edited.is_some() || fx.moved {
            self.wake_cursor();
        }
        if let Some(line) = fx.edited {
            cx.emit(EditorEvent::Edited { line });
        }
        if fx.moved {
            cx.emit(EditorEvent::SelectionChanged);
        }
        if fx.settled {
            cx.emit(EditorEvent::SelectionSettled);
        }
        if let Some(id) = fx.anchor {
            cx.emit(EditorEvent::AnchorClicked(id));
        }
        if fx.redraw {
            cx.notify();
        }
    }

    // ----- document access ------------------------------------------------

    pub fn load(
        &mut self,
        lines: Vec<(BlockKind, String)>,
        marks: Vec<MarkSpan>,
        anchors: Vec<Anchor>,
        cx: &mut Context<Self>,
    ) {
        self.st.load(lines, marks, anchors);
        self.layout = None;
        self.active_anchor = None;
        self.thinking.clear();
        cx.notify();
    }

    pub fn text(&self) -> &str {
        &self.st.text
    }

    pub fn lines(&self) -> Vec<(BlockKind, String)> {
        self.st.lines()
    }

    pub fn marks(&self) -> &[MarkSpan] {
        &self.st.marks
    }

    pub fn anchors(&self) -> &[Anchor] {
        &self.st.anchors
    }

    pub fn title(&self) -> String {
        self.st.title()
    }

    pub fn line_count(&self) -> usize {
        self.st.line_count()
    }

    /// Byte range of line `ix` in the joined text.
    pub fn line_range(&self, ix: usize) -> Range<usize> {
        self.st.line_range(ix)
    }

    pub fn line_text(&self, ix: usize) -> String {
        self.st.text[self.st.line_range(ix)].to_string()
    }

    pub fn line_kind(&self, ix: usize) -> BlockKind {
        self.st.line_kind(ix)
    }

    pub fn line_of(&self, offset: usize) -> usize {
        self.st.line_of(offset)
    }

    pub fn cursor(&self) -> usize {
        self.st.cursor()
    }

    pub fn selection(&self) -> Range<usize> {
        self.st.selected.clone()
    }

    pub fn selected_text(&self) -> String {
        self.st.selected_text()
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    pub fn set_anchors(&mut self, anchors: Vec<Anchor>, cx: &mut Context<Self>) {
        let text = &self.st.text;
        self.st.anchors = anchors
            .into_iter()
            .map(|mut a| {
                a.range = snap_range(text, a.range);
                a
            })
            .collect();
        cx.notify();
    }

    pub fn add_anchor(&mut self, mut anchor: Anchor, cx: &mut Context<Self>) {
        anchor.range = snap_range(&self.st.text, anchor.range);
        self.st.anchors.retain(|a| a.id != anchor.id);
        self.st.anchors.push(anchor);
        cx.notify();
    }

    pub fn remove_anchor(&mut self, id: &str, cx: &mut Context<Self>) {
        self.st.anchors.retain(|a| a.id != id);
        cx.notify();
    }

    pub fn set_anchor_open(&mut self, id: &str, open: bool, cx: &mut Context<Self>) {
        if let Some(a) = self.st.anchors.iter_mut().find(|a| a.id == id) {
            a.open = open;
        }
        cx.notify();
    }

    pub fn anchor(&self, id: &str) -> Option<&Anchor> {
        self.st.anchors.iter().find(|a| a.id == id)
    }

    /// Where an anchor sits, relative to the editor origin: its first rectangle.
    pub fn anchor_rects(&self, id: &str) -> Vec<Bounds<Pixels>> {
        let (Some(layout), Some(a)) = (self.layout.as_ref(), self.anchor(id)) else {
            return vec![];
        };
        let mut r = a.range.clone();
        if r.is_empty() {
            r.end = r.start;
        }
        layout.rects(r)
    }

    /// Selection rectangles relative to the editor origin.
    pub fn selection_rects(&self) -> Vec<Bounds<Pixels>> {
        match self.layout.as_ref() {
            Some(l) if !self.st.selected.is_empty() => l.rects(self.st.selected.clone()),
            _ => vec![],
        }
    }

    /// The caret rectangle in window coordinates.
    pub fn cursor_window_rect(&self) -> Option<Bounds<Pixels>> {
        let layout = self.layout.as_ref()?;
        let c = self.st.cursor();
        let p = layout.caret_point(c, self.st.upstream);
        let lh = layout.lines[layout.line_of(c)].line_height;
        Some(Bounds::new(layout.origin + p, size(px(2.), lh)))
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    pub fn move_cursor_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.st.set_caret(offset, false);
        self.apply(Fx::moved(), cx);
    }

    pub fn select_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.st.select(range, false);
        self.apply(Fx::moved(), cx);
    }

    /// Insert text where the caret is (dictation), replacing any selection.
    pub fn insert_at_cursor(&mut self, text: &str, cx: &mut Context<Self>) {
        let fx = self.st.insert_at_cursor(text);
        self.apply(fx, cx);
    }

    /// Select the first occurrence of `phrase` as if dragged with the mouse.
    pub fn select_phrase(&mut self, phrase: &str, cx: &mut Context<Self>) -> bool {
        match self.st.select_phrase(phrase) {
            Some(fx) => {
                self.apply(fx, cx);
                true
            }
            None => false,
        }
    }

    /// Type `text` as a new paragraph at the end of the note (or into the empty
    /// last paragraph), like someone typing it.
    pub fn append_paragraph(&mut self, text: &str, cx: &mut Context<Self>) {
        let fx = self.st.append_paragraph(text);
        self.apply(fx, cx);
    }

    pub fn set_line_kind(&mut self, line: usize, kind: BlockKind, cx: &mut Context<Self>) {
        let fx = self.st.set_line_kind(line, kind);
        self.apply(fx, cx);
    }

    /// Apply a block kind to every line touched by the selection.
    pub fn set_selection_kind(&mut self, kind: BlockKind, cx: &mut Context<Self>) {
        let fx = self.st.set_selection_kind(kind);
        self.apply(fx, cx);
    }

    pub fn current_line_kind(&self) -> BlockKind {
        self.line_kind(self.line_of(self.cursor()))
    }

    pub fn has_mark(&self, mark: Mark) -> bool {
        self.st.has_mark(mark)
    }

    pub fn toggle_mark(&mut self, mark: Mark, cx: &mut Context<Self>) {
        let fx = self.st.toggle_mark(mark);
        self.apply(fx, cx);
    }

    fn wake_cursor(&mut self) {
        self.cursor_on = true;
        self.blink_epoch += 1;
    }

    fn start_blink(&mut self, cx: &mut Context<Self>) {
        self._blink = Some(cx.spawn(async move |this, cx| {
            let mut seen_epoch = usize::MAX;
            loop {
                cx.background_executor().timer(Duration::from_millis(530)).await;
                let alive = this.update(cx, |ed, cx| {
                    if ed.blink_epoch != seen_epoch {
                        seen_epoch = ed.blink_epoch;
                        ed.cursor_on = true;
                    } else {
                        ed.cursor_on = !ed.cursor_on;
                    }
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
    }

    // ----- actions ----------------------------------------------------------

    fn move_left(&mut self, _: &MoveLeft, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_horizontal(false, false);
        self.apply(fx, cx);
    }

    fn move_right(&mut self, _: &MoveRight, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_horizontal(true, false);
        self.apply(fx, cx);
    }

    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_vertical(self.layout.as_deref(), false, false);
        self.apply(fx, cx);
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_vertical(self.layout.as_deref(), true, false);
        self.apply(fx, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_horizontal(false, true);
        self.apply(fx, cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_horizontal(true, true);
        self.apply(fx, cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_vertical(self.layout.as_deref(), false, true);
        self.apply(fx, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_vertical(self.layout.as_deref(), true, true);
        self.apply(fx, cx);
    }

    fn move_word_left(&mut self, _: &MoveWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_word(false, false);
        self.apply(fx, cx);
    }

    fn move_word_right(&mut self, _: &MoveWordRight, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_word(true, false);
        self.apply(fx, cx);
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_word(false, true);
        self.apply(fx, cx);
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_word(true, true);
        self.apply(fx, cx);
    }

    fn move_line_start(&mut self, _: &MoveLineStart, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_row_edge(self.layout.as_deref(), false, false);
        self.apply(fx, cx);
    }

    fn move_line_end(&mut self, _: &MoveLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_row_edge(self.layout.as_deref(), true, false);
        self.apply(fx, cx);
    }

    fn select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_row_edge(self.layout.as_deref(), false, true);
        self.apply(fx, cx);
    }

    fn select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_row_edge(self.layout.as_deref(), true, true);
        self.apply(fx, cx);
    }

    fn move_doc_start(&mut self, _: &MoveDocStart, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_doc_edge(false, false);
        self.apply(fx, cx);
    }

    fn move_doc_end(&mut self, _: &MoveDocEnd, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_doc_edge(true, false);
        self.apply(fx, cx);
    }

    fn select_doc_start(&mut self, _: &SelectDocStart, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_doc_edge(false, true);
        self.apply(fx, cx);
    }

    fn select_doc_end(&mut self, _: &SelectDocEnd, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.move_doc_edge(true, true);
        self.apply(fx, cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.select_all();
        self.apply(fx, cx);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.backspace();
        self.apply(fx, cx);
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.delete_forward();
        self.apply(fx, cx);
    }

    fn delete_word_left(&mut self, _: &DeleteWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.delete_word(false);
        self.apply(fx, cx);
    }

    fn delete_word_right(&mut self, _: &DeleteWordRight, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.delete_word(true);
        self.apply(fx, cx);
    }

    fn delete_line_left(&mut self, _: &DeleteLineLeft, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.delete_line_left(self.layout.as_deref());
        self.apply(fx, cx);
    }

    fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.newline();
        self.apply(fx, cx);
    }

    fn indent_list(&mut self, _: &IndentList, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.indent();
        self.apply(fx, cx);
    }

    fn outdent_list(&mut self, _: &OutdentList, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.outdent();
        self.apply(fx, cx);
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(EditorEvent::Escape);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.st.selected.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.selected_text()));
        }
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((text, fx)) = self.st.cut() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.apply(fx, cx);
        }
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) else {
            return;
        };
        let fx = self.st.paste(&text);
        self.apply(fx, cx);
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.undo();
        self.apply(fx, cx);
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.redo();
        self.apply(fx, cx);
    }

    fn toggle_bold(&mut self, _: &ToggleBold, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_mark(Mark::Bold, cx);
    }
    fn toggle_italic(&mut self, _: &ToggleItalic, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_mark(Mark::Italic, cx);
    }
    fn toggle_underline(&mut self, _: &ToggleUnderline, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_mark(Mark::Underline, cx);
    }
    fn toggle_strike(&mut self, _: &ToggleStrike, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_mark(Mark::Strike, cx);
    }
    fn toggle_code(&mut self, _: &ToggleCode, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_mark(Mark::Code, cx);
    }

    fn show_character_palette(&mut self, _: &ShowCharacterPalette, window: &mut Window, _: &mut Context<Self>) {
        window.show_character_palette();
    }

    // ----- mouse ------------------------------------------------------------

    /// The caret position under a window point, from the last layout.
    fn hit(&self, p: Point<Pixels>) -> Option<(usize, bool)> {
        let layout = self.layout.as_ref()?;
        Some(layout.hit(p - layout.origin))
    }

    fn on_mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let Some(hit) = self.hit(ev.position) else {
            return;
        };
        let fx = self.st.mouse_down(hit, ev.click_count, ev.modifiers.shift);
        self.apply(fx, cx);
    }

    fn on_mouse_move(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.st.drag.is_none() {
            return;
        }
        if ev.pressed_button != Some(MouseButton::Left) {
            self.st.drag = None;
            return;
        }
        let Some(hit) = self.hit(ev.position) else {
            return;
        };
        let fx = self.st.mouse_drag(hit);
        self.apply(fx, cx);
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.mouse_up(self.hide_anchors);
        self.apply(fx, cx);
    }
}

impl EntityInputHandler for NoteEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.st.range_from_utf16(&range_utf16);
        actual_range.replace(self.st.range_to_utf16(&range));
        Some(self.st.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.st.range_to_utf16(&self.st.selected),
            reversed: self.st.reversed,
        })
    }

    fn set_selected_text_range(&mut self, range_utf16: Range<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.ime_set_selection(&range_utf16);
        window.notify_text_selection_changed(&self.focus);
        self.apply(fx, cx);
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.st.text.encode_utf16().count())
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.st.marked.as_ref().map(|r| self.st.range_to_utf16(r))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let fx = self.st.ime_unmark();
        window.notify_text_input_changed(&self.focus);
        self.apply(fx, cx);
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let fx = self.st.ime_replace(range_utf16.as_ref(), new_text);
        self.apply(fx, cx);
        window.notify_text_input_changed(&self.focus);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let fx = self
            .st
            .ime_replace_and_mark(range_utf16.as_ref(), new_text, new_selected_range_utf16.as_ref());
        window.notify_text_input_changed(&self.focus);
        self.apply(fx, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.layout.as_ref()?;
        let range = self.st.range_from_utf16(&range_utf16);
        let rects = layout.rects(range.clone());
        let r = rects.first().copied().or_else(|| {
            let p = layout.point_of(range.start);
            let lh = layout.lines[layout.line_of(range.start)].line_height;
            Some(Bounds::new(p, size(px(1.), lh)))
        })?;
        Some(Bounds::new(layout.origin + r.origin, r.size))
    }

    fn selection_bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Vec<Bounds<Pixels>> {
        let Some(layout) = self.layout.as_ref() else {
            return vec![];
        };
        let range = self.st.range_from_utf16(&range_utf16);
        layout
            .rects(range)
            .into_iter()
            .map(|r| Bounds::new(layout.origin + r.origin, r.size))
            .collect()
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let layout = self.layout.as_ref()?;
        let off = self.st.clamp(layout.offset_at(point - layout.origin));
        Some(offset_to_utf16(&self.st.text, off))
    }
}

impl Render for NoteEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        if focused != self.was_focused {
            self.was_focused = focused;
            cx.emit(if focused { EditorEvent::Focused } else { EditorEvent::Blurred });
        }
        div()
            .id("note-editor")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .w_full()
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::move_word_left))
            .on_action(cx.listener(Self::move_word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::move_line_start))
            .on_action(cx.listener(Self::move_line_end))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::move_doc_start))
            .on_action(cx.listener(Self::move_doc_end))
            .on_action(cx.listener(Self::select_doc_start))
            .on_action(cx.listener(Self::select_doc_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_word_right))
            .on_action(cx.listener(Self::delete_line_left))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::indent_list))
            .on_action(cx.listener(Self::outdent_list))
            .on_action(cx.listener(Self::escape))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::toggle_bold))
            .on_action(cx.listener(Self::toggle_italic))
            .on_action(cx.listener(Self::toggle_underline))
            .on_action(cx.listener(Self::toggle_strike))
            .on_action(cx.listener(Self::toggle_code))
            .on_action(cx.listener(Self::show_character_palette))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .child(EditorElement { editor: cx.entity() })
    }
}

// ----- layout and painting -------------------------------------------------

pub struct LineStyle {
    /// Fraunces Text for words, Fraunces Display for the title and big headings.
    pub family: &'static str,
    pub size: Pixels,
    pub weight: FontWeight,
    pub italic: bool,
    pub line_height: Pixels,
    pub indent: Pixels,
    pub gap_before: Pixels,
    pub gap_after: Pixels,
    pub color: Hsla,
}

pub fn line_style(kind: BlockKind, note: Pixels, p: &Palette) -> LineStyle {
    let n = note.as_f32();
    let base = LineStyle {
        family: SERIF,
        size: note,
        weight: FontWeight::NORMAL,
        italic: false,
        line_height: px((n * 1.62).round()),
        indent: px(0.),
        gap_before: px(0.),
        gap_after: px((n * 0.68).round()),
        color: p.ink,
    };
    match kind {
        BlockKind::Title => LineStyle {
            family: DISPLAY,
            size: px((n * 1.9).round()),
            weight: FontWeight::MEDIUM,
            line_height: px((n * 1.9 * 1.22).round()),
            gap_after: px((n * 1.1).round()),
            ..base
        },
        BlockKind::H1 => LineStyle {
            family: DISPLAY,
            size: px((n * 1.45).round()),
            weight: FontWeight::SEMIBOLD,
            line_height: px((n * 1.45 * 1.3).round()),
            gap_before: px(n * 0.6),
            gap_after: px(n * 0.35),
            ..base
        },
        BlockKind::H2 => LineStyle {
            family: DISPLAY,
            size: px((n * 1.22).round()),
            weight: FontWeight::SEMIBOLD,
            line_height: px((n * 1.22 * 1.35).round()),
            gap_before: px(n * 0.5),
            gap_after: px(n * 0.3),
            ..base
        },
        BlockKind::H3 => LineStyle {
            size: px((n * 1.06).round()),
            weight: FontWeight::SEMIBOLD,
            line_height: px((n * 1.06 * 1.45).round()),
            gap_before: px(n * 0.4),
            gap_after: px(n * 0.25),
            ..base
        },
        BlockKind::Bullet | BlockKind::Numbered => LineStyle {
            indent: px(22.),
            gap_after: px((n * 0.25).round()),
            ..base
        },
        BlockKind::Quote => LineStyle {
            indent: px(16.),
            italic: true,
            color: p.graphite,
            ..base
        },
        BlockKind::Paragraph => base,
    }
}

/// Grey text for an empty line: "Untitled" in the title and "Just start writing." on an
/// empty last paragraph.
fn placeholder(line: usize, last_line: usize, kind: BlockKind) -> Option<&'static str> {
    if line == 0 {
        Some("Untitled")
    } else if line == last_line && kind == BlockKind::Paragraph {
        Some("Just start writing.")
    } else {
        None
    }
}

struct DocSnapshot {
    text: String,
    kinds: Vec<BlockKind>,
    marks: Vec<MarkSpan>,
    marked: Option<Range<usize>>,
    note_size: Pixels,
    palette: Palette,
}

impl DocSnapshot {
    fn of(st: &State, theme: &Theme) -> Self {
        DocSnapshot {
            text: st.text.clone(),
            kinds: st.kinds.clone(),
            marks: st.marks.clone(),
            marked: st.marked.clone(),
            note_size: theme.note_size,
            palette: theme.p,
        }
    }
}

fn runs_for_line(snap: &DocSnapshot, start: usize, end: usize, style: &LineStyle) -> Vec<TextRun> {
    let mut cuts = vec![start, end];
    for m in &snap.marks {
        if m.end > start && m.start < end {
            cuts.push(m.start.max(start));
            cuts.push(m.end.min(end));
        }
    }
    if let Some(mk) = &snap.marked {
        if mk.end > start && mk.start < end {
            cuts.push(mk.start.max(start));
            cuts.push(mk.end.min(end));
        }
    }
    cuts.sort();
    cuts.dedup();
    let mut runs = vec![];
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }
        let active: Vec<Mark> = snap
            .marks
            .iter()
            .filter(|m| m.start <= a && m.end >= b)
            .map(|m| m.mark)
            .collect();
        let bold = active.contains(&Mark::Bold);
        let italic = active.contains(&Mark::Italic) != style.italic;
        let code = active.contains(&Mark::Code);
        let color = active
            .iter()
            .find_map(|m| match m {
                Mark::Color(c) if *c != 0 => Some(crate::theme::text_color(*c, &snap.palette)),
                _ => None,
            })
            .unwrap_or(style.color);
        let marked = snap.marked.as_ref().is_some_and(|mk| mk.start <= a && mk.end >= b);
        let underline = if active.contains(&Mark::Underline) || marked {
            Some(UnderlineStyle { thickness: px(1.), color: None, wavy: false })
        } else {
            None
        };
        let strikethrough = active
            .contains(&Mark::Strike)
            .then(|| StrikethroughStyle { thickness: px(1.), color: None });
        runs.push(TextRun {
            len: b - a,
            font: Font {
                family: if code { "Menlo".into() } else { style.family.into() },
                features: FontFeatures::default(),
                fallbacks: None,
                weight: if bold { FontWeight::BOLD } else { style.weight },
                style: if italic { FontStyle::Italic } else { FontStyle::Normal },
            },
            color,
            background_color: code.then_some(snap.palette.code),
            underline,
            strikethrough,
        });
    }
    if runs.is_empty() {
        runs.push(TextRun {
            len: 0,
            font: Font {
                family: style.family.into(),
                features: FontFeatures::default(),
                fallbacks: None,
                weight: style.weight,
                style: FontStyle::Normal,
            },
            color: style.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
    runs
}

fn layout_doc(snap: &DocSnapshot, width: Pixels, text_system: &WindowTextSystem) -> EditorLayout {
    let mut lines = vec![];
    let mut y = px(0.);
    let mut start = 0;
    for (i, line_text) in snap.text.split('\n').enumerate() {
        let kind = snap.kinds.get(i).copied().unwrap_or_default();
        let st = line_style(kind, snap.note_size, &snap.palette);
        let end = start + line_text.len();
        let runs = runs_for_line(snap, start, end, &st);
        let wrap = (width - st.indent).max(px(40.));
        let shaped = text_system
            .shape_text(SharedString::from(line_text.to_string()), st.size, &runs, Some(wrap), None)
            .ok()
            .and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) })
            .unwrap_or_default();
        let mut row_starts = vec![0];
        for b in shaped.wrap_boundaries() {
            let run = &shaped.unwrapped_layout.runs[b.run_ix];
            row_starts.push(run.glyphs[b.glyph_ix].index);
        }
        y += st.gap_before;
        let lb = LineBox {
            start,
            len: line_text.len(),
            kind,
            top: y,
            line_height: st.line_height,
            indent: st.indent,
            font_size: st.size,
            wrapped: shaped,
            row_starts,
        };
        y += lb.height() + st.gap_after;
        lines.push(lb);
        start = end + 1;
    }
    EditorLayout { origin: point(px(0.), px(0.)), width, height: y, lines }
}

struct EditorElement {
    editor: Entity<NoteEditor>,
}

impl IntoElement for EditorElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

struct Prepaint {
    layout: Rc<EditorLayout>,
}

impl Element for EditorElement {
    type RequestLayoutState = Rc<RefCell<Option<EditorLayout>>>;
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let theme = Theme::get(cx).clone();
        let ed = self.editor.read(cx);
        let snap = Rc::new(DocSnapshot::of(&ed.st, &theme));
        let cache: Rc<RefCell<Option<EditorLayout>>> = Rc::new(RefCell::new(None));
        let cache2 = cache.clone();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let id = window.request_measured_layout(style, move |known, available, window, _cx| {
            let width = known.width.unwrap_or(match available.width {
                AvailableSpace::Definite(w) => w,
                _ => px(600.),
            });
            let layout = layout_doc(&snap, width, window.text_system());
            let h = layout.height;
            *cache2.borrow_mut() = Some(layout);
            size(width, h)
        });
        (id, cache)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        cache: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let mut layout = match cache.borrow_mut().take() {
            Some(l) if (l.width - bounds.size.width).abs() < px(0.5) => l,
            _ => {
                let theme = Theme::get(cx).clone();
                let snap = DocSnapshot::of(&self.editor.read(cx).st, &theme);
                layout_doc(&snap, bounds.size.width, window.text_system())
            }
        };
        layout.origin = bounds.origin;
        let layout = Rc::new(layout);
        let sig = layout.signature();
        let editor = self.editor.clone();
        let l2 = layout.clone();
        let changed = editor.update(cx, |ed, _| {
            let changed = ed.layout_sig != sig;
            ed.layout = Some(l2);
            ed.layout_sig = sig;
            changed
        });
        if changed {
            cx.defer(move |cx| editor.update(cx, |_, cx| cx.emit(EditorEvent::LayoutChanged)));
        }
        Prepaint { layout }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        pre: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let theme = Theme::get(cx).clone();
        let p = theme.p;
        let ed = self.editor.read(cx);
        let focus = ed.focus.clone();
        let focused = focus.is_focused(window);
        let selected = ed.st.selected.clone();
        let cursor = ed.st.cursor();
        let upstream = ed.st.upstream;
        let cursor_on = ed.cursor_on;
        let anchors = if ed.hide_anchors { vec![] } else { ed.st.anchors.clone() };
        let styles = ed.anchor_styles.clone();
        let gutter = if ed.hide_anchors { vec![] } else { ed.gutter.clone() };
        let hint = ed.placeholder_hint.clone();
        let active = ed.active_anchor.clone();
        let thinking = ed.thinking.clone();
        let last_line = ed.st.kinds.len() - 1;
        let layout = pre.layout.clone();
        let origin = bounds.origin;

        window.handle_input(&focus, ElementInputHandler::new(bounds, self.editor.clone()), cx);

        // Active thread: wash its words.
        if let Some(active) = &active {
            if let Some(a) = anchors.iter().find(|a| &a.id == active) {
                for r in layout.rects(a.range.clone()) {
                    let r = Bounds::new(origin + r.origin - point(px(1.), px(-2.)), size(r.size.width + px(2.), r.size.height - px(4.)));
                    window.paint_quad(fill(r, p.wash).corner_radii(px(2.)));
                }
            }
        }

        // Selection.
        if !selected.is_empty() {
            let color = if focused { p.selection } else { p.wash };
            for r in layout.rects(selected.clone()) {
                let w = if r.size.width < px(4.) { px(4.) } else { r.size.width };
                window.paint_quad(fill(Bounds::new(origin + r.origin, size(w, r.size.height)), color));
            }
        }

        // Block decorations, text, placeholders.
        let mut number = 0;
        for (i, line) in layout.lines.iter().enumerate() {
            let line_origin = origin + point(line.indent, line.top);
            match line.kind {
                BlockKind::Bullet => {
                    number = 0;
                    let dot = Bounds::new(
                        origin + point(px(8.), line.top + line.line_height / 2. - px(2.5)),
                        size(px(5.), px(5.)),
                    );
                    window.paint_quad(fill(dot, p.ink).corner_radii(px(2.5)));
                }
                BlockKind::Numbered => {
                    number += 1;
                    let label = format!("{number}.");
                    let run = TextRun {
                        len: label.len(),
                        font: font(SERIF),
                        color: p.graphite,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    let shaped = window.text_system().shape_line(label.into(), line.font_size, &[run], None);
                    let x = line.indent - px(6.) - shaped.width;
                    let _ = shaped.paint(origin + point(x, line.top), line.line_height, TextAlign::Left, None, window, cx);
                }
                BlockKind::Quote => {
                    number = 0;
                    let bar = Bounds::new(origin + point(px(2.), line.top + px(3.)), size(px(2.), line.height() - px(6.)));
                    window.paint_quad(fill(bar, p.rule));
                }
                _ => number = 0,
            }
            if line.len == 0 {
                let ph: Option<String> = if i == last_line && line.kind == BlockKind::Paragraph && hint.is_some() {
                    hint.clone()
                } else {
                    placeholder(i, last_line, line.kind).map(str::to_string)
                };
                if let Some(ph) = ph {
                    let st = line_style(line.kind, theme.note_size, &p);
                    let run = TextRun {
                        len: ph.len(),
                        font: Font { weight: st.weight, ..font(st.family) },
                        color: p.faint.opacity(if i == 0 { 0.55 } else { 0.8 }),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    // Wrapped like the words it stands in for: a long invitation (the first
                    // run's) stays inside the column instead of running into the margin.
                    let wrap = (bounds.size.width - line.indent).max(px(40.));
                    let shaped = window.text_system().shape_text(SharedString::from(ph), st.size, &[run], Some(wrap), None);
                    let mut y = line_origin.y;
                    for row in shaped.into_iter().flatten() {
                        let _ = row.paint(point(line_origin.x, y), line.line_height, TextAlign::Left, None, window, cx);
                        y += line.line_height * (row.wrap_boundaries().len() + 1) as f32;
                    }
                }
            } else {
                let _ = line.wrapped.paint(line_origin, line.line_height, TextAlign::Left, None, window, cx);
            }
        }

        // Thread anchors: a running stitch under the quoted words, in the mind's thread.
        // A correction starts with a cross-stitch. Tied-off threads keep faint stitches,
        // the record of a question answered.
        for a in anchors.iter().filter(|a| !a.range.is_empty()) {
            let Some(style) = styles.get(&a.id) else { continue };
            let is_active = active.as_deref() == Some(a.id.as_str());
            let color = if a.open { style.color.opacity(if is_active || active.is_none() { 1. } else { 0.7 }) } else { style.color.opacity(0.42) };
            let seed = crate::stitch::seed_of(&a.id);
            for (row, r) in layout.rects(a.range.clone()).into_iter().enumerate() {
                let y = origin.y + r.origin.y + r.size.height - px(3.2);
                let x0 = origin.x + r.origin.x;
                let x1 = x0 + r.size.width;
                if x1 - x0 < px(1.) {
                    continue;
                }
                let cross = row == 0 && style.correction && a.open;
                if cross {
                    crate::stitch::cross_stitch(window, point(x0 + px(4.), y), px(3.3), color);
                }
                crate::stitch::running_stitch(window, x0, x1, y, color, seed.wrapping_add(row as u64), cross);
            }
        }

        // The gutter: what Inkling has made of each paragraph.
        let gx = origin.x - px(26.);
        for (line, mark) in &gutter {
            let Some(l) = layout.lines.get(*line) else { continue };
            let cy = origin.y + l.top + l.line_height / 2.;
            match mark {
                GutterMark::Read => crate::stitch::tacking_stitch(window, point(gx, cy), p.faint.opacity(0.75)),
                GutterMark::Question { number, color } => {
                    crate::stitch::french_knot(window, point(gx, cy - px(3.)), px(4.2), *color, 1.);
                    let label = number.to_string();
                    let run = TextRun {
                        len: label.len(),
                        font: Font { weight: FontWeight::SEMIBOLD, ..font(crate::theme::SANS) },
                        color: p.faint,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    let shaped = window.text_system().shape_line(label.into(), px(10.), &[run], None);
                    let _ = shaped.paint(point(gx - shaped.width / 2., cy + px(3.)), px(12.), TextAlign::Left, None, window, cx);
                }
            }
        }

        // The needle works beside a paragraph while Inkling reads it.
        if !thinking.is_empty() {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f32 / 1000.)
                .unwrap_or(0.);
            let phase = (t / 0.9).fract();
            for off in &thinking {
                let l = &layout.lines[layout.line_of(*off)];
                let cy = origin.y + l.top + l.line_height / 2.;
                crate::stitch::needle(window, point(gx, cy), phase, p.graphite);
            }
            window.request_animation_frame();
        }

        // Caret.
        if focused && selected.is_empty() && cursor_on {
            let pt = layout.caret_point(cursor, upstream);
            let li = layout.line_of(cursor);
            let lh = layout.lines[li].line_height;
            let h = (lh * 0.72).max(px(12.));
            let caret = Bounds::new(origin + pt + point(px(0.), (lh - h) / 2.), size(px(1.5), h));
            window.paint_quad(fill(caret, p.ink));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::BlockKind as K;
    use std::sync::Arc;

    fn doc(text: &str) -> (String, Vec<BlockKind>) {
        let mut kinds = vec![BlockKind::Title];
        kinds.extend(text.matches('\n').map(|_| BlockKind::Paragraph));
        (text.to_string(), kinds)
    }

    fn anchor(range: std::ops::Range<usize>) -> Vec<Anchor> {
        vec![Anchor { id: "t".into(), range, open: true }]
    }

    #[std::prelude::v1::test]
    fn anchor_follows_its_words() {
        let (mut text, mut kinds) = doc("Title\nRedis is single threaded.");
        let q = text.find("single").unwrap();
        let mut anchors = anchor(q..q + "single threaded".len());
        let mut marks = vec![];
        // Typing before the quote shifts it; the quote text is unchanged.
        let at = text.find("Redis").unwrap();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut anchors, at..at, "Our ", None);
        assert_eq!(&text[anchors[0].range.clone()], "single threaded");
        // Typing right after the quote does not grow it.
        let end = anchors[0].range.end;
        apply_edit(&mut text, &mut kinds, &mut marks, &mut anchors, end..end, " and fast", None);
        assert_eq!(&text[anchors[0].range.clone()], "single threaded");
        // Typing inside the quote keeps it attached and includes the new word.
        let mid = anchors[0].range.start + "single".len();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut anchors, mid..mid, "-ish", None);
        assert_eq!(&text[anchors[0].range.clone()], "single-ish threaded");
        // Deleting the quote collapses the anchor where it was.
        let r = anchors[0].range.clone();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut anchors, r.clone(), "", None);
        assert!(anchors[0].range.is_empty());
        assert_eq!(anchors[0].range.start, r.start);
    }

    #[std::prelude::v1::test]
    fn newline_splits_and_merges_blocks() {
        let (mut text, mut kinds) = doc("Title\nfirst second");
        let mut marks = vec![];
        let mut anchors = vec![];
        let at = text.find(" second").unwrap();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut anchors, at..at + 1, "\n", None);
        assert_eq!(text, "Title\nfirst\nsecond");
        assert_eq!(kinds, vec![BlockKind::Title, BlockKind::Paragraph, BlockKind::Paragraph]);
        // Backspace at the start of the third line merges it into the second.
        let nl = text.rfind('\n').unwrap();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut anchors, nl..nl + 1, "", None);
        assert_eq!(text, "Title\nfirstsecond");
        assert_eq!(kinds.len(), 2);
    }

    #[std::prelude::v1::test]
    fn lists_continue_on_enter() {
        let mut text = "Title\nitem".to_string();
        let mut kinds = vec![BlockKind::Title, BlockKind::Bullet];
        let end = text.len();
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut vec![], end..end, "\n", None);
        assert_eq!(kinds, vec![BlockKind::Title, BlockKind::Bullet, BlockKind::Bullet]);
    }

    #[std::prelude::v1::test]
    fn bold_keeps_going_while_typing_but_not_across_paragraphs() {
        let (mut text, mut kinds) = doc("Title\nbold");
        let s = text.find("bold").unwrap();
        let mut marks = vec![MarkSpan { start: s, end: s + 4, mark: Mark::Bold }];
        let end = text.len();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], end..end, "er", None);
        assert_eq!(&text[marks[0].start..marks[0].end], "bolder");
        let end = text.len();
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], end..end, "\nplain", None);
        assert_eq!(&text[marks[0].start..marks[0].end], "bolder");
    }

    #[std::prelude::v1::test]
    fn title_stays_a_title() {
        let (mut text, mut kinds) = doc("Title\nbody");
        let nl = text.find('\n').unwrap();
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut vec![], 0..nl + 1, "", None);
        assert_eq!(kinds[0], BlockKind::Title);
        assert!(kinds.len() >= 2, "an empty body paragraph is kept");
    }

    // ----- headless harness -------------------------------------------------

    /// A note holding `text` (the first line is the title), with the caret at the end.
    fn st(text: &str) -> State {
        let mut s = State::new();
        s.load(text.split('\n').map(|l| (K::Paragraph, l.to_string())).collect(), vec![], vec![]);
        s.set_caret(s.text.len(), false);
        s
    }

    fn st_kinds(lines: &[(BlockKind, &str)]) -> State {
        let mut s = State::new();
        s.load(lines.iter().map(|(k, t)| (*k, t.to_string())).collect(), vec![], vec![]);
        s.set_caret(s.text.len(), false);
        s
    }

    /// Type `text` a grapheme at a time, the way keystrokes arrive from the input system.
    fn typ(s: &mut State, text: &str) {
        for g in text.graphemes(true) {
            s.ime_replace(None, g);
        }
    }

    fn at(s: &State, needle: &str) -> usize {
        s.text.find(needle).unwrap_or_else(|| panic!("{needle:?} is not in {:?}", s.text))
    }

    /// What every editing path must keep true.
    fn check(s: &State) {
        let ok = |o: usize| o <= s.text.len() && s.text.is_char_boundary(o);
        assert_eq!(s.text.split('\n').count(), s.kinds.len(), "one kind per line in {:?}", s.text);
        assert!(s.kinds.len() >= 2, "a title and at least one body line");
        assert_eq!(s.kinds[0], K::Title);
        assert!(s.kinds[1..].iter().all(|k| *k != K::Title), "only line 0 is the title: {:?}", s.kinds);
        assert!(s.selected.start <= s.selected.end && ok(s.selected.start) && ok(s.selected.end), "selection {:?} in {:?}", s.selected, s.text);
        if let Some(m) = &s.marked {
            assert!(m.start <= m.end && ok(m.start) && ok(m.end), "marked {m:?} in {:?}", s.text);
        }
        for m in &s.marks {
            assert!(m.start < m.end && ok(m.start) && ok(m.end), "mark {m:?} in {:?}", s.text);
        }
        for a in &s.anchors {
            assert!(a.range.start <= a.range.end && ok(a.range.start) && ok(a.range.end), "anchor {:?} in {:?}", a.range, s.text);
        }
    }

    fn text_system() -> WindowTextSystem {
        WindowTextSystem::new(Arc::new(TextSystem::new(Arc::new(NoopTextSystem::new()))))
    }

    /// Lay `s` out with GPUI's no-op text system: one glyph per character, 0.6 x the font
    /// size wide (astral characters such as emoji twice that). At a 10 px note size a
    /// paragraph glyph is 6 px, the title's 11.4 px and an H1's 9 px; the body starts at y 34.
    fn lay_out(s: &State, ts: &WindowTextSystem, width: f32) -> EditorLayout {
        let snap = DocSnapshot {
            text: s.text.clone(),
            kinds: s.kinds.clone(),
            marks: s.marks.clone(),
            marked: s.marked.clone(),
            note_size: px(10.),
            palette: Palette::light(),
        };
        layout_doc(&snap, px(width), ts)
    }

    // ----- text helpers -------------------------------------------------------

    #[std::prelude::v1::test]
    fn utf16_offsets_round_trip_through_every_kind_of_character() {
        let text = "Ti\nae\u{301}你👍🏽👨‍👩‍👧🇮🇳x";
        for i in (0..=text.len()).filter(|i| text.is_char_boundary(*i)) {
            assert_eq!(offset_from_utf16(text, offset_to_utf16(text, i)), i);
        }
        // An offset inside a surrogate pair rounds up to the next character.
        let thumbs = text.find('👍').unwrap();
        let u = offset_to_utf16(text, thumbs);
        assert_eq!(offset_from_utf16(text, u + 1), thumbs + '👍'.len_utf8());
        assert_eq!(offset_from_utf16(text, 10_000), text.len());
        assert_eq!(offset_to_utf16(text, text.len()), text.encode_utf16().count());
        // Ranges come back in order.
        let s = st(text);
        assert_eq!(s.range_from_utf16(&(5..2)), offset_from_utf16(text, 2)..offset_from_utf16(text, 5));
        assert_eq!(s.range_to_utf16(&(thumbs..thumbs + 4)), u..u + 2);
    }

    #[std::prelude::v1::test]
    fn the_caret_steps_over_whole_graphemes() {
        let text = "T\nae\u{301}👨‍👩‍👧🇮🇳你\nz";
        let (a, e, family, flag, ni, nl) = (2, 3, 6, 24, 32, 35);
        assert_eq!(next_grapheme(text, a), e);
        assert_eq!(next_grapheme(text, e), family, "e and its combining accent are one grapheme");
        assert_eq!(next_grapheme(text, family), flag, "the ZWJ family is one");
        assert_eq!(next_grapheme(text, flag), ni, "the flag pair is one");
        assert_eq!(next_grapheme(text, ni), nl);
        assert_eq!(next_grapheme(text, nl), nl + 1);
        assert_eq!(next_grapheme(text, text.len()), text.len());
        assert_eq!(prev_grapheme(text, nl + 1), nl);
        assert_eq!(prev_grapheme(text, nl), ni);
        assert_eq!(prev_grapheme(text, ni), flag);
        assert_eq!(prev_grapheme(text, flag), family);
        assert_eq!(prev_grapheme(text, family), e);
        assert_eq!(prev_grapheme(text, a), a - 1);
        assert_eq!(prev_grapheme(text, 0), 0);
        // From inside a cluster, out to its edges; snapping goes to its start.
        let inside = family + '👨'.len_utf8();
        assert_eq!(prev_grapheme(text, inside), family);
        assert_eq!(next_grapheme(text, inside), flag);
        assert_eq!(snap_grapheme(text, inside), family);
        assert_eq!(snap_grapheme(text, e + 1), e, "between the e and its accent");
        assert_eq!(snap_grapheme(text, family + 1), family, "inside a character");
        assert_eq!(snap_grapheme(text, flag), flag);
        assert_eq!(snap_grapheme(text, 999), text.len());
    }

    #[std::prelude::v1::test]
    fn words_for_option_arrows_and_double_click() {
        // f2 o3 o4 -5 b6 a7 r8 _9 _10 b11 a12 z13 .14 \n15 \n16 q17 u18 x19
        let text = "T\nfoo-bar  baz.\n\nqux";
        assert_eq!(next_word(text, 2), 5);
        assert_eq!(next_word(text, 5), 9);
        assert_eq!(next_word(text, 9), 14);
        assert_eq!(next_word(text, 14), 15, "punctuation alone stops at the line end");
        assert_eq!(next_word(text, 15), 16, "at a line end, onto the next line");
        assert_eq!(next_word(text, 16), 17);
        assert_eq!(next_word(text, 17), 20);
        assert_eq!(next_word(text, 20), 20);
        assert_eq!(prev_word(text, 20), 17);
        assert_eq!(prev_word(text, 17), 16);
        assert_eq!(prev_word(text, 16), 15);
        assert_eq!(prev_word(text, 15), 11);
        assert_eq!(prev_word(text, 11), 6);
        assert_eq!(prev_word(text, 6), 2);
        assert_eq!(prev_word(text, 2), 1);
        assert_eq!(prev_word(text, 1), 0);
        assert_eq!(word_at(text, 3), 2..5);
        assert_eq!(word_at(text, 5), 2..5, "a click on the right half of the last letter lands at the word's end");
        assert_eq!(word_at(text, 6), 6..9);
        assert_eq!(word_at(text, 9), 6..9);
        assert_eq!(word_at(text, 10), 9..11, "inside a run of spaces, the spaces");
        assert_eq!(word_at(text, 15), 14..15);
        assert_eq!(word_at(text, 16), 16..16, "an empty line");
        assert_eq!(word_at(text, 20), 17..20, "the end of the note");
        let cjk = "T\n你好";
        assert_eq!(word_at(cjk, 5), 5..8, "each ideograph is a word");
        assert_eq!(next_word(cjk, 2), 5);
    }

    #[std::prelude::v1::test]
    fn text_lookups_accept_any_offset() {
        // T0 ï1 \n3 你4 好7 \n10 👍11 🏽15
        let text = "Tï\n你好\n👍🏽";
        for o in 0..text.len() + 3 {
            let _ = (line_of(text, o), line_range_at(text, o), prev_grapheme(text, o), next_grapheme(text, o));
            let _ = (prev_word(text, o), next_word(text, o), word_at(text, o), offset_to_utf16(text, o));
            let _ = (logical_vertical(text, o, true), logical_vertical(text, o, false));
            assert!(text.is_char_boundary(snap_grapheme(text, o)));
        }
        assert_eq!((line_of(text, 3), line_of(text, 4)), (0, 1));
        assert_eq!(line_range_at(text, 4), 4..10);
        assert_eq!(line_range_at(text, 3), 0..3, "a line break belongs to the line it ends");
        assert_eq!(line_range_in(text, 2), 11..text.len());
        assert_eq!(line_range_in(text, 9), text.len()..text.len());
    }

    #[std::prelude::v1::test]
    fn diff_span_finds_the_change_on_character_boundaries() {
        assert_eq!(diff_span("abc", "abc"), (3, 3, 3));
        assert_eq!(diff_span("abXc", "abc"), (2, 3, 2));
        assert_eq!(diff_span("aé", "aè"), (1, 3, 3), "é and è share their first byte");
        assert_eq!(diff_span("", "new"), (0, 0, 3));
        assert_eq!(diff_span("aaa", "aa"), (2, 3, 2));
    }

    // ----- apply_edit -----------------------------------------------------------

    #[std::prelude::v1::test]
    fn apply_edit_takes_reversed_stale_and_split_ranges() {
        // T0 \n1 n2 a3 ï4 v6 e7 _8 你9 好12
        let (mut text, mut kinds) = doc("T\nnaïve 你好");
        // Reversed, with both ends inside characters: widened to whole characters.
        let end = apply_edit(&mut text, &mut kinds, &mut vec![], &mut vec![], 10..5, "X", None);
        assert_eq!((text.as_str(), end), ("T\nnaX好", 5));
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut vec![], 100..200, "!", None);
        assert_eq!(text, "T\nnaX好!", "past the end means the end");
        // Kinds out of step with the lines are repaired, not indexed past.
        let mut short = vec![K::Title];
        apply_edit(&mut text, &mut short, &mut vec![], &mut vec![], 2..2, "a\nb", None);
        assert_eq!(short.len(), text.split('\n').count());
        let mut long = vec![K::Title, K::Paragraph, K::Bullet, K::Bullet, K::Quote];
        apply_edit(&mut text, &mut long, &mut vec![], &mut vec![], 0..0, "", None);
        assert_eq!(long.len(), text.split('\n').count());
    }

    #[std::prelude::v1::test]
    fn typing_over_a_selection_takes_the_marks_of_what_it_replaces() {
        let bold = |start, end| MarkSpan { start, end, mark: Mark::Bold };
        // "**bold**plain": typing over "plain" does not make it bold.
        let (mut text, mut kinds) = doc("T\nboldplain");
        let mut marks = vec![bold(2, 6)];
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 6..11, "X", None);
        assert_eq!((text.as_str(), marks.clone()), ("T\nboldX", vec![bold(2, 6)]));
        // Typing over the bold word keeps it bold, and typing on keeps going.
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 2..6, "Y", None);
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 3..3, "y", None);
        assert_eq!((text.as_str(), marks.clone()), ("T\nYyX", vec![bold(2, 4)]));
        // A selection that starts in bold gives bold text; one that starts outside does not.
        let (mut text, mut kinds) = doc("T\nabcdef");
        let mut marks = vec![bold(2, 5)];
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 4..7, "Z", None);
        assert_eq!((text.as_str(), marks.clone()), ("T\nabZf", vec![bold(2, 5)]));
        let (mut text, mut kinds) = doc("T\nabcdef");
        let mut marks = vec![bold(5, 8)];
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 3..6, "Z", None);
        assert_eq!((text.as_str(), marks.clone()), ("T\naZef", vec![bold(4, 6)]));
        // Pasting lines over the end of a bold word does not carry bold into them.
        let (mut text, mut kinds) = doc("T\nabc");
        let mut marks = vec![bold(2, 5)];
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 4..5, "X\nY", None);
        assert_eq!((text.as_str(), marks.clone()), ("T\nabX\nY", vec![bold(2, 4)]));
        // Deleting all of a mark removes it.
        apply_edit(&mut text, &mut kinds, &mut marks, &mut vec![], 2..4, "", None);
        assert!(marks.is_empty());
    }

    #[std::prelude::v1::test]
    fn anchors_keep_their_words_through_replacements_and_merges() {
        let (mut text, mut kinds) = doc("T\nRedis is single threaded.");
        let mut anchors = anchor(2..2 + "Redis is single".len());
        // Double-click the first word and type over it: the new word stays in the quote.
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut anchors, 2..7, "V", None);
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut anchors, 3..3, "alkey", None);
        assert_eq!(&text[anchors[0].range.clone()], "Valkey is single");
        // Typing over the last word does not grow the quote past where it ended.
        let s = text.find("single").unwrap();
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut anchors, s..s + 6, "m", None);
        assert_eq!(&text[anchors[0].range.clone()], "Valkey is ");
        // Merging lines: the anchored words move with their line.
        let (mut text, mut kinds) = doc("T\nfoo\nbar baz");
        let mut anchors = anchor(6..9);
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut anchors, 5..6, "", None);
        assert_eq!((text.as_str(), &text[anchors[0].range.clone()]), ("T\nfoobar baz", "bar"));
        // An anchor across a line break keeps both halves when the break goes.
        let (mut text, mut kinds) = doc("T\nfoo\nbar");
        let mut anchors = anchor(3..8);
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut anchors, 5..6, "", None);
        assert_eq!(&text[anchors[0].range.clone()], "ooba");
        // Enter right before the quote takes it to the new line.
        let (mut text, mut kinds) = doc("T\nsee this");
        let mut anchors = anchor(6..10);
        apply_edit(&mut text, &mut kinds, &mut vec![], &mut anchors, 6..6, "\n", None);
        assert_eq!(&text[anchors[0].range.clone()], "this");
        assert_eq!(line_of(&text, anchors[0].range.start), 2);
    }

    // ----- typing, undo and the input method ------------------------------------

    #[std::prelude::v1::test]
    fn typing_is_one_undo_step_until_the_caret_moves() {
        let mut s = st("T\n");
        typ(&mut s, "abc");
        assert_eq!((s.text.as_str(), s.undo.len()), ("T\nabc", 1), "a burst of typing undoes in one step");
        // Moving the caret ends the group, even inside the coalescing window.
        s.move_horizontal(false, false);
        typ(&mut s, "x");
        assert_eq!((s.text.as_str(), s.undo.len()), ("T\nabxc", 2));
        s.undo();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("T\nabc", 4..4));
        s.undo();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("T\n", 2..2));
        assert_eq!(s.undo(), Fx::default(), "nothing left to undo");
        s.redo();
        s.redo();
        assert_eq!(s.text, "T\nabxc");
        assert_eq!(s.redo(), Fx::default());
        // A new edit clears redo.
        s.undo();
        typ(&mut s, "!");
        assert!(s.redo.is_empty());
        // Deleting is a separate group from typing.
        let mut s = st("T\n");
        typ(&mut s, "abc");
        s.backspace();
        s.backspace();
        assert_eq!(s.undo.len(), 2);
        s.undo();
        assert_eq!(s.text, "T\nabc");
        // After a pause longer than the window, typing starts a new group.
        let mut s = st("T\n");
        typ(&mut s, "ab");
        if let Some(t) = Instant::now().checked_sub(COALESCE * 2) {
            s.last_edit = Some((t, EditKind::Insert));
            typ(&mut s, "c");
            assert_eq!(s.undo.len(), 2);
        }
        check(&s);
    }

    #[std::prelude::v1::test]
    fn a_composition_is_one_undo_step_and_reports_utf16() {
        // An emoji before the composition makes UTF-8 and UTF-16 offsets differ.
        let mut s = st("T\n👍");
        let start = s.text.len();
        s.ime_replace_and_mark(None, "n", Some(&(1..1)));
        s.ime_replace_and_mark(None, "ni", Some(&(2..2)));
        s.ime_replace_and_mark(None, "你", Some(&(1..1)));
        assert_eq!(&s.text[start..], "你");
        assert_eq!((s.marked.clone(), s.selected.clone()), (Some(start..start + 3), start + 3..start + 3));
        assert_eq!(s.range_to_utf16(s.marked.as_ref().unwrap()), 4..5);
        // The selection inside the composition is relative to it, in UTF-16.
        s.ime_replace_and_mark(None, "你好", Some(&(0..1)));
        assert_eq!(s.selected, start..start + 3);
        s.ime_replace(None, "你好");
        assert_eq!((s.text.as_str(), s.marked.clone()), ("T\n👍你好", None));
        assert_eq!(s.selected, s.text.len()..s.text.len());
        assert_eq!(s.undo.len(), 1, "not one step per keystroke of the composition");
        s.undo();
        assert_eq!(s.text, "T\n👍", "undo never brings back half-composed text");
        // Leaving a composition keeps its text.
        let mut s = st("T\n");
        s.ime_replace_and_mark(None, "か", None);
        s.ime_unmark();
        assert_eq!((s.text.as_str(), s.marked.clone()), ("T\nか", None));
        typ(&mut s, "x");
        assert_eq!(s.text, "T\nかx");
        // Emptying the marked text cancels the composition.
        let mut s = st("T\nab");
        s.ime_replace_and_mark(None, "zz", None);
        s.ime_replace_and_mark(None, "", None);
        assert_eq!((s.text.as_str(), s.marked.clone()), ("T\nab", None));
        // A composition replaces the selection; an explicit UTF-16 range wins.
        let mut s = st("T\nhello world");
        s.select(2..7, false);
        s.ime_replace_and_mark(None, "y", None);
        assert_eq!(s.text, "T\ny world");
        let mut s = st("T\nhello");
        s.ime_replace(Some(&(2..7)), "bye");
        assert_eq!(s.text, "T\nbye");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn return_through_the_input_system_is_a_newline_on_the_range_it_names() {
        let mut s = st("T\nab");
        s.ime_replace_and_mark(None, "ni", None);
        s.ime_replace(None, "\n");
        assert_eq!((s.text.as_str(), s.marked.clone(), s.selected.clone()), ("T\nab\n", None, 5..5));
        // A lone carriage return is a newline too, and "\r\n" is one newline.
        let mut s = st("T\nab");
        s.ime_replace(None, "\r");
        assert_eq!(s.text, "T\nab\n");
        s.ime_replace(None, "\r\n");
        assert_eq!(s.text, "T\nab\n\n");
        // In a list it continues the list, like the Return key.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Bullet, "item")]);
        s.ime_replace(None, "\n");
        assert_eq!(s.kinds, [K::Title, K::Bullet, K::Bullet]);
        check(&s);
    }

    // ----- blocks -----------------------------------------------------------------

    #[std::prelude::v1::test]
    fn the_title_stays_one_line() {
        // Enter in the middle of the title moves the rest into the body as a paragraph.
        let mut s = st("Title words\nbody");
        s.set_caret(5, false);
        s.newline();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("Title\n words\nbody", 6..6));
        assert_eq!(s.kinds, [K::Title, K::Paragraph, K::Paragraph]);
        // Enter at its end opens an empty first paragraph.
        let mut s = st("Title\nbody");
        s.set_caret(5, false);
        s.newline();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("Title\n\nbody", 6..6));
        // Tab from the title goes to the start of the body and changes no kind.
        let mut s = st("Title\nbody");
        s.set_caret(2, false);
        s.indent();
        assert_eq!((s.selected.clone(), s.kinds.clone()), (6..6, vec![K::Title, K::Paragraph]));
        s.set_caret(2, false);
        assert_eq!(s.outdent(), Fx::default());
        // Pasting lines into the title keeps the first as the title.
        let mut s = st("My note\nbody");
        s.set_caret(3, false);
        s.paste("big\r\nfat\rlines");
        assert_eq!(s.text, "My big\nfat\nlinesnote\nbody");
        assert_eq!(s.kinds, [K::Title, K::Paragraph, K::Paragraph, K::Paragraph]);
        // Block kinds never reach the title, even from a selection inside it.
        let mut s = st("Title\nbody");
        s.select(0..5, false);
        assert_eq!(s.set_selection_kind(K::H1), Fx::default());
        assert_eq!(s.set_line_kind(0, K::H1), Fx::default());
        assert_eq!(s.kinds, [K::Title, K::Paragraph]);
        assert!(s.undo.is_empty(), "no empty undo steps");
        // Deleting everything leaves an empty title and paragraph; typing over it all makes a title.
        let mut s = st_kinds(&[(K::Title, "Title"), (K::H1, "x"), (K::Paragraph, "body")]);
        s.select_all();
        s.backspace();
        assert_eq!((s.text.as_str(), s.kinds.clone()), ("\n", vec![K::Title, K::Paragraph]));
        s.select_all();
        s.paste("A\nB");
        assert_eq!((s.text.as_str(), s.kinds.clone()), ("A\nB", vec![K::Title, K::Paragraph]));
        // A markdown shortcut typed in the title is just text.
        let mut s = st("\n");
        s.set_caret(0, false);
        typ(&mut s, "# x");
        assert_eq!((s.text.as_str(), s.kinds[0]), ("# x\n", K::Title));
        check(&s);
    }

    #[std::prelude::v1::test]
    fn enter_splits_each_kind_of_block() {
        let mut s = st("T\nfirst second");
        s.set_caret(7, false);
        s.newline();
        assert_eq!((s.text.as_str(), s.kinds.clone()), ("T\nfirst\n second", vec![K::Title, K::Paragraph, K::Paragraph]));
        // At the very start of a heading, the heading moves down below an empty paragraph.
        let mut s = st_kinds(&[(K::Title, "T"), (K::H1, "Heading")]);
        s.set_caret(2, false);
        s.newline();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("T\n\nHeading", 3..3));
        assert_eq!(s.kinds, [K::Title, K::Paragraph, K::H1]);
        // In the middle of a heading both halves stay headings; at its end a paragraph follows.
        let mut s = st_kinds(&[(K::Title, "T"), (K::H2, "Chapter One")]);
        s.set_caret(9, false);
        s.newline();
        assert_eq!(s.kinds, [K::Title, K::H2, K::H2]);
        s.set_caret(s.text.len(), false);
        s.newline();
        assert_eq!(s.kinds, [K::Title, K::H2, K::H2, K::Paragraph]);
        // On an empty heading the heading stays and a paragraph follows.
        let mut s = st_kinds(&[(K::Title, "T"), (K::H3, "")]);
        s.newline();
        assert_eq!(s.kinds, [K::Title, K::H3, K::Paragraph]);
        // Lists continue, also from the start of an item, so the numbering does not restart.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Numbered, "one"), (K::Numbered, "two")]);
        s.set_caret(at(&s, "two"), false);
        s.newline();
        assert_eq!(s.text, "T\none\n\ntwo");
        assert_eq!(s.kinds, [K::Title, K::Numbered, K::Numbered, K::Numbered]);
        // Enter on an empty item ends the list; on an empty quote line, the quote.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Bullet, "a"), (K::Bullet, "")]);
        s.newline();
        assert_eq!((s.text.as_str(), s.kinds.clone()), ("T\na\n", vec![K::Title, K::Bullet, K::Paragraph]));
        let mut s = st_kinds(&[(K::Title, "T"), (K::Quote, "q")]);
        s.newline();
        assert_eq!(s.kinds, [K::Title, K::Quote, K::Quote]);
        s.newline();
        assert_eq!(s.kinds, [K::Title, K::Quote, K::Paragraph]);
        // Enter over a selection replaces it with the break.
        let mut s = st("T\nabc def");
        s.select(3..6, false);
        s.newline();
        assert_eq!(s.text, "T\na\ndef");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn backspace_unformats_then_merges_and_delete_pulls_up() {
        let mut s = st_kinds(&[(K::Title, "T"), (K::Paragraph, "para"), (K::Quote, "quote")]);
        s.set_caret(at(&s, "quote"), false);
        s.backspace();
        assert_eq!((s.text.as_str(), s.kinds[2]), ("T\npara\nquote", K::Paragraph));
        s.backspace();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("T\nparaquote", 6..6));
        assert_eq!(s.kinds, [K::Title, K::Paragraph]);
        s.set_caret(0, false);
        assert_eq!(s.backspace(), Fx::default(), "nothing before the start");
        // Merging the only body line into the title keeps an empty body line.
        let mut s = st("Title\nbody");
        s.set_caret(6, false);
        s.backspace();
        assert_eq!((s.text.as_str(), s.selected.clone()), ("Titlebody\n", 5..5));
        assert_eq!(s.kinds, [K::Title, K::Paragraph]);
        // Backspace and Delete take whole graphemes.
        let mut s = st("T\nx👨‍👩‍👧e\u{301}y");
        s.set_caret(at(&s, "y"), false);
        s.backspace();
        s.backspace();
        assert_eq!(s.text, "T\nxy");
        s.set_caret(2, false);
        s.delete_forward();
        assert_eq!(s.text, "T\ny");
        // Delete at the end of a line pulls the next one up; at the end of the note, nothing.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Paragraph, "a"), (K::H1, "b")]);
        s.set_caret(3, false);
        s.delete_forward();
        assert_eq!((s.text.as_str(), s.kinds.clone()), ("T\nab", vec![K::Title, K::Paragraph]));
        s.set_caret(s.text.len(), false);
        assert_eq!(s.delete_forward(), Fx::default());
        s.select(2..3, false);
        s.delete_forward();
        assert_eq!(s.text, "T\nb");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn word_and_line_deletes() {
        let mut s = st("T\nhello big world");
        s.delete_word(false);
        assert_eq!(s.text, "T\nhello big ");
        s.delete_word(false);
        assert_eq!(s.text, "T\nhello ");
        s.set_caret(2, false);
        s.delete_word(true);
        assert_eq!(s.text, "T\n ");
        // At the start of a line, a word delete joins it to the line above.
        let mut s = st("T\nab\ncd");
        s.set_caret(5, false);
        s.delete_word(false);
        assert_eq!(s.text, "T\nabcd");
        // At the very start there is nothing to delete, and no undo step is taken.
        let undo = s.undo.len();
        s.set_caret(0, false);
        assert_eq!(s.delete_word(false), Fx::default());
        assert_eq!(s.undo.len(), undo);
        // Before the first layout, Cmd-Backspace deletes to the line start, then the break.
        let mut s = st("T\nab\ncd");
        s.delete_line_left(None);
        assert_eq!(s.text, "T\nab\n");
        s.delete_line_left(None);
        assert_eq!(s.text, "T\nab");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn markdown_shortcuts_only_at_the_start_of_a_paragraph() {
        let cases = [
            ("# ", K::H1),
            ("## ", K::H2),
            ("### ", K::H3),
            ("- ", K::Bullet),
            ("* ", K::Bullet),
            ("1. ", K::Numbered),
            ("1) ", K::Numbered),
            ("> ", K::Quote),
        ];
        for (typed, kind) in cases {
            let mut s = st("T\n");
            typ(&mut s, typed);
            typ(&mut s, "x");
            assert_eq!((s.text.as_str(), s.kinds[1]), ("T\nx", kind), "{typed:?}");
            // Undo gives the typed characters back as text.
            s.undo();
            s.undo();
            assert_eq!((s.text.clone(), s.kinds[1]), (format!("T\n{typed}"), K::Paragraph), "{typed:?}");
        }
        // Not mid-line, not on a heading, not without the space or for other prefixes.
        let mut s = st("T\na");
        typ(&mut s, "# ");
        assert_eq!((s.text.as_str(), s.kinds[1]), ("T\na# ", K::Paragraph));
        let mut s = st_kinds(&[(K::Title, "T"), (K::H2, "")]);
        typ(&mut s, "- ");
        assert_eq!((s.text.as_str(), s.kinds[1]), ("T\n- ", K::H2));
        let mut s = st("T\n");
        typ(&mut s, "#x 2. ");
        assert_eq!((s.text.as_str(), s.kinds[1]), ("T\n#x 2. ", K::Paragraph));
        // Typed in front of existing text, the shortcut formats that text.
        let mut s = st("T\nheading");
        s.set_caret(2, false);
        typ(&mut s, "# ");
        assert_eq!((s.text.as_str(), s.kinds[1], s.selected.clone()), ("T\nheading", K::H1, 2..2));
    }

    #[std::prelude::v1::test]
    fn block_kinds_apply_to_the_selected_lines() {
        let mut s = st_kinds(&[(K::Title, "T"), (K::H1, "one"), (K::Paragraph, "two"), (K::Paragraph, "three")]);
        // A selection ending at the very start of "three" covers "one" and "two" only, and
        // a mixed selection gets the kind on every line rather than swapping kinds.
        s.select(2..at(&s, "three"), false);
        s.set_selection_kind(K::H1);
        assert_eq!(s.kinds, [K::Title, K::H1, K::H1, K::Paragraph]);
        // When every line has the kind already, they go back to paragraphs.
        s.set_selection_kind(K::H1);
        assert_eq!(s.kinds, [K::Title, K::Paragraph, K::Paragraph, K::Paragraph]);
        // From the title into the body: the title keeps its kind.
        s.select(0..at(&s, "wo"), false);
        s.set_selection_kind(K::Quote);
        assert_eq!(s.kinds, [K::Title, K::Quote, K::Quote, K::Paragraph]);
        // A caret alone applies to its line.
        s.set_caret(at(&s, "three"), false);
        s.set_selection_kind(K::Bullet);
        assert_eq!(s.kinds[3], K::Bullet);
        assert_eq!(s.set_selection_kind(K::Title), Fx::default(), "Title is never applied to the body");
        assert_eq!(s.set_line_kind(2, K::Title), Fx::default());
        check(&s);
    }

    // ----- marks and anchors --------------------------------------------------------

    #[std::prelude::v1::test]
    fn marks_toggle_merge_and_colours_replace_each_other() {
        let span = |start, end, mark| MarkSpan { start, end, mark };
        let mut s = st("T\nhello world");
        s.select(2..7, false);
        s.toggle_mark(Mark::Bold);
        assert!(s.has_mark(Mark::Bold));
        assert_eq!(s.marks, [span(2, 7, Mark::Bold)]);
        // Partly bold becomes all bold, merged into one span.
        s.select(5..13, false);
        assert!(!s.has_mark(Mark::Bold));
        s.toggle_mark(Mark::Bold);
        assert_eq!(s.marks, [span(2, 13, Mark::Bold)]);
        // All bold: the toggle takes it off just the selection.
        s.select(8..13, false);
        s.toggle_mark(Mark::Bold);
        assert_eq!(s.marks, [span(2, 8, Mark::Bold)]);
        // A colour replaces another; Color(0), the default ink, clears them.
        s.select(2..7, false);
        s.toggle_mark(Mark::Color(0xb23b33));
        s.toggle_mark(Mark::Color(0x33689c));
        let colours: Vec<_> = s.marks.iter().filter(|m| matches!(m.mark, Mark::Color(_))).copied().collect();
        assert_eq!(colours, [span(2, 7, Mark::Color(0x33689c))]);
        assert!(s.has_mark(Mark::Color(0x33689c)) && !s.has_mark(Mark::Color(0xb23b33)));
        s.toggle_mark(Mark::Color(0));
        assert_eq!(s.marks, [span(2, 8, Mark::Bold)], "bold untouched, colours gone");
        // Without a selection nothing changes and no undo step is taken.
        let undo = s.undo.len();
        s.set_caret(3, false);
        assert_eq!(s.toggle_mark(Mark::Italic), Fx::default());
        assert!(!s.has_mark(Mark::Italic));
        assert_eq!(s.undo.len(), undo);
        s.undo();
        assert_eq!(s.marks.iter().filter(|m| matches!(m.mark, Mark::Color(_))).count(), 1, "undo takes the toggle back");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn undo_carries_anchors_made_after_the_snapshot() {
        // Replace a word, let the mind anchor a thread to words after it, then undo. Keeping
        // the anchor's offsets would point into the middle of a Cyrillic letter, and saving
        // the note slices the text there.
        let mut s = st("T\nпривет мир");
        let p = at(&s, "привет");
        s.select(p..p + "привет".len(), false);
        typ(&mut s, "hi");
        assert_eq!(s.text, "T\nhi мир");
        let m = at(&s, "мир");
        s.anchors.push(Anchor { id: "new".into(), range: m..m + "мир".len(), open: true });
        s.undo();
        assert_eq!(s.text, "T\nпривет мир");
        assert_eq!(&s.text[s.anchors[0].range.clone()], "мир");
        s.redo();
        assert_eq!(&s.text[s.anchors[0].range.clone()], "мир");
        check(&s);
        // Anchors the snapshot knew go back to where they were, keeping today's open state;
        // an anchor on words the undo removes collapses where they were.
        let mut s = st("T\nsome words");
        s.anchors.push(Anchor { id: "old".into(), range: 2..6, open: true });
        typ(&mut s, " more");
        let more = at(&s, "more");
        s.anchors.push(Anchor { id: "on-more".into(), range: more..more + 4, open: true });
        s.anchors[0].open = false;
        s.undo();
        assert_eq!(s.text, "T\nsome words");
        let old = s.anchors.iter().find(|a| a.id == "old").unwrap();
        assert_eq!((old.range.clone(), old.open), (2..6, false));
        let gone = s.anchors.iter().find(|a| a.id == "on-more").unwrap();
        assert_eq!(gone.range, s.text.len()..s.text.len());
        check(&s);
    }

    // ----- dictation, scripted typing and loading -----------------------------------

    #[std::prelude::v1::test]
    fn dictation_spaces_itself_from_the_words_around_it() {
        let dictate = |text: &str, caret_before: Option<&str>, words: &str| {
            let mut s = st(text);
            if let Some(c) = caret_before {
                let i = at(&s, c);
                s.set_caret(i, false);
            }
            s.insert_at_cursor(words);
            check(&s);
            s
        };
        assert_eq!(dictate("T\nHello", None, "world").text, "T\nHello world");
        assert_eq!(dictate("T\n", None, "Hi").text, "T\nHi");
        assert_eq!(dictate("T\nHello ", None, "there").text, "T\nHello there");
        assert_eq!(dictate("T\nHello", None, ", friend").text, "T\nHello, friend");
        assert_eq!(dictate("T\n(", None, "aside").text, "T\n(aside");
        // Before a word: a space after the words, and the caret stays right after them.
        let mut s = dictate("T\nHello world", Some("world"), "big");
        assert_eq!(s.text, "T\nHello big world");
        assert_eq!(s.selected, at(&s, " world")..at(&s, " world"));
        s.insert_at_cursor("bad");
        assert_eq!(s.text, "T\nHello big bad world");
        // Replacing a selection.
        let mut s = st("T\nsay bad things");
        let b = at(&s, "bad");
        s.select(b..b + 3, false);
        s.insert_at_cursor("good");
        assert_eq!(s.text, "T\nsay good things");
    }

    #[std::prelude::v1::test]
    fn scripted_typing_and_phrase_selection() {
        let mut s = st("T\n");
        s.append_paragraph("first");
        s.append_paragraph("second");
        assert_eq!(s.text, "T\nfirst\nsecond");
        let fx = s.select_phrase("second").unwrap();
        assert!(fx.moved && fx.settled);
        assert_eq!(s.selected_text(), "second");
        assert!(s.select_phrase("").is_none(), "an empty phrase selects nothing");
        assert!(s.select_phrase("absent").is_none());
        assert!(s.select_all().settled);
        assert_eq!(s.selected, 0..s.text.len());
    }

    #[std::prelude::v1::test]
    fn cut_and_paste() {
        let mut s = st("T\none two");
        assert!(s.cut().is_none(), "nothing selected");
        s.select(2..6, false);
        let (cut, _) = s.cut().unwrap();
        assert_eq!((cut.as_str(), s.text.as_str()), ("one ", "T\ntwo"));
        // Line breaks from Windows, old Mac apps and Unicode separators become lines.
        s.set_caret(2, false);
        s.paste("a\r\nb\rc\u{2028}d\u{2029}e");
        assert_eq!(s.text, "T\na\nb\nc\nd\netwo");
        // Pasting lines into a list continues it; pasting nothing removes the selection.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Bullet, "item")]);
        s.paste("x\ny");
        assert_eq!(s.kinds, [K::Title, K::Bullet, K::Bullet]);
        s.select(2..4, false);
        s.paste("");
        assert_eq!(s.text, "T\nemx\ny");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn loading_repairs_stale_offsets_and_kinds() {
        let mut s = State::new();
        s.load(
            vec![(K::H1, "Héllo".into()), (K::Title, "body\nmore".into())],
            vec![
                MarkSpan { start: 2, end: 3, mark: Mark::Bold },
                MarkSpan { start: 0, end: 999, mark: Mark::Italic },
            ],
            vec![
                Anchor { id: "a".into(), range: 2..4, open: true },
                Anchor { id: "b".into(), range: 50..60, open: true },
            ],
        );
        assert_eq!(s.text, "Héllo\nbody more");
        assert_eq!(s.kinds, [K::Title, K::Paragraph]);
        assert_eq!(s.marks, [MarkSpan { start: 1, end: 3, mark: Mark::Bold }], "inside é widens to it; past the end is dropped");
        assert_eq!((s.anchors[0].range.clone(), s.anchors[1].range.clone()), (1..4, 16..16));
        check(&s);
        s.load(vec![], vec![], vec![]);
        assert_eq!((s.text.as_str(), s.kinds.clone()), ("\n", vec![K::Title, K::Paragraph]));
        // Typing in a note just opened is undoable even right after typing in the last one.
        let mut s = st("T\n");
        typ(&mut s, "a");
        s.load(vec![(K::Title, "N".into()), (K::Paragraph, String::new())], vec![], vec![]);
        typ(&mut s, "x");
        s.undo();
        assert_eq!(s.text, "N\n");
    }

    // ----- movement and the mouse -------------------------------------------------

    #[std::prelude::v1::test]
    fn movement_before_the_first_layout() {
        // T0 \n1 a2 b3 👍4 c8 d9 \n10 x11
        let mut s = st("T\nab👍cd\nx");
        // Up and Down move by lines and keep the column in graphemes.
        s.set_caret(8, false);
        s.move_vertical(None, true, false);
        assert_eq!(s.selected, 12..12, "the end of the shorter line");
        s.move_vertical(None, true, false);
        assert_eq!(s.selected, 12..12, "Down on the last line stays at the end");
        s.set_caret(11, false);
        s.move_vertical(None, false, false);
        assert_eq!(s.selected, 2..2);
        s.move_vertical(None, false, false);
        assert_eq!(s.selected, 0..0);
        s.move_vertical(None, false, false);
        assert_eq!(s.selected, 0..0);
        // Shift extends, and the selection flips when the caret crosses its other end.
        s.set_caret(8, false);
        s.move_horizontal(false, true);
        assert_eq!((s.selected.clone(), s.cursor()), (4..8, 4));
        s.move_horizontal(true, true);
        s.move_horizontal(true, true);
        assert_eq!((s.selected.clone(), s.cursor()), (8..9, 9));
        // Left and Right collapse a selection to its edges.
        s.select(2..8, false);
        s.move_horizontal(false, false);
        assert_eq!(s.selected, 2..2);
        s.select(2..8, false);
        s.move_horizontal(true, false);
        assert_eq!(s.selected, 8..8);
        // Words, document edges and line edges.
        s.move_word(true, false);
        assert_eq!(s.selected, 10..10);
        s.move_doc_edge(true, true);
        assert_eq!(s.selected, 10..12);
        s.move_doc_edge(false, false);
        assert_eq!(s.selected, 0..0);
        s.set_caret(3, false);
        s.move_row_edge(None, true, false);
        assert_eq!(s.selected, 10..10);
        s.move_row_edge(None, false, true);
        assert_eq!((s.selected.clone(), s.cursor()), (2..10, 2));
        check(&s);
    }

    #[std::prelude::v1::test]
    fn mouse_clicks_drags_and_shift_clicks() {
        // alpha 2..7, beta 8..12, gamma 13..18, next 19..23, line 24..28
        let mut s = st("T\nalpha beta gamma\nnext line");
        let (beta, gamma) = (at(&s, "beta"), at(&s, "gamma"));
        // Double-click selects the word, also from the right half of its last letter.
        s.mouse_down((beta + 4, false), 2, false);
        assert_eq!(s.selected, beta..beta + 4);
        assert!(s.mouse_up(false).settled);
        // A word drag grows by whole words either way and keeps the first word.
        s.mouse_down((beta + 1, false), 2, false);
        s.mouse_drag((gamma + 2, false));
        assert_eq!(s.selected, beta..gamma + 5);
        s.mouse_drag((3, false));
        assert_eq!((s.selected.clone(), s.reversed), (2..beta + 4, true));
        s.mouse_up(false);
        // Triple-click selects the paragraph; a drag extends by paragraphs.
        s.mouse_down((beta, false), 3, false);
        assert_eq!(s.selected, 2..18);
        s.mouse_drag((at(&s, "line"), false));
        assert_eq!(s.selected, 2..s.text.len());
        s.mouse_up(false);
        // A plain drag selects characters.
        s.mouse_down((beta, false), 1, false);
        s.mouse_drag((beta + 2, false));
        assert_eq!(s.selected, beta..beta + 2);
        s.mouse_up(false);
        // Shift-click before a selection keeps its far end; after it, its start; inside it,
        // it moves the caret end.
        s.select(beta..beta + 4, false);
        s.mouse_down((2, false), 1, true);
        assert_eq!((s.selected.clone(), s.cursor()), (2..beta + 4, 2));
        s.mouse_up(false);
        s.select(beta..beta + 4, false);
        s.mouse_down((gamma + 5, false), 1, true);
        assert_eq!((s.selected.clone(), s.cursor()), (beta..gamma + 5, gamma + 5));
        s.mouse_up(false);
        s.mouse_down((beta + 2, false), 1, true);
        assert_eq!(s.selected, beta..beta + 2);
        s.mouse_up(false);
        // A click inside open anchored words opens the thread; not on their edge, not while
        // threads are hidden, not for a resolved thread.
        s.anchors.push(Anchor { id: "t".into(), range: beta..beta + 4, open: true });
        s.mouse_down((beta + 2, false), 1, false);
        assert_eq!(s.mouse_up(false).anchor.as_deref(), Some("t"));
        s.mouse_down((beta, false), 1, false);
        assert_eq!(s.mouse_up(false).anchor, None);
        s.mouse_down((beta + 2, false), 1, false);
        assert_eq!(s.mouse_up(true).anchor, None);
        s.anchors[0].open = false;
        s.mouse_down((beta + 2, false), 1, false);
        assert_eq!(s.mouse_up(false).anchor, None);
        assert_eq!(s.mouse_up(false), Fx::default(), "a release without a press does nothing");
        // A hit from an old layout past the end lands at the end.
        s.mouse_down((10_000, false), 1, false);
        assert_eq!(s.selected, s.text.len()..s.text.len());
        check(&s);
    }

    // ----- layout -------------------------------------------------------------------

    #[std::prelude::v1::test]
    fn layout_rows_points_and_hits() {
        let ts = text_system();
        // 60 px holds ten 6 px characters: "hello world foo" wraps after "hello ".
        let s = st("T\nhello world foo");
        let l = lay_out(&s, &ts, 60.);
        let body = &l.lines[1];
        assert_eq!((body.start, body.row_starts.clone()), (2, vec![0, 6]));
        let (top, lh) = (body.top, body.line_height);
        assert_eq!((top, lh), (px(34.), px(16.)));
        // A caret on the wrap starts the next row; with upstream affinity it ends the row above.
        assert_eq!(l.point_of(8), point(px(0.), top + lh));
        assert_eq!(l.caret_point(8, true), point(px(36.), top));
        assert_eq!(l.point_of(7), point(px(30.), top));
        // A range over the wrap gives one rectangle per row.
        let r = l.rects(5..11);
        assert_eq!(r.len(), 2);
        assert_eq!((r[0].origin.x, r[0].size.width, r[1].origin.x, r[1].size.width), (px(18.), px(18.), px(0.), px(18.)));
        // Clicks past the end of a space-wrapped row land before its space; past the last
        // row, at the end.
        assert_eq!(l.hit(point(px(200.), top + px(2.))), (7, false));
        assert_eq!(l.hit(point(px(200.), top + lh + px(2.))), (s.text.len(), false));
        assert_eq!(l.hit(point(px(13.), top + px(2.))), (4, false));
        // Above the note, in the gap under the title, and left of the text.
        assert_eq!(l.hit(point(px(10.), px(-50.))), (0, false));
        assert_eq!(l.hit(point(px(10.), top - px(2.))).0, 1, "the gap under the title belongs to it");
        assert_eq!(l.hit(point(px(-5.), top + px(2.))), (2, false));
        assert_eq!((l.line_of(7), l.line_of(1)), (1, 0));
    }

    #[std::prelude::v1::test]
    fn clicks_never_land_inside_a_cluster() {
        // The no-op text system gives a combining accent and each part of a ZWJ emoji a glyph
        // of its own, as some fonts do.
        let ts = text_system();
        let s = st("T\nxe\u{301}y👨‍👩‍👧z");
        let l = lay_out(&s, &ts, 600.);
        let y = l.lines[1].top + px(2.);
        for x in 0..120 {
            let (o, _) = l.hit(point(px(x as f32), y));
            assert_eq!(o, snap_grapheme(&s.text, o), "x {x} lands inside a cluster at {o}");
        }
    }

    #[std::prelude::v1::test]
    fn up_and_down_keep_the_x_across_rows_lines_and_indents() {
        let ts = text_system();
        // Two rows of one paragraph: "hello " and "world foo".
        let mut s = st("T\nhello world foo");
        let l = lay_out(&s, &ts, 60.);
        s.set_caret(5, false);
        s.move_vertical(Some(&l), true, false);
        assert_eq!(s.selected, 11..11, "the same x on the next row");
        s.move_vertical(Some(&l), true, false);
        assert_eq!(s.selected, 17..17, "Down on the last row goes to the end");
        s.move_vertical(Some(&l), false, false);
        assert_eq!(s.selected, 5..5, "the goal x survives the trip to the end");
        s.move_vertical(Some(&l), false, false);
        assert_eq!(s.selected, 1..1, "into the title");
        s.move_vertical(Some(&l), false, false);
        assert_eq!(s.selected, 0..0, "Up on the first row goes to the start");
        // Other moves reset the goal: from the end of the second row (x 54), Up aims for 54.
        s.set_caret(5, false);
        s.move_vertical(Some(&l), true, false);
        s.move_row_edge(Some(&l), true, false);
        assert_eq!(s.selected, 17..17);
        s.move_vertical(Some(&l), false, false);
        assert_eq!(s.selected, 7..7, "the end of \"hello \", not the old column");
        // Indents: the caret keeps its x on screen, not its offset in the line.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Paragraph, "abcdefghij"), (K::Bullet, "abcdefghij")]);
        let l = lay_out(&s, &ts, 600.);
        s.set_caret(2 + 5, false);
        s.move_vertical(Some(&l), true, false);
        assert_eq!(s.selected.start - l.lines[2].start, 1, "x 28 in the bullet (22 + 6), not its sixth letter");
        // Heights and font sizes: a heading's 9 px glyphs, and back.
        let mut s = st_kinds(&[(K::Title, "T"), (K::H1, "abcdefghij"), (K::Paragraph, "abcdefghij")]);
        let l = lay_out(&s, &ts, 600.);
        let (head, para) = (l.lines[1].start, l.lines[2].start);
        s.set_caret(para + 4, false);
        s.move_vertical(Some(&l), false, false);
        assert_eq!(s.selected.start - head, 3, "x 27 is the heading boundary closest to x 24");
        s.move_vertical(Some(&l), true, false);
        assert_eq!(s.selected.start, para + 4);
        // Without shift a selection collapses from its edge in the direction of travel.
        s.select(para + 1..para + 3, false);
        s.move_vertical(Some(&l), false, false);
        assert_eq!(s.selected.start - head, 1, "Up from the selection's start");
        // Goal x targets in a line of wide and multibyte characters land on grapheme starts.
        let mut s = st_kinds(&[(K::Title, "T"), (K::Paragraph, "abcdefghijklmnop"), (K::Paragraph, "你👍🏽e\u{301}👨‍👩‍👧x")]);
        let l = lay_out(&s, &ts, 600.);
        for i in 0..=16 {
            s.set_caret(2 + i, false);
            s.move_vertical(Some(&l), true, false);
            assert_eq!(s.selected.start, snap_grapheme(&s.text, s.selected.start), "from column {i}");
        }
        check(&s);
    }

    #[std::prelude::v1::test]
    fn line_edges_on_wrapped_rows() {
        let ts = text_system();
        // A row wrapped after a space ends before the space.
        let mut s = st("T\nhello world foo");
        let l = lay_out(&s, &ts, 60.);
        s.set_caret(3, false);
        s.move_row_edge(Some(&l), true, false);
        assert_eq!((s.selected.clone(), s.upstream), (7..7, false));
        s.move_row_edge(Some(&l), false, false);
        assert_eq!(s.selected, 2..2);
        // A row broken inside CJK text ends after its last character, drawn at the end of that
        // row (upstream affinity); End again stays, Home goes to the start of the same row.
        let cjk = "T\n你好世界你好世界你好世界";
        let mut s = st(cjk);
        let l = lay_out(&s, &ts, 60.);
        assert_eq!(l.lines[1].row_starts, vec![0, 30]);
        s.set_caret(5, false);
        s.move_row_edge(Some(&l), true, false);
        assert_eq!((s.selected.clone(), s.upstream), (32..32, true));
        assert_eq!(l.caret_point(32, true), point(px(60.), l.lines[1].top));
        s.move_row_edge(Some(&l), true, false);
        assert_eq!(s.selected, 32..32);
        s.move_row_edge(Some(&l), false, false);
        assert_eq!(s.selected, 2..2);
        // Typing there goes after the row's last character.
        s.move_row_edge(Some(&l), true, false);
        typ(&mut s, "!");
        assert_eq!(&s.text[29..33], "好!");
        // A click past the end of that row gives the same caret.
        let l = lay_out(&st(cjk), &ts, 60.);
        assert_eq!(l.hit(point(px(300.), l.lines[1].top + px(2.))), (32, true));
        // Down from that caret leaves from its own row.
        let mut s = st(&format!("{cjk}\nab"));
        let l = lay_out(&s, &ts, 60.);
        s.set_caret(32, true);
        s.move_vertical(Some(&l), true, false);
        assert_eq!(s.selected, 38..38, "x 60 on the second row is that row's end");
        // Cmd-Backspace there deletes that row's text.
        let mut s = st(cjk);
        let l = lay_out(&s, &ts, 60.);
        s.set_caret(32, true);
        s.delete_line_left(Some(&l));
        assert_eq!(s.text, "T\n世界");
        check(&s);
    }

    #[std::prelude::v1::test]
    fn an_out_of_date_layout_cannot_misplace_the_caret() {
        // The layout is rebuilt when the next frame is drawn, so keys that arrive in the same
        // frame as an edit see the old one: its offsets can be past the end or inside characters.
        let ts = text_system();
        let mut s = st("T\nhello world foo\nnaïve 你好 text");
        let l = lay_out(&s, &ts, 60.);
        s.select(2..s.text.len(), false);
        typ(&mut s, "é");
        assert_eq!(s.text, "T\né");
        for down in [true, false] {
            for extend in [true, false] {
                s.set_caret(3, false);
                check(&s);
                s.move_vertical(Some(&l), down, extend);
                check(&s);
                s.move_row_edge(Some(&l), down, extend);
                check(&s);
            }
        }
        s.set_caret(s.text.len(), false);
        s.delete_line_left(Some(&l));
        check(&s);
        for x in [0., 15., 50., 400.] {
            for y in [0., 40., 60., 90., 400.] {
                let hit = l.hit(point(px(x), px(y)));
                for (clicks, shift) in [(1, false), (2, false), (3, true), (1, true)] {
                    s.mouse_down(hit, clicks, shift);
                    check(&s);
                    s.mouse_drag(l.hit(point(px(y), px(x))));
                    check(&s);
                    s.mouse_up(false);
                }
            }
        }
    }

    #[std::prelude::v1::test]
    fn rects_for_carets_selections_and_empty_lines() {
        let ts = text_system();
        // Lines: "T" 0..1, "ab" 2..4, "" 5..5, "cd" 6..8.
        let l = lay_out(&st("T\nab\n\ncd"), &ts, 600.);
        let caret = l.rects(3..3);
        assert_eq!((caret.len(), caret[0].origin.x, caret[0].size.width), (1, px(6.), px(0.)));
        // Across an empty line, a zero-width piece there (painted at least 4 px wide).
        let across = l.rects(3..7);
        assert_eq!(across.len(), 3);
        assert_eq!((across[1].size.width, across[1].origin.y), (px(0.), l.lines[2].top));
        // Just the break after a line with text draws nothing on that line.
        assert!(l.rects(4..5).iter().all(|r| r.origin.y != l.lines[1].top));
        assert!(l.rects(100..200).is_empty());
        // An empty note.
        let empty = lay_out(&State::new(), &ts, 600.);
        assert_eq!(empty.lines.len(), 2);
        assert_eq!(empty.point_of(0), point(px(0.), px(0.)));
        assert_eq!(empty.point_of(1).y, empty.lines[1].top);
        assert_eq!(empty.hit(point(px(300.), px(500.))), (1, false));
        assert_eq!(empty.rects(0..1).len(), 2);
    }

    #[std::prelude::v1::test]
    fn placeholder_rules() {
        assert_eq!(placeholder(0, 1, K::Title), Some("Untitled"));
        assert_eq!(placeholder(1, 1, K::Paragraph), Some("Just start writing."));
        assert_eq!(placeholder(1, 2, K::Paragraph), None, "an empty paragraph that is not the last");
        assert_eq!(placeholder(2, 2, K::Bullet), None, "an empty list item");
        assert_eq!(placeholder(2, 2, K::H1), None, "an empty heading");
        assert_eq!(placeholder(3, 3, K::Paragraph), Some("Just start writing."), "a trailing empty paragraph");
    }

    // ----- everything at once ---------------------------------------------------------

    /// xorshift64*, so the random edits below are the same on every run.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n.max(1) as u64) as usize
        }

        fn coin(&mut self) -> bool {
            self.below(2) == 0
        }
    }

    #[std::prelude::v1::test]
    fn random_editing_keeps_every_invariant() {
        let ts = text_system();
        let pieces = [
            "a", "é", "e\u{301}", "你", "👍🏽", "👨‍👩‍👧", "🇮🇳", " ", "word", "\n", "# ", "- ", "1. ", "> ", "x y\nz", "", "\r\n",
        ];
        let kinds = [K::Paragraph, K::H1, K::H2, K::H3, K::Bullet, K::Numbered, K::Quote, K::Title];
        let marks = [Mark::Bold, Mark::Italic, Mark::Code, Mark::Color(0xb23b33), Mark::Color(0)];
        for seed in 1..=8u64 {
            let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut s = st("Title\nfirst line of the note");
            let mut layout: Option<EditorLayout> = None;
            let mut ids = 0;
            for step in 0..2500 {
                // Layouts come and go out of date, as frames do.
                if r.below(8) == 0 {
                    layout = Some(lay_out(&s, &ts, [60., 130., 600.][r.below(3)]));
                }
                let l = layout.as_ref();
                let len = s.text.len();
                let len16 = s.text.encode_utf16().count();
                let piece = pieces[r.below(pieces.len())];
                match r.below(32) {
                    0..=5 => drop(s.ime_replace(None, piece)),
                    6 => drop(s.newline()),
                    7 => drop(s.backspace()),
                    8 => drop(s.delete_forward()),
                    9 => drop(s.delete_word(r.coin())),
                    10 => drop(s.delete_line_left(l)),
                    11 => drop(s.move_horizontal(r.coin(), r.coin())),
                    12 => drop(s.move_word(r.coin(), r.coin())),
                    13 => drop(s.move_vertical(l, r.coin(), r.coin())),
                    14 => drop(s.move_row_edge(l, r.coin(), r.coin())),
                    15 => drop(s.move_doc_edge(r.coin(), r.coin())),
                    16 => {
                        let (a, b) = (r.below(len + 3), r.below(len + 3));
                        s.select(a..b, r.coin());
                    }
                    17 => {
                        if let Some(l) = l {
                            let p = |r: &mut Rng| point(px(r.below(700) as f32 - 30.), px(r.below(500) as f32 - 30.));
                            let down = l.hit(p(&mut r));
                            s.mouse_down(down, 1 + r.below(3), r.coin());
                            check(&s);
                            s.mouse_drag(l.hit(p(&mut r)));
                            check(&s);
                            s.mouse_up(r.below(4) == 0);
                        }
                    }
                    18 => drop(s.paste(["one\r\ntwo", "x", "\n\n", "line\u{2028}sep", ""][r.below(5)])),
                    19 => drop(s.undo()),
                    20 => drop(s.redo()),
                    21 => drop(s.toggle_mark(marks[r.below(marks.len())])),
                    22 => {
                        // A composition, then a commit, an abandon, or nothing (still composing).
                        s.ime_replace_and_mark(None, "n", Some(&(1..1)));
                        s.ime_replace_and_mark(None, "ni", None);
                        match r.below(3) {
                            0 => drop(s.ime_replace(None, "你")),
                            1 => drop(s.ime_unmark()),
                            _ => {}
                        }
                    }
                    23 => drop(s.set_selection_kind(kinds[r.below(kinds.len())])),
                    24 => {
                        let line = r.below(s.kinds.len() + 1);
                        s.set_line_kind(line, kinds[r.below(kinds.len())]);
                    }
                    25 => drop(if r.coin() { s.indent() } else { s.outdent() }),
                    26 => drop(s.insert_at_cursor(["dictated", "", ", then"][r.below(3)])),
                    27 => drop(s.append_paragraph("appended")),
                    28 => {
                        // The mind anchors a thread to some words (NoteEditor::add_anchor snaps them).
                        let range = snap_range(&s.text, r.below(len + 1)..r.below(len + 1));
                        ids += 1;
                        s.anchors.push(Anchor { id: ids.to_string(), range, open: r.coin() });
                    }
                    29 => {
                        let (a, b) = (r.below(len16 + 3), r.below(len16 + 3));
                        s.ime_set_selection(&(a..b));
                    }
                    30 => {
                        let (a, b) = (r.below(len16 + 3), r.below(len16 + 3));
                        s.ime_replace(Some(&(a.min(b)..a.max(b))), piece);
                    }
                    _ => drop(s.select_all()),
                }
                check(&s);
                // A layout of the current text answers every caret position and every click.
                if step % 50 == 0 {
                    let l = lay_out(&s, &ts, 90.);
                    for o in (0..=s.text.len()).filter(|o| s.text.is_char_boundary(*o)) {
                        let _ = (l.point_of(o), l.caret_point(o, true), l.rects(o..s.text.len()), l.line_of(o));
                        let (row, _) = row_bounds(&s.text, Some(&l), o, r.coin());
                        assert!(row.start <= row.end && s.text.is_char_boundary(row.end), "row {row:?}");
                        let (to, _, _) = vertical_target(&s.text, Some(&l), o, r.coin(), r.coin(), None);
                        assert!(to <= s.text.len());
                    }
                    for _ in 0..30 {
                        let (o, _) = l.hit(point(px(r.below(200) as f32 - 20.), px(r.below(800) as f32 - 20.)));
                        assert_eq!(o, snap_grapheme(&s.text, o), "a click inside a cluster");
                    }
                }
                if s.text.len() > 500 {
                    s.select_all();
                    s.backspace();
                }
            }
        }
    }
}

//! ⌘K: find a note by title or text, start a new one ("Folder / Name" files it),
//! or delete one with ⌘⌫.

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;
use gpui_kit::base::FocusTrapElement;
use gpui_kit::prelude::*;

use crate::store::{Folder, Note, Store};
use crate::theme::{SANS, Theme, ts};
use crate::ui;

#[derive(Clone, Debug)]
pub enum PaletteEvent {
    Open(String),
    Create { title: String, folder: Option<String> },
    Delete(String),
    Dismiss,
}

impl EventEmitter<PaletteEvent> for Palette {}

pub struct Palette {
    input: Entity<InputState>,
    selected: usize,
    current: Option<String>,
    /// The list's scroll position: the highlighted row stays in view as the arrows move it.
    scroll: ScrollHandle,
    /// Tab stays in the palette while it is open (the arrows move through its rows).
    trap: FocusHandle,
    _subs: Vec<Subscription>,
}

struct Row {
    kind: RowKind,
    title: String,
    folder: Option<String>,
    snippet: Option<String>,
}

enum RowKind {
    Note(String),
    Create,
}

impl Palette {
    pub fn new(current: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find or start a note"));
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, _, cx| match ev {
            InputEvent::PressEnter { .. } => this.choose(cx),
            InputEvent::Change => this.select(0, cx),
            _ => {}
        });
        input.update(cx, |i, cx| i.focus(window, cx));
        Self { input, selected: 0, current, scroll: ScrollHandle::new(), trap: cx.focus_handle(), _subs: vec![sub] }
    }

    /// Give the search box the keyboard (again).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |i, cx| i.focus(window, cx));
    }

    /// What is typed, for Loose ends (⇧⌘K) to carry on with.
    pub fn query(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    /// Start with these words typed (what Loose ends had, when ⌘K opens in its place).
    pub fn set_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |i, cx| i.set_value(query.to_string(), window, cx));
        self.select(0, cx);
    }

    fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected = ix;
        self.scroll.scroll_to_item(ix);
        cx.notify();
    }

    fn rows(&self, cx: &App) -> Vec<Row> {
        let store = cx.global::<Store>();
        search(&store.notes, &store.folders, &self.input.read(cx).value())
    }

    fn choose(&mut self, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(row) = rows.get(clamp_selection(self.selected, rows.len())) else { return };
        match &row.kind {
            RowKind::Note(id) => cx.emit(PaletteEvent::Open(id.clone())),
            RowKind::Create => {
                let (folder, title) = parse_new_note(&row.title);
                cx.emit(PaletteEvent::Create { title, folder });
            }
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        match ev.keystroke.key.as_str() {
            "down" => {
                self.select(clamp_selection(self.selected + 1, rows.len()), cx);
                cx.stop_propagation();
            }
            "up" => {
                self.select(clamp_selection(self.selected, rows.len()).saturating_sub(1), cx);
                cx.stop_propagation();
            }
            "escape" => {
                cx.stop_propagation();
                cx.emit(PaletteEvent::Dismiss);
            }
            _ => {}
        }
    }

    /// ⌘⌫: delete the highlighted note. The window catches the keystroke before the
    /// search box's own ⌘⌫ binding can swallow it. Returns whether a note was there.
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) -> bool {
        let rows = self.rows(cx);
        match rows.get(clamp_selection(self.selected, rows.len())) {
            Some(Row { kind: RowKind::Note(id), .. }) => {
                cx.emit(PaletteEvent::Delete(id.clone()));
                true
            }
            _ => false,
        }
    }
}

/// The row actually highlighted: the list can shrink under the selection (a
/// deleted note, a longer query), and every key uses the row the person sees.
fn clamp_selection(selected: usize, rows: usize) -> usize {
    selected.min(rows.saturating_sub(1))
}

/// Notes matching `query`, by title or by text, then a row to start a note with it.
/// Title matches come first; within each group the store's order (newest first).
fn search(notes: &[Note], folders: &[Folder], query: &str) -> Vec<Row> {
    let q = query.trim();
    let ql = q.to_lowercase();
    let mut hits: Vec<(u8, &Note, Option<String>)> = notes
        .iter()
        .filter_map(|n| {
            if ql.is_empty() || n.title.to_lowercase().contains(&ql) {
                return Some((0, n, None));
            }
            let body = n.plain_text();
            let at = find_ignoring_case(&body, &ql)?;
            Some((1, n, Some(snippet_at(&body, at))))
        })
        .collect();
    hits.sort_by_key(|(rank, _, _)| *rank);
    let folder_name = |id: &Option<String>| {
        id.as_ref().and_then(|id| folders.iter().find(|f| &f.id == id).map(|f| f.name.clone()))
    };
    let mut rows: Vec<Row> = hits
        .into_iter()
        .take(30)
        .map(|(_, n, snippet)| Row {
            kind: RowKind::Note(n.id.clone()),
            title: if n.title.trim().is_empty() { "Untitled".into() } else { n.title.clone() },
            folder: folder_name(&n.folder),
            snippet,
        })
        .collect();
    if !q.is_empty() {
        rows.push(Row { kind: RowKind::Create, title: q.to_string(), folder: None, snippet: None });
    }
    rows
}

/// Byte offset in `text` where `query_lower` (already lowercase) first appears,
/// ignoring case. Lowercasing can change a character's length (İ is two bytes, its
/// lowercase three; the Kelvin sign three bytes, `k` one), so an offset found in
/// `text.to_lowercase()` is mapped back to the character it came from instead of
/// being used on `text` directly. Per-character lowercase has the same length as the
/// whole-string one: its only context rule, final sigma, swaps two-byte letters.
fn find_ignoring_case(text: &str, query_lower: &str) -> Option<usize> {
    let at = text.to_lowercase().find(query_lower)?;
    let mut lower_len = 0;
    for (i, c) in text.char_indices() {
        lower_len += c.to_lowercase().map(char::len_utf8).sum::<usize>();
        if lower_len > at {
            return Some(i);
        }
    }
    Some(text.len())
}

/// One line around byte offset `at` of `text`: from 30 characters before it, 80
/// characters in all, whitespace runs folded, with … when it starts mid-text.
fn snippet_at(text: &str, at: usize) -> String {
    let start = text[..at].char_indices().rev().nth(29).map_or(0, |(i, _)| i);
    let excerpt: String = text[start..].chars().take(80).collect();
    let folded = excerpt.split_whitespace().collect::<Vec<_>>().join(" ");
    if start > 0 { format!("…{folded}") } else { folded }
}

/// "Folder / Name" files the new note in Folder (made if needed). The first slash
/// splits, so "a / b / c" is "b / c" in "a"; with nothing before the slash there is
/// no folder and the words stay the title as typed ("/etc/hosts").
fn parse_new_note(input: &str) -> (Option<String>, String) {
    let input = input.trim();
    match input.split_once('/') {
        Some((folder, title)) if !folder.trim().is_empty() => (Some(folder.trim().to_string()), title.trim().to_string()),
        _ => (None, input.to_string()),
    }
}

impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let rows = self.rows(cx);
        let selected = clamp_selection(self.selected, rows.len());
        let current = self.current.clone();
        let query_empty = self.input.read(cx).value().trim().is_empty();
        // the footer names what Return and ⌘⌫ do to the row that is chosen
        let on_note = rows.get(selected).is_some_and(|r| matches!(r.kind, RowKind::Note(_)));
        let mut list = vec![];
        for (i, row) in rows.iter().enumerate() {
            let on = i == selected;
            let create = matches!(row.kind, RowKind::Create);
            let label = match &row.kind {
                RowKind::Note(_) => row.title.clone(),
                RowKind::Create => format!("Start \u{201c}{}\u{201d}", row.title),
            };
            let is_current = matches!(&row.kind, RowKind::Note(id) if Some(id) == current.as_ref());
            list.push(
                div()
                    .id(("palette-row", i))
                    .mx(px(6.))
                    // the row's icon under the search's, 14 px in as the footer's ↩ is,
                    // and its title under the search box's words
                    .pl(px(8.))
                    .pr(px(10.))
                    .py(px(7.))
                    // a row's corner, as the sidebar's and the menus' rows have it
                    .rounded(px(6.))
                    .when(on, |d| d.bg(p.hover))
                    .when(!on, |d| d.hover(move |s| s.bg(p.hover.opacity(0.5))))
                    .cursor_pointer()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(10.))
                    .child(ui::icon(if create { "plus" } else { "file-text" }, 13., p.faint))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(ts::UI))
                                    .text_color(p.ink)
                                    // a long folder name gives way before its slash, which stays
                                    .when_some(row.folder.clone(), |d, f| {
                                        d.child(
                                            div()
                                                .flex_none()
                                                .max_w(px(150.))
                                                .flex()
                                                .gap(px(4.))
                                                .text_color(p.faint)
                                                .child(div().min_w(px(0.)).truncate().child(f))
                                                .child(div().flex_none().child("/")),
                                        )
                                    })
                                    .child(div().min_w(px(0.)).truncate().child(label)),
                            )
                            .when_some(row.snippet.clone(), |d, s| {
                                d.child(div().mt(px(1.)).text_size(px(ts::META)).text_color(p.graphite).truncate().child(s))
                            }),
                    )
                    .when(is_current, |d| d.child(ui::key_hint("Open now", &t)))
                    .when(on && !is_current, |d| d.child(ui::key_hint("\u{21a9}", &t)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = i;
                        this.choose(cx);
                    })),
            );
        }
        let empty = rows.is_empty();
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_center()
            // The card is as tall as its rows, not stretched to the window.
            .items_start()
            .pt(px(120.))
            .bg(p.dim.opacity(0.25))
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.emit(PaletteEvent::Dismiss)))
            .child(
                div()
                    .id("palette")
                    .focus_trap("palette-trap", &self.trap)
                    .w(px(460.))
                    .flex()
                    .flex_col()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.rule)
                    .shadow(ui::modal_shadow(&p))
                    .font_family(SANS)
                    .capture_key_down(cx.listener(Self::on_key))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .border_b_1()
                            .border_color(p.rule)
                            .child(ui::icon("search", 14., p.faint))
                            .child(div().flex_1().child(Input::new(&self.input).appearance(false).px_0().text_size(px(ts::BODY)))),
                    )
                    .when(empty, |d| {
                        d.child(div().px(px(16.)).py(px(14.)).text_size(px(ts::UI)).text_color(p.graphite).child("No notes yet. Type a title to start one."))
                    })
                    .when(!empty, |d| {
                        d.child(div().id("palette-list").max_h(px(336.)).py(px(4.)).overflow_y_scroll().track_scroll(&self.scroll).children(list))
                    })
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(7.))
                            .flex()
                            .gap(px(14.))
                            .border_t_1()
                            .border_color(p.rule)
                            .text_size(px(ts::META))
                            .text_color(p.faint)
                            .child(if on_note { "\u{21a9} open" } else { "\u{21a9} start" })
                            .child(if query_empty { "Type to find or start a note" } else { "\u{201c}Folder / Name\u{201d} files a new note" })
                            .child(div().flex_1())
                            .when(on_note, |d| d.child("\u{2318}\u{232b} delete"))
                            // the second list in this card: every open thread
                            .children(crate::shortcuts::keys_for("app::LooseEnds", cx).map(|k| format!("{k} loose ends"))),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Block, BlockKind, Mode};

    fn note(title: &str, body: &str) -> Note {
        let mut n = Note::new(title, Mode::ThinksWithYou, None);
        n.blocks = vec![Block { kind: BlockKind::Title, text: title.into() }, Block { kind: BlockKind::Paragraph, text: body.into() }];
        n
    }

    fn titles(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r.kind {
                RowKind::Note(_) => r.title.clone(),
                RowKind::Create => format!("Start {}", r.title),
            })
            .collect()
    }

    #[std::prelude::v1::test]
    fn finds_text_ignoring_case_at_the_right_offset() {
        assert_eq!(find_ignoring_case("Hello World", "world"), Some(6));
        assert_eq!(find_ignoring_case("abc", "a"), Some(0));
        assert_eq!(find_ignoring_case("abc", "c"), Some(2));
        assert_eq!(find_ignoring_case("abc", "x"), None);
        // Characters whose lowercase is longer or shorter in bytes: the offset is in
        // the original text and on a character boundary (this used to panic).
        let turkish = "İİİİ日本";
        assert_eq!(find_ignoring_case(turkish, "本"), Some(turkish.find('本').unwrap()));
        let kelvin = "\u{212A}\u{212A}日本 ẞtraße";
        assert_eq!(find_ignoring_case(kelvin, "本"), Some(kelvin.find('本').unwrap()));
        assert_eq!(find_ignoring_case(kelvin, "kk"), Some(0));
        assert_eq!(find_ignoring_case(kelvin, "ßtr"), Some(kelvin.find('ẞ').unwrap()));
        let emoji = "👩‍💻 写代码 Rust 🦀";
        assert_eq!(find_ignoring_case(emoji, "rust"), Some(emoji.find("Rust").unwrap()));
        assert_eq!(find_ignoring_case(emoji, "🦀"), Some(emoji.find('🦀').unwrap()));
    }

    #[std::prelude::v1::test]
    fn snippets_stay_on_character_boundaries() {
        let long = format!("{}needle{}", "写".repeat(50), "🦀".repeat(100));
        let at = long.find("needle").unwrap();
        let s = snippet_at(&long, at);
        assert!(s.starts_with('…') && s.contains("needle"), "{s}");
        assert_eq!(s.chars().count(), 81);
        assert_eq!(snippet_at("needle at the start", 0), "needle at the start");
        let end = "ends with the needle";
        assert_eq!(snippet_at(end, end.len() - "needle".len()), end);
        assert_eq!(snippet_at("one\n\ntwo   three", 0), "one two three");
    }

    #[std::prelude::v1::test]
    fn search_ranks_titles_first_and_offers_to_start_a_note() {
        let notes = vec![
            note("Groceries", "remember the redis book"),
            note("Redis design", "single threaded"),
            note("", "an untitled note about REDIS"),
            note("İstanbul trip", "İİİİ then the word redis after Turkish capitals"),
        ];
        let rows = search(&notes, &[], "  redis ");
        assert_eq!(titles(&rows), vec!["Redis design", "Groceries", "Untitled", "İstanbul trip", "Start redis"]);
        assert!(rows[0].snippet.is_none());
        assert_eq!(rows[1].snippet.as_deref(), Some("remember the redis book"));
        assert!(rows[3].snippet.as_deref().unwrap().contains("redis"));

        let all = search(&notes, &[], "");
        assert_eq!(all.len(), 4, "an empty query lists every note and no start row");
        assert_eq!(search(&notes, &[], "   ").len(), 4);
        assert_eq!(titles(&search(&notes, &[], "nothing like this")), vec!["Start nothing like this"]);

        let many: Vec<Note> = (0..40).map(|i| note(&format!("Note {i}"), "")).collect();
        let rows = search(&many, &[], "note");
        assert_eq!(rows.len(), 31);
        assert!(matches!(rows.last().unwrap().kind, RowKind::Create));
    }

    #[std::prelude::v1::test]
    fn rows_show_their_folder() {
        let mut filed = note("Plan", "");
        filed.folder = Some("f1".into());
        let mut orphan = note("Lost", "");
        orphan.folder = Some("deleted-folder".into());
        let folders = vec![Folder { id: "f1".into(), name: "Work".into(), collapsed: false }];
        let rows = search(&[filed, orphan], &folders, "");
        assert_eq!(rows[0].folder.as_deref(), Some("Work"));
        assert_eq!(rows[1].folder, None);
    }

    #[std::prelude::v1::test]
    fn folder_slash_name() {
        let p = |s: &str| parse_new_note(s);
        assert_eq!(p("Work / Q3 plan"), (Some("Work".into()), "Q3 plan".into()));
        assert_eq!(p("Work/Q3 plan"), (Some("Work".into()), "Q3 plan".into()));
        assert_eq!(p("  Just a title  "), (None, "Just a title".into()));
        assert_eq!(p("a / b / c"), (Some("a".into()), "b / c".into()));
        assert_eq!(p("Work /"), (Some("Work".into()), String::new()), "an untitled note in Work");
        assert_eq!(p("/etc/hosts"), (None, "/etc/hosts".into()), "nothing before the slash: no folder");
        assert_eq!(p(" / "), (None, "/".into()));
        assert_eq!(p("Été / idées"), (Some("Été".into()), "idées".into()));
    }

    #[std::prelude::v1::test]
    fn selection_stays_on_a_row() {
        assert_eq!(clamp_selection(0, 0), 0);
        assert_eq!(clamp_selection(3, 0), 0);
        assert_eq!(clamp_selection(5, 3), 2);
        assert_eq!(clamp_selection(1, 3), 1);
    }
}

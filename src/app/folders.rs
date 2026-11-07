//! Folder pages, note moves, and the drag shared by sidebar and folder-page rows.
use super::*;

#[derive(Clone)]
pub(super) struct DraggedNote {
    pub id: String,
    pub title: String,
}

pub(super) struct NoteDragPreview {
    title: String,
    offset: Point<Pixels>,
}

impl DraggedNote {
    pub(super) fn preview(&self, offset: Point<Pixels>, cx: &mut App) -> Entity<NoteDragPreview> {
        cx.new(|_| NoteDragPreview { title: self.title.clone(), offset })
    }
}

impl Render for NoteDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx);
        // GPUI subtracts the source click offset. Put the compact slip beside the
        // pointer regardless of where in a wide note row the drag began.
        div().relative().left(self.offset.x + px(12.)).top(self.offset.y + px(12.))
            .max_w(px(240.)).px(px(10.)).py(px(7.)).rounded(px(5.))
            .bg(t.p.paper).border_1().border_color(t.voice_ink.opacity(0.4))
            .shadow(vec![BoxShadow { color: t.p.shadow, offset: point(px(0.), px(3.)), blur_radius: px(10.), spread_radius: px(0.), inset: false }])
            .flex().items_center().gap(px(7.)).text_color(t.p.ink).font_family(SANS).text_size(px(ts::SMALL))
            .child(icon("file-text", 13., t.voice_ink))
            .child(div().min_w(px(0.)).truncate().child(self.title.clone()))
    }
}

impl AppView {
    pub(super) fn open_folder(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if id.as_ref().is_some_and(|id| !cx.global::<Store>().folders.iter().any(|f| &f.id == id)) { return; }
        self.page.update(cx, |p, cx| p.flush(cx));
        if self.folder_page.as_ref() != Some(&id) {
            self.folder_scroll.set_offset(point(px(0.), px(0.)));
        }
        self.folder_page = Some(id);
        self.sidebar_open = false;
        self.focus.focus(window, cx);
        cx.notify();
    }

    pub(super) fn move_note(&mut self, id: &str, folder: Option<String>, cx: &mut Context<Self>) {
        if folder.as_ref().is_some_and(|id| !cx.global::<Store>().folders.iter().any(|f| &f.id == id)) { return; }
        let active = self.page.read(cx).note_id() == id;
        let note = if active { Some(self.page.update(cx, |p, cx| p.note_snapshot(cx))) } else { cx.global::<Store>().note(id).cloned() };
        let Some(mut note) = note else { return };
        if note.folder == folder { return; }
        note.folder = folder.clone();
        note.updated_at = chrono::Utc::now();
        if let Err(error) = cx.global::<Store>().try_save_note(&note) {
            self.toast(&format!("Couldn't move this note: {error}"), cx);
            return;
        }
        if let Some(saved) = cx.global_mut::<Store>().notes.iter_mut().find(|n| n.id == id) { *saved = note; }
        if active {
            self.page.update(cx, |p, cx| { p.note.folder = folder.clone(); cx.notify(); });
        }
        let name = folder.as_ref().and_then(|id| cx.global::<Store>().folders.iter().find(|f| &f.id == id)).map_or("Unfiled", |f| f.name.as_str());
        self.toast(&format!("Moved to {name}"), cx);
        cx.notify();
    }

    /// A folder's page: a contents page on a note's own sheet. Its header row holds what
    /// a note's does for a folder (how many notes, a new one, the files they read), in the
    /// same place; the title sits where a note's does; then each note, its first line and
    /// when it last changed. A right click on a note has the note's menu.
    pub(super) fn folder_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = self.folder_page.clone().flatten();
        let store = cx.global::<Store>();
        let folder = id.as_ref().and_then(|id| store.folders.iter().find(|f| &f.id == id)).cloned();
        let title = folder.as_ref().map_or("Unfiled".to_string(), |f| f.name.clone());
        let notes: Vec<Note> = store.notes.iter().filter(|n| n.folder == id || (id.is_none() && n.folder.as_ref().is_some_and(|id| !store.folders.iter().any(|f| &f.id == id)))).cloned().collect();
        let t = Theme::get(cx).clone();
        let p = t.p;
        let n = t.note_size.as_f32();
        let ring = t.voice_ink;
        let control = move |id: &'static str| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(24.))
                .text_size(px(ts::UI))
                .text_color(p.graphite)
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .active(|s| s.opacity(0.7))
        };
        let mut header = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_x(px(16.))
            .gap_y(px(2.))
            .min_h(px(24.))
            .text_size(px(ts::UI))
            .child(div().text_color(p.faint).child(format!("{} {}", notes.len(), if notes.len() == 1 { "note" } else { "notes" })))
            .child(control("folder-new-note").child(icon("plus", 13., p.graphite)).child("New note").keyed_as("folder:new-note", 4., ring, cx, |this, window, cx| this.create_note("", None, window, cx)));
        // The files a folder's notes read, as a note's header shows its own: the folder's
        // name, its full paths in the tooltip.
        if let Some(folder) = &folder {
            let pick_id = folder.id.clone();
            let clear_id = folder.id.clone();
            let dirs = folder.local_directories.clone();
            let label = dirs.first().map(|path| {
                let name = std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path);
                if dirs.len() > 1 { format!("Files · {name} +{}", dirs.len() - 1) } else { format!("Files · {name}") }
            });
            let tip = if dirs.is_empty() { "Choose local folders for these notes to read".to_string() } else { format!("Read by every note here that has none of its own\n{}", dirs.join("\n")) };
            header = header
                .child(control("folder-paths")
                    .child(div().max_w(px(220.)).truncate().child(label.clone().unwrap_or_else(|| "Files".into())))
                    .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
                    .keyed_as("folder:paths", 4., ring, cx, move |this, _, cx| this.choose_folder_directories(pick_id.clone(), cx)))
                .when(label.is_some(), |d| d.child(control("folder-clear-paths").child("Unlink").keyed_as("folder:clear-paths", 4., ring, cx, move |this, _, cx| {
                    let store = cx.global_mut::<Store>();
                    if let Some(f) = store.folders.iter_mut().find(|f| f.id == clear_id) { f.local_directories.clear(); }
                    store.save_folders();
                    this.page.update(cx, |_, cx| cx.notify());
                    cx.notify();
                })));
        }
        let title = div()
            .font_family(crate::theme::DISPLAY)
            .font_weight(FontWeight::MEDIUM)
            .text_size(px((n * 1.9).round()))
            .line_height(px((n * 1.9 * 1.22).round()))
            .text_color(p.ink)
            .child(title);
        let mut list = div().mt(px((n * 1.1).round())).flex().flex_col().border_t_1().border_color(p.rule);
        if notes.is_empty() {
            list = list.child(div().py(px(20.)).text_size(px(ts::BODY)).line_height(px(20.)).text_color(p.graphite).child("No notes here yet. Start one, or drag a note here from the sidebar."));
        }
        for note in notes {
            let (open_id, menu_id) = (note.id.clone(), note.id.clone());
            let group = SharedString::from(format!("folder-note-group-{}", note.id));
            let title = if note.title.trim().is_empty() { "Untitled".to_string() } else { note.title.clone() };
            let excerpt = note.blocks.iter().skip(1).find(|b| !b.text.trim().is_empty()).map(|b| b.text.chars().take(160).collect::<String>());
            let threads = crate::between::open_here(&cx.global::<Store>().notes, &note);
            list = list.child(div().id(SharedString::from(format!("folder-note-{}", note.id)))
                .group(group.clone())
                .py(px(14.)).border_b_1().border_color(p.rule).cursor_pointer().active(|s| s.opacity(0.7))
                .on_drag(DraggedNote { id: note.id.clone(), title: title.clone() }, {
                    let app = cx.entity();
                    move |drag, offset, _, cx| {
                        app.update(cx, |app, cx| { app.sidebar_open = true; cx.notify(); });
                        drag.preview(offset, cx)
                    }
                })
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.note_menu(menu_id.clone(), ev.position, window, cx);
                }))
                // the title and the date share a baseline; the date keeps to the column's edge
                .child(div().flex().items_baseline().gap(px(16.))
                    .child(div().flex_1().min_w(px(0.)).font_family(crate::theme::SERIF).text_size(px((n * 1.22).round())).line_height(px((n * 1.22 * 1.35).round())).text_color(p.ink).truncate()
                        .group_hover(group, move |s| s.text_color(ring)).child(title))
                    .child(div().flex_none().text_size(px(ts::SMALL)).text_color(p.faint).child(note.updated_at.with_timezone(&chrono::Local).format("%-d %b").to_string())))
                .child(div().mt(px(2.)).text_size(px(ts::UI)).line_height(px(18.)).truncate()
                    .text_color(if excerpt.is_some() { p.graphite } else { p.faint })
                    .child(excerpt.unwrap_or_else(|| "Nothing written yet".into())))
                .when(threads > 0, |d| d.child(div().mt(px(6.)).flex().items_center().gap(px(6.)).text_size(px(ts::SMALL)).text_color(p.graphite)
                    .child(ui::small_dot(t.voice_ink, 5.))
                    .child(format!("{threads} {}", if threads == 1 { "loose thread" } else { "loose threads" }))))
                .keyed_as(format!("folder:note-{}", note.id), 4., ring, cx, move |this, window, cx| this.open_note(open_id.clone(), window, cx)));
        }
        let target = id.clone();
        let page = crate::note_page::sheet_page(window, &self.folder_scroll, &p, |left, column, width| {
            div()
                .relative()
                .w(px(width))
                .flex()
                .flex_col()
                .font_family(SANS)
                .child(div().ml(px(left)).w(px(column)).child(header))
                .child(div().h(px(18.)))
                .child(div().ml(px(left)).w(px(column)).child(title).child(list))
                .child(div().h(px(120.)))
                .into_any_element()
        });
        div().id("folder-page").relative().track_focus(&self.focus).size_full()
            .can_drop(move |value, _, cx| value.downcast_ref::<DraggedNote>().is_some_and(|drag| cx.global::<Store>().note(&drag.id).is_some_and(|n| n.folder != target)))
            // a note from the sidebar held over the page: the window's edge takes the thread
            .drag_over::<DraggedNote>(move |s, _, _, _| s.shadow(vec![BoxShadow { color: ring.opacity(0.55), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(3.), inset: true }]))
            .on_drop(cx.listener(move |this, drag: &DraggedNote, _, cx| this.move_note(&drag.id, id.clone(), cx)))
            .child(page)
            .into_any_element()
    }
}

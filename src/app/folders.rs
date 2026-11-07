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

    pub(super) fn folder_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let id = self.folder_page.clone().flatten();
        let store = cx.global::<Store>();
        let folder = id.as_ref().and_then(|id| store.folders.iter().find(|f| &f.id == id)).cloned();
        let title = folder.as_ref().map_or("Unfiled".to_string(), |f| f.name.clone());
        let notes: Vec<Note> = store.notes.iter().filter(|n| n.folder == id || (id.is_none() && n.folder.as_ref().is_some_and(|id| !store.folders.iter().any(|f| &f.id == id)))).cloned().collect();
        let t = Theme::get(cx).clone();
        let p = t.p;
        // A folder is the notebook's contents page: the writing comes first, its file context last.
        let mut contents = div().flex().flex_col()
            .child(div().flex().items_center().gap(px(16.)).pb(px(24.)).border_b_1().border_color(p.rule)
                .child(div().flex_1().min_w(px(0.))
                    .child(div().font_family(crate::theme::DISPLAY).text_size(px(30.)).child(title))
                    .child(div().mt(px(4.)).text_size(px(ts::SMALL)).text_color(p.graphite)
                        .child(format!("{} {}", notes.len(), if notes.len() == 1 { "note" } else { "notes" }))))
                .child(ui::ghost_button("folder-new-note", "New note", &t)
                    .keyed_as("folder:new-note", 4., t.voice_ink, cx, |this, window, cx| this.create_note("", None, window, cx))));
        if notes.is_empty() {
            contents = contents.child(div().py(px(40.)).text_color(p.graphite).child("No notes here yet. Create one, or drag a note from the sidebar."));
        }
        for note in notes {
            let open_id = note.id.clone();
            let title = if note.title.trim().is_empty() { "Untitled".to_string() } else { note.title.clone() };
            let excerpt = note.blocks.iter().skip(1).find(|b| !b.text.trim().is_empty()).map(|b| b.text.chars().take(160).collect::<String>()).unwrap_or_else(|| "Empty note".into());
            let threads = crate::between::open_here(&cx.global::<Store>().notes, &note);
            contents = contents.child(div().id(SharedString::from(format!("folder-note-{}", note.id)))
                .group(SharedString::from(format!("folder-note-group-{}", note.id)))
                .px(px(12.)).py(px(18.)).border_b_1().border_color(p.rule).cursor_pointer().active(|s| s.opacity(0.7))
                .on_drag(DraggedNote { id: note.id.clone(), title: title.clone() }, {
                    let app = cx.entity();
                    move |drag, offset, _, cx| {
                        app.update(cx, |app, cx| { app.sidebar_open = true; cx.notify(); });
                        drag.preview(offset, cx)
                    }
                })
                .child(div().flex().items_center().gap(px(16.))
                    .child(div().flex_1().min_w(px(0.)).font_family(crate::theme::SERIF).text_size(px(22.)).truncate()
                        .group_hover(SharedString::from(format!("folder-note-group-{}", note.id)), move |s| s.text_color(t.voice_ink)).child(title))
                    .child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child(note.updated_at.format("%-d %b").to_string()))
                    .child(icon("arrow-right", 14., t.voice_ink).invisible()
                        .group_hover(SharedString::from(format!("folder-note-group-{}", note.id)), |s| s.visible())))
                .child(div().mt(px(6.)).text_size(px(ts::UI)).text_color(p.graphite).truncate().child(excerpt))
                .when(threads > 0, |d| d.child(div().mt(px(8.)).flex().items_center().gap(px(6.)).text_size(px(ts::SMALL)).text_color(p.graphite)
                    .child(ui::small_dot(t.voice_ink, 5.))
                    .child(format!("{threads} {}", if threads == 1 { "loose thread" } else { "loose threads" }))))
                .keyed_as(format!("folder:note-{}", note.id), 4., t.voice_ink, cx, move |this, window, cx| this.open_note(open_id.clone(), window, cx)));
        }
        if let Some(folder) = folder {
            let pick_id = folder.id.clone();
            let clear_id = folder.id.clone();
            let mut paths = div().mt(px(28.)).flex().flex_col().gap(px(6.))
                .child(div().flex().items_center().gap(px(12.))
                    .child(div().flex_1().text_size(px(ts::SMALL)).text_color(p.graphite).child("Files for these notes"))
                    .child(ui::ghost_button("folder-paths", if folder.local_directories.is_empty() { "Link a folder" } else { "Change" }, &t).keyed_as("folder:paths", 4., t.voice_ink, cx, move |this, _, cx| this.choose_folder_directories(pick_id.clone(), cx)))
                    .when(!folder.local_directories.is_empty(), |d| d.child(ui::ghost_button("folder-clear-paths", "Unlink", &t)
                        .keyed_as("folder:clear-paths", 4., t.voice_ink, cx, move |this, _, cx| {
                            let store = cx.global_mut::<Store>();
                            if let Some(f) = store.folders.iter_mut().find(|f| f.id == clear_id) { f.local_directories.clear(); }
                            store.save_folders();
                            this.page.update(cx, |_, cx| cx.notify());
                            cx.notify();
                        }))));
            if folder.local_directories.is_empty() {
                paths = paths.child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child("Link a local folder for Inkling to read alongside these notes."));
            }
            for path in &folder.local_directories {
                paths = paths.child(div().flex().items_center().gap(px(8.)).text_size(px(ts::SMALL)).text_color(p.graphite)
                    .child(icon("folder", 12., p.graphite)).child(path.clone()));
            }
            contents = contents.child(paths);
        }
        let weave = canvas(|_, _, _| (), move |bounds, _, window, _| {
            let mut y = bounds.origin.y;
            while y < bounds.bottom() {
                window.paint_quad(fill(Bounds::new(point(bounds.origin.x, y), size(bounds.size.width, px(1.))), p.weave));
                y += px(3.);
            }
            let mut x = bounds.origin.x;
            while x < bounds.right() {
                window.paint_quad(fill(Bounds::new(point(x, bounds.origin.y), size(px(1.), bounds.size.height)), p.weave));
                x += px(3.);
            }
        }).absolute().top_0().left_0().size_full();
        let target = id.clone();
        div().id("folder-page").relative().track_focus(&self.focus).size_full().overflow_y_scroll().bg(p.cloth)
            .px(px(36.)).pt(px(64.)).pb(px(36.))
            .can_drop(move |value, _, cx| value.downcast_ref::<DraggedNote>().is_some_and(|drag| cx.global::<Store>().note(&drag.id).is_some_and(|n| n.folder != target)))
            .drag_over::<DraggedNote>(move |s, _, _, _| s.bg(t.voice_ink.opacity(0.08)))
            .on_drop(cx.listener(move |this, drag: &DraggedNote, _, cx| this.move_note(&drag.id, id.clone(), cx)))
            .child(weave)
            .child(div().relative().max_w(px(760.)).mx_auto().min_h(px(420.)).px(px(36.)).py(px(32.)).bg(p.paper).rounded(px(4.)).child(contents))
            .into_any_element()
    }
}

//! Folder pages, note moves, and the drag shared by sidebar and folder-page rows.
use super::*;

#[derive(Clone)]
pub(super) struct DraggedNote {
    pub id: String,
    pub title: String,
}

impl Render for DraggedNote {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx);
        div().px(px(12.)).py(px(8.)).rounded(px(6.)).bg(t.p.card).border_1().border_color(t.p.rule)
            .text_color(t.p.ink).font_family(SANS).text_size(px(ts::UI)).child(self.title.clone())
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
        let notes: Vec<Note> = store.notes.iter().filter(|n| n.folder == id).cloned().collect();
        let t = Theme::get(cx).clone();
        let p = t.p;
        let mut contents = div().flex().flex_col().gap(px(16.))
            .child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child("NOTEBOOK"))
            .child(div().flex().items_center().gap(px(16.))
                .child(div().flex_1().font_family(crate::theme::DISPLAY).text_size(px(34.)).child(title))
                .child(ui::ghost_button("folder-new-note", "New note", &t)
                    .keyed_as("folder:new-note", 4., t.voice_ink, cx, |this, window, cx| this.create_note("", None, window, cx))))
            .child(div().text_size(px(ts::UI)).text_color(p.graphite).child(format!("{} {} · Drag notes here to move them", notes.len(), if notes.len() == 1 { "note" } else { "notes" })));
        if let Some(folder) = folder {
            let pick_id = folder.id.clone();
            let clear_id = folder.id.clone();
            let mut paths = div().flex().flex_col().gap(px(6.)).py(px(16.)).border_t_1().border_b_1().border_color(p.rule)
                .child(div().flex().items_center().gap(px(12.))
                    .child(div().flex_1().text_size(px(ts::UI)).child("Local directories"))
                    .child(ui::ghost_button("folder-paths", "Choose", &t).keyed_as("folder:paths", 4., t.voice_ink, cx, move |this, _, cx| this.choose_folder_directories(pick_id.clone(), cx)))
                    .when(!folder.local_directories.is_empty(), |d| d.child(ui::ghost_button("folder-clear-paths", "Clear", &t)
                        .keyed_as("folder:clear-paths", 4., t.voice_ink, cx, move |this, _, cx| {
                            let store = cx.global_mut::<Store>();
                            if let Some(f) = store.folders.iter_mut().find(|f| f.id == clear_id) { f.local_directories.clear(); }
                            store.save_folders();
                            this.page.update(cx, |_, cx| cx.notify());
                            cx.notify();
                        }))));
            if folder.local_directories.is_empty() {
                paths = paths.child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child("Choose directories for the notes in this folder."));
            }
            for path in &folder.local_directories {
                paths = paths.child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child(path.clone()));
            }
            contents = contents.child(paths);
        }
        if notes.is_empty() {
            contents = contents.child(div().py(px(40.)).text_color(p.graphite).child("No notes here yet. Create one, or drag a note from the sidebar."));
        }
        for note in notes {
            let open_id = note.id.clone();
            let title = if note.title.trim().is_empty() { "Untitled".to_string() } else { note.title.clone() };
            let excerpt = note.blocks.iter().skip(1).find(|b| !b.text.trim().is_empty()).map(|b| b.text.chars().take(160).collect::<String>()).unwrap_or_else(|| "Empty note".into());
            let threads = crate::between::open_here(&cx.global::<Store>().notes, &note);
            contents = contents.child(div().id(SharedString::from(format!("folder-note-{}", note.id)))
                .py(px(16.)).border_b_1().border_color(p.rule).cursor_pointer().hover(move |s| s.bg(p.hover))
                .on_drag(DraggedNote { id: note.id.clone(), title: title.clone() }, |drag, _, _, cx| cx.new(|_| drag.clone()))
                .child(div().font_family(crate::theme::SERIF).text_size(px(22.)).child(title))
                .child(div().mt(px(5.)).text_size(px(ts::UI)).text_color(p.graphite).child(excerpt))
                .child(div().mt(px(8.)).text_size(px(ts::SMALL)).text_color(p.graphite).child(format!("{} · {} loose threads", note.updated_at.format("%-d %b"), threads)))
                .keyed_as(format!("folder:note-{}", note.id), 4., t.voice_ink, cx, move |this, window, cx| this.open_note(open_id.clone(), window, cx)));
        }
        div().id("folder-page").track_focus(&self.focus).size_full().overflow_y_scroll().bg(p.cloth)
            .px(px(36.)).pt(px(64.)).pb(px(36.))
            .drag_over::<DraggedNote>(move |s, _, _, _| s.bg(p.hover))
            .on_drop(cx.listener(move |this, drag: &DraggedNote, _, cx| this.move_note(&drag.id, id.clone(), cx)))
            .child(div().max_w(px(880.)).mx_auto().min_h(px(600.)).px(px(40.)).py(px(40.)).bg(p.paper).rounded(px(4.)).child(contents))
            .into_any_element()
    }
}

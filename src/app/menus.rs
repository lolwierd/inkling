//! Right-click menus on notes and folders, in the sidebar and on a folder's page. They
//! are macOS's own menus, so they look, move and type as every other app's do and can
//! reach past the window's edge. A test window keeps the items instead of showing them:
//! `menu: Context > Delete Note` picks one, as a click on it would.
use super::*;
use gpui_kit::component::native_menu::NativeMenu;

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct OpenNoteAt {
    pub id: String,
}

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct MoveNoteTo {
    pub id: String,
    /// None is Unfiled.
    pub folder: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct DeleteNoteAt {
    pub id: String,
}

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct NewNoteIn {
    pub folder: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct OpenFolderAt {
    pub folder: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct RenameFolder {
    pub id: String,
}

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = app, no_json)]
pub struct DeleteFolderAt {
    pub id: String,
}

/// Whether this is a test window (`INKLING_BACKGROUND`, set only by `scripts/dev`): a
/// native menu there would pop up over the owner's screen and wait for their pointer.
fn test_window() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("INKLING_BACKGROUND").is_some())
}

/// A menu as macOS shows it and as a script reads it: each item's label (a submenu's
/// items as `Move To > Work`) with its action.
pub(super) struct ContextMenu {
    native: NativeMenu,
    pub items: Vec<(String, Box<dyn Action>)>,
}

impl ContextMenu {
    fn new() -> Self {
        Self { native: NativeMenu::new(), items: vec![] }
    }

    fn item(mut self, label: &str, action: impl Action) -> Self {
        self.items.push((label.to_string(), action.boxed_clone()));
        self.native = self.native.menu(label.to_string(), Box::new(action));
        self
    }

    fn checked(mut self, label: &str, on: bool, action: impl Action) -> Self {
        self.items.push((label.to_string(), action.boxed_clone()));
        self.native = self.native.menu_with_check(label.to_string(), on, Box::new(action));
        self
    }

    fn separator(mut self) -> Self {
        self.native = self.native.separator();
        self
    }

    fn submenu(mut self, label: &str, inner: ContextMenu) -> Self {
        self.items.extend(inner.items.into_iter().map(|(name, action)| (format!("{label} > {name}"), action)));
        self.native = self.native.submenu(label.to_string(), inner.native);
        self
    }
}

impl AppView {
    /// A note's menu: open it, move it to another folder, delete it (with Undo).
    pub(super) fn note_menu(&mut self, id: String, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let store = cx.global::<Store>();
        let Some(note) = store.note(&id) else { return };
        let here = note.folder.clone().filter(|f| store.folders.iter().any(|x| &x.id == f));
        let mut to = ContextMenu::new().checked("Unfiled", here.is_none(), MoveNoteTo { id: id.clone(), folder: None });
        if !store.folders.is_empty() {
            to = to.separator();
        }
        for f in &store.folders {
            to = to.checked(&f.name, here.as_ref() == Some(&f.id), MoveNoteTo { id: id.clone(), folder: Some(f.id.clone()) });
        }
        let menu = ContextMenu::new()
            .item("Open", OpenNoteAt { id: id.clone() })
            .submenu("Move To", to)
            .separator()
            .item("Delete Note", DeleteNoteAt { id });
        self.show_menu(menu, at, window, cx);
    }

    /// A folder's menu: a note in it, its page, a new name, or the folder gone (its
    /// notes stay, in Unfiled; Undo puts it back). Unfiled has only the first two.
    pub(super) fn folder_menu(&mut self, folder: Option<String>, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let mut menu = ContextMenu::new()
            .item("New Note", NewNoteIn { folder: folder.clone() })
            .item("Open", OpenFolderAt { folder: folder.clone() });
        if let Some(id) = folder {
            menu = menu.item("Rename…", RenameFolder { id: id.clone() }).separator().item("Delete Folder", DeleteFolderAt { id });
        }
        self.show_menu(menu, at, window, cx);
    }

    fn show_menu(&mut self, menu: ContextMenu, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let ContextMenu { native, items } = menu;
        if test_window() {
            log::info!("script context menu: {}", items.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>().join(" | "));
        } else {
            native.show(at, window, cx);
        }
        self.context_menu = items;
    }

    /// `menu: Context > Item`: the item of the menu shown last, chosen as a click would.
    pub(super) fn pick_context_item(&mut self, label: &str, window: &mut Window, cx: &mut Context<Self>) {
        match self.context_menu.iter().find(|(l, _)| l == label).map(|(_, a)| a.boxed_clone()) {
            Some(action) => window.dispatch_action(action, cx),
            None => log::warn!("script: no context menu item {label:?}"),
        }
    }

    /// Delete the folder, its notes kept in Unfiled; the toast's Undo puts it back.
    pub(super) fn delete_folder(&mut self, id: String, cx: &mut Context<Self>) {
        let kept = {
            let store = cx.global::<Store>();
            store.folders.iter().position(|f| f.id == id).map(|at| {
                let notes = store.notes.iter().filter(|n| n.folder.as_deref() == Some(id.as_str())).map(|n| n.id.clone()).collect();
                (store.folders[at].clone(), at, notes)
            })
        };
        let Some(kept) = kept else { return };
        // the open note may be in it: it is in Unfiled now
        self.page.update(cx, |page, cx| {
            page.flush(cx);
            if page.note.folder.as_deref() == Some(id.as_str()) {
                page.note.folder = None;
            }
        });
        cx.global_mut::<Store>().delete_folder(&id);
        if self.folder_page.as_ref() == Some(&Some(id.clone())) {
            self.folder_page = Some(None);
        }
        if self.renaming.as_ref().is_some_and(|(r, _)| r == &id) {
            self.renaming = None;
        }
        self.deleted_folder = Some(kept);
        crate::toast::show(cx, Kind::FolderDeleted, "Folder deleted; its notes are in Unfiled");
        cx.notify();
    }

    /// Put the folder deleted last back where it was, with the notes that were in it.
    pub(super) fn undo_delete_folder(&mut self, cx: &mut Context<Self>) {
        let Some((folder, at, notes)) = self.deleted_folder.take() else { return };
        if !crate::toast::live(cx, Kind::FolderDeleted) {
            return;
        }
        let id = folder.id.clone();
        let open = self.page.read(cx).note_id().to_string();
        {
            let store = cx.global_mut::<Store>();
            let at = at.min(store.folders.len());
            store.folders.insert(at, folder);
            store.save_folders();
            for note in store.notes.iter_mut().filter(|n| notes.contains(&n.id) && n.folder.is_none()) {
                note.folder = Some(id.clone());
            }
            for note in store.notes.iter().filter(|n| notes.contains(&n.id) && n.folder.as_deref() == Some(id.as_str())) {
                store.save_note(note);
            }
        }
        if notes.contains(&open) {
            self.page.update(cx, |page, cx| {
                page.note.folder = Some(id.clone());
                cx.notify();
            });
        }
        crate::toast::close(cx, Kind::FolderDeleted);
        cx.notify();
    }

    /// A name field in place of the folder's name in the sidebar: Return or clicking
    /// away keeps the new name, Escape keeps the old one.
    pub(super) fn start_rename(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        use gpui_kit::component::input::{InputEvent, InputState};
        let Some(name) = cx.global::<Store>().folders.iter().find(|f| f.id == id).map(|f| f.name.clone()) else { return };
        self.sidebar_open = true;
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Folder name").default_value(name));
        let sub = cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, window, cx| {
            if this.renaming.as_ref().map(|(_, i)| i) != Some(input) {
                return;
            }
            match ev {
                InputEvent::PressEnter { .. } | InputEvent::Blur => this.finish_rename(true, window, cx),
                _ => {}
            }
        });
        self.rename_sub = Some(sub);
        input.update(cx, |i, cx| {
            i.focus(window, cx);
            i.select_all(window, cx);
        });
        self.renaming = Some((id, input));
        cx.notify();
    }

    pub(super) fn finish_rename(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, input)) = self.renaming.take() else { return };
        let name = input.read(cx).value().trim().to_string();
        if keep && !name.is_empty() {
            let store = cx.global_mut::<Store>();
            if store.folders.iter().any(|f| f.id != id && same_folder_name(&f.name, &name)) {
                self.toast(&format!("A folder called {name} is already here"), cx);
            } else if let Some(f) = store.folders.iter_mut().find(|f| f.id == id) {
                f.name = name;
                store.save_folders();
            }
        }
        if input.read(cx).focus_handle(cx).is_focused(window) {
            self.focus_editor(window, cx);
        }
        cx.notify();
    }
}

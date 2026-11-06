//! Every keyboard shortcut in one table: what it does, where it works, the keys it ships
//! with, and the keys you gave it in Settings › Shortcuts.
//!
//! The app's own shortcuts are listed here; the editor's come from `editor::bindings()`,
//! so the editor still owns its defaults. Only the shortcuts you changed are saved, in
//! settings.json under `shortcuts` (action name → keys; an empty list means none), so a
//! default that changes in a later build still reaches everyone who left it alone.
//!
//! `install` binds the table at launch. `apply` rebinds after any change: the keymap is
//! rebuilt from what the component library bound plus this table, and the menu bar is set
//! again, so new keys work at once in the note and in the menus, and old ones stop.

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use gpui_kit::*;

use crate::app::{
    CopyNote, DismissThread, ExportNote, FindLooseThread, JustMyWords, LooseEnds, NewNote, NextThread, OpenPalette,
    OpenSettings, PrevThread, Quit, ToggleChat, ToggleSidebar, TurnOver, UndoDismiss,
};
use crate::store::Store;

/// The app's shortcuts out of the box. They work anywhere in the window.
fn app_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-\\", ToggleSidebar, None),
        KeyBinding::new("cmd-n", NewNote, None),
        KeyBinding::new("cmd-k", OpenPalette, None),
        KeyBinding::new("cmd-shift-k", LooseEnds, None),
        KeyBinding::new("cmd-j", ToggleChat, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-shift-.", JustMyWords, None),
        KeyBinding::new("cmd->", JustMyWords, None),
        KeyBinding::new("cmd-shift-c", CopyNote, None),
        KeyBinding::new("cmd-shift-e", ExportNote, None),
        KeyBinding::new("cmd-enter", FindLooseThread, None),
        // the margin, from the keyboard
        KeyBinding::new("alt-cmd-down", NextThread, None),
        KeyBinding::new("alt-cmd-up", PrevThread, None),
        KeyBinding::new("cmd-shift-backspace", DismissThread, None),
        KeyBinding::new("alt-cmd-z", UndoDismiss, None),
        KeyBinding::new("cmd-shift-b", TurnOver, None),
    ]
}

/// Default keys that are another spelling of a default shown beside them: (action, alias,
/// the default it spells). macOS reports ⌘⇧. as ⌘> on a US layout, so both are bound, and
/// only one is shown.
const ALIASES: &[(&str, &str, &str)] = &[("app::JustMyWords", "cmd->", "cmd-shift-.")];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Notes,
    Threads,
    Window,
    Formatting,
    Editing,
    Moving,
    Selecting,
    Deleting,
}

impl Group {
    pub const ALL: [Group; 8] = [
        Group::Notes,
        Group::Threads,
        Group::Window,
        Group::Formatting,
        Group::Editing,
        Group::Moving,
        Group::Selecting,
        Group::Deleting,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Notes => "Notes",
            Group::Threads => "Threads",
            Group::Window => "Window",
            Group::Formatting => "Formatting",
            Group::Editing => "Editing",
            Group::Moving => "Moving the caret",
            Group::Selecting => "Selecting",
            Group::Deleting => "Deleting",
        }
    }

    /// Where the group's keys work.
    pub fn scope(self) -> &'static str {
        match self {
            Group::Notes | Group::Threads | Group::Window => "Anywhere in Inkling",
            _ => "While writing",
        }
    }
}

/// What each action is called in Settings, its group, and a line about it when the name
/// alone does not say enough. Keyed by action name (what settings.json stores); the order
/// here is the order on screen.
const LABELS: &[(&str, &str, Group, Option<&str>)] = &[
    ("app::NewNote", "New note", Group::Notes, None),
    ("app::OpenPalette", "Find or start a note", Group::Notes, None),
    ("app::ToggleSidebar", "Show or hide the notes list", Group::Notes, None),
    ("app::CopyNote", "Copy as Markdown", Group::Notes, None),
    ("app::ExportNote", "Export…", Group::Notes, None),
    ("app::FindLooseThread", "Find a loose thread", Group::Threads, Some("In a selection. With a thread open, it ties it off.")),
    ("app::NextThread", "Next open thread", Group::Threads, None),
    ("app::PrevThread", "Previous open thread", Group::Threads, None),
    ("app::LooseEnds", "Loose ends", Group::Threads, Some("Every open thread in the notebook, oldest first.")),
    ("app::DismissThread", "Let stand or dismiss the open thread", Group::Threads, Some("A mind's question or correction is let stand, with what you typed in its box as your reason.")),
    ("app::UndoDismiss", "Undo let stand or dismiss", Group::Threads, None),
    ("app::JustMyWords", "Hide threads", Group::Threads, None),
    ("app::TurnOver", "Turn the note over", Group::Threads, Some("Every thread the note has had, and how each one ended.")),
    ("app::ToggleChat", "Chat", Group::Window, None),
    ("app::OpenSettings", "Settings", Group::Window, None),
    ("app::Quit", "Quit Inkling", Group::Window, None),
    ("editor::ToggleBold", "Bold", Group::Formatting, None),
    ("editor::ToggleItalic", "Italic", Group::Formatting, None),
    ("editor::ToggleUnderline", "Underline", Group::Formatting, None),
    ("editor::ToggleStrike", "Strikethrough", Group::Formatting, None),
    ("editor::ToggleCode", "Code", Group::Formatting, None),
    ("editor::ToggleCheck", "Check or uncheck a checklist item", Group::Formatting, Some("Any other line becomes a checklist item.")),
    ("editor::IndentList", "Indent a list item", Group::Formatting, None),
    ("editor::OutdentList", "Outdent a list item", Group::Formatting, None),
    ("editor::Undo", "Undo", Group::Editing, None),
    ("editor::Redo", "Redo", Group::Editing, None),
    ("editor::Cut", "Cut", Group::Editing, None),
    ("editor::Copy", "Copy", Group::Editing, None),
    ("editor::Paste", "Paste", Group::Editing, None),
    ("editor::SelectAll", "Select all", Group::Editing, None),
    ("editor::Newline", "New paragraph", Group::Editing, None),
    ("editor::Escape", "Close the thread or toolbar", Group::Editing, None),
    ("editor::ShowCharacterPalette", "Emoji & symbols", Group::Editing, None),
    ("editor::MoveLeft", "Left", Group::Moving, None),
    ("editor::MoveRight", "Right", Group::Moving, None),
    ("editor::MoveUp", "Up a line", Group::Moving, None),
    ("editor::MoveDown", "Down a line", Group::Moving, None),
    ("editor::MoveWordLeft", "Word left", Group::Moving, None),
    ("editor::MoveWordRight", "Word right", Group::Moving, None),
    ("editor::MoveLineStart", "Start of the line", Group::Moving, None),
    ("editor::MoveLineEnd", "End of the line", Group::Moving, None),
    ("editor::MoveDocStart", "Top of the note", Group::Moving, None),
    ("editor::MoveDocEnd", "End of the note", Group::Moving, None),
    ("editor::SelectLeft", "Left", Group::Selecting, None),
    ("editor::SelectRight", "Right", Group::Selecting, None),
    ("editor::SelectUp", "Up a line", Group::Selecting, None),
    ("editor::SelectDown", "Down a line", Group::Selecting, None),
    ("editor::SelectWordLeft", "Word left", Group::Selecting, None),
    ("editor::SelectWordRight", "Word right", Group::Selecting, None),
    ("editor::SelectLineStart", "To the start of the line", Group::Selecting, None),
    ("editor::SelectLineEnd", "To the end of the line", Group::Selecting, None),
    ("editor::SelectDocStart", "To the top of the note", Group::Selecting, None),
    ("editor::SelectDocEnd", "To the end of the note", Group::Selecting, None),
    ("editor::Backspace", "Character before", Group::Deleting, None),
    ("editor::Delete", "Character after", Group::Deleting, None),
    ("editor::DeleteWordLeft", "Word before", Group::Deleting, None),
    ("editor::DeleteWordRight", "Word after", Group::Deleting, None),
    ("editor::DeleteLineLeft", "To the start of the line", Group::Deleting, None),
];

/// One row of Settings › Shortcuts.
pub struct Shortcut {
    /// The action's name, e.g. "app::NewNote": what settings.json stores.
    pub name: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    pub group: Group,
    /// Keys out of the box, in the order shown; the first is the one the menus show.
    pub defaults: Vec<String>,
    /// Bound with the defaults but never shown (see [`ALIASES`]).
    aliases: Vec<String>,
    /// Where it works: none for anywhere in the window, else the editor's context.
    context: Option<Rc<KeyBindingContextPredicate>>,
    action: Box<dyn Action>,
}

impl Shortcut {
    /// Whether this and `other` can both be reached by the same keys: either works
    /// everywhere, or both work in the same place.
    fn overlaps(&self, other: &Shortcut) -> bool {
        match (&self.context, &other.context) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        }
    }
}

/// The table, built from the app's and the editor's default bindings.
pub fn table() -> Vec<Shortcut> {
    let mut out: Vec<Shortcut> = vec![];
    for b in app_bindings().into_iter().chain(crate::editor::bindings()) {
        let name = b.action().name();
        let keys = keys_of(&b);
        let alias = ALIASES.iter().any(|(n, a, _)| *n == name && *a == keys);
        let row = match out.iter_mut().position(|s| s.name == name) {
            Some(i) => &mut out[i],
            None => {
                let (label, group, hint) = LABELS
                    .iter()
                    .find(|l| l.0 == name)
                    .map(|l| (l.1, l.2, l.3))
                    .unwrap_or((name, Group::Editing, None));
                out.push(Shortcut {
                    name,
                    label,
                    hint,
                    group,
                    defaults: vec![],
                    aliases: vec![],
                    context: b.predicate(),
                    action: b.action().boxed_clone(),
                });
                out.last_mut().expect("just pushed")
            }
        };
        if alias {
            row.aliases.push(keys);
        } else {
            row.defaults.push(keys);
        }
    }
    let rank = |s: &Shortcut| (s.group as usize, LABELS.iter().position(|l| l.0 == s.name).unwrap_or(usize::MAX));
    out.sort_by_key(rank);
    out
}

fn keys_of(b: &KeyBinding) -> String {
    b.keystrokes().iter().map(|k| k.unparse(OperatingSystem::Mac)).collect::<Vec<_>>().join(" ")
}

/// The keys shown for `s`: yours if you changed it (maybe none), else its defaults.
pub fn shown<'a>(s: &'a Shortcut, custom: &'a BTreeMap<String, Vec<String>>) -> &'a [String] {
    custom.get(s.name).map(Vec::as_slice).unwrap_or(&s.defaults)
}

/// The keys `s` answers to: what is shown, plus the hidden aliases while it has its defaults.
fn bound<'a>(s: &'a Shortcut, custom: &'a BTreeMap<String, Vec<String>>) -> Vec<&'a str> {
    match custom.get(s.name) {
        Some(keys) => keys.iter().map(String::as_str).collect(),
        None => s.defaults.iter().chain(&s.aliases).map(String::as_str).collect(),
    }
}

/// Keys as GPUI matches them: modifiers and key. Strings that do not parse compare as written.
fn chord(keys: &str) -> (Option<(Modifiers, String)>, String) {
    match keys.split_whitespace().collect::<Vec<_>>().as_slice() {
        [one] => match Keystroke::parse_with_os(one, OperatingSystem::Mac) {
            Ok(k) => (Some((k.modifiers, k.key)), String::new()),
            Err(_) => (None, keys.to_string()),
        },
        _ => (None, keys.to_string()),
    }
}

/// A clash: these keys (as `s` shows them) also run `other`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clash {
    pub keys: String,
    pub other: &'static str,
}

/// For every shortcut that shares keys with another where both work, which keys and with whom.
pub fn clashes(table: &[Shortcut], custom: &BTreeMap<String, Vec<String>>) -> HashMap<&'static str, Vec<Clash>> {
    let mut out: HashMap<&'static str, Vec<Clash>> = HashMap::new();
    for (i, a) in table.iter().enumerate() {
        for b in &table[i + 1..] {
            if !a.overlaps(b) {
                continue;
            }
            for ka in bound(a, custom) {
                for kb in bound(b, custom) {
                    if chord(ka) != chord(kb) {
                        continue;
                    }
                    for (row, keys, other) in [(a, ka, b), (b, kb, a)] {
                        let keys = shown_spelling(row, keys);
                        let list = out.entry(row.name).or_default();
                        let clash = Clash { keys, other: other.name };
                        if !list.contains(&clash) {
                            list.push(clash);
                        }
                    }
                }
            }
        }
    }
    out
}

/// A hidden alias is reported as the default it spells.
fn shown_spelling(s: &Shortcut, keys: &str) -> String {
    ALIASES
        .iter()
        .find(|(n, a, _)| *n == s.name && *a == keys)
        .map(|(_, _, of)| of.to_string())
        .unwrap_or_else(|| keys.to_string())
}

/// What a key press means while recording a shortcut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Take {
    /// Use these keys.
    Keys(String),
    /// A modifier on its own: keep waiting.
    Wait,
    /// Not usable; says why.
    Refuse(&'static str),
}

/// Keys that do something other than type, so they can be a shortcut on their own.
fn is_named_key(key: &str) -> bool {
    matches!(
        key,
        "left" | "right" | "up" | "down" | "home" | "end" | "pageup" | "pagedown" | "backspace" | "delete" | "tab"
            | "enter" | "escape" | "insert"
    ) || key.strip_prefix('f').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

pub fn take(k: &Keystroke) -> Take {
    if k.is_modifier_key() || k.key.is_empty() {
        return Take::Wait;
    }
    let m = k.modifiers;
    if is_named_key(&k.key) || m.platform || m.control || m.alt || m.function {
        return Take::Keys(k.unparse(OperatingSystem::Mac));
    }
    Take::Refuse("Add ⌘, ⌥ or ⌃. On its own, that key types text.")
}

/// Keys as a macOS menu shows them: modifiers in Apple's order (⌃⌥⇧⌘), then the key.
pub fn glyphs(keys: &str) -> String {
    keys.split_whitespace()
        .map(|one| match Keystroke::parse_with_os(one, OperatingSystem::Mac) {
            Ok(k) => {
                let m = k.modifiers;
                let mut s = String::new();
                if m.function {
                    s.push_str("fn ");
                }
                for (on, glyph) in [(m.control, '⌃'), (m.alt, '⌥'), (m.shift, '⇧'), (m.platform, '⌘')] {
                    if on {
                        s.push(glyph);
                    }
                }
                s.push_str(&key_glyph(&k.key));
                s
            }
            Err(_) => one.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn key_glyph(key: &str) -> String {
    let named = match key {
        "left" => "←",
        "right" => "→",
        "up" => "↑",
        "down" => "↓",
        "backspace" => "⌫",
        "delete" => "⌦",
        // Return, as macOS menus draw it (↵ is not a Mac key)
        "enter" => "↩",
        "tab" => "⇥",
        "escape" => "⎋",
        "space" => "Space",
        "home" => "↖",
        "end" => "↘",
        "pageup" => "⇞",
        "pagedown" => "⇟",
        "insert" => "Ins",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

// ----- live keymap ------------------------------------------------------------------

/// The table, kept for the app's lifetime.
pub struct Shortcuts(pub Vec<Shortcut>);

impl Global for Shortcuts {}

/// Bind every shortcut. Call once at launch, after the component library's own keys and
/// with the store loaded, before the menus are set.
pub fn install(cx: &mut App) {
    cx.set_global(Shortcuts(table()));
    apply(cx);
}

/// Rebuild the keymap from the table and settings.json, then set the menus again.
pub fn apply(cx: &mut App) {
    let ours: Vec<KeyBinding> = {
        let custom = &cx.global::<Store>().settings.shortcuts;
        let mut out = vec![];
        for s in &cx.global::<Shortcuts>().0 {
            for keys in bound(s, custom) {
                match KeyBinding::load(keys, s.action.boxed_clone(), s.context.clone(), false, None, cx) {
                    Ok(b) => out.push(b),
                    Err(e) => log::warn!("shortcuts: skipping {keys:?} for {}: {e}", s.name),
                }
            }
        }
        out
    };
    // Everything not in the table (the component library's keys, for its inputs and
    // menus) stays, in its order, ahead of ours as at launch.
    let others: Vec<KeyBinding> = {
        let table = &cx.global::<Shortcuts>().0;
        let keymap = cx.key_bindings();
        let keymap = keymap.borrow();
        keymap.bindings().filter(|b| !table.iter().any(|s| s.name == b.action().name())).cloned().collect()
    };
    cx.clear_key_bindings();
    cx.bind_keys(others);
    cx.bind_keys(ours);
    refresh_menus(cx);
}

/// A menu item shows the keys its action had when the menus were set, and AppKit still
/// runs it for those keys, so the menus are set again from the keymap as it is now.
fn refresh_menus(cx: &mut App) {
    let Some(menus) = cx.get_menus() else { return };
    cx.set_menus(menus.into_iter().map(menu_of).collect::<Vec<_>>());
}

fn menu_of(m: OwnedMenu) -> Menu {
    Menu { name: m.name, items: m.items.into_iter().map(item_of).collect(), disabled: m.disabled }
}

fn item_of(item: OwnedMenuItem) -> MenuItem {
    match item {
        OwnedMenuItem::Separator => MenuItem::Separator,
        OwnedMenuItem::Submenu(m) => MenuItem::Submenu(menu_of(m)),
        OwnedMenuItem::SystemMenu(m) => MenuItem::SystemMenu(OsMenu { name: m.name, menu_type: m.menu_type }),
        OwnedMenuItem::Action { name, action, os_action, checked, disabled } => {
            MenuItem::Action { name: name.into(), action, os_action, checked, disabled }
        }
    }
}

/// Give `name` these keys (an empty list leaves it without a shortcut), save, and rebind.
/// Keys equal to the defaults are not stored, so the shortcut follows the defaults again.
pub fn set(name: &str, keys: Vec<String>, cx: &mut App) {
    let defaults = cx.global::<Shortcuts>().0.iter().find(|s| s.name == name).map(|s| s.defaults.clone());
    let store = cx.global_mut::<Store>();
    if Some(&keys) == defaults.as_ref() {
        store.settings.shortcuts.remove(name);
    } else {
        store.settings.shortcuts.insert(name.to_string(), keys);
    }
    store.save_settings();
    apply(cx);
}

/// Back to the defaults: one shortcut, or all of them with `None`.
pub fn reset(name: Option<&str>, cx: &mut App) {
    let store = cx.global_mut::<Store>();
    match name {
        Some(n) => {
            store.settings.shortcuts.remove(n);
        }
        None => store.settings.shortcuts.clear(),
    }
    store.save_settings();
    apply(cx);
}

/// `keys` recorded for `slot` of a shortcut's shown keys (`None` adds them): the new list.
/// The same keys twice collapse to one.
pub fn with_keys(current: &[String], slot: Option<usize>, keys: String) -> Vec<String> {
    let mut out: Vec<String> = current.to_vec();
    match slot {
        Some(i) if i < out.len() => out[i] = keys.clone(),
        _ => out.push(keys.clone()),
    }
    let mut seen = vec![];
    out.retain(|k| {
        let c = chord(k);
        let fresh = !seen.contains(&c);
        seen.push(c);
        fresh
    });
    out
}

/// The keys for an action as a menu would show them, for hints elsewhere in the app.
pub fn keys_for(name: &str, cx: &App) -> Option<String> {
    let table = &cx.try_global::<Shortcuts>()?.0;
    let s = table.iter().find(|s| s.name == name)?;
    shown(s, &cx.global::<Store>().settings.shortcuts).first().map(|k| glyphs(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs.iter().map(|(n, k)| (n.to_string(), k.iter().map(|s| s.to_string()).collect())).collect()
    }

    fn row<'a>(t: &'a [Shortcut], name: &str) -> &'a Shortcut {
        t.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no row {name}"))
    }

    #[std::prelude::v1::test]
    fn every_binding_has_a_labelled_row_and_keeps_its_keys() {
        let t = table();
        let mut expected: Vec<(String, String)> = app_bindings()
            .iter()
            .chain(crate::editor::bindings().iter())
            .map(|b| (b.action().name().to_string(), keys_of(b)))
            .collect();
        let mut got: Vec<(String, String)> = t
            .iter()
            .flat_map(|s| s.defaults.iter().chain(&s.aliases).map(move |k| (s.name.to_string(), k.clone())))
            .collect();
        expected.sort();
        got.sort();
        assert_eq!(got, expected, "the table binds exactly what the app and the editor bound before");
        for s in &t {
            assert!(LABELS.iter().any(|l| l.0 == s.name), "{} has no label", s.name);
            assert!(!s.defaults.is_empty(), "{} shows no keys", s.name);
        }
        // Every label names a real action, once.
        for (i, l) in LABELS.iter().enumerate() {
            assert!(t.iter().any(|s| s.name == l.0), "{} labels nothing", l.0);
            assert!(!LABELS[i + 1..].iter().any(|m| m.0 == l.0), "{} is labelled twice", l.0);
        }
        // Rows come grouped, in the order of Group::ALL.
        let groups: Vec<usize> = t.iter().map(|s| s.group as usize).collect();
        assert!(groups.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(row(&t, "app::JustMyWords").defaults, vec!["cmd-shift-."]);
        assert_eq!(row(&t, "editor::MoveLineStart").defaults, vec!["cmd-left", "home", "ctrl-a"]);
        assert!(row(&t, "editor::MoveLeft").context.is_some() && row(&t, "app::NewNote").context.is_none());
    }

    #[std::prelude::v1::test]
    fn the_defaults_do_not_clash() {
        let t = table();
        assert_eq!(clashes(&t, &BTreeMap::new()), HashMap::new());
    }

    #[std::prelude::v1::test]
    fn clashes_are_found_both_ways_and_only_where_both_work() {
        let t = table();
        // A new note on ⌘K: the palette's key.
        let c = clashes(&t, &custom(&[("app::NewNote", &["cmd-k"])]));
        assert_eq!(c["app::NewNote"], vec![Clash { keys: "cmd-k".into(), other: "app::OpenPalette" }]);
        assert_eq!(c["app::OpenPalette"], vec![Clash { keys: "cmd-k".into(), other: "app::NewNote" }]);
        assert_eq!(c.len(), 2);
        // An app shortcut on an editor key clashes while writing.
        let c = clashes(&t, &custom(&[("app::ToggleChat", &["cmd-b"])]));
        assert_eq!(c["editor::ToggleBold"], vec![Clash { keys: "cmd-b".into(), other: "app::ToggleChat" }]);
        // Two editor actions on one key clash; spellings of the same keys are the same keys.
        let c = clashes(&t, &custom(&[("editor::ToggleCode", &["shift-cmd-y"]), ("editor::ToggleBold", &["cmd-shift-y"])]));
        assert_eq!(c["editor::ToggleCode"], vec![Clash { keys: "shift-cmd-y".into(), other: "editor::ToggleBold" }]);
        // A hidden alias clashes as the keys it spells.
        let c = clashes(&t, &custom(&[("app::ToggleChat", &["cmd->"])]));
        assert_eq!(c["app::JustMyWords"], vec![Clash { keys: "cmd-shift-.".into(), other: "app::ToggleChat" }]);
        // A shortcut you emptied clashes with nothing, and its old keys are free.
        let c = clashes(&t, &custom(&[("app::OpenPalette", &[]), ("app::NewNote", &["cmd-k"])]));
        assert!(c.is_empty(), "{c:?}");
    }

    #[std::prelude::v1::test]
    fn custom_keys_replace_the_defaults_and_their_aliases() {
        let t = table();
        let jmw = row(&t, "app::JustMyWords");
        let none = BTreeMap::new();
        assert_eq!(shown(jmw, &none), ["cmd-shift-."]);
        assert_eq!(bound(jmw, &none), vec!["cmd-shift-.", "cmd->"]);
        let mine = custom(&[("app::JustMyWords", &["alt-cmd-h"])]);
        assert_eq!(shown(jmw, &mine), ["alt-cmd-h"]);
        assert_eq!(bound(jmw, &mine), vec!["alt-cmd-h"]);
        let off = custom(&[("app::JustMyWords", &[])]);
        assert!(shown(jmw, &off).is_empty() && bound(jmw, &off).is_empty());
    }

    #[std::prelude::v1::test]
    fn recording_replaces_one_slot_or_adds_and_never_doubles() {
        let cur = vec!["cmd-left".to_string(), "home".to_string(), "ctrl-a".to_string()];
        assert_eq!(with_keys(&cur, Some(1), "alt-left".into()), ["cmd-left", "alt-left", "ctrl-a"]);
        assert_eq!(with_keys(&cur, None, "alt-a".into()), ["cmd-left", "home", "ctrl-a", "alt-a"]);
        // Recording keys the row already has keeps one copy.
        assert_eq!(with_keys(&cur, Some(0), "ctrl-a".into()), ["ctrl-a", "home"]);
        assert_eq!(with_keys(&cur, None, "home".into()), ["cmd-left", "home", "ctrl-a"]);
        assert_eq!(with_keys(&[], Some(3), "cmd-k".into()), ["cmd-k"]);
    }

    #[std::prelude::v1::test]
    fn recording_takes_shortcuts_and_refuses_plain_keys() {
        let k = |s: &str| Keystroke::parse_with_os(s, OperatingSystem::Mac).unwrap();
        assert_eq!(take(&k("cmd-shift-e")), Take::Keys("cmd-shift-e".into()));
        assert_eq!(take(&k("alt-cmd-down")), Take::Keys("alt-cmd-down".into()));
        assert_eq!(take(&k("cmd->")), Take::Keys("cmd->".into()));
        assert_eq!(take(&k("ctrl-a")), Take::Keys("ctrl-a".into()));
        assert_eq!(take(&k("left")), Take::Keys("left".into()));
        assert_eq!(take(&k("backspace")), Take::Keys("backspace".into()));
        assert_eq!(take(&k("f5")), Take::Keys("f5".into()));
        assert!(matches!(take(&k("a")), Take::Refuse(_)));
        assert!(matches!(take(&k("shift-a")), Take::Refuse(_)));
        assert!(matches!(take(&k("space")), Take::Refuse(_)));
        assert!(matches!(take(&k("shift-1")), Take::Refuse(_)));
        assert!(matches!(take(&k("f")), Take::Refuse(_)), "f is a letter, not a function key");
        assert_eq!(take(&k("shift")), Take::Wait);
        assert_eq!(take(&k("cmd")), Take::Wait);
    }

    #[std::prelude::v1::test]
    fn keys_read_like_a_mac_menu() {
        assert_eq!(glyphs("cmd-shift-e"), "⇧⌘E");
        assert_eq!(glyphs("alt-cmd-down"), "⌥⌘↓");
        assert_eq!(glyphs("ctrl-cmd-space"), "⌃⌘Space");
        assert_eq!(glyphs("cmd-shift-."), "⇧⌘.");
        assert_eq!(glyphs("cmd->"), "⌘>");
        assert_eq!(glyphs("cmd-\\"), "⌘\\");
        assert_eq!(glyphs("cmd-enter"), "⌘↩");
        assert_eq!(glyphs("cmd-shift-backspace"), "⇧⌘⌫");
        assert_eq!(glyphs("shift-tab"), "⇧⇥");
        assert_eq!(glyphs("f12"), "F12");
        assert_eq!(glyphs("fn-f1"), "fn F1");
        assert_eq!(glyphs("cmd-k cmd-s"), "⌘K ⌘S");
        assert_eq!(glyphs("not--a-key"), "not--a-key");
    }
}

//! The window: top bar, the notes sidebar, the page, and the overlays
//! (⌘K palette, settings, export).

use gpui_kit::*;
use gpui_kit::prelude::*;

use gpui_kit::base::FocusTrapElement;
use gpui_kit::component::speech::{SpeechEvent, SpeechState};
use std::time::{Duration, Instant};

use crate::export::Format;
use crate::note_page::{NotePage, PageEvent};
use crate::palette::{Palette, PaletteEvent};
use crate::settings::{SettingsEvent, SettingsView, Tab};
use crate::store::{Folder, Note, Store, new_id};
use crate::store::ThreadStatus;
use crate::theme::{SANS, Theme, ts};
use crate::toast::Kind;
use crate::ui::{self, Keyed, icon};

gpui_kit::actions!(
    app,
    [
        Quit,
        ToggleSidebar,
        NewNote,
        OpenPalette,
        ToggleChat,
        OpenSettings,
        JustMyWords,
        CopyNote,
        ExportNote,
        FindLooseThread,
        NextThread,
        PrevThread,
        DismissThread,
        UndoDismiss,
        CloseOverlay,
        ShowWelcome,
        ImportObsidian,
        HideApp,
        HideOthers,
        ShowAll,
        Minimize,
        Zoom,
    ]
);

pub fn init(cx: &mut App) {
    // Every shortcut, the editor's included, with the keys from Settings › Shortcuts.
    crate::shortcuts::install(cx);
    // Tab moves between controls (the window root's own keys). The note keeps Tab for its
    // lists, so ⌃Tab and ⌃⇧Tab move on from anywhere, as in a Mac text view.
    for (keys, action) in [("ctrl-tab", "root::Tab"), ("ctrl-shift-tab", "root::TabPrev")] {
        match cx.build_action(action, None) {
            Ok(a) => cx.bind_keys([KeyBinding::load(keys, a, None, false, None, cx).expect("a valid keystroke")]),
            Err(e) => log::warn!("no {action} to bind {keys} to: {e}"),
        }
    }
    cx.on_action(|_: &Quit, cx| cx.quit());
    // What every Mac app's menus have, with the keys macOS gives them (they stay out of
    // Settings › Shortcuts, as System Settings can rename any menu item's keys).
    cx.bind_keys([KeyBinding::new("cmd-h", HideApp, None), KeyBinding::new("alt-cmd-h", HideOthers, None), KeyBinding::new("cmd-m", Minimize, None)]);
    cx.on_action(|_: &HideApp, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    // the window in front, Settings included
    cx.on_action(|_: &Minimize, cx| {
        if let Some(w) = cx.active_window() {
            let _ = w.update(cx, |_, window, _| window.minimize_window());
        }
    });
    cx.on_action(|_: &Zoom, cx| {
        if let Some(w) = cx.active_window() {
            let _ = w.update(cx, |_, window, _| window.zoom_window());
        }
    });
    cx.set_menus(vec![
        Menu {
            name: "Inkling".into(),
            items: vec![
                MenuItem::action("Settings…", OpenSettings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide Inkling", HideApp),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit Inkling", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Note", NewNote),
                MenuItem::action("Find or Start a Note…", OpenPalette),
                MenuItem::separator(),
                MenuItem::action("Copy This Note", CopyNote),
                MenuItem::action("Export This Note…", ExportNote),
                MenuItem::separator(),
                MenuItem::action("Import from Obsidian…", ImportObsidian),
            ],
            disabled: false,
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Notes", ToggleSidebar),
                MenuItem::action("Chat", ToggleChat),
                MenuItem::action("Hide Threads", JustMyWords),
            ],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Minimize", Minimize), MenuItem::action("Zoom", Zoom)],
            disabled: false,
        },
        Menu {
            name: "Help".into(),
            items: vec![MenuItem::action("Welcome Guide", ShowWelcome)],
            disabled: false,
        },
    ]);
    apply_theme(WindowAppearance::Light, cx);
}

/// Recompute colours and fonts from settings and the system appearance.
pub fn apply_theme(appearance: WindowAppearance, cx: &mut App) {
    let s = cx.global::<Store>().settings.clone();
    let theme = Theme::resolve(s.appearance, appearance, s.ink, s.lettering, s.text_size);
    sync_component_theme(&theme, cx);
    cx.set_global(theme);
}

/// Make the component library's inputs and menus use Inkling's paper and ink.
fn sync_component_theme(theme: &Theme, cx: &mut App) {
    use gpui_kit::component::theme::{Theme as KitTheme, ThemeMode};
    let mode = if theme.p.dark { ThemeMode::Dark } else { ThemeMode::Light };
    KitTheme::change(mode, None, cx);
    let p = theme.p;
    let kit = KitTheme::global_mut(cx);
    kit.font_family = SANS.into();
    kit.font_size = px(14.);
    let c = &mut kit.colors;
    c.background = p.paper;
    c.foreground = p.ink;
    c.caret = p.ink;
    c.selection = p.selection;
    c.border = p.rule;
    c.input = p.rule;
    c.ring = p.faint;
    c.muted_foreground = p.faint;
    c.popover = p.card;
    c.popover_foreground = p.ink;
    c.accent = p.hover;
    c.accent_foreground = p.ink;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportStage {
    Choose,
}

pub struct AppView {
    focus: FocusHandle,
    page: Entity<NotePage>,
    sidebar_open: bool,
    palette: Option<Entity<Palette>>,
    settings: Option<Entity<SettingsView>>,
    export: Option<(ExportStage, Format, bool)>,
    new_folder: Option<Entity<gpui_kit::component::input::InputState>>,
    /// The note deleted last and where it sat in the list: Undo puts it back while its
    /// toast shows.
    deleted: Option<(Note, usize)>,
    /// The blank note made to stand in when the only note was deleted: Undo takes it
    /// away again if nothing was written in it.
    stand_in: Option<String>,
    speech: Entity<SpeechState>,
    talk: Talk,
    talk_timer: Option<Task<()>>,
    /// A dictation session is running or finishing (the badge shows while it is).
    listening: bool,
    partial: String,
    blots: Vec<Blot>,
    next_blot: Instant,
    /// Shared with the page: how present the window's controls are (0..1).
    chrome: std::rc::Rc<std::cell::Cell<f32>>,
    last_pointer: Option<Instant>,
    chrome_frame: Option<Instant>,
    blot_seed: u64,
    // One subscription per overlay, replaced when it opens again, so reopening
    // them all day does not pile up dead subscriptions.
    palette_sub: Option<Subscription>,
    settings_sub: Option<Subscription>,
    folder_sub: Option<Subscription>,
    _subs: Vec<Subscription>,
}

/// How long ⌥ must be held on its own before the microphone starts.
const HOLD_TO_TALK: Duration = Duration::from_millis(380);

/// Hold ⌥ on its own to dictate. Once ⌥ has been held alone for [`HOLD_TO_TALK`]
/// the microphone starts; letting go stops it. A key or another modifier while ⌥
/// is down means ⌥ was a modifier (⌥←, ⌥⌫, ⌥-typed characters, ⌥⌘…), and that
/// hold no longer counts until ⌥ is let go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Talk {
    #[default]
    Idle,
    /// ⌥ alone, waiting out the delay.
    Waiting,
    /// ⌥ was used as a modifier: nothing until it is let go.
    Spoiled,
    /// The microphone is on and ⌥ is held.
    Listening,
}

/// What the view should do after a [`Talk`] transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TalkStep {
    Nothing,
    StartTimer,
    CancelTimer,
    Start,
    Stop,
}

impl Talk {
    /// Modifiers changed. `finishing` is true while the last session still waits
    /// for its final words; a new hold then does not start another.
    fn modifiers(&mut self, m: Modifiers, finishing: bool) -> TalkStep {
        let alone = m.alt && !m.shift && !m.control && !m.platform && !m.function;
        let (next, step) = match (*self, m.alt) {
            (Talk::Listening, false) => (Talk::Idle, TalkStep::Stop),
            (Talk::Waiting, false) => (Talk::Idle, TalkStep::CancelTimer),
            (_, false) => (Talk::Idle, TalkStep::Nothing),
            (Talk::Idle, true) if alone && !finishing => (Talk::Waiting, TalkStep::StartTimer),
            (Talk::Idle, true) => (Talk::Spoiled, TalkStep::Nothing),
            (Talk::Waiting, true) if !alone => (Talk::Spoiled, TalkStep::CancelTimer),
            (state, true) => (state, TalkStep::Nothing),
        };
        *self = next;
        step
    }

    /// Any key other than a bare modifier.
    fn key(&mut self) -> TalkStep {
        if *self == Talk::Waiting {
            *self = Talk::Spoiled;
            return TalkStep::CancelTimer;
        }
        TalkStep::Nothing
    }

    /// The hold delay ran out.
    fn waited(&mut self) -> TalkStep {
        if *self == Talk::Waiting {
            *self = Talk::Listening;
            return TalkStep::Start;
        }
        TalkStep::Nothing
    }

    /// The recognizer ended by itself (silence, an error) while ⌥ may still be down.
    fn ended(&mut self) {
        if *self == Talk::Listening {
            *self = Talk::Spoiled;
        }
    }

    /// The window lost focus: its ⌥ release would never arrive, so stop now.
    fn deactivated(&mut self) -> TalkStep {
        let step = match *self {
            Talk::Listening => TalkStep::Stop,
            Talk::Waiting => TalkStep::CancelTimer,
            _ => TalkStep::Nothing,
        };
        *self = Talk::Idle;
        step
    }
}

/// Ink blots kept at once, and how long one lasts before it has faded.
const MAX_BLOTS: usize = 12;
const BLOT_LIFE: Duration = Duration::from_millis(2800);

/// The file name the save panel suggests for a note: its title, trimmed, without the
/// characters Finder treats as path separators, not hidden (no leading dot), and
/// short enough for the 255-byte name limit with room for the extension.
fn export_file_name(title: &str, ext: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | ':' | '\\' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim_start_matches(|c: char| c == '.' || c.is_whitespace()).trim_end();
    let mut base = if cleaned.is_empty() { "Untitled" } else { cleaned }.to_string();
    if base.len() > 200 {
        let cut = (0..=200).rev().find(|&i| base.is_char_boundary(i)).unwrap_or(0);
        base.truncate(cut);
        base = base.trim_end().to_string();
    }
    format!("{base}.{ext}")
}

/// A tooltip for an icon that has no words of its own: what it does, and its keys as the
/// keymap has them now.
pub(crate) fn tip(name: &'static str, keys: Option<&'static dyn Action>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |window, cx| {
        let t = gpui_kit::component::tooltip::Tooltip::new(name);
        match keys {
            Some(a) => t.action(a, None),
            None => t,
        }
        .build(window, cx)
    }
}

/// Script steps that act on the window itself, as the platform would: keystrokes and
/// the pointer.
fn window_step(step: &crate::script::Step, window: &mut Window, cx: &mut App) {
    use crate::script::Step;
    let none = Modifiers::default();
    let move_to = |position: Point<Pixels>| PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers: none });
    match step {
        Step::Key(keys) => {
            for k in keys {
                match Keystroke::parse_with_os(k, OperatingSystem::Mac) {
                    Ok(ks) => {
                        window.dispatch_keystroke(ks, cx);
                    }
                    Err(e) => log::warn!("script: not a keystroke {k:?}: {e}"),
                }
            }
        }
        Step::Click(x, y) => {
            let position = point(px(*x), px(*y));
            window.dispatch_event(move_to(position), cx);
            let (button, click_count) = (MouseButton::Left, 1);
            window.dispatch_event(PlatformInput::MouseDown(MouseDownEvent { button, position, modifiers: none, click_count, first_mouse: false }), cx);
            window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button, position, modifiers: none, click_count }), cx);
        }
        Step::Pointer(x, y) => {
            window.dispatch_event(move_to(point(px(*x), px(*y))), cx);
        }
        _ => {}
    }
}

/// The size a script last asked the window for. The jiggle nudges around it, not around
/// the size the window has now: that lags a resize still landing, and nudging from it
/// undid the resize.
static ASKED: std::sync::Mutex<Option<(f32, f32)>> = std::sync::Mutex::new(None);

/// Resize the window for a script step (`size:`, `width:`).
pub(crate) fn script_resize(window: &mut Window, w: f32, h: f32) {
    *ASKED.lock().unwrap_or_else(|e| e.into_inner()) = Some((w, h));
    window.resize(size(px(w), px(h)));
}

/// Folder names match ignoring case, in any script ("été" is "ÉTÉ").
fn same_folder_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// A drop of ink left by the pointer ("Ink from the pointer").
struct Blot {
    at: Point<Pixels>,
    born: Instant,
    seed: u64,
}

/// Keep at most [`MAX_BLOTS`], dropping the oldest.
fn add_blot(blots: &mut Vec<Blot>, blot: Blot) {
    blots.push(blot);
    if blots.len() > MAX_BLOTS {
        let extra = blots.len() - MAX_BLOTS;
        blots.drain(..extra);
    }
}

/// Forget the blots that have dried and faded by `now`.
fn prune_blots(blots: &mut Vec<Blot>, now: Instant) {
    blots.retain(|b| now.saturating_duration_since(b.born) < BLOT_LIFE);
}

fn rand(seed: &mut u64) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed % 10_000) as f32 / 10_000.
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        apply_theme(window.appearance(), cx);
        let note = {
            let store = cx.global::<Store>();
            let last = store.settings.last_note.clone();
            last.and_then(|id| store.note(&id).cloned())
                .or_else(|| store.notes.first().cloned())
                .unwrap_or_else(|| Note::new("", store.settings.default_mode, None))
        };
        let page = cx.new(|cx| NotePage::new(note, window, cx));
        let chrome = page.read(cx).chrome.clone();
        let mut subs = vec![];
        subs.push(cx.subscribe_in(&page, window, Self::on_page_event));
        subs.push(cx.observe_window_appearance(window, |_, window, cx| {
            apply_theme(window.appearance(), cx);
            cx.refresh_windows();
        }));
        page.update(cx, |p, cx| p.focus_editor_at_end(window, cx));
        // Edits wait 600 ms before they are written; quitting or closing the window
        // in that time must not lose them.
        subs.push(cx.on_app_quit(|this, cx| {
            this.page.update(cx, |p, cx| p.flush(cx));
            async {}
        }));
        let weak_page = page.downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            let _ = weak_page.update(cx, |p, cx| p.flush(cx));
            true
        });
        subs.push(cx.on_focus_lost(window, |this, window, cx| this.restore_focus(window, cx)));
        subs.push(cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                let step = this.talk.deactivated();
                this.talk_step(step, window, cx);
            }
        }));
        // Every keystroke, before any binding or input sees it: a key while ⌥ is down
        // means ⌥ was a modifier (bindings such as ⌥← consume the key, so a
        // key-down listener would miss it), and Escape closes the overlays this
        // view owns before the editor can take it.
        let view = cx.entity().downgrade();
        subs.push(cx.intercept_keystrokes(move |ev, window, cx| {
            let _ = view.update(cx, |this, cx| this.on_keystroke(&ev.keystroke, window, cx));
        }));
        let speech = cx.new(SpeechState::new);
        subs.push(cx.subscribe_in(&speech, window, |this, _, ev: &SpeechEvent, window, cx| match ev {
            SpeechEvent::Partial(t) => {
                this.partial = t.to_string();
                cx.notify();
            }
            SpeechEvent::Final(t) => {
                let text = t.trim().to_string();
                this.partial.clear();
                this.listening = false;
                this.talk.ended();
                if !text.is_empty() {
                    this.page.update(cx, |p, cx| {
                        p.editor.update(cx, |e, cx| {
                            e.focus(window, cx);
                            e.insert_at_cursor(&text, cx);
                        })
                    });
                }
                cx.notify();
            }
            SpeechEvent::Cancelled => {
                this.listening = false;
                this.talk.ended();
                this.partial.clear();
                cx.notify();
            }
            SpeechEvent::Error(e) => {
                this.listening = false;
                this.talk.ended();
                this.partial.clear();
                crate::toast::show(cx, Kind::Problem, format!("Couldn’t listen: {e}"));
            }
            _ => {}
        }));
        let mut this = Self {
            focus: cx.focus_handle(),
            page,
            sidebar_open: false,
            chrome,
            last_pointer: None,
            chrome_frame: None,
            palette: None,
            settings: None,
            export: None,
            new_folder: None,
            deleted: None,
            stand_in: None,
            speech,
            talk: Talk::Idle,
            talk_timer: None,
            listening: false,
            partial: String::new(),
            blots: vec![],
            next_blot: Instant::now(),
            blot_seed: 0x9e37_79b9_7f4a_7c15,
            palette_sub: None,
            settings_sub: None,
            folder_sub: None,
            _subs: subs,
        };
        this.apply_launch_hooks(window, cx);
        this.run_script(window, cx);
        this.jiggle(window, cx);
        this
    }

    /// Debug aid: macOS stops drawing a window that is fully covered, so a
    /// background screenshot shows a stale frame. INKLING_JIGGLE=<ms> nudges the
    /// window by a pixel every <ms>, which makes AppKit ask for a fresh frame.
    fn jiggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ms) = std::env::var("INKLING_JIGGLE").ok().and_then(|v| v.parse::<u64>().ok()) else { return };
        // At least a frame apart: 0 would resize the window in a busy loop.
        let ms = ms.max(16);
        cx.spawn_in(window, async move |_, cx| {
            let mut grow = true;
            loop {
                cx.background_executor().timer(Duration::from_millis(ms)).await;
                let ok = cx.update(|window, _| {
                    let s = window.viewport_size();
                    let asked = *ASKED.lock().unwrap_or_else(|e| e.into_inner());
                    let (w, h) = asked.map_or((s.width, s.height), |(w, h)| (px(w), px(h)));
                    let d = if grow { px(1.) } else { px(-1.) };
                    window.resize(size(w + d, h));
                });
                if ok.is_err() {
                    break;
                }
                grow = !grow;
            }
        })
        .detach();
    }

    fn run_script(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(path) = std::env::var("INKLING_SCRIPT") else { return };
        let Ok(src) = std::fs::read_to_string(&path) else {
            log::error!("script: cannot read {path}");
            return;
        };
        let steps = crate::script::parse(&src);
        cx.spawn_in(window, async move |this, cx| {
            use crate::script::Step;
            cx.background_executor().timer(std::time::Duration::from_millis(1200)).await;
            for step in steps {
                log::info!("script: {step:?}");
                let ok = match step {
                    Step::Wait(ms) => {
                        cx.background_executor().timer(std::time::Duration::from_millis(ms)).await;
                        continue;
                    }
                    Step::Settle(max) => {
                        let started = std::time::Instant::now();
                        loop {
                            cx.background_executor().timer(std::time::Duration::from_millis(50)).await;
                            let Ok(settled) = this.update_in(cx, |app, _, cx| app.page.read(cx).settled() && !crate::import::importing()) else { return };
                            if settled {
                                log::info!("script: settled after {} ms", started.elapsed().as_millis());
                                break;
                            }
                            if started.elapsed() >= std::time::Duration::from_millis(max) {
                                log::warn!("script: settle gave up after {max} ms");
                                break;
                            }
                        }
                        continue;
                    }
                    // Input goes to the window as the platform would send it, outside any
                    // view's update, so the views it reaches can update themselves.
                    Step::Key(_) | Step::Click(..) | Step::Pointer(..) => {
                        cx.update(|window, cx| window_step(&step, window, cx)).is_ok()
                    }
                    _ => this.update_in(cx, |app, window, cx| app.script_step(step, window, cx)).is_ok(),
                };
                if !ok {
                    break;
                }
                cx.background_executor().timer(std::time::Duration::from_millis(250)).await;
            }
        })
        .detach();
    }

    fn script_step(&mut self, step: crate::script::Step, window: &mut Window, cx: &mut Context<Self>) {
        use crate::script::Step;
        match &step {
            Step::Note(title) => self.create_note(title, None, window, cx),
            Step::Open(what) => match what.as_str() {
                "sidebar" => self.sidebar_open = true,
                "palette" => self.open_palette(window, cx),
                "settings" => self.open_settings(Tab::Customize, window, cx),
                "minds" => self.open_settings(Tab::Minds, window, cx),
                "newmind" => self.open_settings(Tab::NewMind, window, cx),
                "model" => self.open_settings(Tab::Model, window, cx),
                "about" => self.open_settings(Tab::AboutYou, window, cx),
                "export" => self.open_export(window, cx),
                "jmw" => self.toggle_jmw(cx),
                "welcome" => self.show_welcome(window, cx),
                _ => {}
            },
            Step::Close => {
                self.sidebar_open = false;
                self.palette = None;
                self.settings = None;
                self.export = None;
            }
            Step::Quit => cx.quit(),
            Step::Import(dir) => self.import_vault(crate::script::path(dir), cx),
            Step::Size(w, h) => script_resize(window, *w, *h),
            Step::Set(..) => {
                self.page.update(cx, |p, cx| p.script_step(&step, window, cx));
                // Appearance, text size and threads change the theme.
                apply_theme(window.appearance(), cx);
                cx.refresh_windows();
            }
            _ => self.page.update(cx, |p, cx| p.script_step(&step, window, cx)),
        }
        cx.notify();
    }

    /// INKLING_OPEN=sidebar|palette|settings|chat|minds|model opens a panel at
    /// launch (handy for screenshots and demos).
    fn apply_launch_hooks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(what) = std::env::var("INKLING_OPEN") else { return };
        for part in what.split(',') {
            match part.trim() {
                "sidebar" => self.sidebar_open = true,
                "palette" => self.open_palette(window, cx),
                "settings" => self.open_settings(Tab::Customize, window, cx),
                "minds" => self.open_settings(Tab::Minds, window, cx),
                "newmind" => self.open_settings(Tab::NewMind, window, cx),
                "model" => self.open_settings(Tab::Model, window, cx),
                "about" => self.open_settings(Tab::AboutYou, window, cx),
                "chat" => self.page.update(cx, |p, cx| p.toggle_chat(window, cx)),
                "export" => self.open_export(window, cx),
                _ => {}
            }
        }
    }

    /// Hold ⌥ (alone) to dictate; see [`Talk`].
    fn on_modifiers(&mut self, ev: &ModifiersChangedEvent, window: &mut Window, cx: &mut Context<Self>) {
        let step = self.talk.modifiers(ev.modifiers, self.listening);
        self.talk_step(step, window, cx);
    }

    fn talk_step(&mut self, step: TalkStep, window: &mut Window, cx: &mut Context<Self>) {
        match step {
            TalkStep::Nothing => {}
            TalkStep::StartTimer => {
                self.talk_timer = Some(cx.spawn_in(window, async move |this, cx| {
                    cx.background_executor().timer(HOLD_TO_TALK).await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        this.talk_timer = None;
                        let step = this.talk.waited();
                        this.talk_step(step, window, cx);
                    });
                }));
            }
            TalkStep::CancelTimer => self.talk_timer = None,
            TalkStep::Start => {
                self.listening = true;
                self.partial.clear();
                self.speech.update(cx, |s, cx| s.start(cx));
                cx.notify();
            }
            TalkStep::Stop => {
                self.talk_timer = None;
                self.speech.update(cx, |s, cx| s.stop(cx));
            }
        }
    }

    fn on_keystroke(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) {
        if !keystroke.is_modifier_key() {
            let step = self.talk.key();
            self.talk_step(step, window, cx);
        }
        if keystroke.key == "escape" && keystroke.modifiers == Modifiers::default() && self.escape(window, cx) {
            cx.stop_propagation();
        }
        // Tab in an open header menu closes it, and moves on from its button.
        if keystroke.key == "tab" && self.page.read(cx).menu_has_keyboard(window) {
            self.page.update(cx, |p, cx| p.close_menu(window, cx));
        }
        // ⌘⌫ in ⌘K deletes the highlighted note (with Undo). Caught here, before the
        // search box's own ⌘⌫ binding takes it and the shortcut silently does nothing.
        let cmd_only = Modifiers { platform: true, ..Modifiers::default() };
        if keystroke.key == "backspace" && keystroke.modifiers == cmd_only {
            if let Some(palette) = self.palette.clone() {
                if palette.update(cx, |p, cx| p.delete_selected(cx)) {
                    cx.stop_propagation();
                }
            }
        }
    }

    /// Escape closes the topmost thing that is open, one per press: this view's
    /// overlays first (the palette and settings handle their own, being focused), then
    /// whatever the page has open, wherever the keyboard is: a menu, the question box,
    /// the open thread from its reply box, the chat from its box. Returns whether it
    /// closed something.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.export.is_some() {
            self.close_export(window, cx);
        } else if self.palette.is_some() || self.settings.is_some() {
            return false;
        } else if self.sidebar_open && self.new_folder.is_some() {
            self.finish_new_folder(false, window, cx);
        } else if self.sidebar_open {
            self.sidebar_open = false;
            cx.notify();
        } else {
            return self.page.update(cx, |p, cx| p.escape(window, cx));
        }
        true
    }

    /// Nothing in the window has the keyboard any more (the box that had it went away:
    /// a thread closed from the keyboard, a question box that was sent): give it to the
    /// topmost thing still open, else the note, so typing and every shortcut keep
    /// working. Without this the window's shortcuts go deaf until the next click.
    fn restore_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(settings) = &self.settings {
            window.focus(&settings.focus_handle(cx), cx);
        } else if let Some(palette) = self.palette.clone() {
            palette.update(cx, |p, cx| p.focus(window, cx));
        } else if self.export.is_some() {
            window.focus(&self.focus, cx);
        } else {
            self.page.update(cx, |p, cx| p.restore_focus(window, cx));
        }
    }

    fn on_pointer(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        // moving the pointer brings back the controls that typing faded
        self.last_pointer = Some(Instant::now());
        if self.chrome.get() < 1. {
            cx.notify();
        }
        if !cx.global::<Store>().settings.ink_from_pointer || ev.pressed_button.is_some() {
            return;
        }
        let now = Instant::now();
        if now < self.next_blot {
            return;
        }
        let gap = 1.6 + 3.2 * rand(&mut self.blot_seed);
        self.next_blot = now + Duration::from_secs_f32(gap);
        let seed = (self.blot_seed ^ 0xa5a5) | 1;
        add_blot(&mut self.blots, Blot { at: ev.position, born: now, seed });
        cx.notify();
    }

    fn blot_layer(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        prune_blots(&mut self.blots, Instant::now());
        if self.blots.is_empty() {
            return None;
        }
        let ink = Theme::get(cx).p.ink;
        let drops: Vec<(Point<Pixels>, f32, u64)> = self
            .blots
            .iter()
            .map(|b| (b.at, b.born.elapsed().as_secs_f32(), b.seed))
            .collect();
        Some(
            canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    for (at, age, seed) in &drops {
                        let mut s = *seed;
                        // Wet for a moment, then it dries lighter and fades.
                        let alpha = if *age < 0.35 { 0.85 } else { (0.85 - (age - 0.35) * 0.36).max(0.) };
                        let grow = (age * 6.).min(1.);
                        let r = (2.4 + 2.6 * rand(&mut s)) * (0.6 + 0.4 * grow);
                        let mut dots = vec![(0f32, 0f32, r)];
                        for _ in 0..(2 + (rand(&mut s) * 4.) as usize) {
                            let a = rand(&mut s) * std::f32::consts::TAU;
                            let d = r + 2. + rand(&mut s) * 7. * grow;
                            dots.push((a.cos() * d, a.sin() * d, 0.6 + rand(&mut s) * 1.4));
                        }
                        for (dx, dy, rr) in dots {
                            let c = point(at.x + px(dx), at.y + px(dy));
                            let b = Bounds::new(c - point(px(rr), px(rr)), size(px(rr * 2.), px(rr * 2.)));
                            window.paint_quad(fill(b, ink.opacity(alpha)).corner_radii(px(rr)));
                        }
                    }
                    window.request_animation_frame();
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .into_any_element(),
        )
    }

    fn listening_badge(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let chat_open = self.page.read(cx).chat_open;
        // The corner hint is for a quiet page; once threads hang in the margin it would
        // sit on top of their tags, so it steps aside.
        let threads_out = self.page.read(cx).note.threads.iter().any(|t| t.status == ThreadStatus::Open);
        // Nor does it belong on top of a dialog or the drawer, or beside a toast, which
        // sits on the same line along the bottom (and runs into it in a narrow window).
        let covered = self.sidebar_open || self.palette.is_some() || self.settings.is_some() || self.export.is_some() || crate::toast::any(cx);
        if !self.listening && (chat_open || threads_out || covered) {
            return div().into_any_element();
        }
        if !self.listening {
            // on the paper, in the sheet's bottom corner: clear of its edges and the weave
            let right = self.page.read(cx).sheet_inner_right();
            return div()
                .absolute()
                .bottom(px(24.))
                .right(px(right))
                .text_size(px(ts::META))
                .font_family(SANS)
                .text_color(p.faint)
                .child({
                    let keys = |name: &str| crate::shortcuts::keys_for(name, cx);
                    let mut parts = vec!["Hold \u{2325} to speak".to_string()];
                    parts.extend(keys("app::ToggleChat").map(|k| format!("{k} chat")));
                    parts.extend(keys("app::OpenPalette").map(|k| format!("{k} notes")));
                    parts.join(" \u{b7} ")
                })
                .into_any_element();
        }
        let partial = if self.partial.is_empty() { "Listening…".to_string() } else { self.partial.clone() };
        // where the corner hint sits, on the paper, not across the sheet's corner and the
        // weave; beside the chat when it is open
        let right = self.page.read(cx).sheet_inner_right() + if chat_open { crate::note_page::CHAT_WIDTH } else { 0. };
        div()
            .absolute()
            .bottom(px(24.))
            .right(px(right))
            .max_w(px(360.))
            .px(px(12.))
            .py(px(8.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.rule)
            // lifted off the page as the toasts and menus are
            .shadow(ui::popover_shadow(&p))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(ui::small_dot(t.voice_ink, 7.).with_animation(
                "listening",
                Animation::new(Duration::from_millis(900)).repeat(),
                move |d, delta| d.opacity(0.35 + 0.65 * (0.5 + 0.5 * (delta * std::f32::consts::TAU).sin())),
            ))
            .child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child(partial))
            .into_any_element()
    }

    fn on_page_event(&mut self, _: &Entity<NotePage>, ev: &PageEvent, window: &mut Window, cx: &mut Context<Self>) {
        match ev {
            PageEvent::NotesChanged => cx.notify(),
            PageEvent::OpenNote(id) => self.open_note(id.clone(), window, cx),
            PageEvent::OpenSettings(tab) => self.open_settings(*tab, window, cx),
            PageEvent::Learned => self.toast("Inkling noted something about you (Settings › About you)", cx),
            PageEvent::Toast(text) => self.toast(text, cx),
        }
    }

    fn open_note(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |p, cx| p.flush(cx));
        let Some(note) = cx.global::<Store>().note(&id).cloned() else { return };
        self.page.update(cx, |p, cx| p.open(note, window, cx));
        // Saving on the way out records the note being left; the next launch
        // should open this one.
        let store = cx.global_mut::<Store>();
        store.settings.last_note = Some(id);
        store.save_settings();
        self.sidebar_open = false;
        self.palette = None;
        cx.notify();
    }

    fn create_note(&mut self, title: &str, folder_name: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |p, cx| p.flush(cx));
        let folder = folder_name.map(|name| {
            let store = cx.global_mut::<Store>();
            if let Some(f) = store.folders.iter().find(|f| same_folder_name(&f.name, &name)) {
                f.id.clone()
            } else {
                let f = Folder { id: new_id(), name, collapsed: false };
                let id = f.id.clone();
                store.folders.push(f);
                store.save_folders();
                id
            }
        });
        let mode = cx.global::<Store>().settings.default_mode;
        let note = Note::new(title, mode, folder);
        let id = note.id.clone();
        {
            let store = cx.global_mut::<Store>();
            store.save_note(&note);
            store.notes.insert(0, note);
        }
        self.open_note(id, window, cx);
    }

    /// Help > Welcome Guide: the "Start here" note, or a fresh one if it was deleted.
    fn show_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |p, cx| p.flush(cx));
        let existing = cx.global::<Store>().notes.iter().find(|n| n.title == "Start here").map(|n| n.id.clone());
        let id = existing.unwrap_or_else(|| {
            let mode = cx.global::<Store>().settings.default_mode;
            let note = crate::welcome::first_run_note(mode);
            let id = note.id.clone();
            let store = cx.global_mut::<Store>();
            store.save_note(&note);
            store.notes.insert(0, note);
            id
        });
        self.open_note(id, window, cx);
    }

    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = Some(self.page.read(cx).note_id().to_string());
        let palette = cx.new(|cx| Palette::new(current, window, cx));
        let sub = cx.subscribe_in(&palette, window, |this, _, ev: &PaletteEvent, window, cx| match ev {
            PaletteEvent::Open(id) => this.open_note(id.clone(), window, cx),
            PaletteEvent::Create { title, folder } => {
                let (t, f) = (title.clone(), folder.clone());
                this.palette = None;
                this.create_note(&t, f, window, cx);
            }
            PaletteEvent::Delete(id) => this.delete_note(id.clone(), window, cx),
            PaletteEvent::Dismiss => {
                this.palette = None;
                this.focus_editor(window, cx);
                cx.notify();
            }
        });
        self.palette_sub = Some(sub);
        self.palette = Some(palette);
        cx.notify();
    }

    /// Back to the note, the caret where it was.
    fn focus_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |p, cx| p.editor.update(cx, |e, cx| e.focus(window, cx)));
    }

    fn delete_note(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        // Move the page off the note first: switching notes saves the one being
        // left, so deleting before the switch would write it straight back.
        self.stand_in = None;
        if self.page.read(cx).note_id() == id {
            let next = cx.global::<Store>().notes.iter().find(|n| n.id != id).map(|n| n.id.clone());
            match next {
                Some(n) => self.open_note(n, window, cx),
                None => {
                    self.create_note("", None, window, cx);
                    self.stand_in = Some(self.page.read(cx).note_id().to_string());
                }
            }
        }
        // ⌘⌫ in the palette is one keystroke away from clearing the search box, so a
        // deleted note can come back for a few seconds.
        let kept = {
            let store = cx.global::<Store>();
            store.notes.iter().position(|n| n.id == id).map(|at| (store.notes[at].clone(), at))
        };
        cx.global_mut::<Store>().delete_note(&id);
        self.deleted = kept;
        crate::toast::show(cx, Kind::NoteDeleted, "Note deleted");
        cx.notify();
    }

    /// Put the note deleted last back where it was, and open it.
    fn undo_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((note, at)) = self.deleted.take() else { return };
        if !crate::toast::live(cx, Kind::NoteDeleted) {
            return;
        }
        let id = note.id.clone();
        {
            let store = cx.global_mut::<Store>();
            store.save_note(&note);
            let at = at.min(store.notes.len());
            store.notes.insert(at, note);
        }
        crate::toast::close(cx, Kind::NoteDeleted);
        self.palette = None;
        self.open_note(id, window, cx);
        // the blank stand-in goes too, unless something was written in it
        if let Some(blank) = self.stand_in.take() {
            let store = cx.global_mut::<Store>();
            let untouched = store.note(&blank).is_some_and(|n| n.blocks.iter().all(|b| b.text.trim().is_empty()) && n.threads.is_empty() && n.chat.is_empty());
            if untouched {
                store.delete_note(&blank);
            }
            cx.notify();
        }
    }

    /// Settings is a window of its own (see settings.rs); the page follows what changes there.
    fn open_settings(&mut self, tab: Tab, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view): Option<Entity<SettingsView>> = crate::settings::open_window(tab, cx) else { return };
        let sub = cx.subscribe_in(&view, window, |this, _, ev: &SettingsEvent, window, cx| match ev {
            SettingsEvent::Changed => {
                apply_theme(window.appearance(), cx);
                this.page.update(cx, |_, cx| cx.notify());
                cx.refresh_windows();
            }
            SettingsEvent::MindsChanged => this.page.update(cx, |p, cx| p.refresh_mind(cx)),
            SettingsEvent::Close => {
                this.settings = None;
                this.focus_editor(window, cx);
                cx.notify();
            }
        });
        self.settings_sub = Some(sub);
        self.sidebar_open = false;
        cx.notify();
    }

    /// Something done, said in passing (the toasts are one stack: see toast.rs).
    fn toast(&mut self, text: &str, cx: &mut Context<Self>) {
        crate::toast::show(cx, Kind::Done, text);
    }

    /// File > Import from Obsidian…: pick a vault folder and bring its notes in.
    fn choose_vault(&mut self, cx: &mut Context<Self>) {
        let options = PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Import".into()) };
        let rx = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(vault) = paths.into_iter().next() else { return };
            let _ = this.update(cx, |this, cx| this.import_vault(vault, cx));
        })
        .detach();
    }

    fn import_vault(&mut self, vault: std::path::PathBuf, cx: &mut Context<Self>) {
        let import = crate::import::import(vault, cx);
        cx.spawn(async move |this, cx| {
            let message = import.await;
            let _ = this.update(cx, |this, cx| this.toast(&message, cx));
        })
        .detach();
    }

    /// The export dialog takes the keyboard (its card tracks this view's focus), so typing
    /// does not go into the note behind it.
    fn open_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.export = Some((ExportStage::Choose, Format::Markdown, true));
        self.sidebar_open = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn close_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.export = None;
        self.focus_editor(window, cx);
        cx.notify();
    }

    fn run_export(&mut self, format: Format, with_threads: bool, window: &mut Window, cx: &mut Context<Self>) {
        let note = self.page.update(cx, |p, cx| p.note_snapshot(cx));
        let dir = dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
        let name = export_file_name(&note.title, format.ext());
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        self.close_export(window, cx);
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            // Off the main thread: Word export waits for textutil.
            let result = cx
                .background_executor()
                .spawn({
                    let path = path.clone();
                    async move { crate::export::write(&note, with_threads, format, &path) }
                })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => this.toast(&format!("Saved {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("note")), cx),
                Err(e) => crate::toast::show(cx, Kind::Problem, format!("Export failed: {e:#}")),
            });
        })
        .detach();
        cx.notify();
    }

    // ----- rendering ---------------------------------------------------------

    /// The window has no bar. The traffic lights, the sidebar toggle and Chat sit on the
    /// cloth above the sheet, over a strip that drags the window. Typing fades them;
    /// moving the pointer brings them back.
    fn chrome_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        // ("Show threads" lives in the page's own header row, not up here)
        let (typed, chat_open) = {
            let page = self.page.read(cx);
            (page.last_typed(), page.chat_open)
        };
        let calm = typed.is_some_and(|k| self.last_pointer.is_none_or(|m| m < k)) && !self.sidebar_open;
        let target = if calm { 0. } else { 1. };
        let now = Instant::now();
        let dt = self.chrome_frame.map(|l| (now - l).as_secs_f32()).unwrap_or(1. / 60.).clamp(1. / 240., 1. / 30.);
        self.chrome_frame = Some(now);
        let a = self.chrome.get();
        // out slowly, so a first keystroke does not flash the corners; back quickly
        let rate = if target < a { 5. } else { 14. };
        let mut next = a + (target - a) * (1. - (-dt * rate).exp());
        if (next - target).abs() < 0.003 {
            next = target;
        } else {
            window.request_animation_frame();
        }
        self.chrome.set(next);

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(crate::note_page::CHROME_H))
            .window_control_area(WindowControlArea::Drag)
            .opacity(next)
            .when(next > 0.02, |d| {
                d.child(
                    div()
                        .id("toggle-sidebar")
                        .absolute()
                        // centred on the traffic lights' line, as Chat is
                        .top(px(11.))
                        .left(px(76.))
                        .p(px(4.))
                        .rounded(px(5.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .tooltip(tip("Notes", Some(&ToggleSidebar)))
                        .child(icon("menu", 14., p.graphite))
                        // first in the Tab order (no part of the window comes before it)
                        .keyed_as("chrome:sidebar", 5., t.voice_ink, cx, |this, _, cx| {
                            this.sidebar_open = !this.sidebar_open;
                            cx.notify();
                        }),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(8.))
                        .right(px(14.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        // Chat comes with the chat in the Tab order
                        .tab_group()
                        .tab_index(ui::tab::CHAT)
                        // an open chat has its own close button in the same corner
                        .when(!chat_open, |d| {
                            d.child(
                                div()
                                    .id("chat-button")
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .px(px(10.))
                                    .py(px(3.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(p.rule)
                                    .bg(p.paper.opacity(0.6))
                                    .text_size(px(ts::SMALL))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .hover(move |s| s.border_color(p.faint))
                                    .tooltip(tip("Chat about this note", Some(&ToggleChat)))
                                    .child(ui::small_dot(t.voice_ink, 5.))
                                    .child("Chat")
                                    .keyed_as("chrome:chat", 12., t.voice_ink, cx, |this, window, cx| {
                                        this.page.update(cx, |p, cx| p.toggle_chat(window, cx));
                                        cx.notify();
                                    }),
                            )
                        }),
                )
            })
            .into_any_element()
    }

    fn sidebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let current = self.page.read(cx).note_id().to_string();
        let hidden = self.page.read(cx).just_my_words;
        let store = cx.global::<Store>();
        let notes = store.notes.clone();
        let folders = store.folders.clone();
        let brain = crate::brain::Brain::global(cx).read(cx);
        let who = brain
            .account
            .as_ref()
            .and_then(|a| a.email.clone())
            .unwrap_or_else(|| "This Mac".into());

        // One look for every row: the open note sits on the thread's wash, a row under
        // the pointer on a lighter one, so the two never look the same.
        let row = move |id: SharedString, on: bool| {
            div()
                .id(id)
                .mx(px(6.))
                .px(px(8.))
                .py(px(5.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(ts::UI))
                .cursor_pointer()
                .when(on, |d| d.bg(p.hover).font_weight(FontWeight::MEDIUM))
                .when(!on, |d| d.hover(move |s| s.bg(p.hover.opacity(0.5))))
                .active(|s| s.opacity(0.8))
        };
        let ring = t.voice_ink;
        let note_row = |n: &Note, indent: f32, cx: &mut Context<Self>| {
            let id = n.id.clone();
            let on = n.id == current;
            let open = n.open_threads();
            row(SharedString::from(format!("note-{}", n.id)), on)
                .pl(px(8. + indent))
                .child(icon("file-text", 13., p.faint))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .truncate()
                        .text_color(p.ink)
                        .child(if n.title.trim().is_empty() { "Untitled".to_string() } else { n.title.clone() }),
                )
                // open threads: the note's knot, with their count
                .when(open > 0, |d| {
                    d.child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(ts::META))
                            .text_color(p.faint)
                            .child(ui::small_dot(t.voice_ink.opacity(0.8), 5.))
                            .child(open.to_string()),
                    )
                })
                .keyed_as(format!("sidebar:note-{}", n.id), 6., ring, cx, move |this, window, cx| this.open_note(id.clone(), window, cx))
        };

        let mut list = div().flex().flex_col().gap(px(1.));
        for f in &folders {
            let fid = f.id.clone();
            let collapsed = f.collapsed;
            let inside: Vec<&Note> = notes.iter().filter(|n| n.folder.as_deref() == Some(f.id.as_str())).collect();
            let group = SharedString::from(format!("folder-row-{}", f.id));
            list = list.child(
                row(SharedString::from(format!("folder-{}", f.id)), false)
                    .group(group.clone())
                    .text_color(p.graphite)
                    .child(icon(if collapsed { "folder" } else { "folder-open" }, 13., p.faint))
                    .child(div().flex_1().min_w(px(0.)).truncate().child(f.name.clone()))
                    // a folded folder's count lines up with the notes' counts; under the
                    // pointer the remove × takes its place
                    .when(collapsed && !inside.is_empty(), |d| d.child(ui::key_hint(inside.len().to_string(), &t).group_hover(group.clone(), |s| s.invisible())))
                    .child({
                        // Remove the folder; its notes stay, at the top level.
                        let fid = fid.clone();
                        div()
                            .id(SharedString::from(format!("folder-remove-{}", f.id)))
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .right(px(6.))
                            .flex()
                            .items_center()
                            .invisible()
                            .group_hover(group, |s| s.visible())
                            .child(div().p(px(2.)).rounded(px(4.)).hover(move |s| s.bg(p.hover)).child(icon("x", 11., p.faint)))
                            .tooltip(|window, cx| gpui_kit::component::tooltip::Tooltip::new("Remove folder (its notes stay)").build(window, cx))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                cx.global_mut::<Store>().delete_folder(&fid);
                                // The open note may have been in it: it is at the top level now.
                                this.page.update(cx, |page, _| {
                                    if page.note.folder.as_deref() == Some(fid.as_str()) {
                                        page.note.folder = None;
                                    }
                                });
                                this.toast("Folder removed; its notes are still here", cx);
                            }))
                    })
                    .keyed_as(format!("sidebar:folder-{}", f.id), 6., ring, cx, move |_, _, cx| {
                        let store = cx.global_mut::<Store>();
                        if let Some(f) = store.folders.iter_mut().find(|f| f.id == fid) {
                            f.collapsed = !f.collapsed;
                        }
                        store.save_folders();
                        cx.notify();
                    }),
            );
            if !collapsed {
                if inside.is_empty() {
                    list = list.child(div().pl(px(49.)).py(px(3.)).text_size(px(ts::META)).text_color(p.faint).child("No notes in here yet"));
                }
                for n in inside {
                    list = list.child(note_row(n, 14., cx));
                }
            }
        }
        for n in notes.iter().filter(|n| n.folder.as_ref().is_none_or(|f| !folders.iter().any(|x| &x.id == f))) {
            list = list.child(note_row(n, 0., cx));
        }

        // the keys each row shows are the ones bound now (Settings > Shortcuts)
        let keys_of = |name: &str| crate::shortcuts::keys_for(name, cx).unwrap_or_default();
        let (k_copy, k_export, k_hide, k_settings, k_find, k_new) = (
            keys_of("app::CopyNote"),
            keys_of("app::ExportNote"),
            keys_of("app::JustMyWords"),
            keys_of("app::OpenSettings"),
            keys_of("app::OpenPalette"),
            keys_of("app::NewNote"),
        );
        let action = |id: &'static str, label: &'static str, keys: String| {
            div()
                .id(id)
                .mx(px(6.))
                .px(px(8.))
                .py(px(4.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .text_size(px(ts::SMALL))
                .text_color(p.graphite)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover.opacity(0.5)).text_color(p.ink))
                .active(|s| s.opacity(0.8))
                .child(div().flex_1().child(label))
                .child(ui::key_hint(keys, &t))
        };

        let new_folder = self.new_folder.clone();
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_row()
            .tab_group()
            .tab_index(ui::tab::SIDEBAR)
            .child(
                div()
                    .id("sidebar")
                    .w(px(240.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(p.shelf)
                    .font_family(SANS)
                    // the drawer lies on top of the page
                    .shadow(vec![BoxShadow { color: p.shadow, offset: point(px(2.), px(0.)), blur_radius: px(14.), spread_radius: px(0.), inset: false }])
                    .border_r_1()
                    .border_color(p.rule)
                    .child(
                        div()
                            .h(px(46.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .pl(px(86.))
                            .pr(px(10.))
                            .child(ui::wordmark(&t, 18.))
                            // 1 px above the row's middle: on the traffic lights' line
                            .child(ui::icon_button("sidebar-close", "x", 13., &t).mb(px(2.)).tooltip(tip("Close", Some(&ToggleSidebar))).keyed_as("sidebar:close", 5., ring, cx, |this, _, cx| {
                                this.sidebar_open = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        row("sidebar-search".into(), false)
                            .text_color(p.graphite)
                            .child(icon("search", 13., p.faint))
                            .child(div().flex_1().child("Find or start a note"))
                            .child(ui::key_hint(k_find.clone(), &t))
                            .keyed_as("sidebar:search", 6., ring, cx, |this, window, cx| this.open_palette(window, cx)),
                    )
                    .child(
                        row("sidebar-new".into(), false)
                            .text_color(p.graphite)
                            .child(icon("plus", 13., p.faint))
                            .child(div().flex_1().child("New note"))
                            .child(ui::key_hint(k_new, &t))
                            .keyed_as("sidebar:new", 6., ring, cx, |this, window, cx| this.create_note("", None, window, cx)),
                    )
                    .child(div().id("sidebar-list").mt(px(10.)).flex_1().overflow_y_scroll().child(list).child(
                        match new_folder {
                            Some(input) => div()
                                .mx(px(6.))
                                .mt(px(6.))
                                .px(px(8.))
                                .py(px(2.))
                                .rounded(px(6.))
                                .border_1()
                                .border_color(p.faint)
                                .bg(p.paper)
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(icon("folder-plus", 13., p.faint))
                                .child(div().flex_1().child(gpui_kit::component::input::Input::new(&input).appearance(false).px_0().text_size(px(ts::UI))))
                                .into_any_element(),
                            None => row("new-folder".into(), false)
                                .mt(px(6.))
                                .text_color(p.faint)
                                .child(icon("folder-plus", 13., p.faint))
                                .child("New folder")
                                .keyed_as("sidebar:new-folder", 6., ring, cx, |this, window, cx| this.start_new_folder(window, cx))
                                .into_any_element(),
                        },
                    ))
                    .child(
                        div()
                            .pt(px(10.))
                            .pb(px(10.))
                            .border_t_1()
                            .border_color(p.rule)
                            .child(div().px(px(14.)).text_size(px(ts::SMALL)).text_color(p.ink).truncate().child(who))
                            .child(div().px(px(14.)).mb(px(6.)).text_size(px(ts::META)).text_color(p.faint).child("All notes saved on this Mac"))
                            .child(action("sb-copy", "Copy this note", k_copy).keyed_as("sidebar:copy", 6., ring, cx, |this, _, cx| {
                                this.copy_note(cx);
                                this.sidebar_open = false;
                            }))
                            .child(action("sb-export", "Export this note\u{2026}", k_export).keyed_as("sidebar:export", 6., ring, cx, |this, window, cx| {
                                this.open_export(window, cx);
                            }))
                            .child(action("sb-jmw", if hidden { "Show threads" } else { "Hide threads" }, k_hide).keyed_as("sidebar:hide", 6., ring, cx, |this, _, cx| {
                                this.toggle_jmw(cx);
                                this.sidebar_open = false;
                            }))
                            .child(action("sb-custom", "Settings", k_settings).keyed_as("sidebar:settings", 6., ring, cx, |this, window, cx| this.open_settings(Tab::Customize, window, cx)))
                            .child(action("sb-minds", "Minds", String::new()).keyed_as("sidebar:minds", 6., ring, cx, |this, window, cx| this.open_settings(Tab::Minds, window, cx)))
                            .child(action("sb-model", "Model", String::new()).keyed_as("sidebar:model", 6., ring, cx, |this, window, cx| this.open_settings(Tab::Model, window, cx))),
                    ),
            )
            .child(
                div()
                    .id("sidebar-dim")
                    .flex_1()
                    .h_full()
                    .bg(p.cloth.opacity(0.5))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.sidebar_open = false;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    /// A name field in the sidebar. Enter or clicking away makes the folder (when
    /// a name was typed); Escape drops it.
    fn start_new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use gpui_kit::component::input::{InputEvent, InputState};
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Folder name"));
        let sub = cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, window, cx| {
            // Only for the field on screen: a late event from an older one is ignored.
            if this.new_folder.as_ref() != Some(input) {
                return;
            }
            match ev {
                InputEvent::PressEnter { .. } => this.finish_new_folder(true, window, cx),
                InputEvent::Blur => this.finish_new_folder(true, window, cx),
                _ => {}
            }
        });
        self.folder_sub = Some(sub);
        input.update(cx, |i, cx| i.focus(window, cx));
        self.new_folder = Some(input);
        cx.notify();
    }

    fn finish_new_folder(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.new_folder.take() else { return };
        let name = input.read(cx).value().trim().to_string();
        if keep && !name.is_empty() {
            let store = cx.global_mut::<Store>();
            match store.folders.iter_mut().find(|f| same_folder_name(&f.name, &name)) {
                // a folder by that name is already here: open it rather than make a twin
                Some(f) => f.collapsed = false,
                None => store.folders.push(Folder { id: new_id(), name, collapsed: false }),
            }
            store.save_folders();
        }
        // The field is gone; give the keyboard back to the note.
        if input.read(cx).focus_handle(cx).is_focused(window) {
            self.focus_editor(window, cx);
        }
        cx.notify();
    }

    fn copy_note(&mut self, cx: &mut Context<Self>) {
        let md = self.page.update(cx, |p, cx| p.markdown(true, cx));
        cx.write_to_clipboard(ClipboardItem::new_string(md));
        self.toast("Copied as Markdown", cx);
    }

    fn toggle_jmw(&mut self, cx: &mut Context<Self>) {
        self.page.update(cx, |p, cx| p.set_hidden(!p.just_my_words, cx));
        cx.notify();
    }

    fn export_dialog(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (_, format, with_threads) = self.export?;
        let t = Theme::get(cx).clone();
        let p = t.p;
        let title = self.page.read(cx).editor.read(cx).title();
        let ring = t.voice_ink;
        let choice = |id: &'static str, f: Format, name: &'static str, body: &'static str, cx: &mut Context<Self>| {
            ui::option_card(id, name, body, format == f, &t).keyed_as(format!("export:{id}"), 9., ring, cx, move |this, _, cx| {
                if let Some(e) = this.export.as_mut() {
                    e.1 = f;
                }
                cx.notify();
            })
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(p.dim)
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_export(window, cx)))
                .child(
                    div()
                        .id("export")
                        // Tab stays inside the dialog while it is open
                        .focus_trap("export-trap", &self.focus)
                        .w(px(440.))
                        .p(px(18.))
                        .rounded(px(12.))
                        // the dialogs' card, as ⌘K and the sketch seen larger have it
                        .bg(p.card)
                        .border_1()
                        .border_color(p.rule)
                        .shadow(ui::modal_shadow(&p))
                        .font_family(SANS)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        // a long title is cut at a word inside its quotation marks, which both stay
                        .child(div().text_size(px(ts::TITLE)).text_color(p.ink).truncate().child(format!(
                            "Export \u{201c}{}\u{201d}",
                            if title.trim().is_empty() { "Untitled".to_string() } else { crate::note_page::short_title(&title, 42) }
                        )))
                        .child(
                            div()
                                .mt(px(14.))
                                .flex()
                                .gap(px(8.))
                                .child(choice("ex-md", Format::Markdown, "Markdown", "Plain text for anywhere, another AI included.", cx))
                                .child(choice("ex-docx", Format::Word, "Word", "To keep editing in Word, Pages or Google Docs.", cx))
                                .child(choice("ex-html", Format::Html, "HTML", "To read, or print to PDF.", cx)),
                        )
                        .child(
                            div()
                                .id("ex-threads-row")
                                .mt(px(10.))
                                .p(px(10.))
                                .rounded(px(9.))
                                .border_1()
                                .border_color(p.rule)
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .cursor_pointer()
                                .hover(move |s| s.border_color(p.faint))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.))
                                        .child(div().text_size(px(ts::UI)).text_color(p.ink).child("Include the threads"))
                                        .child(div().text_size(px(ts::META)).text_color(p.graphite).child("Each thread sits under the paragraph it’s about.")),
                                )
                                .child(ui::switch("ex-threads", with_threads, &t))
                                .keyed_as("export:threads", 9., ring, cx, |this, _, cx| {
                                    if let Some(e) = this.export.as_mut() {
                                        e.2 = !e.2;
                                    }
                                    cx.notify();
                                }),
                        )
                        .child(
                            div()
                                .mt(px(16.))
                                .flex()
                                .justify_end()
                                .items_center()
                                .gap(px(14.))
                                .child(ui::ghost_button("ex-cancel", "Cancel", &t).keyed_as("export:cancel", 4., ring, cx, |this, window, cx| {
                                    this.close_export(window, cx)
                                }))
                                .child(ui::primary_button("ex-go", "Export\u{2026}", &t).keyed_as("export:go", 12., ring, cx, move |this, window, cx| {
                                    this.run_export(format, with_threads, window, cx)
                                })),
                        ),
                )
                .into_any_element(),
        )
    }

    /// The export dialog from the keyboard: ← → pick the format, Return exports.
    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, format, with_threads)) = self.export else { return };
        let formats = [Format::Markdown, Format::Word, Format::Html];
        let at = formats.iter().position(|f| *f == format).unwrap_or(0);
        match ev.keystroke.key.as_str() {
            "enter" if ev.keystroke.modifiers == Modifiers::default() => self.run_export(format, with_threads, window, cx),
            "left" => self.export = Some((ExportStage::Choose, formats[at.saturating_sub(1)], with_threads)),
            "right" => self.export = Some((ExportStage::Choose, formats[(at + 1).min(formats.len() - 1)], with_threads)),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let sidebar = self.sidebar_open.then(|| self.sidebar(cx));
        let export = self.export_dialog(cx);
        let badge = self.listening_badge(cx);
        let blots = self.blot_layer(cx);
        // The window itself is not focusable: a click on the linen, the header or any
        // other chrome leaves the keyboard where it was (in the note) instead of taking
        // it to the window, where typing goes nowhere.
        div()
            .id("app")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(p.paper)
            .text_color(p.ink)
            .font_family(SANS)
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.create_note("", None, window, cx)))
            .on_action(cx.listener(|this, _: &OpenPalette, window, cx| this.open_palette(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleChat, window, cx| {
                this.page.update(cx, |p, cx| p.toggle_chat(window, cx));
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.open_settings(Tab::Customize, window, cx)))
            .on_action(cx.listener(|this, _: &JustMyWords, _, cx| this.toggle_jmw(cx)))
            .on_action(cx.listener(|this, _: &CopyNote, _, cx| this.copy_note(cx)))
            .on_action(cx.listener(|this, _: &ExportNote, window, cx| this.open_export(window, cx)))
            .on_action(cx.listener(|this, _: &FindLooseThread, window, cx| this.page.update(cx, |p, cx| p.find_loose_thread(window, cx))))
            .on_action(cx.listener(|this, _: &NextThread, window, cx| this.page.update(cx, |p, cx| p.jump_to_open(true, window, cx))))
            .on_action(cx.listener(|this, _: &PrevThread, window, cx| this.page.update(cx, |p, cx| p.jump_to_open(false, window, cx))))
            .on_action(cx.listener(|this, _: &DismissThread, _, cx| this.page.update(cx, |p, cx| p.dismiss_active(cx))))
            .on_action(cx.listener(|this, _: &UndoDismiss, _, cx| this.page.update(cx, |p, cx| p.undo_dismiss(cx))))
            .on_action(cx.listener(|this, _: &ShowWelcome, window, cx| this.show_welcome(window, cx)))
            .on_action(cx.listener(|this, _: &ImportObsidian, _, cx| this.choose_vault(cx)))
            .on_modifiers_changed(cx.listener(Self::on_modifiers))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_move(cx.listener(Self::on_pointer))
            .child(div().flex_1().min_h(px(0.)).child(self.page.clone()))
            .child(self.chrome_layer(window, cx))
            .children(sidebar)
            .children(self.palette.clone())
            .children(self.settings.clone())
            .children(export)
            // every toast, the page's included, in one stack centred under the page
            .children({
                let chat = if self.page.read(cx).chat_open { crate::note_page::CHAT_WIDTH } else { 0. };
                crate::toast::stack(px(chat), window, cx, |this, kind, window, cx| match kind {
                    Kind::NoteDeleted => this.undo_delete(window, cx),
                    Kind::ThreadDismissed => this.page.update(cx, |p, cx| p.undo_dismiss(cx)),
                    _ => {}
                })
            })
            .child(badge)
            .children(blots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(alt: bool, shift: bool, platform: bool) -> Modifiers {
        Modifiers { alt, shift, platform, ..Default::default() }
    }

    #[std::prelude::v1::test]
    fn holding_option_alone_starts_and_release_stops() {
        let mut t = Talk::default();
        assert_eq!(t.modifiers(mods(true, false, false), false), TalkStep::StartTimer);
        assert_eq!(t.waited(), TalkStep::Start);
        assert_eq!(t, Talk::Listening);
        // Other modifiers and keys while talking do not stop it; letting go does.
        assert_eq!(t.modifiers(mods(true, true, false), false), TalkStep::Nothing);
        assert_eq!(t.key(), TalkStep::Nothing);
        assert_eq!(t.modifiers(mods(false, false, false), false), TalkStep::Stop);
        assert_eq!(t, Talk::Idle);
    }

    #[std::prelude::v1::test]
    fn a_quick_tap_never_listens() {
        let mut t = Talk::default();
        t.modifiers(mods(true, false, false), false);
        assert_eq!(t.modifiers(Modifiers::default(), false), TalkStep::CancelTimer);
        assert_eq!(t.waited(), TalkStep::Nothing, "a timer that fires late does nothing");
    }

    #[std::prelude::v1::test]
    fn option_as_a_modifier_spoils_the_hold_until_release() {
        // ⌥← (a binding consumes the arrow; the interceptor still reports it).
        let mut t = Talk::default();
        t.modifiers(mods(true, false, false), false);
        assert_eq!(t.key(), TalkStep::CancelTimer);
        assert_eq!(t.waited(), TalkStep::Nothing);
        assert_eq!(t.key(), TalkStep::Nothing, "more ⌥← presses");
        assert_eq!(t.modifiers(Modifiers::default(), false), TalkStep::Nothing);
        assert_eq!(t, Talk::Idle);

        // ⌥ then ⌘, ⌘ let go while ⌥ stays down: still not a hold to talk.
        let mut t = Talk::default();
        t.modifiers(mods(true, false, false), false);
        assert_eq!(t.modifiers(mods(true, false, true), false), TalkStep::CancelTimer);
        assert_eq!(t.modifiers(mods(true, false, false), false), TalkStep::Nothing);
        assert_eq!(t.waited(), TalkStep::Nothing);

        // ⌘ first, then ⌥.
        let mut t = Talk::default();
        assert_eq!(t.modifiers(mods(true, false, true), false), TalkStep::Nothing);
        assert_eq!(t.modifiers(mods(true, false, false), false), TalkStep::Nothing);
        assert_eq!(t, Talk::Spoiled);
        // A fresh press after letting go counts again.
        t.modifiers(Modifiers::default(), false);
        assert_eq!(t.modifiers(mods(true, false, false), false), TalkStep::StartTimer);
    }

    #[std::prelude::v1::test]
    fn no_second_session_while_the_first_finishes_and_focus_loss_stops() {
        let mut t = Talk::default();
        assert_eq!(t.modifiers(mods(true, false, false), true), TalkStep::Nothing);
        assert_eq!(t, Talk::Spoiled);

        let mut t = Talk::default();
        t.modifiers(mods(true, false, false), false);
        t.waited();
        assert_eq!(t.deactivated(), TalkStep::Stop);
        assert_eq!(t, Talk::Idle);
        t.modifiers(mods(true, false, false), false);
        assert_eq!(t.deactivated(), TalkStep::CancelTimer);

        // The recognizer gave up on its own while ⌥ is still held.
        let mut t = Talk::default();
        t.modifiers(mods(true, false, false), false);
        t.waited();
        t.ended();
        assert_eq!(t, Talk::Spoiled);
        assert_eq!(t.modifiers(Modifiers::default(), false), TalkStep::Nothing);
    }

    #[std::prelude::v1::test]
    fn export_names_are_safe_for_finder() {
        assert_eq!(export_file_name("Rate limiter design", "md"), "Rate limiter design.md");
        assert_eq!(export_file_name("  Q3/Q4: plans \\ ideas  ", "docx"), "Q3-Q4- plans - ideas.docx");
        assert_eq!(export_file_name("", "html"), "Untitled.html");
        assert_eq!(export_file_name("   ", "md"), "Untitled.md");
        assert_eq!(export_file_name(".env notes", "md"), "env notes.md", "not a hidden file");
        assert_eq!(export_file_name("...", "md"), "Untitled.md");
        assert_eq!(export_file_name(" . . hidden", "md"), "hidden.md");
        assert_eq!(export_file_name("tab\there\nnew line", "md"), "tab here new line.md");
        let long = "長".repeat(100);
        let name = export_file_name(&long, "docx");
        assert!(name.len() <= 205 && name.ends_with(".docx") && name.starts_with('長'), "{name}");
    }

    #[std::prelude::v1::test]
    fn folder_names_match_ignoring_case() {
        assert!(same_folder_name("Work", "work"));
        assert!(same_folder_name("ÉTÉ", "été"));
        assert!(same_folder_name(" Work ", "work"));
        assert!(!same_folder_name("Work", "Workshop"));
    }

    #[std::prelude::v1::test]
    fn ink_blots_are_bounded_and_dry_up() {
        let t0 = Instant::now();
        let blot = |secs: u64| Blot { at: point(px(1.), px(2.)), born: t0 + Duration::from_secs(secs), seed: 1 };
        let mut blots = vec![];
        for i in 0..20 {
            add_blot(&mut blots, blot(i));
        }
        assert_eq!(blots.len(), MAX_BLOTS);
        assert_eq!(blots[0].born, t0 + Duration::from_secs(8), "the oldest go first");
        prune_blots(&mut blots, t0 + Duration::from_secs(19) + BLOT_LIFE);
        assert!(blots.is_empty());
        let mut fresh = vec![blot(0)];
        prune_blots(&mut fresh, t0 + Duration::from_millis(2000));
        assert_eq!(fresh.len(), 1);
    }
}

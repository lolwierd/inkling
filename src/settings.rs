//! Settings (⌘,): a window of its own, as in any Mac app. A frosted sidebar of panes
//! rests on linen you can see the desktop through; the controls sit on a flat page. The
//! panes: how Inkling reads, how it looks, your minds, what it knows about you, which
//! model it uses, every keyboard shortcut, and help.
//!
//! A window rather than a sheet over the note, for three reasons. The only blur GPUI has
//! is a window's own (the desktop behind it), so real glass needs a window. A window sits
//! beside the note, so appearance, text size and thread colour show in the note as you
//! change them. And shortcuts are recorded in a window of their own, where the note's
//! keys cannot fire.

mod parts;

use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::time::Duration;

use crate::brain::{Brain, BrainState, effort_for};
use crate::shortcuts::{self, Group, Shortcuts, Take};
use crate::store::{
    Appearance, Frequency, InkColor, Lettering, Mind, ModelChoice, Mode, Pace, Placement, Store, TextSize, Voice,
    chosen_model, mind_templates, new_id,
};
use crate::theme::{Palette, SANS, SERIF, Theme, ink_color, note_size, thread_hex, ts};
use crate::ui::{self, icon};
use parts::{Cap, Glass, group, note, row, section, segment, segmented};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    /// How Inkling reads ("Reading" in the sidebar).
    Customize,
    Appearance,
    Minds,
    NewMind,
    AboutYou,
    Model,
    Shortcuts,
    Help,
}

impl Tab {
    /// The sidebar, in order. A new or edited mind is a page inside Minds.
    const PANES: [Tab; 7] = [Tab::Customize, Tab::Appearance, Tab::Minds, Tab::AboutYou, Tab::Model, Tab::Shortcuts, Tab::Help];

    fn pane(self) -> Tab {
        if self == Tab::NewMind { Tab::Minds } else { self }
    }

    fn label(self) -> &'static str {
        match self.pane() {
            Tab::Customize => "Reading",
            Tab::Appearance => "Appearance",
            Tab::Minds | Tab::NewMind => "Minds",
            Tab::AboutYou => "About you",
            Tab::Model => "Model",
            Tab::Shortcuts => "Shortcuts",
            Tab::Help => "Help",
        }
    }

    fn icon(self) -> &'static str {
        match self.pane() {
            Tab::Customize => "book-open",
            Tab::Appearance => "sun-moon",
            Tab::Minds | Tab::NewMind => "spool",
            Tab::AboutYou => "user-round",
            Tab::Model => "sparkles",
            Tab::Shortcuts => "keyboard",
            Tab::Help => "life-buoy",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Tab::Customize => "How Inkling reads along, and where its questions go.",
            Tab::Appearance => "How Inkling looks. Your note changes as you choose.",
            Tab::Minds => "Each mind reads your notes its own way. Pick one for any note from the row above its title.",
            Tab::NewMind => "A mind is what reads your notes: what it looks for, and how it asks. Give any note the one it needs.",
            Tab::AboutYou => "A short paragraph every note starts out knowing. It stays on this Mac.",
            Tab::Model => "Inkling thinks with Claude through the Claude Agent SDK, using this Mac's Claude Code login, so it runs on your Claude subscription. No API key.",
            Tab::Shortcuts => "Click keys to change them. New keys work at once, in your notes and in the menu bar.",
            Tab::Help => "How Inkling works, and where your notes are kept.",
        }
    }

    /// INKLING_SETTINGS_TAB names a pane to open on, for screenshots.
    fn named(name: &str) -> Option<Tab> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "reading" | "customize" | "settings" => Tab::Customize,
            "appearance" => Tab::Appearance,
            "minds" => Tab::Minds,
            "newmind" => Tab::NewMind,
            "about" | "aboutyou" => Tab::AboutYou,
            "model" => Tab::Model,
            "shortcuts" | "keys" => Tab::Shortcuts,
            "help" => Tab::Help,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub enum SettingsEvent {
    Changed,
    MindsChanged,
    Close,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

/// How long the about-you paragraph may be, in characters.
pub const ABOUT_LIMIT: usize = 700;

/// `text` cut to [`ABOUT_LIMIT`] characters, or None when it fits.
fn over_limit(text: &str) -> Option<String> {
    text.char_indices().nth(ABOUT_LIMIT).map(|(i, _)| text[..i].to_string())
}

/// Effort levels offered when Claude Code has not said which a model takes.
const USUAL_EFFORTS: [&str; 3] = ["low", "medium", "high"];

/// The open settings window, if any: ⌘, again brings it forward instead of opening another.
#[derive(Default)]
struct SettingsWindow(Option<(AnyWindowHandle, WeakEntity<SettingsView>)>);

impl Global for SettingsWindow {}

/// Open Settings at `tab`, or bring the open window forward and show `tab` there.
pub fn open_window(tab: Tab, cx: &mut App) -> Option<Entity<SettingsView>> {
    if let Some((handle, view)) = cx.default_global::<SettingsWindow>().0.clone()
        && let Some(view) = view.upgrade()
    {
        let shown = handle.update(cx, |_, window, cx| {
            view.update(cx, |v, cx| v.show(tab, window, cx));
            window.activate_window();
        });
        if shown.is_ok() {
            return Some(view);
        }
    }
    let background = std::env::var("INKLING_BACKGROUND").is_ok();
    let tab = std::env::var("INKLING_SETTINGS_TAB").ok().and_then(|t| Tab::named(&t)).unwrap_or(tab);
    // INKLING_WINDOW_TITLE names test runs' windows; this one is named after it.
    let title = match std::env::var("INKLING_WINDOW_TITLE") {
        Ok(t) => format!("{t} - Settings"),
        Err(_) => "Inkling Settings".into(),
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(900.), px(720.)), cx))),
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(22.), px(22.))),
        }),
        window_background: WindowBackgroundAppearance::Blurred,
        window_min_size: Some(size(px(820.), px(560.))),
        focus: !background,
        ..Default::default()
    };
    let opened = gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| SettingsView::new(tab, window, cx)));
    let (handle, view) = match opened {
        Ok(v) => v,
        Err(e) => {
            log::error!("could not open the settings window: {e:#}");
            return None;
        }
    };
    let _ = handle.update(cx, |_, window, cx| {
        // The component library paints every window's root in its theme's background;
        // glass needs it clear.
        gpui_kit::base::Root::update(window, cx, |root, _, _| root.style().background = Some(transparent_black().into()));
        if !background {
            window.activate_window();
        }
    });
    cx.default_global::<SettingsWindow>().0 = Some((handle, view.downgrade()));
    Some(view)
}

/// Waiting for keys for a shortcut.
struct Recording {
    name: &'static str,
    /// Which of its keys to replace; None adds keys.
    slot: Option<usize>,
    /// Modifiers held so far, shown while you build the chord.
    held: Modifiers,
    /// Why the last keys could not be used.
    refused: Option<&'static str>,
}

pub struct SettingsView {
    focus: FocusHandle,
    tab: Tab,
    editing: Option<String>,
    template: Option<usize>,
    name: Entity<InputState>,
    description: Entity<InputState>,
    how: Entity<TextareaState>,
    voice: Voice,
    frequency: Frequency,
    about: Entity<TextareaState>,
    model: Entity<InputState>,
    search: Entity<InputState>,
    recording: Option<Recording>,
    /// Takes key presses before any binding while recording, so ⌘Q is recorded, not run.
    record_sub: Option<Subscription>,
    /// Whether the Model pane shows the pinned versions.
    show_versions: bool,
    /// The page's scroll (INKLING_SETTINGS_SCROLL scrolls it at launch, for screenshots).
    scroll: ScrollHandle,
    _subs: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(tab: Tab, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let s = cx.global::<Store>().settings.clone();
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Staff engineer"));
        let description = cx.new(|cx| InputState::new(window, cx).placeholder("Reads designs as if it will be on call for them."));
        let how = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(3, 8)
                .placeholder("Read like a senior engineer reviewing a design doc. Ask about scale and failure. Ignore style.")
        });
        let about = cx.new(|cx| {
            let mut t = TextareaState::new(window, cx)
                .auto_grow(5, 12)
                .placeholder("Nothing yet. What you do, what you're working toward, how you like feedback.");
            t.set_value(s.about_me.clone(), window, cx);
            t
        });
        let model = cx.new(|cx| {
            let mut i = InputState::new(window, cx).placeholder("Claude Code's default");
            i.set_value(s.model.clone(), window, cx);
            i
        });
        let search = cx.new(|cx| {
            let mut i = InputState::new(window, cx).placeholder("Search shortcuts");
            // Test aid: INKLING_SETTINGS_SEARCH fills the shortcut search at launch.
            if let Ok(q) = std::env::var("INKLING_SETTINGS_SEARCH") {
                i.set_value(q, window, cx);
            }
            i
        });
        let mut subs = vec![];
        subs.push(cx.subscribe_in(&about, window, |_, about, ev: &InputEvent, window, cx| {
            if let InputEvent::Change = ev {
                let typed = about.read(cx).value().to_string();
                // Cut the box itself back to the limit, so what it shows is what is
                // kept (setting the value does not send another Change).
                let text = match over_limit(&typed) {
                    Some(cut) => {
                        about.update(cx, |t, cx| {
                            t.set_value(cut.clone(), window, cx);
                            t.set_selected_range(cut.len()..cut.len(), cx);
                        });
                        cut
                    }
                    None => typed,
                };
                let store = cx.global_mut::<Store>();
                store.settings.about_me = text;
                store.save_settings();
                cx.notify();
            }
        }));
        // The save button follows whether the name is filled in; the shortcut list follows the search.
        for input in [&name, &search] {
            subs.push(cx.subscribe(input, |_, _, ev: &InputEvent, cx| {
                if let InputEvent::Change = ev {
                    cx.notify();
                }
            }));
        }
        // Empty is Claude Code's own default.
        subs.push(cx.subscribe_in(&model, window, |_, model, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                let v = model.read(cx).value().trim().to_string();
                let store = cx.global_mut::<Store>();
                store.settings.model = v;
                store.save_settings();
                cx.notify();
            }
        }));
        // Sign-in, the model list and usage arrive from the brain while the window is open.
        subs.push(cx.observe(&Brain::global(cx), |_, _, cx| cx.notify()));
        subs.push(cx.observe_window_appearance(window, |_, window, cx| {
            crate::app::apply_theme(window.appearance(), cx);
            cx.refresh_windows();
        }));
        subs.push(cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.stop_recording(cx);
            }
        }));
        // Closing the last note window closes Settings too, so the app can quit.
        let settings_id = window.window_handle().window_id();
        subs.push(cx.on_window_closed(move |cx, _| {
            if cx.windows().iter().all(|w| w.window_id() == settings_id) {
                cx.defer(move |cx| {
                    let handle = cx.windows().into_iter().find(|w| w.window_id() == settings_id);
                    if let Some(handle) = handle {
                        let _ = handle.update(cx, |_, window, _| window.remove_window());
                    }
                });
            }
        }));
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| this.closing(cx));
            }
            true
        });
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let mut this = Self {
            focus,
            tab: Tab::Customize,
            editing: None,
            template: None,
            name,
            description,
            how,
            voice: Voice::Plain,
            frequency: Frequency::WhenItMatters,
            about,
            model,
            search,
            recording: None,
            record_sub: None,
            show_versions: false,
            scroll: ScrollHandle::new(),
            _subs: subs,
        };
        // Test aid: INKLING_SETTINGS_SCROLL=<px> scrolls the page down that far.
        if let Some(y) = std::env::var("INKLING_SETTINGS_SCROLL").ok().and_then(|v| v.parse::<f32>().ok()) {
            this.scroll.set_offset(point(px(0.), px(-y)));
        }
        this.show(tab, window, cx);
        this.jiggle(window, cx);
        // Test aid: INKLING_SETTINGS_RECORD=<action name, e.g. app::NewNote> opens listening
        // for new keys for that shortcut, so a run can send keys without clicking.
        if let Ok(name) = std::env::var("INKLING_SETTINGS_RECORD") {
            let found = cx.global::<Shortcuts>().0.iter().find(|s| s.name == name.trim()).map(|s| s.name);
            if let Some(name) = found {
                this.tab = Tab::Shortcuts;
                this.start_recording(name, Some(0), window, cx);
            }
        }
        this.press_test_keys(window, cx);
        this
    }

    /// Test aid: INKLING_SETTINGS_KEYS="settings:alt-cmd-right,main:alt-cmd-right" presses
    /// each key in that window (Settings or the note), a second apart, through GPUI's own
    /// dispatch, as a key press would arrive. Commas or spaces separate the presses.
    fn press_test_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(steps) = std::env::var("INKLING_SETTINGS_KEYS") else { return };
        let here = window.window_handle();
        let steps: Vec<(bool, String)> = steps
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter_map(|s| s.split_once(':'))
            .map(|(w, k)| (w == "settings", k.to_string()))
            .collect();
        cx.spawn_in(window, async move |_, cx| {
            for (in_settings, keys) in steps {
                cx.background_executor().timer(Duration::from_millis(1000)).await;
                let Ok(keystroke) = Keystroke::parse_with_os(&keys, OperatingSystem::Mac) else { continue };
                // This task runs in the settings window, which is busy while it runs: its keys
                // go straight to it, the note's through its handle.
                let _ = cx.update(|window, cx| {
                    log::info!("settings test keys: {keys} in {}", if in_settings { "settings" } else { "the note" });
                    if in_settings {
                        window.dispatch_keystroke(keystroke, cx);
                    } else if let Some(note) = cx.windows().into_iter().find(|w| w.window_id() != here.window_id()) {
                        let _ = note.update(cx, |_, window, cx| window.dispatch_keystroke(keystroke, cx));
                    }
                });
            }
        })
        .detach();
    }

    /// Debug aid, as in the note window: INKLING_JIGGLE=<ms> nudges the window by a pixel
    /// every <ms>, so a covered window keeps drawing for background screenshots.
    fn jiggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ms) = std::env::var("INKLING_JIGGLE").ok().and_then(|v| v.parse::<u64>().ok()) else { return };
        let ms = ms.max(16);
        cx.spawn_in(window, async move |_, cx| {
            let mut grow = true;
            loop {
                cx.background_executor().timer(Duration::from_millis(ms)).await;
                let ok = cx.update(|window, _| {
                    let s = window.viewport_size();
                    let d = if grow { px(1.) } else { px(-1.) };
                    window.resize(size(s.width + d, s.height));
                });
                if ok.is_err() {
                    break;
                }
                grow = !grow;
            }
        })
        .detach();
    }

    /// Show a pane (opening again on a pane switches to it).
    pub fn show(&mut self, tab: Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_recording(cx);
        if tab == Tab::NewMind {
            self.start_mind(None, window, cx);
        } else {
            self.tab = tab;
            if self.tab != Tab::Minds {
                self.editing = None;
            }
        }
        cx.notify();
    }

    /// The window is going: tell the app, and forget it.
    fn closing(&mut self, cx: &mut Context<Self>) {
        self.stop_recording(cx);
        cx.default_global::<SettingsWindow>().0 = None;
        cx.emit(SettingsEvent::Close);
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.closing(cx);
        window.remove_window();
    }

    fn update(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut crate::store::Settings)) {
        let store = cx.global_mut::<Store>();
        f(&mut store.settings);
        store.save_settings();
        cx.emit(SettingsEvent::Changed);
        cx.notify();
    }

    fn start_mind(&mut self, mind: Option<Mind>, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = Tab::NewMind;
        self.template = None;
        let m = mind.clone().unwrap_or(Mind {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            how_it_thinks: String::new(),
            voice: Voice::Plain,
            frequency: Frequency::WhenItMatters,
        });
        self.editing = mind.map(|m| m.id);
        self.fill(&m, window, cx);
    }

    fn fill(&mut self, m: &Mind, window: &mut Window, cx: &mut Context<Self>) {
        self.name.update(cx, |i, cx| i.set_value(m.name.clone(), window, cx));
        self.description.update(cx, |i, cx| i.set_value(m.description.clone(), window, cx));
        self.how.update(cx, |i, cx| i.set_value(m.how_it_thinks.clone(), window, cx));
        self.voice = m.voice;
        self.frequency = m.frequency;
        cx.notify();
    }

    fn save_mind(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let mind = Mind {
            id: self.editing.clone().unwrap_or_else(new_id),
            name,
            description: self.description.read(cx).value().trim().to_string(),
            how_it_thinks: self.how.read(cx).value().trim().to_string(),
            voice: self.voice,
            frequency: self.frequency,
        };
        let store = cx.global_mut::<Store>();
        match store.minds.iter_mut().find(|m| m.id == mind.id) {
            Some(m) => *m = mind,
            None => store.minds.push(mind),
        }
        store.save_minds();
        self.tab = Tab::Minds;
        self.editing = None;
        cx.emit(SettingsEvent::MindsChanged);
        cx.notify();
    }

    fn delete_mind(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.editing.take() {
            let store = cx.global_mut::<Store>();
            store.minds.retain(|m| m.id != id);
            store.save_minds();
            cx.emit(SettingsEvent::MindsChanged);
        }
        self.tab = Tab::Minds;
        cx.notify();
    }

    // ----- recording shortcuts ---------------------------------------------------------

    fn start_recording(&mut self, name: &'static str, slot: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        // Out of the search field, so the keys are not typed into it.
        window.focus(&self.focus, cx);
        self.recording = Some(Recording { name, slot, held: window.modifiers(), refused: None });
        let view = cx.entity().downgrade();
        let here = window.window_handle().window_id();
        self.record_sub = Some(cx.intercept_keystrokes(move |ev, window, cx| {
            if window.window_handle().window_id() != here {
                return;
            }
            let keystroke = ev.keystroke.clone();
            if view.update(cx, |this, cx| this.recorded(&keystroke, cx)).unwrap_or(false) {
                cx.stop_propagation();
            }
        }));
        cx.notify();
    }

    fn stop_recording(&mut self, cx: &mut Context<Self>) {
        if self.recording.take().is_some() {
            cx.notify();
        }
        self.record_sub = None;
    }

    /// A key press while recording. Returns whether it was taken (so nothing else runs it).
    fn recorded(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let Some(rec) = self.recording.as_mut() else { return false };
        if keystroke.key == "escape" && !keystroke.modifiers.modified() {
            self.stop_recording(cx);
            return true;
        }
        match shortcuts::take(keystroke) {
            Take::Wait => false,
            Take::Refuse(why) => {
                rec.refused = Some(why);
                cx.notify();
                true
            }
            Take::Keys(keys) => {
                let (name, slot) = (rec.name, rec.slot);
                self.stop_recording(cx);
                let current = self.shown_keys(name, cx);
                shortcuts::set(name, shortcuts::with_keys(&current, slot, keys), cx);
                cx.notify();
                true
            }
        }
    }

    fn shown_keys(&self, name: &str, cx: &App) -> Vec<String> {
        let custom = &cx.global::<Store>().settings.shortcuts;
        cx.global::<Shortcuts>().0.iter().find(|s| s.name == name).map(|s| shortcuts::shown(s, custom).to_vec()).unwrap_or_default()
    }

    fn remove_keys(&mut self, name: &'static str, slot: usize, cx: &mut Context<Self>) {
        let mut keys = self.shown_keys(name, cx);
        if slot < keys.len() {
            keys.remove(slot);
        }
        shortcuts::set(name, keys, cx);
        cx.notify();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.recording.is_some() {
            return;
        }
        let k = &ev.keystroke;
        let m = k.modifiers;
        let only_cmd = m.platform && !m.shift && !m.alt && !m.control;
        match k.key.as_str() {
            "escape" if !m.modified() => self.close(window, cx),
            "w" if only_cmd => self.close(window, cx),
            // Up and down walk the sidebar, as in System Settings, when no field has the keys.
            "up" | "down" if !m.modified() && self.focus.is_focused(window) => {
                let at = Tab::PANES.iter().position(|t| *t == self.tab.pane()).unwrap_or(0);
                let next = if k.key == "up" { at.saturating_sub(1) } else { (at + 1).min(Tab::PANES.len() - 1) };
                self.show(Tab::PANES[next], window, cx);
            }
            _ => {}
        }
    }

    fn on_modifiers(&mut self, ev: &ModifiersChangedEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(rec) = self.recording.as_mut() {
            rec.held = ev.modifiers;
            cx.notify();
        }
    }

    // ----- the panes -------------------------------------------------------------------

    fn reading(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let s = cx.global::<Store>().settings.clone();
        let mode = |id: &'static str, title: &'static str, body: &'static str, m: Mode, cx: &mut Context<Self>| {
            let words = div()
                .child(div().text_size(px(ts::UI)).text_color(p.ink).child(title))
                .child(div().mt(px(4.)).text_size(px(ts::SMALL)).line_height(px(16.)).text_color(p.graphite).child(body));
            parts::tile(id, s.default_mode == m, &t, words)
                .p(px(14.))
                .pr(px(30.))
                .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.default_mode = m)))
        };
        let pace_line = match s.pace {
            Pace::Regular => "Reads a moment after you stop typing.",
            Pace::Slow => "Waits about twice as long, for when you pause mid-thought.",
        };
        let mut pace = segmented(&t);
        for (i, &v) in Pace::ALL.iter().enumerate() {
            pace = pace.child(segment(("pace", i), v.label(), s.pace == v, &t).on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.pace = v))));
        }
        let mut places = div().flex().gap(px(10.));
        for (i, &v) in Placement::ALL.iter().enumerate() {
            let body = match v {
                Placement::Both => "Your words in the middle, questions to either side.",
                Placement::Right => "Every question in one column beside your writing.",
                Placement::Marks => "A small ? or \u{d7} after the passage. Open it when you're ready.",
            };
            let card = div()
                .child(parts::page_picture(v, &p, t.voice_ink).w_full())
                .child(div().mt(px(9.)).px(px(4.)).text_size(px(ts::UI)).text_color(p.ink).child(v.label()))
                .child(div().mt(px(2.)).px(px(4.)).pb(px(4.)).text_size(px(ts::SMALL)).line_height(px(16.)).text_color(p.graphite).child(body));
            places = places.child(
                parts::tile(("place", i), s.placement == v, &t, card)
                    .p(px(8.))
                    .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.placement = v))),
            );
        }
        div()
            .child(section("New notes start in", None, &t).mt(px(4.)))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(mode("m-think", "Reads along", "Reads along and speaks up when something's worth a second look.", Mode::ThinksWithYou, cx))
                    .child(mode("m-ask", "Waits to be asked", "Quiet until you say its name, select words, or open the chat.", Mode::WhenYouAsk, cx)),
            )
            .child(note("Change any note on its own, from the row above its title.", &t))
            .child(section("Reading pace", None, &t))
            .child(group(vec![row("Pace", Some(pace_line.into()), pace, &t).into_any_element()], &t))
            .child(section("Where questions appear", None, &t))
            .child(places)
    }

    fn appearance(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let s = cx.global::<Store>().settings.clone();
        let (light, dark) = (Palette::light(), Palette::dark());
        let (thread_light, thread_dark) = thread_hex(s.ink);
        let (thread_light, thread_dark): (Hsla, Hsla) = (rgb(thread_light).into(), rgb(thread_dark).into());
        let mut looks = div().flex().gap(px(10.));
        for (i, &a) in Appearance::ALL.iter().enumerate() {
            let picture = match a {
                Appearance::Light => parts::window_picture(&light, thread_light),
                Appearance::Dark => parts::window_picture(&dark, thread_dark),
                Appearance::System => {
                    parts::split_picture(parts::window_picture(&light, thread_light), parts::window_picture(&dark, thread_dark))
                }
            };
            let body = match a {
                Appearance::System => "Follows macOS.",
                Appearance::Light => "Linen: paper and ink.",
                Appearance::Dark => "Lamplight: the same cloth after dark.",
            };
            let card = div()
                .child(picture.w_full())
                .child(div().mt(px(9.)).px(px(4.)).text_size(px(ts::UI)).text_color(p.ink).child(a.label()))
                .child(div().mt(px(2.)).px(px(4.)).pb(px(4.)).text_size(px(ts::SMALL)).line_height(px(16.)).text_color(p.graphite).child(body));
            looks = looks.child(
                parts::tile(("look", i), s.appearance == a, &t, card)
                    .p(px(8.))
                    .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.appearance = a))),
            );
        }
        let mut sizes = div().flex().gap(px(10.));
        for (i, &size) in TextSize::ALL.iter().enumerate() {
            let card = div()
                .h_full()
                .flex()
                .flex_col()
                .justify_between()
                .child(div().font_family(SERIF).text_size(note_size(size)).text_color(p.ink).child("A thought."))
                .child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child(size.label()));
            sizes = sizes.child(
                parts::tile(("size", i), s.text_size == size, &t, card)
                    .h(px(78.))
                    .px(px(14.))
                    .py(px(12.))
                    .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.text_size = size))),
            );
        }
        let lettering = |id: &'static str, font: &'static str, l: Lettering, cx: &mut Context<Self>| {
            let card = div()
                .h_full()
                .flex()
                .flex_col()
                .justify_between()
                .child(div().font_family(font).text_size(px(ts::BODY)).text_color(t.voice_ink).child("Does this still hold?"))
                .child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child(l.label()));
            parts::tile(id, s.lettering == l, &t, card)
                .h(px(78.))
                .px(px(14.))
                .py(px(12.))
                .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.lettering = l)))
        };
        let mut swatches = div().flex_none().flex().gap(px(6.));
        for (i, &ink) in InkColor::ALL.iter().enumerate() {
            let on = s.ink == ink;
            swatches = swatches.child(
                div()
                    .id(("ink", i))
                    .size(px(24.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(if on { p.ink } else { transparent_black() })
                    .cursor_pointer()
                    .hover(move |st| if on { st } else { st.border_color(p.rule) })
                    .child(div().size(px(16.)).rounded_full().bg(ink_color(ink, &p)))
                    .on_click(cx.listener(move |this, _, _, cx| this.update(cx, |s| s.ink = ink))),
            );
        }
        let thread_name = if s.ink == InkColor::Ink { format!("{} (Inkling's own)", s.ink.label()) } else { s.ink.label().to_string() };
        div()
            .child(section("Appearance", None, &t).mt(px(4.)))
            .child(looks)
            .child(section("Text size", Some("Your words, in Fraunces".into()), &t))
            .child(sizes)
            .child(section("Inkling's lettering", None, &t))
            .child(div().flex().gap(px(10.)).child(lettering("let-own", SANS, Lettering::Own, cx)).child(lettering("let-notes", SERIF, Lettering::LikeNotes, cx)))
            .child(section("Inkling's thread", None, &t))
            .child(group(
                vec![
                    row("Colour", Some(thread_name.into()), swatches, &t).into_any_element(),
                    row(
                        "Ink from the pointer",
                        Some("Now and then a drop of ink falls as you move the pointer, then dries and fades.".into()),
                        ui::switch("ink-pointer", s.ink_from_pointer, &t)
                            .on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.ink_from_pointer = !s.ink_from_pointer))),
                        &t,
                    )
                    .into_any_element(),
                ],
                &t,
            ))
            .child(note("Questions from the note's own mind are stitched in this thread; other minds keep their own.", &t))
    }

    fn minds(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let minds = cx.global::<Store>().minds.clone();
        let built_in = minds.first().cloned();
        let mut yours = vec![];
        for (i, m) in minds.iter().enumerate().skip(1) {
            let mind = m.clone();
            let detail = (!m.description.trim().is_empty()).then(|| SharedString::from(m.description.clone()));
            yours.push(
                row(
                    m.name.clone(),
                    detail,
                    parts::push_button(("edit-mind", i), "Edit", &t).on_click(cx.listener(move |this, _, window, cx| this.start_mind(Some(mind.clone()), window, cx))),
                    &t,
                )
                .into_any_element(),
            );
        }
        let empty = yours.is_empty();
        div()
            .children(built_in.map(|m| {
                group(
                    vec![row(m.name.clone(), Some(m.description.clone().into()), div().text_size(px(ts::META)).text_color(p.faint).child("Built in"), &t).into_any_element()],
                    &t,
                )
                .mt(px(4.))
            }))
            .child(section("Yours", None, &t))
            .when(empty, |d| {
                let mut chips = div().mt(px(10.)).flex().flex_wrap().gap(px(6.));
                for (i, tm) in mind_templates().into_iter().enumerate() {
                    chips = chips.child(ui::pill(("start-from", i), tm.name.clone(), false, &t).on_click(cx.listener(move |this, _, window, cx| {
                        this.start_mind(None, window, cx);
                        this.template = Some(i);
                        this.fill(&tm, window, cx);
                    })));
                }
                d.child(group(
                    vec![div()
                        .px(px(14.))
                        .py(px(14.))
                        .child(div().text_size(px(ts::UI)).text_color(p.ink).child("None yet"))
                        .child(div().mt(px(2.)).text_size(px(ts::SMALL)).text_color(p.graphite).child("Start from one of these, or write your own with New mind."))
                        .child(chips)
                        .into_any_element()],
                    &t,
                ))
            })
            .when(!empty, |d| d.child(group(yours, &t)))
    }

    fn mind_form(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let templates = mind_templates();
        let mut chips = div().flex().flex_wrap().gap(px(6.));
        for (i, tm) in templates.iter().enumerate() {
            let tm = tm.clone();
            chips = chips.child(ui::pill(("tmpl", i), tm.name.clone(), self.template == Some(i), &t).on_click(cx.listener(move |this, _, window, cx| {
                this.template = Some(i);
                let m = tm.clone();
                this.fill(&m, window, cx);
            })));
        }
        // The box pads its words 12 px, as a text area pads its own: a one-line field is
        // given no padding of its own (`px_0`), a text area none from the box.
        let field = |label: &'static str, hint: &'static str, input: AnyElement, pads_itself: bool| {
            div()
                .px(px(14.))
                .py(px(10.))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(8.))
                        .child(div().text_size(px(ts::UI)).text_color(p.ink).child(label))
                        .child(div().text_size(px(ts::META)).text_color(p.faint).child(hint)),
                )
                .child(div().mt(px(6.)).px(px(if pads_itself { 0. } else { 12. })).py(px(6.)).rounded(px(7.)).bg(p.paper).border_1().border_color(p.rule).text_size(px(ts::UI)).child(input))
                .into_any_element()
        };
        let mut voices = segmented(&t);
        for (i, &v) in Voice::ALL.iter().enumerate() {
            voices = voices.child(segment(("voice", i), v.label(), self.voice == v, &t).on_click(cx.listener(move |this, _, _, cx| {
                this.voice = v;
                cx.notify();
            })));
        }
        let mut freqs = segmented(&t);
        for (i, &f) in Frequency::ALL.iter().enumerate() {
            freqs = freqs.child(segment(("freq", i), f.label(), self.frequency == f, &t).on_click(cx.listener(move |this, _, _, cx| {
                this.frequency = f;
                cx.notify();
            })));
        }
        let editing = self.editing.is_some();
        let named = !self.name.read(cx).value().trim().is_empty();
        div()
            .when(!editing, |d| d.child(section("Start from", None, &t).mt(px(4.))).child(chips))
            .child(section("The mind", None, &t).when(editing, |d| d.mt(px(4.))))
            .child(group(
                vec![
                    field("Name", "", Input::new(&self.name).appearance(false).px_0().into_any_element(), false),
                    field("Description", "Shown when you pick it", Input::new(&self.description).appearance(false).px_0().into_any_element(), false),
                    field("How it thinks", "What it looks for, pushes on, and leaves alone", Textarea::new(&self.how).appearance(false).into_any_element(), true),
                ],
                &t,
            ))
            .child(section("How it speaks", None, &t))
            .child(group(
                vec![row("Voice", None, voices, &t).into_any_element(), row("How often it speaks up", None, freqs, &t).into_any_element()],
                &t,
            ))
            .child(
                div()
                    .mt(px(22.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .when(editing, |d| d.child(parts::push_button("delete-mind", "Delete", &t).on_click(cx.listener(|this, _, _, cx| this.delete_mind(cx)))))
                    .child(div().flex_1())
                    .child(parts::push_button("cancel-mind", "Cancel", &t).on_click(cx.listener(|this, _, _, cx| {
                        this.tab = Tab::Minds;
                        this.editing = None;
                        cx.notify();
                    })))
                    .child(
                        parts::default_button("save-mind", if editing { "Save" } else { "Create mind" }, named, &t)
                            .on_click(cx.listener(|this, _, _, cx| this.save_mind(cx))),
                    ),
            )
    }

    fn about(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let s = cx.global::<Store>().settings.clone();
        div()
            .child(
                group(
                    vec![row(
                        "Inkling learns about you",
                        Some("When you say something lasting about yourself, like what you study or how you like feedback, it can be worked in here.".into()),
                        ui::switch("learns", s.learns_about_me, &t).on_click(cx.listener(|this, _, _, cx| this.update(cx, |s| s.learns_about_me = !s.learns_about_me))),
                        &t,
                    )
                    .into_any_element()],
                    &t,
                )
                .mt(px(4.)),
            )
            .child(section("What Inkling knows", Some(format!("{} of {ABOUT_LIMIT}", s.about_me.chars().count()).into()), &t))
            // The text area pads itself; this lines its words up with the rows' titles.
            .child(group(vec![div().px(px(4.)).py(px(6.)).text_size(px(ts::UI)).line_height(px(19.)).child(Textarea::new(&self.about).appearance(false)).into_any_element()], &t))
            .child(note("Edit it any time. Inkling only adds to it when the switch above is on.", &t))
    }

    fn model_tab(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let s = cx.global::<Store>().settings.clone();
        let brain = Brain::global(cx).read(cx);
        let (choices, live) = brain.model_choices(cx);
        let account = brain.account.clone();
        let limit = brain.limit.clone();
        let state = brain.state.clone();
        let last_error = brain.last_error.clone();
        let status = match (&state, &account) {
            (BrainState::Failed(e), _) => format!("Not running: {e}"),
            (_, Some(a)) => format!(
                "Signed in as {}{}",
                a.email.clone().unwrap_or_else(|| "your Claude account".into()),
                a.plan.as_ref().map(|p| format!(" · {p}")).unwrap_or_default()
            ),
            (BrainState::Starting, None) => "Starting…".into(),
            (BrainState::Ready, None) => "Ready".into(),
        };
        let limit_line = limit.map(|l| {
            let when = l
                .resets_at
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .map(|d| d.with_timezone(&chrono::Local).format("%a %-d %b, %H:%M").to_string());
            let kind = l.kind.unwrap_or_default().replace('_', " ");
            match (l.status.as_str(), when) {
                ("allowed", Some(w)) => format!("Usage: fine ({kind} window resets {w})"),
                ("allowed_warning", Some(w)) => format!("Usage: getting close to the {kind} limit (resets {w})"),
                ("rejected", Some(w)) => format!("Usage: limit reached, resets {w}"),
                (st, _) => format!("Usage: {st}"),
            }
        });
        let account_detail = std::iter::once(status).chain(limit_line).chain(last_error.map(|e| format!("Last error: {e}"))).collect::<Vec<_>>().join("\n");
        let claude = row(
            div().flex().items_center().gap(px(7.)).child(ui::small_dot(t.voice_ink, 6.)).child("Claude"),
            Some(account_detail.into()),
            parts::push_button("check-status", "Check again", &t).on_click(cx.listener(|_, _, _, cx| {
                Brain::global(cx).update(cx, |b, cx| b.check_status(cx));
            })),
            &t,
        );

        // The list as Claude Code gives it: the names that move with releases first, the
        // pinned versions folded under them. The saved model stays visible even when it is
        // not in the list.
        let current = chosen_model(&choices, &s.model);
        let mut rows: Vec<(ModelChoice, bool)> = choices.iter().cloned().enumerate().map(|(i, c)| (c, Some(i) == current)).collect();
        if current.is_none() {
            let (value, name, description) = if s.model.trim().is_empty() {
                (String::new(), "Claude Code's default".to_string(), "Whatever model Claude Code uses unless told otherwise.".to_string())
            } else {
                (s.model.clone(), s.model.clone(), "Not in the list Claude Code gave. It is sent as written.".to_string())
            };
            rows.insert(0, (ModelChoice { value, name, description, resolved: None, efforts: None }, true));
        }
        let pinned = rows.iter().filter(|(c, _)| c.is_pinned()).count();
        let unfold = self.show_versions || rows.iter().any(|(c, on)| *on && c.is_pinned());
        let mut list = vec![];
        for (i, (c, on)) in rows.iter().cloned().enumerate() {
            if c.is_pinned() && !unfold {
                continue;
            }
            if c.is_pinned() && rows.iter().position(|(c, _)| c.is_pinned()) == Some(i) {
                list.push(self.versions_toggle(pinned, true, &t, cx));
            }
            let value = if c.value == "default" { String::new() } else { c.value.clone() };
            let detail = (!c.description.is_empty()).then(|| SharedString::from(c.description.clone()));
            list.push(
                div()
                    .id(("model", i))
                    .cursor_pointer()
                    .hover(|st| st.bg(Glass::of(&p).hover))
                    .child(
                        row(
                            c.name.clone(),
                            detail,
                            div().size(px(18.)).flex().items_center().justify_center().when(on, |d| d.child(icon("check", 15., p.ink))),
                            &t,
                        )
                        .when(on, |d| d.font_weight(FontWeight::MEDIUM)),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let v = value.clone();
                        this.model.update(cx, |m, cx| m.set_value(v.clone(), window, cx));
                        this.update(cx, |s| s.model = v);
                    }))
                    .into_any_element(),
            );
        }
        if pinned > 0 && !unfold {
            list.push(self.versions_toggle(pinned, false, &t, cx));
        }
        list.push(
            row(
                "Model id",
                Some("Any model Claude Code takes. Empty is its default.".into()),
                div().w(px(220.)).h(px(28.)).px(px(9.)).flex().items_center().rounded(px(7.)).bg(p.paper).border_1().border_color(p.rule).text_size(px(ts::UI)).child(Input::new(&self.model).appearance(false).px_0()),
                &t,
            )
            .into_any_element(),
        );
        let source = if live {
            "From Claude Code"
        } else if choices.is_empty() {
            "Claude Code has not listed its models yet"
        } else {
            "As Claude Code listed them last"
        };

        // Effort: the levels this model takes, when Claude Code says; what is sent is shown chosen.
        let picked = rows.iter().find(|(_, on)| *on).map(|(c, _)| c.clone());
        let levels: Option<Vec<String>> = match picked.as_ref().and_then(|c| c.efforts.clone()) {
            Some(l) => Some(l),
            None => Some(USUAL_EFFORTS.iter().map(|s| s.to_string()).collect()),
        };
        let model_name = picked.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "this model".into());
        let effort_row = |label: &'static str, current: &str, read: bool, cx: &mut Context<Self>| {
            let sent = effort_for(&choices, &s.model, current);
            let control = match levels.as_ref().filter(|l| !l.is_empty()) {
                Some(levels) => {
                    let mut seg = segmented(&t);
                    for (i, e) in levels.iter().enumerate() {
                        let e = e.clone();
                        seg = seg.child(segment((if read { "eff-r" } else { "eff-a" }, i), e.clone(), sent == e, &t).on_click(cx.listener(move |this, _, _, cx| {
                            let v = e.clone();
                            this.update(cx, |s| if read { s.effort_read = v } else { s.effort = v });
                        })));
                    }
                    seg.into_any_element()
                }
                None => div().text_size(px(ts::SMALL)).text_color(p.faint).child("None").into_any_element(),
            };
            row(label, None, control, &t).into_any_element()
        };
        let no_effort = levels.as_ref().is_some_and(|l| l.is_empty());
        div()
            .child(group(vec![claude.into_any_element()], &t).mt(px(4.)))
            .child(section("Model", Some(source.into()), &t))
            .child(group(list, &t))
            .child(section("Effort", Some(format!("For {model_name}").into()), &t))
            .child(group(vec![effort_row("Reading along", &s.effort_read, true, cx), effort_row("Asking, drawing", &s.effort, false, cx)], &t))
            .child(note(
                if no_effort {
                    format!("{model_name} takes no effort level.")
                } else {
                    "Reading along runs after every paragraph, so it defaults to low effort.".to_string()
                },
                &t,
            ))
            .child(section("Coming next", None, &t))
            .child(group(
                vec![row("Codex", Some("Sign in with your ChatGPT account using the device code flow, and pick Codex per note.".into()), div(), &t).into_any_element()],
                &t,
            ))
    }

    /// The row that folds the pinned versions away or shows them.
    fn versions_toggle(&self, count: usize, open: bool, t: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let p = t.p;
        div()
            .id("model-versions")
            .px(px(14.))
            .py(px(9.))
            .flex()
            .items_center()
            .gap(px(6.))
            .text_size(px(ts::SMALL))
            .text_color(p.graphite)
            .cursor_pointer()
            .hover(move |s| s.text_color(p.ink))
            .child(icon(if open { "chevron-down" } else { "chevron-right" }, 13., p.graphite))
            .child(format!("Specific versions ({count})"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_versions = !this.show_versions;
                cx.notify();
            }))
            .into_any_element()
    }

    fn shortcuts_pane(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let warn: Hsla = ink_color(InkColor::Orange, &p);
        let custom = cx.global::<Store>().settings.shortcuts.clone();
        let query = self.search.read(cx).value().trim().to_lowercase();
        let terms: Vec<&str> = query.split_whitespace().collect();
        struct Line {
            name: &'static str,
            label: &'static str,
            hint: Option<&'static str>,
            group: Group,
            keys: Vec<String>,
            custom: bool,
            clashes: Vec<(String, &'static str)>,
        }
        let lines: Vec<Line> = {
            let table = &cx.global::<Shortcuts>().0;
            let clashes = shortcuts::clashes(table, &custom);
            let label_of = |name: &str| table.iter().find(|s| s.name == name).map(|s| s.label).unwrap_or("another shortcut");
            let lines = table
                .iter()
                .filter(|s| {
                    let keys = shortcuts::shown(s, &custom);
                    let hay = format!(
                        "{} {} {} {} {}",
                        s.label,
                        s.group.label(),
                        s.hint.unwrap_or(""),
                        keys.join(" "),
                        keys.iter().map(|k| shortcuts::glyphs(k)).collect::<Vec<_>>().join(" ")
                    )
                    .to_lowercase();
                    terms.iter().all(|term| hay.contains(term))
                })
                .map(|s| Line {
                    name: s.name,
                    label: s.label,
                    hint: s.hint,
                    group: s.group,
                    keys: shortcuts::shown(s, &custom).to_vec(),
                    custom: custom.contains_key(s.name),
                    clashes: clashes.get(s.name).map(|c| c.iter().map(|c| (c.keys.clone(), label_of(c.other))).collect()).unwrap_or_default(),
                })
                .collect();
            lines
        };
        let recording = self.recording.as_ref().map(|r| (r.name, r.slot, r.held, r.refused));
        let mut out = div();
        let mut first = true;
        for g in Group::ALL {
            let in_group: Vec<&Line> = lines.iter().filter(|l| l.group == g).collect();
            if in_group.is_empty() {
                continue;
            }
            let mut rows = vec![];
            for l in in_group {
                let name = l.name;
                let mut caps = div().flex_none().flex().items_center().gap(px(6.));
                let listening_here = recording.filter(|r| r.0 == name);
                let listening_cap = |held: Modifiers| {
                    let shown = shortcuts::glyphs(&Keystroke { modifiers: held, key: String::new(), key_char: None }.unparse(OperatingSystem::Mac));
                    parts::keycap(SharedString::from(format!("rec-{name}")), if held.modified() { format!("{shown}…") } else { "Type keys…".into() }, Cap::Listening, warn, &t)
                        .child(ui::small_dot(t.voice_ink, 5.).with_animation(
                            "listening",
                            Animation::new(Duration::from_millis(900)).repeat(),
                            |d, delta| d.opacity(0.35 + 0.65 * (0.5 + 0.5 * (delta * std::f32::consts::TAU).sin())),
                        ))
                };
                for (i, keys) in l.keys.iter().enumerate() {
                    if let Some((_, Some(slot), held, _)) = listening_here
                        && slot == i
                    {
                        caps = caps.child(listening_cap(held));
                        continue;
                    }
                    let clash = l.clashes.iter().any(|(k, _)| k == keys);
                    let cap_group = SharedString::from(format!("cap-{name}-{i}"));
                    caps = caps.child(
                        parts::keycap(SharedString::from(format!("key-{name}-{i}")), shortcuts::glyphs(keys), if clash { Cap::Clash } else { Cap::Keys }, warn, &t)
                            .group(cap_group.clone())
                            .relative()
                            .on_click(cx.listener(move |this, _, window, cx| this.start_recording(name, Some(i), window, cx)))
                            .child(
                                div()
                                    .id(SharedString::from(format!("unkey-{name}-{i}")))
                                    .absolute()
                                    .top(px(-6.))
                                    .right(px(-6.))
                                    .size(px(14.))
                                    .rounded_full()
                                    .bg(p.graphite)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .invisible()
                                    .group_hover(cap_group, |s| s.visible())
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(p.ink))
                                    .active(|s| s.opacity(0.7))
                                    .child(icon("x", 9., p.paper))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.remove_keys(name, i, cx);
                                    })),
                            ),
                    );
                }
                if let Some((_, None, held, _)) = listening_here {
                    caps = caps.child(listening_cap(held));
                } else if l.keys.is_empty() {
                    caps = caps.child(
                        parts::keycap(SharedString::from(format!("none-{name}")), "None", Cap::Empty, warn, &t)
                            .on_click(cx.listener(move |this, _, window, cx| this.start_recording(name, None, window, cx))),
                    );
                }
                let row_group = SharedString::from(format!("row-{name}"));
                let add = parts::text_button(SharedString::from(format!("add-{name}")), "Add", &t)
                    .invisible()
                    .group_hover(row_group.clone(), |s| s.visible())
                    .on_click(cx.listener(move |this, _, window, cx| this.start_recording(name, None, window, cx)));
                let reset = l.custom.then(|| {
                    parts::text_button(SharedString::from(format!("reset-{name}")), "Reset", &t).on_click(cx.listener(move |this, _, _, cx| {
                        this.stop_recording(cx);
                        shortcuts::reset(Some(name), cx);
                        cx.notify();
                    }))
                });
                let mut left = div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().flex().items_center().gap(px(6.)).child(div().text_size(px(ts::UI)).text_color(p.ink).child(l.label)).when(l.custom, |d| {
                        d.child(div().text_size(px(ts::META)).text_color(p.faint).child("Changed"))
                    }))
                    .children(l.hint.map(|h| div().text_size(px(ts::SMALL)).line_height(px(16.)).text_color(p.graphite).child(h)));
                for (keys, other) in &l.clashes {
                    left = left.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(ts::SMALL))
                            .text_color(warn)
                            .child(icon("triangle-alert", 12., warn))
                            .child(format!("{} also runs {}", shortcuts::glyphs(keys), other)),
                    );
                }
                if let Some((_, _, _, Some(why))) = listening_here {
                    left = left.child(div().flex().items_center().gap(px(5.)).text_size(px(ts::SMALL)).text_color(warn).child(icon("info", 12., warn)).child(why));
                } else if listening_here.is_some() {
                    left = left.child(div().text_size(px(ts::SMALL)).text_color(p.graphite).child("Press the new keys. Esc cancels."));
                }
                rows.push(
                    div()
                        .min_h(px(44.))
                        .px(px(14.))
                        .py(px(9.))
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .group(row_group)
                        .child(left)
                        .child(add)
                        .children(reset)
                        .child(caps)
                        .into_any_element(),
                );
            }
            let header = section(g.label(), Some(g.scope().into()), &t);
            out = out.child(if first { header.mt(px(4.)) } else { header }).child(group(rows, &t));
            first = false;
        }
        if first {
            out = out.child(
                div()
                    .mt(px(40.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.))
                    .child(icon("search", 18., p.faint))
                    .child(div().text_size(px(ts::UI)).text_color(p.graphite).child(format!("No shortcut matches “{}”.", self.search.read(cx).value().trim()))),
            );
        }
        out
    }

    fn help(&mut self, cx: &mut Context<Self>) -> Div {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let dir = cx.global::<Store>().dir().to_path_buf();
        let warn = ink_color(InkColor::Orange, &p);
        let lines = [
            ("Write.", "When a paragraph is done, Inkling reads it and only speaks up when something is worth a second look."),
            ("Select words", "for the toolbar: Find a loose thread, Ask Inkling, or Sketch it."),
            ("Start a paragraph with “Inkling,”", "to ask it directly, in either mode."),
        ];
        let text_rows = lines
            .iter()
            .map(|(lead, rest)| {
                div()
                    .px(px(14.))
                    .py(px(10.))
                    .text_size(px(ts::UI))
                    .line_height(px(19.))
                    .text_color(p.graphite)
                    .child(StyledText::new(format!("{lead} {rest}")).with_highlights([(0..lead.len(), HighlightStyle { color: Some(p.ink), ..Default::default() })]))
                    .into_any_element()
            })
            .collect();
        let key_rows = [
            ("app::OpenPalette", "Find or start a note"),
            ("app::ToggleChat", "Chat"),
            ("app::FindLooseThread", "Find a loose thread"),
            ("app::JustMyWords", "Hide threads"),
            ("app::ExportNote", "Export"),
            ("app::OpenSettings", "Settings"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (name, label))| {
            let keys = shortcuts::keys_for(name, cx);
            let cap = match &keys {
                Some(k) => parts::keycap(("help-key", i), k.clone(), Cap::Keys, warn, &t),
                None => parts::keycap(("help-key", i), "None", Cap::Empty, warn, &t),
            };
            row(label, None, cap.cursor_default(), &t).into_any_element()
        })
        .chain(std::iter::once(row("Speak", Some("Dictate into the note.".into()), parts::keycap("help-hold", "Hold ⌥", Cap::Keys, warn, &t).cursor_default(), &t).into_any_element()))
        .collect();
        div()
            .child(group(text_rows, &t).mt(px(4.)))
            .child(section("Keys to know", Some("Change them in Shortcuts".into()), &t))
            .child(group(key_rows, &t))
            .child(section("Your notes", None, &t))
            .child(group(
                vec![row(
                    "Kept on this Mac",
                    Some(dir.display().to_string().into()),
                    parts::push_button("reveal", "Show in Finder", &t).on_click(cx.listener(move |_, _, _, cx| cx.reveal_path(&dir))),
                    &t,
                )
                .into_any_element()],
                &t,
            ))
    }

    // ----- the window ------------------------------------------------------------------

    fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let g = Glass::of(&p);
        let current = self.tab.pane();
        let mut items = div().flex().flex_col().gap(px(2.)).px(px(10.));
        for (i, tab) in Tab::PANES.into_iter().enumerate() {
            let on = tab == current;
            items = items.child(
                div()
                    .id(("pane", i))
                    .h(px(30.))
                    .px(px(9.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .text_size(px(ts::UI))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .when(on, |d| d.bg(g.chosen).border_1().border_color(g.chosen_rim).shadow(g.float(1., 3.)).font_weight(FontWeight::MEDIUM))
                    .when(!on, |d| d.border_1().border_color(transparent_black()).hover(move |s| s.bg(g.hover)).active(move |s| s.bg(g.hover)))
                    .child(icon(tab.icon(), 15., if on { t.voice_ink } else { p.graphite }))
                    .child(tab.label())
                    .on_click(cx.listener(move |this, _, window, cx| this.show(tab, window, cx))),
            );
        }
        div().flex_none().w(px(220.)).h_full().pl(px(10.)).py(px(10.)).child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .rounded(px(16.))
                .relative()
                .bg(linear_gradient(180., linear_color_stop(g.pane_top, 0.), linear_color_stop(g.pane_bottom, 1.)))
                .border_1()
                .border_color(g.rim)
                .shadow(g.floating())
                .child(g.inner_edge(16.))
                // The traffic lights sit here, on the glass; it moves the window like a title bar.
                .child(div().h(px(52.)).flex_none().window_control_area(WindowControlArea::Drag))
                .child(items)
                .child(div().flex_1().window_control_area(WindowControlArea::Drag))
                .child(div().px(px(18.)).pb(px(16.)).flex().items_end().justify_between().child(ui::wordmark(&t, 16.)).child(
                    div().text_size(px(ts::META)).text_color(p.graphite).child(concat!("v", env!("CARGO_PKG_VERSION"))),
                )),
        )
    }

    fn header_extras(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let t = Theme::get(cx).clone();
        let p = t.p;
        match self.tab {
            Tab::Minds => Some(parts::default_button("new-mind", "New mind", true, &t).on_click(cx.listener(|this, _, window, cx| this.start_mind(None, window, cx))).into_any_element()),
            Tab::Shortcuts => {
                let any_custom = !cx.global::<Store>().settings.shortcuts.is_empty();
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .w(px(210.))
                                .h(px(28.))
                                .px(px(9.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .rounded(px(8.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.rule)
                                .child(icon("search", 13., p.faint))
                                .child(div().flex_1().min_w(px(0.)).text_size(px(ts::UI)).child(Input::new(&self.search).appearance(false).px_0())),
                        )
                        .child(
                            parts::push_button("reset-all", "Reset all", &t)
                                .when(!any_custom, |d| d.opacity(0.45).cursor_default())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if any_custom {
                                        this.stop_recording(cx);
                                        shortcuts::reset(None, cx);
                                        cx.notify();
                                    }
                                })),
                        )
                        .into_any_element(),
                )
            }
            _ => None,
        }
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::get(cx).clone();
        let p = t.p;
        let g = Glass::of(&p);
        let content = match self.tab {
            Tab::Customize => self.reading(cx),
            Tab::Appearance => self.appearance(cx),
            Tab::Minds => self.minds(cx),
            Tab::NewMind => self.mind_form(cx),
            Tab::AboutYou => self.about(cx),
            Tab::Model => self.model_tab(cx),
            Tab::Shortcuts => self.shortcuts_pane(cx),
            Tab::Help => self.help(cx),
        };
        let title = match self.tab {
            Tab::NewMind if self.editing.is_some() => "Edit mind",
            Tab::NewMind => "New mind",
            Tab::Shortcuts => "Keyboard shortcuts",
            tab => tab.label(),
        };
        let extras = self.header_extras(cx);
        let back = (self.tab == Tab::NewMind).then(|| {
            div()
                .id("back-to-minds")
                .flex()
                .items_center()
                .gap(px(2.))
                .mb(px(2.))
                .text_size(px(ts::SMALL))
                .text_color(p.graphite)
                .cursor_pointer()
                .hover(move |s| s.text_color(p.ink))
                .child(icon("chevron-left", 13., p.graphite))
                .child("Minds")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.tab = Tab::Minds;
                    this.editing = None;
                    cx.notify();
                }))
        });
        let sidebar = self.sidebar(cx);
        let page = div().flex_1().min_w(px(0.)).h_full().p(px(10.)).child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .rounded(px(16.))
                .overflow_hidden()
                .bg(g.page)
                .border_1()
                .border_color(g.page_rim)
                .shadow(g.float(6., 20.))
                .child(
                    div()
                        .flex_none()
                        .px(px(32.))
                        .pt(px(26.))
                        .pb(px(6.))
                        .window_control_area(WindowControlArea::Drag)
                        .children(back)
                        .child(
                            div()
                                .min_h(px(28.))
                                .flex()
                                .items_center()
                                .gap(px(16.))
                                .child(div().flex_1().min_w(px(0.)).text_size(px(ts::PANE)).font_weight(FontWeight::SEMIBOLD).text_color(p.ink).child(title))
                                .children(extras),
                        )
                        .child(div().mt(px(4.)).max_w(px(600.)).text_size(px(ts::SMALL)).line_height(px(17.)).text_color(p.graphite).child(self.tab.blurb())),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("settings-scroll-{:?}", self.tab)))
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .child(div().px(px(32.)).pt(px(8.)).pb(px(32.)).child(content.max_w(px(640.)))),
                ),
        );
        div()
            .id("settings")
            .key_context("Settings")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .bg(g.desk)
            .font_family(SANS)
            .text_color(p.ink)
            .on_key_down(cx.listener(Self::on_key))
            .on_modifiers_changed(cx.listener(Self::on_modifiers))
            // A click anywhere else ends recording; a keycap's own click starts it again.
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.stop_recording(cx)))
            .child(sidebar)
            .child(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every size in Settings is one of the interface's (`theme::ts`), so its rows, labels and
    /// hints line up with the rest of Inkling. Only a picture is drawn to its own scale.
    #[std::prelude::v1::test]
    fn settings_text_comes_in_the_shared_sizes() {
        let literal = ["text_size", "(px(", ""].concat();
        let scaled = ["text_size", "(px(ts::"].concat();
        for (file, src) in [("settings.rs", include_str!("settings.rs")), ("settings/parts.rs", include_str!("settings/parts.rs"))] {
            let lines: Vec<&str> = src.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let picture = i > 0 && lines[i - 1].contains("the picture's scale");
                assert!(!line.contains(&literal) || line.contains(&scaled) || picture, "{file}:{}: a size off the scale: {}", i + 1, line.trim());
            }
        }
    }

    #[std::prelude::v1::test]
    fn about_you_is_capped_in_characters_not_bytes() {
        assert_eq!(over_limit(""), None);
        assert_eq!(over_limit(&"a".repeat(700)), None);
        assert_eq!(over_limit(&"a".repeat(701)).unwrap().len(), 700);
        // 700 two- and four-byte characters fit; the 701st is cut on its boundary.
        let accents = "é".repeat(700);
        assert_eq!(accents.len(), 1400);
        assert_eq!(over_limit(&accents), None);
        let emoji = "🦀".repeat(705);
        let cut = over_limit(&emoji).unwrap();
        assert_eq!(cut.chars().count(), 700);
        assert_eq!(cut.len(), 2800);
        let mixed = format!("{}x日本", "あ".repeat(699));
        assert_eq!(over_limit(&mixed).unwrap(), format!("{}x", "あ".repeat(699)));
    }

    #[std::prelude::v1::test]
    fn every_pane_is_in_the_sidebar_and_can_be_opened_by_name() {
        assert_eq!(Tab::PANES.len(), 7);
        for tab in Tab::PANES {
            assert_eq!(tab.pane(), tab);
            assert!(!tab.label().is_empty() && !tab.blurb().is_empty());
        }
        assert_eq!(Tab::NewMind.pane(), Tab::Minds);
        for (name, tab) in [("reading", Tab::Customize), ("Shortcuts", Tab::Shortcuts), (" model ", Tab::Model), ("about", Tab::AboutYou), ("newmind", Tab::NewMind)] {
            assert_eq!(Tab::named(name), Some(tab), "{name}");
        }
        assert_eq!(Tab::named("nope"), None);
    }
}
